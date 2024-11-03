use diesel::prelude::*;
#[allow(dead_code)]
#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: i32,
    pub openid: String,
    pub team: Option<i32>,
    pub username: String,
    pub password: String,
    pub salt: String,
    pub priviledge: i32,
}
