use chrono::{DateTime, Duration, Utc};
use skiplist::SkipMap;
use std::collections::{HashMap, LinkedList};
use std::hash::Hash;
use std::iter::{self, FromIterator};

pub struct SyncCache<Key, Val> {
    // Map of the key to the cached value and the expiry
    map: HashMap<Key, (Val, DateTime<Utc>)>,
    // Sorted map of expiry date to the key used for
    // determining the next value to expire
    // TODO bucket the expiries into groups for more efficient removal
    expiries: SkipMap<DateTime<Utc>, LinkedList<Key>>,
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
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    #[inline]
    pub fn get(&self, key: &Key) -> Option<&Val> {
        self.map.get(key).map(|(val, _)| val)
    }

    // Insert the given key and value.
    // If the cache is at capacity, the item expiring soonest will be evicted.
    // TODO if you set a value to a lower ttl than it was before, should
    // we respect the longest one or the most recent one?
    pub fn set(&mut self, key: Key, value: Val, ttl: Duration) -> bool {
        let expiry = Utc::now() + ttl;

        // Remove the previous expiry if there was one
        let had_key = if let Some((_, expiry)) = self.map.get(&key) {
            self.expiries.remove(expiry);
            true
        } else {
            false
        };

        // If the map is at capacity, evict one entry before inserting
        if !had_key && self.map.len() == self.map.capacity() {
            self.evict();
        }

        self.map.insert(key.clone(), (value, expiry.clone()));

        // In the expiries map, keys are stored in a linked list in case there are
        // multiple keys with the exact same expiry. In most cases, there will not
        // already be a set of keys with the given expiry, so we can just insert
        // the list directly. If there was a key list already with that expiry,
        // we append that list to the one we just inserted.
        if let Some(mut prev_key_list) = self
            .expiries
            .insert(expiry, LinkedList::from_iter(iter::once(key)))
        {
            let key_list = self.expiries.get_mut(&expiry).unwrap();
            key_list.append(&mut prev_key_list);
        }

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

    pub fn remove_expired_items(&mut self) -> bool {
        let mut removed_items = false;
        while self.has_expired_items() {
            let (_, expired) = self.expiries.pop_front().unwrap();

            // Remove each expired key from the map
            for key in expired.iter() {
                if self.map.remove(&key).is_some() {
                    removed_items = true;
                }
            }
        }
        removed_items
    }

    // Remove the item expiring soonest.
    // If there are multiple items with the same expiry,
    // only remove one of them.
    #[inline]
    fn evict(&mut self) {
        let key = if let Some((_, key_list)) = self.expiries.front_mut() {
            let key = key_list.pop_front();

            // If there was only one key expiring at this time, remove
            // the entry from the expiries
            if key_list.is_empty() {
                drop(key_list);
                self.expiries.pop_front();
            }
            key
        } else {
            None
        };

        if let Some(key) = key {
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
    fn has_expired_items() {
        let mut cache = SyncCache::with_capacity(5);
        assert_eq!(cache.has_expired_items(), false);

        cache.set("a", 1, Duration::hours(1));
        assert_eq!(cache.has_expired_items(), false);

        cache.set("b", 2, Duration::milliseconds(-1));
        assert_eq!(cache.has_expired_items(), true);

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

        cache.remove_expired_items();
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
}
