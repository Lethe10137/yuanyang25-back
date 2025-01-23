use chrono::{DateTime, TimeZone, Utc};
use diesel::query_dsl::QueryDsl;
use diesel::result::Error;
use diesel::ExpressionMethods;
use diesel_async::RunQueryDsl;

use std::sync::Arc;

use crate::api::puzzle::Puzzle;
use crate::models::*;
use crate::util::api_util::ERROR_DB_CONNECTION;

use crate::util::auto_fetch::Expiration;

use super::{
    api_util::{log_server_error, APIError},
    auto_fetch::MyExpiry,
    stat::{fetch_statistic, PuzzleStatistic},
};

use crate::DbPool;

use super::auto_fetch::{AutoCache, AutoCacheReadHandle, AutoCacheWriteHandle};

use moka::future::Cache as MokaCache;

type APICache<K, V> = AutoCache<
    K,
    V,
    Box<dyn Fn(K) -> AutoCacheReadHandle<V, APIError> + Send + Sync>,
    Box<dyn Fn(K, V) -> AutoCacheWriteHandle<APIError> + Send + Sync>,
    APIError,
>;

#[allow(clippy::type_complexity)]
pub struct Cache {
    pub unlock_cache: APICache<(TeamId, DecipherId), Option<i32>>,
    pub puzzle_cache: APICache<PuzzleId, Arc<Puzzle>>,
    pub time_punish_cache: APICache<(TeamId, PuzzleId), DateTime<Utc>>,
    pub decipher_cache: APICache<DecipherId, Arc<Decipher>>,
    pub stat: MokaCache<(), (Expiration, Arc<PuzzleStatistic>)>,
    pool: Arc<DbPool>,
}

#[derive(Debug, serde::Serialize)]
pub struct CacheStatusResponse {
    unlock: (usize, usize),
    puzzle: (usize, usize),
    time_punish: (usize, usize),
    decipher: (usize, usize),
}

fn fetchdb_unlock_level(
    pool: Arc<DbPool>,
    key: (TeamId, DecipherId),
) -> AutoCacheReadHandle<Option<i32>, APIError> {
    use crate::schema::unlock::dsl::*;
    tokio::spawn(async move {
        let mut conn = pool
            .get()
            .await
            .map_err(|e| log_server_error(e, "cache", ERROR_DB_CONNECTION))?;
        let (team_key, decipher_key) = key;
        match unlock
            .filter(team.eq(team_key))
            .filter(decipher.eq(decipher_key))
            .select(level)
            .first::<i32>(&mut conn)
            .await
        {
            Ok(0) => Ok((Some(0), Expiration::Long)),
            Ok(level_value) => Ok((Some(level_value), Expiration::Short)),
            Err(Error::NotFound) => Ok((None, Expiration::Short)),
            Err(err) => Err(log_server_error(err, "cache", ERROR_DB_CONNECTION)),
        }
    })
}

fn writedb_unlock_level(
    pool: Arc<DbPool>,
    key: (TeamId, DecipherId),
    value: Option<i32>,
) -> AutoCacheWriteHandle<APIError> {
    use crate::schema::unlock::dsl::*;
    tokio::spawn(async move {
        let mut conn = pool
            .get()
            .await
            .map_err(|e| log_server_error(e, "cache", ERROR_DB_CONNECTION))?;
        let (team_key, decipher_key) = key;
        if let Some(value) = value {
            diesel::insert_into(unlock)
                .values((
                    team.eq(team_key),
                    decipher.eq(decipher_key),
                    level.eq(value),
                ))
                .on_conflict((team, decipher))
                .do_update()
                .set(level.eq(value))
                .execute(&mut conn)
                .await
                .map_err(|e| log_server_error(e, "cache", ERROR_DB_CONNECTION))?;
        }
        Ok(())
    })
}

fn fetchdb_puzzle(
    pool: Arc<DbPool>,
    puzzle_id: PuzzleId,
) -> AutoCacheReadHandle<Arc<Puzzle>, APIError> {
    use crate::schema::answer::dsl as answer_dsl;
    use crate::schema::other_answer::dsl as other_answer_dsl;
    use crate::schema::puzzle::dsl as puzzle_dsl;

    tokio::spawn(async move {
        let mut conn = pool
            .get()
            .await
            .map_err(|e| log_server_error(e, "cache", ERROR_DB_CONNECTION))?;
        let puzzle_item = match puzzle_dsl::puzzle
            .filter(puzzle_dsl::id.eq(puzzle_id))
            .select((
                puzzle_dsl::meta,
                puzzle_dsl::bounty,
                puzzle_dsl::title,
                puzzle_dsl::decipher,
                puzzle_dsl::depth,
            ))
            .first::<PuzzleBase>(&mut conn)
            .await
        {
            Ok(p) => Ok(p),
            Err(Error::NotFound) => Err(APIError::InvalidQuery),
            Err(err) => Err(log_server_error(err, "cache", ERROR_DB_CONNECTION)),
        }?;

        let answers = match answer_dsl::answer
            .filter(answer_dsl::puzzle.eq(puzzle_id))
            .select((answer_dsl::sha256, answer_dsl::level))
            .load::<(String, i32)>(&mut conn)
            .await
        {
            Ok(p) => Ok(p),
            Err(Error::NotFound) => Ok(vec![]),
            Err(err) => Err(log_server_error(err, "cache", ERROR_DB_CONNECTION)),
        }?;

        let other_answers = match other_answer_dsl::other_answer
            .filter(other_answer_dsl::puzzle.eq(puzzle_id))
            .select((
                other_answer_dsl::sha256,
                other_answer_dsl::content,
                other_answer_dsl::id,
            ))
            .load::<(String, String, i32)>(&mut conn)
            .await
        {
            Ok(p) => Ok(p),
            Err(Error::NotFound) => Ok(vec![]),
            Err(err) => Err(log_server_error(err, "cache", ERROR_DB_CONNECTION)),
        }?;

        Ok((
            Arc::new(Puzzle::new(
                puzzle_item,
                answers,
                other_answers
                    .into_iter()
                    .map(|(sha, content, refnum)| (sha, (refnum, content)))
                    .collect(),
            )),
            Expiration::Long,
        ))
    })
}

fn fetchdb_decipher(
    pool: Arc<DbPool>,
    decipher_id: DecipherId,
) -> AutoCacheReadHandle<Arc<Decipher>, APIError> {
    use crate::schema::decipher::dsl::*;

    tokio::spawn(async move {
        let mut conn = pool
            .get()
            .await
            .map_err(|e| log_server_error(e, "cache", ERROR_DB_CONNECTION))?;
        let item = match decipher
            .filter(id.eq(decipher_id))
            .select((pricing_type, base_price, depth, root))
            .first::<Decipher>(&mut conn)
            .await
        {
            Ok(p) => Ok(p),
            Err(Error::NotFound) => Err(APIError::InvalidQuery),
            Err(err) => Err(log_server_error(err, "cache", ERROR_DB_CONNECTION)),
        }?;

        Ok((Arc::new(item), Expiration::Long))
    })
}

fn fetchdb_time_punish(
    pool: Arc<DbPool>,
    key: (TeamId, PuzzleId),
) -> AutoCacheReadHandle<DateTime<Utc>, APIError> {
    use crate::schema::wrong_answer_cnt::dsl::*;
    tokio::spawn(async move {
        let mut conn = pool
            .get()
            .await
            .map_err(|e| log_server_error(e, "cache", ERROR_DB_CONNECTION))?;
        let (team_key, puzzle_key) = key;
        match wrong_answer_cnt
            .filter(team.eq(team_key))
            .filter(puzzle.eq(puzzle_key))
            .select(time_penalty_until)
            .first::<DateTime<Utc>>(&mut conn)
            .await
        {
            Ok(time) => Ok((time, Expiration::Middle)),
            Err(Error::NotFound) => Ok((Utc.timestamp_opt(1, 0).unwrap(), Expiration::AtOnce)),
            Err(err) => Err(log_server_error(err, "cache", ERROR_DB_CONNECTION)),
        }
    })
}

impl Cache {
    // 初始化
    pub fn new(pool: Arc<DbPool>) -> Self {
        let fetch_closure_unlock = {
            let pool = Arc::clone(&pool);
            Box::new(move |key| fetchdb_unlock_level(Arc::clone(&pool), key))
        };

        let write_closure_unlock = {
            let pool = Arc::clone(&pool);
            Box::new(move |key, value| writedb_unlock_level(Arc::clone(&pool), key, value))
        };

        let fetch_closure_puzzle = {
            let pool = Arc::clone(&pool);
            Box::new(move |key| fetchdb_puzzle(Arc::clone(&pool), key))
        };

        let fetch_closure_decipher = {
            let pool = Arc::clone(&pool);
            Box::new(move |key| fetchdb_decipher(Arc::clone(&pool), key))
        };

        let fetch_closure_time_punish = {
            let pool = Arc::clone(&pool);
            Box::new(move |key| fetchdb_time_punish(Arc::clone(&pool), key))
        };

        Self {
            unlock_cache: AutoCache::new(4096, fetch_closure_unlock, write_closure_unlock),
            puzzle_cache: AutoCache::new(
                32,
                fetch_closure_puzzle,
                Box::new(|_, _| unimplemented!()), // Never write a puzzle
            ),
            time_punish_cache: AutoCache::new(
                4096,
                fetch_closure_time_punish,
                Box::new(|_, _| tokio::spawn(async { Ok(()) })), // Is written otherwise
            ),
            decipher_cache: AutoCache::new(
                256,
                fetch_closure_decipher,
                Box::new(|_, _| unimplemented!()), // Never write a puzzle
            ),

            stat: MokaCache::builder()
                .max_capacity(2)
                .expire_after(MyExpiry)
                .build(),
            pool: pool.clone(),
        }
    }

    pub fn get_size(&self) -> CacheStatusResponse {
        CacheStatusResponse {
            unlock: self.unlock_cache.size(),
            puzzle: self.puzzle_cache.size(),
            time_punish: self.time_punish_cache.size(),
            decipher: self.decipher_cache.size(),
        }
    }

    pub async fn get_stat(&self) -> Result<Arc<PuzzleStatistic>, APIError> {
        if let Some((_, data)) = self.stat.get(&()).await {
            return Ok(data);
        }
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| log_server_error(e, "cache", ERROR_DB_CONNECTION))?;
        let new_data = Arc::new(fetch_statistic(&mut conn).await?);
        self.stat
            .get_with((), async { (Expiration::Middle, new_data.clone()) })
            .await;
        Ok(new_data)
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
        self.puzzle_cache
            .get(puzzle_id)
            .await
            .map(|puzzle| query(&puzzle))
    }

    pub async fn query_wa_penalty(
        &self,
        team_id: TeamId,
        puzzle_id: PuzzleId,
    ) -> Result<Option<DateTime<Utc>>, APIError> {
        let cached_penalty = self.time_punish_cache.get((team_id, puzzle_id)).await?;
        let now = Utc::now();

        if cached_penalty > now {
            return Ok(Some(cached_penalty));
        }

        self.time_punish_cache
            .invalidate((team_id, puzzle_id))
            .await;

        if cached_penalty < Utc.timestamp_opt(1000000, 0).unwrap() {
            //Only happens when fetched no such record.
            return Ok(None);
        }

        Ok(None)
    }
}
