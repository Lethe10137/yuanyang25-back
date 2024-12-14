use crate::schema::{team, users};
use crate::util::{api_util::*, cipher_util};

use actix_web::{get, post, web, HttpResponse, Responder};
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl};
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

// [[API]]
// desp: Create a team.
// Method: GET
// URL: /create_team
// Request Body: N/A
// Response Body: `CreateTeamResponse`
#[post("/create_team")]
async fn create_team(
    pool: web::Data<DbPool>,
    session: Session,
) -> Result<impl Responder, APIError> {
    let location = "create_team";

    let (user_id, _) = user_privilege_check(&session, PRIVILEGE_MINIMAL)?;

    let mut conn = pool
        .get()
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let result = conn
        .transaction::<_, APIError, _>(|conn| {
            // Check if user exists

            let user = fetch_user_from_id(user_id, conn)?
                .ok_or(APIError::InvalidSession)
                .inspect_err(|_| session.clear())?;

            if let Some(old_team) = user.team {
                Ok(CreateTeamResponse::AlreadyInTeam { id: old_team })
            } else {
                // Create a new team
                use crate::schema::team::dsl as team_dsl;
                let new_team = diesel::insert_into(team_dsl::team)
                    .values((
                        team_dsl::size.eq(1),
                        team_dsl::salt.eq(hex::encode(cipher_util::get_salt::<32>())),
                    )) // 32 * 8 = 256 Bits salt encoded into 64 hexdecimal digits
                    .get_result::<Team>(conn)
                    .map_err(|e| log_server_error(e, location, ERROR_DB_UNKNOWN))?;

                // Update the user's team reference
                diesel::update(users::table.filter(users::id.eq(user_id)))
                    .set(users::team.eq(Some(new_team.id)))
                    .execute(conn)
                    .map_err(|e| log_server_error(e, location, ERROR_DB_UNKNOWN))?;

                Ok(CreateTeamResponse::Success { id: new_team.id })
            }
        })
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
async fn team_veri(pool: web::Data<DbPool>, session: Session) -> Result<impl Responder, APIError> {
    let location = "team_veri";

    let (user_id, _) = user_privilege_check(&session, PRIVILEGE_MINIMAL)?;

    let mut conn = pool
        .get()
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let result = conn
        .transaction::<_, APIError, _>(|conn| {
            let user = fetch_user_from_id(user_id, conn)?
                .ok_or(APIError::InvalidSession)
                .inspect_err(|_| session.clear())?;

            match user
                .team
                .map(|team_id| fetch_team_from_id(team_id, conn))
                .transpose()?
                .flatten()
            {
                Some(team) => Ok(TeamTOTPResponse::Success {
                    id: team.id,
                    totp: cipher_util::gen_totp(team.salt.as_str())
                        .tap_mut(|code| code.truncate(VERICODE_LENGTH)),
                }),
                None => Ok(TeamTOTPResponse::NotInTeam),
            }
        })
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
    session: Session,
) -> Result<impl Responder, APIError> {
    let location = "join_team";
    form.sanity()?;
    let (user_id, user_priv) = user_privilege_check(&session, PRIVILEGE_MINIMAL)?;
    let mut conn = pool
        .get()
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;
    let team_id = form.team_id;

    let result = conn
        .transaction::<_, APIError, _>(|conn| {
            let user = fetch_user_from_id(user_id, conn)?
                .ok_or(APIError::InvalidSession)
                .inspect_err(|_| session.clear())?;

            let team_to_join = fetch_team_from_id(team_id, conn)?;

            match (user, team_to_join) {
                (user, Some(team)) => {
                    if user.team.is_some() {
                        Ok(JoinTeamResponse::AlreadyInTeam)
                    } else if team.max_size <= team.size {
                        Ok(JoinTeamResponse::TeamFull)
                    } else if team.is_staff && user_priv < PRIVILEGE_STAFF {
                        info!("user priv {user_priv} too low");
                        Ok(JoinTeamResponse::AuthError)
                    } else if cipher_util::verify_totp(team.salt.as_str(), &form.vericode) {
                        // Update the user's team reference
                        diesel::update(users::table.filter(users::id.eq(user_id)))
                            .set(users::team.eq(Some(team_id)))
                            .execute(conn)
                            .map_err(|e| log_server_error(e, location, ERROR_DB_UNKNOWN))?;

                        // Update the team's size
                        diesel::update(team::table.filter(team::id.eq(team_id)))
                            .set(team::size.eq(team.size + 1))
                            .execute(conn)
                            .map_err(|e| log_server_error(e, location, ERROR_DB_UNKNOWN))?;

                        Ok(JoinTeamResponse::Success { id: team_id })
                    } else {
                        info!("veri {}", form.vericode);
                        Ok(JoinTeamResponse::AuthError)
                    }
                }
                _ => Ok(JoinTeamResponse::AuthError),
            }
        })
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
async fn exit_team(pool: web::Data<DbPool>, session: Session) -> Result<impl Responder, APIError> {
    let location = "exit_team";

    let (user_id, _) = user_privilege_check(&session, PRIVILEGE_MINIMAL)?;

    let mut conn = pool
        .get()
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let result = conn
        .transaction::<_, APIError, _>(|conn| {
            let user = fetch_user_from_id(user_id, conn)?
                .ok_or(APIError::InvalidSession)
                .inspect_err(|_| session.clear())?;

            match user
                .team
                .map(|team_id| fetch_team_from_id(team_id, conn))
                .transpose()?
                .flatten()
            {
                Some(team) if !team.confirmed => {
                    // Update the user's team reference
                    diesel::update(users::table.filter(users::id.eq(user_id)))
                        .set(users::team.eq::<Option<i32>>(None))
                        .execute(conn)
                        .map_err(|e| log_server_error(e, location, ERROR_DB_UNKNOWN))?;

                    // Update the team's size
                    diesel::update(team::table.filter(team::id.eq(team.id)))
                        .set(team::size.eq(team.size - 1))
                        .execute(conn)
                        .map_err(|e| log_server_error(e, location, ERROR_DB_UNKNOWN))?;

                    Ok(ExitTeamResponse::Success { id: team.id })
                }
                Some(_) => Ok(ExitTeamResponse::NotAllowed),
                None => Ok(ExitTeamResponse::NotInTeam),
            }
        })
        .map_err(|e| e.set_location(location).tap(APIError::log))?;

    Ok(HttpResponse::Ok().json(result))
}
