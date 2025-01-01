pub mod api;
pub mod util;

pub mod models;
pub mod schema;

use diesel_async::pooled_connection::bb8::Pool;
use diesel_async::AsyncPgConnection;

pub type DbPool = Pool<AsyncPgConnection>;

pub const VERICODE_LENGTH: usize = 16;

pub trait Ext<R>: Sized {
    fn tap_mut(mut self, f: impl FnOnce(&mut Self) -> R) -> Self {
        f(&mut self);
        self
    }

    fn tap(self, f: impl FnOnce(&Self) -> R) -> Self {
        f(&self);
        self
    }
}

impl<T, R> Ext<R> for T {}
