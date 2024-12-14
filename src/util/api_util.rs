use actix_session::Session;
use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    HttpResponse,
};
use diesel::{pg::Pg, result::Error};

use derive_more::derive::Display;
use diesel::prelude::*;

use crate::{
    models::{Team, User},
    Ext,
};
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

    #[display("Server error at {location}, ref[{refnum}]: {msg}")]
    ServerError {
        location: &'static str,
        msg: &'static str,
        refnum: uuid::Uuid,
    },

    #[display("Try again later")]
    TryAgain,
}

impl APIError {
    pub fn set_location(self, location: &'static str) -> Self {
        match self {
            APIError::ServerError {
                location: _,
                msg,
                refnum,
            } => APIError::ServerError {
                location,
                msg,
                refnum,
            },
            _ => self,
        }
    }

    pub fn log(&self) {
        if let APIError::ServerError {
            location,
            msg,
            refnum,
        } = self
        {
            error!("Server error at {location}, ref[{refnum}]: {msg}");
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
            e => new_unlocated_server_error(e, "Transaction"),
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
                refnum: _,
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

pub fn fetch_user_from_id<M>(user_id: i32, conn: &mut M) -> Result<Option<User>, APIError>
where
    M: Connection<Backend = Pg> + diesel::connection::LoadConnection,
{
    use crate::schema::users::dsl::*;
    match users.filter(id.eq(user_id)).first::<User>(conn) {
        Ok(t) => Ok(Some(t)),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(e) => Err(new_unlocated_server_error(e, ERROR_DB_UNKNOWN)),
    }
}

pub fn fetch_team_from_id<M>(team_id: i32, conn: &mut M) -> Result<Option<Team>, APIError>
where
    M: Connection<Backend = Pg> + diesel::connection::LoadConnection,
{
    use crate::schema::team::dsl::*;
    match team.filter(id.eq(team_id)).first::<Team>(conn) {
        Ok(t) => Ok(Some(t)),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(e) => Err(new_unlocated_server_error(e, ERROR_DB_UNKNOWN)),
    }
}

pub fn log_server_error<E>(error: E, location: &'static str, msg: &'static str) -> APIError
where
    E: derive_more::Display,
{
    new_unlocated_server_error(error, msg)
        .set_location(location)
        .tap(APIError::log)
}

pub fn new_unlocated_server_error<E>(error: E, msg: &'static str) -> APIError
where
    E: derive_more::Display,
{
    let refnum = uuid::Uuid::new_v4();
    error!("Error [{refnum}]: {error}");
    APIError::ServerError {
        location: LOCATION_UNKNOWN,
        msg,
        refnum,
    }
}

pub static SESSION_USER_ID: &str = "user_id";
pub static SESSION_PRIVILEGE: &str = "user_privilege";

pub static ERROR_DB_CONNECTION: &str = "db_connction_failed";
pub static ERROR_SESSION_INSERT: &str = "session_setting_failed";
pub static ERROR_DB_UNKNOWN: &str = "database_unknown";

pub static LOCATION_UNKNOWN: &str = "[unknown]";

pub const PRIVILEGE_MINIMAL: i32 = 0;
pub const PRIVILEGE_STAFF: i32 = 2;
pub const PRIVILEGE_ADMIN: i32 = 4;
