//! PyO3 Python Bindings for wbgeostats Kriging Library
//!
//! Exposes kriging, variography, and cross-validation functionality to Python.
//! Build with: maturin develop

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
use pyo3::types::PyDict;

#[cfg(feature = "python")]
use std::sync::Arc;

#[cfg(feature = "python")]
use crate::variogram::{
    EmpiricalVariogramBuilder, VariogramModel, VariogramModelFamily, VariogramFitter,
};

#[cfg(feature = "python")]
use crate::kriging::OrdinaryKriging;

#[cfg(feature = "python")]
use crate::cv::LeaveOneOutCV;

// ──────────────────────────────────────────────────────────────────────────
// Python Type Wrappers
// ──────────────────────────────────────────────────────────────────────────

/// Python wrapper for VariogramModel
#[cfg(feature = "python")]
#[pyclass(name = "VariogramModel")]
pub struct PyVariogramModel {
    pub model: VariogramModel,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyVariogramModel {
    #[new]
    fn new(
        family: &str,
        nugget: f64,
        partial_sill: f64,
        range: f64,
    ) -> PyResult<Self> {
        let family = match family.to_lowercase().as_str() {
            "spherical" => VariogramModelFamily::Spherical,
            "exponential" => VariogramModelFamily::Exponential,
            "gaussian" => VariogramModelFamily::Gaussian,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Invalid family. Use: spherical, exponential, or gaussian",
            )),
        };

        Ok(PyVariogramModel {
            model: VariogramModel {
                family,
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

    #[getter]
    fn wrss(&self) -> f64 {
        self.model.wrss
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

/// Python wrapper for KrigingResult
#[cfg(feature = "python")]
#[pyclass(name = "KrigingResult")]
#[derive(Clone)]
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

/// Python wrapper for OrdinaryKriging
#[cfg(feature = "python")]
#[pyclass(name = "OrdinaryKriging")]
pub struct PyOrdinaryKriging {
    kriging: Arc<OrdinaryKriging>,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyOrdinaryKriging {
    #[new]
    fn new(
        coords: Vec<(f64, f64)>,
        values: Vec<f64>,
        vario: &PyVariogramModel,
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

    fn predict_batch(
        &self,
        coords: Vec<(f64, f64)>,
    ) -> PyResult<Vec<PyKrigingResult>> {
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

// ──────────────────────────────────────────────────────────────────────────
// Module Functions
// ──────────────────────────────────────────────────────────────────────────

/// Estimate empirical variogram from point data
#[cfg(feature = "python")]
#[pyfunction]
fn estimate_variogram(
    coords: Vec<(f64, f64)>,
    values: Vec<f64>,
    lag_distance: Option<f64>,
    num_lags: Option<i32>,
) -> PyResult<PyObject> {
    if coords.len() != values.len() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Coordinate and value arrays must have equal length",
        ));
    }

    let mut builder = EmpiricalVariogramBuilder::default();

    if let Some(lag) = lag_distance {
        builder = builder.lag_distance(lag);
    }
    if let Some(n) = num_lags {
        builder = builder.num_lags(n);
    }

    let vario = builder
        .build(&coords, &values)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    Python::with_gil(|py| {
        let dict = PyDict::new(py);
        dict.set_item("num_lags", vario.lags.len())?;
        dict.set_item("max_lag", vario.max_lag)?;
        dict.set_item("lags", vario.lags.clone())?;
        Ok(dict.into())
    })
}

/// Fit theoretical variogram model to empirical data
#[cfg(feature = "python")]
#[pyfunction]
fn fit_variogram(
    lags: Vec<f64>,
    semivariances: Vec<f64>,
    model_family: &str,
) -> PyResult<PyVariogramModel> {
    if lags.len() != semivariances.len() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Lag and semivariance arrays must have equal length",
        ));
    }

    let family = match model_family.to_lowercase().as_str() {
        "spherical" => VariogramModelFamily::Spherical,
        "exponential" => VariogramModelFamily::Exponential,
        "gaussian" => VariogramModelFamily::Gaussian,
        _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Invalid family. Use: spherical, exponential, or gaussian",
        )),
    };

    let fitter = VariogramFitter::new(lags, semivariances);
    let model = fitter
        .fit(family)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    Ok(PyVariogramModel { model })
}

/// Leave-One-Out Cross-Validation for kriging model assessment
#[cfg(feature = "python")]
#[pyfunction]
fn cross_validate_kriging(
    coords: Vec<(f64, f64)>,
    values: Vec<f64>,
    vario: &PyVariogramModel,
) -> PyResult<PyObject> {
    if coords.len() != values.len() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Coordinate and value arrays must have equal length",
        ));
    }

    let cv = LeaveOneOutCV::new(coords, values, vario.model.clone());
    let metrics = cv
        .validate()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    Python::with_gil(|py| {
        let dict = PyDict::new(py);
        dict.set_item("mean_error", metrics.mean_error)?;
        dict.set_item("rmse", metrics.rmse)?;
        dict.set_item("rmsse", metrics.rmsse)?;
        dict.set_item("correlation", metrics.correlation)?;
        dict.set_item("sample_size", metrics.sample_size)?;
        dict.set_item("is_well_calibrated", metrics.is_well_calibrated())?;
        Ok(dict.into())
    })
}

// ──────────────────────────────────────────────────────────────────────────
// Python Module Definition
// ──────────────────────────────────────────────────────────────────────────

#[cfg(feature = "python")]
#[pymodule]
pub fn wbgeostats(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyVariogramModel>()?;
    m.add_class::<PyKrigingResult>()?;
    m.add_class::<PyOrdinaryKriging>()?;
    m.add_function(wrap_pyfunction!(estimate_variogram, m)?)?;
    m.add_function(wrap_pyfunction!(fit_variogram, m)?)?;
    m.add_function(wrap_pyfunction!(cross_validate_kriging, m)?)?;

    m.add(
        "__doc__",
        "Production-grade geostatistics kriging library for Python",
    )?;

    Ok(())
}
