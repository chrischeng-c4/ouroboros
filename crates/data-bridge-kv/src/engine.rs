//! Sharded KV storage engine
//!
//! Partitions keyspace into multiple shards for multi-core scalability.
//! Each shard uses RwLock for concurrent reads and exclusive writes.

use crate::error::KvError;
use crate::types::{KvKey, KvValue};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

/// Default number of shards (power of 2 for efficient modulo)
const DEFAULT_NUM_SHARDS: usize = 256;

/// Entry in the KV store with metadata
#[derive(Debug, Clone)]
pub struct Entry {
    /// The stored value
    pub value: KvValue,
    /// When the entry was created
    pub created_at: Instant,
    /// Optional expiration time (TTL)
    pub expires_at: Option<Instant>,
    /// Version for CAS operations
    pub version: u64,
}

impl Entry {
    /// Create a new entry
    pub fn new(value: KvValue, ttl: Option<Duration>) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            expires_at: ttl.map(|d| now + d),
            version: 1,
        }
    }

    /// Check if entry has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|exp| Instant::now() >= exp).unwrap_or(false)
    }
}

/// A single shard containing a portion of the keyspace
pub struct Shard {
    data: RwLock<HashMap<String, Entry>>,
}

impl Shard {
    /// Create a new empty shard
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }

    /// Get a value by key (returns None if expired)
    pub fn get(&self, key: &str) -> Option<Entry> {
        let guard = self.data.read();
        guard.get(key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.clone())
            }
        })
    }

    /// Set a value with optional TTL
    pub fn set(&self, key: String, value: KvValue, ttl: Option<Duration>) -> Option<Entry> {
        let mut guard = self.data.write();
        // Clean up expired entry if exists
        if let Some(existing) = guard.get(&key) {
            if existing.is_expired() {
                guard.remove(&key);
            }
        }
        guard.insert(key, Entry::new(value, ttl))
    }

    /// Delete a key, returns the old entry if existed
    pub fn delete(&self, key: &str) -> Option<Entry> {
        let mut guard = self.data.write();
        guard.remove(key)
    }

    /// Check if key exists (and not expired)
    pub fn exists(&self, key: &str) -> bool {
        let guard = self.data.read();
        guard.get(key).map(|e| !e.is_expired()).unwrap_or(false)
    }

    /// Atomic increment for Int values
    pub fn incr(&self, key: &str, delta: i64) -> Result<i64, KvError> {
        let mut guard = self.data.write();

        match guard.get_mut(key) {
            Some(entry) if !entry.is_expired() => {
                match &mut entry.value {
                    KvValue::Int(n) => {
                        *n = n.saturating_add(delta);
                        entry.version += 1;
                        Ok(*n)
                    }
                    other => Err(KvError::TypeMismatch {
                        expected: "Int".to_string(),
                        actual: format!("{:?}", std::mem::discriminant(other)),
                    }),
                }
            }
            _ => {
                // Key doesn't exist, create with delta as initial value
                guard.insert(key.to_string(), Entry::new(KvValue::Int(delta), None));
                Ok(delta)
            }
        }
    }

    /// Compare-And-Swap: atomically update if current value matches expected
    pub fn cas(&self, key: &str, expected: &KvValue, new_value: KvValue, ttl: Option<Duration>) -> Result<bool, KvError> {
        let mut guard = self.data.write();

        match guard.get_mut(key) {
            Some(entry) if !entry.is_expired() => {
                if &entry.value == expected {
                    entry.value = new_value;
                    entry.version += 1;
                    if let Some(d) = ttl {
                        entry.expires_at = Some(Instant::now() + d);
                    }
                    Ok(true)
                } else {
                    Ok(false)  // Value didn't match
                }
            }
            _ => Err(KvError::KeyNotFound(key.to_string())),
        }
    }

    /// Get entry count (including expired - for stats)
    pub fn len(&self) -> usize {
        self.data.read().len()
    }

    /// Check if shard is empty
    pub fn is_empty(&self) -> bool {
        self.data.read().is_empty()
    }

    /// Remove all expired entries, returns count removed
    pub fn cleanup_expired(&self) -> usize {
        let mut guard = self.data.write();
        let before = guard.len();
        guard.retain(|_, entry| !entry.is_expired());
        before - guard.len()
    }
}

impl Default for Shard {
    fn default() -> Self {
        Self::new()
    }
}

/// High-performance sharded KV engine
pub struct KvEngine {
    shards: Vec<Shard>,
    num_shards: usize,
}

impl KvEngine {
    /// Create a new KV engine with default number of shards (256)
    pub fn new() -> Self {
        Self::with_shards(DEFAULT_NUM_SHARDS)
    }

    /// Create a new KV engine with specified number of shards
    pub fn with_shards(num_shards: usize) -> Self {
        let shards = (0..num_shards).map(|_| Shard::new()).collect();
        Self { shards, num_shards }
    }

    /// Get the shard for a given key
    #[inline]
    fn shard_for_key(&self, key: &str) -> &Shard {
        let hash = Self::hash_key(key);
        let idx = hash as usize % self.num_shards;
        &self.shards[idx]
    }

    /// Hash a key to u64
    #[inline]
    fn hash_key(key: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Get a value by key
    pub fn get(&self, key: &KvKey) -> Option<KvValue> {
        self.shard_for_key(key.as_str())
            .get(key.as_str())
            .map(|entry| entry.value)
    }

    /// Set a value with optional TTL
    pub fn set(&self, key: &KvKey, value: KvValue, ttl: Option<Duration>) {
        self.shard_for_key(key.as_str())
            .set(key.as_str().to_string(), value, ttl);
    }

    /// Delete a key
    pub fn delete(&self, key: &KvKey) -> bool {
        self.shard_for_key(key.as_str())
            .delete(key.as_str())
            .is_some()
    }

    /// Check if key exists
    pub fn exists(&self, key: &KvKey) -> bool {
        self.shard_for_key(key.as_str()).exists(key.as_str())
    }

    /// Atomic increment
    pub fn incr(&self, key: &KvKey, delta: i64) -> Result<i64, KvError> {
        self.shard_for_key(key.as_str()).incr(key.as_str(), delta)
    }

    /// Atomic decrement (convenience wrapper)
    pub fn decr(&self, key: &KvKey, delta: i64) -> Result<i64, KvError> {
        self.incr(key, -delta)
    }

    /// Compare-And-Swap
    pub fn cas(&self, key: &KvKey, expected: &KvValue, new_value: KvValue, ttl: Option<Duration>) -> Result<bool, KvError> {
        self.shard_for_key(key.as_str())
            .cas(key.as_str(), expected, new_value, ttl)
    }

    /// Get total entry count across all shards
    pub fn len(&self) -> usize {
        self.shards.iter().map(|s| s.len()).sum()
    }

    /// Check if engine is empty
    pub fn is_empty(&self) -> bool {
        self.shards.iter().all(|s| s.is_empty())
    }

    /// Get number of shards
    pub fn num_shards(&self) -> usize {
        self.num_shards
    }

    /// Cleanup expired entries across all shards, returns total removed
    pub fn cleanup_expired(&self) -> usize {
        self.shards.iter().map(|s| s.cleanup_expired()).sum()
    }
}

impl Default for KvEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::thread;

    #[test]
    fn test_basic_set_get() {
        let engine = KvEngine::new();
        let key = KvKey::new("test_key").unwrap();

        engine.set(&key, KvValue::String("hello".to_string()), None);

        let result = engine.get(&key);
        assert_eq!(result, Some(KvValue::String("hello".to_string())));
    }

    #[test]
    fn test_get_nonexistent() {
        let engine = KvEngine::new();
        let key = KvKey::new("nonexistent").unwrap();

        assert_eq!(engine.get(&key), None);
    }

    #[test]
    fn test_delete() {
        let engine = KvEngine::new();
        let key = KvKey::new("to_delete").unwrap();

        engine.set(&key, KvValue::Int(42), None);
        assert!(engine.exists(&key));

        assert!(engine.delete(&key));
        assert!(!engine.exists(&key));
    }

    #[test]
    fn test_exists() {
        let engine = KvEngine::new();
        let key = KvKey::new("exists_key").unwrap();

        assert!(!engine.exists(&key));
        engine.set(&key, KvValue::Int(1), None);
        assert!(engine.exists(&key));
    }

    #[test]
    fn test_incr_existing() {
        let engine = KvEngine::new();
        let key = KvKey::new("counter").unwrap();

        engine.set(&key, KvValue::Int(10), None);

        let result = engine.incr(&key, 5).unwrap();
        assert_eq!(result, 15);

        let result = engine.decr(&key, 3).unwrap();
        assert_eq!(result, 12);
    }

    #[test]
    fn test_incr_nonexistent() {
        let engine = KvEngine::new();
        let key = KvKey::new("new_counter").unwrap();

        let result = engine.incr(&key, 100).unwrap();
        assert_eq!(result, 100);
    }

    #[test]
    fn test_incr_type_mismatch() {
        let engine = KvEngine::new();
        let key = KvKey::new("string_key").unwrap();

        engine.set(&key, KvValue::String("not a number".to_string()), None);

        let result = engine.incr(&key, 1);
        assert!(matches!(result, Err(KvError::TypeMismatch { .. })));
    }

    #[test]
    fn test_cas_success() {
        let engine = KvEngine::new();
        let key = KvKey::new("cas_key").unwrap();

        engine.set(&key, KvValue::String("initial".to_string()), None);

        let result = engine.cas(
            &key,
            &KvValue::String("initial".to_string()),
            KvValue::String("updated".to_string()),
            None,
        ).unwrap();

        assert!(result);
        assert_eq!(engine.get(&key), Some(KvValue::String("updated".to_string())));
    }

    #[test]
    fn test_cas_failure() {
        let engine = KvEngine::new();
        let key = KvKey::new("cas_key").unwrap();

        engine.set(&key, KvValue::String("actual".to_string()), None);

        let result = engine.cas(
            &key,
            &KvValue::String("wrong_expected".to_string()),
            KvValue::String("new".to_string()),
            None,
        ).unwrap();

        assert!(!result);
        assert_eq!(engine.get(&key), Some(KvValue::String("actual".to_string())));
    }

    #[test]
    fn test_cas_nonexistent() {
        let engine = KvEngine::new();
        let key = KvKey::new("missing").unwrap();

        let result = engine.cas(
            &key,
            &KvValue::Int(0),
            KvValue::Int(1),
            None,
        );

        assert!(matches!(result, Err(KvError::KeyNotFound(_))));
    }

    #[test]
    fn test_ttl_expiration() {
        let engine = KvEngine::new();
        let key = KvKey::new("ttl_key").unwrap();

        // Set with 10ms TTL
        engine.set(&key, KvValue::Int(42), Some(Duration::from_millis(10)));

        // Should exist immediately
        assert!(engine.exists(&key));

        // Wait for expiration
        thread::sleep(Duration::from_millis(20));

        // Should be gone
        assert!(!engine.exists(&key));
        assert_eq!(engine.get(&key), None);
    }

    #[test]
    fn test_decimal_value() {
        let engine = KvEngine::new();
        let key = KvKey::new("decimal_key").unwrap();

        let decimal = Decimal::new(12345, 2); // 123.45
        engine.set(&key, KvValue::Decimal(decimal), None);

        let result = engine.get(&key);
        assert_eq!(result, Some(KvValue::Decimal(decimal)));
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;

        let engine = Arc::new(KvEngine::new());
        let mut handles = vec![];

        // Spawn 10 threads, each incrementing 100 different keys
        for _t in 0..10 {
            let engine = Arc::clone(&engine);
            handles.push(thread::spawn(move || {
                for i in 0..100 {
                    let key = KvKey::new(format!("key_{}", i)).unwrap();
                    engine.incr(&key, 1).unwrap();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Each of 100 keys should have been incremented 10 times
        for i in 0..100 {
            let key = KvKey::new(format!("key_{}", i)).unwrap();
            assert_eq!(engine.get(&key), Some(KvValue::Int(10)));
        }
    }

    #[test]
    fn test_sharding_distribution() {
        let engine = KvEngine::with_shards(16);

        // Insert 1000 keys
        for i in 0..1000 {
            let key = KvKey::new(format!("key_{}", i)).unwrap();
            engine.set(&key, KvValue::Int(i), None);
        }

        // Check that keys are distributed across shards
        let mut non_empty_shards = 0;
        for shard in &engine.shards {
            if !shard.is_empty() {
                non_empty_shards += 1;
            }
        }

        // With 1000 keys and 16 shards, we expect most shards to have keys
        assert!(non_empty_shards >= 14, "Poor distribution: only {} shards have data", non_empty_shards);
    }

    #[test]
    fn test_cleanup_expired() {
        let engine = KvEngine::new();

        // Add some keys with short TTL
        for i in 0..10 {
            let key = KvKey::new(format!("expire_{}", i)).unwrap();
            engine.set(&key, KvValue::Int(i), Some(Duration::from_millis(5)));
        }

        // Add some keys without TTL
        for i in 0..10 {
            let key = KvKey::new(format!("persist_{}", i)).unwrap();
            engine.set(&key, KvValue::Int(i), None);
        }

        assert_eq!(engine.len(), 20);

        // Wait for expiration
        thread::sleep(Duration::from_millis(10));

        // Cleanup
        let removed = engine.cleanup_expired();
        assert_eq!(removed, 10);
        assert_eq!(engine.len(), 10);
    }
}
