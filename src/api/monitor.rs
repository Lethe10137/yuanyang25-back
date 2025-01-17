use std::sync::Arc;

use crate::util::cache::Cache;

use crate::util::api_util::*;
use actix_session::Session;
use actix_web::{get, web, HttpResponse, Responder};

use crate::DbPool;

#[get("/cache_size")]
async fn cache_size(
    mut session: Session,
    pool: web::Data<Arc<DbPool>>,
    cache: web::Data<Arc<Cache>>,
) -> Result<impl Responder, APIError> {
    let location = "cache_size";
    get_team_id(&mut session, &pool, PRIVILEGE_ADMIN, location).await?;
    Ok(HttpResponse::Ok().json(cache.get_size()))
}
