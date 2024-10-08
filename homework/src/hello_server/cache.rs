//! Thread-safe key/value cache.

use std::collections::hash_map::{Entry, HashMap};
use std::hash::Hash;
use std::sync::{Arc, Mutex, RwLock};

/// Cache that remembers the result for each key.
#[derive(Debug)]
pub struct Cache<K, V> {
    // todo! This is an example cache type. Build your own cache type that satisfies the
    // specification for `get_or_insert_with`.
    inner: RwLock<HashMap<K, Arc<Mutex<Option<V>>>>>,
}

impl<K, V> Default for Cache<K, V> {
    fn default() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

/*
    1.	Concurrency: Different keys should be handled concurrently without blocking each other.
    2.	Single Computation: The computation function should only be run once per key,
        even if multiple threads try to access the same key concurrently.
*/
impl<K: Eq + Hash + Clone, V: Clone> Cache<K, V> {
    /// Retrieve the value or insert a new one created by `f`.
    ///
    /// An invocation to this function should not block another invocation with a different key. For
    /// example, if a thread calls `get_or_insert_with(key1, f1)` and another thread calls
    /// `get_or_insert_with(key2, f2)` (`key1≠key2`, `key1,key2∉cache`) concurrently, `f1` and `f2`
    /// should run concurrently.
    ///
    /// On the other hand, since `f` may consume a lot of resource (= money), it's undesirable to
    /// duplicate the work. That is, `f` should be run only once for each key. Specifically, even
    /// for concurrent invocations of `get_or_insert_with(key, f)`, `f` is called only once per key.
    ///
    /// Hint: the [`Entry`] API may be useful in implementing this function.
    ///
    /// [`Entry`]: https://doc.rust-lang.org/stable/std/collections/hash_map/struct.HashMap.html#method.entry
    pub fn get_or_insert_with<F: FnOnce(K) -> V>(&self, key: K, f: F) -> V {
        let value_arc_option = {
            let read_map = self.inner.read().unwrap();
            read_map.get(&key).cloned()
        };

        if let Some(cloned_value) = value_arc_option.and_then(|value_arc| {
            value_arc
                .lock()
                .ok()
                .and_then(|value_guard| value_guard.as_ref().cloned())
        }) {
            return cloned_value;
        }

        let value = {
            let mut write_map = self.inner.write().unwrap();
            write_map
                .entry(key.clone())
                .or_insert_with(|| Arc::new(Mutex::new(None)))
                .clone()
        };

        let mut value_guard = value.lock().unwrap();

        if value_guard.is_none() {
            *value_guard = Some(f(key));
        }

        value_guard.clone().unwrap()
    }
}
