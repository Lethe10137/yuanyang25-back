use std::sync::Arc;

use actix_session::Session;
use actix_web::{get, post, web, HttpResponse, Responder};

use diesel::prelude::*;

use crate::models::*;
use crate::util::economy::compulsory_team_balance;

use diesel_async::{AsyncConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};

use crate::util::api_util::{
    find_min_active_id, get_oracle_by_id, get_oracle_by_id_and_team,
    get_oracles_by_team_and_puzzle, get_oracles_from_id, update_active_oracle_and_return_team,
    user_privilege_check, PRIVILEGE_STAFF,
};
use crate::{
    util::{
        api_util::{
            get_team_id, log_server_error, APIError, APIRequest, ERROR_DB_CONNECTION,
            PRIVILEGE_MINIMAL,
        },
        cache::Cache,
        economy::{oracle_price, try_modify_team_balance},
    },
    DbPool, Ext,
};

#[derive(Debug, Deserialize)]
struct CreateOracleRequest {
    puzzle_id: i32,
    content: String,
}

const ORACLE_LENGTH_LIMIT_BYTES: usize = 700;

impl APIRequest for CreateOracleRequest {
    fn ok(&self) -> bool {
        self.puzzle_id >= 0 && self.content.len() <= ORACLE_LENGTH_LIMIT_BYTES
    }
}

#[derive(Debug, Serialize)]
enum CreateOracleResponse {
    TooManyActiveOracle,
    Sucess {
        oracle_id: i32,
        cost: i64,
        new_balance: i64,
    }, // returns the id
}

#[post("/create_oracle")]
async fn create_oracle(
    pool: web::Data<Arc<DbPool>>,
    cache: web::Data<Arc<Cache>>,
    form: web::Json<CreateOracleRequest>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "create_oracle";
    form.sanity()?;

    let team_id = get_team_id(&mut session, &pool, PRIVILEGE_MINIMAL, location).await?;
    let puzzle_id = form.puzzle_id;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    // check that the puzzle exist
    cache.query_puzzle_cached(puzzle_id, |_| ()).await?;

    let oracle_deposit = oracle_price();

    let new_oracle = NewOracle {
        puzzle: puzzle_id,
        team: team_id,
        cost: oracle_deposit,
        query: &form.content,
        response: "",
        active: true,
    };

    let result = conn
        .transaction::<_, APIError, _>(|conn| {
            Box::pin(async move {
                use crate::schema::oracle::dsl::*;

                let active_count: i64 = oracle
                    .filter(team.eq(team_id).and(active.eq(true)))
                    .select(diesel::dsl::count_star())
                    .get_result::<i64>(conn)
                    .await?;

                if active_count >= 5 {
                    return Ok(CreateOracleResponse::TooManyActiveOracle);
                }

                let new_balance = try_modify_team_balance(
                    team_id,
                    -new_oracle.cost,
                    format!("Create oracle on puzzle {}", puzzle_id).as_str(),
                    conn,
                    None,
                )
                .await?;

                let inserted_id: i32 = diesel::insert_into(oracle)
                    .values(&new_oracle)
                    .returning(id)
                    .get_result(conn)
                    .await?;

                Ok(CreateOracleResponse::Sucess {
                    oracle_id: inserted_id,
                    cost: -new_oracle.cost,
                    new_balance,
                })
            })
        })
        .await
        .map_err(|e| e.set_location(location).tap(APIError::log))?;

    Ok(HttpResponse::Ok().json(result))
}

#[derive(Debug, Deserialize)]
struct GetOracleRequest {
    pub oracle_id: i32,
}

impl APIRequest for GetOracleRequest {
    fn ok(&self) -> bool {
        self.oracle_id >= 0
    }
}

type GetOracleResponse = OracleRecord;

#[get("/get_oracle")]
async fn get_oracle(
    pool: web::Data<Arc<DbPool>>,
    form: web::Query<GetOracleRequest>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "get_oracle";
    form.sanity()?;

    let team_id = get_team_id(&mut session, &pool, PRIVILEGE_MINIMAL, location).await?;

    let oracle_id = form.oracle_id;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    //如果是staff, 可以获取任何存在的oracle
    //否则， 访问别的队伍的oracle会得到400
    let result: Option<GetOracleResponse> =
        if user_privilege_check(&session, PRIVILEGE_STAFF).is_ok() {
            get_oracle_by_id(oracle_id, &mut conn).await?
        } else {
            get_oracle_by_id_and_team(oracle_id, team_id, &mut conn).await?
        };

    if let Some(record) = result {
        Ok(HttpResponse::Ok().json(record))
    } else {
        Err(APIError::InvalidQuery)
    }
}

#[derive(Debug, Deserialize)]
struct CheckOracleRequest {
    pub puzzle_id: i32,
}

impl APIRequest for CheckOracleRequest {
    fn ok(&self) -> bool {
        self.puzzle_id >= 0
    }
}

#[derive(Debug, Serialize)]
struct CheckOracleResponse {
    active: Vec<i32>,
    inactive: Vec<i32>,
}

#[get("/check_oracle")]
async fn check_oracle(
    pool: web::Data<Arc<DbPool>>,
    cache: web::Data<Arc<Cache>>,
    form: web::Query<CheckOracleRequest>,
    mut session: Session,
) -> Result<impl Responder, APIError> {
    let location = "check_oracle";
    form.sanity()?;

    let team_id = get_team_id(&mut session, &pool, PRIVILEGE_MINIMAL, location).await?;
    let puzzle_id = form.puzzle_id;

    // check that the puzzle exist
    cache.query_puzzle_cached(puzzle_id, |_| ()).await?;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let (mut active, mut inactive): (Vec<i32>, Vec<i32>) = (vec![], vec![]);

    get_oracles_by_team_and_puzzle(team_id, puzzle_id, &mut conn)
        .await?
        .into_iter()
        .for_each(|item| {
            if item.active {
                &mut active
            } else {
                &mut inactive
            }
            .push(item.id)
        });

    Ok(HttpResponse::Ok().json(CheckOracleResponse { active, inactive }))
}

#[derive(Debug, Deserialize)]
struct ListOracleRequest {
    pub start_oracle_id: i32,
    pub limit: usize,
}

impl APIRequest for ListOracleRequest {
    fn ok(&self) -> bool {
        self.start_oracle_id >= 0 && self.limit <= 25
    }
}

#[derive(Serialize)]
struct ListOracleResponse {
    oracles: Vec<OracleSummaryStaff>,
}

#[get("/staff_list_oracle")]
async fn staff_list_oracle(
    pool: web::Data<Arc<DbPool>>,
    form: web::Query<ListOracleRequest>,
    session: Session,
) -> Result<impl Responder, APIError> {
    let location = "staff_list_oracle";
    form.sanity()?;

    user_privilege_check(&session, PRIVILEGE_STAFF)?;

    let start_oracle_id = form.start_oracle_id;
    let limit = form.limit;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let oracles = get_oracles_from_id(start_oracle_id, &mut conn, limit).await?;

    Ok(HttpResponse::Ok().json(ListOracleResponse { oracles }))
}

#[derive(Serialize)]
enum WorkFromResponse {
    Start(i32),
    Nothing(&'static str),
}

#[get("/staff_work_from")]
async fn staff_work_from(
    pool: web::Data<Arc<DbPool>>,
    session: Session,
) -> Result<impl Responder, APIError> {
    let location = "staff_work_from";

    user_privilege_check(&session, PRIVILEGE_STAFF)?;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    let result = find_min_active_id(&mut conn).await?;

    let result = match result {
        Some(i) => WorkFromResponse::Start(i),
        _ => WorkFromResponse::Nothing("All clear!"),
    };

    Ok(HttpResponse::Ok().json(result))
}

#[derive(Debug, Deserialize)]
struct ReplyOracleRequest {
    pub oracle_id: i32,
    pub refund_amount: i64,
    pub content: String,
}

impl APIRequest for ReplyOracleRequest {
    fn ok(&self) -> bool {
        self.oracle_id >= 0 && self.content.len() <= ORACLE_LENGTH_LIMIT_BYTES
    }
}

#[post("/staff_reply_oracle")]
async fn staff_reply_oracle(
    pool: web::Data<Arc<DbPool>>,
    form: web::Json<ReplyOracleRequest>,
    session: Session,
) -> Result<impl Responder, APIError> {
    let location = "staff_reply_oracle";
    form.sanity()?;

    let (staff_id, _) = user_privilege_check(&session, PRIVILEGE_STAFF)?;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| log_server_error(e, location, ERROR_DB_CONNECTION))?;

    //如果refund超过了cost, 会被自动取min
    //如果尝试回复一个已经被回复过的，会400
    conn.transaction::<_, APIError, _>(|conn| {
        Box::pin(async move {
            let (affected, amount) = update_active_oracle_and_return_team(
                form.oracle_id,
                form.refund_amount,
                form.content.clone(),
                conn,
            )
            .await?
            .ok_or(APIError::InvalidQuery)?;
            compulsory_team_balance(
                affected,
                amount,
                format!("Refund for oracle {} by staff {}", form.oracle_id, staff_id).as_str(),
                conn,
            )
            .await?;
            Ok(())
        })
    })
    .await
    .map_err(|e| e.set_location(location).tap(APIError::log))?;

    Ok(HttpResponse::Ok())
}
