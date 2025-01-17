use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use diesel::result::Error as DieselError;
use diesel::QueryDsl;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use log::debug;
use once_cell::sync::Lazy;
use std::cmp::max;
use std::env;
use std::ops::DerefMut;

use dotenv::dotenv;

use super::api_util::{new_unlocated_server_error, APIError};

#[derive(Debug)]
pub enum UpdateBalanceError {
    InsufficientFunds,
    TransactionCancel(i64),
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
            UpdateBalanceError::TransactionCancel(balance) => {
                APIError::TransactionCancel { balance }
            }
            UpdateBalanceError::DieselError(error) => new_unlocated_server_error(error, "Economy"),
        }
    }
}

pub async fn try_modify_team_balance<C>(
    team_id: i32,
    amount: i64,
    description: &str,
    conn: &mut C,
    purchase: Option<i32>,
) -> Result<i64, UpdateBalanceError>
where
    C: DerefMut<Target = AsyncPgConnection> + std::marker::Send,
{
    modify_team_balance(team_id, amount, description, conn, false, purchase).await
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
    modify_team_balance(team_id, amount, description, conn, true, None).await
}

/// Attempts to modify the team's token balance and logs the transaction.
/// CAVEAT: Always used within a sql transaction!
async fn modify_team_balance<C>(
    team_id: i32,
    amount: i64,
    description: &str,
    conn: &mut C,
    allow_negative: bool,
    purchase: Option<i32>,
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

    // Log the transaction
    diesel::insert_into(transaction_dsl::transaction)
        .values((
            transaction_dsl::team.eq(team_id),
            transaction_dsl::desp.eq(description),
            transaction_dsl::amount.eq(amount),
            transaction_dsl::balance.eq(new_balance),
            transaction_dsl::allowance.eq(time_allowance),
            transaction_dsl::purchase_ref.eq(purchase),
        ))
        .execute(conn)
        .await
        .map_err(|e| match e {
            DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                UpdateBalanceError::TransactionCancel(current_balance + time_allowance)
            }
            e => UpdateBalanceError::from(e),
        })?;

    // Update the team's balance
    diesel::update(team_dsl::team.filter(team_dsl::id.eq(team_id)))
        .set((
            team_dsl::token_balance.eq(new_balance),
            team_dsl::confirmed.eq(true),
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

const HINT_DEFLATION_TIMES: f64 = 15.0;
const HINT_BASE_TIMES: f64 = 1.0;
const HINT_DEFLATION_DAYS: f64 = 7.0;

const SKIP_DEFLATION_TIMES: f64 = 8.0;
const SKIP_BASE_TIMES: f64 = 5.0;
const SKIP_DEFLATION_DAYS: f64 = 7.0;

const AWARD_DEFLATION_TIMES: f64 = 1.25;
const AWARD_BASE_TIMES: f64 = 2.0;
const AWARD_DEFLATION_DAYS: f64 = 7.0;

const UNLOCK_INFLATION_TIMES: f64 = 2.0;
const UNLOCK_BASE_TIMES: f64 = 0.5;
const UNLOCK_INFLATION_DAYS: f64 = 3.0;

// Drop from 15.0 to 1.0 in 7 days
pub fn hint_factor() -> f64 {
    let relative = game_start_minutes() / (HINT_DEFLATION_DAYS * 1440.0);
    let relative = relative.clamp(0.0, 1.0);

    debug!(
        "hint factor = {}",
        HINT_DEFLATION_TIMES.powf(1.0 - relative) * HINT_BASE_TIMES
    );
    HINT_DEFLATION_TIMES.powf(1.0 - relative) * HINT_BASE_TIMES
}

// Drop from 40.0 to 5.0 in 7 days
pub fn skip_factor() -> f64 {
    let relative = game_start_minutes() / (SKIP_DEFLATION_DAYS * 1440.0);
    let relative = relative.clamp(0.0, 1.0);
    debug!(
        "skip factor = {}",
        SKIP_DEFLATION_TIMES.powf(1.0 - relative) * SKIP_BASE_TIMES
    );
    SKIP_DEFLATION_TIMES.powf(1.0 - relative) * SKIP_BASE_TIMES
}

// Rise from 0.5 to 1 in 3 days
pub fn unlock_factor() -> f64 {
    let relative = game_start_minutes() / (UNLOCK_INFLATION_DAYS * 1440.0);
    let relative = relative.clamp(0.0, 1.0);
    debug!(
        "unlock factor = {}",
        UNLOCK_INFLATION_TIMES.powf(relative) * UNLOCK_BASE_TIMES
    );
    UNLOCK_INFLATION_TIMES.powf(relative) * UNLOCK_BASE_TIMES
}

// Drop from 2.5 to 2.0 in 7 days
pub fn reward_factor() -> f64 {
    let relative = game_start_minutes() / (AWARD_DEFLATION_DAYS * 1440.0);
    let relative = relative.clamp(0.0, 1.0);
    debug!(
        "awrad factor = {}",
        AWARD_DEFLATION_TIMES.powf(1.0 - relative) * AWARD_BASE_TIMES
    );
    AWARD_DEFLATION_TIMES.powf(1.0 - relative) * AWARD_BASE_TIMES
}

pub fn puzzle_reward(base_reward: i32, factor: f64) -> i64 {
    (base_reward as f64 * reward_factor() * factor) as i64
}

pub fn deciper_price(pricing_type: i32, base_price: i32) -> i64 {
    let result = match pricing_type {
        //normal unlock
        0 => (unlock_factor() * base_price as f64) as i64,
        //hint
        1 => (hint_factor() * base_price as f64) as i64,
        //normal skip
        2 => (skip_factor() * base_price as f64) as i64,
        // price of meta, mannualy priced
        _ => base_price as i64,
    };
    debug!(
        "type = {}, base = {}, result = {}",
        pricing_type, base_price, result
    );
    result
}

pub fn time_allowance() -> i64 {
    (game_start_minutes() * 25.0) as i64
}
