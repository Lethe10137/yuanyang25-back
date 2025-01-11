use crate::schema::users;
use crate::util::api_util::*;
use crate::VERICODE_LENGTH;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use actix_web::{get, post, web, HttpResponse, Responder};
use dotenv::dotenv;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::env;

use crate::models::User;
use crate::util::cipher_util::DecodeTokenError;
use crate::{schema, util::cipher_util, DbPool};

use actix_session::Session;

use crate::util::api_util::{ERROR_DB_UNKNOWN, SESSION_PRIVILEGE, SESSION_USER_ID};

#[derive(Debug, Deserialize)]
struct RegisterRequest {
    // Max 100.
    username: String,
    token: String,
    // SHA256 of the password.
    password: String,
}

impl APIRequest for RegisterRequest {
    fn ok(&self) -> bool {
        self.username.len() <= 100 && self.password.len() == 64 && self.token.len() == 128
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
struct LoginRequest {
    userid: i32,
    auth: AuthMethod,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", content = "data")]
enum AuthMethod {
    Password(String),
    Totp(String),
}

impl APIRequest for LoginRequest {
    fn ok(&self) -> bool {
        match &self.auth {
            AuthMethod::Password(pw) => pw.len() == 64,
            AuthMethod::Totp(veri) => veri.len() == VERICODE_LENGTH,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum LoginResponse {
    //Returns the user id
    Success(i32),
    Error,
}

static LOGIN_TOKEN: Lazy<String> = Lazy::new(|| {
    dotenv().ok();
    env::var("LOGIN_TOKEN").expect("Environment variable LOGIN_TOKEN not set")
});

static REGISTER_TOKEN: Lazy<String> = Lazy::new(|| {
    dotenv().ok();
    env::var("REGISTER_TOKEN").expect("Environment variable REGISTER_TOKEN not set")
});

fn set_loggedin_session(
    session: &mut Session,
    id: i32,
    privilege: i32,
    location: &'static str,
) -> Result<(), APIError> {
    session
        .insert(SESSION_USER_ID, id)
        .map_err(|e| log_server_error(e, location, ERROR_SESSION_INSERT))?;
    session
        .insert(SESSION_PRIVILEGE, privilege)
        .map_err(|e| log_server_error(e, location, ERROR_SESSION_INSERT))?;
    Ok(())
}

// [[API]]
// desp: Register or update password with token from wechat.
// Method: Post
// URL: /register
// Request Body: `RegisterRequest`
// Response Body: `RegisterResponse`
//
#[post("/register")]
pub async fn register_user(
    pool: web::Data<DbPool>,
    form: web::Json<RegisterRequest>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    use schema::users;

    let location = "register";
    form.sanity()?;
    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let response = match cipher_util::decode_token(form.token.as_str(), REGISTER_TOKEN.as_str()) {
        Ok((_version, mark, openid)) => {
            let (salt, salted_password) =
                cipher_util::gen_salted_password(&form.password, &LOGIN_TOKEN);

            let user: User = diesel::insert_into(users::table)
                .values((
                    users::username.eq(&form.username),
                    users::openid.eq(openid.as_str()),
                    users::privilege.eq(mark as i32),
                    users::salt.eq(&salt),
                    users::password.eq(&salted_password),
                ))
                .on_conflict(users::openid)
                .do_update()
                .set((
                    users::username.eq(&form.username),
                    users::privilege.eq(mark as i32),
                    users::salt.eq(&salt),
                    users::password.eq(&salted_password),
                ))
                .returning(User::as_returning())
                .get_result(&mut conn)
                .await
                .map_err(|e| log_server_error(e, location, ERROR_DB_UNKNOWN))?;

            session.clear();
            set_loggedin_session(&mut session, user.id, user.privilege, "register")?;
            RegisterResponse::Success(user.id)
        }
        Err(err) => RegisterResponse::Failed(err),
    };
    Ok(HttpResponse::Ok().json(response))
}

// [[API]]
// desp: Login with password.
// Method: Post
// URL: /login
// Request Body: `LoginRequest`
// Response Body: `LoginResponse`
//
#[post("/login")]
async fn login_user(
    pool: web::Data<DbPool>,
    form: web::Json<LoginRequest>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "login";
    form.sanity()?;
    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let id = form.userid;

    let result: LoginResponse = if let Ok(user) = users::table
        .filter(users::id.eq(id))
        .get_result::<User>(&mut conn)
        .await
    {
        match &form.auth {
            AuthMethod::Password(pw) => {
                if let Some(user) =
                    cipher_util::check_salted_password(&user, pw.as_str(), &LOGIN_TOKEN)
                {
                    session.clear();
                    set_loggedin_session(&mut session, user.id, user.privilege, "login_password")?;

                    LoginResponse::Success(user.id)
                } else {
                    LoginResponse::Error
                }
            }
            AuthMethod::Totp(veri) => {
                if cipher_util::verify_totp(user.openid.as_str(), veri.as_str()) {
                    session.clear();
                    set_loggedin_session(&mut session, user.id, user.privilege, "login_totp")?;
                    LoginResponse::Success(id)
                } else {
                    LoginResponse::Error
                }
            }
        }
    } else {
        LoginResponse::Error
    };

    Ok(HttpResponse::Ok().json(result))
}

// For debug only!
#[get("/user")]
async fn get_user(session: Session) -> impl Responder {
    if let (Ok(Some(user_id)), Ok(Some(user_privilege))) = (
        session.get::<i32>(SESSION_USER_ID),
        session.get::<i32>(SESSION_PRIVILEGE),
    ) {
        HttpResponse::Ok().body(format!("Privilege {}, User id {}", user_privilege, user_id))
    } else {
        HttpResponse::Unauthorized().body("No user logged in")
    }
}
