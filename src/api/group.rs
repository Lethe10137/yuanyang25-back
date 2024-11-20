use crate::schema::users;
use crate::util::api_util::*;

use actix_web::{post, web, HttpResponse, Responder};
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl};
use serde::Serialize;

use crate::models::{Team, User};
use crate::DbPool;

use actix_session::Session;

#[derive(Debug, Serialize)]
enum CreateTeamResponse {
    Success { id: i32 },
    AlreadyInTeam { id: i32 },
}

// [[API]]
// desp: Register or update password with token from wechat.
// Method: GET
// URL: /create_team
// Request Body: N/A
// Response Body: `CreateTeamResponse`
//
#[post("/create_team")]
async fn create_team(
    pool: web::Data<DbPool>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "create_team";

    let (user_id, _) = user_privilege_check(&session, PRIVILEGE_MINIMAL)?;
    let mut conn = pool.get().map_err(|_| APIError::ServerError {
        location,
        msg: ERROR_DB_CONNECTION,
    })?;

    let result = conn
        .transaction::<_, APIError, _>(|conn| {
            // Check if user exists
            let user: User = fetch_user_from_id(user_id, conn, &mut session, "create_team")?;

            if let Some(old_team) = user.team {
                Ok(CreateTeamResponse::AlreadyInTeam { id: old_team })
            } else {
                // Create a new team
                use crate::schema::team::dsl as team_dsl;
                let new_team = diesel::insert_into(team_dsl::team)
                    .values(team_dsl::size.eq(1))
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
        .map_err(|e| e.set_location(location))?;

    Ok(HttpResponse::Accepted().json(result))
}
