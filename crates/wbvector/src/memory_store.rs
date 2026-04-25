use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::Layer;

/// URI prefix for vectors stored in the global in-process memory store.
pub const VECTOR_MEMORY_PREFIX: &str = "memory://vector/";

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static VECTOR_STORE: OnceLock<Mutex<HashMap<String, Arc<Layer>>>> = OnceLock::new();

fn store() -> &'static Mutex<HashMap<String, Arc<Layer>>> {
    VECTOR_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Returns `true` if `path` is an in-memory vector path.
pub fn vector_is_memory_path(path: &str) -> bool {
    path.starts_with(VECTOR_MEMORY_PREFIX)
}

/// Strips the memory prefix from `path`, returning the store key, or `None` if absent.
pub fn vector_path_to_id(path: &str) -> Option<&str> {
    path.strip_prefix(VECTOR_MEMORY_PREFIX)
}

/// Builds a `memory://vector/<id>` path from a store key.
pub fn make_vector_memory_path(id: &str) -> String {
    format!("{VECTOR_MEMORY_PREFIX}{id}")
}

/// Inserts `vector` into the global store and returns its new unique key.
pub fn put_vector(vector: Layer) -> String {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed).to_string();
    if let Ok(mut map) = store().lock() {
        map.insert(id.clone(), Arc::new(vector));
    }
    id
}

/// Inserts a shared vector handle into the global store and returns its new unique key.
pub fn put_vector_arc(vector: Arc<Layer>) -> String {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed).to_string();
    if let Ok(mut map) = store().lock() {
        map.insert(id.clone(), vector);
    }
    id
}

/// Retrieves a shared handle to the vector associated with `id`, or `None` if absent.
pub fn get_vector_arc_by_id(id: &str) -> Option<Arc<Layer>> {
    store().lock().ok().and_then(|map| map.get(id).cloned())
}

/// Retrieves a shared handle to the vector identified by a `memory://vector/<id>` path.
pub fn get_vector_arc_by_path(path: &str) -> Option<Arc<Layer>> {
    vector_path_to_id(path).and_then(get_vector_arc_by_id)
}

/// Retrieves a clone of the vector associated with `id`, or `None` if absent.
pub fn get_vector_by_id(id: &str) -> Option<Layer> {
    get_vector_arc_by_id(id).map(|v| (*v).clone())
}

/// Replaces the vector associated with `id`, returning `true` if an entry existed.
pub fn replace_vector_by_id(id: &str, vector: Layer) -> bool {
    store()
        .lock()
        .map(|mut map| map.insert(id.to_string(), Arc::new(vector)).is_some())
        .unwrap_or(false)
}

/// Replaces the vector identified by a `memory://vector/<id>` path, returning `true` if an entry existed.
pub fn replace_vector_by_path(path: &str, vector: Layer) -> bool {
    vector_path_to_id(path)
        .map(|id| replace_vector_by_id(id, vector))
        .unwrap_or(false)
}

/// Removes and returns the vector associated with `id`, or `None` if absent.
pub fn remove_vector_by_id(id: &str) -> Option<Layer> {
    store()
        .lock()
        .ok()
        .and_then(|mut map| map.remove(id))
        .map(|v| Arc::try_unwrap(v).unwrap_or_else(|shared| (*shared).clone()))
}

/// Removes and returns the vector identified by a `memory://vector/<id>` path.
pub fn remove_vector_by_path(path: &str) -> Option<Layer> {
    vector_path_to_id(path).and_then(remove_vector_by_id)
}

/// Removes all vectors from the global store and returns the number removed.
pub fn clear_vectors() -> usize {
    store()
        .lock()
        .map(|mut map| {
            let count = map.len();
            map.clear();
            count
        })
        .unwrap_or(0)
}

/// Returns the number of vectors currently held in the global store.
pub fn vector_count() -> usize {
    store().lock().map(|map| map.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn memory_store_test_guard() -> MutexGuard<'static, ()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("memory_store test lock poisoned")
    }

    #[test]
    fn remove_vector_by_id_removes_only_target_entry() {
        let _guard = memory_store_test_guard();
        clear_vectors();

        let id1 = put_vector(Layer::new("a"));
        let id2 = put_vector(Layer::new("b"));
        assert!(get_vector_by_id(&id1).is_some());
        assert!(get_vector_by_id(&id2).is_some());

        let removed = remove_vector_by_id(&id1).expect("vector should be removed by id");
        assert_eq!(removed.name, "a");
        assert!(get_vector_by_id(&id1).is_none());
        assert!(get_vector_by_id(&id2).is_some());

        clear_vectors();
    }

    #[test]
    fn remove_vector_by_path_and_clear_vectors_work() {
        let _guard = memory_store_test_guard();
        clear_vectors();

        let id1 = put_vector(Layer::new("x"));
        let id2 = put_vector(Layer::new("y"));
        let path1 = make_vector_memory_path(&id1);

        let removed = remove_vector_by_path(&path1).expect("vector should be removed by path");
        assert_eq!(removed.name, "x");
        assert!(get_vector_by_id(&id1).is_none());
        assert!(get_vector_by_id(&id2).is_some());
        assert!(vector_count() >= 1);

        let cleared = clear_vectors();
        assert!(cleared >= 1);
        assert!(get_vector_by_id(&id2).is_none());
    }
}
