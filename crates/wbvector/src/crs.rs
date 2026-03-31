use wbprojection::{
    EpsgIdentifyPolicy,
    EpsgIdentifyReport,
    epsg_from_srs_reference as wb_epsg_from_srs_reference,
    identify_epsg_from_wkt_report as wb_identify_epsg_from_wkt_report,
    identify_epsg_from_wkt_with_policy as wb_identify_epsg_from_wkt_with_policy,
    to_ogc_wkt,
    Crs,
};

pub(crate) fn ogc_wkt_from_epsg(epsg: u32) -> Option<String> {
    to_ogc_wkt(epsg).ok()
}

pub(crate) fn crs_name_from_epsg(epsg: u32) -> Option<String> {
    Crs::from_epsg(epsg).ok().map(|c| c.name)
}

pub(crate) fn canonical_epsg_srs_name(epsg: u32) -> String {
    format!("EPSG:{epsg}")
}

pub(crate) fn canonical_gml_epsg_srs_name(epsg: u32) -> String {
    format!("http://www.opengis.net/def/crs/EPSG/0/{epsg}")
}

pub(crate) fn epsg_from_srs_reference(s: &str) -> Option<u32> {
    wb_epsg_from_srs_reference(s)
}

#[allow(dead_code)]
pub(crate) fn epsg_from_wkt(wkt: &str) -> Option<u32> {
    epsg_from_wkt_lenient(wkt)
}

pub(crate) fn epsg_from_wkt_lenient(wkt: &str) -> Option<u32> {
    wb_identify_epsg_from_wkt_with_policy(wkt, EpsgIdentifyPolicy::Lenient)
}

#[allow(dead_code)]
pub(crate) fn epsg_from_wkt_strict(wkt: &str) -> Option<u32> {
    wb_identify_epsg_from_wkt_with_policy(wkt, EpsgIdentifyPolicy::Strict)
}

#[allow(dead_code)]
pub(crate) fn epsg_from_wkt_report(wkt: &str, strict: bool) -> Option<EpsgIdentifyReport> {
    let policy = if strict {
        EpsgIdentifyPolicy::Strict
    } else {
        EpsgIdentifyPolicy::Lenient
    };
    wb_identify_epsg_from_wkt_report(wkt, policy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_epsg_authority() {
        let wkt = "GEOGCS[\"WGS 84\",AUTHORITY[\"EPSG\",\"4326\"]]";
        assert_eq!(epsg_from_wkt(wkt), Some(4326));
    }

    #[test]
    fn parses_epsg_id_wkt2_style() {
        let wkt = "GEOGCRS[\"WGS 84\",ID[\"EPSG\",4326]]";
        assert_eq!(epsg_from_wkt(wkt), Some(4326));
    }

    #[test]
    fn parses_epsg_colon_reference() {
        assert_eq!(epsg_from_srs_reference("EPSG:3857"), Some(3857));
    }

    #[test]
    fn parses_epsg_urn_reference() {
        assert_eq!(epsg_from_srs_reference("urn:ogc:def:crs:EPSG::32633"), Some(32633));
    }

    #[test]
    fn parses_epsg_http_reference() {
        assert_eq!(
            epsg_from_srs_reference("http://www.opengis.net/def/crs/EPSG/0/4326"),
            Some(4326)
        );
    }

    #[test]
    fn does_not_extract_epsg_from_arbitrary_wkt_text() {
        let wkt = "PROJCS[\"Custom (EPSG:2056)\",GEOGCS[\"WGS 84\"]]";
        assert_eq!(epsg_from_wkt(wkt), None);
    }

    #[test]
    fn identifies_legacy_nad83_csrs_utm_wkt_without_authority() {
        let prj = "PROJCS[\"NAD83_CSRS_UTM_zone_17N\",GEOGCS[\"GCS_NAD83(CSRS)\",DATUM[\"D_North_American_1983_CSRS\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.017453292519943295]],PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",-81],PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1]]";
        assert_eq!(epsg_from_wkt_lenient(prj), Some(2958));
    }

    #[test]
    fn strict_mode_rejects_ambiguous_legacy_csrs_utm() {
        let prj = "PROJCS[\"NAD83_CSRS_UTM_zone_17N\",GEOGCS[\"GCS_NAD83(CSRS)\",DATUM[\"D_North_American_1983_CSRS\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.017453292519943295]],PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",-81],PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1]]";
        assert_eq!(epsg_from_wkt_strict(prj), None);
    }

    #[test]
    fn report_exposes_top_candidates_for_legacy_csrs_utm() {
        let prj = "PROJCS[\"NAD83_CSRS_UTM_zone_17N\",GEOGCS[\"GCS_NAD83(CSRS)\",DATUM[\"D_North_American_1983_CSRS\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.017453292519943295]],PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",-81],PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1]]";
        let report = epsg_from_wkt_report(prj, false).expect("expected CRS report");
        assert!(report.passed_threshold);
        assert!(report.ambiguous);
        assert_eq!(report.resolved_code, Some(2958));
        assert!(report.top_candidates.len() >= 2);
        assert_eq!(report.top_candidates[0].code, 2958);
    }
}
