use std::collections::HashMap;
use std::sync::Arc;

use crate::models::{PuzzleBase, WaPenalty};

use crate::util::api_util::*;
use crate::util::auto_fetch::Expiration;
use crate::util::cache::Cache;
use crate::util::cipher_util::cipher_chain;
use crate::util::economy::{
    compulsory_team_balance, deciper_price, puzzle_reward, try_modify_team_balance,
};

use actix_web::{get, post, web, HttpResponse, Responder};

use chrono::{DateTime, Utc};
use diesel::query_dsl::methods::FilterDsl;
use diesel::ExpressionMethods;
use diesel_async::{AsyncConnection, RunQueryDsl};

use serde::{Deserialize, Serialize};

use crate::{DbPool, Ext};

#[derive(Clone)]
pub struct Puzzle {
    pub base: PuzzleBase,
    pub answers: HashMap<String, i32>,
    pub other_answers: HashMap<String, String>,
}

pub enum CheckAnswerResult {
    Accepted {
        reward_tokens: i64,
        level: i32,
        total: i32,
    },
    WrongAnswer,
    Toast(String),
}

impl Puzzle {
    pub fn new(
        base: PuzzleBase,
        answers: Vec<(String, i32)>,
        other_answers: Vec<(String, String)>,
    ) -> Self {
        Self {
            base,
            answers: answers.into_iter().collect(),
            other_answers: other_answers.into_iter().collect(),
        }
    }
    pub fn check(&self, submission: &str) -> CheckAnswerResult {
        if let Some(toast) = self.other_answers.get(submission).cloned() {
            return CheckAnswerResult::Toast(toast);
        }

        match self.answers.get(submission).cloned() {
            Some(0) => CheckAnswerResult::Accepted {
                reward_tokens: puzzle_reward(self.base.bounty),
                level: 0,
                total: self.base.depth,
            },
            Some(level) => CheckAnswerResult::Accepted {
                reward_tokens: 0,
                level,
                total: self.base.depth,
            },
            _ => CheckAnswerResult::WrongAnswer,
        }
    }
}

use actix_session::Session;

#[derive(Debug, Deserialize)]
struct DecipherKeyRequest {
    decipher_id: i32,
}

impl APIRequest for DecipherKeyRequest {
    fn ok(&self) -> bool {
        self.decipher_id >= 0
    }
}

#[derive(Debug, Serialize)]
enum DecipherKeyResponse {
    // returns the decipher key!.
    Success(String),
    Price(i64),
}

// [[API]]
// desp: Return the decipher_key of a puzzle
// Method: GET
// URL: /decipher_key
// Request Body: `DecipherKeyRequest`
// Response Body: `DecipherKeyResponse`
#[get("/decipher_key")]
async fn decipher_key(
    pool: web::Data<Arc<DbPool>>,
    cache: web::Data<Arc<Cache>>,
    form: web::Query<DecipherKeyRequest>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "decipher_key";

    let team_id = get_team_id(&mut session, &pool, PRIVILEGE_MINIMAL, location).await?;

    let decipher_id = form.decipher_id;
    let answer = cache.decipher_cache.get(decipher_id).await?;

    let result = if let Some(level) = cache.unlock_cache.get((team_id, decipher_id)).await? {
        DecipherKeyResponse::Success(answer.get_key(level))
    } else {
        DecipherKeyResponse::Price(deciper_price(answer.pricing_type, answer.base_price))
    };

    Ok(HttpResponse::Ok().json(result))
}

#[derive(Debug, Deserialize)]
struct UnlockRequest {
    decipher_id: i32,
}

impl APIRequest for UnlockRequest {
    fn ok(&self) -> bool {
        self.decipher_id >= 0
    }
}

#[derive(Debug, Serialize)]
enum UnlockResponse {
    Success {
        key: String,
        price: i64,
        new_balance: i64,
    },
    AlreadyUnlocked(String),
}

// [[API]]
// desp: Pay to unlock
// Method: POST
// URL: /unlock
// Request Body: `UnlockRequest`
// Response Body: `UnlockResponse`
#[post("/unlock")]
async fn unlock(
    pool: web::Data<Arc<DbPool>>,
    cache: web::Data<Arc<Cache>>,
    form: web::Query<UnlockRequest>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "unlock";

    let team_id = get_team_id(&mut session, &pool, PRIVILEGE_MINIMAL, location).await?;

    let decipher_id = form.decipher_id;
    let answer = cache.decipher_cache.get(decipher_id).await?;

    if let Some(level) = cache.unlock_cache.get((team_id, decipher_id)).await? {
        return Ok(HttpResponse::Ok().json(UnlockResponse::AlreadyUnlocked(answer.get_key(level))));
    }

    let price = deciper_price(answer.pricing_type, answer.base_price);
    let level = (answer.depth - 1).max(0);
    let key = cipher_chain(&answer.root, level as usize);

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let result = conn
        .transaction::<_, APIError, _>(|conn| {
            Box::pin(async move {
                let new_balance = try_modify_team_balance(
                    team_id,
                    -price,
                    format!("Purchasing decipher_key {}", decipher_id).as_str(),
                    conn,
                    Some(decipher_id),
                )
                .await
                .map_err(Into::<APIError>::into)
                .map_err(|e| e.set_location(location).tap(APIError::log));

                match new_balance {
                    Ok(new_balance) => Ok(UnlockResponse::Success {
                        key,
                        price,
                        new_balance,
                    }),
                    Err(APIError::TransactionCancel { .. }) => {
                        Ok(UnlockResponse::AlreadyUnlocked(key))
                    }
                    Err(e) => Err(e),
                }
            })
        })
        .await
        .map_err(|e| e.set_location(location).tap(APIError::log))?;

    cache
        .unlock_cache
        .set(
            (team_id, decipher_id),
            Some(level),
            if level == 0 {
                Expiration::Long
            } else {
                Expiration::Short
            },
        )
        .await?;

    Ok(HttpResponse::Ok().json(result))
}

#[derive(Debug, Deserialize)]
struct SubmitAnswerRequest {
    puzzle_id: i32,
    answer: String,
}

impl APIRequest for SubmitAnswerRequest {
    fn ok(&self) -> bool {
        self.puzzle_id >= 0 && self.answer.len() == 64
    }
}

#[derive(Debug, Serialize)]
enum SubmitAnswerResponse {
    HasSubmitted,
    Success {
        puzzle_id: i32,
        award_token: i64,
        new_balance: i64,
        key: String,
    },
    TryAgainAfter(i64),
    WrongAnswer {
        penalty_token: i64,
        try_again_after: i64,
        new_balance: i64,
    },
    PleaseToast(String),
}

// [[API]]
// desp: Submit a puzzle
// Method: POST
// URL: /submit
// Request Body: `SubmitAnswerRequest`
// Response Body: `SubmitAnswerResponse`
#[post("/submit_answer")]
async fn submit_answer(
    pool: web::Data<Arc<DbPool>>,
    cache: web::Data<Arc<Cache>>,
    form: web::Json<SubmitAnswerRequest>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "submit_answer";
    let team_id = get_team_id(&mut session, &pool, PRIVILEGE_MINIMAL, location).await?;

    let puzzle_id = form.puzzle_id;

    if let Some(wa_penalty_until) = cache.query_wa_penalty(team_id, puzzle_id).await? {
        return Ok(HttpResponse::Ok().json(SubmitAnswerResponse::TryAgainAfter(
            wa_penalty_until.timestamp(),
        )));
    }

    let (check_result, decipher_id, is_meta) = cache
        .query_puzzle_cached(puzzle_id, |puzzle: &Puzzle| {
            (
                puzzle.check(&form.answer),
                puzzle.base.decipher,
                puzzle.base.meta,
            )
        })
        .await?;

    let old_level = if let Some(level) = cache.unlock_cache.get((team_id, decipher_id)).await? {
        level
    } else {
        return Err(APIError::InvalidQuery);
    };

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let result = conn
        .transaction::<_, APIError, _>(|conn| {
            Box::pin(async move {
                match check_result {
                    CheckAnswerResult::Toast(a) => Ok(SubmitAnswerResponse::PleaseToast(a)),

                    CheckAnswerResult::Accepted {
                        reward_tokens,
                        level,
                        total,
                    } => {
                        use crate::schema::submission::dsl::*;
                        let submission_result = diesel::insert_into(submission)
                            .values((
                                team.eq(team_id),
                                puzzle.eq(puzzle_id),
                                depth.eq(level),
                                reward.eq(reward_tokens),
                                meta.eq(is_meta),
                            ))
                            .on_conflict((team, puzzle, depth))
                            .do_nothing()
                            .execute(conn)
                            .await?;
                        if submission_result == 0 {
                            return Ok(SubmitAnswerResponse::HasSubmitted);
                        }

                        let new_balance = compulsory_team_balance(
                            team_id,
                            reward_tokens,
                            format!(
                                "Reward for puzzle {}, {} / {}",
                                puzzle_id,
                                total - level,
                                total
                            )
                            .as_str(),
                            conn,
                        )
                        .await
                        .map_err(Into::<APIError>::into)
                        .map_err(|e| e.set_location(location).tap(APIError::log))?;

                        if level < old_level {
                            let old_penalty = fetch_wa_cnt(puzzle_id, team_id, conn).await?;
                            let new_penalty = old_penalty
                                .map_or_else(WaPenalty::new, WaPenalty::on_new_mid_answer);
                            insert_or_update_wa_cnt(puzzle_id, team_id, new_penalty, conn).await?;
                        }

                        let level = level.min(old_level);
                        cache
                            .unlock_cache
                            .set(
                                (team_id, decipher_id),
                                Some(level),
                                if level == 0 {
                                    Expiration::Long
                                } else {
                                    Expiration::Short
                                },
                            )
                            .await?;
                        let answer = cache.decipher_cache.get(decipher_id).await?;

                        Ok(SubmitAnswerResponse::Success {
                            puzzle_id,
                            award_token: reward_tokens,
                            new_balance,
                            key: answer.get_key(level),
                        })
                    }

                    CheckAnswerResult::WrongAnswer => {
                        let mut penalty = fetch_wa_cnt(puzzle_id, team_id, conn)
                            .await?
                            .unwrap_or_else(WaPenalty::new);
                        let fine = penalty.on_wrong_answer();
                        assert!(fine >= 0);

                        let new_balance = compulsory_team_balance(
                            team_id,
                            -fine,
                            format!("Wrong answer penalty puzzle {}", puzzle_id).as_str(),
                            conn,
                        )
                        .await
                        .map_err(Into::<APIError>::into)
                        .map_err(|e| e.set_location(location).tap(APIError::log))?;

                        let result = SubmitAnswerResponse::WrongAnswer {
                            try_again_after: penalty.time_penalty_until.timestamp(),
                            penalty_token: fine,
                            new_balance,
                        };

                        insert_or_update_wa_cnt(puzzle_id, team_id, penalty, conn).await?;

                        Ok(result)
                    }
                }
            })
        })
        .await
        .map_err(|e| e.set_location(location).tap(APIError::log))?;

    Ok(HttpResponse::Ok().json(result))
}

#[derive(Debug, Serialize)]
pub enum PuzzleStatus {
    Passed,
    Unlocked,
    Locked,
}

#[derive(Debug, Serialize)]
pub struct PuzzleStatusItem {
    puzzle_id: i32,
    passed: usize,
    unlocked: usize,
}

#[derive(Debug, Serialize)]
struct PuzzleStatusResponse {
    updated: i64, // unix timestamp in seconds
    data: Vec<PuzzleStatusItem>,
}

#[get("/puzzle_status")]
async fn puzzle_status(cache: web::Data<Arc<Cache>>) -> Result<impl Responder, APIError> {
    let location = "puzzle_status";
    let cacheddata = cache
        .get_stat()
        .await
        .map_err(|e| e.set_location(location).tap(APIError::log))?;

    Ok(HttpResponse::Ok().json(PuzzleStatusResponse {
        data: cacheddata
            .data
            .iter()
            .map(|t| PuzzleStatusItem {
                puzzle_id: t.puzzle_id,
                passed: t.teams_passed as usize,
                unlocked: t.teams_unlocked as usize,
            })
            .collect(),
        updated: cacheddata.time.timestamp(),
    }))
}

#[derive(Debug, Serialize)]
enum RankResponse {
    Success { rank_record: i32, time: i64 },
    NotFound,
}

#[get("/rank")]
async fn rank(
    mut session: Session,
    pool: web::Data<Arc<DbPool>>,
) -> Result<impl Responder, APIError> {
    let location = "rank";

    let team_id = get_team_id(&mut session, &pool, PRIVILEGE_MINIMAL, location).await?;
    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    use crate::schema::final_meta_submission::dsl::*;
    use diesel::query_dsl::methods::SelectDsl;
    use diesel::result::Error;

    let result = final_meta_submission
        .filter(team.eq(team_id))
        .select((id, time))
        .first::<(i32, DateTime<Utc>)>(&mut conn)
        .await;

    let result = match result {
        Ok((rank, record_time)) => Ok(RankResponse::Success {
            rank_record: rank,
            time: record_time.timestamp(),
        }),
        Err(Error::NotFound) => Ok(RankResponse::NotFound),
        Err(e) => Err(log_server_error(e, location, ERROR_DB_UNKNOWN)),
    }?;

    Ok(HttpResponse::Ok().json(result))
}
