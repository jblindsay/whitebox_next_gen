use std::path::Path;

use wbprojection::{
    EpsgIdentifyPolicy,
    identify_epsg_from_wkt_report,
};

fn load_wkt_from_arg(arg: &str) -> Result<String, String> {
    if Path::new(arg).exists() {
        std::fs::read_to_string(arg)
            .map_err(|e| format!("failed to read '{arg}': {e}"))
    } else {
        Ok(arg.to_string())
    }
}

fn main() {
    let mut args = std::env::args();
    let _bin = args.next();

    let Some(input) = args.next() else {
        eprintln!("Usage: cargo run --example epsg_identify_report -- '<WKT or path-to-.prj/.wkt>' [strict]");
        std::process::exit(2);
    };

    let strict = args
        .next()
        .map(|s| s.eq_ignore_ascii_case("strict"))
        .unwrap_or(false);

    let policy = if strict {
        EpsgIdentifyPolicy::Strict
    } else {
        EpsgIdentifyPolicy::Lenient
    };

    let wkt = match load_wkt_from_arg(&input) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(1);
        }
    };

    match identify_epsg_from_wkt_report(&wkt, policy) {
        Some(report) => {
            println!("policy={policy:?}");
            println!("resolved_code={:?}", report.resolved_code);
            println!("passed_threshold={}", report.passed_threshold);
            println!("ambiguous={}", report.ambiguous);
            println!("used_embedded_epsg={}", report.used_embedded_epsg);
            println!("top_candidates={}", report.top_candidates.len());
            println!("code,total,kind,datum,ellipsoid,zone,params,name");
            for c in report.top_candidates {
                println!(
                    "{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}",
                    c.code,
                    c.total_score,
                    c.kind_score,
                    c.datum_score,
                    c.ellipsoid_score,
                    c.zone_score,
                    c.parameter_score,
                    c.name_score
                );
            }
        }
        None => {
            println!("no report (input could not be parsed as a CRS)");
            std::process::exit(3);
        }
    }
}
