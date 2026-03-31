use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::Raster;

/// URI prefix for rasters stored in the global in-process memory store.
pub const RASTER_MEMORY_PREFIX: &str = "memory://raster/";

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static RASTER_STORE: OnceLock<Mutex<HashMap<String, Raster>>> = OnceLock::new();

fn store() -> &'static Mutex<HashMap<String, Raster>> {
    RASTER_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Returns `true` if `path` is an in-memory raster path (i.e. starts with [`RASTER_MEMORY_PREFIX`]).
pub fn raster_is_memory_path(path: &str) -> bool {
    path.starts_with(RASTER_MEMORY_PREFIX)
}

/// Strips the memory prefix from `path`, returning the store key, or `None` if the prefix is absent.
pub fn raster_path_to_id(path: &str) -> Option<&str> {
    path.strip_prefix(RASTER_MEMORY_PREFIX)
}

/// Builds a `memory://raster/<id>` path from a store key.
pub fn make_raster_memory_path(id: &str) -> String {
    format!("{RASTER_MEMORY_PREFIX}{id}")
}

/// Inserts `raster` into the global store and returns its new unique key.
pub fn put_raster(raster: Raster) -> String {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed).to_string();
    if let Ok(mut map) = store().lock() {
        map.insert(id.clone(), raster);
    }
    id
}

/// Retrieves a clone of the raster associated with `id`, or `None` if absent.
pub fn get_raster_by_id(id: &str) -> Option<Raster> {
    store()
        .lock()
        .ok()
        .and_then(|map| map.get(id).cloned())
}

/// Removes and returns the raster associated with `id`, or `None` if absent.
pub fn remove_raster_by_id(id: &str) -> Option<Raster> {
    store()
        .lock()
        .ok()
        .and_then(|mut map| map.remove(id))
}

/// Removes and returns the raster identified by a `memory://raster/<id>` path.
pub fn remove_raster_by_path(path: &str) -> Option<Raster> {
    raster_path_to_id(path).and_then(remove_raster_by_id)
}

/// Removes all rasters from the global store and returns the number removed.
pub fn clear_rasters() -> usize {
    store()
        .lock()
        .map(|mut map| {
            let count = map.len();
            map.clear();
            count
        })
        .unwrap_or(0)
}

/// Returns the number of rasters currently held in the global store.
pub fn raster_count() -> usize {
    store().lock().map(|map| map.len()).unwrap_or(0)
}

/// Returns an estimate of the heap bytes held by all rasters in the global store.
///
/// The estimate covers only the typed cell-data buffer (`data` field);
/// it excludes metadata, projection strings, and allocator overhead.
pub fn raster_store_bytes() -> usize {
    store()
        .lock()
        .map(|map| {
            map.values()
                .map(|r| r.data.len() * r.data_type.size_bytes())
                .sum()
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DataType, RasterConfig};

    fn make_test_raster(value: f64) -> Raster {
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        Raster::from_data(cfg, vec![value, value + 1.0, value + 2.0, value + 3.0]).unwrap()
    }

    #[test]
    fn remove_raster_by_id_removes_only_target_entry() {
        clear_rasters();

        let id1 = put_raster(make_test_raster(1.0));
        let id2 = put_raster(make_test_raster(10.0));
        assert!(get_raster_by_id(&id1).is_some());
        assert!(get_raster_by_id(&id2).is_some());

        let removed = remove_raster_by_id(&id1).expect("raster should be removed by id");
        assert_eq!(removed.get(0, 0, 0), 1.0);
        assert!(get_raster_by_id(&id1).is_none());
        assert!(get_raster_by_id(&id2).is_some());

        clear_rasters();
    }

    #[test]
    fn remove_raster_by_path_and_clear_rasters_work() {
        clear_rasters();

        let id1 = put_raster(make_test_raster(5.0));
        let id2 = put_raster(make_test_raster(20.0));
        let path1 = make_raster_memory_path(&id1);

        let removed = remove_raster_by_path(&path1).expect("raster should be removed by path");
        assert_eq!(removed.get(0, 0, 0), 5.0);
        assert!(get_raster_by_id(&id1).is_none());
        assert!(get_raster_by_id(&id2).is_some());
        assert!(raster_count() >= 1);

        let cleared = clear_rasters();
        assert!(cleared >= 1);
        assert!(get_raster_by_id(&id2).is_none());
    }
}

