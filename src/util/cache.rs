use chrono::{DateTime, Utc};
use diesel_async::pooled_connection::bb8::PooledConnection;
use std::collections::HashMap;

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::puzzle::Puzzle;
use crate::models::*;
use crate::util::api_util::{
    check_is_after, fetch_puzzle_from_id, fetch_wa_cnt, ERROR_DB_CONNECTION,
};

use super::api_util::{
    fetch_unlock_time, insert_or_update_unlock_time, log_server_error, APIError,
};

use crate::DbPool;

use diesel_async::AsyncPgConnection;

#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct Cache {
    unlock_cache: Arc<RwLock<HashMap<(TeamId, PuzzleId), DateTime<Utc>>>>,
    puzzle_cache: Arc<RwLock<HashMap<PuzzleId, Puzzle>>>,
    time_punish_cache: Arc<RwLock<HashMap<(TeamId, PuzzleId), DateTime<Utc>>>>,
    pool: DbPool,
}

impl Cache {
    // 初始化
    pub fn new(pool: DbPool) -> Self {
        Self {
            unlock_cache: Arc::new(RwLock::new(HashMap::new())),
            puzzle_cache: Arc::new(RwLock::new(HashMap::new())),
            time_punish_cache: Arc::new(RwLock::new(HashMap::new())),
            pool,
        }
    }

    async fn get_connection(&self) -> Result<PooledConnection<AsyncPgConnection>, APIError> {
        self.pool
            .get()
            .await
            .map_err(|e| log_server_error(e, "cache", ERROR_DB_CONNECTION))
    }

    pub async fn query_puzzle_cached<T, F>(
        &self,
        puzzle_id: PuzzleId,
        query: F,
    ) -> Result<T, APIError>
    where
        F: FnOnce(&Puzzle) -> T,
        T: Sized,
    {
        let guard = self.puzzle_cache.read();
        if let Some(entry) = guard.await.get(&puzzle_id) {
            return Ok(query(entry));
        }

        let entry = fetch_puzzle_from_id(puzzle_id, &mut self.get_connection().await?).await?;

        let guard = self.puzzle_cache.write();
        let mut cache = guard.await;
        let result = query(&entry);
        cache.insert(puzzle_id, entry);
        Ok(result)
    }

    pub async fn check_unlock_cached(
        &self,
        team_id: TeamId,
        puzzle_id: PuzzleId,
    ) -> Result<Option<String>, APIError> {
        let query = |puzzle: &Puzzle| puzzle.base.key.clone();
        let guard = self.unlock_cache.read();
        let unlocked = guard.await.get(&(team_id, puzzle_id)).is_some();

        if unlocked {
            Ok(Some(self.query_puzzle_cached(puzzle_id, query).await?))
        } else if let Some(unlock_time) =
            fetch_unlock_time(puzzle_id, team_id, &mut self.get_connection().await?).await?
        {
            self.unlock_cache
                .write()
                .await
                .insert((team_id, puzzle_id), unlock_time);
            Ok(Some(self.query_puzzle_cached(puzzle_id, query).await?))
        } else {
            Ok(None)
        }
    }

    pub async fn check_no_submission_cached(
        &self,
        team_id: TeamId,
        puzzle_id: PuzzleId,
    ) -> Result<Option<DateTime<Utc>>, APIError> {
        let guard = self.time_punish_cache.read();

        let now = Utc::now();

        if let Some(no_submission_until) = guard
            .await
            .get(&(team_id, puzzle_id))
            .cloned()
            .and_then(|t| check_is_after(t, now))
        {
            //The cached penalty is valid till now!
            return Ok(Some(no_submission_until));
        }

        if let Some(new_penalty) =
            fetch_wa_cnt(puzzle_id, team_id, &mut self.get_connection().await?)
                .await?
                .and_then(|wa_penalty| check_is_after(wa_penalty.time_penalty_until, now))
        {
            //Found new valid penalty in database, cache and return it.
            self.time_punish_cache
                .write()
                .await
                .insert((team_id, puzzle_id), new_penalty);
            return Ok(Some(new_penalty));
        }
        Ok(None)
    }

    pub async fn add_unlock_cache(
        &self,
        team_id: TeamId,
        puzzle_id: PuzzleId,
    ) -> Result<(), APIError> {
        let unlock_time = Utc::now();

        self.unlock_cache
            .write()
            .await
            .insert((team_id, puzzle_id), unlock_time);

        insert_or_update_unlock_time(
            puzzle_id,
            team_id,
            unlock_time,
            &mut self.get_connection().await?,
        )
        .await
    }
}
