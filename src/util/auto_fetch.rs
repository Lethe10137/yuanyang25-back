use log::{debug, info};
use moka::future::Cache;
use moka::notification::RemovalCause;
use moka::Expiry;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

use std::fmt::Debug;

/// An enum to represent the expiration of a value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Expiration {
    AtOnce,
    Short,
    Middle,
    Long,
    Never,
}

impl Expiration {
    /// Returns the duration of this expiration.
    pub fn as_duration(&self) -> Option<Duration> {
        match self {
            Expiration::AtOnce => Some(Duration::from_secs(0)),
            Expiration::Short => Some(Duration::from_secs(2)),
            Expiration::Middle => Some(Duration::from_secs(600)),
            Expiration::Long => Some(Duration::from_secs(7200)),
            Expiration::Never => None,
        }
    }
}

pub struct MyExpiry;

impl<K, V> Expiry<K, (Expiration, V)> for MyExpiry
where
    K: Clone + std::hash::Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn expire_after_create(
        &self,
        _key: &K,
        value: &(Expiration, V),
        _created_at: std::time::Instant,
    ) -> Option<Duration> {
        value.0.as_duration()
    }

    fn expire_after_update(
        &self,
        _key: &K,
        value: &(Expiration, V),
        _updated_at: std::time::Instant,
        _duration_until_expiry: Option<Duration>,
    ) -> Option<Duration> {
        value.0.as_duration()
    }
}

pub type AutoCacheReadHandle<T, E> = JoinHandle<Result<(T, Expiration), E>>;
pub type AutoCacheWriteHandle<E> = JoinHandle<Result<(), E>>;

pub struct AutoCache<K, V, F, G, E>
where
    K: Clone + std::hash::Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    F: Fn(K) -> AutoCacheReadHandle<V, E> + Send + Sync + 'static,
    G: Fn(K, V) -> AutoCacheWriteHandle<E> + Send + Sync + 'static, // write into db
{
    cache: Cache<K, (Expiration, V)>,
    capacity: usize,
    value_loader: Arc<F>,
    value_writer: Arc<G>,
}

fn eviction_listener<K: Debug, V>(key: Arc<K>, _value: V, cause: RemovalCause) {
    let value_type = std::any::type_name::<V>();
    info!("Evicted key {key:?} -> {} Cause: {cause:?}", value_type);
}

impl<K, V, F, G, E> AutoCache<K, V, F, G, E>
where
    K: Clone + std::hash::Hash + Eq + Send + Sync + Debug + 'static,
    V: Clone + Send + Sync + 'static,
    F: Fn(K) -> AutoCacheReadHandle<V, E> + Send + Sync + 'static, // read from db
    G: Fn(K, V) -> AutoCacheWriteHandle<E> + Send + Sync + 'static, // write into db
{
    pub fn new(capacity: usize, value_loader: F, value_writer: G) -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(capacity as u64) // 设置 LRU 驱逐策略
                .expire_after(MyExpiry)
                .eviction_listener(eviction_listener)
                .build(),
            capacity,
            value_loader: Arc::new(value_loader),
            value_writer: Arc::new(value_writer),
        }
    }

    pub fn size(&self) -> (usize, usize) {
        (self.cache.weighted_size() as usize, self.capacity)
    }

    pub async fn get(&self, key: K) -> Result<V, E> {
        if let Some(value) = self.cache.get(&key).await {
            debug!("Got cached key {key:?} -> {}", std::any::type_name::<V>());
            return Ok(value.1);
        }

        let value_loader = self.value_loader.clone();
        let key_clone = key.clone();

        debug!("Fetching key {key:?} -> {}", std::any::type_name::<V>());
        let (value, expiry) = (value_loader)(key_clone)
            .await
            .expect("Value loader panicked")?;

        if expiry != Expiration::AtOnce {
            debug!(
                "Caching fetched key {key:?} -> {}",
                std::any::type_name::<V>()
            );
            self.cache
                .get_with(key, async { (expiry, value.clone()) })
                .await;
        }

        Ok(value)
    }

    // write through
    pub async fn set(&self, key: K, value: V, expiry: Expiration) -> Result<(), E> {
        if expiry != Expiration::AtOnce {
            self.cache
                .get_with(key.clone(), async { (expiry, value.clone()) })
                .await;
            debug!(
                "Caching setted key {key:?} -> {}",
                std::any::type_name::<V>()
            );
        }

        debug!(
            "Wtring setted key {key:?} -> {}",
            std::any::type_name::<V>()
        );
        (self.value_writer)(key, value)
            .await
            .expect("Value writer panicked")
    }

    pub async fn invalidate(&self, key: K) {
        self.cache.invalidate(&key).await
    }
}
