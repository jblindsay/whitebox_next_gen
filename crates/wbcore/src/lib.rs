use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub mod tool_args_ext;
pub use tool_args_ext::{
    parse_optional_output_path,
    parse_raster_path_arg,
    parse_raster_path_value,
    parse_vector_path_arg,
    parse_vector_path_value,
};

pub type ToolArgs = BTreeMap<String, Value>;
pub type ToolId = &'static str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LicenseTier {
    Open,
    Pro,
    Enterprise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCategory {
    Raster,
    Vector,
    Lidar,
    Topology,
    Hydrology,
    Terrain,
    Conversion,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParamSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub id: ToolId,
    pub display_name: &'static str,
    pub summary: &'static str,
    pub category: ToolCategory,
    pub license_tier: LicenseTier,
    pub params: Vec<ToolParamSpec>,
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("license denied: {0}")]
    LicenseDenied(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("execution error: {0}")]
    Execution(String),
}

pub trait CapabilityProvider: Send + Sync {
    fn has_tool_access(&self, tool_id: ToolId, tier: LicenseTier) -> bool;
}

pub trait ProgressSink: Send + Sync {
    fn info(&self, _msg: &str) {}
    fn progress(&self, _pct: f64) {}
}

/// Coalesces progress into integer-percent buckets and emits each bucket at most once.
///
/// This is designed for parallel loops: workers can call `emit_unit_fraction` frequently,
/// while actual callback emissions remain bounded and monotonic.
pub struct PercentCoalescer {
    min_bucket: usize,
    max_bucket: usize,
    next_bucket: AtomicUsize,
}

impl PercentCoalescer {
    pub fn new(min_bucket: usize, max_bucket: usize) -> Self {
        assert!(min_bucket <= max_bucket, "min_bucket must be <= max_bucket");
        assert!(max_bucket <= 100, "max_bucket must be <= 100");
        Self {
            min_bucket,
            max_bucket,
            next_bucket: AtomicUsize::new(min_bucket),
        }
    }

    pub fn emit_unit_fraction(&self, sink: &dyn ProgressSink, fraction01: f64) {
        let clamped = fraction01.clamp(0.0, 1.0);
        let span = self.max_bucket.saturating_sub(self.min_bucket);
        let target = self.min_bucket + ((clamped * span as f64).floor() as usize);
        self.emit_to_bucket(sink, target);
    }

    pub fn finish(&self, sink: &dyn ProgressSink) {
        self.emit_to_bucket(sink, self.max_bucket);
    }

    fn emit_to_bucket(&self, sink: &dyn ProgressSink, mut target: usize) {
        if target > self.max_bucket {
            target = self.max_bucket;
        }

        loop {
            let next = self.next_bucket.load(Ordering::Relaxed);
            if next > target || next > self.max_bucket {
                break;
            }

            if self
                .next_bucket
                .compare_exchange(next, next + 1, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                sink.progress((next as f64) / 100.0);
            }
        }
    }
}

pub struct ToolContext<'a> {
    pub progress: &'a dyn ProgressSink,
    pub capabilities: &'a dyn CapabilityProvider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressEvent {
    Info(String),
    Percent(f64),
}

#[derive(Default)]
pub struct RecordingProgressSink {
    events: Mutex<Vec<ProgressEvent>>,
}

impl RecordingProgressSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn take_events(self) -> Vec<ProgressEvent> {
        self.events.into_inner().unwrap_or_else(|_| Vec::new())
    }
}

impl ProgressSink for RecordingProgressSink {
    fn info(&self, msg: &str) {
        if let Ok(mut events) = self.events.lock() {
            events.push(ProgressEvent::Info(msg.to_string()));
        }
    }

    fn progress(&self, pct: f64) {
        if let Ok(mut events) = self.events.lock() {
            events.push(ProgressEvent::Percent(pct.clamp(0.0, 1.0)));
        }
    }
}

struct TeeProgressSink<'a> {
    external: &'a dyn ProgressSink,
    recorder: &'a RecordingProgressSink,
}

impl ProgressSink for TeeProgressSink<'_> {
    fn info(&self, msg: &str) {
        self.external.info(msg);
        self.recorder.info(msg);
    }

    fn progress(&self, pct: f64) {
        self.external.progress(pct);
        self.recorder.progress(pct);
    }
}

struct NullProgressSink;

impl ProgressSink for NullProgressSink {}

pub struct AllowAllCapabilities;

impl CapabilityProvider for AllowAllCapabilities {
    fn has_tool_access(&self, _tool_id: ToolId, _tier: LicenseTier) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MaxTierCapabilities {
    pub max_tier: LicenseTier,
}

impl CapabilityProvider for MaxTierCapabilities {
    fn has_tool_access(&self, _tool_id: ToolId, tier: LicenseTier) -> bool {
        tier <= self.max_tier
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParamDescriptor {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub id: String,
    pub display_name: String,
    pub summary: String,
    pub category: ToolCategory,
    pub license_tier: LicenseTier,
    pub params: Vec<ToolParamDescriptor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolStability {
    Experimental,
    Beta,
    Stable,
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub name: String,
    pub description: String,
    pub args: ToolArgs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    pub id: String,
    pub display_name: String,
    pub summary: String,
    pub category: ToolCategory,
    pub license_tier: LicenseTier,
    pub params: Vec<ToolParamDescriptor>,
    pub defaults: ToolArgs,
    pub examples: Vec<ToolExample>,
    pub tags: Vec<String>,
    pub stability: ToolStability,
}

impl From<ToolMetadata> for ToolDescriptor {
    fn from(m: ToolMetadata) -> Self {
        let params = m
            .params
            .into_iter()
            .map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            })
            .collect();

        Self {
            id: m.id.to_string(),
            display_name: m.display_name.to_string(),
            summary: m.summary.to_string(),
            category: m.category,
            license_tier: m.license_tier,
            params,
        }
    }
}

impl From<ToolManifest> for ToolDescriptor {
    fn from(m: ToolManifest) -> Self {
        Self {
            id: m.id,
            display_name: m.display_name,
            summary: m.summary,
            category: m.category,
            license_tier: m.license_tier,
            params: m.params,
        }
    }
}

impl From<ToolMetadata> for ToolManifest {
    fn from(m: ToolMetadata) -> Self {
        let params = m
            .params
            .into_iter()
            .map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            })
            .collect();

        Self {
            id: m.id.to_string(),
            display_name: m.display_name.to_string(),
            summary: m.summary.to_string(),
            category: m.category,
            license_tier: m.license_tier,
            params,
            defaults: ToolArgs::new(),
            examples: Vec::new(),
            tags: Vec::new(),
            stability: ToolStability::Stable,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub tool_id: String,
    pub args: ToolArgs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResponse {
    pub tool_id: String,
    pub outputs: BTreeMap<String, Value>,
    pub progress: Vec<ProgressEvent>,
}

pub trait ToolRuntimeRegistry: Send + Sync {
    fn list_tools(&self) -> Vec<ToolMetadata>;
    fn list_manifests(&self) -> Vec<ToolManifest> {
        self.list_tools().into_iter().map(ToolManifest::from).collect()
    }
    fn run_tool(&self, id: &str, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError>;
}

pub struct ToolRuntime<'a, R, C>
where
    R: ToolRuntimeRegistry,
    C: CapabilityProvider,
{
    pub registry: &'a R,
    pub capabilities: &'a C,
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimeOptions {
    pub max_tier: LicenseTier,
    pub expose_locked_tools: bool,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            max_tier: LicenseTier::Open,
            expose_locked_tools: false,
        }
    }
}

pub struct OwnedToolRuntime<R>
where
    R: ToolRuntimeRegistry,
{
    pub registry: R,
    pub options: RuntimeOptions,
    capabilities: MaxTierCapabilities,
}

pub struct OwnedToolRuntimeWithCapabilities<R, C>
where
    R: ToolRuntimeRegistry,
    C: CapabilityProvider,
{
    pub registry: R,
    pub options: RuntimeOptions,
    capabilities: C,
}

impl<R> OwnedToolRuntime<R>
where
    R: ToolRuntimeRegistry,
{
    pub fn with_options(registry: R, options: RuntimeOptions) -> Self {
        let capabilities = MaxTierCapabilities {
            max_tier: options.max_tier,
        };
        Self {
            registry,
            options,
            capabilities,
        }
    }

    pub fn runtime(&self) -> ToolRuntime<'_, R, MaxTierCapabilities> {
        ToolRuntime {
            registry: &self.registry,
            capabilities: &self.capabilities,
        }
    }

    pub fn list_visible_manifests(&self) -> Vec<ToolManifest> {
        let manifests = self.runtime().list_manifests();
        if self.options.expose_locked_tools {
            return manifests;
        }

        let allowed_ids: BTreeSet<String> = self
            .registry
            .list_tools()
            .into_iter()
            .filter(|m| self.capabilities.has_tool_access(m.id, m.license_tier))
            .map(|m| m.id.to_string())
            .collect();

        manifests
            .into_iter()
            .filter(|m| allowed_ids.contains(&m.id))
            .collect()
    }

    pub fn execute(&self, req: ExecuteRequest) -> Result<ExecuteResponse, ToolError> {
        self.runtime().execute(req)
    }

    pub fn execute_with_progress_sink(
        &self,
        req: ExecuteRequest,
        progress: &dyn ProgressSink,
    ) -> Result<ExecuteResponse, ToolError> {
        self.runtime().execute_with_progress_sink(req, progress)
    }
}

impl<R, C> OwnedToolRuntimeWithCapabilities<R, C>
where
    R: ToolRuntimeRegistry,
    C: CapabilityProvider,
{
    pub fn new(registry: R, options: RuntimeOptions, capabilities: C) -> Self {
        Self {
            registry,
            options,
            capabilities,
        }
    }

    pub fn runtime(&self) -> ToolRuntime<'_, R, C> {
        ToolRuntime {
            registry: &self.registry,
            capabilities: &self.capabilities,
        }
    }

    pub fn list_visible_manifests(&self) -> Vec<ToolManifest> {
        let manifests = self.runtime().list_manifests();
        if self.options.expose_locked_tools {
            return manifests;
        }

        let allowed_ids: BTreeSet<String> = self
            .registry
            .list_tools()
            .into_iter()
            .filter(|m| self.capabilities.has_tool_access(m.id, m.license_tier))
            .map(|m| m.id.to_string())
            .collect();

        manifests
            .into_iter()
            .filter(|m| allowed_ids.contains(&m.id))
            .collect()
    }

    pub fn execute(&self, req: ExecuteRequest) -> Result<ExecuteResponse, ToolError> {
        self.runtime().execute(req)
    }

    pub fn execute_with_progress_sink(
        &self,
        req: ExecuteRequest,
        progress: &dyn ProgressSink,
    ) -> Result<ExecuteResponse, ToolError> {
        self.runtime().execute_with_progress_sink(req, progress)
    }
}

pub struct ToolRuntimeBuilder<R>
where
    R: ToolRuntimeRegistry,
{
    registry: R,
    options: RuntimeOptions,
}

impl<R> ToolRuntimeBuilder<R>
where
    R: ToolRuntimeRegistry,
{
    pub fn new(registry: R) -> Self {
        Self {
            registry,
            options: RuntimeOptions::default(),
        }
    }

    pub fn max_tier(mut self, tier: LicenseTier) -> Self {
        self.options.max_tier = tier;
        self
    }

    pub fn expose_locked_tools(mut self, expose: bool) -> Self {
        self.options.expose_locked_tools = expose;
        self
    }

    pub fn build(self) -> OwnedToolRuntime<R> {
        OwnedToolRuntime::with_options(self.registry, self.options)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BindingTarget {
    Python,
    R,
}

pub fn generate_wrapper_stub(manifest: &ToolManifest, target: BindingTarget) -> String {
    let fn_name = manifest.id.replace('-', "_");
    match target {
        BindingTarget::Python => format!(
            "def {fn_name}(**kwargs):\n    \"\"\"{summary}\"\"\"\n    return run_tool_json('{tool_id}', kwargs)\n",
            summary = manifest.summary,
            tool_id = manifest.id,
        ),
        BindingTarget::R => format!(
            "{fn_name} <- function(...) {{\n  # {summary}\n  run_tool_json('{tool_id}', list(...))\n}}\n",
            summary = manifest.summary,
            tool_id = manifest.id,
        ),
    }
}

impl<'a, R, C> ToolRuntime<'a, R, C>
where
    R: ToolRuntimeRegistry,
    C: CapabilityProvider,
{
    pub fn list_manifests(&self) -> Vec<ToolManifest> {
        self.registry.list_manifests()
    }

    pub fn list_descriptors(&self) -> Vec<ToolDescriptor> {
        self.registry
            .list_manifests()
            .into_iter()
            .map(ToolDescriptor::from)
            .collect()
    }

    pub fn execute(&self, req: ExecuteRequest) -> Result<ExecuteResponse, ToolError> {
        let null_progress = NullProgressSink;
        self.execute_with_progress_sink(req, &null_progress)
    }

    pub fn execute_with_progress_sink(
        &self,
        req: ExecuteRequest,
        progress: &dyn ProgressSink,
    ) -> Result<ExecuteResponse, ToolError> {
        if req.tool_id.trim().is_empty() {
            return Err(ToolError::InvalidRequest("tool_id cannot be empty".to_string()));
        }

        let recorded = RecordingProgressSink::new();
        let tee = TeeProgressSink {
            external: progress,
            recorder: &recorded,
        };
        let ctx = ToolContext {
            progress: &tee,
            capabilities: self.capabilities,
        };

        let result = self.registry.run_tool(&req.tool_id, &req.args, &ctx)?;
        Ok(ExecuteResponse {
            tool_id: req.tool_id,
            outputs: result.outputs,
            progress: recorded.take_events(),
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolRunResult {
    pub outputs: BTreeMap<String, Value>,
}

pub trait Tool: Send + Sync {
    fn metadata(&self) -> ToolMetadata;
    fn manifest(&self) -> ToolManifest {
        ToolManifest::from(self.metadata())
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError>;
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct DemoRegistry;

    impl ToolRuntimeRegistry for DemoRegistry {
        fn list_tools(&self) -> Vec<ToolMetadata> {
            vec![ToolMetadata {
                id: "demo_add",
                display_name: "Demo Add",
                summary: "Adds a constant to each value",
                category: ToolCategory::Raster,
                license_tier: LicenseTier::Open,
                params: vec![
                    ToolParamSpec {
                        name: "input",
                        description: "Input values",
                        required: true,
                    },
                    ToolParamSpec {
                        name: "constant",
                        description: "Added value",
                        required: true,
                    },
                ],
            }]
        }

        fn run_tool(&self, id: &str, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
            if id != "demo_add" {
                return Err(ToolError::NotFound(id.to_string()));
            }
            if !ctx.capabilities.has_tool_access("demo_add", LicenseTier::Open) {
                return Err(ToolError::LicenseDenied("demo_add".to_string()));
            }

            let input = args
                .get("input")
                .and_then(Value::as_array)
                .ok_or_else(|| ToolError::Validation("missing input".to_string()))?;
            let c = args
                .get("constant")
                .and_then(Value::as_f64)
                .ok_or_else(|| ToolError::Validation("missing constant".to_string()))?;

            ctx.progress.info("running demo_add");
            let mut out = Vec::with_capacity(input.len());
            for (i, v) in input.iter().enumerate() {
                let n = v
                    .as_f64()
                    .ok_or_else(|| ToolError::Validation("non-numeric input".to_string()))?;
                out.push(n + c);
                ctx.progress.progress((i + 1) as f64 / input.len().max(1) as f64);
            }

            let mut outputs = BTreeMap::new();
            outputs.insert("result".to_string(), json!(out));
            Ok(ToolRunResult { outputs })
        }
    }

    #[test]
    fn max_tier_capabilities_respect_ordering() {
        let caps = MaxTierCapabilities {
            max_tier: LicenseTier::Open,
        };
        assert!(caps.has_tool_access("x", LicenseTier::Open));
        assert!(!caps.has_tool_access("x", LicenseTier::Pro));
    }

    #[test]
    fn runtime_execute_captures_outputs_and_progress() {
        let runtime = ToolRuntime {
            registry: &DemoRegistry,
            capabilities: &AllowAllCapabilities,
        };

        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!([1.0, 2.0]));
        args.insert("constant".to_string(), json!(3.0));

        let response = runtime
            .execute(ExecuteRequest {
                tool_id: "demo_add".to_string(),
                args,
            })
            .expect("execution should succeed");

        assert_eq!(response.tool_id, "demo_add");
        assert_eq!(response.outputs.get("result"), Some(&json!([4.0, 5.0])));
        assert!(response
            .progress
            .iter()
            .any(|e| matches!(e, ProgressEvent::Info(msg) if msg == "running demo_add")));
    }

    #[test]
    fn list_descriptors_returns_owned_metadata() {
        let runtime = ToolRuntime {
            registry: &DemoRegistry,
            capabilities: &AllowAllCapabilities,
        };

        let list = runtime.list_descriptors();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "demo_add");
        assert_eq!(list[0].params.len(), 2);
    }

    #[test]
    fn list_manifests_contains_default_stability() {
        let runtime = ToolRuntime {
            registry: &DemoRegistry,
            capabilities: &AllowAllCapabilities,
        };

        let manifests = runtime.list_manifests();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].id, "demo_add");
        assert_eq!(manifests[0].stability, ToolStability::Stable);
    }

    #[test]
    fn owned_runtime_filters_locked_tools() {
        let registry = DemoRegistry;
        let runtime = ToolRuntimeBuilder::new(registry)
            .max_tier(LicenseTier::Open)
            .build();

        let manifests = runtime.list_visible_manifests();
        assert_eq!(manifests.len(), 1);
    }

    #[test]
    fn wrapper_stub_generation_produces_expected_prefix() {
        let manifest = ToolManifest {
            id: "demo_add".to_string(),
            display_name: "Demo Add".to_string(),
            summary: "Adds values".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: Vec::new(),
            defaults: ToolArgs::new(),
            examples: Vec::new(),
            tags: Vec::new(),
            stability: ToolStability::Stable,
        };

        let py = generate_wrapper_stub(&manifest, BindingTarget::Python);
        let r = generate_wrapper_stub(&manifest, BindingTarget::R);
        assert!(py.starts_with("def demo_add"));
        assert!(r.starts_with("demo_add <- function"));
    }

    #[test]
    fn percent_coalescer_emits_each_bucket_once() {
        let sink = RecordingProgressSink::new();
        let c = PercentCoalescer::new(1, 5);

        c.emit_unit_fraction(&sink, 0.0);
        c.emit_unit_fraction(&sink, 0.4);
        c.emit_unit_fraction(&sink, 1.0);
        c.finish(&sink);

        let events = sink.take_events();
        let percents: Vec<f64> = events
            .into_iter()
            .filter_map(|e| match e {
                ProgressEvent::Percent(p) => Some(p),
                _ => None,
            })
            .collect();

        assert_eq!(percents, vec![0.01, 0.02, 0.03, 0.04, 0.05]);
    }
}
