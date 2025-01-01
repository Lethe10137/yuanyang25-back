use crate::schema::{team, users};
use crate::util::{api_util::*, cipher_util};

use actix_web::{get, post, web, HttpResponse, Responder};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::AsyncConnection;
use diesel_async::RunQueryDsl;
use log::info;
use serde::{Deserialize, Serialize};

use crate::models::Team;
use crate::{DbPool, Ext, VERICODE_LENGTH};

use actix_session::Session;


#[derive(Debug, Serialize)]
enum CreateTeamResponse {
    Success { id: i32 },
    AlreadyInTeam { id: i32 },
}

fn handle_session<T>(session: &mut Session) -> impl FnMut((T, bool)) -> T + '_ {
    |(value, kill_session)| {
        if kill_session {
            session.clear();
        }
        value
    }
}

// [[API]]
// desp: Create a team.
// Method: GET
// URL: /create_team
// Request Body: N/A
// Response Body: `CreateTeamResponse`
#[post("/create_team")]
async fn create_team(
    pool: web::Data<DbPool>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "create_team";

    let (user_id, _) = user_privilege_check(&session, PRIVILEGE_MINIMAL)?;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let result = conn
        .transaction::<_, APIError, _>(|conn| {
            Box::pin(async move {
                // Check if user exists
                let mut kill_session = false;

                let user = fetch_user_from_id(user_id, conn)
                    .await?
                    .ok_or(APIError::InvalidSession)
                    .inspect_err(|_| kill_session = true)?;

                if let Some(old_team) = user.team {
                    Ok((
                        CreateTeamResponse::AlreadyInTeam { id: old_team },
                        kill_session,
                    ))
                } else {
                    // Create a new team
                    use crate::schema::team::dsl as team_dsl;
                    let new_team = diesel::insert_into(team_dsl::team)
                        .values((
                            team_dsl::size.eq(1),
                            team_dsl::salt.eq(hex::encode(cipher_util::get_salt::<32>())),
                        )) // 32 * 8 = 256 Bits salt encoded into 64 hexdecimal digits
                        .get_result::<Team>(conn)
                        .await
                        .map_err(|e| log_server_error(e, location, ERROR_DB_UNKNOWN))?;

                    // Update the user's team reference
                    diesel::update(users::table.filter(users::id.eq(user_id)))
                        .set(users::team.eq(Some(new_team.id)))
                        .execute(conn)
                        .await
                        .map_err(|e| log_server_error(e, location, ERROR_DB_UNKNOWN))?;

                    Ok((
                        CreateTeamResponse::Success { id: new_team.id },
                        kill_session,
                    ))
                }
            })
        })
        .await
        .map(handle_session(&mut session))
        .map_err(|e| e.set_location(location).tap(APIError::log))?;

    Ok(HttpResponse::Ok().json(result))
}

#[derive(Debug, Serialize)]
enum TeamTOTPResponse {
    Success { id: i32, totp: String },
    NotInTeam,
}

// [[API]]
// desp: Return the verification code for joinning in a team.
// Method: GET
// URL: /team_veri
// Request Body: N/A
// Response Body: `TeamTOTPResponse`
#[get("/team_veri")]
async fn team_veri(
    pool: web::Data<DbPool>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "team_veri";

    let (user_id, _) = user_privilege_check(&session, PRIVILEGE_MINIMAL)?;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let result = conn
        .transaction::<_, APIError, _>(|conn| {
            Box::pin(async move {
                let mut kill_session = false;
                let user = fetch_user_from_id(user_id, conn)
                    .await?
                    .ok_or(APIError::InvalidSession)
                    .inspect_err(|_| kill_session = true)?;

                match if let Some(team_id) = user.team {
                    fetch_team_from_id(team_id, conn).await?
                } else {
                    None
                } {
                    Some(team) => Ok((
                        TeamTOTPResponse::Success {
                            id: team.id,
                            totp: cipher_util::gen_totp(team.salt.as_str())
                                .tap_mut(|code| code.truncate(VERICODE_LENGTH)),
                        },
                        kill_session,
                    )),
                    None => Ok((TeamTOTPResponse::NotInTeam, kill_session)),
                }
            })
        })
        .await
        .map(handle_session(&mut session))
        .map_err(|e| e.set_location(location).tap(APIError::log))?;

    Ok(HttpResponse::Ok().json(result))
}

#[derive(Debug, Deserialize)]
struct JoinTeamRequest {
    team_id: i32,
    vericode: String,
}

impl APIRequest for JoinTeamRequest {
    fn ok(&self) -> bool {
        self.team_id >= 0 && self.vericode.len() == VERICODE_LENGTH
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum JoinTeamResponse {
    Success { id: i32 },
    AlreadyInTeam,
    TeamFull,
    AuthError,
}

// [[API]]
// desp: Return the verification code for joinning in a team.
// Method: POST
// URL: /join_team
// Request Body: N/A
// Response Body: `TeamTOTPResponse`
#[post("/join_team")]
async fn join_team(
    pool: web::Data<DbPool>,
    form: web::Json<JoinTeamRequest>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "join_team";
    form.sanity()?;
    let (user_id, user_priv) = user_privilege_check(&session, PRIVILEGE_MINIMAL)?;
    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let result = conn
        .transaction::<_, APIError, _>(|conn| {
            Box::pin(async move {
                let team_id = form.team_id;
                let mut kill_session = false;
                let user = fetch_user_from_id(user_id, conn)
                    .await?
                    .ok_or(APIError::InvalidSession)
                    .inspect_err(|_| kill_session = true)?;

                let team_to_join = fetch_team_from_id(team_id, conn).await?;

                match (user, team_to_join) {
                    (user, Some(team)) => {
                        if user.team.is_some() {
                            Ok((JoinTeamResponse::AlreadyInTeam, kill_session))
                        } else if team.max_size <= team.size {
                            Ok((JoinTeamResponse::TeamFull, kill_session))
                        } else if team.is_staff && user_priv < PRIVILEGE_STAFF {
                            info!("user priv {user_priv} too low");
                            Ok((JoinTeamResponse::AuthError, kill_session))
                        } else if cipher_util::verify_totp(team.salt.as_str(), &form.vericode) {
                            // Update the user's team reference
                            diesel::update(users::table.filter(users::id.eq(user_id)))
                                .set(users::team.eq(Some(team_id)))
                                .execute(conn)
                                .await
                                .map_err(|e| log_server_error(e, location, ERROR_DB_UNKNOWN))?;

                            // Update the team's size
                            diesel::update(team::table.filter(team::id.eq(team_id)))
                                .set(team::size.eq(team.size + 1))
                                .execute(conn)
                                .await
                                .map_err(|e| log_server_error(e, location, ERROR_DB_UNKNOWN))?;

                            Ok((JoinTeamResponse::Success { id: team_id }, kill_session))
                        } else {
                            info!("veri {}", form.vericode);
                            Ok((JoinTeamResponse::AuthError, kill_session))
                        }
                    }
                    _ => Ok((JoinTeamResponse::AuthError, kill_session)),
                }
            })
        })
        .await
        .map(handle_session(&mut session))
        .map_err(|e| e.set_location(location).tap(APIError::log))?;

    Ok(HttpResponse::Ok().json(result))
}

#[derive(Debug, Serialize)]
enum ExitTeamResponse {
    Success { id: i32 },
    NotInTeam,
    NotAllowed,
}

// [[API]]
// desp: Return the verification code for joinning in a team.
// Method: POST
// URL: /exit_team
// Request Body: N/A
// Response Body: `ExitTeamResponse`
#[post("/exit_team")]
async fn exit_team(pool: web::Data<DbPool>, mut session: Session) -> Result<impl Responder, APIError> {
    let location = "exit_team";

    let (user_id, _) = user_privilege_check(&session, PRIVILEGE_MINIMAL)?;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let result = conn
        .transaction::<_, APIError, _>(|conn| {
            Box::pin(async move {
                let mut kill_session = false;
                let user = fetch_user_from_id(user_id, conn)
                    .await?
                    .ok_or(APIError::InvalidSession)
                    .inspect_err(|_| kill_session = true)?;

                match if let Some(team_id) = user.team {
                    fetch_team_from_id(team_id, conn).await?
                } else {
                    None
                } {
                    Some(team) if !team.confirmed => {
                        // Update the user's team reference
                        diesel::update(users::table.filter(users::id.eq(user_id)))
                            .set(users::team.eq::<Option<i32>>(None))
                            .execute(conn).await
                            .map_err(|e| log_server_error(e, location, ERROR_DB_UNKNOWN))?;

                        // Update the team's size
                        diesel::update(team::table.filter(team::id.eq(team.id)))
                            .set(team::size.eq(team.size - 1))
                            .execute(conn).await
                            .map_err(|e| log_server_error(e, location, ERROR_DB_UNKNOWN))?;

                        Ok((ExitTeamResponse::Success { id: team.id }, kill_session))
                    }
                    Some(_) => Ok((ExitTeamResponse::NotAllowed, kill_session)),
                    None => Ok((ExitTeamResponse::NotInTeam, kill_session)),
                }
            })
        })
        .await
        .map(handle_session(&mut session))
        .map_err(|e| e.set_location(location).tap(APIError::log))?;

    Ok(HttpResponse::Ok().json(result))
}
