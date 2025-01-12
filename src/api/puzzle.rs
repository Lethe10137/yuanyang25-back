use std::collections::HashMap;

use crate::models::{Hint, MidAnswer, PuzzleBase};

use crate::util::cache::Cache;
use crate::util::cipher_util::prepare_hashed_answer;
use crate::util::economy::{
    compulsory_team_balance, puzzle_reward, puzzle_unlock_price, try_modify_team_balance,
};
use crate::util::{api_util::*, cipher_util::check_answer};

use actix_web::{get, post, web, HttpResponse, Responder};

use diesel::ExpressionMethods;
use diesel_async::{AsyncConnection, RunQueryDsl};

use serde::{Deserialize, Serialize};

use crate::{DbPool, Ext};

#[derive(Clone)]
pub struct Puzzle {
    pub base: PuzzleBase,
    pub mid_answers: HashMap<String, MidAnswer>,
    pub hints: HashMap<i32, Hint>,
}

pub enum CheckAnswerResult {
    Accepted { reward_tokens: i64 },
    AcceptedMidAnswer { mid_id: i32, response: String },
    WrongAnswer,
}

impl Puzzle {
    pub fn new(base: PuzzleBase, hints: Vec<Hint>, mid_answers: Vec<MidAnswer>) -> Self {
        let mid_answers = mid_answers
            .into_iter()
            .map(|mid_ans| (prepare_hashed_answer(&mid_ans.query, &base.key), mid_ans))
            .collect::<HashMap<String, MidAnswer>>();

        let hints = hints
            .into_iter()
            .map(|hint| (hint.id, hint))
            .collect::<HashMap<i32, Hint>>();

        Self {
            base,
            mid_answers,
            hints,
        }
    }

    pub fn check(&self, submission: &str) -> CheckAnswerResult {
        if check_answer(&self.base.answer, &self.base.key, submission) {
            return CheckAnswerResult::Accepted {
                reward_tokens: puzzle_reward(self.base.bounty),
            };
        }
        if let Some(mid) = self.mid_answers.get(submission) {
            return CheckAnswerResult::AcceptedMidAnswer {
                mid_id: mid.id,
                response: mid.response.clone(),
            };
        }
        CheckAnswerResult::WrongAnswer
    }
}

use actix_session::Session;

#[derive(Debug, Deserialize)]
struct DecipherKeyRequest {
    puzzle_id: i32,
}

impl APIRequest for DecipherKeyRequest {
    fn ok(&self) -> bool {
        self.puzzle_id >= 0
    }
}

#[derive(Debug, Serialize)]
enum DecipherKeyResponse {
    // returns the decipher key!.
    Success(String),
    NotAllowed,
}

// [[API]]
// desp: Return the decipher_key of a puzzle
// Method: GET
// URL: /decipher_key
// Request Body: `DecipherKeyRequest`
// Response Body: `DecipherKeyResponse`
#[get("/decipher_key")]
async fn decipher_key(
    pool: web::Data<DbPool>,
    cache: web::Data<Cache>,
    form: web::Query<DecipherKeyRequest>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "decipher_key";

    let team_id = get_team_id(&mut session, &pool, PRIVILEGE_MINIMAL, location).await?;

    let puzzle_id = form.puzzle_id;

    let result = if let Some(result) = cache.check_unlock_cached(team_id, puzzle_id).await? {
        DecipherKeyResponse::Success(result)
    } else {
        DecipherKeyResponse::NotAllowed
    };

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
enum SubmitAmswerResponse {
    HasSubmitted,
    Success {
        puzzle_id: i32,
        award_token: i64,
        new_balance: i64,
    },
    TryAgainAfter(i64),
    WrongAnswer {
        penalty_token: i64,
        try_again_after: i64,
        new_balance: i64,
    },
    MidAnswerResponse(String),
}

// [[API]]
// desp: Submit a puzzle
// Method: POST
// URL: /submit
// Request Body: `SubmitAnswerRequest`
// Response Body: `SubmitAnswerResponse`
#[post("/submit_answer")]
async fn submit_answer(
    pool: web::Data<DbPool>,
    cache: web::Data<Cache>,
    form: web::Json<SubmitAnswerRequest>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "submit_answer";
    let team_id = get_team_id(&mut session, &pool, PRIVILEGE_MINIMAL, location).await?;

    let puzzle_id = form.puzzle_id;

    if let Some(wa_penalty_until) = cache.check_no_submission_cached(team_id, puzzle_id).await? {
        return Ok(HttpResponse::Ok().json(SubmitAmswerResponse::TryAgainAfter(
            wa_penalty_until.timestamp(),
        )));
    }

    //Iff None, wrong answer!
    let check_result = cache
        .query_puzzle_cached(puzzle_id, |puzzle| puzzle.check(&form.answer))
        .await?;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let result = conn
        .transaction::<_, APIError, _>(|conn| {
            Box::pin(async move {
                match check_result {
                    CheckAnswerResult::Accepted { reward_tokens } => {
                        use crate::schema::submission::dsl::*;
                        let submission_result = diesel::insert_into(submission)
                            .values((
                                team.eq(team_id),
                                puzzle.eq(puzzle_id),
                                reward.eq(reward_tokens),
                            ))
                            .on_conflict((team, puzzle))
                            .do_nothing()
                            .execute(conn)
                            .await?;
                        if submission_result == 0 {
                            return Ok(SubmitAmswerResponse::HasSubmitted);
                        }
                        let new_balance = try_modify_team_balance(
                            team_id,
                            reward_tokens,
                            format!("Reward for puzzle {}", puzzle_id).as_str(),
                            conn,
                        )
                        .await
                        .map_err(Into::<APIError>::into)
                        .map_err(|e| e.set_location(location).tap(APIError::log))?;

                        Ok(SubmitAmswerResponse::Success {
                            puzzle_id,
                            award_token: reward_tokens,
                            new_balance,
                        })
                    }
                    CheckAnswerResult::AcceptedMidAnswer { mid_id, response } => {
                        use crate::schema::mid_answer_submission::dsl::*;
                        let submission_result = diesel::insert_into(mid_answer_submission)
                            .values((team.eq(team_id), mid_answer.eq(mid_id)))
                            .on_conflict((team, mid_answer))
                            .do_nothing()
                            .execute(conn)
                            .await?;

                        if submission_result == 1 {
                            //newly submitted mid answer
                            let old_penalty = fetch_wa_cnt(puzzle_id, team_id, conn).await?;
                            let new_penalty = old_penalty
                                .map_or_else(WaPenalty::new, WaPenalty::on_new_mid_answer);
                            insert_or_update_wa_cnt(puzzle_id, team_id, new_penalty, conn).await?;
                        }

                        Ok(SubmitAmswerResponse::MidAnswerResponse(response))
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

                        let result = SubmitAmswerResponse::WrongAnswer {
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

#[derive(Debug, Deserialize)]
struct UnlockRequest {
    puzzle_id: i32,
}

impl APIRequest for UnlockRequest {
    fn ok(&self) -> bool {
        self.puzzle_id >= 0
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
    NotAllowed,
}

// [[API]]
// desp: Pay to unlock
// Method: POST
// URL: /unlock
// Request Body: `UnlockRequest`
// Response Body: `UnlockResponse`
#[post("/unlock")]
async fn unlock(
    pool: web::Data<DbPool>,
    cache: web::Data<Cache>,
    form: web::Query<UnlockRequest>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "unlock";

    let team_id = get_team_id(&mut session, &pool, PRIVILEGE_MINIMAL, location).await?;

    let puzzle_id = form.puzzle_id;

    if let Some(result) = cache.check_unlock_cached(team_id, puzzle_id).await? {
        return Ok(HttpResponse::Ok().json(UnlockResponse::AlreadyUnlocked(result)));
    }

    let (price, key, is_meta) = cache
        .query_puzzle_cached(puzzle_id, |puzzle| {
            (
                puzzle_unlock_price(puzzle.base.unlock),
                puzzle.base.key.clone(),
                puzzle.base.meta,
            )
        })
        .await
        .map_err(|e| e.set_location(location).tap(APIError::log))?;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    if is_meta && count_passed(team_id, &mut conn).await? < 6 {
        return Ok(HttpResponse::Ok().json(UnlockResponse::NotAllowed));
    }

    let result = conn
        .transaction::<_, APIError, _>(|conn| {
            Box::pin(async move {
                let new_balance = try_modify_team_balance(
                    team_id,
                    -price,
                    format!("Unlocking puzzle {}", puzzle_id).as_str(),
                    conn,
                )
                .await
                .map_err(Into::<APIError>::into)
                .map_err(|e| e.set_location(location).tap(APIError::log))?;

                use crate::schema::unlock::dsl::*;

                diesel::insert_into(unlock)
                    .values((team.eq(team_id), puzzle.eq(puzzle_id)))
                    .execute(conn)
                    .await?;

                Ok(UnlockResponse::Success {
                    key,
                    price,
                    new_balance,
                })
            })
        })
        .await
        .map_err(|e| e.set_location(location).tap(APIError::log))?;

    Ok(HttpResponse::Ok().json(result))
}
