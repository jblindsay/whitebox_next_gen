//! Map projection implementations.
//!
//! Each projection lives in its own submodule and implements the common
//! projection interface via the `ProjectionImpl` trait.

mod aitoff;
mod albers;
mod axis_oriented;
mod azimuthal_equidistant;
mod behrmann;
mod bonne;
mod cassini;
mod central_conic;
mod collignon;
mod craster;
mod cylindrical_equal_area;
mod eckert_i;
mod eckert_ii;
mod eckert_iii;
mod eckert_iv;
mod eckert_v;
mod eckert_vi;
mod equirectangular;
mod equal_earth;
mod euler;
mod fahey;
mod foucaut;
mod gall_peters;
mod gall_stereographic;
mod geographic;
mod geocentric;
mod geostationary;
mod gnomonic;
mod hammer;
mod hatano;
mod hobo_dyer;
mod hotine_oblique_mercator;
mod lagrange;
mod kavrayskiy_v;
mod kavrayskiy_vii;
mod krovak;
mod lambert_azimuthal_equal_area;
mod lambert_conformal_conic;
mod loximuthal;
mod mercator;
mod miller_cylindrical;
mod murdoch_i;
mod murdoch_ii;
mod murdoch_iii;
mod mbt_s;
mod mbt_fps;
mod mbtfpp;
mod mbtfpq;
mod mollweide;
mod natural_earth;
mod natural_earth_ii;
mod nell;
mod nell_hammer;
mod orthographic;
mod patterson;
mod perspective_conic;
mod polar_stereographic;
mod polyconic;
mod putnins_p2;
mod putnins_p3;
mod putnins_p3p;
mod putnins_p4p;
mod putnins_p5;
mod putnins_p5p;
mod putnins_p6;
mod putnins_p6p;
mod putnins_p1;
mod quartic_authalic;
mod robinson;
mod sinusoidal;
mod stereographic;
mod times;
mod tissot;
mod two_point_equidistant;
mod tobler_mercator;
mod transverse_cylindrical_equal_area;
mod transverse_mercator;
mod van_der_grinten;
mod vertical;
mod wagner_i;
mod wagner_ii;
mod wagner_iii;
mod wagner_iv;
mod wagner_v;
mod wagner_vi;
mod werenskiold_i;
mod winkel_i;
mod winkel_ii;
mod winkel_tripel;
mod vitkovsky_i;

use crate::datum::Datum;
use crate::ellipsoid::Ellipsoid;
use crate::error::Result;
use crate::transform::{CoordTransform, Point2D};

/// Identifies which map projection algorithm to use.
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectionKind {
    /// Geographic lon/lat degrees (identity pass-through).
    Geographic,
    /// Geocentric Earth-Centered Earth-Fixed (ECEF) Cartesian XYZ.
    Geocentric,
    /// Geostationary satellite view (near-sided perspective from geostationary orbit).
    Geostationary {
        /// Satellite altitude above the ellipsoid in meters.
        satellite_height: f64,
        /// Sweep axis selection (`true` for x-sweep, `false` for y-sweep).
        sweep_x: bool,
    },
    /// Vertical CRS (height-only axis).
    Vertical,
    /// Mercator cylindrical conformal projection.
    Mercator,
    /// Web Mercator (Google/EPSG:3857), sphere-based.
    WebMercator,
    /// Transverse Mercator (basis for UTM).
    TransverseMercator,
    /// Transverse Mercator with south-orientated axis convention.
    TransverseMercatorSouthOrientated,
    /// Universal Transverse Mercator (zone + hemisphere).
    Utm {
        /// UTM longitudinal zone in the range 1..=60.
        zone: u8,
        /// `true` for southern hemisphere; `false` for northern.
        south: bool,
    },
    /// Lambert Conformal Conic with one or two standard parallels.
    LambertConformalConic {
        /// First standard parallel in degrees.
        lat1: f64,
        /// Optional second standard parallel in degrees.
        lat2: Option<f64>,
    },
    /// Albers Equal-Area Conic.
    AlbersEqualAreaConic {
        /// First standard parallel in degrees.
        lat1: f64,
        /// Second standard parallel in degrees.
        lat2: f64,
    },
    /// Azimuthal Equidistant.
    AzimuthalEquidistant,
    /// Two-Point Equidistant.
    TwoPointEquidistant {
        /// Longitude of first control point in degrees.
        lon1: f64,
        /// Latitude of first control point in degrees.
        lat1: f64,
        /// Longitude of second control point in degrees.
        lon2: f64,
        /// Latitude of second control point in degrees.
        lat2: f64,
    },
    /// Lambert Azimuthal Equal-Area.
    LambertAzimuthalEqualArea,
    /// Krovak oblique conformal conic (EPSG method 9819).
    Krovak,
    /// Hotine Oblique Mercator (azimuth at projection center).
    ///
    /// `rectified_grid_angle` is used by Rectified Skew Orthomorphic methods.
    /// If omitted, it is treated as equal to `azimuth`.
    HotineObliqueMercator {
        /// Projection azimuth at center in degrees.
        azimuth: f64,
        /// Optional rectified grid angle in degrees.
        rectified_grid_angle: Option<f64>,
    },
    /// Central Conic projection.
    CentralConic {
        /// Standard parallel in degrees.
        lat1: f64,
    },
    /// Lagrange projection.
    Lagrange {
        /// Standard parallel in degrees.
        lat1: f64,
        /// Lagrange projection parameter `w`.
        w: f64,
    },
    /// Loximuthal projection.
    Loximuthal {
        /// Reference latitude in degrees.
        lat1: f64,
    },
    /// Euler conic projection.
    Euler {
        /// First standard parallel in degrees.
        lat1: f64,
        /// Second standard parallel in degrees.
        lat2: f64,
    },
    /// Tissot conic projection.
    Tissot {
        /// First standard parallel in degrees.
        lat1: f64,
        /// Second standard parallel in degrees.
        lat2: f64,
    },
    /// Murdoch I conic projection.
    MurdochI {
        /// First standard parallel in degrees.
        lat1: f64,
        /// Second standard parallel in degrees.
        lat2: f64,
    },
    /// Murdoch II conic projection.
    MurdochII {
        /// First standard parallel in degrees.
        lat1: f64,
        /// Second standard parallel in degrees.
        lat2: f64,
    },
    /// Murdoch III conic projection.
    MurdochIII {
        /// First standard parallel in degrees.
        lat1: f64,
        /// Second standard parallel in degrees.
        lat2: f64,
    },
    /// Perspective Conic projection.
    PerspectiveConic {
        /// First standard parallel in degrees.
        lat1: f64,
        /// Second standard parallel in degrees.
        lat2: f64,
    },
    /// Vitkovsky I conic projection.
    VitkovskyI {
        /// First standard parallel in degrees.
        lat1: f64,
        /// Second standard parallel in degrees.
        lat2: f64,
    },
    /// Tobler-Mercator projection.
    ToblerMercator,
    /// Winkel II projection.
    WinkelII,
    /// Kavrayskiy V projection.
    KavrayskiyV,
    /// Stereographic.
    Stereographic,
    /// Oblique Stereographic.
    ObliqueStereographic,
    /// Polar Stereographic (ellipsoidal).
    ///
    /// `north = true` for north-pole aspect, `false` for south-pole aspect.
    /// `lat_ts = Some(phi)` uses the given standard parallel (scale = 1 there, Variant B).
    /// `lat_ts = None` uses `ProjectionParams::scale` as k₀ at the pole (Variant A).
    PolarStereographic {
        /// `true` for north-pole aspect, `false` for south-pole aspect.
        north: bool,
        /// Optional latitude of true scale in degrees.
        lat_ts: Option<f64>,
    },
    /// Orthographic.
    Orthographic,
    /// Sinusoidal pseudocylindrical.
    Sinusoidal,
    /// Mollweide pseudocylindrical.
    Mollweide,
    /// McBryde-Thomas Flat-Pole Sine (No. 2).
    MbtFps,
    /// McBryde-Thomas Flat-Polar Sine (No. 1).
    MbtS,
    /// McBryde-Thomas Flat-Polar Parabolic.
    Mbtfpp,
    /// McBryde-Thomas Flat-Polar Quartic.
    Mbtfpq,
    /// Nell projection.
    Nell,
    /// Equal Earth pseudocylindrical equal-area.
    EqualEarth,
    /// Lambert Cylindrical Equal-Area.
    CylindricalEqualArea {
        /// Latitude of true scale in degrees.
        lat_ts: f64,
    },
    /// Equirectangular (Plate Carrée).
    Equirectangular {
        /// Standard parallel in degrees.
        lat_ts: f64,
    },
    /// Robinson pseudocylindrical.
    Robinson,
    /// Gnomonic azimuthal projection.
    Gnomonic,
    /// Aitoff compromise projection.
    Aitoff,
    /// Van der Grinten I world projection.
    VanDerGrinten,
    /// Winkel Tripel world projection.
    WinkelTripel,
    /// Hammer equal-area world projection.
    Hammer,
    /// Hatano Asymmetrical Equal Area projection.
    Hatano,
    /// Eckert I pseudocylindrical world projection.
    EckertI,
    /// Eckert II pseudocylindrical world projection.
    EckertII,
    /// Eckert III pseudocylindrical world projection.
    EckertIII,
    /// Eckert IV equal-area world projection.
    EckertIV,
    /// Eckert V pseudocylindrical world projection.
    EckertV,
    /// Miller cylindrical projection.
    MillerCylindrical,
    /// Gall stereographic projection.
    GallStereographic,
    /// Gall-Peters equal-area cylindrical projection.
    GallPeters,
    /// Behrmann equal-area cylindrical projection.
    Behrmann,
    /// Hobo-Dyer equal-area cylindrical projection.
    HoboDyer,
    /// Wagner I projection.
    WagnerI,
    /// Wagner II projection.
    WagnerII,
    /// Wagner III projection.
    WagnerIII,
    /// Wagner IV projection.
    WagnerIV,
    /// Wagner V projection.
    WagnerV,
    /// Natural Earth projection.
    NaturalEarth,
    /// Natural Earth II projection.
    NaturalEarthII,
    /// Wagner VI projection.
    WagnerVI,
    /// Eckert VI projection.
    EckertVI,
    /// Transverse Cylindrical Equal Area.
    TransverseCylindricalEqualArea,
    /// American Polyconic projection.
    Polyconic,
    /// Cassini-Soldner projection.
    Cassini,
    /// Bonne projection.
    Bonne,
    /// Bonne with south-orientated axis convention.
    BonneSouthOrientated,
    /// Craster Parabolic (Putnins P4) projection.
    Craster,
    /// Putnins P4' projection.
    PutninsP4p,
    /// Fahey projection.
    Fahey,
    /// Times projection.
    Times,
    /// Patterson cylindrical projection.
    Patterson,
    /// Putnins P3 projection.
    PutninsP3,
    /// Putnins P3' projection.
    PutninsP3p,
    /// Putnins P5 projection.
    PutninsP5,
    /// Putnins P5' projection.
    PutninsP5p,
    /// Putnins P1 projection.
    PutninsP1,
    /// Putnins P2 projection.
    PutninsP2,
    /// Putnins P6 projection.
    PutninsP6,
    /// Putnins P6' projection.
    PutninsP6p,
    /// Quartic Authalic projection.
    QuarticAuthalic,
    /// Foucaut projection.
    Foucaut,
    /// Winkel I projection.
    WinkelI,
    /// Werenskiold I projection.
    WerenskioldI,
    /// Collignon equal-area projection.
    Collignon,
    /// Nell-Hammer projection.
    NellHammer,
    /// Kavrayskiy VII projection.
    KavrayskiyVII,
}

/// Parameters for constructing a projection.
#[derive(Debug, Clone)]
pub struct ProjectionParams {
    /// Which projection algorithm to use.
    pub kind: ProjectionKind,
    /// Central longitude (degrees). Default: 0.
    pub lon0: f64,
    /// Central latitude (degrees). Default: 0.
    pub lat0: f64,
    /// False easting (meters). Default: 0.
    pub false_easting: f64,
    /// False northing (meters). Default: 0.
    pub false_northing: f64,
    /// Scale factor at natural origin. Default: 1.
    pub scale: f64,
    /// Reference ellipsoid.
    pub ellipsoid: Ellipsoid,
    /// Datum (includes ellipsoid; overrides `ellipsoid` if set explicitly).
    pub datum: Datum,
}

impl Default for ProjectionParams {
    fn default() -> Self {
        ProjectionParams {
            kind: ProjectionKind::Equirectangular { lat_ts: 0.0 },
            lon0: 0.0,
            lat0: 0.0,
            false_easting: 0.0,
            false_northing: 0.0,
            scale: 1.0,
            ellipsoid: Ellipsoid::WGS84,
            datum: Datum::WGS84,
        }
    }
}

impl ProjectionParams {
    /// Create params for a specific projection kind with WGS84 ellipsoid.
    pub fn new(kind: ProjectionKind) -> Self {
        ProjectionParams {
            kind,
            ..Default::default()
        }
    }

    /// Create UTM projection params for the given zone and hemisphere.
    pub fn utm(zone: u8, south: bool) -> Self {
        let lon0 = (zone as f64 - 1.0) * 6.0 - 180.0 + 3.0;
        ProjectionParams {
            kind: ProjectionKind::Utm { zone, south },
            lon0,
            lat0: 0.0,
            false_easting: 500_000.0,
            false_northing: if south { 10_000_000.0 } else { 0.0 },
            scale: 0.9996,
            ellipsoid: Ellipsoid::WGS84,
            datum: Datum::WGS84,
        }
    }

    /// Create Web Mercator (EPSG:3857) params.
    pub fn web_mercator() -> Self {
        ProjectionParams {
            kind: ProjectionKind::WebMercator,
            lon0: 0.0,
            lat0: 0.0,
            false_easting: 0.0,
            false_northing: 0.0,
            scale: 1.0,
            ellipsoid: Ellipsoid::SPHERE,
            datum: Datum::WGS84,
        }
    }

    /// Set central longitude.
    pub fn with_lon0(mut self, lon0: f64) -> Self {
        self.lon0 = lon0;
        self
    }

    /// Set central latitude.
    pub fn with_lat0(mut self, lat0: f64) -> Self {
        self.lat0 = lat0;
        self
    }

    /// Set false easting.
    pub fn with_false_easting(mut self, fe: f64) -> Self {
        self.false_easting = fe;
        self
    }

    /// Set false northing.
    pub fn with_false_northing(mut self, fn_: f64) -> Self {
        self.false_northing = fn_;
        self
    }

    /// Set scale factor.
    pub fn with_scale(mut self, k: f64) -> Self {
        self.scale = k;
        self
    }

    /// Set the ellipsoid.
    pub fn with_ellipsoid(mut self, ellipsoid: Ellipsoid) -> Self {
        self.ellipsoid = ellipsoid.clone();
        self.datum = Datum {
            name: "Custom",
            ellipsoid,
            transform: crate::datum::DatumTransform::None,
        };
        self
    }
}

/// Internal trait implemented by each projection algorithm.
trait ProjectionImpl: Send + Sync {
    /// Forward projection: (lon_deg, lat_deg) → (x_m, y_m).
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)>;

    /// Inverse projection: (x_m, y_m) → (lon_deg, lat_deg).
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)>;
}

/// A configured map projection ready for forward and inverse transformations.
pub struct Projection {
    params: ProjectionParams,
    inner: Box<dyn ProjectionImpl>,
}

impl std::fmt::Debug for Projection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Projection")
            .field("params", &self.params)
            .finish()
    }
}

impl Projection {
    /// Build a [`Projection`] from the given parameters.
    pub fn new(params: ProjectionParams) -> Result<Self> {
        let inner: Box<dyn ProjectionImpl> = match &params.kind {
            ProjectionKind::Geographic => Box::new(
                geographic::GeographicProj::new(&params)?,
            ),
            ProjectionKind::Geocentric => Box::new(
                geocentric::GeocentricProj::new(&params)?,
            ),
            ProjectionKind::Geostationary {
                satellite_height,
                sweep_x,
            } => Box::new(
                geostationary::GeostationaryProj::new(&params, *satellite_height, *sweep_x)?,
            ),
            ProjectionKind::Vertical => Box::new(
                vertical::VerticalProj::new(&params)?,
            ),
            ProjectionKind::Mercator => Box::new(
                mercator::MercatorProj::new(&params)?,
            ),
            ProjectionKind::WebMercator => Box::new(
                mercator::WebMercatorProj::new(&params)?,
            ),
            ProjectionKind::TransverseMercator => Box::new(
                transverse_mercator::TransverseMercatorProj::new(&params)?,
            ),
            ProjectionKind::TransverseMercatorSouthOrientated => Box::new(
                axis_oriented::AxisOrientedProj::new(
                    Box::new(transverse_mercator::TransverseMercatorProj::new(&params)?),
                    params.false_easting,
                    params.false_northing,
                    true,
                    true,
                ),
            ),
            ProjectionKind::Utm { zone, south } => {
                let _ = (zone, south); // Already baked into params
                Box::new(transverse_mercator::TransverseMercatorProj::new(&params)?)
            }
            ProjectionKind::LambertConformalConic { lat1, lat2 } => Box::new(
                lambert_conformal_conic::LccProj::new(&params, *lat1, *lat2)?,
            ),
            ProjectionKind::AlbersEqualAreaConic { lat1, lat2 } => Box::new(
                albers::AlbersProj::new(&params, *lat1, *lat2)?,
            ),
            ProjectionKind::AzimuthalEquidistant => Box::new(
                azimuthal_equidistant::AzimuthalEquidistantProj::new(&params)?,
            ),
            ProjectionKind::TwoPointEquidistant { lon1, lat1, lon2, lat2 } => Box::new(
                two_point_equidistant::TwoPointEquidistantProj::new(&params, *lon1, *lat1, *lon2, *lat2)?,
            ),
            ProjectionKind::LambertAzimuthalEqualArea => Box::new(
                lambert_azimuthal_equal_area::LambertAzimuthalEqualAreaProj::new(&params)?,
            ),
            ProjectionKind::Krovak => Box::new(
                krovak::KrovakProj::new(&params)?,
            ),
            ProjectionKind::HotineObliqueMercator {
                azimuth,
                rectified_grid_angle,
            } => Box::new(
                hotine_oblique_mercator::HotineObliqueMercatorProj::new(
                    &params,
                    *azimuth,
                    rectified_grid_angle.unwrap_or(*azimuth),
                )?,
            ),
            ProjectionKind::CentralConic { lat1 } => Box::new(
                central_conic::CentralConicProj::new(&params, *lat1)?,
            ),
            ProjectionKind::Lagrange { lat1, w } => Box::new(
                lagrange::LagrangeProj::new(&params, *lat1, *w)?,
            ),
            ProjectionKind::Loximuthal { lat1 } => Box::new(
                loximuthal::LoximuthalProj::new(&params, *lat1)?,
            ),
            ProjectionKind::Euler { lat1, lat2 } => Box::new(
                euler::EulerProj::new(&params, *lat1, *lat2)?,
            ),
            ProjectionKind::Tissot { lat1, lat2 } => Box::new(
                tissot::TissotProj::new(&params, *lat1, *lat2)?,
            ),
            ProjectionKind::MurdochI { lat1, lat2 } => Box::new(
                murdoch_i::MurdochIProj::new(&params, *lat1, *lat2)?,
            ),
            ProjectionKind::MurdochII { lat1, lat2 } => Box::new(
                murdoch_ii::MurdochIIProj::new(&params, *lat1, *lat2)?,
            ),
            ProjectionKind::MurdochIII { lat1, lat2 } => Box::new(
                murdoch_iii::MurdochIIIProj::new(&params, *lat1, *lat2)?,
            ),
            ProjectionKind::PerspectiveConic { lat1, lat2 } => Box::new(
                perspective_conic::PerspectiveConicProj::new(&params, *lat1, *lat2)?,
            ),
            ProjectionKind::VitkovskyI { lat1, lat2 } => Box::new(
                vitkovsky_i::VitkovskyIProj::new(&params, *lat1, *lat2)?,
            ),
            ProjectionKind::ToblerMercator => Box::new(
                tobler_mercator::ToblerMercatorProj::new(&params)?,
            ),
            ProjectionKind::WinkelII => Box::new(
                winkel_ii::WinkelIIProj::new(&params)?,
            ),
            ProjectionKind::KavrayskiyV => Box::new(
                kavrayskiy_v::KavrayskiyVProj::new(&params)?,
            ),
            ProjectionKind::Stereographic => Box::new(
                stereographic::StereographicProj::new(&params)?,
            ),
            ProjectionKind::PolarStereographic { north, lat_ts } => Box::new(
                polar_stereographic::PolarStereographicProj::new(&params, *north, *lat_ts)?,
            ),
            ProjectionKind::ObliqueStereographic => Box::new(
                stereographic::StereographicProj::new(&params)?,
            ),
            ProjectionKind::Orthographic => Box::new(
                orthographic::OrthographicProj::new(&params)?,
            ),
            ProjectionKind::Sinusoidal => Box::new(
                sinusoidal::SinusoidalProj::new(&params)?,
            ),
            ProjectionKind::Mollweide => Box::new(
                mollweide::MollweideProj::new(&params)?,
            ),
            ProjectionKind::MbtFps => Box::new(
                mbt_fps::MbtFpsProj::new(&params)?,
            ),
            ProjectionKind::MbtS => Box::new(
                mbt_s::MbtSProj::new(&params)?,
            ),
            ProjectionKind::Mbtfpp => Box::new(
                mbtfpp::MbtfppProj::new(&params)?,
            ),
            ProjectionKind::Mbtfpq => Box::new(
                mbtfpq::MbtfpqProj::new(&params)?,
            ),
            ProjectionKind::Nell => Box::new(
                nell::NellProj::new(&params)?,
            ),
            ProjectionKind::EqualEarth => Box::new(
                equal_earth::EqualEarthProj::new(&params)?,
            ),
            ProjectionKind::MillerCylindrical => Box::new(
                miller_cylindrical::MillerCylindricalProj::new(&params)?,
            ),
            ProjectionKind::GallStereographic => Box::new(
                gall_stereographic::GallStereographicProj::new(&params)?,
            ),
            ProjectionKind::GallPeters => Box::new(
                gall_peters::GallPetersProj::new(&params)?,
            ),
            ProjectionKind::Behrmann => Box::new(
                behrmann::BehrmannProj::new(&params)?,
            ),
            ProjectionKind::HoboDyer => Box::new(
                hobo_dyer::HoboDyerProj::new(&params)?,
            ),
            ProjectionKind::WagnerI => Box::new(
                wagner_i::WagnerIProj::new(&params)?,
            ),
            ProjectionKind::WagnerII => Box::new(
                wagner_ii::WagnerIiProj::new(&params)?,
            ),
            ProjectionKind::WagnerIII => Box::new(
                wagner_iii::WagnerIiiProj::new(&params)?,
            ),
            ProjectionKind::WagnerIV => Box::new(
                wagner_iv::WagnerIvProj::new(&params)?,
            ),
            ProjectionKind::WagnerV => Box::new(
                wagner_v::WagnerVProj::new(&params)?,
            ),
            ProjectionKind::NaturalEarth => Box::new(
                natural_earth::NaturalEarthProj::new(&params)?,
            ),
            ProjectionKind::NaturalEarthII => Box::new(
                natural_earth_ii::NaturalEarthIIProj::new(&params)?,
            ),
            ProjectionKind::WagnerVI => Box::new(
                wagner_vi::WagnerViProj::new(&params)?,
            ),
            ProjectionKind::EckertVI => Box::new(
                eckert_vi::EckertViProj::new(&params)?,
            ),
            ProjectionKind::TransverseCylindricalEqualArea => Box::new(
                transverse_cylindrical_equal_area::TransverseCylindricalEqualAreaProj::new(&params)?,
            ),
            ProjectionKind::Polyconic => Box::new(
                polyconic::PolyconicProj::new(&params)?,
            ),
            ProjectionKind::Cassini => Box::new(
                cassini::CassiniProj::new(&params)?,
            ),
            ProjectionKind::Bonne => Box::new(
                bonne::BonneProj::new(&params)?,
            ),
            ProjectionKind::BonneSouthOrientated => Box::new(
                axis_oriented::AxisOrientedProj::new(
                    Box::new(bonne::BonneProj::new(&params)?),
                    params.false_easting,
                    params.false_northing,
                    true,
                    true,
                ),
            ),
            ProjectionKind::Craster => Box::new(
                craster::CrasterProj::new(&params)?,
            ),
            ProjectionKind::PutninsP4p => Box::new(
                putnins_p4p::PutninsP4pProj::new(&params)?,
            ),
            ProjectionKind::Fahey => Box::new(
                fahey::FaheyProj::new(&params)?,
            ),
            ProjectionKind::Times => Box::new(
                times::TimesProj::new(&params)?,
            ),
            ProjectionKind::Patterson => Box::new(
                patterson::PattersonProj::new(&params)?,
            ),
            ProjectionKind::PutninsP3 => Box::new(
                putnins_p3::PutninsP3Proj::new(&params)?,
            ),
            ProjectionKind::PutninsP3p => Box::new(
                putnins_p3p::PutninsP3pProj::new(&params)?,
            ),
            ProjectionKind::PutninsP5 => Box::new(
                putnins_p5::PutninsP5Proj::new(&params)?,
            ),
            ProjectionKind::PutninsP5p => Box::new(
                putnins_p5p::PutninsP5pProj::new(&params)?,
            ),
            ProjectionKind::PutninsP1 => Box::new(
                putnins_p1::PutninsP1Proj::new(&params)?,
            ),
            ProjectionKind::PutninsP2 => Box::new(
                putnins_p2::PutninsP2Proj::new(&params)?,
            ),
            ProjectionKind::PutninsP6 => Box::new(
                putnins_p6::PutninsP6Proj::new(&params)?,
            ),
            ProjectionKind::PutninsP6p => Box::new(
                putnins_p6p::PutninsP6pProj::new(&params)?,
            ),
            ProjectionKind::QuarticAuthalic => Box::new(
                quartic_authalic::QuarticAuthalicProj::new(&params)?,
            ),
            ProjectionKind::Foucaut => Box::new(
                foucaut::FoucautProj::new(&params)?,
            ),
            ProjectionKind::WinkelI => Box::new(
                winkel_i::WinkelIProj::new(&params)?,
            ),
            ProjectionKind::WerenskioldI => Box::new(
                werenskiold_i::WerenskioldIProj::new(&params)?,
            ),
            ProjectionKind::CylindricalEqualArea { lat_ts } => Box::new(
                cylindrical_equal_area::CylindricalEqualAreaProj::new(&params, *lat_ts)?,
            ),
            ProjectionKind::Equirectangular { lat_ts } => Box::new(
                equirectangular::EquirectangularProj::new(&params, *lat_ts)?,
            ),
            ProjectionKind::Robinson => Box::new(
                robinson::RobinsonProj::new(&params)?,
            ),
            ProjectionKind::Gnomonic => Box::new(
                gnomonic::GnomonicProj::new(&params)?,
            ),
            ProjectionKind::Aitoff => Box::new(
                aitoff::AitoffProj::new(&params)?,
            ),
            ProjectionKind::VanDerGrinten => Box::new(
                van_der_grinten::VanDerGrintenProj::new(&params)?,
            ),
            ProjectionKind::WinkelTripel => Box::new(
                winkel_tripel::WinkelTripelProj::new(&params)?,
            ),
            ProjectionKind::Hammer => Box::new(
                hammer::HammerProj::new(&params)?,
            ),
            ProjectionKind::Hatano => Box::new(
                hatano::HatanoProj::new(&params)?,
            ),
            ProjectionKind::EckertI => Box::new(
                eckert_i::EckertIProj::new(&params)?,
            ),
            ProjectionKind::EckertII => Box::new(
                eckert_ii::EckertIiProj::new(&params)?,
            ),
            ProjectionKind::EckertIII => Box::new(
                eckert_iii::EckertIiiProj::new(&params)?,
            ),
            ProjectionKind::EckertIV => Box::new(
                eckert_iv::EckertIvProj::new(&params)?,
            ),
            ProjectionKind::EckertV => Box::new(
                eckert_v::EckertVProj::new(&params)?,
            ),
            ProjectionKind::Collignon => Box::new(
                collignon::CollignonProj::new(&params)?,
            ),
            ProjectionKind::NellHammer => Box::new(
                nell_hammer::NellHammerProj::new(&params)?,
            ),
            ProjectionKind::KavrayskiyVII => Box::new(
                kavrayskiy_vii::KavrayskiyViiProj::new(&params)?,
            ),
        };

        Ok(Projection { params, inner })
    }

    /// Forward projection: geographic (lon, lat) degrees → projected (x, y) meters.
    pub fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        self.inner.forward(lon_deg, lat_deg)
    }

    /// Inverse projection: projected (x, y) meters → geographic (lon, lat) degrees.
    pub fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        self.inner.inverse(x, y)
    }

    /// Return a reference to the projection parameters.
    pub fn params(&self) -> &ProjectionParams {
        &self.params
    }

    /// Return the name of the projection.
    pub fn name(&self) -> &str {
        match &self.params.kind {
            ProjectionKind::Geographic => "Geographic",
            ProjectionKind::Geocentric => "Geocentric",
            ProjectionKind::Geostationary { .. } => "Geostationary Satellite View",
            ProjectionKind::Vertical => "Vertical",
            ProjectionKind::Mercator => "Mercator",
            ProjectionKind::WebMercator => "Web Mercator",
            ProjectionKind::TransverseMercator => "Transverse Mercator",
            ProjectionKind::TransverseMercatorSouthOrientated => "Transverse Mercator South Orientated",
            ProjectionKind::Utm { zone, south } => {
                let _ = (zone, south);
                "UTM"
            }
            ProjectionKind::LambertConformalConic { .. } => "Lambert Conformal Conic",
            ProjectionKind::AlbersEqualAreaConic { .. } => "Albers Equal-Area Conic",
            ProjectionKind::AzimuthalEquidistant => "Azimuthal Equidistant",
            ProjectionKind::TwoPointEquidistant { .. } => "Two-Point Equidistant",
            ProjectionKind::LambertAzimuthalEqualArea => "Lambert Azimuthal Equal-Area",
            ProjectionKind::Krovak => "Krovak",
            ProjectionKind::HotineObliqueMercator { .. } => "Hotine Oblique Mercator",
            ProjectionKind::CentralConic { .. } => "Central Conic",
            ProjectionKind::Lagrange { .. } => "Lagrange",
            ProjectionKind::Loximuthal { .. } => "Loximuthal",
            ProjectionKind::Euler { .. } => "Euler",
            ProjectionKind::Tissot { .. } => "Tissot",
            ProjectionKind::MurdochI { .. } => "Murdoch I",
            ProjectionKind::MurdochII { .. } => "Murdoch II",
            ProjectionKind::MurdochIII { .. } => "Murdoch III",
            ProjectionKind::PerspectiveConic { .. } => "Perspective Conic",
            ProjectionKind::VitkovskyI { .. } => "Vitkovsky I",
            ProjectionKind::ToblerMercator => "Tobler-Mercator",
            ProjectionKind::WinkelII => "Winkel II",
            ProjectionKind::KavrayskiyV => "Kavrayskiy V",
            ProjectionKind::Stereographic => "Stereographic",
            ProjectionKind::PolarStereographic { north, .. } => {
                if *north { "Polar Stereographic (North)" } else { "Polar Stereographic (South)" }
            }
            ProjectionKind::ObliqueStereographic => "Oblique Stereographic",
            ProjectionKind::Orthographic => "Orthographic",
            ProjectionKind::Sinusoidal => "Sinusoidal",
            ProjectionKind::Mollweide => "Mollweide",
            ProjectionKind::MbtFps => "McBryde-Thomas Flat-Pole Sine (No. 2)",
            ProjectionKind::MbtS => "McBryde-Thomas Flat-Polar Sine (No. 1)",
            ProjectionKind::Mbtfpp => "McBryde-Thomas Flat-Polar Parabolic",
            ProjectionKind::Mbtfpq => "McBryde-Thomas Flat-Polar Quartic",
            ProjectionKind::Nell => "Nell",
            ProjectionKind::EqualEarth => "Equal Earth",
            ProjectionKind::CylindricalEqualArea { .. } => "Cylindrical Equal Area",
            ProjectionKind::Equirectangular { .. } => "Equirectangular",
            ProjectionKind::Robinson => "Robinson",
            ProjectionKind::Gnomonic => "Gnomonic",
            ProjectionKind::Aitoff => "Aitoff",
            ProjectionKind::VanDerGrinten => "Van der Grinten",
            ProjectionKind::WinkelTripel => "Winkel Tripel",
            ProjectionKind::Hammer => "Hammer",
            ProjectionKind::Hatano => "Hatano",
            ProjectionKind::EckertI => "Eckert I",
            ProjectionKind::EckertII => "Eckert II",
            ProjectionKind::EckertIII => "Eckert III",
            ProjectionKind::EckertIV => "Eckert IV",
            ProjectionKind::EckertV => "Eckert V",
            ProjectionKind::MillerCylindrical => "Miller Cylindrical",
            ProjectionKind::GallStereographic => "Gall Stereographic",
            ProjectionKind::GallPeters => "Gall-Peters",
            ProjectionKind::Behrmann => "Behrmann",
            ProjectionKind::HoboDyer => "Hobo-Dyer",
            ProjectionKind::WagnerI => "Wagner I",
            ProjectionKind::WagnerII => "Wagner II",
            ProjectionKind::WagnerIII => "Wagner III",
            ProjectionKind::WagnerIV => "Wagner IV",
            ProjectionKind::WagnerV => "Wagner V",
            ProjectionKind::NaturalEarth => "Natural Earth",
            ProjectionKind::NaturalEarthII => "Natural Earth II",
            ProjectionKind::WagnerVI => "Wagner VI",
            ProjectionKind::EckertVI => "Eckert VI",
            ProjectionKind::TransverseCylindricalEqualArea => "Transverse Cylindrical Equal Area",
            ProjectionKind::Polyconic => "Polyconic",
            ProjectionKind::Cassini => "Cassini-Soldner",
            ProjectionKind::Bonne => "Bonne",
            ProjectionKind::BonneSouthOrientated => "Bonne South Orientated",
            ProjectionKind::Craster => "Craster",
            ProjectionKind::PutninsP4p => "Putnins P4'",
            ProjectionKind::Fahey => "Fahey",
            ProjectionKind::Times => "Times",
            ProjectionKind::Patterson => "Patterson",
            ProjectionKind::PutninsP3 => "Putnins P3",
            ProjectionKind::PutninsP3p => "Putnins P3'",
            ProjectionKind::PutninsP5 => "Putnins P5",
            ProjectionKind::PutninsP5p => "Putnins P5'",
            ProjectionKind::PutninsP1 => "Putnins P1",
            ProjectionKind::PutninsP2 => "Putnins P2",
            ProjectionKind::PutninsP6 => "Putnins P6",
            ProjectionKind::PutninsP6p => "Putnins P6'",
            ProjectionKind::QuarticAuthalic => "Quartic Authalic",
            ProjectionKind::Foucaut => "Foucaut",
            ProjectionKind::WinkelI => "Winkel I",
            ProjectionKind::WerenskioldI => "Werenskiold I",
            ProjectionKind::Collignon => "Collignon",
            ProjectionKind::NellHammer => "Nell-Hammer",
            ProjectionKind::KavrayskiyVII => "Kavrayskiy VII",
        }
    }
}

impl CoordTransform for Projection {
    fn transform_fwd(&self, point: Point2D) -> Result<Point2D> {
        let (x, y) = self.forward(point.x, point.y)?;
        Ok(Point2D::new(x, y))
    }

    fn transform_inv(&self, point: Point2D) -> Result<Point2D> {
        let (lon, lat) = self.inverse(point.x, point.y)?;
        Ok(Point2D::new(lon, lat))
    }
}
