use crate::util::cache::Cache;
use crate::util::economy::{puzzle_reward, try_modify_team_balance};
use crate::util::{api_util::*, cipher_util::check_answer};

use actix_web::{get, post, web, HttpResponse, Responder};
use diesel::ExpressionMethods;
use diesel_async::{AsyncConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};

use crate::{DbPool, Ext};

use actix_session::Session;

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
    // returns the decipher key!.
    Success(String),
    NotAllowed,
}

// [[API]]
// desp: Return the decipher_key of a puzzle
// Method: GET
// URL: /decipher_key
// Request Body: `UnlockRequest`
// Response Body: `UnlockResponse`
#[get("/decipher_key")]
async fn decipher_key(
    pool: web::Data<DbPool>,
    cache: web::Data<Cache>,
    form: web::Query<UnlockRequest>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "decipher_key";

    let team_id = get_team_id(&mut session, &pool, PRIVILEGE_MINIMAL, location).await?;

    let puzzle_id = form.puzzle_id;

    let result = if let Some(result) = cache.check_unlock_cached(team_id, puzzle_id).await? {
        UnlockResponse::Success(result)
    } else {
        UnlockResponse::NotAllowed
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
    Success { puzzle_id: i32, award_token: i64 },
    WrongAnswer,
    Submitted,
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

    //Iff None, wrong answer!
    let reward_tokens = cache
        .query_puzzle_cached(puzzle_id, |puzzle| {
            check_answer(&puzzle.answer, &puzzle.key, &form.answer)
                .then(|| puzzle_reward(puzzle.bounty))
        })
        .await?;

    let result = if let Some(reward_tokens) = reward_tokens {
        let mut conn = pool
            .get()
            .await
            .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

        use crate::schema::submission::dsl::*;

        conn.transaction::<_, APIError, _>(|conn| {
            Box::pin(async move {
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
                    return Ok(SubmitAmswerResponse::Submitted);
                } else {
                    try_modify_team_balance(
                        team_id,
                        reward_tokens,
                        format!("Reward for puzzle {}", puzzle_id).as_str(),
                        conn,
                    )
                    .await
                    .map_err(Into::<APIError>::into)
                    .map_err(|e| e.set_location(location).tap(APIError::log))?;
                }

                Ok(SubmitAmswerResponse::Success {
                    puzzle_id,
                    award_token: reward_tokens,
                })
            })
        })
        .await
        .map_err(|e| e.set_location(location).tap(APIError::log))?
    } else {
        SubmitAmswerResponse::WrongAnswer
    };

    Ok(HttpResponse::Ok().json(result))
}
