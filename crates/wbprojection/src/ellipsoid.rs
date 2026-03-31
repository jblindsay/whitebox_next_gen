//! Reference ellipsoid definitions.
//!
//! An ellipsoid (or spheroid) is the mathematical model used to approximate
//! the shape of the Earth. Different geodetic datums use different ellipsoids.

/// A reference ellipsoid, defined by its semi-major axis and flattening.
#[derive(Debug, Clone, PartialEq)]
pub struct Ellipsoid {
    /// Name of the ellipsoid.
    pub name: &'static str,
    /// Semi-major axis in meters (equatorial radius).
    pub a: f64,
    /// Semi-minor axis in meters (polar radius).
    pub b: f64,
    /// Flattening: f = (a - b) / a
    pub f: f64,
    /// First eccentricity squared: e² = 1 - (b/a)²
    pub e2: f64,
    /// First eccentricity: e
    pub e: f64,
    /// Second eccentricity squared: e'² = (a/b)² - 1
    pub ep2: f64,
}

impl Ellipsoid {
    /// Construct an ellipsoid from semi-major axis and inverse flattening.
    pub fn from_a_inv_f(name: &'static str, a: f64, inv_f: f64) -> Self {
        let f = 1.0 / inv_f;
        let b = a * (1.0 - f);
        let e2 = 2.0 * f - f * f;
        let e = e2.sqrt();
        let ep2 = e2 / (1.0 - e2);
        Ellipsoid { name, a, b, f, e2, e, ep2 }
    }

    /// Construct a sphere with the given radius.
    pub fn sphere(name: &'static str, radius: f64) -> Self {
        Ellipsoid {
            name,
            a: radius,
            b: radius,
            f: 0.0,
            e2: 0.0,
            e: 0.0,
            ep2: 0.0,
        }
    }

    /// Returns true if this is a sphere (zero flattening).
    pub fn is_sphere(&self) -> bool {
        self.f == 0.0
    }

    /// List of supported standard ellipsoid names for lookup.
    pub fn standard_names() -> &'static [&'static str] {
        &[
            "WGS 84",
            "WGS 72",
            "GRS 80",
            "GRS 67",
            "Clarke 1866",
            "Clarke 1880 (RGS)",
            "International 1924",
            "Bessel 1841",
            "Airy 1830",
            "Airy 1830 Modified",
            "Krassowsky 1940",
            "IAU 1976",
            "Everest 1830",
            "Helmert 1906",
            "Australian National Spheroid",
            "Fischer 1960",
            "Sphere",
        ]
    }

    /// Lookup a standard ellipsoid by name or common alias (case-insensitive).
    pub fn from_name(name: &str) -> Option<Self> {
        let key = name
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();

        match key.as_str() {
            "wgs84" => Some(Self::WGS84.clone()),
            "wgs72" => Some(Self::from_a_inv_f("WGS 72", 6_378_135.0, 298.26)),
            "grs80" => Some(Self::GRS80.clone()),
            "grs67" => Some(Self::from_a_inv_f("GRS 67", 6_378_160.0, 298.247_167_427)),
            "clarke1866" => Some(Self::CLARKE1866.clone()),
            "clarke1880rgs" | "clarke1880" => Some(Self::CLARKE1880_RGS.clone()),
            "international1924" | "international" | "hayford" => {
                Some(Self::INTERNATIONAL.clone())
            }
            "bessel1841" | "bessel" => Some(Self::BESSEL.clone()),
            "airy1830" | "airy" => Some(Self::AIRY1830.clone()),
            "airy1830modified" | "airy1830mod" | "airymodified" => {
                Some(Self::AIRY1830_MOD.clone())
            }
            "krassowsky1940" | "krassovsky1940" | "krassowsky" | "krassovsky" => {
                Some(Self::KRASSOWSKY1940.clone())
            }
            "iau1976" | "iau76" => Some(Self::IAU1976.clone()),
            "everest1830" | "everest" => Some(Self::EVEREST1830.clone()),
            "helmert1906" | "helmert" => Some(Self::HELMERT1906.clone()),
            "australiannationalspheroid" | "ans" => {
                Some(Self::from_a_inv_f("Australian National Spheroid", 6_378_160.0, 298.25))
            }
            "fischer1960" | "fischer" => {
                Some(Self::from_a_inv_f("Fischer 1960", 6_378_166.0, 298.3))
            }
            "sphere" => Some(Self::SPHERE.clone()),
            _ => None,
        }
    }

    /// Lookup an ellipsoid from a common EPSG ellipsoid code.
    pub fn from_epsg_ellipsoid(code: u16) -> Option<Self> {
        match code {
            7030 => Some(Self::WGS84.clone()),
            7043 => Self::from_name("WGS 72"),
            7019 => Some(Self::GRS80.clone()),
            7048 => Self::from_name("GRS 67"),
            7008 => Some(Self::CLARKE1866.clone()),
            7012 => Some(Self::CLARKE1880_RGS.clone()),
            7022 => Some(Self::INTERNATIONAL.clone()),
            7004 => Some(Self::BESSEL.clone()),
            7001 => Some(Self::AIRY1830.clone()),
            7002 => Some(Self::AIRY1830_MOD.clone()),
            7024 => Some(Self::KRASSOWSKY1940.clone()),
            7049 => Some(Self::IAU1976.clone()),
            7015 => Some(Self::EVEREST1830.clone()),
            _ => None,
        }
    }

    /// Compute the radius of curvature in the meridian at geodetic latitude φ.
    pub fn meridian_radius(&self, lat_rad: f64) -> f64 {
        let sin_lat = lat_rad.sin();
        let denom = (1.0 - self.e2 * sin_lat * sin_lat).powf(1.5);
        self.a * (1.0 - self.e2) / denom
    }

    /// Compute the radius of curvature in the prime vertical at geodetic latitude φ.
    pub fn normal_radius(&self, lat_rad: f64) -> f64 {
        let sin_lat = lat_rad.sin();
        let denom = (1.0 - self.e2 * sin_lat * sin_lat).sqrt();
        self.a / denom
    }

    /// Mean radius (arithmetic mean of a, a, b).
    pub fn mean_radius(&self) -> f64 {
        (2.0 * self.a + self.b) / 3.0
    }

    /// Authalic sphere radius (equal-area sphere).
    pub fn authalic_radius(&self) -> f64 {
        // Rq = a * sqrt(q_p / 2)  where q_p = q at the pole
        let e = self.e;
        if e < 1e-12 {
            return self.a;
        }
        let qp = 1.0 - (1.0 - self.e2) / (2.0 * e) * ((1.0 - e) / (1.0 + e)).ln();
        self.a * (qp / 2.0).sqrt()
    }
}

/// Well-known reference ellipsoids.
impl Ellipsoid {
    /// WGS 84 ellipsoid (used by GPS).
    pub const WGS84: Ellipsoid = Ellipsoid {
        name: "WGS 84",
        a: 6_378_137.0,
        b: 6_356_752.314_245_179,
        f: 1.0 / 298.257_223_563,
        e2: 0.006_694_379_990_141_317,
        e: 0.081_819_190_842_622,
        ep2: 0.006_739_496_742_276_437,
    };

    /// GRS 80 ellipsoid (used by NAD83 and ETRS89).
    pub const GRS80: Ellipsoid = Ellipsoid {
        name: "GRS 80",
        a: 6_378_137.0,
        b: 6_356_752.314_140_347,
        f: 1.0 / 298.257_222_101,
        e2: 0.006_694_380_022_900_88,
        e: 0.081_819_191_042_816,
        ep2: 0.006_739_496_775_478_69,
    };

    /// Clarke 1866 ellipsoid (used by NAD27).
    pub const CLARKE1866: Ellipsoid = Ellipsoid {
        name: "Clarke 1866",
        a: 6_378_206.4,
        b: 6_356_583.8,
        f: 1.0 / 294.978_698_2,
        e2: 0.006_768_657_997_291_135,
        e: 0.082_271_854_223_003,
        ep2: 0.006_814_784_945_915_49,
    };

    /// International 1924 (Hayford) ellipsoid.
    pub const INTERNATIONAL: Ellipsoid = Ellipsoid {
        name: "International 1924",
        a: 6_378_388.0,
        b: 6_356_911.946_127_947,
        f: 1.0 / 297.0,
        e2: 0.006_722_670_022_333_32,
        e: 0.081_991_889_979_165,
        ep2: 0.006_768_170_197_224_17,
    };

    /// Bessel 1841 ellipsoid.
    pub const BESSEL: Ellipsoid = Ellipsoid {
        name: "Bessel 1841",
        a: 6_377_397.155,
        b: 6_356_078.962_818_189,
        f: 1.0 / 299.152_812_8,
        e2: 0.006_674_372_230_614_02,
        e: 0.081_696_831_215_268,
        ep2: 0.006_719_218_798_716_95,
    };

    /// Airy 1830 ellipsoid.
    pub const AIRY1830: Ellipsoid = Ellipsoid {
        name: "Airy 1830",
        a: 6_377_563.396,
        b: 6_356_256.909_237_285,
        f: 1.0 / 299.324_964_6,
        e2: 0.006_670_539_761_597_337,
        e: 0.081_673_373_874_141_39,
        ep2: 0.006_715_334_910_116_594,
    };

    /// Airy 1830 Modified ellipsoid.
    pub const AIRY1830_MOD: Ellipsoid = Ellipsoid {
        name: "Airy 1830 Modified",
        a: 6_377_340.189,
        b: 6_356_034.447_938_535,
        f: 1.0 / 299.324_964_6,
        e2: 0.006_670_540_074_149_133,
        e: 0.081_673_375_787_840_93,
        ep2: 0.006_715_335_227_302_461,
    };

    /// Krassowsky 1940 ellipsoid.
    pub const KRASSOWSKY1940: Ellipsoid = Ellipsoid {
        name: "Krassowsky 1940",
        a: 6_378_245.0,
        b: 6_356_863.018_773_047,
        f: 1.0 / 298.3,
        e2: 0.006_693_421_622_965_943,
        e: 0.081_813_334_016_931_18,
        ep2: 0.006_738_525_414_683_474,
    };

    /// IAU 1976 ellipsoid.
    pub const IAU1976: Ellipsoid = Ellipsoid {
        name: "IAU 1976",
        a: 6_378_140.0,
        b: 6_356_755.288_157_528,
        f: 1.0 / 298.257,
        e2: 0.006_694_384_999_587_966,
        e: 0.081_819_221_455_523_37,
        ep2: 0.006_739_501_819_473_361,
    };

    /// Clarke 1880 (RGS) ellipsoid.
    pub const CLARKE1880_RGS: Ellipsoid = Ellipsoid {
        name: "Clarke 1880 (RGS)",
        a: 6_378_249.145,
        b: 6_356_514.869_549_775_5,
        f: 1.0 / 293.465,
        e2: 0.006_803_511_282_849_064,
        e: 0.082_483_400_044_185_04,
        ep2: 0.006_850_116_125_195_659,
    };

    /// Everest 1830 ellipsoid.
    pub const EVEREST1830: Ellipsoid = Ellipsoid {
        name: "Everest 1830",
        a: 6_377_276.345,
        b: 6_356_075.413_140_24,
        f: 1.0 / 300.8017,
        e2: 0.006_637_846_630_199_688,
        e: 0.081_472_980_982_652_7,
        ep2: 0.006_682_202_062_643_52,
    };

    /// Helmert 1906 ellipsoid.
    pub const HELMERT1906: Ellipsoid = Ellipsoid {
        name: "Helmert 1906",
        a: 6_378_200.0,
        b: 6_356_818.169_627_891,
        f: 1.0 / 298.3,
        e2: 0.006_693_421_622_965_943,
        e: 0.081_813_334_016_931_15,
        ep2: 0.006_738_525_414_683_491,
    };

    /// Sphere of mean Earth radius (~6371 km).
    pub const SPHERE: Ellipsoid = Ellipsoid {
        name: "Sphere",
        a: 6_371_000.0,
        b: 6_371_000.0,
        f: 0.0,
        e2: 0.0,
        e: 0.0,
        ep2: 0.0,
    };
}

impl Default for Ellipsoid {
    fn default() -> Self {
        Ellipsoid::WGS84.clone()
    }
}

impl std::fmt::Display for Ellipsoid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_sphere() {
            return write!(f, "{} (a={:.3}m, sphere)", self.name, self.a);
        }
        write!(
            f,
            "{} (a={:.3}m, f=1/{:.6})",
            self.name,
            self.a,
            1.0 / self.f
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Ellipsoid;

    #[test]
    fn from_name_supports_aliases() {
        let a = Ellipsoid::from_name("WGS84").unwrap();
        let b = Ellipsoid::from_name("wGs 84").unwrap();
        let c = Ellipsoid::from_name("krassovsky-1940").unwrap();
        let d = Ellipsoid::from_name("helmert 1906").unwrap();

        assert!((a.a - Ellipsoid::WGS84.a).abs() < 1e-12);
        assert!((b.a - Ellipsoid::WGS84.a).abs() < 1e-12);
        assert!((c.a - Ellipsoid::KRASSOWSKY1940.a).abs() < 1e-12);
        assert!((d.a - Ellipsoid::HELMERT1906.a).abs() < 1e-12);
    }

    #[test]
    fn from_epsg_ellipsoid_resolves_common_codes() {
        assert_eq!(Ellipsoid::from_epsg_ellipsoid(7030).unwrap().name, "WGS 84");
        assert_eq!(Ellipsoid::from_epsg_ellipsoid(7019).unwrap().name, "GRS 80");
        assert_eq!(Ellipsoid::from_epsg_ellipsoid(7024).unwrap().name, "Krassowsky 1940");
        assert!(Ellipsoid::from_epsg_ellipsoid(9999).is_none());
    }

    #[test]
    fn display_sphere_is_safe() {
        let s = Ellipsoid::SPHERE.to_string();
        assert!(s.contains("sphere"));
        assert!(s.contains("a="));
    }
}
