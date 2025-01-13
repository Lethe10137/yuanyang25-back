use chrono::{DateTime, Utc};
use diesel::prelude::*;

pub type TeamId = i32;
pub type PuzzleId = i32;
pub type DecipherId = i32;
pub type UserId = i32;

#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: UserId,
    pub openid: String,
    pub team: Option<TeamId>,
    pub username: String,
    pub password: String,
    pub salt: String,
    pub privilege: i32,
}

#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = crate::schema::team)]
pub struct Team {
    pub id: TeamId,
    pub is_staff: bool,
    pub token_balance: i64,
    pub confirmed: bool,
    pub max_size: i32,
    pub size: i32,
    pub salt: String,
}

#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = crate::schema::unlock)]
pub struct Unlock {
    pub level: i32,
    pub team: TeamId,
    pub decipher: DecipherId,
}

#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = crate::schema::puzzle)]
pub struct PuzzleBase {
    pub meta: bool,
    pub bounty: i32,
    pub title: String,
    pub decipher: i32,
    pub depth: i32,
}

#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = crate::schema::wrong_answer_cnt)]
pub struct WaPenalty {
    pub time_penalty_until: DateTime<Utc>,
    pub token_penalty_level: i32,
    pub time_penalty_level: i32,
}

#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = crate::schema::decipher)]
pub struct Decipher {
    pub pricing_type: i32,
    pub base_price: i32,
    pub depth: i32,
    pub root: String,
}
