use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::*;
use crate::util::api_util::fetch_puzzle_from_id;

use super::api_util::{fetch_unlock_time, APIError};

type DbPool = Pool<ConnectionManager<PgConnection>>;

use diesel::{pg::Pg, result::Error};

use diesel_async::AsyncPgConnection;

struct Cache {
    unlock_cache: Arc<RwLock<HashMap<(TeamId, PuzzleId), DateTime<Utc>>>>,
    puzzle_cache: Arc<RwLock<HashMap<PuzzleId, Puzzle>>>,
    pool: DbPool,
}

impl Cache {
    // 初始化
    fn new(pool: DbPool) -> Self {
        Self {
            unlock_cache: Arc::new(RwLock::new(HashMap::new())),
            puzzle_cache: Arc::new(RwLock::new(HashMap::new())),
            pool,
        }
    }

    async fn query_puzzle_cached<T, F>(
        &self,
        puzzle_id: PuzzleId,
        conn: &mut AsyncPgConnection,
        query: F,
    ) -> Result<T, APIError>
    where
        F: FnOnce(&Puzzle) -> T,
        T: Sized,
    {
        if let Some(entry) = self.puzzle_cache.read().await.get(&puzzle_id) {
            Ok(query(entry))
        } else {
            let entry = fetch_puzzle_from_id(puzzle_id, conn).await?;
            let mut cache = self.puzzle_cache.write().await;
            let result = query(&entry);
            cache.insert(puzzle_id, entry);
            Ok(result)
        }
    }

    async fn check_unlock_cached(
        &self,
        team_id: TeamId,
        puzzle_id: PuzzleId,
        conn: &mut AsyncPgConnection,
    ) -> Result<Option<String>, APIError> {
        let query = |puzzle: &Puzzle| puzzle.key.clone();
        let cache = self.unlock_cache.read().await;
        if cache.get(&(team_id, puzzle_id)).is_some() {
            Ok(Some(
                self.query_puzzle_cached(puzzle_id, conn, query).await?,
            ))
        } else if let Some(unlock_time) = fetch_unlock_time(puzzle_id, team_id, conn).await? {
            self.unlock_cache
                .write()
                .await
                .insert((team_id, puzzle_id), unlock_time);
            Ok(Some(
                self.query_puzzle_cached(puzzle_id, conn, query).await?,
            ))
        } else {
            Ok(None)
        }
    }
}
