use std::sync::Arc;

use serde::Deserialize;

use crate::util::api_util::*;
use actix_session::Session;
use actix_web::{get, post, web, HttpResponse, Responder};

use crate::DbPool;

#[get("/my_email")]
async fn get_email(
    session: Session,
    pool: web::Data<Arc<DbPool>>,
) -> Result<impl Responder, APIError> {
    let location = "my_email";

    let (user_id, _) = user_privilege_check(&session, PRIVILEGE_MINIMAL)?;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    if let Some(email) = get_email_by_user(user_id, &mut conn)
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?
    {
        Ok(HttpResponse::Ok().json(email))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
#[derive(Debug, Deserialize)]
struct EmailRequest {
    pub email: String,
}

impl APIRequest for EmailRequest {
    fn ok(&self) -> bool {
        self.email.len() <= 100
    }
}

#[post("/my_email")]
async fn post_email(
    session: Session,
    pool: web::Data<Arc<DbPool>>,
    form: web::Json<EmailRequest>,
) -> Result<impl Responder, APIError> {
    let location = "my_email";
    form.sanity()?;

    let (user_id, _) = user_privilege_check(&session, PRIVILEGE_MINIMAL)?;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    insert_or_update_email(user_id, form.email.clone(), &mut conn)
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    Ok(HttpResponse::Ok().finish())
}
