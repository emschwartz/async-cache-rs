use chrono::{DateTime, Duration, Utc};
use skiplist::SkipMap;
use std::{collections::HashMap, hash::Hash};

pub struct SyncCache<Key, Val> {
    // Map of the key to the cached value and the expiry
    map: HashMap<Key, (Val, DateTime<Utc>)>,
    // Sorted map of expiry date to the key used for
    // determining the next value to expire
    // TODO bucket the expiries into groups for more efficient removal
    expiries: SkipMap<DateTime<Utc>, Key>,
}

impl<Key, Val> SyncCache<Key, Val>
where
    Key: Eq + Hash + Clone,
{
    #[inline]
    pub fn new() -> Self {
        SyncCache {
            map: HashMap::new(),
            expiries: SkipMap::new(),
        }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        SyncCache {
            map: HashMap::with_capacity(capacity),
            expiries: SkipMap::new(),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.map.clear();
        self.expiries.clear();
    }

    #[inline]
    pub fn get(&self, key: &Key) -> Option<&Val> {
        self.map.get(key).map(|(val, _)| val)
    }

    // Insert the given key and value.
    // If the cache is at capacity, the item expiring soonest will be evicted.
    // TODO if you set a value to a lower ttl than it was before, should
    // we respect the longest one or the most recent one?
    #[inline]
    pub fn set(&mut self, key: Key, value: Val, ttl: Duration) -> bool {
        let expiry = Utc::now() + ttl;

        let had_key = if let Some((_, expiry)) = self.map.get(&key) {
            self.expiries.remove(expiry);
            true
        } else {
            // if the map is at capacity, evict one entry before inserting
            if self.map.capacity() == self.map.len() {
                self.remove_next_expiring();
            }
            false
        };

        self.map.insert(key.clone(), (value, expiry.clone()));
        self.expiries.insert(expiry, key);

        had_key
    }

    #[inline]
    pub fn has_expired_items(&self) -> bool {
        if let Some((expiry, _)) = self.expiries.front() {
            expiry <= &Utc::now()
        } else {
            false
        }
    }

    #[inline]
    pub fn remove_expired_items(&mut self) -> bool {
        let mut removed_items = false;
        while self.has_expired_items() {
            if self.remove_next_expiring().is_some() {
                removed_items = true;
            }
        }
        removed_items
    }

    // Removes the next expiring item and returns it.
    #[inline]
    fn remove_next_expiring(&mut self) -> Option<(Key, Val)> {
        if let Some((_, key)) = self.expiries.pop_front() {
            if let Some((val, _)) = self.map.remove(&key) {
                return Some((key, val));
            }
        }
        None
    }
}
