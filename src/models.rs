use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: i32,
    pub openid: String,
    pub team: Option<i32>,
    pub username: String,
    pub password: String,
    pub salt: String,
    pub privilege: i32,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::team)]
pub struct Team {
    pub id: i32,
    pub is_staff: bool,
    pub token_balance: i64,
    pub confirmed: bool,
    pub max_size: i32,
    pub size: i32,
    pub salt: String,
}
