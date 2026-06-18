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
    IMPLICIT_MEMORY_VECTOR_OUTPUT_PATH,
};

pub type ToolArgs = BTreeMap<String, Value>;
pub type ToolId = &'static str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LicenseTier {
    #[serde(alias = "Open")]
    #[serde(alias = "open")]
    Open,
    #[serde(alias = "Pro")]
    #[serde(alias = "pro")]
    Pro,
    #[serde(alias = "Enterprise")]
    #[serde(alias = "enterprise")]
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

impl Default for ToolParamSpec {
    fn default() -> Self {
        Self {
            name: "",
            description: "",
            required: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolValueCardinality {
    Single,
    Multiple,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolVectorGeometry {
    Point,
    Line,
    Polygon,
    LineOrPolygon,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolScalarKind {
    Integer,
    Float,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolDatasetSchema {
    Raster,
    Vector { geometry: ToolVectorGeometry },
    Lidar,
    Table,
    Json,
    Text,
    File,
    Mixed { members: Vec<ToolDatasetSchema> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolInputMode {
    Existing,
    ExistingOrNumber,
    ExistingOrString,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutputMode {
    New,
    Report,
    Sidecar,
    InPlace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInputSchema {
    pub mode: ToolInputMode,
    pub dataset: ToolDatasetSchema,
    pub cardinality: ToolValueCardinality,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolOutputSchema {
    pub mode: ToolOutputMode,
    pub dataset: ToolDatasetSchema,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolEnumOption {
    pub value: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolEnumSchema {
    pub options: Vec<ToolEnumOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolFieldSchema {
    pub parent: String,
    pub geometry: Option<ToolVectorGeometry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolParamSchema {
    Input(ToolInputSchema),
    Output(ToolOutputSchema),
    Scalar { scalar: ToolScalarKind },
    Enum(ToolEnumSchema),
    Bool,
    String,
    Field(ToolFieldSchema),
    FieldDefinition,
}

impl ToolParamSchema {
    pub fn input(dataset: ToolDatasetSchema) -> Self {
        Self::Input(ToolInputSchema {
            mode: ToolInputMode::Existing,
            dataset,
            cardinality: ToolValueCardinality::Single,
        })
    }

    pub fn input_multiple(dataset: ToolDatasetSchema) -> Self {
        Self::Input(ToolInputSchema {
            mode: ToolInputMode::Existing,
            dataset,
            cardinality: ToolValueCardinality::Multiple,
        })
    }

    pub fn input_raster() -> Self {
        Self::input(ToolDatasetSchema::Raster)
    }

    pub fn input_vector_any() -> Self {
        Self::input(ToolDatasetSchema::Vector {
            geometry: ToolVectorGeometry::Any,
        })
    }

    pub fn input_vector(geometry: ToolVectorGeometry) -> Self {
        Self::input(ToolDatasetSchema::Vector { geometry })
    }

    pub fn input_lidar() -> Self {
        Self::input(ToolDatasetSchema::Lidar)
    }

    pub fn input_existing_or_number(dataset: ToolDatasetSchema) -> Self {
        Self::Input(ToolInputSchema {
            mode: ToolInputMode::ExistingOrNumber,
            dataset,
            cardinality: ToolValueCardinality::Single,
        })
    }

    pub fn output(dataset: ToolDatasetSchema) -> Self {
        Self::Output(ToolOutputSchema {
            mode: ToolOutputMode::New,
            dataset,
        })
    }

    pub fn output_raster() -> Self {
        Self::output(ToolDatasetSchema::Raster)
    }

    pub fn output_vector_any() -> Self {
        Self::output(ToolDatasetSchema::Vector {
            geometry: ToolVectorGeometry::Any,
        })
    }

    pub fn bool() -> Self {
        Self::Bool
    }

    pub fn string() -> Self {
        Self::String
    }

    pub fn scalar_integer() -> Self {
        Self::Scalar {
            scalar: ToolScalarKind::Integer,
        }
    }

    pub fn scalar_float() -> Self {
        Self::Scalar {
            scalar: ToolScalarKind::Float,
        }
    }

    pub fn field(parent: &str, geometry: Option<ToolVectorGeometry>) -> Self {
        Self::Field(ToolFieldSchema {
            parent: parent.to_string(),
            geometry,
        })
    }

    pub fn field_definition() -> Self {
        Self::FieldDefinition
    }

    pub fn enum_values(options: &[&str]) -> Self {
        Self::Enum(ToolEnumSchema {
            options: options
                .iter()
                .map(|value| ToolEnumOption {
                    value: (*value).to_string(),
                    label: None,
                })
                .collect(),
        })
    }

    pub fn io_role(&self) -> Option<ToolIoRole> {
        match self {
            Self::Input(_) => Some(ToolIoRole::Input),
            Self::Output(_) => Some(ToolIoRole::Output),
            Self::Scalar { .. } | Self::Enum(_) | Self::Bool | Self::String | Self::Field(_) | Self::FieldDefinition => None,
        }
    }

    pub fn coarse_data_kind(&self) -> ToolDataKind {
        match self {
            Self::Input(schema) => schema.dataset.coarse_data_kind(),
            Self::Output(schema) => schema.dataset.coarse_data_kind(),
            Self::Scalar { .. } => ToolDataKind::Number,
            Self::Enum(_) | Self::String | Self::Field(_) | Self::FieldDefinition => ToolDataKind::String,
            Self::Bool => ToolDataKind::Bool,
        }
    }
}

impl ToolDatasetSchema {
    pub fn coarse_data_kind(&self) -> ToolDataKind {
        match self {
            Self::Raster => ToolDataKind::Raster,
            Self::Vector { .. } => ToolDataKind::Vector,
            Self::Lidar => ToolDataKind::Lidar,
            Self::Table => ToolDataKind::Table,
            Self::Json => ToolDataKind::Json,
            Self::Text => ToolDataKind::Text,
            Self::File => ToolDataKind::File,
            Self::Mixed { members } => members
                .first()
                .map(ToolDatasetSchema::coarse_data_kind)
                .unwrap_or(ToolDataKind::Unknown),
        }
    }
}

pub fn param_schema_map(entries: &[(&str, ToolParamSchema)]) -> BTreeMap<String, ToolParamSchema> {
    entries
        .iter()
        .map(|(name, schema)| ((*name).to_string(), schema.clone()))
        .collect()
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
#[serde(rename_all = "snake_case")]
pub enum ToolIoRole {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolDataKind {
    Raster,
    Vector,
    Lidar,
    Table,
    Json,
    Text,
    File,
    Bool,
    Number,
    String,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParamDescriptor {
    pub name: String,
    pub description: String,
    pub required: bool,
}

impl Default for ToolParamDescriptor {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            required: false,
        }
    }
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

impl From<ToolParamSpec> for ToolParamDescriptor {
    fn from(p: ToolParamSpec) -> Self {
        Self {
            name: p.name.to_string(),
            description: p.description.to_string(),
            required: p.required,
        }
    }
}

impl From<&ToolParamSpec> for ToolParamDescriptor {
    fn from(p: &ToolParamSpec) -> Self {
        Self {
            name: p.name.to_string(),
            description: p.description.to_string(),
            required: p.required,
        }
    }
}

fn looks_like_output_param(name: &str, description: &str) -> bool {
    let n = name.trim().to_ascii_lowercase();
    let d = description.trim().to_ascii_lowercase();

    if matches!(
        n.as_str(),
        "output" | "out" | "output_file" | "output_path" | "destination" | "dst"
    ) {
        return true;
    }
    if n.starts_with("output_") || n.starts_with("out_") || n.starts_with("destination_") {
        return true;
    }

    let persist_markers = [
        "output file",
        "output path",
        "destination file",
        "destination path",
        "save to",
        "write to",
        "report file",
    ];
    persist_markers.iter().any(|m| d.contains(m))
}

fn infer_data_kind(name: &str, description: &str, role: &ToolIoRole) -> ToolDataKind {
    let text = format!("{} {}", name.to_ascii_lowercase(), description.to_ascii_lowercase());

    if text.contains("raster")
        || text.contains("dem")
        || text.contains("geotiff")
        || text.contains(".tif")
        || text.contains(".tiff")
        || text.contains("grid")
    {
        return ToolDataKind::Raster;
    }
    if text.contains("vector")
        || text.contains("feature")
        || text.contains("geopackage")
        || text.contains("gpkg")
        || text.contains("geojson")
        || text.contains("topojson")
        || text.contains(".shp")
    {
        return ToolDataKind::Vector;
    }
    if text.contains("lidar")
        || text.contains("las")
        || text.contains("laz")
        || text.contains("copc")
        || text.contains("e57")
        || text.contains("ply")
        || text.contains("zlidar")
    {
        return ToolDataKind::Lidar;
    }
    if text.contains("csv") || text.contains("table") {
        return ToolDataKind::Table;
    }
    if text.contains("json") {
        return ToolDataKind::Json;
    }
    if text.contains("txt") || text.contains("text") || text.contains("html") || text.contains("xml") {
        return ToolDataKind::Text;
    }

    if matches!(role, ToolIoRole::Output) {
        return ToolDataKind::File;
    }
    ToolDataKind::String
}

fn role_and_kind_from_schema(schema: &ToolParamSchema) -> (Option<ToolIoRole>, ToolDataKind) {
    (schema.io_role(), schema.coarse_data_kind())
}

fn schema_from_role_and_kind(role: Option<ToolIoRole>, kind: ToolDataKind) -> Option<ToolParamSchema> {
    let dataset_from_kind = |k: ToolDataKind| -> Option<ToolDatasetSchema> {
        match k {
            ToolDataKind::Raster => Some(ToolDatasetSchema::Raster),
            ToolDataKind::Vector => Some(ToolDatasetSchema::Vector {
                geometry: ToolVectorGeometry::Any,
            }),
            ToolDataKind::Lidar => Some(ToolDatasetSchema::Lidar),
            ToolDataKind::Table => Some(ToolDatasetSchema::Table),
            ToolDataKind::Json => Some(ToolDatasetSchema::Json),
            ToolDataKind::Text => Some(ToolDatasetSchema::Text),
            ToolDataKind::File => Some(ToolDatasetSchema::File),
            _ => None,
        }
    };

    match (role, kind) {
        (Some(ToolIoRole::Input), k) => {
            if let Some(ds) = dataset_from_kind(k.clone()) {
                Some(ToolParamSchema::input(ds))
            } else {
                match k {
                    ToolDataKind::Bool => Some(ToolParamSchema::bool()),
                    ToolDataKind::Number => Some(ToolParamSchema::scalar_float()),
                    ToolDataKind::String | ToolDataKind::Unknown => Some(ToolParamSchema::string()),
                    _ => None,
                }
            }
        }
        (Some(ToolIoRole::Output), k) => {
            if let Some(ds) = dataset_from_kind(k.clone()) {
                Some(ToolParamSchema::output(ds))
            } else {
                match k {
                    ToolDataKind::Bool => Some(ToolParamSchema::bool()),
                    ToolDataKind::Number => Some(ToolParamSchema::scalar_float()),
                    ToolDataKind::String | ToolDataKind::Unknown => Some(ToolParamSchema::string()),
                    _ => None,
                }
            }
        }
        (None, ToolDataKind::Bool) => Some(ToolParamSchema::bool()),
        (None, ToolDataKind::Number) => Some(ToolParamSchema::scalar_float()),
        (None, ToolDataKind::String)
        | (None, ToolDataKind::File)
        | (None, ToolDataKind::Text)
        | (None, ToolDataKind::Json)
        | (None, ToolDataKind::Unknown) => Some(ToolParamSchema::string()),
        (None, k) => dataset_from_kind(k).map(ToolParamSchema::input),
    }
}

fn fallback_param_description(name: &str) -> String {
    if name.trim().is_empty() {
        return String::new();
    }

    let mut out = String::new();
    let mut capitalize_next = true;
    for ch in name.chars() {
        if ch == '_' || ch == '-' {
            if !out.ends_with(' ') {
                out.push(' ');
            }
            capitalize_next = true;
            continue;
        }

        if capitalize_next {
            out.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            out.push(ch);
        }
    }

    out.trim().to_string()
}

pub fn manifest_with_param_schema_json(
    manifest: &ToolManifest,
    param_schemas: &BTreeMap<String, ToolParamSchema>,
) -> Value {
    let mut entry = serde_json::to_value(manifest).unwrap_or_else(|_| Value::Null);
    let Value::Object(obj) = &mut entry else {
        return serde_json::to_value(manifest).unwrap_or(Value::Null);
    };

    let params_val = obj
        .get("params")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));

    let mut params_in: Vec<Value> = match params_val {
        Value::Array(params) => params,
        _ => Vec::new(),
    };

    if params_in.is_empty() && !param_schemas.is_empty() {
        for name in param_schemas.keys() {
            params_in.push(serde_json::json!({
                "name": name,
                "description": fallback_param_description(name),
                "required": false
            }));
        }
    }

    let mut params_out = Vec::new();
    for p in params_in {
        let mut po = match p {
            Value::Object(v) => v,
            _ => continue,
        };

        let name = po
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let description = po
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        if description.trim().is_empty() {
            po.insert(
                "description".to_string(),
                Value::String(fallback_param_description(&name)),
            );
        }

        let explicit_schema = param_schemas.get(&name);
        let (role, data_kind) = if let Some(schema) = explicit_schema {
            role_and_kind_from_schema(schema)
        } else {
            let inferred_role = if looks_like_output_param(&name, &description) {
                ToolIoRole::Output
            } else {
                ToolIoRole::Input
            };
            let inferred_kind = infer_data_kind(&name, &description, &inferred_role);
            (Some(inferred_role), inferred_kind)
        };

        let synthesized_schema = schema_from_role_and_kind(role.clone(), data_kind.clone());
        if let Some(schema) = explicit_schema.or(synthesized_schema.as_ref()) {
            po.insert(
                "schema".to_string(),
                serde_json::to_value(schema).unwrap_or(Value::Null),
            );
        }

        if let Some(role) = role {
            po.insert(
                "io_role".to_string(),
                serde_json::to_value(role)
                    .unwrap_or_else(|_| Value::String("input".to_string())),
            );
        }

        po.insert(
            "data_kind".to_string(),
            serde_json::to_value(data_kind)
                .unwrap_or_else(|_| Value::String("unknown".to_string())),
        );

        params_out.push(Value::Object(po));
    }

    obj.insert("params".to_string(), Value::Array(params_out));
    entry
}

pub fn manifest_with_io_schema_json(manifest: &ToolManifest) -> Value {
    manifest_with_param_schema_json(manifest, &BTreeMap::new())
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
        let params = m.params.into_iter().map(ToolParamDescriptor::from).collect();

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
        let params = m.params.into_iter().map(ToolParamDescriptor::from).collect();

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
                            ..Default::default()
                    },
                    ToolParamSpec {
                        name: "constant",
                        description: "Added value",
                        required: true,
                            ..Default::default()
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
