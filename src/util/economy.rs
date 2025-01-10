use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::QueryDsl;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use once_cell::sync::Lazy;
use std::cmp::max;
use std::env;
use std::ops::DerefMut;

use dotenv::dotenv;

use super::api_util::{new_unlocated_server_error, APIError};

#[derive(Debug)]
pub enum UpdateBalanceError {
    InsufficientFunds,
    DieselError(DieselError),
}

impl From<DieselError> for UpdateBalanceError {
    fn from(err: DieselError) -> Self {
        UpdateBalanceError::DieselError(err)
    }
}

impl From<UpdateBalanceError> for APIError {
    fn from(value: UpdateBalanceError) -> Self {
        match value {
            UpdateBalanceError::InsufficientFunds => APIError::InsufficientTokens,
            UpdateBalanceError::DieselError(error) => new_unlocated_server_error(error, "Economy"),
        }
    }
}

pub async fn try_modify_team_balance<C>(
    team_id: i32,
    amount: i64,
    description: &str,
    conn: &mut C,
) -> Result<i64, UpdateBalanceError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    modify_team_balance(team_id, amount, description, conn, false).await
}

pub async fn compulsory_team_balance<C>(
    team_id: i32,
    amount: i64,
    description: &str,
    conn: &mut C,
) -> Result<i64, UpdateBalanceError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    modify_team_balance(team_id, amount, description, conn, true).await
}

/// Attempts to modify the team's token balance and logs the transaction.
/// CAVEAT: Always used within a sql transaction!
async fn modify_team_balance<C>(
    team_id: i32,
    amount: i64,
    description: &str,
    conn: &mut C,
    allow_negative: bool,
) -> Result<i64, UpdateBalanceError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    use crate::schema::team::dsl as team_dsl;
    use crate::schema::transaction::dsl as transaction_dsl;

    // Query the current balance of the team
    let current_balance = team_dsl::team
        .filter(team_dsl::id.eq(team_id))
        .select(team_dsl::token_balance)
        .first::<i64>(conn)
        .await?;

    let new_balance = current_balance + amount;

    let time_allowance = time_allowance();

    // Ensure that the new balance is not negative
    if new_balance + time_allowance < 0 && !allow_negative {
        return Err(UpdateBalanceError::InsufficientFunds);
    }

    // Update the team's balance
    diesel::update(team_dsl::team.filter(team_dsl::id.eq(team_id)))
        .set((
            team_dsl::token_balance.eq(new_balance),
            team_dsl::confirmed.eq(true),
        ))
        .execute(conn)
        .await?;

    // Log the transaction
    diesel::insert_into(transaction_dsl::transaction)
        .values((
            transaction_dsl::team.eq(team_id),
            transaction_dsl::desp.eq(description),
            transaction_dsl::amount.eq(amount),
            transaction_dsl::balance.eq(new_balance),
            transaction_dsl::allowance.eq(time_allowance),
        ))
        .execute(conn)
        .await?;

    Ok(new_balance + time_allowance)
}

static GAME_EPOCH: Lazy<DateTime<Utc>> = Lazy::new(|| {
    dotenv().ok();
    env::var("GAME_EPOCH")
        .map(|time_str| time_str.as_str().parse::<DateTime<Utc>>().ok())
        .ok()
        .flatten()
        .unwrap_or_else(|| "2025-01-29T12:00:00Z".parse::<DateTime<Utc>>().unwrap())
});

pub fn game_start_minutes() -> f64 {
    let diff = Utc::now() - GAME_EPOCH.to_utc();
    max(0, diff.num_seconds()) as f64 / 60.0
}

pub fn puzzle_reward(base_reward: i32) -> i64 {
    (base_reward * 2).into()
}

pub fn puzzle_unlock_price(base_price: i32) -> i64 {
    (base_price * 2).into()
}

pub fn time_allowance() -> i64 {
    (game_start_minutes() * 25.0) as i64
}
