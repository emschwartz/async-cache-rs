use chrono::{DateTime, Duration, DurationRound, Utc};
use priority_queue::PriorityQueue;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::hash::Hash;

// Synchronous, non-thread-safe cache backed by a HashMap
// and a SkipList of key expirations
pub struct SyncCache<Key: Eq + Hash, Val> {
    // Map of the key to the cached value and the expiry
    map: HashMap<Key, (Val, DateTime<Utc>)>,
    // Priority queue tracking keys' expiry times
    expiries: PriorityQueue<Key, Reverse<DateTime<Utc>>>,
}

impl<Key, Val> SyncCache<Key, Val>
where
    Key: Eq + Hash + Clone,
{
    #[inline]
    pub fn new() -> Self {
        SyncCache {
            map: HashMap::new(),
            expiries: PriorityQueue::new(),
        }
    }

    #[inline]
    // Note: this creates a HashMap and a PriorityQueue each with the given capacity
    pub fn with_capacity(capacity: usize) -> Self {
        SyncCache {
            map: HashMap::with_capacity(capacity),
            expiries: PriorityQueue::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.map.clear();
        self.expiries.clear();
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    #[inline]
    pub fn get(&self, key: &Key) -> Option<&Val> {
        self.map.get(key).map(|(val, _)| val)
    }

    #[inline]
    pub(crate) fn get_with_expiry(&self, key: &Key) -> Option<&(Val, DateTime<Utc>)> {
        self.map.get(key)
    }

    // Insert the given key and value.
    //
    // If the cache is at capacity, the item expiring soonest will be evicted.
    pub fn set(&mut self, key: Key, value: Val, ttl: Duration) -> bool {
        // Round the date down to the nearest 10 milliseconds
        let expiry = (Utc::now() + ttl)
            .duration_trunc(Duration::milliseconds(10))
            .unwrap();

        // If the map is at capacity, evict one entry before inserting
        let had_key = self.map.contains_key(&key);
        if !had_key && self.map.len() == self.map.capacity() {
            self.evict();
        }

        self.map.insert(key.clone(), (value, expiry.clone()));
        self.expiries.push(key, Reverse(expiry));

        had_key
    }

    #[inline]
    pub fn remove(&mut self, key: &Key) -> bool {
        self.expiries.remove(key);
        self.map.remove(&key).is_some()
    }

    #[inline]
    pub fn has_expired_items(&self) -> bool {
        if let Some((_, expiry)) = self.expiries.peek() {
            expiry.0 <= Utc::now()
        } else {
            false
        }
    }

    pub fn remove_expired_items(&mut self) -> bool {
        let mut removed_items = false;
        while self.has_expired_items() {
            let (key, _) = self.expiries.pop().unwrap();
            if self.map.remove(&key).is_some() {
                removed_items = true;
            }
        }
        removed_items
    }

    // Remove the item expiring soonest.
    // If there are multiple items with the same expiry,
    // only remove one of them.
    #[inline]
    fn evict(&mut self) {
        if let Some((key, _)) = self.expiries.pop() {
            self.map.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_get_set() {
        let mut cache = SyncCache::with_capacity(5);
        cache.set("a", 1, Duration::hours(1));
        cache.set("b", 2, Duration::hours(1));

        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(cache.get(&"b"), Some(&2));
    }

    #[test]
    fn overwriting_key() {
        let mut cache = SyncCache::with_capacity(5);
        cache.set("a", 1, Duration::hours(1));
        cache.set("a", 2, Duration::hours(1));

        assert_eq!(cache.get(&"a"), Some(&2));

        // even if the duration is shorter
        cache.set("a", 3, Duration::seconds(10));
        assert_eq!(cache.get(&"a"), Some(&3));
    }

    #[test]
    fn has_expired_items() {
        let mut cache = SyncCache::with_capacity(5);
        assert_eq!(cache.has_expired_items(), false);

        cache.set("a", 1, Duration::hours(1));
        assert_eq!(cache.has_expired_items(), false);

        cache.set("b", 2, Duration::milliseconds(-100));
        assert_eq!(cache.has_expired_items(), true);
        cache.remove(&"b");
        assert_eq!(cache.has_expired_items(), false);

        cache.set("c", 2, Duration::hours(-1));
        assert_eq!(cache.has_expired_items(), true);
    }

    #[test]
    fn remove_expired_items() {
        let mut cache = SyncCache::with_capacity(5);
        cache.set("a", 1, Duration::hours(-1));
        cache.set("b", 2, Duration::hours(1));
        cache.set("c", 3, Duration::milliseconds(-1));
        cache.set("d", 4, Duration::days(1));

        assert_eq!(cache.remove_expired_items(), true);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"b"), Some(&2));
        assert_eq!(cache.get(&"c"), None);
        assert_eq!(cache.get(&"d"), Some(&4));
    }

    #[test]
    fn eviction() {
        let mut cache = SyncCache::with_capacity(3);
        cache.set("a", 1, Duration::hours(1));
        cache.set("b", 2, Duration::minutes(1));
        cache.set("c", 3, Duration::seconds(1));
        cache.set("d", 4, Duration::days(1));

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&"c"), None);
        assert_eq!(cache.get(&"d"), Some(&4));
    }

    #[test]
    fn multiple_keys_same_expiry() {
        let mut cache = SyncCache::with_capacity(3);
        cache.set("a", 1, Duration::hours(-1));
        cache.set("b", 2, Duration::hours(-1));
        cache.set("c", 3, Duration::hours(1));

        assert_eq!(cache.remove_expired_items(), true);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"b"), None);
    }
}
