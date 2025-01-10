use std::ops::DerefMut;

use actix_session::Session;
use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    HttpResponse,
};
use chrono::{DateTime, Utc};
use diesel::result::Error;

use derive_more::derive::Display;
use diesel::prelude::*;

use crate::{
    models::{Puzzle, PuzzleId, Team, TeamId, User},
    DbPool, Ext,
};
use log::{error, info};

use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;

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

#[derive(Debug, Display, PartialEq, Eq)]
pub enum APIError {
    #[display("Invalid form data")]
    InvalidFormData,

    #[display("Invalid query")]
    InvalidQuery,

    #[display("Invalid session")]
    InvalidSession,

    #[display("Not logged in")]
    NotLogin,

    #[display("Not in a team")]
    NotInTeam,

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
        session.get::<i32>(SESSION_USER_ID),
        session.get::<i32>(SESSION_PRIVILEGE),
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

pub async fn get_team_id(
    session: &mut Session,
    pool: &DbPool,
    location: &'static str,
) -> Result<i32, APIError> {
    if let Ok(Some(team_id)) = session.get::<i32>(SESSION_TEAM_ID) {
        info!("team {} cached from session", team_id);
        return Ok(team_id);
    }
    let (user_id, _) = user_privilege_check(session, PRIVILEGE_MINIMAL)?;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let user = fetch_user_from_id(user_id, &mut conn)
        .await?
        .ok_or(APIError::InvalidSession)
        .inspect_err(kill_session(session))
        .map_err(|e| e.set_location(location).tap(APIError::log))?;

    if let Some(team_id) = user.team {
        if check_confirmed_from_team_id(team_id, &mut conn)
            .await?
            .is_some_and(|confirmed| confirmed)
        {
            session.insert(SESSION_TEAM_ID, team_id).ok();
            info!("user {} team {} cached in session", user_id, team_id);
        } else {
            info!("user {} team {} not confirmed", user_id, team_id);
        }
        Ok(team_id)
    } else {
        Err(APIError::NotInTeam)
    }
}

pub async fn fetch_user_from_id<C>(user_id: i32, conn: &mut C) -> Result<Option<User>, APIError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    use crate::schema::users::dsl::*;

    match users.filter(id.eq(user_id)).first::<User>(conn).await {
        Ok(user) => Ok(Some(user)),
        Err(Error::NotFound) => Ok(None),
        Err(e) => Err(new_unlocated_server_error(e, ERROR_DB_UNKNOWN)),
    }
}

pub async fn fetch_team_from_id<C>(team_id: i32, conn: &mut C) -> Result<Option<Team>, APIError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    use crate::schema::team::dsl::*;

    match team.filter(id.eq(team_id)).first::<Team>(conn).await {
        Ok(t) => Ok(Some(t)),
        Err(Error::NotFound) => Ok(None),
        Err(e) => Err(new_unlocated_server_error(e, ERROR_DB_UNKNOWN)),
    }
}

pub async fn check_confirmed_from_team_id<C>(
    team_id: i32,
    conn: &mut C,
) -> Result<Option<bool>, APIError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    use crate::schema::team::dsl::*;

    match team
        .filter(id.eq(team_id))
        .select(confirmed)
        .first::<bool>(conn)
        .await
    {
        Ok(t) => Ok(Some(t)),
        Err(Error::NotFound) => Ok(None),
        Err(e) => Err(new_unlocated_server_error(e, ERROR_DB_UNKNOWN)),
    }
}

pub async fn fetch_puzzle_from_id<C>(puzzle_id: i32, conn: &mut C) -> Result<Puzzle, APIError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    use crate::schema::puzzle::dsl::*;

    match puzzle
        .filter(id.eq(puzzle_id))
        .select((unlock, bounty, title, answer, key))
        .first::<Puzzle>(conn)
        .await
    {
        Ok(p) => Ok(p),
        Err(Error::NotFound) => Err(APIError::InvalidQuery),
        Err(e) => Err(new_unlocated_server_error(e, ERROR_DB_UNKNOWN)),
    }
}

pub async fn fetch_unlock_time<C>(
    puzzle_id: PuzzleId,
    team_id: TeamId,
    conn: &mut C,
) -> Result<Option<DateTime<Utc>>, APIError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    use crate::schema::unlock::dsl::*;

    match unlock
        .filter(team.eq(team_id).and(puzzle.eq(puzzle_id)))
        .select(time)
        .first::<DateTime<Utc>>(conn)
        .await
    {
        Ok(t) => Ok(Some(t)),
        Err(Error::NotFound) => Ok(None),
        Err(e) => Err(new_unlocated_server_error(e, ERROR_DB_UNKNOWN)),
    }
}

pub async fn insert_or_update_unlock_time<C>(
    puzzle_id: PuzzleId,
    team_id: TeamId,
    new_time: DateTime<Utc>,
    conn: &mut C,
) -> Result<(), APIError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    use crate::schema::unlock::dsl::*;

    diesel::insert_into(unlock)
        .values((team.eq(team_id), puzzle.eq(puzzle_id), time.eq(new_time)))
        .on_conflict((team, puzzle))
        .do_update()
        .set(time.eq(new_time))
        .execute(conn)
        .await
        .map_err(|e| new_unlocated_server_error(e, ERROR_DB_UNKNOWN))?;

    Ok(())
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

pub fn handle_session<T>(session: &mut Session) -> impl FnMut((T, bool)) -> T + '_ {
    |(value, kill_session)| {
        if kill_session {
            session.clear();
        }
        value
    }
}

pub fn kill_session(session: &mut Session) -> impl FnMut(&APIError) + '_ {
    |result| {
        if result == &APIError::InvalidSession {
            session.clear()
        };
    }
}

pub static SESSION_USER_ID: &str = "user_id";
pub static SESSION_PRIVILEGE: &str = "user_privilege";
pub static SESSION_TEAM_ID: &str = "team_id";

pub static ERROR_DB_CONNECTION: &str = "db_connction_failed";
pub static ERROR_SESSION_INSERT: &str = "session_setting_failed";
pub static ERROR_DB_UNKNOWN: &str = "database_unknown";

pub static LOCATION_UNKNOWN: &str = "[unknown]";

pub const PRIVILEGE_MINIMAL: i32 = 0;
pub const PRIVILEGE_STAFF: i32 = 2;
pub const PRIVILEGE_ADMIN: i32 = 4;
