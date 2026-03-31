//! CRS metadata helpers for LiDAR datasets.
//!
//! This module stores lightweight CRS metadata (`epsg`, `wkt`) and contains
//! helper parsers used by LAS/COPC metadata adapters.

use wbprojection::{
    EpsgIdentifyPolicy,
    epsg_from_srs_reference as wb_epsg_from_srs_reference,
    identify_epsg_from_wkt_with_policy as wb_identify_epsg_from_wkt_with_policy,
    to_ogc_wkt,
};

/// Coordinate reference system metadata attached to a LiDAR source.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Crs {
    /// EPSG code (when known).
    pub epsg: Option<u32>,
    /// OGC WKT text (when known).
    pub wkt: Option<String>,
}

impl Crs {
    /// Create an empty CRS metadata object.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create CRS metadata from an EPSG code.
    pub fn from_epsg(epsg: u32) -> Self {
        Self { epsg: Some(epsg), wkt: ogc_wkt_from_epsg(epsg) }
    }

    /// Add/override EPSG code.
    pub fn with_epsg(mut self, epsg: u32) -> Self {
        self.epsg = Some(epsg);
        self
    }

    /// Add/override WKT text.
    pub fn with_wkt(mut self, wkt: impl Into<String>) -> Self {
        self.wkt = Some(wkt.into());
        self
    }
}

/// Resolve EPSG → OGC WKT using `wbprojection` tables.
pub fn ogc_wkt_from_epsg(epsg: u32) -> Option<String> {
    to_ogc_wkt(epsg).ok()
}

/// Parse EPSG code from common CRS reference strings.
///
/// Supports forms like:
/// - `4326`
/// - `EPSG:4326`
/// - `urn:ogc:def:crs:EPSG::32633`
/// - `http://www.opengis.net/def/crs/EPSG/0/3857`
pub fn epsg_from_srs_reference(s: &str) -> Option<u32> {
    wb_epsg_from_srs_reference(s)
}

/// Parse EPSG code from WKT text with lenient adaptive matching.
///
/// Resolution order:
/// 1) Embedded EPSG authority/ID/reference markers.
/// 2) Adaptive best-match against supported EPSG candidates.
pub fn epsg_from_wkt(wkt: &str) -> Option<u32> {
    epsg_from_wkt_lenient(wkt)
}

/// Parse EPSG code from WKT text with lenient adaptive matching.
pub fn epsg_from_wkt_lenient(wkt: &str) -> Option<u32> {
    wb_identify_epsg_from_wkt_with_policy(wkt, EpsgIdentifyPolicy::Lenient)
}

/// Parse EPSG code from WKT text with strict ambiguity rejection.
pub fn epsg_from_wkt_strict(wkt: &str) -> Option<u32> {
    wb_identify_epsg_from_wkt_with_policy(wkt, EpsgIdentifyPolicy::Strict)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_epsg_srs_forms() {
        assert_eq!(epsg_from_srs_reference("EPSG:3857"), Some(3857));
        assert_eq!(
            epsg_from_srs_reference("urn:ogc:def:crs:EPSG::32633"),
            Some(32633)
        );
    }

    #[test]
    fn parses_epsg_from_wkt_authority() {
        let wkt = "GEOGCS[\"WGS 84\",AUTHORITY[\"EPSG\",\"4326\"]]";
        assert_eq!(epsg_from_wkt(wkt), Some(4326));
    }

    #[test]
    fn parses_epsg_from_legacy_wkt_without_authority_lenient() {
        let wkt = "PROJCS[\"NAD83_CSRS_UTM_zone_17N\",GEOGCS[\"GCS_NAD83(CSRS)\",DATUM[\"D_North_American_1983_CSRS\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.017453292519943295]],PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",-81],PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1]]";
        assert_eq!(epsg_from_wkt_lenient(wkt), Some(2958));
    }

    #[test]
    fn strict_mode_rejects_ambiguous_legacy_wkt() {
        let wkt = "PROJCS[\"NAD83_CSRS_UTM_zone_17N\",GEOGCS[\"GCS_NAD83(CSRS)\",DATUM[\"D_North_American_1983_CSRS\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.017453292519943295]],PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",-81],PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1]]";
        assert_eq!(epsg_from_wkt_strict(wkt), None);
    }

    #[test]
    fn does_not_extract_epsg_from_arbitrary_wkt_text() {
        let wkt = "PROJCS[\"Custom (EPSG:2056)\",GEOGCS[\"WGS 84\"]]";
        assert_eq!(epsg_from_wkt(wkt), None);
    }
}
