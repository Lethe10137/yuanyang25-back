extern crate diesel;
extern crate dotenv;

use std::sync::Arc;

use actix_web::{web, App, HttpServer};

use diesel_async::pooled_connection::{bb8::Pool, AsyncDieselConnectionManager};
use diesel_async::AsyncPgConnection;

use server::api::{puzzle, register, team};
use server::util::{cache::Cache, cipher_util};

use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use log::warn;
use server::DbPool;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let cookie_token = std::env::var("COOKIE_TOKEN").expect("COOKIE_TOKEN must be set");

    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    let pool: DbPool = Pool::builder()
        .build(manager)
        .await
        .expect("Failed to link to db");

    let secret_key = cipher_util::gen_cookie_key(&cookie_token);

    let is_production = match std::env::var("MODE") {
        Ok(mode) if mode == "dev" => {
            warn!("Under development mode.");
            false
        }
        _ => true, // Production mode as default!
    };

    let pool = Arc::new(pool);
    let cache = Arc::new(Cache::new(pool.clone()));

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(cache.clone()))
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(is_production) //  cookie_secure disabled under development mode.
                    .build(),
            )
            .service(register::register_user)
            .service(register::get_user)
            .service(register::login_user)
            .service(team::create_team)
            .service(team::team_veri)
            .service(team::join_team)
            .service(team::exit_team)
            .service(team::info)
            .service(puzzle::decipher_key)
            .service(puzzle::submit_answer)
            .service(puzzle::unlock)
            .service(puzzle::puzzle_status)
    })
    .bind("0.0.0.0:9000")?
    .run()
    .await
}
