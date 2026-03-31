use crate::crs::Crs;
use crate::compound_crs::CompoundCrs;
use crate::datum::{Datum, DatumTransform};
use crate::ellipsoid::Ellipsoid;
use crate::error::{ProjectionError, Result};
use crate::projections::{ProjectionKind, ProjectionParams};
use crate::to_degrees;
use std::collections::HashMap;

#[derive(Debug, Clone)]
enum WktValue {
    Text(String),
    Number(f64),
    Node(WktNode),
}

#[derive(Debug, Clone)]
struct WktNode {
    keyword: String,
    values: Vec<WktValue>,
}

impl WktNode {
    fn first_text(&self) -> Option<&str> {
        self.values.iter().find_map(|value| match value {
            WktValue::Text(text) => Some(text.as_str()),
            _ => None,
        })
    }

    fn child(&self, names: &[&str]) -> Option<&WktNode> {
        self.values.iter().find_map(|value| match value {
            WktValue::Node(node) if names.iter().any(|name| normalized(&node.keyword) == normalized(name)) => Some(node),
            _ => None,
        })
    }

    fn direct_child(&self, names: &[&str]) -> Option<&WktNode> {
        self.values.iter().find_map(|value| match value {
            WktValue::Node(node) if names.iter().any(|name| normalized(&node.keyword) == normalized(name)) => Some(node),
            _ => None,
        })
    }

    fn children<'a>(&'a self, names: &'a [&'a str]) -> impl Iterator<Item = &'a WktNode> + 'a {
        self.values.iter().filter_map(move |value| match value {
            WktValue::Node(node) if names.iter().any(|name| normalized(&node.keyword) == normalized(name)) => Some(node),
            _ => None,
        })
    }

}

struct WktParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> WktParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse(mut self) -> Result<WktNode> {
        self.skip_ws();
        let node = self.parse_node()?;
        self.skip_ws();
        while self.pos != self.input.len() {
            if self.peek_char() != Some(',') {
                return Err(ProjectionError::UnsupportedProjection(
                    "unexpected trailing content in WKT".to_string(),
                ));
            }
            self.bump_char();
            self.skip_ws();
            let trailing = self.parse_node()?;
            let key = normalized(&trailing.keyword);
            if key != "vertcs" {
                return Err(ProjectionError::UnsupportedProjection(format!(
                    "unexpected trailing WKT node '{}'",
                    trailing.keyword
                )));
            }
            self.skip_ws();
        }
        Ok(node)
    }

    fn parse_node(&mut self) -> Result<WktNode> {
        let keyword = self.parse_identifier()?;
        self.skip_ws();
        let open = self.peek_char().ok_or_else(|| {
            ProjectionError::UnsupportedProjection("unexpected end of WKT".to_string())
        })?;
        let close = match open {
            '[' => ']',
            '(' => ')',
            _ => {
                return Err(ProjectionError::UnsupportedProjection(format!(
                    "expected '[' or '(' after WKT keyword '{keyword}'"
                )))
            }
        };
        self.bump_char();

        let mut values = Vec::new();
        loop {
            self.skip_ws();
            if self.peek_char() == Some(close) {
                self.bump_char();
                break;
            }
            values.push(self.parse_value()?);
            self.skip_ws();
            match self.peek_char() {
                Some(',') => {
                    self.bump_char();
                }
                Some(ch) if ch == close => {
                    self.bump_char();
                    break;
                }
                Some(ch) => {
                    return Err(ProjectionError::UnsupportedProjection(format!(
                        "unexpected character '{ch}' in WKT"
                    )))
                }
                None => {
                    return Err(ProjectionError::UnsupportedProjection(
                        "unterminated WKT node".to_string(),
                    ))
                }
            }
        }

        Ok(WktNode { keyword, values })
    }

    fn parse_value(&mut self) -> Result<WktValue> {
        self.skip_ws();
        let Some(ch) = self.peek_char() else {
            return Err(ProjectionError::UnsupportedProjection(
                "unexpected end of WKT while parsing value".to_string(),
            ));
        };

        if ch == '"' {
            return Ok(WktValue::Text(self.parse_string()?));
        }

        if ch == '+' || ch == '-' || ch == '.' || ch.is_ascii_digit() {
            return Ok(WktValue::Number(self.parse_number()?));
        }

        let start = self.pos;
        let ident = self.parse_identifier()?;
        self.skip_ws();
        match self.peek_char() {
            Some('[') | Some('(') => {
                self.pos = start;
                Ok(WktValue::Node(self.parse_node()?))
            }
            _ => Ok(WktValue::Text(ident)),
        }
    }

    fn parse_identifier(&mut self) -> Result<String> {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.bump_char();
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(ProjectionError::UnsupportedProjection(
                "expected WKT identifier".to_string(),
            ));
        }
        Ok(self.input[start..self.pos].to_string())
    }

    fn parse_string(&mut self) -> Result<String> {
        if self.bump_char() != Some('"') {
            return Err(ProjectionError::UnsupportedProjection(
                "expected opening quote in WKT string".to_string(),
            ));
        }
        let mut out = String::new();
        while let Some(ch) = self.bump_char() {
            if ch == '"' {
                if self.peek_char() == Some('"') {
                    self.bump_char();
                    out.push('"');
                } else {
                    return Ok(out);
                }
            } else {
                out.push(ch);
            }
        }
        Err(ProjectionError::UnsupportedProjection(
            "unterminated quoted string in WKT".to_string(),
        ))
    }

    fn parse_number(&mut self) -> Result<f64> {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() || matches!(ch, '+' | '-' | '.' | 'e' | 'E') {
                self.bump_char();
            } else {
                break;
            }
        }
        self.input[start..self.pos].parse::<f64>().map_err(|_| {
            ProjectionError::UnsupportedProjection(format!(
                "invalid numeric literal '{}' in WKT",
                &self.input[start..self.pos]
            ))
        })
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek_char(), Some(ch) if ch.is_ascii_whitespace()) {
            self.bump_char();
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn bump_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }
}

pub(crate) fn parse_crs_from_wkt(wkt: &str) -> Result<Crs> {
    let root = WktParser::new(wkt).parse()?;
    build_crs_from_node(&root)
}

pub(crate) fn parse_compound_crs_from_wkt(wkt: &str) -> Result<CompoundCrs> {
    let root = WktParser::new(wkt).parse()?;
    build_compound_crs_from_node(&root)
}

fn build_crs_from_node(root: &WktNode) -> Result<Crs> {
    match normalized(&root.keyword).as_str() {
        "geogcs" | "geogcrs" | "geodcrs" | "geodeticcrs" => build_geographic_crs(root),
        "projcs" | "projcrs" | "projectedcrs" => build_projected_crs(root),
        "geoccs" | "geocrs" | "geocentriccrs" => build_geocentric_crs(root),
        "vertcs" | "vertcrs" | "verticalcrs" => build_vertical_crs(root),
        "compdcs" | "compoundcrs" => Err(ProjectionError::UnsupportedProjection(
            "compound WKT requires compound_from_wkt()".to_string(),
        )),
        other => Err(ProjectionError::UnsupportedProjection(format!(
            "unsupported WKT root '{other}'"
        ))),
    }
}

fn build_compound_crs_from_node(root: &WktNode) -> Result<CompoundCrs> {
    let root_key = normalized(&root.keyword);
    if root_key != "compdcs" && root_key != "compoundcrs" {
        return Err(ProjectionError::UnsupportedProjection(
            "WKT root is not COMPD_CS/COMPOUNDCRS".to_string(),
        ));
    }

    let name = root.first_text().unwrap_or("Unnamed compound CRS");
    let (horizontal_components, vertical_components) = flatten_compound_components(root)?;

    if horizontal_components.is_empty() {
        return Err(ProjectionError::UnsupportedProjection(
            "compound WKT missing horizontal component".to_string(),
        ));
    }
    if horizontal_components.len() > 1 {
        return Err(ProjectionError::UnsupportedProjection(
            "compound WKT has multiple horizontal components; expected exactly one".to_string(),
        ));
    }

    if vertical_components.is_empty() {
        return Err(ProjectionError::UnsupportedProjection(
            "compound WKT missing vertical component".to_string(),
        ));
    }
    if vertical_components.len() > 1 {
        return Err(ProjectionError::UnsupportedProjection(
            "compound WKT has multiple vertical components; expected exactly one".to_string(),
        ));
    }

    let mut horizontal_components = horizontal_components;
    let mut vertical_components = vertical_components;
    let horizontal = horizontal_components.remove(0);
    let vertical = vertical_components.remove(0);
    CompoundCrs::new(name, horizontal, vertical)
}

fn flatten_compound_components(root: &WktNode) -> Result<(Vec<Crs>, Vec<Crs>)> {
    let mut horizontal_components = Vec::new();
    let mut vertical_components = Vec::new();

    for value in &root.values {
        let WktValue::Node(child) = value else {
            continue;
        };

        let key = normalized(&child.keyword);
        match key.as_str() {
            "projcs" | "projcrs" | "projectedcrs" | "geogcs" | "geogcrs" | "geodcrs"
            | "geodeticcrs" | "geoccs" | "geocrs" | "geocentriccrs" => {
                let crs = build_crs_from_node(child)?;
                if matches!(crs.projection.params().kind, ProjectionKind::Vertical) {
                    vertical_components.push(crs);
                } else {
                    horizontal_components.push(crs);
                }
            }
            "vertcs" | "vertcrs" | "verticalcrs" => {
                vertical_components.push(build_vertical_crs(child)?);
            }
            "compdcs" | "compoundcrs" => {
                let (nested_h, nested_v) = flatten_compound_components(child)?;
                horizontal_components.extend(nested_h);
                vertical_components.extend(nested_v);
            }
            _ => {}
        }
    }

    Ok((horizontal_components, vertical_components))
}

fn build_geographic_crs(root: &WktNode) -> Result<Crs> {
    let name = root.first_text().unwrap_or("Unnamed geographic CRS");
    let geodetic_root = if root.child(&["DATUM", "GEODETICDATUM"]).is_some() {
        root
    } else {
        root.child(&["GEOGCS", "GEOGCRS", "GEODCRS", "GEODETICCRS", "BASEGEOGCRS", "BASEGEODCRS"])
            .unwrap_or(root)
    };
    let geodetic = parse_geodetic_context(geodetic_root)?;
    Crs::new(
        name,
        geodetic.datum,
        ProjectionParams::new(ProjectionKind::Geographic).with_ellipsoid(geodetic.ellipsoid),
    )
}

fn build_geocentric_crs(root: &WktNode) -> Result<Crs> {
    let name = root.first_text().unwrap_or("Unnamed geocentric CRS");
    let geodetic = parse_geodetic_context(root)?;
    Crs::new(
        name,
        geodetic.datum,
        ProjectionParams::new(ProjectionKind::Geocentric).with_ellipsoid(geodetic.ellipsoid),
    )
}

fn build_vertical_crs(root: &WktNode) -> Result<Crs> {
    let name = root.first_text().unwrap_or("Unnamed vertical CRS");
    Crs::new(name, Datum::WGS84, ProjectionParams::new(ProjectionKind::Vertical))
}

fn build_projected_crs(root: &WktNode) -> Result<Crs> {
    let name = root.first_text().unwrap_or("Unnamed projected CRS");

    let geodetic_node = root
        .child(&["GEOGCS", "BASEGEOGCRS", "GEOGCRS", "BASEGEODCRS", "GEODCRS"])
        .ok_or_else(|| ProjectionError::UnsupportedProjection("projected WKT missing geographic base CRS".to_string()))?;
    let geodetic = parse_geodetic_context(geodetic_node)?;

    let projected_unit_factor = projected_length_unit_factor(root).unwrap_or(1.0);
    let angular_unit_factor = geodetic.angular_unit_factor;
    let conversion = root.child(&["CONVERSION"]);

    let method_name = if let Some(conversion_node) = conversion {
        conversion_node
            .child(&["METHOD"])
            .and_then(WktNode::first_text)
            .ok_or_else(|| ProjectionError::UnsupportedProjection("WKT2 conversion is missing METHOD".to_string()))?
    } else {
        root.child(&["PROJECTION"])
            .and_then(WktNode::first_text)
            .ok_or_else(|| ProjectionError::UnsupportedProjection("WKT1 projected CRS is missing PROJECTION".to_string()))?
    };

    let mut parameter_map = HashMap::new();
    let parameter_parent = conversion.unwrap_or(root);
    for parameter in parameter_parent.children(&["PARAMETER"]) {
        if let Some((key, value)) = parse_parameter(parameter, angular_unit_factor, projected_unit_factor) {
            parameter_map.insert(key, value);
        }
    }

    let method_key = normalized(method_name);
    let mut params = build_projection_params(&method_key, &parameter_map, geodetic.prime_meridian_deg)?;
    params = params.with_ellipsoid(geodetic.ellipsoid);
    Crs::new(name, geodetic.datum, params)
}

#[derive(Clone)]
struct GeodeticContext {
    datum: Datum,
    ellipsoid: Ellipsoid,
    angular_unit_factor: f64,
    prime_meridian_deg: f64,
}

fn parse_geodetic_context(root: &WktNode) -> Result<GeodeticContext> {
    let datum_node = root
        .child(&["DATUM", "GEODETICDATUM"])
        .ok_or_else(|| ProjectionError::UnsupportedProjection("WKT missing DATUM/GEODETICDATUM".to_string()))?;
    let datum_name = datum_node.first_text().unwrap_or("Custom");
    let ellipsoid_node = datum_node
        .child(&["SPHEROID", "ELLIPSOID"])
        .ok_or_else(|| ProjectionError::UnsupportedProjection("WKT datum missing SPHEROID/ELLIPSOID".to_string()))?;

    let ellipsoid_name = ellipsoid_node.first_text().unwrap_or("Sphere");
    let semi_major = nth_number(ellipsoid_node, 0).ok_or_else(|| {
        ProjectionError::UnsupportedProjection("WKT ellipsoid missing semi-major axis".to_string())
    })?;
    let inv_f = nth_number(ellipsoid_node, 1).unwrap_or(0.0);
    let ellipsoid = if inv_f.abs() < 1e-15 {
        Ellipsoid::sphere("Sphere", semi_major)
    } else if let Some(known) = Ellipsoid::from_name(ellipsoid_name) {
        known
    } else {
        Ellipsoid::from_a_inv_f("Custom", semi_major, inv_f)
    };

    let prime_meridian_deg = root
        .child(&["PRIMEM", "PRIMEMERIDIAN"])
        .and_then(|node| nth_number(node, 0))
        .map(|value| value * angle_unit_factor_from_node(root).unwrap_or(std::f64::consts::PI / 180.0))
        .map(to_degrees)
        .unwrap_or(0.0);
    let datum = datum_from_name(datum_name, &ellipsoid);

    Ok(GeodeticContext {
        datum,
        ellipsoid,
        angular_unit_factor: angle_unit_factor_from_node(root).unwrap_or(std::f64::consts::PI / 180.0),
        prime_meridian_deg,
    })
}

fn build_projection_params(
    method_key: &str,
    params: &HashMap<String, f64>,
    prime_meridian_deg: f64,
) -> Result<ProjectionParams> {
    let mut false_easting = param(params, &["falseeasting"], 0.0);
    let mut false_northing = param(params, &["falsenorthing"], 0.0);
    let mut scale = param(params, &["scalefactor", "scalefactoratnaturalorigin"], 1.0);

    let central_meridian = longitude_param(
        params,
        &[
            "centralmeridian",
            "longitudeofcenter",
            "longitudeoforigin",
            "longitudeofnaturalorigin",
            "initiallongitude",
        ],
        prime_meridian_deg,
        0.0,
    );
    let latitude_of_origin = param(
        params,
        &["latitudeoforigin", "latitudeofcenter", "projectionlatitude", "latitudeofnaturalorigin"],
        0.0,
    );

    let kind = match method_key {
        "geographic" => ProjectionKind::Geographic,
        "mercator" | "mercator1sp" => ProjectionKind::Mercator,
        "mercatorauxiliarysphere" => ProjectionKind::WebMercator,
        "transversemercator" => ProjectionKind::TransverseMercator,
        "transversemercatorsouthorientated" => ProjectionKind::TransverseMercatorSouthOrientated,
        "transversemercatorzonedgridsystem" => ProjectionKind::TransverseMercator,
        "tunisiamininggrid" => ProjectionKind::TransverseMercator,
        "newzealandmapgrid" => ProjectionKind::TransverseMercator,
        "lambertconformalconic" | "lambertconformalconic1sp" | "lambertconicconformalwestorientated" => {
            let lat1 = param(params, &["standardparallel1"], latitude_of_origin);
            if params.contains_key("standardparallel2") {
                ProjectionKind::LambertConformalConic {
                    lat1,
                    lat2: Some(param(params, &["standardparallel2"], lat1)),
                }
            } else {
                ProjectionKind::LambertConformalConic {
                    lat1,
                    lat2: None,
                }
            }
        }
        "lambertconformalconic2sp" | "lambertconformalconic2spbelgium" | "lambertconformalconicspbelgium" => ProjectionKind::LambertConformalConic {
            lat1: param(params, &["standardparallel1"], latitude_of_origin),
            lat2: Some(param(params, &["standardparallel2"], latitude_of_origin)),
        },
        "lambertconicnearconformal" => ProjectionKind::LambertConformalConic {
            lat1: latitude_of_origin,
            lat2: None,
        },
        "albers" | "albersconicequalarea" => ProjectionKind::AlbersEqualAreaConic {
            lat1: param(params, &["standardparallel1"], latitude_of_origin),
            lat2: param(params, &["standardparallel2"], latitude_of_origin),
        },
        "azimuthalequidistant" => ProjectionKind::AzimuthalEquidistant,
        "twopointequidistant" => ProjectionKind::TwoPointEquidistant {
            lon1: longitude_param(params, &["longitudeof1stpoint"], prime_meridian_deg, 0.0),
            lat1: param(params, &["latitudeof1stpoint"], 0.0),
            lon2: longitude_param(params, &["longitudeof2ndpoint"], prime_meridian_deg, 0.0),
            lat2: param(params, &["latitudeof2ndpoint"], 0.0),
        },
        "lambertazimuthalequalarea" => ProjectionKind::LambertAzimuthalEqualArea,
        "krovak" => ProjectionKind::Krovak,
        "hotineobliquemercatorazimuthcenter" => ProjectionKind::HotineObliqueMercator {
            azimuth: param(params, &["azimuth"], 0.0),
            rectified_grid_angle: None,
        },
        // Natural-origin variant: in the spherical approximation the natural origin
        // coincides with the projection centre, so we can reuse HOMAC unchanged.
        "hotineobliquemercatorazimuthnaturalorigin" => ProjectionKind::HotineObliqueMercator {
            azimuth: param(params, &["azimuth"], 0.0),
            rectified_grid_angle: None,
        },
        "rectifiedskeworthomorphiccenter" => ProjectionKind::HotineObliqueMercator {
            azimuth: param(params, &["azimuth"], 0.0),
            rectified_grid_angle: Some(param(params, &["xyplanerotation", "rectifiedgridangle"], param(params, &["azimuth"], 0.0))),
        },
        "rectifiedskeworthomorphicnaturalorigin" => ProjectionKind::HotineObliqueMercator {
            azimuth: param(params, &["azimuth"], 0.0),
            rectified_grid_angle: Some(param(params, &["xyplanerotation", "rectifiedgridangle"], param(params, &["azimuth"], 0.0))),
        },
        "labordeobliquemercator" => ProjectionKind::HotineObliqueMercator {
            azimuth: param(params, &["azimuth"], 0.0),
            rectified_grid_angle: None,
        },
        "centralconic" => ProjectionKind::CentralConic {
            lat1: param(params, &["standardparallel1"], latitude_of_origin),
        },
        "lagrange" => ProjectionKind::Lagrange {
            lat1: param(params, &["latitudeoforigin", "standardparallel1"], 0.0),
            w: param(params, &["w"], 1.4),
        },
        "loximuthal" => ProjectionKind::Loximuthal {
            lat1: param(params, &["standardparallel1"], 0.0),
        },
        "euler" => ProjectionKind::Euler {
            lat1: param(params, &["standardparallel1"], 0.0),
            lat2: param(params, &["standardparallel2"], 0.0),
        },
        "tissot" => ProjectionKind::Tissot {
            lat1: param(params, &["standardparallel1"], 0.0),
            lat2: param(params, &["standardparallel2"], 0.0),
        },
        "murdochi" => ProjectionKind::MurdochI {
            lat1: param(params, &["standardparallel1"], 0.0),
            lat2: param(params, &["standardparallel2"], 0.0),
        },
        "murdochii" => ProjectionKind::MurdochII {
            lat1: param(params, &["standardparallel1"], 0.0),
            lat2: param(params, &["standardparallel2"], 0.0),
        },
        "murdochiii" => ProjectionKind::MurdochIII {
            lat1: param(params, &["standardparallel1"], 0.0),
            lat2: param(params, &["standardparallel2"], 0.0),
        },
        "perspectiveconic" => ProjectionKind::PerspectiveConic {
            lat1: param(params, &["standardparallel1"], 0.0),
            lat2: param(params, &["standardparallel2"], 0.0),
        },
        "vitkovskyi" => ProjectionKind::VitkovskyI {
            lat1: param(params, &["standardparallel1"], 0.0),
            lat2: param(params, &["standardparallel2"], 0.0),
        },
        "toblermercator" => ProjectionKind::ToblerMercator,
        "winkelii" => ProjectionKind::WinkelII,
        "kavrayskiyv" => ProjectionKind::KavrayskiyV,
        "stereographic" => ProjectionKind::Stereographic,
        "obliquestereographic" => ProjectionKind::ObliqueStereographic,
        // Polar Stereographic — Variant A (k₀ at the pole, scale from scalefactor param)
        "polarstereographicvarianta" => ProjectionKind::PolarStereographic {
            north: latitude_of_origin >= 0.0,
            lat_ts: None,
        },
        "polarstereographicvariantc" => {
            let lat_ts = param(params, &["latitudeofstandardparallel", "standardparallel1"], -90.0);
            ProjectionKind::PolarStereographic {
                north: lat_ts >= 0.0,
                lat_ts: Some(lat_ts),
            }
        }
        // North/South Pole forms: scale = 1 at the given standard parallel
        "stereographicnorthpole" => ProjectionKind::PolarStereographic {
            north: true,
            lat_ts: Some(param(params, &["standardparallel1"], 90.0)),
        },
        "stereographicsouthpole" => ProjectionKind::PolarStereographic {
            north: false,
            lat_ts: Some(param(params, &["standardparallel1"], -90.0)),
        },
        // IGAC Plano Cartesiano: a Transverse Mercator centred at the given point with
        // height-adjusted scale k = 1 / (1 + H/R). Scale and lat0/lon0 are fixed up
        // in the post-match block below.
        "igacplanocartesiano" => ProjectionKind::TransverseMercator,
        "orthographic" => ProjectionKind::Orthographic,
        "sinusoidal" => ProjectionKind::Sinusoidal,
        "mollweide" => ProjectionKind::Mollweide,
        "mcbrydethomasflatpolesine" => ProjectionKind::MbtFps,
        "mcbrydethomasflatpolarsine" => ProjectionKind::MbtS,
        "mcbrydethomasflatpolarparabolic" => ProjectionKind::Mbtfpp,
        "mcbrydethomasflatpolarquartic" => ProjectionKind::Mbtfpq,
        "nell" => ProjectionKind::Nell,
        "equalearth" => ProjectionKind::EqualEarth,
        "lambertcylindricalequalarea" | "cylindricalequalarea" => ProjectionKind::CylindricalEqualArea {
            lat_ts: param(params, &["standardparallel1"], 0.0),
        },
        "equirectangular" | "eqc" | "equidistantcylindrical" | "platecarree" => ProjectionKind::Equirectangular {
            lat_ts: param(params, &["standardparallel1"], 0.0),
        },
        "robinson" => ProjectionKind::Robinson,
        "gnomonic" => ProjectionKind::Gnomonic,
        "aitoff" => ProjectionKind::Aitoff,
        "vandergrinten" | "vandergrinteni" => ProjectionKind::VanDerGrinten,
        "winkeltripel" => ProjectionKind::WinkelTripel,
        "hammer" => ProjectionKind::Hammer,
        "hatano" => ProjectionKind::Hatano,
        "eckerti" => ProjectionKind::EckertI,
        "eckertii" => ProjectionKind::EckertII,
        "eckertiii" => ProjectionKind::EckertIII,
        "eckertiv" => ProjectionKind::EckertIV,
        "eckertv" => ProjectionKind::EckertV,
        "millercylindrical" => ProjectionKind::MillerCylindrical,
        "gallstereographic" => ProjectionKind::GallStereographic,
        "gallpeters" => ProjectionKind::GallPeters,
        "behrmann" => ProjectionKind::Behrmann,
        "hobodyer" => ProjectionKind::HoboDyer,
        "wagneri" => ProjectionKind::WagnerI,
        "wagnerii" => ProjectionKind::WagnerII,
        "wagneriii" => ProjectionKind::WagnerIII,
        "wagneriv" => ProjectionKind::WagnerIV,
        "wagnerv" => ProjectionKind::WagnerV,
        "naturalearth" => ProjectionKind::NaturalEarth,
        "naturalearthii" => ProjectionKind::NaturalEarthII,
        "wagnervi" => ProjectionKind::WagnerVI,
        "eckertvi" => ProjectionKind::EckertVI,
        "transversecylindricalequalarea" => ProjectionKind::TransverseCylindricalEqualArea,
        "polyconic" => ProjectionKind::Polyconic,
        "cassini" => ProjectionKind::Cassini,
        "bonne" => ProjectionKind::Bonne,
        "bonnesouthorientated" => ProjectionKind::BonneSouthOrientated,
        "crasterparabolic" => ProjectionKind::Craster,
        "putninsp4p" => ProjectionKind::PutninsP4p,
        "fahey" => ProjectionKind::Fahey,
        "times" => ProjectionKind::Times,
        "patterson" => ProjectionKind::Patterson,
        "putninsp3" => ProjectionKind::PutninsP3,
        "putninsp3p" => ProjectionKind::PutninsP3p,
        "putninsp5" => ProjectionKind::PutninsP5,
        "putninsp5p" => ProjectionKind::PutninsP5p,
        "putninsp1" => ProjectionKind::PutninsP1,
        "putninsp2" => ProjectionKind::PutninsP2,
        "putninsp6" => ProjectionKind::PutninsP6,
        "putninsp6p" => ProjectionKind::PutninsP6p,
        "quarticauthalic" => ProjectionKind::QuarticAuthalic,
        "foucaut" => ProjectionKind::Foucaut,
        "winkeli" => ProjectionKind::WinkelI,
        "werenskioldi" => ProjectionKind::WerenskioldI,
        "collignon" => ProjectionKind::Collignon,
        "nellhammer" => ProjectionKind::NellHammer,
        "kavrayskiyvii" => ProjectionKind::KavrayskiyVII,
        "geostationarysatellite" => ProjectionKind::Geostationary {
            satellite_height: param(params, &["satelliteheight"], 35_785_831.0),
            sweep_x: false,
        },
        other => {
            return Err(ProjectionError::UnsupportedProjection(format!(
                "unsupported WKT projection method '{other}'"
            )))
        }
    };

    let mut out = ProjectionParams::new(kind)
        .with_lon0(central_meridian)
        .with_lat0(latitude_of_origin)
        .with_false_easting(false_easting)
        .with_false_northing(false_northing)
        .with_scale(scale);

    if matches!(
        method_key,
        "lambertazimuthalequalarea"
            | "azimuthalequidistant"
            | "hotineobliquemercatorazimuthcenter"
            | "hotineobliquemercatorazimuthnaturalorigin"
            | "rectifiedskeworthomorphiccenter"
            | "rectifiedskeworthomorphicnaturalorigin"
            | "labordeobliquemercator"
            | "krovak"
    ) {
        out = out
            .with_lon0(longitude_param(
                params,
                &["centralmeridian", "longitudeofcenter", "longitudeoforigin"],
                prime_meridian_deg,
                central_meridian,
            ))
            .with_lat0(param(
                params,
                &["latitudeofcenter", "latitudeoforigin"],
                latitude_of_origin,
            ));
    }
    // IGAC Plano Cartesiano: apply height-based scale factor k = R/(R+H).
    // lon0 / lat0 are already correct (extracted from LongitudeOfCenter / LatitudeOfCenter).
    if method_key == "igacplanocartesiano" {
        let height = param(params, &["height"], 0.0);
        scale = 1.0 / (1.0 + height / 6_371_000.0);
        out = out.with_scale(scale);
    }

    // Polar Stereographic Variant C uses false-origin parameter names.
    if method_key == "polarstereographicvariantc" {
        false_easting = param(params, &["eastingatfalseorigin"], false_easting);
        false_northing = param(params, &["northingatfalseorigin"], false_northing);
        out = out
            .with_false_easting(false_easting)
            .with_false_northing(false_northing);
    }

    Ok(out)
}

fn parse_parameter(
    parameter: &WktNode,
    default_angle_unit_factor: f64,
    default_length_unit_factor: f64,
) -> Option<(String, f64)> {
    let name = normalized(parameter.first_text()?);
    let raw_value = nth_number(parameter, 0)?;

    let value = if let Some(unit) = parameter.direct_child(&["ANGLEUNIT"]) {
        raw_value * unit_factor(unit, default_angle_unit_factor)
    } else if let Some(unit) = parameter.direct_child(&["LENGTHUNIT", "UNIT"]) {
        raw_value * unit_factor(unit, default_length_unit_factor)
    } else if is_angular_parameter(&name) {
        raw_value * default_angle_unit_factor
    } else if is_linear_parameter(&name) {
        raw_value * default_length_unit_factor
    } else {
        raw_value
    };

    let converted = if is_angular_parameter(&name) {
        to_degrees(value)
    } else {
        value
    };

    Some((name, converted))
}

fn angle_unit_factor_from_node(node: &WktNode) -> Option<f64> {
    node.direct_child(&["ANGLEUNIT", "UNIT"]).map(|unit| unit_factor(unit, std::f64::consts::PI / 180.0))
}

fn projected_length_unit_factor(node: &WktNode) -> Option<f64> {
    if let Some(unit) = node.direct_child(&["LENGTHUNIT"]) {
        return Some(unit_factor(unit, 1.0));
    }
    // WKT1 PROJCS uses trailing UNIT at the projected CRS root.
    for value in node.values.iter().rev() {
        if let WktValue::Node(unit) = value {
            if normalized(&unit.keyword) == "unit" {
                return Some(unit_factor(unit, 1.0));
            }
        }
    }
    None
}

fn unit_factor(node: &WktNode, default_factor: f64) -> f64 {
    nth_number(node, 0).unwrap_or(default_factor)
}

fn nth_number(node: &WktNode, nth: usize) -> Option<f64> {
    node.values
        .iter()
        .filter_map(|value| match value {
            WktValue::Number(number) => Some(*number),
            _ => None,
        })
        .nth(nth)
}

fn param(params: &HashMap<String, f64>, keys: &[&str], default_value: f64) -> f64 {
    keys.iter()
        .find_map(|key| params.get(&normalized(key)).copied())
        .unwrap_or(default_value)
}

fn longitude_param(
    params: &HashMap<String, f64>,
    keys: &[&str],
    prime_meridian_deg: f64,
    default_value: f64,
) -> f64 {
    param(params, keys, default_value) + prime_meridian_deg
}

fn normalized(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn is_angular_parameter(name: &str) -> bool {
    matches!(
        name,
        "centralmeridian"
            | "initiallongitude"
            | "longitudeofcenter"
            | "longitudeoforigin"
            | "longitudeof1stpoint"
            | "longitudeof2ndpoint"
            | "latitudeoforigin"
            | "latitudeofcenter"
            | "latitudeof1stpoint"
            | "latitudeof2ndpoint"
            | "standardparallel1"
            | "standardparallel2"
            | "azimuth"
            | "rectifiedgridangle"
            | "xyplanerotation"
            | "w"
    )
}

fn is_linear_parameter(name: &str) -> bool {
    matches!(name, "falseeasting" | "falsenorthing" | "satelliteheight")
}

fn datum_from_name(name: &str, ellipsoid: &Ellipsoid) -> Datum {
    match normalized(name).as_str() {
        "wgs84" | "wgs1984" | "dwgs1984" | "worldgeodeticsystem1984" => Datum::WGS84,
        "nad83" | "northamericandatum1983" | "dnorthamericandatum1983" => Datum::NAD83,
        "nad83csrs" | "northamericandatum1983csrs" => Datum::NAD83_CSRS,
        "nad83nsrs2007" => Datum::NAD83_NSRS2007,
        "nad83harn" => Datum::NAD83_HARN,
        "nad27" | "northamericandatum1927" | "dnorthamericandatum1927" => Datum::NAD27,
        "etrs89" => Datum::ETRS89,
        "ed50" | "europeandatum1950" => Datum::ED50,
        "gda94" => Datum::GDA94,
        "gda2020" => Datum::GDA2020,
        "cgcs2000" => Datum::CGCS2000,
        "sirgas2000" => Datum::SIRGAS2000,
        "newbeijing" => Datum::NEW_BEIJING,
        "xian1980" => Datum::XIAN_1980,
        "nzgd2000" => Datum::NZGD2000,
        "jgd2000" => Datum::JGD2000,
        "jgd2011" => Datum::JGD2011,
        "rdn2008" => Datum::RDN2008,
        "vn2000" => Datum::VN2000,
        "osgb36" => Datum::OSGB36,
        "dhdn" => Datum::DHDN,
        "pulkovo194258" => Datum::PULKOVO1942_58,
        "pulkovo194283" => Datum::PULKOVO1942_83,
        "sjtsk" => Datum::S_JTSK,
        "belge1972" => Datum::BELGE1972,
        "amersfoort" => Datum::AMERSFOORT,
        "tm65" => Datum::TM65,
        "katanga1955" => Datum::KATANGA1955,
        "cape" => Datum::CAPE,
        "puertorico1927" => Datum::PUERTO_RICO_1927,
        "stcroix" => Datum::ST_CROIX,
        "ch1903" => Datum::CH1903,
        "ch1903plus" => Datum::CH1903_PLUS,
        "svy21" => Datum::SVY21,
        _ => Datum {
            name: "Custom",
            ellipsoid: ellipsoid.clone(),
            transform: DatumTransform::None,
        },
    }
}
