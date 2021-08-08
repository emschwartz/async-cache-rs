use crate::sync_cache::SyncCache;
use chrono::Duration;
use std::future::Future;
use std::hash::Hash;
use std::marker::PhantomData;
use std::pin::Pin;
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
    #[inline]
    pub fn new() -> Self {
        AsyncCache {
            cache: Arc::new(RwLock::new(SyncCache::new())),
            _key: PhantomData,
            _val: PhantomData,
        }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        AsyncCache {
            cache: Arc::new(RwLock::new(SyncCache::with_capacity(capacity))),
            _key: PhantomData,
            _val: PhantomData,
        }
    }

    // Returns the value corresponding to the given key if it is in the cache.
    //
    // If the cache has expired values, it will first acquire a write lock,
    // remove the expired values, and then return the result for the given key.
    //
    // Note that this returns a cloned value instead of a reference because
    // the value in the map may be expired and removed before the return value is used.
    // To avoid cloning objects that are expensive to clone, simply wrap
    // those objects in an Arc.
    #[inline]
    pub async fn get(&self, key: &Key) -> Option<Val> {
        // TODO as soon as a single key expires, every get command will try to get a write lock unnecessarily
        // only one actually needs to
        // also, this does not guarantee that stale data cannot be read if there is read/write contention
        {
            let cache = self.cache.read().await;
            if !cache.has_expired_items() {
                return cache.get(key).cloned();
            }
        }

        let mut cache = self.cache.write().await;
        cache.remove_expired_items();
        cache.get(key).cloned()
    }

    #[inline]
    pub async fn set(&self, key: Key, value: Val, ttl: Duration) -> bool {
        self.cache.write().await.set(key, value, ttl)
    }

    #[inline]
    pub async fn remove(&self, key: &Key) -> bool {
        self.cache.write().await.remove(key)
    }

    #[inline]
    pub async fn clear(&self) {
        self.cache.write().await.clear()
    }

    #[inline]
    pub async fn len(&self) -> usize {
        self.cache.read().await.len()
    }

    #[inline]
    pub async fn is_empty(&self) -> bool {
        self.cache.read().await.is_empty()
    }

    // Returns a version of the given function that caches the return values
    // using the input as the Key and the returned Duration as the value's TTL
    pub fn cache_fn<'a, Fut, ErrType>(
        &self,
        f: impl Fn(Key) -> Fut + 'a,
    ) -> impl Fn(Key) -> Pin<Box<dyn Future<Output = Result<Val, ErrType>> + 'a>> + 'a
    where
        Key: 'a,
        Val: 'a,
        // TODO maybe use std::time::Duration or u32 in the function signature
        // TODO maybe define a trait like GetTtl on the return type instead of requiring it to be a tuple
        Fut: Future<Output = Result<(Val, Duration), ErrType>> + 'static,
    {
        let f = Arc::new(f);
        let cache = self.cache.clone();
        move |key| {
            let cache = cache.clone();
            let f = f.clone();

            Box::pin(async move {
                // Try getting the value from the cache fist
                if let Some(val) = cache.read().await.get(&key) {
                    return Ok(val.clone());
                }

                // If the result wasn't already in the cache, call the function
                // and store the result in the cache (as long as the function did not error)
                let (val, ttl) = f(key.clone()).await?;
                cache.write().await.set(key, val.clone(), ttl);
                Ok(val)
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::spawn;

    #[tokio::test]
    async fn basic_get_set() {
        let cache = AsyncCache::with_capacity(5);
        cache.set("a", 1u32, Duration::milliseconds(200)).await;
        assert_eq!(cache.get(&"a").await, Some(1));

        spawn(async move {
            cache.set("b", 2, Duration::seconds(10)).await;
            assert_eq!(cache.get(&"b").await, Some(2));
        });
    }

    #[tokio::test]
    async fn getting_expired_value() {
        let cache = AsyncCache::with_capacity(5);
        // expired
        cache.set("a", 1u32, Duration::milliseconds(-200)).await;
        // not expired
        cache.set("b", 2u32, Duration::milliseconds(200)).await;

        assert_eq!(cache.get(&"a").await, None);
        assert_eq!(cache.get(&"b").await, Some(2));
    }

    #[tokio::test]
    async fn cache_fn() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();
        let f = move |num: u32| {
            let calls = calls_clone.clone();
            async move {
                if calls.fetch_add(1, Ordering::Relaxed) == 0 {
                    Err("some error")
                } else {
                    Ok((format!("{}", num), Duration::seconds(10)))
                }
            }
        };
        let cache = AsyncCache::with_capacity(3);
        let cached = cache.cache_fn(f);

        // passes through error and does not cache it
        assert_eq!(cached(1).await, Err("some error"));
        assert_eq!(cache.is_empty().await, true);

        // returns and caches ok value
        assert_eq!(cached(1).await, Ok("1".to_string()));
        assert_eq!(cache.get(&1).await, Some("1".to_string()));

        // does not call the function again if it is cached
        assert_eq!(cached(1).await, Ok("1".to_string()));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
