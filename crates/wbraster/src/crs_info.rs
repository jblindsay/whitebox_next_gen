//! Lightweight spatial reference representation.
//!
//! We deliberately avoid pulling in a full projection library. The `CrsInfo`
//! struct stores the raw Well-Known Text string (WKT) and a numeric EPSG code,
//! which is sufficient for round-tripping through every supported format.

use wbprojection::{
    EpsgIdentifyPolicy,
    identify_epsg_from_wkt_with_policy,
    to_ogc_wkt,
};

/// A spatial / coordinate reference system description.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrsInfo {
    /// EPSG numeric authority code, if known (e.g. 4326 for WGS-84 geographic).
    pub epsg: Option<u32>,
    /// OGC Well-Known Text representation of the CRS.
    pub wkt: Option<String>,
    /// The projection string in PROJ4/PROJ format, if available.
    pub proj4: Option<String>,
}

impl CrsInfo {
    /// Create an empty `CrsInfo`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from an EPSG code alone.
    pub fn from_epsg(code: u32) -> Self {
        Self {
            epsg: Some(code),
            wkt: to_ogc_wkt(code).ok(),
            ..Default::default()
        }
    }

    /// Create from a WKT string.
    pub fn from_wkt(wkt: impl Into<String>) -> Self {
        Self::from_wkt_with_policy(wkt, EpsgIdentifyPolicy::Lenient)
    }

    /// Create from a WKT string with explicit identification policy.
    pub fn from_wkt_with_policy(wkt: impl Into<String>, policy: EpsgIdentifyPolicy) -> Self {
        let wkt = wkt.into();
        Self {
            epsg: identify_epsg_from_wkt_with_policy(&wkt, policy),
            wkt: Some(wkt),
            ..Default::default()
        }
    }

    /// Create from WKT with strict ambiguity rejection.
    pub fn from_wkt_strict(wkt: impl Into<String>) -> Self {
        Self::from_wkt_with_policy(wkt, EpsgIdentifyPolicy::Strict)
    }

    /// Returns `true` if no CRS information is stored.
    pub fn is_unknown(&self) -> bool {
        self.epsg.is_none() && self.wkt.is_none() && self.proj4.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::CrsInfo;

    #[test]
    fn from_epsg_populates_wkt_when_available() {
        let crs = CrsInfo::from_epsg(4326);
        assert_eq!(crs.epsg, Some(4326));
        assert!(crs.wkt.as_deref().map(|w| !w.is_empty()).unwrap_or(false));
    }

    #[test]
    fn from_wkt_infers_epsg_when_authority_present() {
        let wkt = "GEOGCS[\"WGS 84\",AUTHORITY[\"EPSG\",\"4326\"]]";
        let crs = CrsInfo::from_wkt(wkt);
        assert_eq!(crs.epsg, Some(4326));
        assert_eq!(crs.wkt.as_deref(), Some(wkt));
    }

    #[test]
    fn from_wkt_lenient_infers_legacy_epsg_without_authority() {
        let wkt = "PROJCS[\"NAD83_CSRS_UTM_zone_17N\",GEOGCS[\"GCS_NAD83(CSRS)\",DATUM[\"D_North_American_1983_CSRS\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.017453292519943295]],PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",-81],PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1]]";
        let crs = CrsInfo::from_wkt(wkt);
        assert_eq!(crs.epsg, Some(2958));
    }

    #[test]
    fn from_wkt_strict_rejects_ambiguous_legacy_epsg_without_authority() {
        let wkt = "PROJCS[\"NAD83_CSRS_UTM_zone_17N\",GEOGCS[\"GCS_NAD83(CSRS)\",DATUM[\"D_North_American_1983_CSRS\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.017453292519943295]],PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",-81],PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1]]";
        let crs = CrsInfo::from_wkt_strict(wkt);
        assert_eq!(crs.epsg, None);
    }
}
