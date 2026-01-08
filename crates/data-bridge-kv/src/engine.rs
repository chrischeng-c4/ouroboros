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

    /// Set if not exists (atomic)
    pub fn setnx(&self, key: String, value: KvValue, ttl: Option<Duration>) -> bool {
        let mut guard = self.data.write();

        // Check if key exists and not expired
        if let Some(entry) = guard.get(&key) {
            if !entry.is_expired() {
                return false; // Key exists, operation fails
            }
        }

        // Key doesn't exist or expired - set it
        guard.insert(key, Entry::new(value, ttl));
        true
    }

    /// Acquire a lock with owner ID and TTL
    pub fn lock(&self, key: String, owner: String, ttl: Duration) -> bool {
        self.setnx(key, KvValue::String(owner), Some(ttl))
    }

    /// Release a lock (only if owned)
    pub fn unlock(&self, key: &str, owner: &str) -> Result<bool, KvError> {
        let mut guard = self.data.write();

        match guard.get(key) {
            Some(entry) if !entry.is_expired() => {
                match &entry.value {
                    KvValue::String(stored_owner) if stored_owner == owner => {
                        guard.remove(key);
                        Ok(true)
                    }
                    KvValue::String(stored_owner) => {
                        Err(KvError::LockOwnerMismatch {
                            expected: owner.to_string(),
                            actual: stored_owner.clone(),
                        })
                    }
                    _ => Err(KvError::TypeMismatch {
                        expected: "String (lock owner)".to_string(),
                        actual: "other type".to_string(),
                    }),
                }
            }
            _ => Ok(false), // Lock not held or expired
        }
    }

    /// Extend lock TTL (only if owned)
    pub fn extend_lock(&self, key: &str, owner: &str, ttl: Duration) -> Result<bool, KvError> {
        let mut guard = self.data.write();

        match guard.get_mut(key) {
            Some(entry) if !entry.is_expired() => {
                match &entry.value {
                    KvValue::String(stored_owner) if stored_owner == owner => {
                        entry.expires_at = Some(Instant::now() + ttl);
                        entry.version += 1;
                        Ok(true)
                    }
                    KvValue::String(stored_owner) => {
                        Err(KvError::LockOwnerMismatch {
                            expected: owner.to_string(),
                            actual: stored_owner.clone(),
                        })
                    }
                    _ => Err(KvError::TypeMismatch {
                        expected: "String (lock owner)".to_string(),
                        actual: "other type".to_string(),
                    }),
                }
            }
            _ => Ok(false), // Lock not held or expired
        }
    }

    /// Export all entries (for persistence/snapshots)
    pub fn export_all(&self) -> HashMap<String, Entry> {
        let guard = self.data.read();
        guard.clone()
    }

    /// Import entries (for recovery from snapshots)
    pub fn import_all(&self, entries: HashMap<String, Entry>) {
        let mut guard = self.data.write();
        guard.extend(entries);
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
    /// Optional persistence handle for WAL and snapshots
    /// Uses RwLock for interior mutability (enables setting after Arc wrapping)
    persistence: RwLock<Option<std::sync::Arc<crate::persistence::handle::PersistenceHandle>>>,
}

impl KvEngine {
    /// Create a new KV engine with default number of shards (256)
    pub fn new() -> Self {
        Self::with_shards(DEFAULT_NUM_SHARDS)
    }

    /// Create a new KV engine with specified number of shards
    pub fn with_shards(num_shards: usize) -> Self {
        let shards = (0..num_shards).map(|_| Shard::new()).collect();
        Self {
            shards,
            num_shards,
            persistence: RwLock::new(None),
        }
    }

    /// Enable persistence on this engine
    ///
    /// Can be called after wrapping in Arc - uses interior mutability.
    /// Sets up WAL logging and periodic snapshots.
    pub fn enable_persistence(
        &self,
        persistence_handle: std::sync::Arc<crate::persistence::handle::PersistenceHandle>,
    ) {
        *self.persistence.write() = Some(persistence_handle);
    }

    /// Log an operation to WAL (if persistence is enabled)
    #[inline]
    fn log_wal(&self, op: crate::persistence::format::WalOp) {
        if let Some(ref persistence) = *self.persistence.read() {
            persistence.log_operation(op);
        }
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
        // Log to WAL first (non-blocking)
        self.log_wal(crate::persistence::format::WalOp::Set {
            key: key.as_str().to_string(),
            value: value.clone(),
            ttl,
        });

        // Apply to in-memory store
        self.shard_for_key(key.as_str())
            .set(key.as_str().to_string(), value, ttl);
    }

    /// Delete a key
    pub fn delete(&self, key: &KvKey) -> bool {
        // Log to WAL
        self.log_wal(crate::persistence::format::WalOp::Delete {
            key: key.as_str().to_string(),
        });

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
        // Log to WAL
        self.log_wal(crate::persistence::format::WalOp::Incr {
            key: key.as_str().to_string(),
            delta,
        });

        self.shard_for_key(key.as_str()).incr(key.as_str(), delta)
    }

    /// Atomic decrement (convenience wrapper)
    pub fn decr(&self, key: &KvKey, delta: i64) -> Result<i64, KvError> {
        // Log to WAL
        self.log_wal(crate::persistence::format::WalOp::Decr {
            key: key.as_str().to_string(),
            delta,
        });

        // Call shard directly to avoid double-logging
        self.shard_for_key(key.as_str()).incr(key.as_str(), -delta)
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

    /// Set if not exists
    pub fn setnx(&self, key: &KvKey, value: KvValue, ttl: Option<Duration>) -> bool {
        // Log to WAL
        self.log_wal(crate::persistence::format::WalOp::SetNx {
            key: key.as_str().to_string(),
            value: value.clone(),
            ttl,
        });

        self.shard_for_key(key.as_str())
            .setnx(key.as_str().to_string(), value, ttl)
    }

    /// Acquire a lock
    pub fn lock(&self, key: &KvKey, owner: &str, ttl: Duration) -> bool {
        // Log to WAL
        self.log_wal(crate::persistence::format::WalOp::Lock {
            key: key.as_str().to_string(),
            owner: owner.to_string(),
            ttl,
        });

        self.shard_for_key(key.as_str())
            .lock(key.as_str().to_string(), owner.to_string(), ttl)
    }

    /// Release a lock
    pub fn unlock(&self, key: &KvKey, owner: &str) -> Result<bool, KvError> {
        // Log to WAL
        self.log_wal(crate::persistence::format::WalOp::Unlock {
            key: key.as_str().to_string(),
            owner: owner.to_string(),
        });

        self.shard_for_key(key.as_str())
            .unlock(key.as_str(), owner)
    }

    /// Extend lock TTL
    pub fn extend_lock(&self, key: &KvKey, owner: &str, ttl: Duration) -> Result<bool, KvError> {
        // Log to WAL
        self.log_wal(crate::persistence::format::WalOp::ExtendLock {
            key: key.as_str().to_string(),
            owner: owner.to_string(),
            ttl,
        });

        self.shard_for_key(key.as_str())
            .extend_lock(key.as_str(), owner, ttl)
    }

    // ==================== Batch Operations ====================

    /// Get multiple values by keys (MGET)
    ///
    /// Returns a vector of Option<KvValue> in the same order as the input keys.
    /// Missing or expired keys return None.
    ///
    /// # Performance
    /// This is more efficient than multiple GET calls as it:
    /// - Reduces function call overhead
    /// - Allows better CPU cache utilization
    /// - Can be optimized by the compiler
    ///
    /// # Example
    /// ```
    /// use data_bridge_kv::engine::KvEngine;
    /// use data_bridge_kv::types::{KvKey, KvValue};
    ///
    /// let engine = KvEngine::new();
    /// let key1 = KvKey::new("key1").unwrap();
    /// let key2 = KvKey::new("key2").unwrap();
    /// let key3 = KvKey::new("key3").unwrap();
    ///
    /// engine.set(&key1, KvValue::String("value1".to_string()), None);
    /// engine.set(&key2, KvValue::String("value2".to_string()), None);
    ///
    /// let keys = vec![&key1, &key2, &key3];
    /// let values = engine.mget(&keys);
    /// assert_eq!(values.len(), 3);
    /// assert!(values[0].is_some());
    /// assert!(values[1].is_some());
    /// assert!(values[2].is_none()); // key3 doesn't exist
    /// ```
    pub fn mget(&self, keys: &[&KvKey]) -> Vec<Option<KvValue>> {
        keys.iter()
            .map(|key| self.get(key))
            .collect()
    }

    /// Set multiple key-value pairs (MSET)
    ///
    /// Sets multiple keys in a single operation. All keys will have the same TTL.
    ///
    /// # Performance
    /// This is more efficient than multiple SET calls for the same reasons as MGET.
    ///
    /// # Example
    /// ```
    /// use data_bridge_kv::engine::KvEngine;
    /// use data_bridge_kv::types::{KvKey, KvValue};
    ///
    /// let engine = KvEngine::new();
    /// let key1 = KvKey::new("key1").unwrap();
    /// let key2 = KvKey::new("key2").unwrap();
    ///
    /// let pairs = vec![
    ///     (&key1, KvValue::String("value1".to_string())),
    ///     (&key2, KvValue::Int(42)),
    /// ];
    ///
    /// engine.mset(&pairs, None);
    ///
    /// assert_eq!(engine.get(&key1), Some(KvValue::String("value1".to_string())));
    /// assert_eq!(engine.get(&key2), Some(KvValue::Int(42)));
    /// ```
    pub fn mset(&self, pairs: &[(&KvKey, KvValue)], ttl: Option<Duration>) {
        // Log single batch operation to WAL
        let wal_pairs: Vec<(String, KvValue)> = pairs
            .iter()
            .map(|(key, value)| (key.as_str().to_string(), value.clone()))
            .collect();

        self.log_wal(crate::persistence::format::WalOp::MSet {
            pairs: wal_pairs,
            ttl,
        });

        // Apply to in-memory shards
        for (key, value) in pairs {
            self.shard_for_key(key.as_str())
                .set(key.as_str().to_string(), value.clone(), ttl);
        }
    }

    /// Delete multiple keys (MDEL)
    ///
    /// Deletes multiple keys in a single operation.
    ///
    /// # Returns
    /// The number of keys that were actually deleted (existed before deletion).
    ///
    /// # Example
    /// ```
    /// use data_bridge_kv::engine::KvEngine;
    /// use data_bridge_kv::types::{KvKey, KvValue};
    ///
    /// let engine = KvEngine::new();
    /// let key1 = KvKey::new("key1").unwrap();
    /// let key2 = KvKey::new("key2").unwrap();
    /// let key3 = KvKey::new("key3").unwrap();
    ///
    /// engine.set(&key1, KvValue::Int(1), None);
    /// engine.set(&key2, KvValue::Int(2), None);
    ///
    /// let keys = vec![&key1, &key2, &key3];
    /// let deleted = engine.mdel(&keys);
    /// assert_eq!(deleted, 2); // key1 and key2 deleted, key3 didn't exist
    /// ```
    pub fn mdel(&self, keys: &[&KvKey]) -> usize {
        // Log single batch operation to WAL
        let wal_keys: Vec<String> = keys
            .iter()
            .map(|key| key.as_str().to_string())
            .collect();

        self.log_wal(crate::persistence::format::WalOp::MDel {
            keys: wal_keys,
        });

        // Apply to in-memory shards
        keys.iter()
            .filter(|key| {
                self.shard_for_key(key.as_str())
                    .delete(key.as_str())
                    .is_some()
            })
            .count()
    }

    /// Check if multiple keys exist (MEXISTS)
    ///
    /// Returns a vector of booleans indicating whether each key exists.
    ///
    /// # Example
    /// ```
    /// use data_bridge_kv::engine::KvEngine;
    /// use data_bridge_kv::types::{KvKey, KvValue};
    ///
    /// let engine = KvEngine::new();
    /// let key1 = KvKey::new("key1").unwrap();
    /// let key2 = KvKey::new("key2").unwrap();
    ///
    /// engine.set(&key1, KvValue::Int(1), None);
    ///
    /// let keys = vec![&key1, &key2];
    /// let exists = engine.mexists(&keys);
    /// assert_eq!(exists, vec![true, false]);
    /// ```
    pub fn mexists(&self, keys: &[&KvKey]) -> Vec<bool> {
        keys.iter()
            .map(|key| self.exists(key))
            .collect()
    }

    // ==================== Persistence Support ====================

    /// Export all entries from a specific shard (for persistence/snapshots)
    pub fn export_shard(&self, shard_id: usize) -> Option<HashMap<String, Entry>> {
        if shard_id >= self.num_shards {
            return None;
        }
        Some(self.shards[shard_id].export_all())
    }

    /// Import entries into a specific shard (for recovery from snapshots)
    pub fn import_shard(&self, shard_id: usize, entries: HashMap<String, Entry>) -> bool {
        if shard_id >= self.num_shards {
            return false;
        }
        self.shards[shard_id].import_all(entries);
        true
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

    #[test]
    fn test_setnx_success() {
        let engine = KvEngine::new();
        let key = KvKey::new("setnx_key").unwrap();

        // First SETNX should succeed
        assert!(engine.setnx(&key, KvValue::String("value1".to_string()), None));

        // Second SETNX should fail (key exists)
        assert!(!engine.setnx(&key, KvValue::String("value2".to_string()), None));

        // Value should still be the first one
        assert_eq!(engine.get(&key), Some(KvValue::String("value1".to_string())));
    }

    #[test]
    fn test_setnx_expired() {
        let engine = KvEngine::new();
        let key = KvKey::new("setnx_expired").unwrap();

        // Set with short TTL
        engine.setnx(&key, KvValue::String("old".to_string()), Some(Duration::from_millis(10)));

        // Wait for expiration
        thread::sleep(Duration::from_millis(20));

        // SETNX should now succeed (expired)
        assert!(engine.setnx(&key, KvValue::String("new".to_string()), None));
        assert_eq!(engine.get(&key), Some(KvValue::String("new".to_string())));
    }

    #[test]
    fn test_lock_unlock() {
        let engine = KvEngine::new();
        let key = KvKey::new("lock_test").unwrap();

        // Acquire lock
        assert!(engine.lock(&key, "worker-1", Duration::from_secs(30)));

        // Second acquire should fail
        assert!(!engine.lock(&key, "worker-2", Duration::from_secs(30)));

        // Unlock by wrong owner should fail
        let result = engine.unlock(&key, "worker-2");
        assert!(matches!(result, Err(KvError::LockOwnerMismatch { .. })));

        // Unlock by correct owner should succeed
        assert!(engine.unlock(&key, "worker-1").unwrap());

        // Now another worker can acquire
        assert!(engine.lock(&key, "worker-2", Duration::from_secs(30)));
    }

    #[test]
    fn test_lock_expiration() {
        let engine = KvEngine::new();
        let key = KvKey::new("lock_expire").unwrap();

        // Acquire with short TTL
        assert!(engine.lock(&key, "worker-1", Duration::from_millis(10)));

        // Wait for expiration
        thread::sleep(Duration::from_millis(20));

        // Another worker can now acquire (lock expired)
        assert!(engine.lock(&key, "worker-2", Duration::from_secs(30)));
    }

    #[test]
    fn test_extend_lock() {
        let engine = KvEngine::new();
        let key = KvKey::new("lock_extend").unwrap();

        // Acquire lock
        assert!(engine.lock(&key, "worker-1", Duration::from_millis(50)));

        // Extend by wrong owner should fail
        let result = engine.extend_lock(&key, "worker-2", Duration::from_secs(30));
        assert!(matches!(result, Err(KvError::LockOwnerMismatch { .. })));

        // Extend by correct owner should succeed
        assert!(engine.extend_lock(&key, "worker-1", Duration::from_secs(30)).unwrap());

        // Wait a bit - lock should still be held (was extended)
        thread::sleep(Duration::from_millis(60));
        assert!(engine.exists(&key));
    }
}
