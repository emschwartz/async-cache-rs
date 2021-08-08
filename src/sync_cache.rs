use chrono::{DateTime, Duration, Utc};
use skiplist::SkipMap;
use std::{collections::HashMap, hash::Hash};

pub struct SyncCache<Key, Val> {
    // Map of the key to the cached value and the number of entries
    // in self.expiries that point to this key.
    // (the usize will only ever be >1 if `set` is called multiple
    // times for the same key before the expiry of the initial `set` call)
    map: HashMap<Key, (Val, usize)>,
    // TODO bucket the expiries into groups for more efficient removal
    expiries: SkipMap<DateTime<Utc>, Key>,
}

impl<Key, Val> SyncCache<Key, Val>
where
    Key: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        SyncCache {
            // TODO should we actually reserve the full capacity?
            map: HashMap::new(),
            expiries: SkipMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        SyncCache {
            // TODO should we actually reserve the full capacity?
            map: HashMap::with_capacity(capacity),
            expiries: SkipMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.expiries.clear();
    }

    pub fn get(&self, key: &Key) -> Option<&Val> {
        self.map.get(key).map(|(val, _)| val)
    }

    // Insert the given key and value.
    // If the cache is at capacity, the item expiring soonest will be evicted.
    pub fn set(&mut self, key: Key, value: Val, ttl: Duration) -> bool {
        let had_key = if let Some(entry) = self.map.get_mut(&key) {
            // overwrite the cached value with the new one
            entry.0 = value;
            // increment the count of how many entries in self.expiries point to this key
            // (because we are about to add one below)
            entry.1 += 1;
            true
        } else {
            // if the map is at capacity, evict one entry before inserting
            if self.map.capacity() == self.map.len() {
                self.evict();
            }
            self.map.insert(key.clone(), (value, 1));
            false
        };

        let expiry = Utc::now() + ttl;
        self.expiries.insert(expiry, key);

        had_key
    }

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
            if self.maybe_remove_next().is_some() {
                removed_items = true;
            }
        }
        removed_items
    }

    // Removes the next expiring item and returns it.
    // This will not actually remove the item from the map if
    // the same key has been set again with a later expiry.
    fn maybe_remove_next(&mut self) -> Option<(Key, Val)> {
        if let Some((_, key)) = self.expiries.pop_front() {
            let remaining = if let Some(entry) = self.map.get_mut(&key) {
                entry.1.saturating_sub(1)
            } else {
                0
            };

            if remaining == 0 {
                self.map.remove(&key).map(|(val, _)| (key, val))
            } else {
                None
            }
        } else {
            None
        }
    }

    // Evict the item expiring soonest
    fn evict(&mut self) -> Option<(Key, Val)> {
        // Starting with the entry expiring next, start removing items from the expiry list
        // and break as soon as an entry is actually removed from the map.
        while !self.expiries.is_empty() {
            if let Some(entry) = self.maybe_remove_next() {
                return Some(entry);
            }
        }
        return None;
    }
}
