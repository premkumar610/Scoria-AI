// local_engine/src/cache/lru.rs

use std::{
    collections::{HashMap, LinkedList},
    hash::Hash,
    time::{Duration, Instant},
};

/// LRU Cache entry with expiration and access tracking
struct CacheEntry<V> {
    value: V,
    last_accessed: Instant,
    expires_at: Option<Instant>,
}

/// Main LRU Cache structure
pub struct LRUCache<K, V> {
    capacity: usize,
    store: HashMap<K, CacheEntry<V>>,
    access_order: LinkedList<K>,
    default_ttl: Option<Duration>,
}

impl<K, V> LRUCache<K, V>
where
    K: Eq + Hash + Clone,
{
    /// Create new LRU Cache with capacity and optional TTL
    pub fn new(capacity: usize, default_ttl: Option<Duration>) -> Self {
        Self {
            capacity,
            store: HashMap::with_capacity(capacity),
            access_order: LinkedList::new(),
            default_ttl,
        }
    }

    /// Insert key-value pair with optional custom TTL
    pub fn insert(&mut self, key: K, value: V, custom_ttl: Option<Duration>) -> Option<V> {
        let expires_at = custom_ttl
            .or(self.default_ttl)
            .map(|d| Instant::now() + d);

        let entry = CacheEntry {
            value,
            last_accessed: Instant::now(),
            expires_at,
        };

        // Evict expired entries first
        self.evict_expired();

        let old_value = self.store.insert(key.clone(), entry);
        self.update_access_order(&key);

        if self.store.len() > self.capacity {
            self.evict_lru();
        }

        old_value.map(|e| e.value)
    }

    /// Get mutable reference to value, updating access time
    pub fn get(&mut self, key: &K) -> Option<&mut V> {
        if let Some(entry) = self.store.get_mut(key) {
            if entry.is_expired() {
                self.store.remove(key);
                return None;
            }

            entry.last_accessed = Instant::now();
            self.update_access_order(key);
            Some(&mut entry.value)
        } else {
            None
        }
    }

    /// Remove least recently used entry
    fn evict_lru(&mut self) -> Option<(K, V)> {
        while let Some(key) = self.access_order.pop_front() {
            if let Some(entry) = self.store.remove(&key) {
                if !entry.is_expired() {
                    return Some((key, entry.value));
                }
            }
        }
        None
    }

    /// Evict all expired entries
    fn evict_expired(&mut self) {
        let now = Instant::now();
        self.store.retain(|k, v| {
            let retain = !v.is_expired_at(now);
            if !retain {
                self.access_order.retain(|x| x != k);
            }
            retain
        });
    }

    /// Update access order tracking
    fn update_access_order(&mut self, key: &K) {
        self.access_order.retain(|k| k != key);
        self.access_order.push_back(key.clone());
    }
}

impl<V> CacheEntry<V> {
    fn is_expired(&self) -> bool {
        self.is_expired_at(Instant::now())
    }

    fn is_expired_at(&self, timestamp: Instant) -> bool {
        self.expires_at
            .map(|expiry| timestamp > expiry)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_basic_lru() {
        let mut cache = LRUCache::new(2, None);
        
        cache.insert("a", 1, None);
        cache.insert("b", 2, None);
        
        assert_eq!(cache.get(&"a"), Some(&mut 1));
        
        cache.insert("c", 3, None);
        
        assert_eq!(cache.get(&"b"), None); // b should be evicted
        assert_eq!(cache.get(&"a"), Some(&mut 1));
        assert_eq!(cache.get(&"c"), Some(&mut 3));
    }

    #[test]
    fn test_ttl_expiration() {
        let mut cache = LRUCache::new(3, Some(Duration::from_secs(1)));
        
        cache.insert(1, "a", None);
        cache.insert(2, "b", Some(Duration::from_millis(500)));
        
        sleep(Duration::from_secs(1));
        
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), None);
    }

    #[test]
    fn test_custom_ttl_override() {
        let mut cache = LRUCache::new(2, Some(Duration::from_secs(1)));
        
        cache.insert("a", 1, Some(Duration::from_secs(3)));
        cache.insert("b", 2, None); // Uses default 1s TTL
        
        sleep(Duration::from_secs(2));
        
        assert_eq!(cache.get(&"a"), Some(&mut 1)); // Still valid
        assert_eq!(cache.get(&"b"), None); // Expired
    }
}
