use std::ops::DerefMut;

use actix_session::Session;
use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    HttpResponse,
};
use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use diesel::result::Error;

use derive_more::derive::Display;
use diesel::prelude::*;

use crate::{
    models::{PuzzleId, Team, TeamId, User, WaPenalty},
    util::economy::time_allowance,
    DbPool, Ext,
};
use log::error;

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

    #[display("Insufficient Token")]
    InsufficientTokens,

    #[display("Unauthorized access")]
    Unauthorized,

    #[display("Transaction cancelled, balance matained as {balance}")]
    TransactionCancel { balance: i64 },

    #[display("Server error at {location}, ref[{refnum}]: {msg}")]
    ServerError {
        location: &'static str,
        msg: &'static str,
        refnum: uuid::Uuid,
    },
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
        new_unlocated_server_error(e, "Transaction")
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
    privilege_needed: i32,
    location: &'static str,
) -> Result<i32, APIError> {
    if let Ok(Some(team_id)) = session.get::<i32>(SESSION_TEAM_ID) {
        return Ok(team_id);
    }
    let (user_id, _) = user_privilege_check(session, privilege_needed)?;

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

pub async fn fetch_balance<C>(team_id: TeamId, conn: &mut C) -> Result<Option<i64>, APIError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    use crate::schema::team::dsl::*;

    match team
        .filter(id.eq(team_id))
        .select(token_balance)
        .first::<i64>(conn)
        .await
    {
        Ok(t) => Ok(Some(t + time_allowance())),
        Err(Error::NotFound) => Ok(None),
        Err(e) => Err(new_unlocated_server_error(e, ERROR_DB_UNKNOWN)),
    }
}

static TIME_PENALTY: [i64; 11] = [10, 60, 120, 120, 120, 240, 480, 480, 480, 480, 600]; // in seconds
static TOKEN_PENALTY: [i64; 11] = [30, 40, 50, 60, 70, 80, 90, 100, 200, 400, 500];

impl Default for WaPenalty {
    fn default() -> Self {
        Self::new()
    }
}

impl WaPenalty {
    pub fn new() -> Self {
        Self {
            time_penalty_until: Utc.timestamp_opt(1, 0).unwrap(),
            token_penalty_level: 0,
            time_penalty_level: 0,
        }
    }

    pub fn on_wrong_answer(&mut self) -> i64 {
        let time_penalty = TIME_PENALTY
            .get(self.time_penalty_level as usize)
            .or(TIME_PENALTY.last())
            .cloned()
            .unwrap_or(600);
        let token_penalty = TOKEN_PENALTY
            .get(self.token_penalty_level as usize)
            .or(TOKEN_PENALTY.last())
            .cloned()
            .unwrap_or(500);
        self.token_penalty_level += 1;
        self.time_penalty_level += 1;
        self.time_penalty_until = Utc::now() + TimeDelta::seconds(time_penalty);
        token_penalty
    }

    pub fn on_new_mid_answer(self) -> Self {
        Self {
            token_penalty_level: 0,
            ..self
        }
    }
}

pub fn check_is_after(to_check: DateTime<Utc>, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    (now <= to_check).then_some(to_check)
}

pub async fn fetch_wa_cnt<C>(
    puzzle_id: PuzzleId,
    team_id: TeamId,
    conn: &mut C,
) -> Result<Option<WaPenalty>, APIError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    use crate::schema::wrong_answer_cnt::dsl::*;

    match wrong_answer_cnt
        .filter(team.eq(team_id).and(puzzle.eq(puzzle_id)))
        .select((time_penalty_until, token_penalty_level, time_penalty_level))
        .first::<(DateTime<Utc>, i32, i32)>(conn)
        .await
    {
        Ok((time_penalty_until_, token_penalty_level_, time_penalty_level_)) => {
            Ok(Some(WaPenalty {
                time_penalty_until: time_penalty_until_,
                token_penalty_level: token_penalty_level_,
                time_penalty_level: time_penalty_level_,
            }))
        }
        Err(Error::NotFound) => Ok(None),
        Err(e) => Err(new_unlocated_server_error(e, ERROR_DB_UNKNOWN)),
    }
}

pub async fn count_passed<C>(team_id: TeamId, conn: &mut C) -> Result<usize, APIError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    use crate::schema::submission::dsl::*;

    submission
        .filter(team.eq(team_id))
        .count()
        .execute(conn)
        .await
        .map_err(|e| new_unlocated_server_error(e, ERROR_DB_UNKNOWN))
}

pub async fn insert_or_update_wa_cnt<C>(
    puzzle_id: PuzzleId,
    team_id: TeamId,
    data: WaPenalty,
    conn: &mut C,
) -> Result<(), APIError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    use crate::schema::wrong_answer_cnt::dsl::*;
    diesel::insert_into(wrong_answer_cnt)
        .values((
            team.eq(team_id),
            puzzle.eq(puzzle_id),
            token_penalty_level.eq(data.token_penalty_level),
            time_penalty_level.eq(data.time_penalty_level),
            time_penalty_until.eq(data.time_penalty_until),
        ))
        .on_conflict((team, puzzle))
        .do_update()
        .set((
            token_penalty_level.eq(data.token_penalty_level),
            time_penalty_level.eq(data.time_penalty_level),
            time_penalty_until.eq(data.time_penalty_until),
        ))
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

pub fn allow_err<T>(old: Result<T, APIError>, allow: APIError) -> Result<Option<T>, APIError> {
    match old {
        Err(e) if e == allow => Ok(None),
        Ok(i) => Ok(Some(i)),
        Err(e) => Err(e),
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
