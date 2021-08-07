use chrono::{DateTime, Duration, Utc};
use skiplist::SkipMap;
use std::{collections::HashMap, hash::Hash};

pub struct SyncCache<Key, Val> {
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

    pub fn set(&mut self, key: Key, value: Val, ttl: Duration) -> bool {
        let had_key = if let Some(entry) = self.map.get_mut(&key) {
            entry.1 += 1;
            true
        } else {
            // TODO check capacity first and evict items if necessary
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

    pub fn remove_expired_items(&mut self) {
        while self.has_expired_items() {
            let (_, key) = self.expiries.pop_front().unwrap();

            let remaining = if let Some(entry) = self.map.get_mut(&key) {
                entry.1.saturating_sub(1)
            } else {
                unreachable!();
            };
            if remaining == 0 {
                self.map.remove(&key);
            }
        }
    }
}
