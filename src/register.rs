use crate::api_util::APIRequest;
use crate::schema::users;
use actix_web::{get, post, web, HttpResponse, Responder};
use dotenv::dotenv;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::env;

use diesel::prelude::*;

use crate::cipher_util::DecodeTokenError;
use crate::models::User;
use crate::{cipher_util, schema, DbPool};

use actix_session::{Session, SessionInsertError};
use log::{error, info};

use crate::api_util::{SESSION_PRIVILEDGE, SESSION_USER_ID, SESSION_VERIFY};

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
struct LoginRequest {
    userid: i32,
    auth: AuthMethod,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", content = "data")]
enum AuthMethod {
    Password(String),
    Verification(String),
}

impl APIRequest for LoginRequest {
    fn sanity(self) -> Option<Self> {
        let ok = match &self.auth {
            AuthMethod::Password(pw) => pw.len() == 64,
            AuthMethod::Verification(veri) => veri.len() == 8,
        };

        if ok {
            Some(self)
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum LoginResponse {
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
    priviledge: i32,
) -> Result<(), SessionInsertError> {
    session.insert(SESSION_USER_ID, id)?;
    session.insert(SESSION_PRIVILEDGE, priviledge)?;
    Ok(())
}

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
    mut session: Session,
) -> impl Responder {
    use schema::users;

    let form = sanity!(form);
    let mut conn = pool.get().expect("Failed to get DB connection");

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
                    if let Err(e) = set_loggedin_session(&mut session, u.id, u.priviledge) {
                        internal_error!(e, "login_session");
                    } else {
                        info!("Setting cookie for user {}", u.id);
                    }
                    RegisterResponse::Success(u.id)
                }
                Err(e) => {
                    internal_error!(e, "register_database")
                }
            }
        }
        Err(err) => RegisterResponse::Failed(err),
    };

    HttpResponse::Ok().json(response)
}

// [[API]]
// Description: Login with password.
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
) -> impl Responder {
    let form = sanity!(form);

    let id = form.userid;
    let mut conn = pool.get().expect("Failed to get DB connection");

    let result: LoginResponse = if let Ok(user) = users::table
        .filter(users::id.eq(id))
        .get_result::<User>(&mut conn)
    {
        match form.auth {
            AuthMethod::Password(pw) => {
                if let Some(user) =
                    cipher_util::check_salted_password(&user, pw.as_str(), &LOGIN_TOKEN)
                {
                    session.clear();
                    if let Err(e) = set_loggedin_session(&mut session, user.id, user.priviledge) {
                        internal_error!(e, "login_session");
                    } else {
                        info!("Setting cookie for user {}", user.id);
                    }
                    LoginResponse::Success(user.id)
                } else {
                    LoginResponse::Error
                }
            }
            AuthMethod::Verification(veri) => {
                if let Some(verify_session) = session.get::<String>(SESSION_VERIFY).unwrap() {
                    if cipher_util::verify(
                        user.openid.as_str(),
                        verify_session.as_str(),
                        veri.as_str(),
                    ) {
                        session.clear();
                        if let Err(e) = set_loggedin_session(&mut session, user.id, user.priviledge)
                        {
                            internal_error!(e, "login_session");
                        } else {
                            info!("Setting cookie for user {}", user.id);
                        }
                        LoginResponse::Success(id)
                    } else {
                        LoginResponse::Error
                    }
                } else {
                    LoginResponse::Error
                }
            }
        }
    } else {
        LoginResponse::Error
    };

    HttpResponse::Ok().json(result)
}

// For debug only!
#[get("/user")]
async fn get_user(session: Session) -> impl Responder {
    if let (Ok(Some(user_id)), Ok(Some(user_priveledge))) = (
        session.get::<i32>(SESSION_USER_ID),
        session.get::<i32>(SESSION_PRIVILEDGE),
    ) {
        HttpResponse::Ok().body(format!(
            "Priveledge {}, User id {}",
            user_priveledge, user_id
        ))
    } else {
        HttpResponse::Unauthorized().body("No user logged in")
    }
}
