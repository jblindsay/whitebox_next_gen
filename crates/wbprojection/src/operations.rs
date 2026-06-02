//! Coordinate operation definitions and runtime registry.
//!
//! This module provides a lightweight, opt-in operation registry for explicit
//! operation-code-based transform routing.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use crate::error::{ProjectionError, Result};

/// Operation method category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationMethod {
    /// Generic datum-pipeline operation.
    DatumPipeline,
    /// Static grid-shift operation.
    GridShift,
    /// Dynamic/epoch-aware grid-shift operation.
    DynamicGridShift,
    /// Concatenated multi-step operation.
    Concatenated,
}

/// Coordinate operation definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinateOperationDef {
    /// Operation code (for example, an EPSG operation code).
    pub operation_code: u32,
    /// Source CRS code.
    pub source_crs_code: u32,
    /// Target CRS code.
    pub target_crs_code: u32,
    /// Operation method family.
    pub method: OperationMethod,
    /// True when this operation is preferred for this source/target pair.
    pub preferred: bool,
}

impl CoordinateOperationDef {
    /// Convenience constructor.
    pub const fn new(
        operation_code: u32,
        source_crs_code: u32,
        target_crs_code: u32,
        method: OperationMethod,
    ) -> Self {
        Self {
            operation_code,
            source_crs_code,
            target_crs_code,
            method,
            preferred: false,
        }
    }

    /// Mark this operation as preferred.
    pub const fn preferred(mut self, preferred: bool) -> Self {
        self.preferred = preferred;
        self
    }
}

static OPERATION_REGISTRY: OnceLock<RwLock<HashMap<u32, CoordinateOperationDef>>> = OnceLock::new();

fn registry() -> &'static RwLock<HashMap<u32, CoordinateOperationDef>> {
    OPERATION_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register or replace a coordinate operation definition.
pub fn register_coordinate_operation(op: CoordinateOperationDef) -> Result<()> {
    let mut m = registry().write().map_err(|_| {
        ProjectionError::DatumError("operation registry lock poisoned".to_string())
    })?;
    m.insert(op.operation_code, op);
    Ok(())
}

/// Remove a coordinate operation definition by code.
pub fn unregister_coordinate_operation(operation_code: u32) -> Result<bool> {
    let mut m = registry().write().map_err(|_| {
        ProjectionError::DatumError("operation registry lock poisoned".to_string())
    })?;
    Ok(m.remove(&operation_code).is_some())
}

/// Clear all runtime operation definitions.
pub fn clear_coordinate_operations() -> Result<()> {
    let mut m = registry().write().map_err(|_| {
        ProjectionError::DatumError("operation registry lock poisoned".to_string())
    })?;
    m.clear();
    Ok(())
}

/// Returns true when an operation code is registered.
pub fn has_coordinate_operation(operation_code: u32) -> Result<bool> {
    let m = registry().read().map_err(|_| {
        ProjectionError::DatumError("operation registry lock poisoned".to_string())
    })?;
    Ok(m.contains_key(&operation_code))
}

/// Fetch a registered operation definition by code.
pub fn get_coordinate_operation(operation_code: u32) -> Result<Option<CoordinateOperationDef>> {
    let m = registry().read().map_err(|_| {
        ProjectionError::DatumError("operation registry lock poisoned".to_string())
    })?;
    Ok(m.get(&operation_code).cloned())
}

#[cfg(test)]
/// Acquire a global guard for tests that mutate the coordinate operation registry.
///
/// This is used to serialize tests across modules because the registry is global
/// process state and test runners execute tests in parallel by default.
pub(crate) fn coordinate_operation_test_guard() -> std::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_operation_registry_round_trip() {
        clear_coordinate_operations().unwrap();

        let op = CoordinateOperationDef::new(10715, 22817, 2958, OperationMethod::DynamicGridShift)
            .preferred(true);
        register_coordinate_operation(op.clone()).unwrap();

        assert!(has_coordinate_operation(10715).unwrap());
        let loaded = get_coordinate_operation(10715).unwrap();
        assert_eq!(loaded, Some(op));

        assert!(unregister_coordinate_operation(10715).unwrap());
        assert!(!has_coordinate_operation(10715).unwrap());
    }
}
