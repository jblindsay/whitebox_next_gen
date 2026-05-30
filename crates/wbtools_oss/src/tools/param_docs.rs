use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde::Deserialize;
use wbcore::{ToolDatasetSchema, ToolParamSchema, ToolVectorGeometry};

#[derive(Debug, Clone, Deserialize)]
struct ParamDoc {
    name: String,
    #[serde(rename = "type")]
    param_type: String,
    required: bool,
    description: String,
}

type ToolDocMap = BTreeMap<String, Vec<ParamDoc>>;

fn docs() -> &'static ToolDocMap {
    static DOCS: OnceLock<ToolDocMap> = OnceLock::new();
    DOCS.get_or_init(|| {
        serde_json::from_str(include_str!("generated_param_docs.json"))
            .unwrap_or_else(|_| BTreeMap::new())
    })
}

fn parse_literal_options(param_type: &str) -> Option<Vec<String>> {
    let lower = param_type.to_ascii_lowercase();
    let start = lower.find("literal[")? + "literal[".len();
    let end = lower.rfind(']')?;
    if end <= start {
        return None;
    }
    let inner = &param_type[start..end];
    let mut out = Vec::new();
    for chunk in inner.split(',') {
        let v = chunk.trim().trim_matches('"').trim_matches('\'');
        if !v.is_empty() {
            out.push(v.to_string());
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn detect_dataset(text: &str) -> Option<ToolDatasetSchema> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("raster") {
        return Some(ToolDatasetSchema::Raster);
    }
    if lower.contains("vector") {
        return Some(ToolDatasetSchema::Vector {
            geometry: ToolVectorGeometry::Any,
        });
    }
    if lower.contains("lidar") || lower.contains("las") || lower.contains("laz") {
        return Some(ToolDatasetSchema::Lidar);
    }
    if lower.contains("table") || lower.contains("csv") {
        return Some(ToolDatasetSchema::Table);
    }
    if lower.contains("json") {
        return Some(ToolDatasetSchema::Json);
    }
    if lower.contains("text") || lower.contains("txt") {
        return Some(ToolDatasetSchema::Text);
    }
    if lower.contains("file") || lower.contains("path") {
        return Some(ToolDatasetSchema::File);
    }
    None
}

fn output_like(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == "output"
        || n == "output_path"
        || n == "output_file"
        || n.starts_with("output_")
        || n.ends_with("_output")
}

fn normalize_param_name(name: &str, known_keys: &BTreeSet<String>) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed == "callback" {
        return None;
    }

    if known_keys.contains(trimmed) {
        return Some(trimmed.to_string());
    }

    let mapped = match trimmed {
        "output_path" => Some("output".to_string()),
        "d8_pointer" if known_keys.contains("d8_pntr") => Some("d8_pntr".to_string()),
        "pour_points" if known_keys.contains("pour_pts") => Some("pour_pts".to_string()),
        _ => {
            if let Some(base) = trimmed.strip_suffix("_output_path") {
                Some(format!("{base}_output"))
            } else if let Some(base) = trimmed.strip_suffix("_path") {
                if known_keys.contains(base) {
                    Some(base.to_string())
                } else if known_keys.contains(&format!("{base}_output")) {
                    Some(format!("{base}_output"))
                } else {
                    Some(base.to_string())
                }
            } else {
                None
            }
        }
    };

    if let Some(candidate) = mapped {
        return Some(candidate);
    }

    Some(trimmed.to_string())
}

fn schema_from_doc(doc: &ParamDoc) -> Option<ToolParamSchema> {
    let ty = doc.param_type.to_ascii_lowercase();

    if let Some(options) = parse_literal_options(&doc.param_type) {
        let refs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
        return Some(ToolParamSchema::enum_values(&refs));
    }

    if ty.contains("bool") {
        return Some(ToolParamSchema::bool());
    }
    if ty.contains("int") {
        return Some(ToolParamSchema::scalar_integer());
    }
    if ty.contains("float") || ty.contains("double") || ty.contains("number") {
        return Some(ToolParamSchema::scalar_float());
    }

    let dataset = detect_dataset(&doc.param_type)
        .or_else(|| detect_dataset(&doc.description))
        .or_else(|| detect_dataset(&doc.name));

    if let Some(ds) = dataset {
        if output_like(&doc.name) {
            return Some(ToolParamSchema::output(ds));
        }

        if ty.contains("list") || ty.contains("sequence") || ty.contains("tuple") {
            return Some(ToolParamSchema::input_multiple(ds));
        }

        return Some(ToolParamSchema::input(ds));
    }

    if ty.contains("str") || ty.contains("string") || ty.contains("path") {
        return Some(ToolParamSchema::string());
    }

    None
}

pub fn doc_tool_param_schemas(tool_id: &str) -> Option<BTreeMap<String, ToolParamSchema>> {
    let params = docs().get(tool_id)?;
    let mut out = BTreeMap::new();
    let known_keys = BTreeSet::new();
    for p in params {
        let Some(name) = normalize_param_name(&p.name, &known_keys) else {
            continue;
        };
        if let Some(schema) = schema_from_doc(p) {
            out.insert(name, schema);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

pub fn doc_tool_param_descriptions(
    tool_id: &str,
    known_keys: &BTreeSet<String>,
) -> Option<BTreeMap<String, String>> {
    let params = docs().get(tool_id)?;
    let mut out = BTreeMap::new();
    for p in params {
        let Some(name) = normalize_param_name(&p.name, known_keys) else {
            continue;
        };
        let desc = p.description.trim();
        if !desc.is_empty() {
            out.insert(name, desc.to_string());
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

pub fn doc_tool_param_required(
    tool_id: &str,
    known_keys: &BTreeSet<String>,
) -> Option<BTreeMap<String, bool>> {
    let params = docs().get(tool_id)?;
    let mut out = BTreeMap::new();
    for p in params {
        let Some(name) = normalize_param_name(&p.name, known_keys) else {
            continue;
        };
        out.insert(name, p.required);
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}
