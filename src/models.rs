use chrono::{DateTime, Utc};
use diesel::prelude::*;

pub type TeamId = i32;
pub type PuzzleId = i32;
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

//TODO: 检查(team, puzzle)是不是有索引

#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = crate::schema::unlock)]
pub struct Unlock {
    pub time: DateTime<Utc>,
    pub team: TeamId,
    pub puzzle: PuzzleId,
}

#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = crate::schema::puzzle)]
pub struct PuzzleBase {
    pub unlock: i32,
    pub bounty: i32,
    pub title: String,
    pub answer: String,
    pub key: String,
}

#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = crate::schema::mid_answer)]
pub struct MidAnswer {
    pub id: i32,
    pub puzzle: i32,
    pub query: String,
    pub response: String,
}

#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = crate::schema::hint)]
pub struct Hint {
    pub id: i32,
    pub title: String,
    pub base_price: i64,
    pub puzzle: i32,
    pub content: String,
}
