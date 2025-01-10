use crate::util::api_util::*;
use crate::util::cache::Cache;

use actix_web::{get, post, web, HttpResponse, Responder};

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

    let team_id = get_team_id(&mut session, &pool, location).await?;

    let puzzle_id = form.puzzle_id;

    let result = if let Some(result) = cache.check_unlock_cached(team_id, puzzle_id).await? {
        UnlockResponse::Success(result)
    } else {
        UnlockResponse::NotAllowed
    };

    Ok(HttpResponse::Ok().json(result))
}
