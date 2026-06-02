//! Tests for external authoritative fixture ingestion.

use crate::{
    csrs_preferred_operation_support_snapshot,
    europe_phase1_preferred_operation_support_snapshot,
    us_phase1_preferred_operation_support_snapshot,
    CsrsPreferredOperationStatus,
    EuropePreferredOperationStatus,
    UsPreferredOperationStatus,
};
use std::collections::HashSet;

const NRCAN_TRX_FIXTURE: &str = include_str!("data/authoritative/nrcan_trx_nad83csrs_to_itrf2014_epoch2010_checkpoints.csv");
const NRCAN_EPOCH_PROPAGATION_FIXTURE: &str = include_str!("data/authoritative/nrcan_nad83csrs_epoch_propagation_2010_to_2020_checkpoints.csv");
const NRCAN_TRX_CSRS_2002_TO_2010_FIXTURE: &str =
    include_str!("data/authoritative/nrcan_trx_nad83csrs_epoch_2002_to_2010_guelph_vancouver_sample2.csv");
const OP10715_CSRS_V3_TO_V8_TEMPLATE: &str =
    include_str!("data/authoritative/op10715_csrs_v3_to_v8_checkpoints_template.csv");
const CSRS_V4_TO_V8_TEMPLATE: &str =
    include_str!("data/authoritative/csrs_v4_to_v8_checkpoints_template.csv");
const CSRS_V5_TO_V8_TEMPLATE: &str =
    include_str!("data/authoritative/csrs_v5_to_v8_checkpoints_template.csv");
const CSRS_V6_TO_V8_TEMPLATE: &str =
    include_str!("data/authoritative/csrs_v6_to_v8_checkpoints_template.csv");
const CSRS_V7_TO_V8_TEMPLATE: &str =
    include_str!("data/authoritative/csrs_v7_to_v8_checkpoints_template.csv");
const CSRS_V8_TO_V3_TEMPLATE: &str =
    include_str!("data/authoritative/csrs_v8_to_v3_checkpoints_template.csv");
const CSRS_V8_TO_V4_TEMPLATE: &str =
    include_str!("data/authoritative/csrs_v8_to_v4_checkpoints_template.csv");
const CSRS_V8_TO_V5_TEMPLATE: &str =
    include_str!("data/authoritative/csrs_v8_to_v5_checkpoints_template.csv");
const CSRS_V8_TO_V6_TEMPLATE: &str =
    include_str!("data/authoritative/csrs_v8_to_v6_checkpoints_template.csv");
const CSRS_V8_TO_V7_TEMPLATE: &str =
    include_str!("data/authoritative/csrs_v8_to_v7_checkpoints_template.csv");
const US_NSRS2007_TO_NAD83_2011_TEMPLATE: &str =
    include_str!("data/authoritative/us_nsrs2007_to_nad83_2011_checkpoints_template.csv");
const EUROPE_ETRS89_REALIZATION_TEMPLATE: &str =
    include_str!("data/authoritative/europe_etrs89_realization_checkpoints_template.csv");

#[derive(Debug)]
struct NrcanTrxCheckpoint {
    station: String,
    input_lat_deg: f64,
    input_lon_deg: f64,
    input_h_m: f64,
    output_lat_deg: f64,
    output_lon_deg: f64,
    output_h_m: f64,
    vphi_mm_per_yr: f64,
    vlambda_mm_per_yr: f64,
    vh_mm_per_yr: f64,
}

#[derive(Debug)]
struct NrcanEpochPropagationCheckpoint {
    station: String,
    input_lat_deg: f64,
    input_lon_positive_west_deg: f64,
    input_h_m: f64,
    origin_epoch_iso: String,
    destination_epoch_iso: String,
    output_lat_deg: f64,
    output_lon_positive_west_deg: f64,
    output_h_m: f64,
    vphi_mm_per_yr: f64,
    vlambda_mm_per_yr: f64,
    vh_mm_per_yr: f64,
}

#[derive(Debug)]
struct NrcanTrxCsrsCheckpoint {
    station: String,
    lat_deg: f64,
    lon_deg: f64,
    h_m: f64,
    vn_mm_per_yr: f64,
    ve_mm_per_yr: f64,
    vh_mm_per_yr: f64,
}

#[derive(Debug)]
struct CsrsPairTemplateCheckpoint {
    station: String,
    source_crs_epsg: u32,
    target_crs_epsg: u32,
    operation_code: Option<u32>,
    epoch_decimal_year: f64,
    input_x_m: f64,
    input_y_m: f64,
    input_z_m: f64,
    output_x_m: f64,
    output_y_m: f64,
    output_z_m: f64,
    source_reference: String,
}

fn parse_optional_u32(value: &str, field: &str) -> Result<Option<u32>, String> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    value
        .trim()
        .parse::<u32>()
        .map(Some)
        .map_err(|e| format!("failed parsing {field}='{value}': {e}"))
}

fn parse_u32(value: &str, field: &str) -> Result<u32, String> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|e| format!("failed parsing {field}='{value}': {e}"))
}

fn parse_f64(value: &str, field: &str) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .map_err(|e| format!("failed parsing {field}='{value}': {e}"))
}

fn parse_nrcan_trx_fixture(csv: &str) -> Result<Vec<NrcanTrxCheckpoint>, String> {
    let mut data_lines = Vec::new();
    let mut header_seen = false;

    for line in csv.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !header_seen {
            let expected = "station,input_lat_deg,input_lon_deg,input_h_m,output_lat_deg,output_lon_deg,output_h_m,vphi_mm_per_yr,vlambda_mm_per_yr,vh_mm_per_yr";
            if trimmed != expected {
                return Err(format!(
                    "unexpected header; expected '{expected}', got '{trimmed}'"
                ));
            }
            header_seen = true;
            continue;
        }
        data_lines.push(trimmed.to_string());
    }

    if !header_seen {
        return Err("missing CSV header".to_string());
    }
    if data_lines.is_empty() {
        return Err("no data rows in fixture".to_string());
    }

    let mut rows = Vec::with_capacity(data_lines.len());
    for (idx, line) in data_lines.iter().enumerate() {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() != 10 {
            return Err(format!(
                "row {} has {} columns, expected 10",
                idx + 1,
                cols.len()
            ));
        }

        let parse = |value: &str, field: &str| -> Result<f64, String> {
            value
                .parse::<f64>()
                .map_err(|e| format!("failed parsing {field}='{value}': {e}"))
        };

        rows.push(NrcanTrxCheckpoint {
            station: cols[0].to_string(),
            input_lat_deg: parse(cols[1], "input_lat_deg")?,
            input_lon_deg: parse(cols[2], "input_lon_deg")?,
            input_h_m: parse(cols[3], "input_h_m")?,
            output_lat_deg: parse(cols[4], "output_lat_deg")?,
            output_lon_deg: parse(cols[5], "output_lon_deg")?,
            output_h_m: parse(cols[6], "output_h_m")?,
            vphi_mm_per_yr: parse(cols[7], "vphi_mm_per_yr")?,
            vlambda_mm_per_yr: parse(cols[8], "vlambda_mm_per_yr")?,
            vh_mm_per_yr: parse(cols[9], "vh_mm_per_yr")?,
        });
    }

    Ok(rows)
}

fn parse_nrcan_epoch_propagation_fixture(
    csv: &str,
) -> Result<Vec<NrcanEpochPropagationCheckpoint>, String> {
    let mut data_lines = Vec::new();
    let mut header_seen = false;

    for line in csv.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !header_seen {
            let expected = "station,input_lat_deg,input_lon_positive_west_deg,input_h_m,origin_epoch_iso,destination_epoch_iso,output_lat_deg,output_lon_positive_west_deg,output_h_m,vphi_mm_per_yr,vlambda_mm_per_yr,vh_mm_per_yr";
            if trimmed != expected {
                return Err(format!(
                    "unexpected header; expected '{expected}', got '{trimmed}'"
                ));
            }
            header_seen = true;
            continue;
        }
        data_lines.push(trimmed.to_string());
    }

    if !header_seen {
        return Err("missing CSV header".to_string());
    }
    if data_lines.is_empty() {
        return Err("no data rows in fixture".to_string());
    }

    let mut rows = Vec::with_capacity(data_lines.len());
    for (idx, line) in data_lines.iter().enumerate() {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() != 12 {
            return Err(format!(
                "row {} has {} columns, expected 12",
                idx + 1,
                cols.len()
            ));
        }

        let parse = |value: &str, field: &str| -> Result<f64, String> {
            value
                .parse::<f64>()
                .map_err(|e| format!("failed parsing {field}='{value}': {e}"))
        };

        rows.push(NrcanEpochPropagationCheckpoint {
            station: cols[0].to_string(),
            input_lat_deg: parse(cols[1], "input_lat_deg")?,
            input_lon_positive_west_deg: parse(cols[2], "input_lon_positive_west_deg")?,
            input_h_m: parse(cols[3], "input_h_m")?,
            origin_epoch_iso: cols[4].to_string(),
            destination_epoch_iso: cols[5].to_string(),
            output_lat_deg: parse(cols[6], "output_lat_deg")?,
            output_lon_positive_west_deg: parse(cols[7], "output_lon_positive_west_deg")?,
            output_h_m: parse(cols[8], "output_h_m")?,
            vphi_mm_per_yr: parse(cols[9], "vphi_mm_per_yr")?,
            vlambda_mm_per_yr: parse(cols[10], "vlambda_mm_per_yr")?,
            vh_mm_per_yr: parse(cols[11], "vh_mm_per_yr")?,
        });
    }

    Ok(rows)
}

fn parse_nrcan_trx_csrs_fixture(csv: &str) -> Result<Vec<NrcanTrxCsrsCheckpoint>, String> {
    let mut data_lines = Vec::new();
    let mut header_seen = false;

    for line in csv.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !header_seen {
            let expected = "station,lat,lon,height,vn,ve,vh";
            if trimmed != expected {
                return Err(format!(
                    "unexpected header; expected '{expected}', got '{trimmed}'"
                ));
            }
            header_seen = true;
            continue;
        }
        data_lines.push(trimmed.to_string());
    }

    if !header_seen {
        return Err("missing CSV header".to_string());
    }
    if data_lines.is_empty() {
        return Err("no data rows in fixture".to_string());
    }

    let mut rows = Vec::with_capacity(data_lines.len());
    for (idx, line) in data_lines.iter().enumerate() {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() != 7 {
            return Err(format!(
                "row {} has {} columns, expected 7",
                idx + 1,
                cols.len()
            ));
        }

        rows.push(NrcanTrxCsrsCheckpoint {
            station: cols[0].trim().to_string(),
            lat_deg: parse_f64(cols[1], "lat")?,
            lon_deg: parse_f64(cols[2], "lon")?,
            h_m: parse_f64(cols[3], "height")?,
            vn_mm_per_yr: parse_f64(cols[4], "vn")?,
            ve_mm_per_yr: parse_f64(cols[5], "ve")?,
            vh_mm_per_yr: parse_f64(cols[6], "vh")?,
        });
    }

    Ok(rows)
}

fn parse_csrs_pair_template_fixture(csv: &str) -> Result<Vec<CsrsPairTemplateCheckpoint>, String> {
    let mut data_lines = Vec::new();
    let mut header_seen = false;
    let expected = "station,source_crs_epsg,target_crs_epsg,operation_code,epoch_decimal_year,input_x_m,input_y_m,input_z_m,output_x_m,output_y_m,output_z_m,source_reference";

    for line in csv.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !header_seen {
            if trimmed != expected {
                return Err(format!(
                    "unexpected header; expected '{expected}', got '{trimmed}'"
                ));
            }
            header_seen = true;
            continue;
        }
        data_lines.push(trimmed.to_string());
    }

    if !header_seen {
        return Err("missing CSV header".to_string());
    }

    // Empty templates are valid while corridor checkpoints are still pending.
    if data_lines.is_empty() {
        return Ok(Vec::new());
    }

    let mut rows = Vec::with_capacity(data_lines.len());
    for (idx, line) in data_lines.iter().enumerate() {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() != 12 {
            return Err(format!(
                "row {} has {} columns, expected 12",
                idx + 1,
                cols.len()
            ));
        }

        rows.push(CsrsPairTemplateCheckpoint {
            station: cols[0].trim().to_string(),
            source_crs_epsg: parse_u32(cols[1], "source_crs_epsg")?,
            target_crs_epsg: parse_u32(cols[2], "target_crs_epsg")?,
            operation_code: parse_optional_u32(cols[3], "operation_code")?,
            epoch_decimal_year: parse_f64(cols[4], "epoch_decimal_year")?,
            input_x_m: parse_f64(cols[5], "input_x_m")?,
            input_y_m: parse_f64(cols[6], "input_y_m")?,
            input_z_m: parse_f64(cols[7], "input_z_m")?,
            output_x_m: parse_f64(cols[8], "output_x_m")?,
            output_y_m: parse_f64(cols[9], "output_y_m")?,
            output_z_m: parse_f64(cols[10], "output_z_m")?,
            source_reference: cols[11].trim().to_string(),
        });
    }

    Ok(rows)
}

fn assert_phase1_metadata_conventions(row: &CsrsPairTemplateCheckpoint, region: &str) {
    assert!(
        (1980.0..=2100.0).contains(&row.epoch_decimal_year),
        "phase-1 {region} template row must have epoch_decimal_year within [1980, 2100]"
    );

    let source_reference = row.source_reference.trim();
    let source_reference_lower = source_reference.to_ascii_lowercase();
    let placeholder_tokens = [
        "tbd",
        "todo",
        "pending",
        "n/a",
        "na",
        "template",
        "placeholder",
        "fill-me",
        "fillme",
        "unknown",
    ];

    assert!(
        !source_reference.is_empty(),
        "phase-1 {region} template row must include source_reference"
    );
    assert!(
        !placeholder_tokens
            .iter()
            .any(|token| source_reference_lower == *token),
        "phase-1 {region} template row source_reference must not be a placeholder token"
    );
    assert!(
        source_reference.contains(':') || source_reference.contains('/'),
        "phase-1 {region} template row source_reference must include a namespaced code or URL"
    );
}

#[test]
fn nrcan_trx_authoritative_fixture_parses_and_is_well_formed() {
    let rows = parse_nrcan_trx_fixture(NRCAN_TRX_FIXTURE)
        .expect("authoritative TRX fixture should parse");

    assert!(
        rows.len() >= 2,
        "expected at least 2 authoritative checkpoints; got {}",
        rows.len()
    );

    for row in &rows {
        assert!(!row.station.trim().is_empty(), "station name should be non-empty");
        assert!((-90.0..=90.0).contains(&row.input_lat_deg));
        assert!((-180.0..=180.0).contains(&row.input_lon_deg));
        assert!((-90.0..=90.0).contains(&row.output_lat_deg));
        assert!((-180.0..=180.0).contains(&row.output_lon_deg));

        let values = [
            row.input_h_m,
            row.output_h_m,
            row.vphi_mm_per_yr,
            row.vlambda_mm_per_yr,
            row.vh_mm_per_yr,
        ];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");
    }
}

#[test]
fn nrcan_trx_authoritative_fixture_has_expected_station_rows() {
    let rows = parse_nrcan_trx_fixture(NRCAN_TRX_FIXTURE)
        .expect("authoritative TRX fixture should parse");

    let has_vancouver = rows.iter().any(|r| r.station == "vancouver");
    let has_sample2 = rows.iter().any(|r| r.station == "sample2");

    assert!(has_vancouver, "fixture must include vancouver row");
    assert!(has_sample2, "fixture must include sample2 row");
}

#[test]
fn nrcan_epoch_propagation_fixture_parses_and_is_well_formed() {
    let rows = parse_nrcan_epoch_propagation_fixture(NRCAN_EPOCH_PROPAGATION_FIXTURE)
        .expect("authoritative epoch-propagation fixture should parse");

    assert_eq!(rows.len(), 3, "expected 3 authoritative checkpoints");

    for row in &rows {
        assert!(!row.station.trim().is_empty(), "station name should be non-empty");
        assert!((-90.0..=90.0).contains(&row.input_lat_deg));
        assert!((0.0..=180.0).contains(&row.input_lon_positive_west_deg));
        assert!((-90.0..=90.0).contains(&row.output_lat_deg));
        assert!((0.0..=180.0).contains(&row.output_lon_positive_west_deg));
        assert_eq!(row.origin_epoch_iso, "2010-01-01");
        assert_eq!(row.destination_epoch_iso, "2020-01-01");

        let values = [
            row.input_h_m,
            row.output_h_m,
            row.vphi_mm_per_yr,
            row.vlambda_mm_per_yr,
            row.vh_mm_per_yr,
        ];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");

        let moved_horizontally = (row.output_lat_deg - row.input_lat_deg).abs() > 0.0
            || (row.output_lon_positive_west_deg - row.input_lon_positive_west_deg).abs() > 0.0;
        let moved_vertically = (row.output_h_m - row.input_h_m).abs() > 0.0;
        assert!(moved_horizontally || moved_vertically, "epoch propagation should change at least one coordinate component");
    }
}

#[test]
fn nrcan_epoch_propagation_fixture_has_expected_station_rows() {
    let rows = parse_nrcan_epoch_propagation_fixture(NRCAN_EPOCH_PROPAGATION_FIXTURE)
        .expect("authoritative epoch-propagation fixture should parse");

    let has_waterloo = rows.iter().any(|r| r.station == "waterloo");
    let has_vancouver = rows.iter().any(|r| r.station == "vancouver");
    let has_sample2 = rows.iter().any(|r| r.station == "sample2");

    assert!(has_waterloo, "fixture must include waterloo row");
    assert!(has_vancouver, "fixture must include vancouver row");
    assert!(has_sample2, "fixture must include sample2 row");
}

#[test]
fn nrcan_trx_csrs_2002_to_2010_fixture_parses_and_is_well_formed() {
    let rows = parse_nrcan_trx_csrs_fixture(NRCAN_TRX_CSRS_2002_TO_2010_FIXTURE)
        .expect("authoritative TRX CSRS fixture should parse");

    assert_eq!(rows.len(), 3, "expected 3 authoritative checkpoints");

    for row in &rows {
        assert!(!row.station.trim().is_empty(), "station name should be non-empty");
        assert!((-90.0..=90.0).contains(&row.lat_deg));
        assert!((-180.0..=180.0).contains(&row.lon_deg));
        let values = [row.h_m, row.vn_mm_per_yr, row.ve_mm_per_yr, row.vh_mm_per_yr];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");
    }
}

#[test]
fn nrcan_trx_csrs_2002_to_2010_fixture_has_expected_station_rows() {
    let rows = parse_nrcan_trx_csrs_fixture(NRCAN_TRX_CSRS_2002_TO_2010_FIXTURE)
        .expect("authoritative TRX CSRS fixture should parse");

    let has_guelph = rows.iter().any(|r| r.station == "guelph");
    let has_vancouver = rows.iter().any(|r| r.station == "vancouver");
    let has_sample2 = rows.iter().any(|r| r.station == "sample2");

    assert!(has_guelph, "fixture must include guelph row");
    assert!(has_vancouver, "fixture must include vancouver row");
    assert!(has_sample2, "fixture must include sample2 row");
}

#[test]
fn op10715_csrs_v3_to_v8_template_fixture_is_parseable() {
    let rows = parse_csrs_pair_template_fixture(OP10715_CSRS_V3_TO_V8_TEMPLATE)
        .expect("op10715 template fixture should parse");

    for row in &rows {
        assert!(!row.station.is_empty(), "station should be non-empty");
        assert!(row.source_crs_epsg >= 22307 && row.source_crs_epsg <= 22324);
        assert!(row.target_crs_epsg >= 22807 && row.target_crs_epsg <= 22824);
        assert!(
            (row.source_crs_epsg - 22300) == (row.target_crs_epsg - 22800),
            "source/target zones should match"
        );
        if let Some(code) = row.operation_code {
            assert!(code > 0, "operation_code must be positive when supplied");
        }
        let values = [
            row.epoch_decimal_year,
            row.input_x_m,
            row.input_y_m,
            row.input_z_m,
            row.output_x_m,
            row.output_y_m,
            row.output_z_m,
        ];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");
        assert!(
            !row.source_reference.is_empty(),
            "source_reference should be non-empty"
        );
    }
}

#[test]
fn csrs_v4_to_v8_template_fixture_is_parseable() {
    let rows = parse_csrs_pair_template_fixture(CSRS_V4_TO_V8_TEMPLATE)
        .expect("v4->v8 template fixture should parse");

    for row in &rows {
        assert!(!row.station.is_empty(), "station should be non-empty");
        assert!(row.source_crs_epsg >= 22407 && row.source_crs_epsg <= 22424);
        assert!(row.target_crs_epsg >= 22807 && row.target_crs_epsg <= 22824);
        assert!(
            (row.source_crs_epsg - 22400) == (row.target_crs_epsg - 22800),
            "source/target zones should match"
        );
        if let Some(code) = row.operation_code {
            assert!(code > 0, "operation_code must be positive when supplied");
        }
        let values = [
            row.epoch_decimal_year,
            row.input_x_m,
            row.input_y_m,
            row.input_z_m,
            row.output_x_m,
            row.output_y_m,
            row.output_z_m,
        ];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");
        assert!(
            !row.source_reference.is_empty(),
            "source_reference should be non-empty"
        );
    }
}

#[test]
fn csrs_v5_to_v8_template_fixture_is_parseable() {
    let rows = parse_csrs_pair_template_fixture(CSRS_V5_TO_V8_TEMPLATE)
        .expect("v5->v8 template fixture should parse");

    for row in &rows {
        assert!(!row.station.is_empty(), "station should be non-empty");
        assert!(row.source_crs_epsg >= 22507 && row.source_crs_epsg <= 22524);
        assert!(row.target_crs_epsg >= 22807 && row.target_crs_epsg <= 22824);
        assert!(
            (row.source_crs_epsg - 22500) == (row.target_crs_epsg - 22800),
            "source/target zones should match"
        );
        if let Some(code) = row.operation_code {
            assert!(code > 0, "operation_code must be positive when supplied");
        }
        let values = [
            row.epoch_decimal_year,
            row.input_x_m,
            row.input_y_m,
            row.input_z_m,
            row.output_x_m,
            row.output_y_m,
            row.output_z_m,
        ];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");
        assert!(
            !row.source_reference.is_empty(),
            "source_reference should be non-empty"
        );
    }
}

#[test]
fn csrs_v6_to_v8_template_fixture_is_parseable() {
    let rows = parse_csrs_pair_template_fixture(CSRS_V6_TO_V8_TEMPLATE)
        .expect("v6->v8 template fixture should parse");

    for row in &rows {
        assert!(!row.station.is_empty(), "station should be non-empty");
        assert!(row.source_crs_epsg >= 22607 && row.source_crs_epsg <= 22624);
        assert!(row.target_crs_epsg >= 22807 && row.target_crs_epsg <= 22824);
        assert!(
            (row.source_crs_epsg - 22600) == (row.target_crs_epsg - 22800),
            "source/target zones should match"
        );
        if let Some(code) = row.operation_code {
            assert!(code > 0, "operation_code must be positive when supplied");
        }
        let values = [
            row.epoch_decimal_year,
            row.input_x_m,
            row.input_y_m,
            row.input_z_m,
            row.output_x_m,
            row.output_y_m,
            row.output_z_m,
        ];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");
        assert!(
            !row.source_reference.is_empty(),
            "source_reference should be non-empty"
        );
    }
}

#[test]
fn csrs_v7_to_v8_template_fixture_is_parseable() {
    let rows = parse_csrs_pair_template_fixture(CSRS_V7_TO_V8_TEMPLATE)
        .expect("v7->v8 template fixture should parse");

    for row in &rows {
        assert!(!row.station.is_empty(), "station should be non-empty");
        assert!(row.source_crs_epsg >= 22707 && row.source_crs_epsg <= 22724);
        assert!(row.target_crs_epsg >= 22807 && row.target_crs_epsg <= 22824);
        assert!(
            (row.source_crs_epsg - 22700) == (row.target_crs_epsg - 22800),
            "source/target zones should match"
        );
        if let Some(code) = row.operation_code {
            assert!(code > 0, "operation_code must be positive when supplied");
        }
        let values = [
            row.epoch_decimal_year,
            row.input_x_m,
            row.input_y_m,
            row.input_z_m,
            row.output_x_m,
            row.output_y_m,
            row.output_z_m,
        ];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");
        assert!(
            !row.source_reference.is_empty(),
            "source_reference should be non-empty"
        );
    }
}

#[test]
fn csrs_v8_to_v4_template_fixture_is_parseable() {
    let rows = parse_csrs_pair_template_fixture(CSRS_V8_TO_V4_TEMPLATE)
        .expect("v8->v4 template fixture should parse");

    for row in &rows {
        assert!(!row.station.is_empty(), "station should be non-empty");
        assert!(row.source_crs_epsg >= 22807 && row.source_crs_epsg <= 22824);
        assert!(row.target_crs_epsg >= 22407 && row.target_crs_epsg <= 22424);
        assert!(
            (row.source_crs_epsg - 22800) == (row.target_crs_epsg - 22400),
            "source/target zones should match"
        );
        if let Some(code) = row.operation_code {
            assert!(code > 0, "operation_code must be positive when supplied");
        }
        let values = [
            row.epoch_decimal_year,
            row.input_x_m,
            row.input_y_m,
            row.input_z_m,
            row.output_x_m,
            row.output_y_m,
            row.output_z_m,
        ];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");
        assert!(
            !row.source_reference.is_empty(),
            "source_reference should be non-empty"
        );
    }
}

#[test]
fn csrs_v8_to_v3_template_fixture_is_parseable() {
    let rows = parse_csrs_pair_template_fixture(CSRS_V8_TO_V3_TEMPLATE)
        .expect("v8->v3 template fixture should parse");

    for row in &rows {
        assert!(!row.station.is_empty(), "station should be non-empty");
        assert!(row.source_crs_epsg >= 22807 && row.source_crs_epsg <= 22824);
        assert!(row.target_crs_epsg >= 22307 && row.target_crs_epsg <= 22324);
        assert!(
            (row.source_crs_epsg - 22800) == (row.target_crs_epsg - 22300),
            "source/target zones should match"
        );
        if let Some(code) = row.operation_code {
            assert!(code > 0, "operation_code must be positive when supplied");
        }
        let values = [
            row.epoch_decimal_year,
            row.input_x_m,
            row.input_y_m,
            row.input_z_m,
            row.output_x_m,
            row.output_y_m,
            row.output_z_m,
        ];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");
        assert!(
            !row.source_reference.is_empty(),
            "source_reference should be non-empty"
        );
    }
}

#[test]
fn csrs_v8_to_v6_template_fixture_is_parseable() {
    let rows = parse_csrs_pair_template_fixture(CSRS_V8_TO_V6_TEMPLATE)
        .expect("v8->v6 template fixture should parse");

    for row in &rows {
        assert!(!row.station.is_empty(), "station should be non-empty");
        assert!(row.source_crs_epsg >= 22807 && row.source_crs_epsg <= 22824);
        assert!(row.target_crs_epsg >= 22607 && row.target_crs_epsg <= 22624);
        assert!(
            (row.source_crs_epsg - 22800) == (row.target_crs_epsg - 22600),
            "source/target zones should match"
        );
        if let Some(code) = row.operation_code {
            assert!(code > 0, "operation_code must be positive when supplied");
        }
        let values = [
            row.epoch_decimal_year,
            row.input_x_m,
            row.input_y_m,
            row.input_z_m,
            row.output_x_m,
            row.output_y_m,
            row.output_z_m,
        ];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");
        assert!(
            !row.source_reference.is_empty(),
            "source_reference should be non-empty"
        );
    }
}

#[test]
fn csrs_v8_to_v7_template_fixture_is_parseable() {
    let rows = parse_csrs_pair_template_fixture(CSRS_V8_TO_V7_TEMPLATE)
        .expect("v8->v7 template fixture should parse");

    for row in &rows {
        assert!(!row.station.is_empty(), "station should be non-empty");
        assert!(row.source_crs_epsg >= 22807 && row.source_crs_epsg <= 22824);
        assert!(row.target_crs_epsg >= 22707 && row.target_crs_epsg <= 22724);
        assert!(
            (row.source_crs_epsg - 22800) == (row.target_crs_epsg - 22700),
            "source/target zones should match"
        );
        if let Some(code) = row.operation_code {
            assert!(code > 0, "operation_code must be positive when supplied");
        }
        let values = [
            row.epoch_decimal_year,
            row.input_x_m,
            row.input_y_m,
            row.input_z_m,
            row.output_x_m,
            row.output_y_m,
            row.output_z_m,
        ];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");
        assert!(
            !row.source_reference.is_empty(),
            "source_reference should be non-empty"
        );
    }
}

#[test]
fn csrs_v8_to_v5_template_fixture_is_parseable() {
    let rows = parse_csrs_pair_template_fixture(CSRS_V8_TO_V5_TEMPLATE)
        .expect("v8->v5 template fixture should parse");

    for row in &rows {
        assert!(!row.station.is_empty(), "station should be non-empty");
        assert!(row.source_crs_epsg >= 22807 && row.source_crs_epsg <= 22824);
        assert!(row.target_crs_epsg >= 22507 && row.target_crs_epsg <= 22524);
        assert!(
            (row.source_crs_epsg - 22800) == (row.target_crs_epsg - 22500),
            "source/target zones should match"
        );
        if let Some(code) = row.operation_code {
            assert!(code > 0, "operation_code must be positive when supplied");
        }
        let values = [
            row.epoch_decimal_year,
            row.input_x_m,
            row.input_y_m,
            row.input_z_m,
            row.output_x_m,
            row.output_y_m,
            row.output_z_m,
        ];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");
        assert!(
            !row.source_reference.is_empty(),
            "source_reference should be non-empty"
        );
    }
}

#[test]
fn us_nsrs2007_to_nad83_2011_template_fixture_is_parseable() {
    let rows = parse_csrs_pair_template_fixture(US_NSRS2007_TO_NAD83_2011_TEMPLATE)
        .expect("US NSRS2007->NAD83(2011) template fixture should parse");

    for row in &rows {
        assert!(!row.station.is_empty(), "station should be non-empty");
        assert!(
            row.source_crs_epsg >= 3465 && row.source_crs_epsg <= 3751,
            "expected source EPSG in NSRS2007 state-plane family"
        );
        assert!(
            row.target_crs_epsg >= 6355 && row.target_crs_epsg <= 6613,
            "expected target EPSG in NAD83(2011) state-plane family"
        );
        if let Some(code) = row.operation_code {
            assert!(code > 0, "operation_code must be positive when supplied");
        }
        let values = [
            row.epoch_decimal_year,
            row.input_x_m,
            row.input_y_m,
            row.input_z_m,
            row.output_x_m,
            row.output_y_m,
            row.output_z_m,
        ];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");
        assert!(
            !row.source_reference.is_empty(),
            "source_reference should be non-empty"
        );

        // Filled rows must point to resolvable CRS entries and expected families.
        let src = crate::from_epsg(row.source_crs_epsg).expect("source EPSG should resolve");
        let dst = crate::from_epsg(row.target_crs_epsg).expect("target EPSG should resolve");
        assert!(
            src.name.contains("NSRS2007"),
            "source CRS name should indicate NSRS2007 family"
        );
        assert!(
            dst.name.contains("2011"),
            "target CRS name should indicate NAD83(2011) family"
        );
    }
}

#[test]
fn europe_etrs89_realization_template_fixture_is_parseable() {
    let rows = parse_csrs_pair_template_fixture(EUROPE_ETRS89_REALIZATION_TEMPLATE)
        .expect("Europe ETRS89 realization template fixture should parse");

    for row in &rows {
        assert!(!row.station.is_empty(), "station should be non-empty");
        if let Some(code) = row.operation_code {
            assert!(code > 0, "operation_code must be positive when supplied");
        }
        let values = [
            row.epoch_decimal_year,
            row.input_x_m,
            row.input_y_m,
            row.input_z_m,
            row.output_x_m,
            row.output_y_m,
            row.output_z_m,
        ];
        assert!(values.iter().all(|v| v.is_finite()), "all numeric fields must be finite");
        assert!(
            !row.source_reference.is_empty(),
            "source_reference should be non-empty"
        );

        // Filled rows should stay within ETRS89-centered realization corridors.
        let src = crate::from_epsg(row.source_crs_epsg).expect("source EPSG should resolve");
        let dst = crate::from_epsg(row.target_crs_epsg).expect("target EPSG should resolve");
        assert_eq!(src.datum.name, "ETRS 89", "source datum should be ETRS89 family");
        assert_eq!(dst.datum.name, "ETRS 89", "target datum should be ETRS89 family");
    }
}

#[test]
fn us_nsrs2007_to_nad83_2011_template_phase1_pairs_are_allowlisted() {
    let rows = parse_csrs_pair_template_fixture(US_NSRS2007_TO_NAD83_2011_TEMPLATE)
        .expect("US NSRS2007->NAD83(2011) template fixture should parse");

    // Phase-1 US seed corridors for first authoritative captures.
    // Reverse directions are now allowlisted as active bidirectional corridors.
    let allowlist: HashSet<(u32, u32)> = [
        (3582u32, 6487u32),
        (6487u32, 3582u32),
        (3600u32, 6568u32),
        (6568u32, 3600u32),
    ]
    .into_iter()
    .collect();

    for row in &rows {
        assert!(
            allowlist.contains(&(row.source_crs_epsg, row.target_crs_epsg)),
            "phase-1 US template row must use an allowlisted corridor"
        );
        assert!(
            row.operation_code.is_some(),
            "phase-1 US template rows must include operation_code"
        );
        assert_phase1_metadata_conventions(row, "US");
    }
}

#[test]
fn europe_etrs89_realization_template_phase1_pairs_are_allowlisted() {
    let rows = parse_csrs_pair_template_fixture(EUROPE_ETRS89_REALIZATION_TEMPLATE)
        .expect("Europe ETRS89 realization template fixture should parse");

    // Phase-1 Europe seed corridors for first authoritative captures.
    // Reverse directions are now allowlisted as active bidirectional corridors.
    let allowlist: HashSet<(u32, u32)> = [
        (4258u32, 4258u32),
        (25801u32, 3035u32),
        (25832u32, 3035u32),
        (3035u32, 25801u32),
        (3035u32, 25832u32),
    ]
    .into_iter()
    .collect();

    for row in &rows {
        assert!(
            allowlist.contains(&(row.source_crs_epsg, row.target_crs_epsg)),
            "phase-1 Europe template row must use an allowlisted corridor"
        );
        assert!(
            row.operation_code.is_some(),
            "phase-1 Europe template rows must include operation_code"
        );
        assert_phase1_metadata_conventions(row, "Europe");
    }
}

#[test]
fn csrs_template_fixture_scope_matches_current_corridor_policy() {
    let snapshot = csrs_preferred_operation_support_snapshot();
    let expected_pairs = [
        ("v3", "v8", CsrsPreferredOperationStatus::Active, Some(10715)),
        ("v4", "v8", CsrsPreferredOperationStatus::Active, Some(10715)),
        ("v5", "v8", CsrsPreferredOperationStatus::Active, Some(10715)),
        ("v6", "v8", CsrsPreferredOperationStatus::Active, Some(10715)),
        ("v7", "v8", CsrsPreferredOperationStatus::Active, Some(10715)),
        ("v8", "v3", CsrsPreferredOperationStatus::Active, Some(10715)),
        ("v8", "v4", CsrsPreferredOperationStatus::Active, Some(10715)),
        ("v8", "v5", CsrsPreferredOperationStatus::Active, Some(10715)),
        ("v8", "v6", CsrsPreferredOperationStatus::Active, Some(10715)),
        ("v8", "v7", CsrsPreferredOperationStatus::Active, Some(10715)),
    ];

    for (src, dst, expected_status, expected_code) in expected_pairs {
        let pair = snapshot
            .pairs
            .iter()
            .find(|p| p.source_realization == src && p.target_realization == dst)
            .expect("expected corridor pair to be present in support snapshot");
        assert_eq!(pair.status, expected_status);
        assert_eq!(pair.preferred_operation_code, expected_code);
        assert_eq!(pair.zone_min, snapshot.zone_min);
        assert_eq!(pair.zone_max, snapshot.zone_max);
    }
}

#[test]
fn csrs_template_inventory_covers_active_and_pending_corridors() {
    let template_pairs: HashSet<(&str, &str)> = [
        ("v3", "v8"),
        ("v4", "v8"),
        ("v5", "v8"),
        ("v6", "v8"),
        ("v7", "v8"),
        ("v8", "v3"),
        ("v8", "v4"),
        ("v8", "v5"),
        ("v8", "v6"),
        ("v8", "v7"),
    ]
    .into_iter()
    .collect();

    let snapshot = csrs_preferred_operation_support_snapshot();
    for (src, dst) in template_pairs {
        let pair = snapshot
            .pairs
            .iter()
            .find(|p| p.source_realization == src && p.target_realization == dst)
            .expect("template pair should exist in support snapshot");
        assert_eq!(
            pair.status,
            CsrsPreferredOperationStatus::Active,
            "template pair should be active under current broad CSRS policy"
        );
        assert_eq!(pair.preferred_operation_code, Some(10715));
    }
}

#[test]
fn us_phase1_snapshot_includes_reverse_seed_corridors_as_active() {
    let snapshot = us_phase1_preferred_operation_support_snapshot();
    let expected_pairs = [
        (3582u32, 6487u32),
        (6487u32, 3582u32),
        (3600u32, 6568u32),
        (6568u32, 3600u32),
    ];

    for (src, dst) in expected_pairs {
        let pair = snapshot
            .pairs
            .iter()
            .find(|p| p.source_crs_epsg == src && p.target_crs_epsg == dst)
            .expect("expected US phase-1 seed corridor to be present");
        assert_eq!(pair.status, UsPreferredOperationStatus::Active);
    }
}

#[test]
fn europe_phase1_snapshot_includes_reverse_seed_corridors_as_active() {
    let snapshot = europe_phase1_preferred_operation_support_snapshot();
    let expected_pairs = [
        (4258u32, 4258u32),
        (25801u32, 3035u32),
        (25832u32, 3035u32),
        (3035u32, 25801u32),
        (3035u32, 25832u32),
    ];

    for (src, dst) in expected_pairs {
        let pair = snapshot
            .pairs
            .iter()
            .find(|p| p.source_crs_epsg == src && p.target_crs_epsg == dst)
            .expect("expected Europe phase-1 seed corridor to be present");
        assert_eq!(pair.status, EuropePreferredOperationStatus::Active);
    }
}
