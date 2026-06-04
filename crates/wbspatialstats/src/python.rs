//! PyO3 Python Bindings for wbspatialstats Spatial Statistics Library
//!
//! Exposes kriging, variography, and cross-validation functionality to Python.
//! Build with: maturin develop

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
use std::sync::Arc;

#[cfg(feature = "python")]
use crate::variogram::{
    EmpiricalVariogramBuilder, VariogramModel, VariogramModelFamily, VariogramFitter, LagBin,
};

#[cfg(feature = "python")]
use crate::kriging::{OrdinaryKriging, LocalOrdinaryKriging, SimpleKriging, SpaceTimeKriging};

#[cfg(feature = "python")]
use crate::cv::LeaveOneOutCV;

// ──────────────────────────────────────────────────────────────────────────
// Python Wrapper Types
// ──────────────────────────────────────────────────────────────────────────

/// Python-exposed VariogramModel wrapper
#[cfg(feature = "python")]
#[pyclass(name = "VariogramModel")]
pub struct PyVariogramModel {
    pub model: VariogramModel,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyVariogramModel {
    #[new]
    fn new(family: &str, nugget: f64, partial_sill: f64, range: f64) -> PyResult<Self> {
        let vario_family = match family.to_lowercase().as_str() {
            "spherical" => VariogramModelFamily::Spherical,
            "exponential" => VariogramModelFamily::Exponential,
            "gaussian" => VariogramModelFamily::Gaussian,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Invalid family. Use: spherical, exponential, or gaussian",
                ))
            }
        };

        Ok(PyVariogramModel {
            model: VariogramModel {
                family: vario_family,
                nugget,
                partial_sill,
                range,
                wrss: 0.0,
                condition_number: 1.0,
            },
        })
    }

    #[getter]
    fn family(&self) -> String {
        format!("{:?}", self.model.family).to_lowercase()
    }

    #[getter]
    fn nugget(&self) -> f64 {
        self.model.nugget
    }

    #[getter]
    fn partial_sill(&self) -> f64 {
        self.model.partial_sill
    }

    #[getter]
    fn range(&self) -> f64 {
        self.model.range
    }

    #[getter]
    fn total_sill(&self) -> f64 {
        self.model.nugget + self.model.partial_sill
    }

    fn __repr__(&self) -> String {
        format!(
            "VariogramModel(family='{}', nugget={}, psill={}, range={})",
            format!("{:?}", self.model.family).to_lowercase(),
            self.model.nugget,
            self.model.partial_sill,
            self.model.range
        )
    }
}

/// Python-exposed KrigingResult wrapper
#[cfg(feature = "python")]
#[pyclass(name = "KrigingResult")]
pub struct PyKrigingResult {
    pub prediction: f64,
    pub variance: f64,
    pub std_error: f64,
    pub ci_lower: f64,
    pub ci_upper: f64,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyKrigingResult {
    #[getter]
    fn prediction(&self) -> f64 {
        self.prediction
    }

    #[getter]
    fn variance(&self) -> f64 {
        self.variance
    }

    #[getter]
    fn std_error(&self) -> f64 {
        self.std_error
    }

    #[getter]
    fn ci_lower(&self) -> f64 {
        self.ci_lower
    }

    #[getter]
    fn ci_upper(&self) -> f64 {
        self.ci_upper
    }

    fn __repr__(&self) -> String {
        format!(
            "KrigingResult(pred={}, var={}, ci=[{}, {}])",
            self.prediction, self.variance, self.ci_lower, self.ci_upper
        )
    }
}

/// Python-exposed OrdinaryKriging wrapper
#[cfg(feature = "python")]
#[pyclass(name = "OrdinaryKriging")]
pub struct PyOrdinaryKriging {
    kriging: Arc<OrdinaryKriging>,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyOrdinaryKriging {
    #[new]
    fn new(coords: Vec<(f64, f64)>, values: Vec<f64>, vario: &PyVariogramModel) -> PyResult<Self> {
        if coords.len() != values.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Coordinate and value arrays must have equal length",
            ));
        }

        if coords.len() < 3 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "At least 3 training points required",
            ));
        }

        let kriging = OrdinaryKriging::new(coords, values, vario.model.clone())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PyOrdinaryKriging {
            kriging: Arc::new(kriging),
        })
    }

    fn predict(&self, x: f64, y: f64) -> PyResult<PyKrigingResult> {
        let result = self
            .kriging
            .predict((x, y))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PyKrigingResult {
            prediction: result.prediction,
            variance: result.variance,
            std_error: result.std_error,
            ci_lower: result.ci_lower,
            ci_upper: result.ci_upper,
        })
    }

    fn predict_batch(&self, coords: Vec<(f64, f64)>) -> PyResult<Vec<PyKrigingResult>> {
        let results = self
            .kriging
            .predict_batch(&coords)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|r| PyKrigingResult {
                prediction: r.prediction,
                variance: r.variance,
                std_error: r.std_error,
                ci_lower: r.ci_lower,
                ci_upper: r.ci_upper,
            })
            .collect())
    }

    fn __repr__(&self) -> String {
        format!(
            "OrdinaryKriging(training_points={})",
            self.kriging.training_coords.len()
        )
    }
}

/// Python-exposed LocalOrdinaryKriging wrapper
#[cfg(feature = "python")]
#[pyclass(name = "LocalOrdinaryKriging")]
pub struct PyLocalOrdinaryKriging {
    kriging: Arc<LocalOrdinaryKriging>,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyLocalOrdinaryKriging {
    #[new]
    fn new(
        coords: Vec<(f64, f64)>,
        values: Vec<f64>,
        vario: &PyVariogramModel,
        k: Option<usize>,
    ) -> PyResult<Self> {
        if coords.len() != values.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Coordinate and value arrays must have equal length",
            ));
        }

        if coords.len() < 3 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "At least 3 training points required",
            ));
        }

        let k = k.unwrap_or(20).min(coords.len());

        if k < 3 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "k must be at least 3 for kriging",
            ));
        }

        let kriging = LocalOrdinaryKriging::new(coords, values, vario.model.clone(), k)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PyLocalOrdinaryKriging {
            kriging: Arc::new(kriging),
        })
    }

    fn predict(&self, x: f64, y: f64) -> PyResult<PyKrigingResult> {
        let result = self
            .kriging
            .predict((x, y))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PyKrigingResult {
            prediction: result.prediction,
            variance: result.variance,
            std_error: result.std_error,
            ci_lower: result.ci_lower,
            ci_upper: result.ci_upper,
        })
    }

    fn predict_batch(&self, coords: Vec<(f64, f64)>) -> PyResult<Vec<PyKrigingResult>> {
        let results = self
            .kriging
            .predict_batch(&coords)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|r| PyKrigingResult {
                prediction: r.prediction,
                variance: r.variance,
                std_error: r.std_error,
                ci_lower: r.ci_lower,
                ci_upper: r.ci_upper,
            })
            .collect())
    }

    #[getter]
    fn k(&self) -> usize {
        self.kriging.k()
    }

    #[getter]
    fn n_training(&self) -> usize {
        self.kriging.n_training()
    }

    fn __repr__(&self) -> String {
        format!(
            "LocalOrdinaryKriging(training_points={}, k={})",
            self.kriging.n_training(),
            self.kriging.k()
        )
    }
}

/// Python-exposed SimpleKriging wrapper
#[cfg(feature = "python")]
#[pyclass(name = "SimpleKriging")]
pub struct PySimpleKriging {
    kriging: Arc<SimpleKriging>,
}

#[cfg(feature = "python")]
#[pymethods]
impl PySimpleKriging {
    #[new]
    fn new(coords: Vec<(f64, f64)>, values: Vec<f64>, vario: &PyVariogramModel, known_mean: f64) -> PyResult<Self> {
        if coords.len() != values.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Coordinate and value arrays must have equal length",
            ));
        }

        if coords.len() < 3 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "At least 3 training points required",
            ));
        }

        let kriging = SimpleKriging::new(coords, values, vario.model.clone(), known_mean)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PySimpleKriging {
            kriging: Arc::new(kriging),
        })
    }

    fn predict(&self, x: f64, y: f64) -> PyResult<PyKrigingResult> {
        let result = self
            .kriging
            .predict(x, y)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PyKrigingResult {
            prediction: result.prediction,
            variance: result.variance,
            std_error: result.std_error,
            ci_lower: result.ci_lower,
            ci_upper: result.ci_upper,
        })
    }

    fn predict_batch(&self, coords: Vec<(f64, f64)>) -> PyResult<Vec<PyKrigingResult>> {
        let results = self
            .kriging
            .predict_batch(&coords)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|r| PyKrigingResult {
                prediction: r.prediction,
                variance: r.variance,
                std_error: r.std_error,
                ci_lower: r.ci_lower,
                ci_upper: r.ci_upper,
            })
            .collect())
    }

    #[getter]
    fn known_mean(&self) -> f64 {
        self.kriging.known_mean()
    }

    #[getter]
    fn n_training(&self) -> usize {
        self.kriging.n_training()
    }

    fn __repr__(&self) -> String {
        format!(
            "SimpleKriging(training_points={}, known_mean={})",
            self.kriging.n_training(),
            self.kriging.known_mean()
        )
    }
}

/// Python-exposed SpaceTimeKriging wrapper
#[cfg(feature = "python")]
#[pyclass(name = "SpaceTimeKriging")]
pub struct PySpaceTimeKriging {
    kriging: Arc<SpaceTimeKriging>,
}

#[cfg(feature = "python")]
#[pymethods]
impl PySpaceTimeKriging {
    #[new]
    fn new(
        coords_spatial: Vec<(f64, f64)>,
        coords_temporal: Vec<f64>,
        values: Vec<f64>,
        vario_spatial: &PyVariogramModel,
        vario_temporal: &PyVariogramModel,
    ) -> PyResult<Self> {
        if coords_spatial.len() != coords_temporal.len() || coords_spatial.len() != values.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Spatial coords, temporal coords, and values must have equal length",
            ));
        }

        if coords_spatial.len() < 4 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "At least 4 spatio-temporal points required",
            ));
        }

        let kriging = SpaceTimeKriging::new(
            coords_spatial,
            coords_temporal,
            values,
            vario_spatial.model.clone(),
            vario_temporal.model.clone(),
        )
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PySpaceTimeKriging {
            kriging: Arc::new(kriging),
        })
    }

    fn predict(&self, x: f64, y: f64, t: f64) -> PyResult<PyKrigingResult> {
        let result = self
            .kriging
            .predict(x, y, t)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PyKrigingResult {
            prediction: result.prediction,
            variance: result.variance,
            std_error: result.std_error,
            ci_lower: result.ci_lower,
            ci_upper: result.ci_upper,
        })
    }

    fn predict_batch(&self, coords_spatial: Vec<(f64, f64)>, coords_temporal: Vec<f64>) -> PyResult<Vec<PyKrigingResult>> {
        let results = self
            .kriging
            .predict_batch(coords_spatial, coords_temporal)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|r| PyKrigingResult {
                prediction: r.prediction,
                variance: r.variance,
                std_error: r.std_error,
                ci_lower: r.ci_lower,
                ci_upper: r.ci_upper,
            })
            .collect())
    }

    #[getter]
    fn n_training(&self) -> usize {
        self.kriging.n_training()
    }

    fn __repr__(&self) -> String {
        format!("SpaceTimeKriging(training_points={})", self.kriging.n_training())
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Functional Python API
// ──────────────────────────────────────────────────────────────────────────

/// Estimate empirical variogram from point data
#[cfg(feature = "python")]
#[pyfunction]
fn estimate_variogram(
    x: Vec<f64>,
    y: Vec<f64>,
    values: Vec<f64>,
    lag_distance: Option<f64>,
    max_lag_count: Option<usize>,
) -> PyResult<std::collections::HashMap<String, Vec<f64>>> {
    if x.len() != y.len() || x.len() != values.len() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "x, y, and values must have equal length",
        ));
    }

    let coords: Vec<(f64, f64)> = x.iter().zip(y.iter()).map(|(&a, &b)| (a, b)).collect();
    let mut builder = EmpiricalVariogramBuilder::default();

    if let Some(lag) = lag_distance {
        builder = builder.lag_distance(lag);
    }
    if let Some(count) = max_lag_count {
        builder = builder.max_lag_count(count);
    }

    let vario = builder.build(&coords, &values).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Variogram estimation failed: {}", e))
    })?;

    let distances: Vec<f64> = vario.lags.iter().map(|b| b.distance).collect();
    let semivariances: Vec<f64> = vario.lags.iter().map(|b| b.semivariance).collect();
    let pair_counts: Vec<f64> = vario.lags.iter().map(|b| b.pair_count as f64).collect();

    let mut result = std::collections::HashMap::new();
    result.insert("distances".to_string(), distances);
    result.insert("semivariances".to_string(), semivariances);
    result.insert("pair_counts".to_string(), pair_counts);
    result.insert("max_lag".to_string(), vec![vario.max_lag]);

    Ok(result)
}

/// Fit theoretical variogram model to empirical data
#[cfg(feature = "python")]
#[pyfunction]
fn fit_variogram(
    distances: Vec<f64>,
    semivariances: Vec<f64>,
    pair_counts: Vec<i32>,
    model_family: &str,
) -> PyResult<PyVariogramModel> {
    if distances.len() != semivariances.len() || distances.len() != pair_counts.len() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "distances, semivariances, and pair_counts must have equal length",
        ));
    }

    let family = match model_family.to_lowercase().as_str() {
        "spherical" => VariogramModelFamily::Spherical,
        "exponential" => VariogramModelFamily::Exponential,
        "gaussian" => VariogramModelFamily::Gaussian,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Invalid family. Use: 'spherical', 'exponential', or 'gaussian'",
            ))
        }
    };

    let lags: Vec<LagBin> = distances
        .iter()
        .zip(semivariances.iter())
        .zip(pair_counts.iter())
        .map(|((&d, &sv), &pc)| LagBin {
            distance: d,
            semivariance: sv,
            pair_count: pc as usize,
        })
        .collect();

    let model = VariogramFitter::fit(&lags, family).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Variogram fitting failed: {}", e))
    })?;

    Ok(PyVariogramModel { model })
}

/// Leave-One-Out Cross-Validation for kriging model assessment
#[cfg(feature = "python")]
#[pyfunction]
fn cross_validate_kriging(
    x: Vec<f64>,
    y: Vec<f64>,
    values: Vec<f64>,
    vario: &PyVariogramModel,
) -> PyResult<std::collections::HashMap<String, f64>> {
    if x.len() != y.len() || x.len() != values.len() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "x, y, and values must have equal length",
        ));
    }

    let coords: Vec<(f64, f64)> = x.iter().zip(y.iter()).map(|(&x, &y)| (x, y)).collect();

    let metrics = LeaveOneOutCV::validate(&coords, &values, &vario.model).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Cross-validation failed: {}", e))
    })?;

    let mut result = std::collections::HashMap::new();
    result.insert("mean_error".to_string(), metrics.mean_error);
    result.insert("rmse".to_string(), metrics.rmse);
    result.insert("rmsse".to_string(), metrics.rmsse);
    result.insert("correlation".to_string(), metrics.correlation);
    result.insert("sample_size".to_string(), metrics.sample_size as f64);
    result.insert(
        "is_well_calibrated".to_string(),
        if metrics.is_well_calibrated() { 1.0 } else { 0.0 },
    );

    Ok(result)
}

// ──────────────────────────────────────────────────────────────────────────
// Python Module Definition
// ──────────────────────────────────────────────────────────────────────────

#[cfg(feature = "python")]
#[pymodule]
fn wbspatialstats(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyVariogramModel>()?;
    m.add_class::<PyKrigingResult>()?;
    m.add_class::<PyOrdinaryKriging>()?;
    m.add_class::<PyLocalOrdinaryKriging>()?;
    m.add_class::<PySimpleKriging>()?;
    m.add_class::<PySpaceTimeKriging>()?;
    m.add_function(wrap_pyfunction!(estimate_variogram, m)?)?;
    m.add_function(wrap_pyfunction!(fit_variogram, m)?)?;
    m.add_function(wrap_pyfunction!(cross_validate_kriging, m)?)?;

    Ok(())
}
