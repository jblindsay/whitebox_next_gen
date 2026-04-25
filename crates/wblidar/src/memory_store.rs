use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::frontend::PointCloud;

/// URI prefix for LiDAR point clouds stored in the global in-process memory store.
pub const LIDAR_MEMORY_PREFIX: &str = "memory://lidar/";

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static LIDAR_STORE: OnceLock<Mutex<HashMap<String, Arc<PointCloud>>>> = OnceLock::new();

fn store() -> &'static Mutex<HashMap<String, Arc<PointCloud>>> {
    LIDAR_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Returns `true` if `path` is an in-memory LiDAR path.
pub fn lidar_is_memory_path(path: &str) -> bool {
    path.starts_with(LIDAR_MEMORY_PREFIX)
}

/// Strips the memory prefix from `path`, returning the store key, or `None` if absent.
pub fn lidar_path_to_id(path: &str) -> Option<&str> {
    path.strip_prefix(LIDAR_MEMORY_PREFIX)
}

/// Builds a `memory://lidar/<id>` path from a store key.
pub fn make_lidar_memory_path(id: &str) -> String {
    format!("{LIDAR_MEMORY_PREFIX}{id}")
}

/// Inserts `cloud` into the global store and returns its new unique key.
pub fn put_lidar(cloud: PointCloud) -> String {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed).to_string();
    if let Ok(mut map) = store().lock() {
        map.insert(id.clone(), Arc::new(cloud));
    }
    id
}

/// Inserts a shared LiDAR handle into the global store and returns its new unique key.
pub fn put_lidar_arc(cloud: Arc<PointCloud>) -> String {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed).to_string();
    if let Ok(mut map) = store().lock() {
        map.insert(id.clone(), cloud);
    }
    id
}

/// Retrieves a shared handle to the LiDAR cloud associated with `id`, or `None` if absent.
pub fn get_lidar_arc_by_id(id: &str) -> Option<Arc<PointCloud>> {
    store().lock().ok().and_then(|map| map.get(id).cloned())
}

/// Retrieves a shared handle to the LiDAR cloud identified by a `memory://lidar/<id>` path.
pub fn get_lidar_arc_by_path(path: &str) -> Option<Arc<PointCloud>> {
    lidar_path_to_id(path).and_then(get_lidar_arc_by_id)
}

/// Retrieves a clone of the LiDAR cloud associated with `id`, or `None` if absent.
pub fn get_lidar_by_id(id: &str) -> Option<PointCloud> {
    get_lidar_arc_by_id(id).map(|v| (*v).clone())
}

/// Replaces the LiDAR cloud associated with `id`, returning `true` if an entry existed.
pub fn replace_lidar_by_id(id: &str, cloud: PointCloud) -> bool {
    store()
        .lock()
        .map(|mut map| map.insert(id.to_string(), Arc::new(cloud)).is_some())
        .unwrap_or(false)
}

/// Replaces the LiDAR cloud identified by a `memory://lidar/<id>` path, returning `true` if an entry existed.
pub fn replace_lidar_by_path(path: &str, cloud: PointCloud) -> bool {
    lidar_path_to_id(path)
        .map(|id| replace_lidar_by_id(id, cloud))
        .unwrap_or(false)
}

/// Removes and returns the LiDAR cloud associated with `id`, or `None` if absent.
pub fn remove_lidar_by_id(id: &str) -> Option<PointCloud> {
    store()
        .lock()
        .ok()
        .and_then(|mut map| map.remove(id))
        .map(|v| Arc::try_unwrap(v).unwrap_or_else(|shared| (*shared).clone()))
}

/// Removes and returns the LiDAR cloud identified by a `memory://lidar/<id>` path.
pub fn remove_lidar_by_path(path: &str) -> Option<PointCloud> {
    lidar_path_to_id(path).and_then(remove_lidar_by_id)
}

/// Removes all LiDAR clouds from the global store and returns the number removed.
pub fn clear_lidars() -> usize {
    store()
        .lock()
        .map(|mut map| {
            let count = map.len();
            map.clear();
            count
        })
        .unwrap_or(0)
}

/// Returns the number of LiDAR clouds currently held in the global store.
pub fn lidar_count() -> usize {
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
    fn remove_lidar_by_id_removes_only_target_entry() {
        let _guard = memory_store_test_guard();
        clear_lidars();

        let id1 = put_lidar(PointCloud::default());
        let id2 = put_lidar(PointCloud::default());
        assert!(get_lidar_by_id(&id1).is_some());
        assert!(get_lidar_by_id(&id2).is_some());

        let removed = remove_lidar_by_id(&id1).expect("lidar should be removed by id");
        assert!(removed.points.is_empty());
        assert!(get_lidar_by_id(&id1).is_none());
        assert!(get_lidar_by_id(&id2).is_some());

        clear_lidars();
    }

    #[test]
    fn remove_lidar_by_path_and_clear_lidars_work() {
        let _guard = memory_store_test_guard();
        clear_lidars();

        let id1 = put_lidar(PointCloud::default());
        let id2 = put_lidar(PointCloud::default());
        let path1 = make_lidar_memory_path(&id1);

        let removed = remove_lidar_by_path(&path1).expect("lidar should be removed by path");
        assert!(removed.points.is_empty());
        assert!(get_lidar_by_id(&id1).is_none());
        assert!(get_lidar_by_id(&id2).is_some());
        assert!(lidar_count() >= 1);

        let cleared = clear_lidars();
        assert!(cleared >= 1);
        assert!(get_lidar_by_id(&id2).is_none());
    }
}
