use actix_session::Session;
use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    HttpResponse,
};
use diesel::{pg::Pg, result::Error};

use derive_more::derive::Display;
use diesel::prelude::*;

use crate::models::User;
use log::{error, info};

pub trait APIRequest: Sized {
    fn ok(&self) -> bool;
    fn sanity(&self) -> Result<(), APIError> {
        if self.ok() {
            Ok(())
        } else {
            Err(APIError::InvalidFormData)
        }
    }
}

#[derive(Debug, Display)]
pub enum APIError {
    #[display("Invalid form data")]
    InvalidFormData,

    #[display("Invalid session")]
    InvalidSession,

    #[display("Not logged in")]
    NotLogin,

    #[display("Unauthorized access")]
    Unauthorized,

    #[display("Server error at {location}: {msg}")]
    ServerError {
        location: &'static str,
        msg: &'static str,
    },

    #[display("Try again later")]
    TryAgain,
}

impl APIError {
    pub fn set_location(self, location: &'static str) -> Self {
        match self {
            APIError::ServerError { location: _, msg } => APIError::ServerError { location, msg },
            _ => self,
        }
    }
}

impl From<Error> for APIError {
    fn from(e: Error) -> Self {
        match e {
            Error::AlreadyInTransaction => {
                info!("Already in Transaction.");
                APIError::TryAgain
            }
            e => {
                error!("Error in Transaction: {}", &e);
                APIError::ServerError {
                    location: "",
                    msg: ERROR_DB_UNKNOWN,
                }
            }
        }
    }
}

impl error::ResponseError for APIError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            APIError::InvalidFormData => StatusCode::NOT_ACCEPTABLE,
            APIError::ServerError {
                location: _,
                msg: _,
            } => StatusCode::INTERNAL_SERVER_ERROR,
            APIError::TryAgain => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}

pub fn user_privilege_check(session: &Session, require: i32) -> Result<(i32, i32), APIError> {
    if let (Ok(Some(user_id)), Ok(Some(user_privilege))) = (
        session.get::<i32>(crate::util::api_util::SESSION_USER_ID),
        session.get::<i32>(crate::util::api_util::SESSION_PRIVILEGE),
    ) {
        if user_privilege >= require {
            Ok((user_id, user_privilege))
        } else {
            Err(APIError::Unauthorized)
        }
    } else {
        Err(APIError::NotLogin)
    }
}

pub fn fetch_user_from_id<M>(
    user_id: i32,
    conn: &mut M,
    session: &mut Session,
    location: &'static str,
) -> Result<User, APIError>
where
    M: Connection<Backend = Pg> + diesel::connection::LoadConnection,
{
    use crate::schema::users::dsl::*;
    users
        .filter(id.eq(user_id))
        .first::<User>(conn)
        .map_err(|err| match err {
            diesel::result::Error::NotFound => {
                session.clear();
                APIError::InvalidSession
            }
            e => {
                error!("fetch_user() at {}, {}", location, e);
                APIError::ServerError {
                    location,
                    msg: ERROR_DB_UNKNOWN,
                }
            }
        })
}

pub fn log_server_error<E>(error: E, location: &'static str, msg: &'static str) -> APIError
where
    E: derive_more::Display,
{
    error!("[{}]:{}", location, error);
    APIError::ServerError { location, msg }
}

pub static SESSION_USER_ID: &str = "user_id";
pub static SESSION_PRIVILEGE: &str = "user_privilege";

pub static ERROR_DB_CONNECTION: &str = "db_connction_failed";
pub static ERROR_SESSION_INSERT: &str = "session_setting_failed";
pub static ERROR_DB_UNKNOWN: &str = "database_unknown";

pub const PRIVILEGE_MINIMAL: i32 = 0;
pub const PRIVILEGE_STAFF: i32 = 2;
pub const PRIVILEGE_ADMIN: i32 = 4;
