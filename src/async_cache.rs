use crate::sync_cache::SyncCache;
use chrono::Duration;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AsyncCache<Key, Val> {
    cache: Arc<RwLock<SyncCache<Key, Val>>>,
    _key: PhantomData<Key>,
    _val: PhantomData<Val>,
}

impl<Key, Val> AsyncCache<Key, Val>
where
    Key: Eq + Hash + Clone,
    Val: Clone,
{
    pub fn new() -> Self {
        AsyncCache {
            cache: Arc::new(RwLock::new(SyncCache::new())),
            _key: PhantomData,
            _val: PhantomData,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        AsyncCache {
            cache: Arc::new(RwLock::new(SyncCache::with_capacity(capacity))),
            _key: PhantomData,
            _val: PhantomData,
        }
    }

    pub async fn get(&self, key: &Key) -> Option<Val> {
        // TODO as soon as a single key expires, every get command will try to get a write lock unnecessarily
        // only one actually needs to
        // also, this does not guarantee that stale data cannot be read if there is read/write contention
        {
            let cache = self.cache.read().await;
            if !cache.has_expired_items() {
                return cache.get(key).map(|val| val.clone());
            }
        }

        let mut cache = self.cache.write().await;
        cache.remove_expired_items();
        cache.get(key).map(|val| val.clone())
    }

    pub async fn set(&self, key: Key, value: Val, ttl: Duration) -> bool {
        self.cache.write().await.set(key, value, ttl)
    }

    pub async fn clear(&mut self) {
        self.cache.write().await.clear()
    }
}
