pub mod api;
pub mod util;

pub mod models;
pub mod schema;

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;
