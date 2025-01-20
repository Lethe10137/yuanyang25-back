extern crate diesel;
extern crate dotenv;

use std::sync::Arc;

use actix_cors::Cors;
use actix_web::dev::RequestHead;
use actix_web::http::header::HeaderValue;
use actix_web::{web, App, HttpServer};

use diesel_async::pooled_connection::{bb8::Pool, AsyncDieselConnectionManager};
use diesel_async::AsyncPgConnection;

use server::api::{monitor, oracle, puzzle, register, team};
use server::util::{cache::Cache, cipher_util};

use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use log::warn;
use server::DbPool;

fn cors_check(head: &HeaderValue, _: &RequestHead) -> bool {
    if let Ok(origin) = head.to_str() {
        match origin {
            "https://2025.yuanyang.app" => true,
            "https://yuanyang.app" => true,
            "http://localhost:5173" => true,
            url => url.ends_with("yuanyang25-front.netlify.app"), // for deploy preview
        }
    } else {
        false
    }
}

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
                Cors::default()
                    .allowed_origin_fn(cors_check)
                    .allow_any_header()
                    .allow_any_method()
                    .supports_credentials(),
            )
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(is_production) // 在生产环境下使用 `Secure`，在开发模式下可以禁用
                    .cookie_same_site(actix_web::cookie::SameSite::None) // 设置 SameSite=None 以支持跨站点请求
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
            .service(puzzle::rank)
            .service(monitor::cache_size)
            .service(oracle::create_oracle)
            .service(oracle::get_oracle)
            .service(oracle::check_oracle)
            .service(oracle::staff_list_oracle)
            .service(oracle::staff_reply_oracle)
    })
    .bind("0.0.0.0:9000")?
    .run()
    .await
}
