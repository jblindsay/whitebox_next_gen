use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Mutex;
use wbcore::{
    generate_wrapper_stub, BindingTarget, ExecuteRequest, LicenseTier, OwnedToolRuntime,
    OwnedToolRuntimeWithCapabilities, RuntimeOptions,
    ProgressSink, ToolArgs, ToolError, ToolManifest, ToolRuntimeBuilder, ToolRuntimeRegistry,
};
use wblicense_core::{
    verify_signed_entitlement_json, EntitlementCapabilities, LicenseError, VerificationKeyStore,
};
use wbtools_oss::{register_default_tools as register_default_oss_tools, ToolRegistry as OssRegistry};
#[cfg(feature = "pro")]
use wbtools_pro::{register_default_tools as register_default_pro_tools, ToolRegistry as ProRegistry};

mod wb_environment;
pub use wb_environment::{
    Bundle,
    Lidar,
    LidarMetadata,
    Raster,
    RasterConfigs,
    Vector,
    VectorMetadata,
    WbEnvironment,
};
pub use wb_environment::{WbCategoryToolCallable, WbDomainNamespace, WbToolCategory, WbToolSubcategory};

struct CompositeRegistry {
    oss: OssRegistry,
    #[cfg(feature = "pro")]
    pro: Option<ProRegistry>,
}

impl ToolRuntimeRegistry for CompositeRegistry {
    fn list_tools(&self) -> Vec<wbcore::ToolMetadata> {
        #[cfg(feature = "pro")]
        let mut out = self.oss.list();
        #[cfg(not(feature = "pro"))]
        let out = self.oss.list();
        #[cfg(feature = "pro")]
        if let Some(pro) = &self.pro {
            out.extend(pro.list());
        }
        out
    }

    fn list_manifests(&self) -> Vec<ToolManifest> {
        #[cfg(feature = "pro")]
        let mut out = self.oss.manifests();
        #[cfg(not(feature = "pro"))]
        let out = self.oss.manifests();
        #[cfg(feature = "pro")]
        if let Some(pro) = &self.pro {
            out.extend(pro.manifests());
        }
        out
    }

    fn run_tool(&self, id: &str, args: &ToolArgs, ctx: &wbcore::ToolContext) -> Result<wbcore::ToolRunResult, ToolError> {
        match self.oss.run(id, args, ctx) {
            Ok(v) => Ok(v),
            Err(ToolError::NotFound(_)) => {
                #[cfg(feature = "pro")]
                if let Some(pro) = &self.pro {
                    return pro.run(id, args, ctx);
                }
                Err(ToolError::NotFound(id.to_string()))
            }
            Err(e) => Err(e),
        }
    }
}

fn validate_include_pro(include_pro: bool) -> Result<(), ToolError> {
    #[cfg(feature = "pro")]
    let _ = include_pro;

    #[cfg(not(feature = "pro"))]
    if include_pro {
        return Err(ToolError::InvalidRequest(
            "include_pro=true requested but this build does not include Pro support; rebuild with feature 'pro'".to_string(),
        ));
    }
    Ok(())
}


pub struct PythonToolRuntime {
    runtime: RuntimeMode,
    include_pro: bool,
    requested_tier: LicenseTier,
}

enum RuntimeMode {
    Tier(OwnedToolRuntime<CompositeRegistry>),
    Entitled(OwnedToolRuntimeWithCapabilities<CompositeRegistry, EntitlementCapabilities>),
}

impl Default for PythonToolRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonToolRuntime {
    pub fn new() -> Self {
        Self::new_with_options(false, LicenseTier::Open)
            .expect("default runtime construction should not fail")
    }

    #[cfg(feature = "pro")]
    pub fn new_with_options(include_pro: bool, max_tier: LicenseTier) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        let pro = if include_pro {
            let mut pro = ProRegistry::new();
            register_default_pro_tools(&mut pro);
            Some(pro)
        } else {
            None
        };

        Ok(Self {
            runtime: RuntimeMode::Tier(
                ToolRuntimeBuilder::new(CompositeRegistry { oss, pro })
                    .max_tier(max_tier)
                    .build(),
            ),
            include_pro,
            requested_tier: max_tier,
        })
    }

    #[cfg(feature = "pro")]
    pub fn new_with_floating_license_id(
        include_pro: bool,
        fallback_tier: LicenseTier,
        floating_license_id: &str,
        provider_url: Option<&str>,
        machine_id: Option<&str>,
        customer_id: Option<&str>,
    ) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;

        let _ = (floating_license_id, provider_url, machine_id, customer_id);

        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        let pro = if include_pro {
            let mut pro = ProRegistry::new();
            register_default_pro_tools(&mut pro);
            Some(pro)
        } else {
            None
        };

        Ok(Self {
            runtime: RuntimeMode::Tier(
                ToolRuntimeBuilder::new(CompositeRegistry { oss, pro })
                    .max_tier(fallback_tier)
                    .build(),
            ),
            include_pro,
            requested_tier: fallback_tier,
        })
    }

    #[cfg(not(feature = "pro"))]
    pub fn new_with_options(include_pro: bool, max_tier: LicenseTier) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        Ok(Self {
            runtime: RuntimeMode::Tier(
                ToolRuntimeBuilder::new(CompositeRegistry { oss })
                    .max_tier(max_tier)
                    .build(),
            ),
            include_pro,
            requested_tier: max_tier,
        })
    }

    #[cfg(feature = "pro")]
    pub fn new_with_entitlement_json(
        include_pro: bool,
        fallback_tier: LicenseTier,
        signed_entitlement_json: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
    ) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        let pro = if include_pro {
            let mut pro = ProRegistry::new();
            register_default_pro_tools(&mut pro);
            Some(pro)
        } else {
            None
        };

        let capabilities = entitlement_capabilities_from_json(
            signed_entitlement_json,
            public_key_kid,
            public_key_b64url,
        )?;

        Ok(Self {
            runtime: RuntimeMode::Entitled(OwnedToolRuntimeWithCapabilities::new(
                CompositeRegistry { oss, pro },
                RuntimeOptions {
                    max_tier: fallback_tier,
                    expose_locked_tools: false,
                },
                capabilities,
            )),
            include_pro,
            requested_tier: fallback_tier,
        })
    }

    #[cfg(not(feature = "pro"))]
    pub fn new_with_entitlement_json(
        include_pro: bool,
        fallback_tier: LicenseTier,
        signed_entitlement_json: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
    ) -> Result<Self, ToolError> {
        validate_include_pro(include_pro)?;
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);

        let capabilities = entitlement_capabilities_from_json(
            signed_entitlement_json,
            public_key_kid,
            public_key_b64url,
        )?;

        Ok(Self {
            runtime: RuntimeMode::Entitled(OwnedToolRuntimeWithCapabilities::new(
                CompositeRegistry { oss },
                RuntimeOptions {
                    max_tier: fallback_tier,
                    expose_locked_tools: false,
                },
                capabilities,
            )),
            include_pro,
            requested_tier: fallback_tier,
        })
    }

    pub fn visible_manifests(&self) -> Vec<ToolManifest> {
        match &self.runtime {
            RuntimeMode::Tier(runtime) => runtime.list_visible_manifests(),
            RuntimeMode::Entitled(runtime) => runtime.list_visible_manifests(),
        }
    }

    /// Returns every manifest in the build catalog (OSS + Pro), regardless of
    /// the current runtime's tier or include_pro flag.  Used for `include_locked=True`
    /// discovery queries.
    pub fn build_catalog_manifests(&self) -> Vec<ToolManifest> {
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);
        #[cfg(feature = "pro")]
        let mut manifests = oss.manifests();
        #[cfg(not(feature = "pro"))]
        let manifests = oss.manifests();
        #[cfg(feature = "pro")]
        {
            let mut pro = ProRegistry::new();
            register_default_pro_tools(&mut pro);
            manifests.extend(pro.manifests());
        }
        manifests
    }

    fn tool_manifest_by_id_from_build_catalog(&self, tool_id: &str) -> Option<ToolManifest> {
        let mut oss = OssRegistry::new();
        register_default_oss_tools(&mut oss);
        #[cfg(feature = "pro")]
        let mut manifests = oss.manifests();
        #[cfg(not(feature = "pro"))]
        let manifests = oss.manifests();

        #[cfg(feature = "pro")]
        {
            let mut pro = ProRegistry::new();
            register_default_pro_tools(&mut pro);
            manifests.extend(pro.manifests());
        }

        manifests.into_iter().find(|m| m.id == tool_id)
    }

    fn handle_not_found_with_license_context(&self, tool_id: &str) -> ToolError {
        let Some(manifest) = self.tool_manifest_by_id_from_build_catalog(tool_id) else {
            return ToolError::NotFound(tool_id.to_string());
        };

        let required = manifest.license_tier;
        let effective = self.effective_tier();
        if !self.include_pro && matches!(required, LicenseTier::Pro | LicenseTier::Enterprise) {
            return ToolError::LicenseDenied(format!(
                "This is a PRO tool: {tool_id}. Current runtime: include_pro={}, tier={}, effective_tier={}. Reason: pro_not_included. Action: enable include_pro=True and use a valid Pro/Enterprise entitlement.",
                self.include_pro,
                license_tier_to_str(self.requested_tier),
                license_tier_to_str(effective),
            ));
        }

        if required > effective {
            return ToolError::LicenseDenied(format!(
                "This is a PRO tool: {tool_id}. Current runtime: include_pro={}, tier={}, effective_tier={}. Reason: tier_insufficient (requires {}). Action: use tier='{}' or higher with a valid entitlement.",
                self.include_pro,
                license_tier_to_str(self.requested_tier),
                license_tier_to_str(effective),
                license_tier_to_str(required),
                license_tier_to_str(required),
            ));
        }

        ToolError::NotFound(tool_id.to_string())
    }

    pub fn list_tools_json(&self) -> Value {
        let tools: Vec<Value> = self.visible_manifests().into_iter().map(|m| json!(m)).collect();
        Value::Array(tools)
    }

    pub fn run_tool_json(&self, tool_id: &str, args_json: &str) -> Result<Value, ToolError> {
        let args = parse_args_json(args_json)?;

        let response = match &self.runtime {
            RuntimeMode::Tier(runtime) => runtime.execute(ExecuteRequest {
                tool_id: tool_id.to_string(),
                args,
            }),
            RuntimeMode::Entitled(runtime) => runtime.execute(ExecuteRequest {
                tool_id: tool_id.to_string(),
                args,
            }),
        }
        .map_err(|e| match e {
            ToolError::NotFound(_) => self.handle_not_found_with_license_context(tool_id),
            other => other,
        })?;
        Ok(Value::Object(response.outputs.into_iter().collect()))
    }

    pub fn run_tool_json_with_progress(&self, tool_id: &str, args_json: &str) -> Result<Value, ToolError> {
        let args = parse_args_json(args_json)?;

        let response = match &self.runtime {
            RuntimeMode::Tier(runtime) => runtime.execute(ExecuteRequest {
                tool_id: tool_id.to_string(),
                args,
            }),
            RuntimeMode::Entitled(runtime) => runtime.execute(ExecuteRequest {
                tool_id: tool_id.to_string(),
                args,
            }),
        }
        .map_err(|e| match e {
            ToolError::NotFound(_) => self.handle_not_found_with_license_context(tool_id),
            other => other,
        })?;

        Ok(json!({
            "tool_id": response.tool_id,
            "outputs": response.outputs,
            "progress": response.progress,
        }))
    }

    pub fn run_tool_json_with_progress_sink(
        &self,
        tool_id: &str,
        args_json: &str,
        progress: &dyn ProgressSink,
    ) -> Result<Value, ToolError> {
        let args = parse_args_json(args_json)?;

        let response = match &self.runtime {
            RuntimeMode::Tier(runtime) => runtime.execute_with_progress_sink(
                ExecuteRequest {
                    tool_id: tool_id.to_string(),
                    args,
                },
                progress,
            ),
            RuntimeMode::Entitled(runtime) => runtime.execute_with_progress_sink(
                ExecuteRequest {
                    tool_id: tool_id.to_string(),
                    args,
                },
                progress,
            ),
        }
        .map_err(|e| match e {
            ToolError::NotFound(_) => self.handle_not_found_with_license_context(tool_id),
            other => other,
        })?;

        Ok(json!({
            "tool_id": response.tool_id,
            "outputs": response.outputs,
            "progress": response.progress,
        }))
    }

    pub fn effective_tier(&self) -> LicenseTier {
        match &self.runtime {
            RuntimeMode::Tier(runtime) => runtime.options.max_tier,
            RuntimeMode::Entitled(runtime) => runtime.runtime().capabilities.max_tier,
        }
    }
}

fn license_tier_to_str(tier: LicenseTier) -> &'static str {
    match tier {
        LicenseTier::Open => "open",
        LicenseTier::Pro => "pro",
        LicenseTier::Enterprise => "enterprise",
    }
}

fn entitlement_capabilities_from_json(
    signed_entitlement_json: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
) -> Result<EntitlementCapabilities, ToolError> {
    let mut key_store = VerificationKeyStore::new();
    key_store
        .insert_base64url_public_key(public_key_kid, public_key_b64url)
        .map_err(map_license_error)?;
    let verified = verify_signed_entitlement_json(signed_entitlement_json, &key_store, current_unix())
        .map_err(map_license_error)?;
    Ok(EntitlementCapabilities::from_verified(&verified, current_unix()))
}

fn current_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn map_license_error(err: LicenseError) -> ToolError {
    ToolError::LicenseDenied(err.to_string())
}

fn read_entitlement_file(path: &str) -> Result<String, ToolError> {
    std::fs::read_to_string(path)
        .map_err(|e| ToolError::InvalidRequest(format!("failed to read entitlement file '{path}': {e}")))
}

#[derive(Default)]
pub(crate) struct PyCallbackSink {
    callback: Mutex<Option<Py<PyAny>>>,
    callback_error: Mutex<Option<String>>,
}

impl PyCallbackSink {
    pub(crate) fn new(callback: Py<PyAny>) -> Self {
        Self {
            callback: Mutex::new(Some(callback)),
            callback_error: Mutex::new(None),
        }
    }

    fn emit_event(&self, event: Value) {
        let payload = match serde_json::to_string(&event) {
            Ok(v) => v,
            Err(e) => {
                let _ = self.set_error(format!("event serialization error: {e}"));
                return;
            }
        };

        let attached = Python::try_attach(|py| {
            let guard = match self.callback.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    let _ = self.set_error("callback mutex poisoned".to_string());
                    return;
                }
            };

            if let Some(callback) = guard.as_ref() {
                if let Err(e) = callback.call1(py, (payload.as_str(),)) {
                    let _ = self.set_error(format!("callback error: {e}"));
                }
            }
        });

        if attached.is_none() {
            let _ = self.set_error("python interpreter not attached".to_string());
        }
    }

    fn set_error(&self, msg: String) -> Result<(), ()> {
        let mut guard = self.callback_error.lock().map_err(|_| ())?;
        if guard.is_none() {
            *guard = Some(msg);
        }
        Err(())
    }

    pub(crate) fn take_error(&self) -> Option<String> {
        match self.callback_error.lock() {
            Ok(mut guard) => guard.take(),
            Err(_) => Some("callback error state poisoned".to_string()),
        }
    }
}

impl ProgressSink for PyCallbackSink {
    fn info(&self, msg: &str) {
        self.emit_event(json!({ "type": "message", "message": msg }));
    }

    fn progress(&self, pct: f64) {
        self.emit_event(json!({ "type": "progress", "percent": pct.clamp(0.0, 1.0) }));
    }
}

fn parse_args_json(args_json: &str) -> Result<ToolArgs, ToolError> {
    let value: Value = serde_json::from_str(args_json)
        .map_err(|e| ToolError::Validation(format!("invalid JSON arguments: {e}")))?;

    let map = value
        .as_object()
        .ok_or_else(|| ToolError::Validation("arguments must be a JSON object".to_string()))?;

    let mut args = ToolArgs::new();
    for (k, v) in map {
        args.insert(k.clone(), v.clone());
    }
    Ok(args)
}

fn py_any_to_json_value(value: &Bound<'_, PyAny>) -> PyResult<Value> {
    if value.is_none() {
        return Ok(Value::Null);
    }

    if let Ok(v) = value.extract::<bool>() {
        return Ok(Value::Bool(v));
    }

    if let Ok(v) = value.extract::<i64>() {
        return Ok(Value::Number(v.into()));
    }

    if let Ok(v) = value.extract::<u64>() {
        return Ok(Value::Number(v.into()));
    }

    if let Ok(v) = value.extract::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(v) {
            return Ok(Value::Number(n));
        }
        return Err(PyValueError::new_err(
            "cannot serialize non-finite float (NaN or infinity) to JSON",
        ));
    }

    if let Ok(v) = value.extract::<String>() {
        return Ok(Value::String(v));
    }

    if let Ok(r) = value.extract::<pyo3::PyRef<'_, Raster>>() {
        return Ok(json!({
            "__wbw_type__": "raster",
            "path": r.file_path.to_string_lossy().to_string(),
            "active_band": r.active_band,
        }));
    }

    if let Ok(v) = value.extract::<pyo3::PyRef<'_, Vector>>() {
        return Ok(json!({
            "__wbw_type__": "vector",
            "path": v.file_path.to_string_lossy().to_string(),
        }));
    }

    if let Ok(v) = value.extract::<pyo3::PyRef<'_, Lidar>>() {
        return Ok(json!({
            "__wbw_type__": "lidar",
            "path": v.file_path.to_string_lossy().to_string(),
        }));
    }

    if let Ok(list) = value.cast::<PyList>() {
        let mut arr = Vec::with_capacity(list.len());
        for item in list.iter() {
            arr.push(py_any_to_json_value(&item)?);
        }
        return Ok(Value::Array(arr));
    }

    if let Ok(tuple) = value.cast::<PyTuple>() {
        let mut arr = Vec::with_capacity(tuple.len());
        for item in tuple.iter() {
            arr.push(py_any_to_json_value(&item)?);
        }
        return Ok(Value::Array(arr));
    }

    if let Ok(dict) = value.cast::<PyDict>() {
        let mut out = serde_json::Map::new();
        for (k, v) in dict.iter() {
            let key = k.extract::<String>().map_err(|_| {
                PyValueError::new_err("all argument dictionary keys must be strings")
            })?;
            out.insert(key, py_any_to_json_value(&v)?);
        }
        return Ok(Value::Object(out));
    }

    Err(PyValueError::new_err(
        "unsupported argument type; use JSON-compatible values or Raster/Vector/Lidar objects",
    ))
}

fn parse_args_py_any(args: &Bound<'_, PyAny>) -> PyResult<ToolArgs> {
    if let Ok(args_json) = args.extract::<String>() {
        return parse_args_json(&args_json).map_err(map_tool_error);
    }

    let dict = args.cast::<PyDict>().map_err(|_| {
        PyValueError::new_err("arguments must be a JSON string or a Python dict")
    })?;

    let mut out = ToolArgs::new();
    for (k, v) in dict.iter() {
        let key = k
            .extract::<String>()
            .map_err(|_| PyValueError::new_err("all argument dictionary keys must be strings"))?;
        out.insert(key, py_any_to_json_value(&v)?);
    }
    Ok(out)
}

fn parse_tier(tier: &str) -> Result<LicenseTier, ToolError> {
    match tier.to_ascii_lowercase().as_str() {
        "open" => Ok(LicenseTier::Open),
        "pro" => Ok(LicenseTier::Pro),
        "enterprise" => Ok(LicenseTier::Enterprise),
        _ => Err(ToolError::InvalidRequest(format!(
            "invalid tier '{tier}', expected open|pro|enterprise"
        ))),
    }
}

fn map_tool_error(err: ToolError) -> PyErr {
    match err {
        ToolError::Validation(msg) => PyValueError::new_err(msg),
        ToolError::NotFound(msg) => PyValueError::new_err(msg),
        ToolError::InvalidRequest(msg) => PyValueError::new_err(msg),
        ToolError::LicenseDenied(msg) => PyRuntimeError::new_err(msg),
        ToolError::Execution(msg) => PyRuntimeError::new_err(msg),
    }
}

fn extract_tool_ids(value: &Value) -> PyResult<Vec<String>> {
    let arr = value.as_array().ok_or_else(|| {
        PyRuntimeError::new_err("tools payload was not a list")
    })?;

    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        if let Some(id) = item.get("id").and_then(Value::as_str) {
            out.push(id.to_string());
        }
    }
    Ok(out)
}

fn json_scalar_to_py(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    let payload = serde_json::to_string(value)
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))?;
    let json_mod = py.import("json")?;
    let obj = json_mod.call_method1("loads", (payload,))?;
    Ok(obj.unbind())
}

fn decode_typed_object(py: Python<'_>, map: &serde_json::Map<String, Value>) -> PyResult<Option<Py<PyAny>>> {
    let Some(kind) = map.get("__wbw_type__").and_then(Value::as_str) else {
        return Ok(None);
    };

    match kind {
        "raster" => {
            let path = map
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| PyValueError::new_err("typed output 'raster' requires string field 'path'"))?;
            let active_band = map
                .get("active_band")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            let raster = Py::new(
                py,
                Raster {
                    file_path: PathBuf::from(path),
                    active_band,
                },
            )?;
            Ok(Some(raster.into_any()))
        }
        "vector" => {
            let path = map
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| PyValueError::new_err("typed output 'vector' requires string field 'path'"))?;
            let vector = Py::new(
                py,
                Vector {
                    file_path: PathBuf::from(path),
                },
            )?;
            Ok(Some(vector.into_any()))
        }
        "lidar" => {
            let path = map
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| PyValueError::new_err("typed output 'lidar' requires string field 'path'"))?;
            let lidar = Py::new(
                py,
                Lidar {
                    file_path: PathBuf::from(path),
                },
            )?;
            Ok(Some(lidar.into_any()))
        }
        "tuple" => {
            let items = map
                .get("items")
                .and_then(Value::as_array)
                .ok_or_else(|| PyValueError::new_err("typed output 'tuple' requires array field 'items'"))?;

            let py_items: PyResult<Vec<Py<PyAny>>> = items
                .iter()
                .map(|item| json_value_to_python_object(py, item))
                .collect();
            let py_items = py_items?;
            let tuple = PyTuple::new(py, py_items.iter().map(|v| v.bind(py)))?;
            Ok(Some(tuple.into_any().unbind()))
        }
        _ => Err(PyValueError::new_err(format!(
            "unsupported typed output kind '{}'; expected raster|vector|lidar|tuple",
            kind
        ))),
    }
}

fn json_value_to_python_object(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            json_scalar_to_py(py, value)
        }
        Value::Array(values) => {
            let list = PyList::empty(py);
            for v in values {
                let item = json_value_to_python_object(py, v)?;
                list.append(item.bind(py))?;
            }
            Ok(list.into_any().unbind())
        }
        Value::Object(map) => {
            if let Some(typed) = decode_typed_object(py, map)? {
                return Ok(typed);
            }

            let dict = PyDict::new(py);
            for (k, v) in map {
                let item = json_value_to_python_object(py, v)?;
                dict.set_item(k, item.bind(py))?;
            }
            Ok(dict.into_any().unbind())
        }
    }
}

#[pyclass(unsendable)]
struct RuntimeSession {
    runtime: PythonToolRuntime,
}

#[pymethods]
impl RuntimeSession {
    #[new]
    #[pyo3(signature = (include_pro=false, tier="open"))]
    fn new(include_pro: bool, tier: &str) -> PyResult<Self> {
        let parsed_tier = parse_tier(tier).map_err(map_tool_error)?;
        Ok(Self {
            runtime: PythonToolRuntime::new_with_options(include_pro, parsed_tier)
                .map_err(map_tool_error)?,
        })
    }

    #[staticmethod]
    #[pyo3(signature = (signed_entitlement_json, public_key_kid, public_key_b64url, include_pro=false, fallback_tier="open"))]
    fn from_signed_entitlement_json(
        signed_entitlement_json: &str,
        public_key_kid: &str,
        public_key_b64url: &str,
        include_pro: bool,
        fallback_tier: &str,
    ) -> PyResult<Self> {
        let parsed_tier = parse_tier(fallback_tier).map_err(map_tool_error)?;
        Ok(Self {
            runtime: PythonToolRuntime::new_with_entitlement_json(
                include_pro,
                parsed_tier,
                signed_entitlement_json,
                public_key_kid,
                public_key_b64url,
            )
            .map_err(map_tool_error)?,
        })
    }

    #[staticmethod]
    #[cfg(feature = "pro")]
    #[pyo3(signature = (floating_license_id, include_pro=true, fallback_tier="open", provider_url=None, machine_id=None, customer_id=None))]
    fn from_floating_license_id(
        floating_license_id: &str,
        include_pro: bool,
        fallback_tier: &str,
        provider_url: Option<&str>,
        machine_id: Option<&str>,
        customer_id: Option<&str>,
    ) -> PyResult<Self> {
        let parsed_tier = parse_tier(fallback_tier).map_err(map_tool_error)?;
        Ok(Self {
            runtime: PythonToolRuntime::new_with_floating_license_id(
                include_pro,
                parsed_tier,
                floating_license_id,
                provider_url,
                machine_id,
                customer_id,
            )
            .map_err(map_tool_error)?,
        })
    }

    #[staticmethod]
    #[cfg(not(feature = "pro"))]
    #[pyo3(signature = (floating_license_id, include_pro=true, fallback_tier="open", provider_url=None, machine_id=None, customer_id=None))]
    fn from_floating_license_id(
        floating_license_id: &str,
        include_pro: bool,
        fallback_tier: &str,
        provider_url: Option<&str>,
        machine_id: Option<&str>,
        customer_id: Option<&str>,
    ) -> PyResult<Self> {
        let _ = (floating_license_id, provider_url, machine_id, customer_id);
        let parsed_tier = parse_tier(fallback_tier).map_err(map_tool_error)?;
        Ok(Self {
            runtime: PythonToolRuntime::new_with_options(include_pro, parsed_tier)
                .map_err(map_tool_error)?,
        })
    }

    fn list_tools_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.runtime.list_tools_json())
            .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
    }

    fn list_tools(&self) -> PyResult<Vec<String>> {
        extract_tool_ids(&self.runtime.list_tools_json())
    }

    fn run_tool_json(&self, tool_id: &str, args_json: &str) -> PyResult<String> {
        let out = self
            .runtime
            .run_tool_json(tool_id, args_json)
            .map_err(map_tool_error)?;
        serde_json::to_string(&out)
            .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
    }

    fn run_tool(&self, py: Python<'_>, tool_id: &str, args: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let args_map = parse_args_py_any(args)?;
        let args_json = serde_json::to_string(&args_map)
            .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))?;
        let out = self
            .runtime
            .run_tool_json(tool_id, &args_json)
            .map_err(map_tool_error)?;
        json_value_to_python_object(py, &out)
    }

    fn run_tool_json_with_progress(&self, tool_id: &str, args_json: &str) -> PyResult<String> {
        let out = self
            .runtime
            .run_tool_json_with_progress(tool_id, args_json)
            .map_err(map_tool_error)?;
        serde_json::to_string(&out)
            .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
    }

    fn run_tool_json_stream(&self, tool_id: &str, args_json: &str, callback: Py<PyAny>) -> PyResult<String> {
        let sink = PyCallbackSink::new(callback);
        let out = self
            .runtime
            .run_tool_json_with_progress_sink(tool_id, args_json, &sink)
            .map_err(map_tool_error)?;
        if let Some(msg) = sink.take_error() {
            return Err(PyRuntimeError::new_err(msg));
        }
        serde_json::to_string(&out)
            .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
    }

    fn run_tool_stream(
        &self,
        py: Python<'_>,
        tool_id: &str,
        args: &Bound<'_, PyAny>,
        callback: Py<PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let args_map = parse_args_py_any(args)?;
        let args_json = serde_json::to_string(&args_map)
            .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))?;
        let sink = PyCallbackSink::new(callback);
        let out = self
            .runtime
            .run_tool_json_with_progress_sink(tool_id, &args_json, &sink)
            .map_err(map_tool_error)?;
        if let Some(msg) = sink.take_error() {
            return Err(PyRuntimeError::new_err(msg));
        }
        let outputs = out
            .get("outputs")
            .ok_or_else(|| PyRuntimeError::new_err("missing outputs in tool response"))?;
        json_value_to_python_object(py, outputs)
    }
}

#[pyfunction]
fn list_tools_json() -> PyResult<String> {
    let rt = PythonToolRuntime::new();
    serde_json::to_string(&rt.list_tools_json())
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
}

#[pyfunction]
fn list_tools() -> PyResult<Vec<String>> {
    let rt = PythonToolRuntime::new();
    extract_tool_ids(&rt.list_tools_json())
}

#[pyfunction]
#[pyo3(signature = (include_pro=false, tier="open"))]
fn list_tools_json_with_options(include_pro: bool, tier: &str) -> PyResult<String> {
    let parsed_tier = parse_tier(tier).map_err(map_tool_error)?;
    let rt = PythonToolRuntime::new_with_options(include_pro, parsed_tier)
        .map_err(map_tool_error)?;
    serde_json::to_string(&rt.list_tools_json())
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
}

#[pyfunction]
#[pyo3(signature = (include_pro=false, tier="open"))]
fn list_tools_with_options(include_pro: bool, tier: &str) -> PyResult<Vec<String>> {
    let parsed_tier = parse_tier(tier).map_err(map_tool_error)?;
    let rt = PythonToolRuntime::new_with_options(include_pro, parsed_tier)
        .map_err(map_tool_error)?;
    extract_tool_ids(&rt.list_tools_json())
}

#[pyfunction]
#[pyo3(signature = (signed_entitlement_json, public_key_kid, public_key_b64url, include_pro=false, fallback_tier="open"))]
fn list_tools_json_with_entitlement_options(
    signed_entitlement_json: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
    include_pro: bool,
    fallback_tier: &str,
) -> PyResult<String> {
    let parsed_tier = parse_tier(fallback_tier).map_err(map_tool_error)?;
    let rt = PythonToolRuntime::new_with_entitlement_json(
        include_pro,
        parsed_tier,
        signed_entitlement_json,
        public_key_kid,
        public_key_b64url,
    )
    .map_err(map_tool_error)?;
    serde_json::to_string(&rt.list_tools_json())
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
}

#[pyfunction]
#[pyo3(signature = (entitlement_file, public_key_kid, public_key_b64url, include_pro=false, fallback_tier="open"))]
fn list_tools_json_with_entitlement_file_options(
    entitlement_file: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
    include_pro: bool,
    fallback_tier: &str,
) -> PyResult<String> {
    let signed_entitlement_json = read_entitlement_file(entitlement_file).map_err(map_tool_error)?;
    list_tools_json_with_entitlement_options(
        &signed_entitlement_json,
        public_key_kid,
        public_key_b64url,
        include_pro,
        fallback_tier,
    )
}

#[pyfunction]
fn run_tool_json(tool_id: &str, args_json: &str) -> PyResult<String> {
    let rt = PythonToolRuntime::new();
    let out = rt.run_tool_json(tool_id, args_json).map_err(map_tool_error)?;
    serde_json::to_string(&out)
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
}

#[pyfunction]
fn run_tool(py: Python<'_>, tool_id: &str, args: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let args_map = parse_args_py_any(args)?;
    let args_json = serde_json::to_string(&args_map)
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))?;
    let rt = PythonToolRuntime::new();
    let out = rt.run_tool_json(tool_id, &args_json).map_err(map_tool_error)?;
    json_value_to_python_object(py, &out)
}

#[pyfunction]
fn run_tool_json_with_progress(tool_id: &str, args_json: &str) -> PyResult<String> {
    let rt = PythonToolRuntime::new();
    let out = rt
        .run_tool_json_with_progress(tool_id, args_json)
        .map_err(map_tool_error)?;
    serde_json::to_string(&out)
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
}

#[pyfunction]
fn run_tool_json_stream(tool_id: &str, args_json: &str, callback: Py<PyAny>) -> PyResult<String> {
    let rt = PythonToolRuntime::new();
    let sink = PyCallbackSink::new(callback);
    let out = rt
        .run_tool_json_with_progress_sink(tool_id, args_json, &sink)
        .map_err(map_tool_error)?;
    if let Some(msg) = sink.take_error() {
        return Err(PyRuntimeError::new_err(msg));
    }
    serde_json::to_string(&out)
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
}

#[pyfunction]
fn run_tool_stream(
    py: Python<'_>,
    tool_id: &str,
    args: &Bound<'_, PyAny>,
    callback: Py<PyAny>,
) -> PyResult<Py<PyAny>> {
    let args_map = parse_args_py_any(args)?;
    let args_json = serde_json::to_string(&args_map)
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))?;
    let rt = PythonToolRuntime::new();
    let sink = PyCallbackSink::new(callback);
    let out = rt
        .run_tool_json_with_progress_sink(tool_id, &args_json, &sink)
        .map_err(map_tool_error)?;
    if let Some(msg) = sink.take_error() {
        return Err(PyRuntimeError::new_err(msg));
    }
    let outputs = out
        .get("outputs")
        .ok_or_else(|| PyRuntimeError::new_err("missing outputs in tool response"))?;
    json_value_to_python_object(py, outputs)
}

#[pyfunction]
#[pyo3(signature = (tool_id, args_json, include_pro=false, tier="open"))]
fn run_tool_json_with_options(tool_id: &str, args_json: &str, include_pro: bool, tier: &str) -> PyResult<String> {
    let parsed_tier = parse_tier(tier).map_err(map_tool_error)?;
    let rt = PythonToolRuntime::new_with_options(include_pro, parsed_tier)
        .map_err(map_tool_error)?;
    let out = rt.run_tool_json(tool_id, args_json).map_err(map_tool_error)?;
    serde_json::to_string(&out)
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
}

#[pyfunction]
#[pyo3(signature = (tool_id, args_json, signed_entitlement_json, public_key_kid, public_key_b64url, include_pro=false, fallback_tier="open"))]
fn run_tool_json_with_entitlement_options(
    tool_id: &str,
    args_json: &str,
    signed_entitlement_json: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
    include_pro: bool,
    fallback_tier: &str,
) -> PyResult<String> {
    let parsed_tier = parse_tier(fallback_tier).map_err(map_tool_error)?;
    let rt = PythonToolRuntime::new_with_entitlement_json(
        include_pro,
        parsed_tier,
        signed_entitlement_json,
        public_key_kid,
        public_key_b64url,
    )
    .map_err(map_tool_error)?;
    let out = rt.run_tool_json(tool_id, args_json).map_err(map_tool_error)?;
    serde_json::to_string(&out)
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
}

#[pyfunction]
#[pyo3(signature = (tool_id, args_json, entitlement_file, public_key_kid, public_key_b64url, include_pro=false, fallback_tier="open"))]
fn run_tool_json_with_entitlement_file_options(
    tool_id: &str,
    args_json: &str,
    entitlement_file: &str,
    public_key_kid: &str,
    public_key_b64url: &str,
    include_pro: bool,
    fallback_tier: &str,
) -> PyResult<String> {
    let signed_entitlement_json = read_entitlement_file(entitlement_file).map_err(map_tool_error)?;
    run_tool_json_with_entitlement_options(
        tool_id,
        args_json,
        &signed_entitlement_json,
        public_key_kid,
        public_key_b64url,
        include_pro,
        fallback_tier,
    )
}

#[pyfunction]
#[pyo3(signature = (tool_id, args, include_pro=false, tier="open"))]
fn run_tool_with_options(
    py: Python<'_>,
    tool_id: &str,
    args: &Bound<'_, PyAny>,
    include_pro: bool,
    tier: &str,
) -> PyResult<Py<PyAny>> {
    let args_map = parse_args_py_any(args)?;
    let args_json = serde_json::to_string(&args_map)
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))?;
    let parsed_tier = parse_tier(tier).map_err(map_tool_error)?;
    let rt = PythonToolRuntime::new_with_options(include_pro, parsed_tier)
        .map_err(map_tool_error)?;
    let out = rt.run_tool_json(tool_id, &args_json).map_err(map_tool_error)?;
    json_value_to_python_object(py, &out)
}

#[pyfunction]
#[pyo3(signature = (tool_id, args_json, include_pro=false, tier="open"))]
fn run_tool_json_with_progress_options(
    tool_id: &str,
    args_json: &str,
    include_pro: bool,
    tier: &str,
) -> PyResult<String> {
    let parsed_tier = parse_tier(tier).map_err(map_tool_error)?;
    let rt = PythonToolRuntime::new_with_options(include_pro, parsed_tier)
        .map_err(map_tool_error)?;
    let out = rt
        .run_tool_json_with_progress(tool_id, args_json)
        .map_err(map_tool_error)?;
    serde_json::to_string(&out)
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
}

#[pyfunction]
#[pyo3(signature = (tool_id, args_json, callback, include_pro=false, tier="open"))]
fn run_tool_json_stream_options(
    tool_id: &str,
    args_json: &str,
    callback: Py<PyAny>,
    include_pro: bool,
    tier: &str,
) -> PyResult<String> {
    let parsed_tier = parse_tier(tier).map_err(map_tool_error)?;
    let rt = PythonToolRuntime::new_with_options(include_pro, parsed_tier)
        .map_err(map_tool_error)?;
    let sink = PyCallbackSink::new(callback);
    let out = rt
        .run_tool_json_with_progress_sink(tool_id, args_json, &sink)
        .map_err(map_tool_error)?;
    if let Some(msg) = sink.take_error() {
        return Err(PyRuntimeError::new_err(msg));
    }
    serde_json::to_string(&out)
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
}

#[pyfunction]
#[pyo3(signature = (tool_id, args, callback, include_pro=false, tier="open"))]
fn run_tool_stream_options(
    py: Python<'_>,
    tool_id: &str,
    args: &Bound<'_, PyAny>,
    callback: Py<PyAny>,
    include_pro: bool,
    tier: &str,
) -> PyResult<Py<PyAny>> {
    let args_map = parse_args_py_any(args)?;
    let args_json = serde_json::to_string(&args_map)
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))?;
    let parsed_tier = parse_tier(tier).map_err(map_tool_error)?;
    let rt = PythonToolRuntime::new_with_options(include_pro, parsed_tier)
        .map_err(map_tool_error)?;
    let sink = PyCallbackSink::new(callback);
    let out = rt
        .run_tool_json_with_progress_sink(tool_id, &args_json, &sink)
        .map_err(map_tool_error)?;
    if let Some(msg) = sink.take_error() {
        return Err(PyRuntimeError::new_err(msg));
    }
    let outputs = out
        .get("outputs")
        .ok_or_else(|| PyRuntimeError::new_err("missing outputs in tool response"))?;
    json_value_to_python_object(py, outputs)
}

/// Helper function to run any tool with convenient arguments.
/// Returns the result as a JSON string.
fn _run_tool_convenient(
    tool_id: &str,
    input: &Raster,
    output: Option<&str>,
    callback: Option<Py<PyAny>>,
    include_pro: bool,
    tier: &str,
) -> PyResult<String> {
    let parsed_tier = parse_tier(tier).map_err(map_tool_error)?;
    let rt = PythonToolRuntime::new_with_options(include_pro, parsed_tier)
        .map_err(map_tool_error)?;

    let out_path = wb_environment::resolve_unary_output_path(&input.file_path, tool_id, output, None);

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let args_json = serde_json::to_string(&json!({
        "input": input.file_path.to_string_lossy().to_string(),
        "output": out_path.to_string_lossy().to_string()
    }))
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))?;

    let out = if let Some(cb) = callback {
        let sink = PyCallbackSink::new(cb);
        let result = rt
            .run_tool_json_with_progress_sink(tool_id, &args_json, &sink)
            .map_err(map_tool_error)?;
        if let Some(msg) = sink.take_error() {
            return Err(PyRuntimeError::new_err(msg));
        }
        result
    } else {
        rt.run_tool_json_with_progress(tool_id, &args_json)
            .map_err(map_tool_error)?
    };

    serde_json::to_string(&out)
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
}

// Convenience wrapper functions for unary raster math tools
#[pyfunction]
#[pyo3(signature = (input, output=None, callback=None, include_pro=false, tier="open"))]
fn abs(input: &Raster, output: Option<&str>, callback: Option<Py<PyAny>>, include_pro: bool, tier: &str) -> PyResult<String> {
    _run_tool_convenient("abs", input, output, callback, include_pro, tier)
}

#[pyfunction]
#[pyo3(signature = (input, output=None, callback=None, include_pro=false, tier="open"))]
fn ceil(input: &Raster, output: Option<&str>, callback: Option<Py<PyAny>>, include_pro: bool, tier: &str) -> PyResult<String> {
    _run_tool_convenient("ceil", input, output, callback, include_pro, tier)
}

#[pyfunction]
#[pyo3(signature = (input, output=None, callback=None, include_pro=false, tier="open"))]
fn floor(input: &Raster, output: Option<&str>, callback: Option<Py<PyAny>>, include_pro: bool, tier: &str) -> PyResult<String> {
    _run_tool_convenient("floor", input, output, callback, include_pro, tier)
}

#[pyfunction]
#[pyo3(signature = (input, output=None, callback=None, include_pro=false, tier="open"))]
fn round(input: &Raster, output: Option<&str>, callback: Option<Py<PyAny>>, include_pro: bool, tier: &str) -> PyResult<String> {
    _run_tool_convenient("round", input, output, callback, include_pro, tier)
}

#[pyfunction]
#[pyo3(signature = (input, output=None, callback=None, include_pro=false, tier="open"))]
fn sqrt(input: &Raster, output: Option<&str>, callback: Option<Py<PyAny>>, include_pro: bool, tier: &str) -> PyResult<String> {
    _run_tool_convenient("sqrt", input, output, callback, include_pro, tier)
}

#[pyfunction]
#[pyo3(signature = (input, output=None, callback=None, include_pro=false, tier="open"))]
fn square(input: &Raster, output: Option<&str>, callback: Option<Py<PyAny>>, include_pro: bool, tier: &str) -> PyResult<String> {
    _run_tool_convenient("square", input, output, callback, include_pro, tier)
}

#[pyfunction]
#[pyo3(signature = (input, output=None, callback=None, include_pro=false, tier="open"))]
fn ln(input: &Raster, output: Option<&str>, callback: Option<Py<PyAny>>, include_pro: bool, tier: &str) -> PyResult<String> {
    _run_tool_convenient("ln", input, output, callback, include_pro, tier)
}

#[pyfunction]
#[pyo3(signature = (input, output=None, callback=None, include_pro=false, tier="open"))]
fn log10(input: &Raster, output: Option<&str>, callback: Option<Py<PyAny>>, include_pro: bool, tier: &str) -> PyResult<String> {
    _run_tool_convenient("log10", input, output, callback, include_pro, tier)
}

#[pyfunction]
#[pyo3(signature = (input, output=None, callback=None, include_pro=false, tier="open"))]
fn sin(input: &Raster, output: Option<&str>, callback: Option<Py<PyAny>>, include_pro: bool, tier: &str) -> PyResult<String> {
    _run_tool_convenient("sin", input, output, callback, include_pro, tier)
}

#[pyfunction]
#[pyo3(signature = (input, output=None, callback=None, include_pro=false, tier="open"))]
fn cos(input: &Raster, output: Option<&str>, callback: Option<Py<PyAny>>, include_pro: bool, tier: &str) -> PyResult<String> {
    _run_tool_convenient("cos", input, output, callback, include_pro, tier)
}

#[pyfunction]
#[pyo3(signature = (include_pro=false, tier="open", target="python"))]
fn generate_wrapper_stubs_json(include_pro: bool, tier: &str, target: &str) -> PyResult<String> {
    let parsed_tier = parse_tier(tier).map_err(map_tool_error)?;
    let rt = PythonToolRuntime::new_with_options(include_pro, parsed_tier)
        .map_err(map_tool_error)?;
    let target = match target.to_ascii_lowercase().as_str() {
        "python" => BindingTarget::Python,
        "r" => BindingTarget::R,
        _ => {
            return Err(PyValueError::new_err(
                "invalid target, expected 'python' or 'r'",
            ))
        }
    };

    let mut stubs = serde_json::Map::new();
    for manifest in rt.visible_manifests() {
        let mut stub = generate_wrapper_stub(&manifest, target);
        if matches!(manifest.license_tier, LicenseTier::Pro | LicenseTier::Enterprise)
            && matches!(target, BindingTarget::Python)
        {
            // Make tier visible in generated stubs so IDE hover/autocomplete surfaces it.
            stub = format!("# [PRO] {}\n{}", manifest.id, stub);
        }
        stubs.insert(manifest.id.clone(), Value::String(stub));
    }
    serde_json::to_string(&Value::Object(stubs))
        .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
}

#[pyfunction]
#[pyo3(signature = (floating_license_id=None, include_pro=None, tier="open", provider_url=None, machine_id=None, customer_id=None))]
fn whitebox_tools(
    floating_license_id: Option<&str>,
    include_pro: Option<bool>,
    tier: &str,
    provider_url: Option<&str>,
    machine_id: Option<&str>,
    customer_id: Option<&str>,
) -> PyResult<WbEnvironment> {
    let parsed_tier = parse_tier(tier).map_err(map_tool_error)?;
    let resolved_include_pro = include_pro.unwrap_or(floating_license_id.is_some());

    #[cfg(feature = "pro")]
    {
        let runtime = if let Some(license_id) = floating_license_id {
            PythonToolRuntime::new_with_floating_license_id(
                resolved_include_pro,
                parsed_tier,
                license_id,
                provider_url,
                machine_id,
                customer_id,
            )
            .map_err(map_tool_error)?
        } else {
            PythonToolRuntime::new_with_options(resolved_include_pro, parsed_tier)
                .map_err(map_tool_error)?
        };
        return Ok(WbEnvironment::from_runtime(runtime, resolved_include_pro));
    }

    #[cfg(not(feature = "pro"))]
    {
        let _ = (floating_license_id, provider_url, machine_id, customer_id);
        let runtime = PythonToolRuntime::new_with_options(resolved_include_pro, parsed_tier)
            .map_err(map_tool_error)?;
        Ok(WbEnvironment::from_runtime(runtime, resolved_include_pro))
    }
}

#[pymodule]
fn whitebox_workflows(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<RuntimeSession>()?;
    m.add_class::<Raster>()?;
    m.add_class::<RasterConfigs>()?;
    m.add_class::<VectorMetadata>()?;
    m.add_class::<LidarMetadata>()?;
    m.add_class::<Bundle>()?;
    m.add_class::<Vector>()?;
    m.add_class::<Lidar>()?;
    m.add_class::<WbEnvironment>()?;
    m.add_class::<WbToolCategory>()?;
    m.add_class::<WbToolSubcategory>()?;
    m.add_class::<WbCategoryToolCallable>()?;
    m.add_class::<WbDomainNamespace>()?;
    m.add_function(wrap_pyfunction!(list_tools_json, m)?)?;
    m.add_function(wrap_pyfunction!(list_tools, m)?)?;
    m.add_function(wrap_pyfunction!(list_tools_json_with_options, m)?)?;
    m.add_function(wrap_pyfunction!(list_tools_with_options, m)?)?;
    m.add_function(wrap_pyfunction!(list_tools_json_with_entitlement_options, m)?)?;
    m.add_function(wrap_pyfunction!(list_tools_json_with_entitlement_file_options, m)?)?;
    m.add_function(wrap_pyfunction!(run_tool_json, m)?)?;
    m.add_function(wrap_pyfunction!(run_tool, m)?)?;
    m.add_function(wrap_pyfunction!(run_tool_json_with_progress, m)?)?;
    m.add_function(wrap_pyfunction!(run_tool_json_stream, m)?)?;
    m.add_function(wrap_pyfunction!(run_tool_stream, m)?)?;
    m.add_function(wrap_pyfunction!(run_tool_json_with_options, m)?)?;
    m.add_function(wrap_pyfunction!(run_tool_json_with_entitlement_options, m)?)?;
    m.add_function(wrap_pyfunction!(run_tool_json_with_entitlement_file_options, m)?)?;
    m.add_function(wrap_pyfunction!(run_tool_with_options, m)?)?;
    m.add_function(wrap_pyfunction!(run_tool_json_with_progress_options, m)?)?;
    m.add_function(wrap_pyfunction!(run_tool_json_stream_options, m)?)?;
    m.add_function(wrap_pyfunction!(run_tool_stream_options, m)?)?;
    m.add_function(wrap_pyfunction!(generate_wrapper_stubs_json, m)?)?;
    m.add_function(wrap_pyfunction!(whitebox_tools, m)?)?;
    
    // Convenience functions for unary raster math tools
    m.add_function(wrap_pyfunction!(abs, m)?)?;
    m.add_function(wrap_pyfunction!(ceil, m)?)?;
    m.add_function(wrap_pyfunction!(floor, m)?)?;
    m.add_function(wrap_pyfunction!(round, m)?)?;
    m.add_function(wrap_pyfunction!(sqrt, m)?)?;
    m.add_function(wrap_pyfunction!(square, m)?)?;
    m.add_function(wrap_pyfunction!(ln, m)?)?;
    m.add_function(wrap_pyfunction!(log10, m)?)?;
    m.add_function(wrap_pyfunction!(sin, m)?)?;
    m.add_function(wrap_pyfunction!(cos, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    #[cfg(feature = "pro")]
    use std::sync::OnceLock;
    use pyo3::types::{PyDict, PyTuple};
    use wbcore::ProgressEvent;
    use wbraster::{DataType, Raster as WbRaster, RasterConfig, RasterFormat};

    #[derive(Default)]
    struct TestCollectSink {
        events: Mutex<Vec<ProgressEvent>>,
    }

    impl ProgressSink for TestCollectSink {
        fn info(&self, msg: &str) {
            if let Ok(mut events) = self.events.lock() {
                events.push(ProgressEvent::Info(msg.to_string()));
            }
        }

        fn progress(&self, pct: f64) {
            if let Ok(mut events) = self.events.lock() {
                events.push(ProgressEvent::Percent(pct));
            }
        }
    }

    #[cfg(feature = "pro")]
    fn license_env_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    #[cfg(feature = "pro")]
    struct EnvGuard {
        saved: Vec<(String, Option<String>)>,
    }

    #[cfg(feature = "pro")]
    impl EnvGuard {
        fn set(entries: &[(&str, Option<String>)]) -> Self {
            let mut saved = Vec::with_capacity(entries.len());
            for (key, new_val) in entries {
                saved.push(((*key).to_string(), std::env::var(key).ok()));
                match new_val {
                    Some(v) => unsafe { std::env::set_var(key, v) },
                    None => unsafe { std::env::remove_var(key) },
                }
            }
            Self { saved }
        }
    }

    #[cfg(feature = "pro")]
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, old_val) in &self.saved {
                match old_val {
                    Some(v) => unsafe { std::env::set_var(key, v) },
                    None => unsafe { std::env::remove_var(key) },
                }
            }
        }
    }

    #[cfg(feature = "pro")]
    fn unique_missing_state_path(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "wbw_python_license_state_{}_{}_{}.json",
            tag,
            std::process::id(),
            nanos
        ))
    }

    fn temp_raster_io_paths(tag: &str) -> (PathBuf, PathBuf) {
        let unique = format!(
            "{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock ok")
                .as_nanos()
        );
        let input = std::env::temp_dir().join(format!("wbw_py_{unique}_in.asc"));
        let output = std::env::temp_dir().join(format!("wbw_py_{unique}_out.asc"));
        (input, output)
    }

    fn write_small_input_raster(path: &PathBuf) {
        let mut raster = WbRaster::new(RasterConfig {
            cols: 2,
            rows: 1,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            cell_size_y: None,
            nodata: -9999.0,
            data_type: DataType::F64,
            crs: Default::default(),
            metadata: Vec::new(),
        });
        raster.set(0, 0, 0, -1.0).expect("set");
        raster.set(0, 0, 1, 2.0).expect("set");
        raster
            .write(path, RasterFormat::EsriAscii)
            .expect("write input raster");
    }

    #[test]
    fn list_tools_contains_known_tool() {
        let rt = PythonToolRuntime::new();
        let tools = rt.list_tools_json();
        let arr = tools.as_array().expect("list should be an array");
        let has_add = arr
            .iter()
            .any(|v| v.get("id").and_then(Value::as_str) == Some("add"));
        assert!(has_add);
    }

    #[test]
    fn run_tool_json_executes_registry_tool() {
        let rt = PythonToolRuntime::new();
        let (input, output) = temp_raster_io_paths("run_tool_json_executes_registry_tool");
        write_small_input_raster(&input);
        let args = format!(
            "{{\"input\":\"{}\",\"output\":\"{}\"}}",
            input.to_string_lossy(),
            output.to_string_lossy()
        );
        let out = rt
            .run_tool_json("abs", &args)
            .expect("tool should run");

        assert_eq!(out.get("cells_processed"), Some(&json!(2)));
        assert_eq!(out.get("output"), Some(&json!(output.to_string_lossy().to_string())));
        let _ = std::fs::remove_file(input);
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn runtime_can_execute_multiple_calls() {
        let rt = PythonToolRuntime::new();
        let (in1, out1) = temp_raster_io_paths("runtime_can_execute_multiple_calls_1");
        write_small_input_raster(&in1);
        let args1 = format!(
            "{{\"input\":\"{}\",\"output\":\"{}\"}}",
            in1.to_string_lossy(),
            out1.to_string_lossy()
        );
        let first = rt
            .run_tool_json("abs", &args1)
            .expect("first run should succeed");

        let (in2, out2) = temp_raster_io_paths("runtime_can_execute_multiple_calls_2");
        write_small_input_raster(&in2);
        let args2 = format!(
            "{{\"input\":\"{}\",\"output\":\"{}\"}}",
            in2.to_string_lossy(),
            out2.to_string_lossy()
        );
        let second = rt
            .run_tool_json("square", &args2)
            .expect("second run should succeed");

        assert_eq!(first.get("output"), Some(&json!(out1.to_string_lossy().to_string())));
        assert_eq!(second.get("output"), Some(&json!(out2.to_string_lossy().to_string())));
        let _ = std::fs::remove_file(in1);
        let _ = std::fs::remove_file(out1);
        let _ = std::fs::remove_file(in2);
        let _ = std::fs::remove_file(out2);
    }

    #[test]
    fn run_tool_json_with_progress_returns_progress_events() {
        let rt = PythonToolRuntime::new();
        let (input, output) = temp_raster_io_paths("run_tool_json_with_progress_returns_progress_events");
        write_small_input_raster(&input);
        let args = format!(
            "{{\"input\":\"{}\",\"output\":\"{}\"}}",
            input.to_string_lossy(),
            output.to_string_lossy()
        );
        let out = rt
            .run_tool_json_with_progress("abs", &args)
            .expect("tool should run");

        let progress = out
            .get("progress")
            .and_then(Value::as_array)
            .expect("progress should be array");
        assert!(!progress.is_empty());
        let _ = std::fs::remove_file(input);
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn run_tool_json_with_progress_sink_emits_live_events() {
        let rt = PythonToolRuntime::new();
        let sink = TestCollectSink::default();
        let (input, output) = temp_raster_io_paths("run_tool_json_with_progress_sink_emits_live_events");
        write_small_input_raster(&input);
        let args = format!(
            "{{\"input\":\"{}\",\"output\":\"{}\"}}",
            input.to_string_lossy(),
            output.to_string_lossy()
        );
        let _ = rt
            .run_tool_json_with_progress_sink("abs", &args, &sink)
            .expect("tool should run");

        let events = sink.events.lock().expect("events lock");
        assert!(!events.is_empty());
        let _ = std::fs::remove_file(input);
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn pro_tools_hidden_without_pro_options() {
        let rt = PythonToolRuntime::new();
        let tools = rt.list_tools_json();
        let arr = tools.as_array().expect("list should be an array");
        let has_pro = arr
            .iter()
            .any(|v| v.get("id").and_then(Value::as_str) == Some("raster_power"));
        assert!(!has_pro);
    }

    #[test]
    #[cfg(feature = "pro")]
    fn pro_tools_visible_and_runnable_with_pro_options() {
        let rt = PythonToolRuntime::new_with_options(true, LicenseTier::Pro)
            .expect("pro runtime construction should succeed");
        let tools = rt.list_tools_json();
        let arr = tools.as_array().expect("list should be an array");
        let has_pro = arr
            .iter()
            .any(|v| v.get("id").and_then(Value::as_str) == Some("raster_power"));
        assert!(has_pro);

        let out = rt
            .run_tool_json("raster_power", "{\"input\":[2,3],\"exponent\":2}")
            .expect("pro tool should run");
        assert_eq!(out.get("result"), Some(&json!([4.0, 9.0])));
    }

    #[test]
    #[cfg(feature = "pro")]
    fn provider_bootstrap_fail_open_with_missing_state_defaults_to_open() {
        let env_guard = license_env_lock().lock().expect("env lock");
        let state_path = unique_missing_state_path("fail_open");
        let _ = std::fs::remove_file(&state_path);

        let _guard = EnvGuard::set(&[
            ("WBW_LICENSE_PROVIDER_URL", Some("http://127.0.0.1:9".to_string())),
            ("WBW_LICENSE_POLICY", Some("fail_open".to_string())),
            (
                "WBW_LICENSE_STATE_PATH",
                Some(state_path.to_string_lossy().to_string()),
            ),
            ("WBW_LICENSE_LEASE_SECONDS", Some("3600".to_string())),
        ]);

        let rt = PythonToolRuntime::new_with_options(true, LicenseTier::Open)
            .expect("fail-open bootstrap should not block runtime construction");
        assert_eq!(rt.effective_tier(), LicenseTier::Open);

        let tools = rt.list_tools_json();
        let arr = tools.as_array().expect("list should be an array");
        let has_pro = arr
            .iter()
            .any(|v| v.get("id").and_then(Value::as_str) == Some("raster_power"));
        assert!(!has_pro, "expected OSS/open fallback to hide pro tools");

        let _ = std::fs::remove_file(state_path);
        drop(env_guard);
    }

    #[test]
    #[cfg(feature = "pro")]
    fn provider_bootstrap_fail_closed_with_missing_state_returns_error() {
        let env_guard = license_env_lock().lock().expect("env lock");
        let state_path = unique_missing_state_path("fail_closed");
        let _ = std::fs::remove_file(&state_path);

        let _guard = EnvGuard::set(&[
            ("WBW_LICENSE_PROVIDER_URL", Some("http://127.0.0.1:9".to_string())),
            ("WBW_LICENSE_POLICY", Some("fail_closed".to_string())),
            (
                "WBW_LICENSE_STATE_PATH",
                Some(state_path.to_string_lossy().to_string()),
            ),
            ("WBW_LICENSE_LEASE_SECONDS", Some("3600".to_string())),
        ]);

        match PythonToolRuntime::new_with_options(true, LicenseTier::Open) {
            Ok(_) => panic!("fail-closed bootstrap should reject runtime construction"),
            Err(err) => assert!(matches!(err, ToolError::LicenseDenied(_))),
        }

        let _ = std::fs::remove_file(state_path);
        drop(env_guard);
    }

    #[test]
    #[cfg(feature = "pro")]
    fn curvature_tools_are_visible_with_pro_options() {
        let rt = PythonToolRuntime::new_with_options(true, LicenseTier::Pro)
            .expect("pro runtime construction should succeed");
        let tools = rt.list_tools_json();
        let arr = tools.as_array().expect("list should be an array");

        let ids = [
            // OSS curvature tools
            "plan_curvature",
            "profile_curvature",
            "tangential_curvature",
            "total_curvature",
            "mean_curvature",
            "gaussian_curvature",
            // PRO curvature tools
            "minimal_curvature",
            "maximal_curvature",
            "shape_index",
            "curvedness",
            "unsphericity",
            "ring_curvature",
            "rotor",
            "difference_curvature",
            "horizontal_excess_curvature",
            "vertical_excess_curvature",
            "accumulation_curvature",
            "multiscale_curvatures",
            "generating_function",
            "principal_curvature_direction",
            "casorati_curvature",
        ];

        for id in ids {
            let present = arr
                .iter()
                .any(|v| v.get("id").and_then(Value::as_str) == Some(id));
            assert!(present, "expected curvature tool '{}' to be visible", id);
        }
    }

    #[test]
    #[cfg(not(feature = "pro"))]
    fn include_pro_rejected_when_pro_feature_disabled() {
        let err = match PythonToolRuntime::new_with_options(true, LicenseTier::Pro) {
            Ok(_) => panic!("include_pro should be rejected without 'pro' feature"),
            Err(err) => err,
        };
        assert!(matches!(err, ToolError::InvalidRequest(_)));
    }

    #[test]
    fn pro_curvature_tools_hidden_without_pro_options() {
        let rt = PythonToolRuntime::new();
        let tools = rt.list_tools_json();
        let arr = tools.as_array().expect("list should be an array");

        let pro_only_ids = [
            "minimal_curvature",
            "maximal_curvature",
            "shape_index",
            "curvedness",
            "unsphericity",
            "ring_curvature",
            "rotor",
            "difference_curvature",
            "horizontal_excess_curvature",
            "vertical_excess_curvature",
            "accumulation_curvature",
            "multiscale_curvatures",
            "generating_function",
            "principal_curvature_direction",
            "casorati_curvature",
        ];

        for id in pro_only_ids {
            let present = arr
                .iter()
                .any(|v| v.get("id").and_then(Value::as_str) == Some(id));
            assert!(!present, "pro-only curvature tool '{}' should be hidden", id);
        }
    }

    #[test]
    fn invalid_tier_rejected() {
        let err = parse_tier("gold").expect_err("should reject invalid tier");
        assert!(matches!(err, ToolError::InvalidRequest(_)));
    }

    #[test]
    fn wrapper_stub_generation_returns_known_tool() {
        let rt = PythonToolRuntime::new();
        let mut stubs = serde_json::Map::new();
        for manifest in rt.visible_manifests() {
            stubs.insert(
                manifest.id.clone(),
                Value::String(generate_wrapper_stub(&manifest, BindingTarget::Python)),
            );
        }
        let value = Value::Object(stubs);
        assert!(value.get("add").is_some());
    }

    #[test]
    fn typed_outputs_materialize_python_objects() {
        Python::initialize();
        Python::attach(|py| {
            let value = json!({
                "single_raster": {"__wbw_type__": "raster", "path": "dem.tif", "active_band": 2},
                "tuple_outputs": {
                    "__wbw_type__": "tuple",
                    "items": [
                        {"__wbw_type__": "raster", "path": "a.tif"},
                        {"__wbw_type__": "vector", "path": "roads.gpkg"}
                    ]
                },
                "nested": [
                    {"__wbw_type__": "lidar", "path": "tile.laz"},
                    123
                ]
            });

            let obj = json_value_to_python_object(py, &value).expect("conversion should succeed");
            let dict = obj
                .bind(py)
                .cast::<PyDict>()
                .expect("top-level should be dict");

            let single = dict
                .get_item("single_raster")
                .expect("dict lookup should succeed")
                .expect("key should exist");
            assert!(single.is_instance_of::<Raster>());

            let tuple_any = dict
                .get_item("tuple_outputs")
                .expect("dict lookup should succeed")
                .expect("key should exist");
            let tuple = tuple_any
                .cast::<PyTuple>()
                .expect("tuple_outputs should be tuple");
            assert_eq!(tuple.len(), 2);
            assert!(tuple.get_item(0).expect("tuple item").is_instance_of::<Raster>());
            assert!(tuple.get_item(1).expect("tuple item").is_instance_of::<Vector>());
        });
    }

    #[test]
    fn typed_output_missing_path_is_rejected() {
        Python::initialize();
        Python::attach(|py| {
            let bad = json!({"__wbw_type__": "raster"});
            assert!(json_value_to_python_object(py, &bad).is_err());
        });
    }

    #[test]
    fn typed_run_tool_add_returns_raster_instance() {
        use std::fs;
        use std::path::PathBuf;
        use std::time::{SystemTime, UNIX_EPOCH};
        use pyo3::types::PyDict;
        use wbraster::{DataType, Raster as WbRaster, RasterConfig, RasterFormat};

        struct TempDirGuard {
            path: PathBuf,
        }

        impl TempDirGuard {
            fn new(prefix: &str) -> Self {
                let nanos = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0);
                let path = std::env::temp_dir().join(format!(
                    "wbw_python_add_typed_{}_{}_{}",
                    prefix,
                    std::process::id(),
                    nanos
                ));
                fs::create_dir_all(&path).unwrap();
                Self { path }
            }
        }

        impl Drop for TempDirGuard {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.path);
            }
        }

        fn write_raster(path: &str, values: [f64; 4]) {
            let mut r = WbRaster::new(RasterConfig {
                cols: 2,
                rows: 2,
                bands: 1,
                x_min: 0.0,
                y_min: 0.0,
                cell_size: 1.0,
                nodata: -9999.0,
                data_type: DataType::F32,
                ..Default::default()
            });
            r.set(0, 0, 0, values[0]).unwrap();
            r.set(0, 0, 1, values[1]).unwrap();
            r.set(0, 1, 0, values[2]).unwrap();
            r.set(0, 1, 1, values[3]).unwrap();
            r.write(path, RasterFormat::GeoTiff).unwrap();
        }

        let td = TempDirGuard::new("run_tool_add");
        let input1 = td.path.join("a.tif");
        let input2 = td.path.join("b.tif");
        let output = td.path.join("sum.tif");

        write_raster(input1.to_str().unwrap(), [1.0, 2.0, 3.0, 4.0]);
        write_raster(input2.to_str().unwrap(), [5.0, 6.0, 7.0, 8.0]);

        Python::initialize();
        Python::attach(|py| {
            let input1_obj = Py::new(
                py,
                Raster {
                    file_path: input1.clone(),
                    active_band: 0,
                },
            )
            .expect("raster object should be constructible");
            let input2_obj = Py::new(
                py,
                Raster {
                    file_path: input2.clone(),
                    active_band: 0,
                },
            )
            .expect("raster object should be constructible");

            let first_args = PyDict::new(py);
            first_args
                .set_item("input1", input1_obj.bind(py))
                .expect("set input1");
            first_args
                .set_item("input2", input2_obj.bind(py))
                .expect("set input2");

            let first = run_tool(py, "add", first_args.as_any())
                .expect("typed run_tool first call should succeed");
            assert!(first.bind(py).is_instance_of::<Raster>());

            let first_raster: pyo3::PyRef<'_, Raster> = first
                .bind(py)
                .extract()
                .expect("first output should extract as Raster");
            assert!(
                first_raster
                    .file_path
                    .to_string_lossy()
                    .starts_with("memory://raster/")
            );
            drop(first_raster);

            let second_args = PyDict::new(py);
            second_args
                .set_item("input1", first.bind(py))
                .expect("set input1 second");
            second_args
                .set_item("input2", input1_obj.bind(py))
                .expect("set input2 second");
            second_args
                .set_item("output", output.to_string_lossy().to_string())
                .expect("set output second");

            let second = run_tool(py, "add", second_args.as_any())
                .expect("typed run_tool second call should succeed");
            assert!(second.bind(py).is_instance_of::<Raster>());
        });

        let out = WbRaster::read(output.to_str().unwrap()).expect("output raster should exist");
        assert_eq!(out.get(0, 0, 0), 7.0);
        assert_eq!(out.get(0, 0, 1), 10.0);
        assert_eq!(out.get(0, 1, 0), 13.0);
        assert_eq!(out.get(0, 1, 1), 16.0);
    }
}
