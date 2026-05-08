//! PROJ string parser for wbprojection.
//!
//! Handles PROJ4-compatible projection strings such as:
//! ```text
//! +proj=utm +zone=17 +datum=NAD83 +units=m +no_defs
//! +proj=lcc +lat_1=49 +lat_2=77 +lat_0=49 +lon_0=-95 +x_0=0 +y_0=0 +datum=NAD83 +units=m
//! +proj=tmerc +lat_0=0 +lon_0=-75 +k=0.9996 +x_0=500000 +y_0=0 +ellps=GRS80 +units=m
//! ```
//!
//! Also accepts `+init=epsg:XXXX` and `EPSG:XXXX` shortcuts (resolved through the
//! built-in EPSG registry).
//!
//! # Unit handling
//! PROJ specifies `+x_0` / `+y_0` (false easting/northing) in **metres** regardless
//! of the `+units=` parameter. `+units=` and `+to_meter=` describe the output
//! coordinate unit of the forward transformation, not the structural parameters.
//! Since `wbprojection` projections work internally in metres, `+units=` is parsed
//! and stored in [`ParsedProjUnits`] but does not alter `ProjectionParams`.

use std::collections::HashMap;

use crate::crs::Crs;
use crate::datum::{Datum, DatumTransform, HelmertParams};
use crate::ellipsoid::Ellipsoid;
use crate::error::{ProjectionError, Result};
use crate::projections::{Projection, ProjectionKind, ProjectionParams};

// ─── Public surface ──────────────────────────────────────────────────────────

/// The unit encoding described by a PROJ string's `+units=` / `+to_meter=` tokens.
///
/// The value is the scale factor that converts projection output coordinates
/// into metres (i.e. `output_in_m = output * to_meter`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParsedProjUnits {
    /// Metres-per-output-unit scale factor.
    pub to_meter: f64,
    /// Short label from `+units=`, if present.
    pub label: Option<&'static str>,
}

impl Default for ParsedProjUnits {
    fn default() -> Self {
        ParsedProjUnits { to_meter: 1.0, label: Some("m") }
    }
}

/// Full result of parsing a PROJ string.
///
/// In addition to the [`Crs`] you can inspect the declared output units via
/// [`ParsedProjString::units`].  Most callers only need [`ParsedProjString::crs`].
#[derive(Debug, Clone)]
pub struct ParsedProjString {
    /// The resolved coordinate reference system.
    pub crs: Crs,
    /// Declared output-coordinate units (does not affect stored metre params).
    pub units: ParsedProjUnits,
}

/// Parse a PROJ4-compatible projection string into a [`Crs`].
///
/// This is the main entry point. It accepts:
/// - Full `+key=value` PROJ strings.
/// - `+init=epsg:XXXX` shortcuts (resolved via the built-in EPSG registry).
/// - Bare `EPSG:XXXX` or numeric EPSG codes.
///
/// # Errors
/// Returns [`ProjectionError::UnsupportedProjection`] if `+proj=` is not
/// recognised, or [`ProjectionError::InvalidParameter`] for malformed values.
pub(crate) fn parse_crs_from_proj_string(s: &str) -> Result<Crs> {
    parse_proj_string(s).map(|p| p.crs)
}

/// Parse a PROJ string and return both the CRS and the declared output units.
pub(crate) fn parse_proj_string(s: &str) -> Result<ParsedProjString> {
    let s = s.trim();

    // ── +init=epsg:XXXX  or  EPSG:XXXX  or bare number ───────────────────────
    if let Some(code) = try_epsg_shortcut(s) {
        let crs = crate::epsg::from_epsg(code)?;
        return Ok(ParsedProjString { crs, units: ParsedProjUnits::default() });
    }

    let tokens = tokenize(s);

    // ── ellipsoid, then datum ─────────────────────────────────────────────────
    let ellipsoid = resolve_ellipsoid(&tokens)?;
    let datum = resolve_datum(&tokens, &ellipsoid);

    // ── common numeric params ─────────────────────────────────────────────────
    let lon0 = parse_angle(&tokens, "lon_0")
        .or_else(|| parse_angle(&tokens, "lonc"))   // omerc center
        .unwrap_or(0.0);
    let lat0 = parse_angle(&tokens, "lat_0").unwrap_or(0.0);
    let x0   = parse_f64(&tokens, "x_0").unwrap_or(0.0);
    let y0   = parse_f64(&tokens, "y_0").unwrap_or(0.0);
    let k0   = parse_f64(&tokens, "k_0")
        .or_else(|| parse_f64(&tokens, "k"))
        .unwrap_or(1.0);

    // Standard parallels / lat of true scale
    let lat1   = parse_angle(&tokens, "lat_1");
    let lat2   = parse_angle(&tokens, "lat_2");
    let lat_ts = parse_angle(&tokens, "lat_ts");

    // Units
    let units = resolve_units(&tokens);

    // ── projection kind ───────────────────────────────────────────────────────
    let proj_name = tokens
        .get("proj")
        .and_then(|v| v.as_deref())
        .unwrap_or("latlong");

    // UTM is special: we apply full canonical UTM params and return early.
    if proj_name == "utm" {
        return build_utm(&tokens, &ellipsoid, &datum, units);
    }

    let kind = match proj_name {
        // ── geographic / geocentric ──────────────────────────────────────────
        "longlat" | "latlong" | "lonlat" | "latlon" | "geographic" | "geog" => {
            ProjectionKind::Geographic
        }
        "geocent" | "geocentric" | "ecef" | "cart" => ProjectionKind::Geocentric,

        // ── Mercator family ──────────────────────────────────────────────────
        "merc" => {
            if ellipsoid.is_sphere() {
                // A spherical earth with +proj=merc is typically Web Mercator
                // (e.g. the standard definition of EPSG:3857).
                ProjectionKind::WebMercator
            } else {
                ProjectionKind::Mercator
            }
        }
        "webmerc" | "web_mercator" | "wmerc" => ProjectionKind::WebMercator,
        "tmerc" | "tmerc_ellps" => ProjectionKind::TransverseMercator,
        "tmerc_so" => ProjectionKind::TransverseMercatorSouthOrientated,
        "mill" | "miller" => ProjectionKind::MillerCylindrical,
        "gall" | "gstmerc" | "gall_stere" => ProjectionKind::GallStereographic,
        "gall_peters" | "gallpeters" | "cea_gall_peters" => ProjectionKind::GallPeters,
        "behrmann" => ProjectionKind::Behrmann,
        "hobo_dyer" | "hobodyer" => ProjectionKind::HoboDyer,
        "tob_mer" | "tobmerc" => ProjectionKind::ToblerMercator,
        "tcea" => ProjectionKind::TransverseCylindricalEqualArea,
        "kav5" | "kav_v" | "kavrayskiy_v" => ProjectionKind::KavrayskiyV,

        "cea" => ProjectionKind::CylindricalEqualArea {
            lat_ts: lat_ts.unwrap_or(0.0),
        },
        "eqc" | "eqrect" | "plate_carree" | "equirectangular" => {
            ProjectionKind::Equirectangular {
                lat_ts: lat_ts.unwrap_or(0.0),
            }
        }

        // ── UTM (already handled above, but kept for clarity) ────────────────
        // (unreachable in practice after the early return above)

        // ── Oblique Mercator ─────────────────────────────────────────────────
        "omerc" | "hotine" | "somerc" => {
            let azimuth = parse_angle(&tokens, "alpha")
                .or_else(|| parse_angle(&tokens, "azimuth"))
                .unwrap_or(0.0);
            let gamma = parse_angle(&tokens, "gamma");
            ProjectionKind::HotineObliqueMercator {
                azimuth,
                rectified_grid_angle: gamma,
            }
        }

        // ── Conic projections ────────────────────────────────────────────────
        "lcc" => {
            let l1 = lat1.ok_or_else(|| ProjectionError::InvalidParameter {
                param: "lat_1".into(),
                reason: "LCC requires +lat_1=".into(),
            })?;
            ProjectionKind::LambertConformalConic { lat1: l1, lat2 }
        }
        "aea" => {
            let l1 = lat1.ok_or_else(|| ProjectionError::InvalidParameter {
                param: "lat_1".into(),
                reason: "Albers Equal-Area requires +lat_1=".into(),
            })?;
            let l2 = lat2.ok_or_else(|| ProjectionError::InvalidParameter {
                param: "lat_2".into(),
                reason: "Albers Equal-Area requires +lat_2=".into(),
            })?;
            ProjectionKind::AlbersEqualAreaConic { lat1: l1, lat2: l2 }
        }
        "krovak" => ProjectionKind::Krovak,
        "bonne" => ProjectionKind::Bonne,
        "bonne_so" | "bonne_south" => ProjectionKind::BonneSouthOrientated,
        "pconic" => {
            let l1 = lat1.ok_or_else(|| ProjectionError::InvalidParameter {
                param: "lat_1".into(),
                reason: "Perspective Conic requires +lat_1=".into(),
            })?;
            let l2 = lat2.ok_or_else(|| ProjectionError::InvalidParameter {
                param: "lat_2".into(),
                reason: "Perspective Conic requires +lat_2=".into(),
            })?;
            ProjectionKind::PerspectiveConic { lat1: l1, lat2: l2 }
        }
        "euler" => {
            let l1 = lat1.ok_or_else(|| ProjectionError::InvalidParameter {
                param: "lat_1".into(),
                reason: "Euler conic requires +lat_1=".into(),
            })?;
            let l2 = lat2.ok_or_else(|| ProjectionError::InvalidParameter {
                param: "lat_2".into(),
                reason: "Euler conic requires +lat_2=".into(),
            })?;
            ProjectionKind::Euler { lat1: l1, lat2: l2 }
        }
        "tissot" => {
            let l1 = lat1.ok_or_else(|| ProjectionError::InvalidParameter {
                param: "lat_1".into(),
                reason: "Tissot conic requires +lat_1=".into(),
            })?;
            let l2 = lat2.ok_or_else(|| ProjectionError::InvalidParameter {
                param: "lat_2".into(),
                reason: "Tissot conic requires +lat_2=".into(),
            })?;
            ProjectionKind::Tissot { lat1: l1, lat2: l2 }
        }
        "murd1" => {
            let l1 = lat1.ok_or_else(|| missing_param("lat_1", "murd1"))?;
            let l2 = lat2.ok_or_else(|| missing_param("lat_2", "murd1"))?;
            ProjectionKind::MurdochI { lat1: l1, lat2: l2 }
        }
        "murd2" => {
            let l1 = lat1.ok_or_else(|| missing_param("lat_1", "murd2"))?;
            let l2 = lat2.ok_or_else(|| missing_param("lat_2", "murd2"))?;
            ProjectionKind::MurdochII { lat1: l1, lat2: l2 }
        }
        "murd3" => {
            let l1 = lat1.ok_or_else(|| missing_param("lat_1", "murd3"))?;
            let l2 = lat2.ok_or_else(|| missing_param("lat_2", "murd3"))?;
            ProjectionKind::MurdochIII { lat1: l1, lat2: l2 }
        }
        "vitk1" => {
            let l1 = lat1.ok_or_else(|| missing_param("lat_1", "vitk1"))?;
            let l2 = lat2.ok_or_else(|| missing_param("lat_2", "vitk1"))?;
            ProjectionKind::VitkovskyI { lat1: l1, lat2: l2 }
        }
        "ccon" => {
            let l1 = lat1.ok_or_else(|| missing_param("lat_1", "ccon"))?;
            ProjectionKind::CentralConic { lat1: l1 }
        }
        "lagrng" => {
            let l1 = lat1.ok_or_else(|| missing_param("lat_1", "lagrng"))?;
            let w = parse_f64(&tokens, "w").unwrap_or(2.0);
            ProjectionKind::Lagrange { lat1: l1, w }
        }
        "loxim" => ProjectionKind::Loximuthal {
            lat1: lat1.unwrap_or(0.0),
        },

        // ── Azimuthal / perspective ──────────────────────────────────────────
        "aeqd" => ProjectionKind::AzimuthalEquidistant,
        "laea" => ProjectionKind::LambertAzimuthalEqualArea,
        "ortho" => ProjectionKind::Orthographic,
        "gnom" => ProjectionKind::Gnomonic,
        "tpeqd" => {
            let lon1 = parse_angle(&tokens, "lon_1").unwrap_or(0.0);
            let ll1  = lat1.unwrap_or(0.0);
            let lon2 = parse_angle(&tokens, "lon_2").unwrap_or(0.0);
            let ll2  = lat2.unwrap_or(0.0);
            ProjectionKind::TwoPointEquidistant { lon1, lat1: ll1, lon2, lat2: ll2 }
        }
        "geos" => {
            let h = parse_f64(&tokens, "h").unwrap_or(35_786_023.0);
            let sweep_x = tokens.get("sweep")
                .and_then(|v| v.as_deref())
                .map(|v| v.eq_ignore_ascii_case("x"))
                .unwrap_or(false);
            ProjectionKind::Geostationary { satellite_height: h, sweep_x }
        }

        // ── Stereographic family ─────────────────────────────────────────────
        "stere" => {
            if lat0.abs() > 89.0 {
                // lat_0 near ±90° → polar variant
                ProjectionKind::PolarStereographic {
                    north: lat0 >= 0.0,
                    lat_ts,
                }
            } else {
                ProjectionKind::Stereographic
            }
        }
        "ups" => ProjectionKind::PolarStereographic {
            north: lat0 >= 0.0,
            lat_ts,
        },
        "sterea" | "oblique_stereographic" | "ostere" => ProjectionKind::ObliqueStereographic,

        // ── Pseudocylindrical ────────────────────────────────────────────────
        "sinu" => ProjectionKind::Sinusoidal,
        "moll" => ProjectionKind::Mollweide,
        "robin" | "robin_s" => ProjectionKind::Robinson,
        "eck1" => ProjectionKind::EckertI,
        "eck2" => ProjectionKind::EckertII,
        "eck3" => ProjectionKind::EckertIII,
        "eck4" => ProjectionKind::EckertIV,
        "eck5" => ProjectionKind::EckertV,
        "eck6" => ProjectionKind::EckertVI,
        "nell" => ProjectionKind::Nell,
        "eqearth" => ProjectionKind::EqualEarth,
        "crast" | "putp4p" => ProjectionKind::Craster,
        "fouc" | "fouc_s" => ProjectionKind::Foucaut,
        "qua_aut" => ProjectionKind::QuarticAuthalic,
        "putp1" => ProjectionKind::PutninsP1,
        "putp2" => ProjectionKind::PutninsP2,
        "putp3" => ProjectionKind::PutninsP3,
        "putp3p" => ProjectionKind::PutninsP3p,
        "putp5" => ProjectionKind::PutninsP5,
        "putp5p" => ProjectionKind::PutninsP5p,
        "putp6" => ProjectionKind::PutninsP6,
        "putp6p" => ProjectionKind::PutninsP6p,
        "mbtfps" => ProjectionKind::MbtFps,
        "mbt_s" => ProjectionKind::MbtS,
        "mbtfpp" => ProjectionKind::Mbtfpp,
        "mbtfpq" => ProjectionKind::Mbtfpq,
        "fahey" => ProjectionKind::Fahey,
        "times" => ProjectionKind::Times,
        "patterson" => ProjectionKind::Patterson,
        "natearth" | "natural_earth" => ProjectionKind::NaturalEarth,
        "natearth2" | "natural_earth2" => ProjectionKind::NaturalEarthII,
        "wag1" => ProjectionKind::WagnerI,
        "wag2" => ProjectionKind::WagnerII,
        "wag3" => ProjectionKind::WagnerIII,
        "wag4" => ProjectionKind::WagnerIV,
        "wag5" => ProjectionKind::WagnerV,
        "wag6" => ProjectionKind::WagnerVI,
        "wink1" => ProjectionKind::WinkelI,
        "wink2" => ProjectionKind::WinkelII,
        "wintri" => ProjectionKind::WinkelTripel,
        "weren" => ProjectionKind::WerenskioldI,
        "nell_h" => ProjectionKind::NellHammer,
        "collg" => ProjectionKind::Collignon,
        "kav7" | "kav_vii" | "kavrayskiy_vii" => ProjectionKind::KavrayskiyVII,
        "aitoff" => ProjectionKind::Aitoff,
        "vandg" | "vandg1" | "van_der_grinten" => ProjectionKind::VanDerGrinten,
        "hammer" | "hammer_eckert" => ProjectionKind::Hammer,
        "hatano" => ProjectionKind::Hatano,

        // ── Misc ─────────────────────────────────────────────────────────────
        "poly" => ProjectionKind::Polyconic,
        "cass" => ProjectionKind::Cassini,

        unknown => {
            return Err(ProjectionError::UnsupportedProjection(format!(
                "PROJ +proj={unknown} is not supported by wbprojection"
            )));
        }
    };

    let params = ProjectionParams {
        kind,
        lon0,
        lat0,
        false_easting: x0,
        false_northing: y0,
        scale: k0,
        ellipsoid: ellipsoid.clone(),
        datum: datum.clone(),
    };

    let crs_name = build_name(&tokens, proj_name, &datum);

    Ok(ParsedProjString {
        crs: Crs {
            name: crs_name,
            datum,
            projection: Projection::new(params)?,
        },
        units,
    })
}

// ─── UTM early-return builder ─────────────────────────────────────────────────

fn build_utm(
    tokens: &HashMap<String, Option<String>>,
    ellipsoid: &Ellipsoid,
    datum: &Datum,
    units: ParsedProjUnits,
) -> Result<ParsedProjString> {
    let zone = tokens
        .get("zone")
        .and_then(|v| v.as_deref())
        .and_then(|v| v.parse::<u8>().ok())
        .ok_or_else(|| ProjectionError::InvalidParameter {
            param: "zone".into(),
            reason: "UTM requires +zone=N where N is 1..=60".into(),
        })?;

    if !(1..=60).contains(&zone) {
        return Err(ProjectionError::InvalidParameter {
            param: "zone".into(),
            reason: format!("UTM zone {zone} is out of range 1..=60"),
        });
    }

    let south = tokens.contains_key("south");
    let lon0 = (zone as f64 - 1.0) * 6.0 - 180.0 + 3.0;

    let params = ProjectionParams {
        kind: ProjectionKind::Utm { zone, south },
        lon0,
        lat0: 0.0,
        false_easting: 500_000.0,
        false_northing: if south { 10_000_000.0 } else { 0.0 },
        scale: 0.9996,
        ellipsoid: ellipsoid.clone(),
        datum: datum.clone(),
    };

    let hemi = if south { "S" } else { "N" };
    let name = format!("{} / UTM zone {}{}", datum.name, zone, hemi);

    Ok(ParsedProjString {
        crs: Crs {
            name,
            datum: datum.clone(),
            projection: Projection::new(params)?,
        },
        units,
    })
}

// ─── Tokenizer ────────────────────────────────────────────────────────────────

/// Tokenize a PROJ string into a `{lowercase_key → Option<value>}` map.
///
/// Tokens without `=` (flags like `+south`, `+no_defs`) are stored with
/// `None` as the value.  The leading `+` is stripped.
fn tokenize(s: &str) -> HashMap<String, Option<String>> {
    let mut map = HashMap::new();
    for raw in s.split_whitespace() {
        let token = raw.trim_start_matches('+');
        if token.is_empty() {
            continue;
        }
        match token.split_once('=') {
            Some((k, v)) => { map.insert(k.to_ascii_lowercase(), Some(v.to_string())); }
            None         => { map.insert(token.to_ascii_lowercase(), None); }
        }
    }
    map
}

// ─── EPSG shortcut detection ──────────────────────────────────────────────────

/// Return the EPSG code if `s` is a bare code or `+init=epsg:`/`EPSG:` shortcut.
fn try_epsg_shortcut(s: &str) -> Option<u32> {
    // "+init=epsg:XXXX"
    if let Some(rest) = strip_prefix_ci(s, "+init=epsg:") {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return digits.parse().ok();
        }
    }
    // "EPSG:XXXX" or "epsg:XXXX"
    if let Some(rest) = strip_prefix_ci(s, "epsg:") {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return digits.parse().ok();
        }
    }
    // Bare numeric code (e.g. "4326")
    if s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty() {
        return s.parse().ok();
    }
    None
}

// ─── Ellipsoid resolution ─────────────────────────────────────────────────────

fn resolve_ellipsoid(tokens: &HashMap<String, Option<String>>) -> Result<Ellipsoid> {
    // 1) +ellps= explicit ellipsoid name
    if let Some(Some(name)) = tokens.get("ellps") {
        if let Some(e) = ellps_from_proj_name(name) {
            return Ok(e);
        }
    }

    // 2) +R= sphere radius shorthand
    if let Some(r) = parse_f64(tokens, "r") {
        return Ok(Ellipsoid::sphere("Custom", r));
    }

    // 3) Infer from +datum=
    if let Some(Some(datum_name)) = tokens.get("datum") {
        if let Some(e) = datum_name_to_ellipsoid(datum_name) {
            return Ok(e);
        }
    }

    // 4) +a= with optional +b=, +rf=, or +f=
    if let Some(a) = parse_f64(tokens, "a") {
        if let Some(b) = parse_f64(tokens, "b") {
            return Ok(if (a - b).abs() < 1.0 {
                Ellipsoid::sphere("Custom", a)
            } else {
                Ellipsoid::from_a_inv_f("Custom", a, a / (a - b))
            });
        }
        if let Some(inv_f) = parse_f64(tokens, "rf") {
            return Ok(Ellipsoid::from_a_inv_f("Custom", a, inv_f));
        }
        if let Some(f) = parse_f64(tokens, "f") {
            return Ok(Ellipsoid::from_a_inv_f("Custom", a, 1.0 / f));
        }
        // +a only → sphere
        return Ok(Ellipsoid::sphere("Custom", a));
    }

    // Default: WGS84
    Ok(Ellipsoid::WGS84.clone())
}

/// Map a PROJ `+ellps=` name to an [`Ellipsoid`].
fn ellps_from_proj_name(name: &str) -> Option<Ellipsoid> {
    match name.to_ascii_lowercase().as_str() {
        "wgs84" | "wgs_84"                         => Some(Ellipsoid::WGS84.clone()),
        "wgs72" | "wgs_72"                         => Some(Ellipsoid::from_a_inv_f("WGS 72", 6_378_135.0, 298.26)),
        "grs80" | "grs1980" | "grs_1980"           => Some(Ellipsoid::GRS80.clone()),
        "grs67" | "grs1967"                        => Some(Ellipsoid::from_a_inv_f("GRS 67", 6_378_160.0, 298.247_167_427)),
        "clrk66" | "clrk1866" | "clarke1866"       => Some(Ellipsoid::CLARKE1866.clone()),
        "clrk80" | "clrk80rgs" | "clarke1880rgs"   => Some(Ellipsoid::CLARKE1880_RGS.clone()),
        "intl" | "international" | "hayford"       => Some(Ellipsoid::INTERNATIONAL.clone()),
        "bessel" | "bess_nam"                      => Some(Ellipsoid::BESSEL.clone()),
        "airy"                                     => Some(Ellipsoid::AIRY1830.clone()),
        "mod_airy" | "airy_mod"                    => Some(Ellipsoid::AIRY1830_MOD.clone()),
        "krass" | "krassovsky" | "krassowsky"      => Some(Ellipsoid::KRASSOWSKY1940.clone()),
        "iau76" | "iau_1976"                       => Some(Ellipsoid::IAU1976.clone()),
        "evrstss" | "evrst30" | "everest"          => Some(Ellipsoid::EVEREST1830.clone()),
        "helmert"                                  => Some(Ellipsoid::HELMERT1906.clone()),
        "sphere"                                   => Some(Ellipsoid::SPHERE.clone()),
        "ans" => Some(Ellipsoid::from_a_inv_f("Australian National Spheroid", 6_378_160.0, 298.25)),
        "new_intl" => Some(Ellipsoid::from_a_inv_f("New International 1967", 6_378_157.5, 298.2496)),
        "grs75" => Some(Ellipsoid::from_a_inv_f("GRS 75", 6_378_140.0, 298.257)),
        "sgs85" => Some(Ellipsoid::from_a_inv_f("SGS 1985", 6_378_136.0, 298.257)),
        _ => None,
    }
}

/// Return the implied ellipsoid for a named PROJ datum.
fn datum_name_to_ellipsoid(datum: &str) -> Option<Ellipsoid> {
    match datum.to_ascii_lowercase().as_str() {
        "wgs84"                       => Some(Ellipsoid::WGS84.clone()),
        "wgs72"                       => Some(Ellipsoid::from_a_inv_f("WGS 72", 6_378_135.0, 298.26)),
        "nad83" | "hpgn" | "nad83_csrs" | "nad83csrs" => Some(Ellipsoid::GRS80.clone()),
        "nad27"                       => Some(Ellipsoid::CLARKE1866.clone()),
        "etrs89" | "etrf89"           => Some(Ellipsoid::GRS80.clone()),
        "gda94"                       => Some(Ellipsoid::GRS80.clone()),
        "gda2020"                     => Some(Ellipsoid::GRS80.clone()),
        "nzgd2000" | "nzgd49"        => Some(Ellipsoid::GRS80.clone()),
        "sirgas2000"                  => Some(Ellipsoid::GRS80.clone()),
        "ed50" | "european1950"       => Some(Ellipsoid::INTERNATIONAL.clone()),
        "osgb36"                      => Some(Ellipsoid::AIRY1830.clone()),
        "jgd2000" | "jgd2011"        => Some(Ellipsoid::GRS80.clone()),
        "ggrs87" | "greek"            => Some(Ellipsoid::GRS80.clone()),
        _                             => None,
    }
}

// ─── Datum resolution ─────────────────────────────────────────────────────────

fn resolve_datum(tokens: &HashMap<String, Option<String>>, ellipsoid: &Ellipsoid) -> Datum {
    // 1) Named datum via +datum=
    if let Some(Some(name)) = tokens.get("datum") {
        if let Some(d) = datum_from_proj_name(name) {
            return d;
        }
    }

    // 2) Helmert shift via +towgs84=dx,dy,dz[,rx,ry,rz,ds]
    if let Some(Some(tw)) = tokens.get("towgs84") {
        let parts: Vec<f64> = tw
            .split(',')
            .filter_map(|p| p.trim().parse::<f64>().ok())
            .collect();
        if parts.len() >= 7 {
            let hp = HelmertParams {
                tx: parts[0], ty: parts[1], tz: parts[2],
                rx: parts[3], ry: parts[4], rz: parts[5],
                ds: parts[6],
            };
            return Datum {
                name: "Custom",
                ellipsoid: ellipsoid.clone(),
                transform: DatumTransform::Helmert7(hp),
            };
        } else if parts.len() == 3 {
            return Datum {
                name: "Custom",
                ellipsoid: ellipsoid.clone(),
                transform: DatumTransform::Helmert3(HelmertParams::translation(parts[0], parts[1], parts[2])),
            };
        }
    }

    // 3) No explicit datum info → custom datum using the resolved ellipsoid
    Datum {
        name: "Custom",
        ellipsoid: ellipsoid.clone(),
        transform: DatumTransform::None,
    }
}

/// Map a PROJ `+datum=` name to a named [`Datum`] constant.
fn datum_from_proj_name(name: &str) -> Option<Datum> {
    match name.to_ascii_lowercase().as_str() {
        "wgs84"                        => Some(Datum::WGS84),
        "nad83" | "hpgn"              => Some(Datum::NAD83),
        "nad83_csrs" | "nad83csrs"    => Some(Datum::NAD83_CSRS),
        "nad27"                        => Some(Datum::NAD27),
        "etrs89" | "etrf89"           => Some(Datum::ETRS89),
        "gda94"                        => Some(Datum::GDA94),
        "gda2020"                      => Some(Datum::GDA2020),
        "sirgas2000"                   => Some(Datum::SIRGAS2000),
        "nzgd2000"                     => Some(Datum::NZGD2000),
        _                              => None,
    }
}

// ─── Unit resolution ──────────────────────────────────────────────────────────

fn resolve_units(tokens: &HashMap<String, Option<String>>) -> ParsedProjUnits {
    // +to_meter= takes precedence
    if let Some(tm) = parse_f64(tokens, "to_meter") {
        return ParsedProjUnits { to_meter: tm, label: None };
    }
    match tokens.get("units").and_then(|v| v.as_deref()).unwrap_or("m") {
        "m" | "metre" | "meters" | "meter" => ParsedProjUnits { to_meter: 1.0, label: Some("m") },
        "ft" | "foot"                       => ParsedProjUnits { to_meter: 0.304_8, label: Some("ft") },
        "us-ft" | "us_ft" | "surveyfeet"   => ParsedProjUnits { to_meter: 0.304_800_609_601_219, label: Some("us-ft") },
        "km"                                => ParsedProjUnits { to_meter: 1_000.0, label: Some("km") },
        "cm"                                => ParsedProjUnits { to_meter: 0.01, label: Some("cm") },
        "mm"                                => ParsedProjUnits { to_meter: 0.001, label: Some("mm") },
        "mi" | "mile"                       => ParsedProjUnits { to_meter: 1_609.344, label: Some("mi") },
        "link"                              => ParsedProjUnits { to_meter: 0.201_168, label: Some("link") },
        "chain"                             => ParsedProjUnits { to_meter: 20.116_8, label: Some("chain") },
        _                                   => ParsedProjUnits { to_meter: 1.0, label: Some("m") },
    }
}

// ─── Token helpers ────────────────────────────────────────────────────────────

fn parse_f64(tokens: &HashMap<String, Option<String>>, key: &str) -> Option<f64> {
    tokens.get(key)?.as_deref()?.parse::<f64>().ok()
}

/// Parse an angle (degrees) that may be:
/// - Plain decimal: `"-75.5"`
/// - Radians suffixed with `r`: `"1.3089969r"`
/// - PROJ DMS: `"78d30'"` or `"78d30'N"`
fn parse_angle(tokens: &HashMap<String, Option<String>>, key: &str) -> Option<f64> {
    parse_angle_str(tokens.get(key)?.as_deref()?)
}

fn parse_angle_str(s: &str) -> Option<f64> {
    if s.is_empty() {
        return None;
    }
    // Radians suffix
    if s.ends_with('r') || s.ends_with('R') {
        return s[..s.len() - 1].parse::<f64>().ok().map(|r| r.to_degrees());
    }
    // Check for DMS form (contains 'd' as degree separator)
    let lower = s.to_ascii_lowercase();
    if lower.contains('d') && !lower.starts_with("nan") {
        let neg = lower.starts_with('-');
        let s2 = lower.trim_start_matches('-');
        let mut parts = s2.splitn(2, 'd');
        let deg_s = parts.next()?;
        let deg: f64 = deg_s.parse().ok()?;
        let rest = parts.next().unwrap_or("").trim_end_matches(|c: char| c.is_ascii_alphabetic());
        let minutes = if let Some((m, _)) = rest.split_once('\'') {
            m.trim().parse::<f64>().unwrap_or(0.0)
        } else if rest.is_empty() {
            0.0
        } else {
            rest.trim().parse::<f64>().unwrap_or(0.0)
        } / 60.0;
        let v = deg + minutes;
        return Some(if neg { -v } else { v });
    }
    s.parse::<f64>().ok()
}

// ─── CRS name builder ─────────────────────────────────────────────────────────

fn build_name(
    tokens: &HashMap<String, Option<String>>,
    proj_name: &str,
    datum: &Datum,
) -> String {
    if let Some(Some(title)) = tokens.get("title") {
        return title.clone();
    }
    format!("{} / {}", datum.name, proj_name.to_ascii_uppercase())
}

// ─── Misc helpers ─────────────────────────────────────────────────────────────

fn missing_param(param: &str, proj: &str) -> ProjectionError {
    ProjectionError::InvalidParameter {
        param: param.into(),
        reason: format!("+proj={proj} requires +{param}="),
    }
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Crs {
        parse_crs_from_proj_string(s).unwrap_or_else(|e| panic!("parse failed: {e}"))
    }

    #[test]
    fn test_utm_nad83() {
        let crs = parse("+proj=utm +zone=17 +datum=NAD83 +units=m +no_defs");
        assert!(crs.name.contains("17N"));
        // Test round-trip: Toronto approx
        let (x, y) = crs.forward(-79.38, 43.65).unwrap();
        assert!((x - 630_000.0).abs() < 2_000.0, "easting {x}");
        assert!((y - 4_833_000.0).abs() < 2_000.0, "northing {y}");
        let (lon, lat) = crs.inverse(x, y).unwrap();
        assert!((lon - (-79.38)).abs() < 0.001);
        assert!((lat - 43.65).abs() < 0.001);
    }

    #[test]
    fn test_utm_south() {
        let crs = parse("+proj=utm +zone=34 +south +datum=WGS84");
        assert!(crs.name.contains("34S"));
        let (_, y) = crs.forward(21.0, -30.0).unwrap();
        // Southern hemisphere → northing > 5_000_000
        assert!(y > 5_000_000.0, "northing={y}");
    }

    #[test]
    fn test_lcc_two_sp() {
        let crs = parse("+proj=lcc +lat_1=49 +lat_2=77 +lat_0=49 +lon_0=-95 +x_0=0 +y_0=0 +datum=NAD83 +units=m");
        let (x, y) = crs.forward(-75.0, 55.0).unwrap();
        assert!(x.is_finite() && y.is_finite());
    }

    #[test]
    fn test_albers() {
        let crs = parse("+proj=aea +lat_1=29.5 +lat_2=45.5 +lat_0=23 +lon_0=-96 +x_0=0 +y_0=0 +datum=NAD83 +units=m");
        let (x, y) = crs.forward(-96.0, 37.5).unwrap();
        assert!(x.is_finite() && y.is_finite());
    }

    #[test]
    fn test_geographic() {
        let crs = parse("+proj=longlat +datum=WGS84 +no_defs");
        let (x, y) = crs.forward(-75.0, 45.0).unwrap();
        // Geographic pass-through: output = input
        assert!((x - (-75.0)).abs() < 1e-9);
        assert!((y - 45.0).abs() < 1e-9);
    }

    #[test]
    fn test_epsg_shortcut() {
        let crs = parse("+init=epsg:32617");
        assert!(crs.name.contains("17N") || crs.name.contains("17"), "name={}", crs.name);
    }

    #[test]
    fn test_epsg_bare() {
        let crs = parse("4326");
        assert!(crs.name.contains("4326") || crs.name.to_lowercase().contains("wgs"));
    }

    #[test]
    fn test_tmerc() {
        let crs = parse("+proj=tmerc +lat_0=0 +lon_0=-75 +k=0.9999 +x_0=304800 +y_0=0 +datum=NAD83 +units=m");
        let (x, y) = crs.forward(-75.0, 44.0).unwrap();
        assert!((x - 304_800.0).abs() < 2.0, "x={x}");
        assert!(y.is_finite());
    }

    #[test]
    fn test_custom_ellipsoid_a_b() {
        let crs = parse("+proj=tmerc +a=6378137 +b=6356752.3141 +lon_0=0 +x_0=0 +y_0=0 +k=1");
        let (x, y) = crs.forward(1.0, 0.0).unwrap();
        assert!(x.is_finite() && y.is_finite());
    }

    #[test]
    fn test_towgs84_three_param() {
        let crs = parse("+proj=tmerc +ellps=intl +towgs84=-87,-98,-121,0,0,0,0 +lon_0=0 +k=1 +x_0=0 +y_0=0");
        let p = crs.projection.params();
        // Should have International ellipsoid
        assert!((p.ellipsoid.a - 6_378_388.0).abs() < 10.0, "a={}", p.ellipsoid.a);
    }

    #[test]
    fn test_stere_polar_north() {
        let crs = parse("+proj=stere +lat_0=90 +lat_ts=71 +lon_0=0 +datum=WGS84");
        let (x, y) = crs.forward(0.0, 85.0).unwrap();
        assert!(x.is_finite() && y.is_finite());
    }

    #[test]
    fn test_units_parsed() {
        let parsed = parse_proj_string("+proj=tmerc +lon_0=0 +k=1 +x_0=0 +y_0=0 +datum=WGS84 +units=us-ft").unwrap();
        assert!((parsed.units.to_meter - 0.304_800_609_601_219).abs() < 1e-12);
    }

    #[test]
    fn test_unsupported_proj_error() {
        let err = parse_crs_from_proj_string("+proj=nzmg +datum=nzgd49");
        assert!(matches!(err, Err(ProjectionError::UnsupportedProjection(_))));
    }

    #[test]
    fn test_missing_utm_zone_error() {
        let err = parse_crs_from_proj_string("+proj=utm +datum=WGS84");
        assert!(matches!(err, Err(ProjectionError::InvalidParameter { .. })));
    }
}
