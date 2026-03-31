use std::collections::BTreeMap;

use serde_json::Value;
use wbcore::{CapabilityProvider, LicenseTier, ProgressSink, ToolArgs, ToolContext};
use wbtools_oss::{register_default_tools, ToolRegistry};

struct DemoCapabilities;

impl CapabilityProvider for DemoCapabilities {
    fn has_tool_access(&self, _tool_id: &'static str, tier: LicenseTier) -> bool {
        matches!(tier, LicenseTier::Open)
    }
}

struct StdoutProgress;

impl ProgressSink for StdoutProgress {
    fn info(&self, msg: &str) {
        println!("[info] {msg}");
    }

    fn progress(&self, pct: f64) {
        println!("[progress] {:.0}%", pct * 100.0);
    }
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  cargo run -p wbtools_oss --example run_tool -- list");
    eprintln!(
        "  cargo run -p wbtools_oss --example run_tool -- run <tool_id> '<json-args>'"
    );
    eprintln!("Example:");
    eprintln!(
        "  cargo run -p wbtools_oss --example run_tool -- run add '{{\"input1\":\"a.tif\",\"input2\":\"b.tif\",\"output\":\"sum.tif\"}}'"
    );
}

fn parse_args(json_text: &str) -> Result<ToolArgs, String> {
    let value: Value = serde_json::from_str(json_text)
        .map_err(|e| format!("invalid JSON arguments: {e}"))?;

    let map = value
        .as_object()
        .ok_or_else(|| "arguments must be a JSON object".to_string())?;

    let mut out = BTreeMap::new();
    for (k, v) in map {
        out.insert(k.clone(), v.clone());
    }
    Ok(out)
}

fn main() {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(2);
    }

    match args[1].as_str() {
        "list" => {
            for m in registry.list() {
                println!("{} - {}", m.id, m.display_name);
            }
        }
        "run" => {
            if args.len() < 4 {
                print_usage();
                std::process::exit(2);
            }

            let tool_id = &args[2];
            let tool_args = match parse_args(&args[3]) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(2);
                }
            };

            let caps = DemoCapabilities;
            let progress = StdoutProgress;
            let ctx = ToolContext {
                progress: &progress,
                capabilities: &caps,
            };

            match registry.run(tool_id, &tool_args, &ctx) {
                Ok(result) => match serde_json::to_string_pretty(&result.outputs) {
                    Ok(txt) => println!("{txt}"),
                    Err(e) => {
                        eprintln!("failed to serialize outputs: {e}");
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("run failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            print_usage();
            std::process::exit(2);
        }
    }
}
