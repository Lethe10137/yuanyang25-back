extern crate diesel;
extern crate dotenv;

use actix_web::{web, App, HttpServer};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};

use server::api::register;
use server::util::cipher_util;

use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use log::warn;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let cookie_token = std::env::var("COOKIE_TOKEN").expect("COOKIE_TOKEN must be set");

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let secret_key = cipher_util::gen_cookie_key(&cookie_token);

    let is_production = match std::env::var("MODE") {
        Ok(mode) if mode == "dev" => {
            warn!("Under development mode.");
            false
        }
        _ => true, // Production mode as default!
    };

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(is_production) //  cookie_secure disabled under development mode.
                    .build(),
            )
            .service(register::register_user)
            .service(register::get_user)
            .service(register::login_user)
    })
    .bind("0.0.0.0:9000")?
    .run()
    .await
}
