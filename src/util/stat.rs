use std::ops::DerefMut;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Serialize;

use super::api_util::{log_server_error, APIError, ERROR_DB_UNKNOWN};

#[derive(Serialize, QueryableByName, Queryable, Clone)] // 添加 QueryableByName
pub struct CountItem {
    #[diesel(sql_type = Integer)]
    pub puzzle_id: i32,
    #[diesel(sql_type = Integer)]
    pub decipher: i32,
    #[diesel(sql_type = BigInt)]
    pub teams_passed: i64,
    #[diesel(sql_type = BigInt)]
    pub teams_unlocked: i64,
}

pub struct PuzzleStatistic {
    pub data: Vec<CountItem>,
    pub time: DateTime<Utc>,
}

pub async fn fetch_statistic<C>(conn: &mut C) -> Result<PuzzleStatistic, APIError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    // Define the query to fetch puzzle stats
    let query = diesel::sql_query(
        r#"
        SELECT 
            p.id AS puzzle_id,
            p.decipher,
            COALESCE(COUNT(DISTINCT CASE WHEN s.depth = 0 THEN s.team ELSE NULL END), 0) AS teams_passed,
            COALESCE(COUNT(DISTINCT u.team), 0) AS teams_unlocked
        FROM puzzle AS p
        LEFT JOIN submission AS s
            ON p.id = s.puzzle
        LEFT JOIN unlock AS u
            ON p.decipher = u.decipher
        GROUP BY p.id, p.decipher
        ORDER BY p.id;
    "#,
    );

    // Execute the query and map results to CountItem
    let data: Vec<CountItem> = query
        .load(conn)
        .await
        .map_err(|e| log_server_error(e, "stat", ERROR_DB_UNKNOWN))?;

    // Create the statistic object
    let statistic = PuzzleStatistic {
        data,
        time: Utc::now(),
    };

    Ok(statistic)
}

pub async fn fetch_statistic_for_team<C>(
    conn: &mut C,
    team_id: i32,
) -> Result<PuzzleStatistic, APIError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    // Define the query to fetch puzzle stats for a specific team
    let query = diesel::sql_query(r#"
        SELECT 
            p.id AS puzzle_id,
            p.decipher,
            COALESCE(SUM(CASE WHEN s.depth = 0 AND s.team = $1 THEN 1 ELSE 0 END), 0) AS teams_passed,
            COALESCE(COUNT(DISTINCT CASE WHEN u.team = $1 THEN u.team ELSE NULL END), 0) AS teams_unlocked
        FROM puzzle AS p
        LEFT JOIN submission AS s
            ON p.id = s.puzzle
        LEFT JOIN unlock AS u
            ON p.decipher = u.decipher
        GROUP BY p.id, p.decipher
        ORDER BY p.id;
    "#)
    .bind::<diesel::sql_types::Integer, _>(team_id);

    // Execute the query and map results to CountItem
    let data: Vec<CountItem> = query
        .load(conn)
        .await
        .map_err(|e| log_server_error(e, "stat", ERROR_DB_UNKNOWN))?;

    // Create the statistic object
    let statistic = PuzzleStatistic {
        data,
        time: Utc::now(),
    };

    Ok(statistic)
}
