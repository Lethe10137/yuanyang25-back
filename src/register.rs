use crate::api_util::APIRequest;
use actix_web::{get, post, web, HttpResponse, Responder};
use dotenv::dotenv;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::env;

use diesel::prelude::*;

use crate::cipher_util::DecodeTokenError;
use crate::models::User;
use crate::{cipher_util, schema, DbPool};

use actix_session::Session;

#[derive(Debug, Deserialize)]
struct RegisterRequest {
    // Max 100.
    username: String,
    token: String,
    // SHA256 of the password.
    password: String,
}

impl APIRequest for RegisterRequest {
    fn sanity(self) -> Option<Self> {
        if self.username.len() > 100 || self.password.len() != 64 || self.token.len() != 128 {
            return None;
        }
        Some(self)
    }
}

#[derive(Debug, Serialize)]
enum RegisterResponse {
    // returns the user id.
    Success(i32),
    // json that describes the failure.
    Failed(DecodeTokenError),
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginData {
    userid: String,
    verification: String,
}

static LOGIN_TOKEN: Lazy<String> = Lazy::new(|| {
    dotenv().ok();
    env::var("LOGIN_TOKEN").expect("Environment variable LOGIN_TOKEN not set")
});

static REGISTER_TOKEN: Lazy<String> = Lazy::new(|| {
    dotenv().ok();
    env::var("REGISTER_TOKEN").expect("Environment variable REGISTER_TOKEN not set")
});

use log::{error, info};

// [[API]]
// Description: Register or update password with token from wechat.
// Method: Post
// URL: /register
// Request Body: `RegisterRequest`
// Response Body: `RegisterResponse`
//
#[post("/register")]
async fn register_user(
    pool: web::Data<DbPool>,
    form: web::Json<RegisterRequest>,
    session: Session,
) -> impl Responder {
    use schema::users;
    let mut conn = pool.get().expect("Failed to get DB connection");

    let form = sanity!(form);

    let response = match cipher_util::decode_token(&form.token, REGISTER_TOKEN.as_str()) {
        Ok((_version, mark, openid)) => {
            let (salt, salted_password) =
                cipher_util::gen_salted_password(&form.password, &LOGIN_TOKEN);

            let result = diesel::insert_into(users::table)
                .values((
                    users::username.eq(&form.username),
                    users::openid.eq(openid.as_str()),
                    users::priviledge.eq(mark as i32),
                    users::salt.eq(&salt),
                    users::password.eq(&salted_password),
                ))
                .on_conflict(users::openid)
                .do_update()
                .set((
                    users::username.eq(&form.username),
                    users::priviledge.eq(mark as i32),
                    users::salt.eq(&salt),
                    users::password.eq(&salted_password),
                ))
                .returning(User::as_returning())
                .get_result(&mut conn);

            match result {
                Ok(u) => {
                    session.clear();
                    if let Err(e) = session.insert("user_id", u.id) {
                        error!("{}", e);
                    } else {
                        info!("Setting cookie for user {}", u.id);
                    }
                    RegisterResponse::Success(u.id)
                }
                Err(e) => {
                    error!("{}", e);
                    return HttpResponse::InternalServerError()
                        .body("Database error! Contact ch-li21@mails.tsinghua.edu.cn.");
                }
            }
        }
        Err(err) => RegisterResponse::Failed(err),
    };

    HttpResponse::Ok().json(response)
}

#[get("/user")]
async fn get_user(session: Session) -> impl Responder {
    if let Some(user_id) = session.get::<i32>("user_id").unwrap() {
        HttpResponse::Ok().body(format!("User ID: {}", user_id))
    } else {
        HttpResponse::Unauthorized().body("No user logged in")
    }
}
