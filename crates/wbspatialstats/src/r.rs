//! extendr R Bindings for wbspatialstats Spatial Statistics Library
//!
//! Functional interface for kriging, variography, and cross-validation.
//! Returns plain lists (Robj) with named vectors for easy R consumption.

#[cfg(feature = "r")]
use extendr_api::prelude::*;

#[cfg(feature = "r")]
use crate::variogram::{
    EmpiricalVariogramBuilder, VariogramModel, VariogramModelFamily, VariogramFitter, LagBin,
};

#[cfg(feature = "r")]
use crate::kriging::{OrdinaryKriging, LocalOrdinaryKriging, SimpleKriging, SpaceTimeKriging};

#[cfg(feature = "r")]
use crate::cv::LeaveOneOutCV;

// ──────────────────────────────────────────────────────────────────────────
// Variogram Functions
// ──────────────────────────────────────────────────────────────────────────

/// Estimate empirical variogram from point data
///
/// # Arguments
/// - `x`: X coordinates
/// - `y`: Y coordinates
/// - `values`: Observed values
/// - `lag_distance`: Optional lag bin size (auto-computed if NULL)
/// - `max_lag_count`: Optional maximum number of lag bins (default: 15)
///
/// # Returns
/// List with: num_lags, max_lag, distances (vector), semivariances (vector), pair_counts (vector)
#[cfg(feature = "r")]
#[extendr]
pub fn estimate_variogram(
    x: Vec<f64>,
    y: Vec<f64>,
    values: Vec<f64>,
    lag_distance: Robj,
    max_lag_count: Robj,
) -> Result<Robj> {
    if x.len() != y.len() || x.len() != values.len() {
        return Err(extendr_api::Error::Other(
            "x, y, and values must have equal length".to_string(),
        ));
    }

    let coords: Vec<(f64, f64)> = x.iter().zip(y.iter()).map(|(&a, &b)| (a, b)).collect();
    let mut builder = EmpiricalVariogramBuilder::default();

    // Handle optional lag_distance parameter
    if !lag_distance.is_null() && lag_distance.len() > 0 {
        if let Some(lag) = lag_distance.as_real() {
            builder = builder.lag_distance(lag);
        }
    }

    // Handle optional max_lag_count parameter
    if !max_lag_count.is_null() && max_lag_count.len() > 0 {
        if let Some(count) = max_lag_count.as_integer() {
            builder = builder.max_lag_count(count as usize);
        }
    }

    let vario = builder.build(&coords, &values).map_err(|e| {
        extendr_api::Error::Other(format!("Variogram estimation failed: {}", e))
    })?;

    // Extract lag bin data for R
    let distances: Vec<f64> = vario.lags.iter().map(|b| b.distance).collect();
    let semivariances: Vec<f64> = vario.lags.iter().map(|b| b.semivariance).collect();
    let pair_counts: Vec<i32> = vario
        .lags
        .iter()
        .map(|b| b.pair_count as i32)
        .collect();

    Ok(list!(
        num_lags = vario.lags.len() as i32,
        max_lag = vario.max_lag,
        distances = distances,
        semivariances = semivariances,
        pair_counts = pair_counts,
    )
    .into())
}

/// Fit theoretical variogram model to empirical variogram
///
/// # Arguments
/// - `distances`: Lag distances
/// - `semivariances`: Semivariance values at each lag
/// - `pair_counts`: Number of pairs in each lag bin
/// - `model_family`: "spherical", "exponential", or "gaussian"
///
/// # Returns
/// List with: family, nugget, partial_sill, range, total_sill, wrss
#[cfg(feature = "r")]
#[extendr]
pub fn fit_variogram(
    distances: Vec<f64>,
    semivariances: Vec<f64>,
    pair_counts: Vec<i32>,
    model_family: &str,
) -> Result<Robj> {
    if distances.len() != semivariances.len() || distances.len() != pair_counts.len() {
        return Err(extendr_api::Error::Other(
            "distances, semivariances, and pair_counts must have equal length".to_string(),
        ));
    }

    let family = match model_family.to_lowercase().as_str() {
        "spherical" => VariogramModelFamily::Spherical,
        "exponential" => VariogramModelFamily::Exponential,
        "gaussian" => VariogramModelFamily::Gaussian,
        _ => {
            return Err(extendr_api::Error::Other(
                "Invalid family. Use: 'spherical', 'exponential', or 'gaussian'".to_string(),
            ))
        }
    };

    // Build LagBin vec from input arrays
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
        extendr_api::Error::Other(format!("Variogram fitting failed: {}", e))
    })?;

    Ok(list!(
        family = format!("{:?}", model.family).to_lowercase(),
        nugget = model.nugget,
        partial_sill = model.partial_sill,
        range = model.range,
        total_sill = model.nugget + model.partial_sill,
        wrss = model.wrss,
    )
    .into())
}

// ──────────────────────────────────────────────────────────────────────────
// Kriging Functions
// ──────────────────────────────────────────────────────────────────────────

/// Perform ordinary kriging single-point prediction
///
/// # Arguments
/// - `train_x`, `train_y`: Training data coordinates
/// - `train_values`: Training data values
/// - `pred_x`, `pred_y`: Prediction location
/// - `family`, `nugget`, `psill`, `range`: Variogram model parameters
///
/// # Returns
/// List with: prediction, variance, std_error, ci_lower, ci_upper
#[cfg(feature = "r")]
#[extendr]
pub fn kriging_predict(
    train_x: Vec<f64>,
    train_y: Vec<f64>,
    train_values: Vec<f64>,
    pred_x: f64,
    pred_y: f64,
    family: &str,
    nugget: f64,
    psill: f64,
    range: f64,
) -> Result<Robj> {
    if train_x.len() != train_y.len() || train_x.len() != train_values.len() {
        return Err(extendr_api::Error::Other(
            "Training x, y, and values must have equal length".to_string(),
        ));
    }

    if train_x.len() < 3 {
        return Err(extendr_api::Error::Other(
            "At least 3 training points required".to_string(),
        ));
    }

    let vario_family = match family.to_lowercase().as_str() {
        "spherical" => VariogramModelFamily::Spherical,
        "exponential" => VariogramModelFamily::Exponential,
        "gaussian" => VariogramModelFamily::Gaussian,
        _ => {
            return Err(extendr_api::Error::Other(
                "Invalid family. Use: 'spherical', 'exponential', or 'gaussian'".to_string(),
            ))
        }
    };

    let vario_model = VariogramModel {
        family: vario_family,
        nugget,
        partial_sill: psill,
        range,
        wrss: 0.0,
        condition_number: 1.0,
    };

    let coords: Vec<(f64, f64)> =
        train_x
            .iter()
            .zip(train_y.iter())
            .map(|(&x, &y)| (x, y))
            .collect();

    let kriging = OrdinaryKriging::new(coords, train_values, vario_model).map_err(|e| {
        extendr_api::Error::Other(format!("Kriging initialization failed: {}", e))
    })?;

    let result = kriging.predict((pred_x, pred_y)).map_err(|e| {
        extendr_api::Error::Other(format!("Prediction failed: {}", e))
    })?;

    Ok(list!(
        prediction = result.prediction,
        variance = result.variance,
        std_error = result.std_error,
        ci_lower = result.ci_lower,
        ci_upper = result.ci_upper,
    )
    .into())
}

/// Perform ordinary kriging grid prediction (vectorized)
///
/// # Arguments
/// - `train_x`, `train_y`: Training data coordinates
/// - `train_values`: Training data values
/// - `pred_x`, `pred_y`: Grid prediction locations (vectors)
/// - `family`, `nugget`, `psill`, `range`: Variogram model parameters
///
/// # Returns
/// List with vectors: prediction, variance, std_error, ci_lower, ci_upper
#[cfg(feature = "r")]
#[extendr]
pub fn kriging_predict_grid(
    train_x: Vec<f64>,
    train_y: Vec<f64>,
    train_values: Vec<f64>,
    pred_x: Vec<f64>,
    pred_y: Vec<f64>,
    family: &str,
    nugget: f64,
    psill: f64,
    range: f64,
) -> Result<Robj> {
    if train_x.len() != train_y.len() || train_x.len() != train_values.len() {
        return Err(extendr_api::Error::Other(
            "Training x, y, and values must have equal length".to_string(),
        ));
    }

    if pred_x.len() != pred_y.len() {
        return Err(extendr_api::Error::Other(
            "pred_x and pred_y must have equal length".to_string(),
        ));
    }

    if train_x.len() < 3 {
        return Err(extendr_api::Error::Other(
            "At least 3 training points required".to_string(),
        ));
    }

    let vario_family = match family.to_lowercase().as_str() {
        "spherical" => VariogramModelFamily::Spherical,
        "exponential" => VariogramModelFamily::Exponential,
        "gaussian" => VariogramModelFamily::Gaussian,
        _ => {
            return Err(extendr_api::Error::Other(
                "Invalid family. Use: 'spherical', 'exponential', or 'gaussian'".to_string(),
            ))
        }
    };

    let vario_model = VariogramModel {
        family: vario_family,
        nugget,
        partial_sill: psill,
        range,
        wrss: 0.0,
        condition_number: 1.0,
    };

    let train_coords: Vec<(f64, f64)> = train_x
        .iter()
        .zip(train_y.iter())
        .map(|(&x, &y)| (x, y))
        .collect();

    let pred_coords: Vec<(f64, f64)> = pred_x
        .iter()
        .zip(pred_y.iter())
        .map(|(&x, &y)| (x, y))
        .collect();

    let kriging = OrdinaryKriging::new(train_coords, train_values, vario_model).map_err(|e| {
        extendr_api::Error::Other(format!("Kriging initialization failed: {}", e))
    })?;

    let results = kriging.predict_batch(&pred_coords).map_err(|e| {
        extendr_api::Error::Other(format!("Batch prediction failed: {}", e))
    })?;

    let predictions: Vec<f64> = results.iter().map(|r| r.prediction).collect();
    let variances: Vec<f64> = results.iter().map(|r| r.variance).collect();
    let std_errors: Vec<f64> = results.iter().map(|r| r.std_error).collect();
    let ci_lowers: Vec<f64> = results.iter().map(|r| r.ci_lower).collect();
    let ci_uppers: Vec<f64> = results.iter().map(|r| r.ci_upper).collect();

    Ok(list!(
        prediction = predictions,
        variance = variances,
        std_error = std_errors,
        ci_lower = ci_lowers,
        ci_upper = ci_uppers,
    )
    .into())
}

/// Leave-One-Out Cross-Validation for kriging model assessment
///
/// # Arguments
/// - `x`, `y`: Data coordinates
/// - `values`: Observed values
/// - `family`, `nugget`, `psill`, `range`: Variogram model parameters
///
/// # Returns
/// List with: mean_error, rmse, rmsse, correlation, sample_size, is_well_calibrated
#[cfg(feature = "r")]
#[extendr]
pub fn kriging_cross_validate(
    x: Vec<f64>,
    y: Vec<f64>,
    values: Vec<f64>,
    family: &str,
    nugget: f64,
    psill: f64,
    range: f64,
) -> Result<Robj> {
    if x.len() != y.len() || x.len() != values.len() {
        return Err(extendr_api::Error::Other(
            "x, y, and values must have equal length".to_string(),
        ));
    }

    let vario_family = match family.to_lowercase().as_str() {
        "spherical" => VariogramModelFamily::Spherical,
        "exponential" => VariogramModelFamily::Exponential,
        "gaussian" => VariogramModelFamily::Gaussian,
        _ => {
            return Err(extendr_api::Error::Other(
                "Invalid family. Use: 'spherical', 'exponential', or 'gaussian'".to_string(),
            ))
        }
    };

    let vario_model = VariogramModel {
        family: vario_family,
        nugget,
        partial_sill: psill,
        range,
        wrss: 0.0,
        condition_number: 1.0,
    };

    let coords: Vec<(f64, f64)> = x.iter().zip(y.iter()).map(|(&x, &y)| (x, y)).collect();

    let metrics = LeaveOneOutCV::validate(&coords, &values, &vario_model).map_err(|e| {
        extendr_api::Error::Other(format!("Cross-validation failed: {}", e))
    })?;

    Ok(list!(
        mean_error = metrics.mean_error,
        rmse = metrics.rmse,
        rmsse = metrics.rmsse,
        correlation = metrics.correlation,
        sample_size = metrics.sample_size as i32,
        is_well_calibrated = metrics.is_well_calibrated(),
    )
    .into())
}

// ──────────────────────────────────────────────────────────────────────────
// Local Kriging Functions (k-nearest neighbors)
// ──────────────────────────────────────────────────────────────────────────

/// Perform local kriging prediction (single point, k-nearest neighbors)
///
/// # Arguments
/// - `train_x`, `train_y`: Training data coordinates
/// - `train_values`: Training data values
/// - `pred_x`, `pred_y`: Prediction location
/// - `k`: Number of nearest neighbors (default: 20)
/// - `family`, `nugget`, `psill`, `range`: Variogram model parameters
///
/// # Returns
/// List with: prediction, variance, std_error, ci_lower, ci_upper
#[cfg(feature = "r")]
#[extendr]
pub fn kriging_predict_local(
    train_x: Vec<f64>,
    train_y: Vec<f64>,
    train_values: Vec<f64>,
    pred_x: f64,
    pred_y: f64,
    k: i32,
    family: &str,
    nugget: f64,
    psill: f64,
    range: f64,
) -> Result<Robj> {
    if train_x.len() != train_y.len() || train_x.len() != train_values.len() {
        return Err(extendr_api::Error::Other(
            "Training x, y, and values must have equal length".to_string(),
        ));
    }

    if train_x.len() < 3 {
        return Err(extendr_api::Error::Other(
            "At least 3 training points required".to_string(),
        ));
    }

    let k = k as usize;
    if k < 3 || k > train_x.len() {
        return Err(extendr_api::Error::Other(
            format!("k must be between 3 and {} (number of training points)", train_x.len()),
        ));
    }

    let vario_family = match family.to_lowercase().as_str() {
        "spherical" => VariogramModelFamily::Spherical,
        "exponential" => VariogramModelFamily::Exponential,
        "gaussian" => VariogramModelFamily::Gaussian,
        _ => {
            return Err(extendr_api::Error::Other(
                "Invalid family. Use: 'spherical', 'exponential', or 'gaussian'".to_string(),
            ))
        }
    };

    let vario_model = VariogramModel {
        family: vario_family,
        nugget,
        partial_sill: psill,
        range,
        wrss: 0.0,
        condition_number: 1.0,
    };

    let coords: Vec<(f64, f64)> = train_x
        .iter()
        .zip(train_y.iter())
        .map(|(&x, &y)| (x, y))
        .collect();

    let local_kriging = LocalOrdinaryKriging::new(coords, train_values, vario_model, k).map_err(|e| {
        extendr_api::Error::Other(format!("Local kriging initialization failed: {}", e))
    })?;

    let result = local_kriging.predict((pred_x, pred_y)).map_err(|e| {
        extendr_api::Error::Other(format!("Local prediction failed: {}", e))
    })?;

    Ok(list!(
        prediction = result.prediction,
        variance = result.variance,
        std_error = result.std_error,
        ci_lower = result.ci_lower,
        ci_upper = result.ci_upper,
        k = k as i32,
    )
    .into())
}

/// Perform local kriging grid prediction (vectorized, k-nearest neighbors)
///
/// # Arguments
/// - `train_x`, `train_y`: Training data coordinates
/// - `train_values`: Training data values
/// - `pred_x`, `pred_y`: Grid prediction locations (vectors)
/// - `k`: Number of nearest neighbors (default: 20)
/// - `family`, `nugget`, `psill`, `range`: Variogram model parameters
///
/// # Returns
/// List with vectors: prediction, variance, std_error, ci_lower, ci_upper, k
#[cfg(feature = "r")]
#[extendr]
pub fn kriging_predict_grid_local(
    train_x: Vec<f64>,
    train_y: Vec<f64>,
    train_values: Vec<f64>,
    pred_x: Vec<f64>,
    pred_y: Vec<f64>,
    k: i32,
    family: &str,
    nugget: f64,
    psill: f64,
    range: f64,
) -> Result<Robj> {
    if train_x.len() != train_y.len() || train_x.len() != train_values.len() {
        return Err(extendr_api::Error::Other(
            "Training x, y, and values must have equal length".to_string(),
        ));
    }

    if pred_x.len() != pred_y.len() {
        return Err(extendr_api::Error::Other(
            "pred_x and pred_y must have equal length".to_string(),
        ));
    }

    if train_x.len() < 3 {
        return Err(extendr_api::Error::Other(
            "At least 3 training points required".to_string(),
        ));
    }

    let k = k as usize;
    if k < 3 || k > train_x.len() {
        return Err(extendr_api::Error::Other(
            format!("k must be between 3 and {} (number of training points)", train_x.len()),
        ));
    }

    let vario_family = match family.to_lowercase().as_str() {
        "spherical" => VariogramModelFamily::Spherical,
        "exponential" => VariogramModelFamily::Exponential,
        "gaussian" => VariogramModelFamily::Gaussian,
        _ => {
            return Err(extendr_api::Error::Other(
                "Invalid family. Use: 'spherical', 'exponential', or 'gaussian'".to_string(),
            ))
        }
    };

    let vario_model = VariogramModel {
        family: vario_family,
        nugget,
        partial_sill: psill,
        range,
        wrss: 0.0,
        condition_number: 1.0,
    };

    let train_coords: Vec<(f64, f64)> = train_x
        .iter()
        .zip(train_y.iter())
        .map(|(&x, &y)| (x, y))
        .collect();

    let pred_coords: Vec<(f64, f64)> = pred_x
        .iter()
        .zip(pred_y.iter())
        .map(|(&x, &y)| (x, y))
        .collect();

    let local_kriging =
        LocalOrdinaryKriging::new(train_coords, train_values, vario_model, k).map_err(|e| {
            extendr_api::Error::Other(format!("Local kriging initialization failed: {}", e))
        })?;

    let results = local_kriging.predict_batch(&pred_coords).map_err(|e| {
        extendr_api::Error::Other(format!("Local batch prediction failed: {}", e))
    })?;

    let predictions: Vec<f64> = results.iter().map(|r| r.prediction).collect();
    let variances: Vec<f64> = results.iter().map(|r| r.variance).collect();
    let std_errors: Vec<f64> = results.iter().map(|r| r.std_error).collect();
    let ci_lowers: Vec<f64> = results.iter().map(|r| r.ci_lower).collect();
    let ci_uppers: Vec<f64> = results.iter().map(|r| r.ci_upper).collect();

    Ok(list!(
        prediction = predictions,
        variance = variances,
        std_error = std_errors,
        ci_lower = ci_lowers,
        ci_upper = ci_uppers,
        k = k as i32,
    )
    .into())
}

/// Perform simple kriging prediction (single point, known mean)
///
/// # Arguments
/// - `train_x`, `train_y`: Training data coordinates
/// - `train_values`: Training data values
/// - `pred_x`, `pred_y`: Prediction location
/// - `known_mean`: The known constant mean of the field
/// - `family`, `nugget`, `psill`, `range`: Variogram model parameters
///
/// # Returns
/// List with: prediction, variance, std_error, ci_lower, ci_upper, known_mean
#[cfg(feature = "r")]
#[extendr]
pub fn kriging_predict_simple(
    train_x: Vec<f64>,
    train_y: Vec<f64>,
    train_values: Vec<f64>,
    pred_x: f64,
    pred_y: f64,
    known_mean: f64,
    family: &str,
    nugget: f64,
    psill: f64,
    range: f64,
) -> Result<Robj> {
    if train_x.len() != train_y.len() || train_x.len() != train_values.len() {
        return Err(extendr_api::Error::Other(
            "Training x, y, and values must have equal length".to_string(),
        ));
    }

    if train_x.len() < 3 {
        return Err(extendr_api::Error::Other(
            "At least 3 training points required".to_string(),
        ));
    }

    let vario_family = match family.to_lowercase().as_str() {
        "spherical" => VariogramModelFamily::Spherical,
        "exponential" => VariogramModelFamily::Exponential,
        "gaussian" => VariogramModelFamily::Gaussian,
        _ => {
            return Err(extendr_api::Error::Other(
                "Invalid family. Use: 'spherical', 'exponential', or 'gaussian'".to_string(),
            ))
        }
    };

    let vario_model = VariogramModel {
        family: vario_family,
        nugget,
        partial_sill: psill,
        range,
        wrss: 0.0,
        condition_number: 1.0,
    };

    let coords: Vec<(f64, f64)> = train_x
        .iter()
        .zip(train_y.iter())
        .map(|(&x, &y)| (x, y))
        .collect();

    let simple_kriging = SimpleKriging::new(coords, train_values, vario_model, known_mean).map_err(|e| {
        extendr_api::Error::Other(format!("Simple kriging initialization failed: {}", e))
    })?;

    let result = simple_kriging.predict(pred_x, pred_y).map_err(|e| {
        extendr_api::Error::Other(format!("Simple prediction failed: {}", e))
    })?;

    Ok(list!(
        prediction = result.prediction,
        variance = result.variance,
        std_error = result.std_error,
        ci_lower = result.ci_lower,
        ci_upper = result.ci_upper,
        known_mean = known_mean,
    )
    .into())
}

/// Perform simple kriging grid prediction (vectorized, known mean)
///
/// # Arguments
/// - `train_x`, `train_y`: Training data coordinates
/// - `train_values`: Training data values
/// - `pred_x`, `pred_y`: Grid prediction locations (vectors)
/// - `known_mean`: The known constant mean of the field
/// - `family`, `nugget`, `psill`, `range`: Variogram model parameters
///
/// # Returns
/// List with vectors: prediction, variance, std_error, ci_lower, ci_upper, known_mean
#[cfg(feature = "r")]
#[extendr]
pub fn kriging_predict_grid_simple(
    train_x: Vec<f64>,
    train_y: Vec<f64>,
    train_values: Vec<f64>,
    pred_x: Vec<f64>,
    pred_y: Vec<f64>,
    known_mean: f64,
    family: &str,
    nugget: f64,
    psill: f64,
    range: f64,
) -> Result<Robj> {
    if train_x.len() != train_y.len() || train_x.len() != train_values.len() {
        return Err(extendr_api::Error::Other(
            "Training x, y, and values must have equal length".to_string(),
        ));
    }

    if pred_x.len() != pred_y.len() {
        return Err(extendr_api::Error::Other(
            "pred_x and pred_y must have equal length".to_string(),
        ));
    }

    if train_x.len() < 3 {
        return Err(extendr_api::Error::Other(
            "At least 3 training points required".to_string(),
        ));
    }

    let vario_family = match family.to_lowercase().as_str() {
        "spherical" => VariogramModelFamily::Spherical,
        "exponential" => VariogramModelFamily::Exponential,
        "gaussian" => VariogramModelFamily::Gaussian,
        _ => {
            return Err(extendr_api::Error::Other(
                "Invalid family. Use: 'spherical', 'exponential', or 'gaussian'".to_string(),
            ))
        }
    };

    let vario_model = VariogramModel {
        family: vario_family,
        nugget,
        partial_sill: psill,
        range,
        wrss: 0.0,
        condition_number: 1.0,
    };

    let train_coords: Vec<(f64, f64)> = train_x
        .iter()
        .zip(train_y.iter())
        .map(|(&x, &y)| (x, y))
        .collect();

    let pred_coords: Vec<(f64, f64)> = pred_x
        .iter()
        .zip(pred_y.iter())
        .map(|(&x, &y)| (x, y))
        .collect();

    let simple_kriging = SimpleKriging::new(train_coords, train_values, vario_model, known_mean).map_err(|e| {
        extendr_api::Error::Other(format!("Simple kriging initialization failed: {}", e))
    })?;

    let results = simple_kriging.predict_batch(&pred_coords).map_err(|e| {
        extendr_api::Error::Other(format!("Simple batch prediction failed: {}", e))
    })?;

    let predictions: Vec<f64> = results.iter().map(|r| r.prediction).collect();
    let variances: Vec<f64> = results.iter().map(|r| r.variance).collect();
    let std_errors: Vec<f64> = results.iter().map(|r| r.std_error).collect();
    let ci_lowers: Vec<f64> = results.iter().map(|r| r.ci_lower).collect();
    let ci_uppers: Vec<f64> = results.iter().map(|r| r.ci_upper).collect();

    Ok(list!(
        prediction = predictions,
        variance = variances,
        std_error = std_errors,
        ci_lower = ci_lowers,
        ci_upper = ci_uppers,
        known_mean = known_mean,
    )
    .into())
}

// ──────────────────────────────────────────────────────────────────────────
// Spatio-Temporal Kriging Functions
// ──────────────────────────────────────────────────────────────────────────

/// Perform spatio-temporal kriging prediction at a single point
///
/// # Arguments
/// - `train_x`, `train_y`: Training spatial coordinates
/// - `train_t`: Training temporal coordinates
/// - `train_values`: Training data values
/// - `pred_x`, `pred_y`: Prediction spatial location
/// - `pred_t`: Prediction temporal location
/// - `family_s`, `nugget_s`, `psill_s`, `range_s`: Spatial variogram parameters
/// - `family_t`, `nugget_t`, `psill_t`, `range_t`: Temporal variogram parameters
///
/// # Returns
/// List with: prediction, variance, std_error, ci_lower, ci_upper
#[cfg(feature = "r")]
#[extendr]
pub fn kriging_predict_spacetime(
    train_x: Vec<f64>,
    train_y: Vec<f64>,
    train_t: Vec<f64>,
    train_values: Vec<f64>,
    pred_x: f64,
    pred_y: f64,
    pred_t: f64,
    family_s: &str,
    nugget_s: f64,
    psill_s: f64,
    range_s: f64,
    family_t: &str,
    nugget_t: f64,
    psill_t: f64,
    range_t: f64,
) -> Result<Robj> {
    if train_x.len() != train_y.len() || train_x.len() != train_t.len() || train_x.len() != train_values.len() {
        return Err(extendr_api::Error::Other(
            "Training x, y, t, and values must have equal length".to_string(),
        ));
    }

    if train_x.len() < 4 {
        return Err(extendr_api::Error::Other(
            "At least 4 spatio-temporal training points required".to_string(),
        ));
    }

    let vario_family_s = match family_s.to_lowercase().as_str() {
        "spherical" => VariogramModelFamily::Spherical,
        "exponential" => VariogramModelFamily::Exponential,
        "gaussian" => VariogramModelFamily::Gaussian,
        _ => {
            return Err(extendr_api::Error::Other(
                "Invalid spatial family. Use: 'spherical', 'exponential', or 'gaussian'".to_string(),
            ))
        }
    };

    let vario_family_t = match family_t.to_lowercase().as_str() {
        "spherical" => VariogramModelFamily::Spherical,
        "exponential" => VariogramModelFamily::Exponential,
        "gaussian" => VariogramModelFamily::Gaussian,
        _ => {
            return Err(extendr_api::Error::Other(
                "Invalid temporal family. Use: 'spherical', 'exponential', or 'gaussian'".to_string(),
            ))
        }
    };

    let vario_spatial = VariogramModel {
        family: vario_family_s,
        nugget: nugget_s,
        partial_sill: psill_s,
        range: range_s,
        wrss: 0.0,
        condition_number: 1.0,
    };

    let vario_temporal = VariogramModel {
        family: vario_family_t,
        nugget: nugget_t,
        partial_sill: psill_t,
        range: range_t,
        wrss: 0.0,
        condition_number: 1.0,
    };

    let train_coords_spatial: Vec<(f64, f64)> = train_x
        .iter()
        .zip(train_y.iter())
        .map(|(&x, &y)| (x, y))
        .collect();

    let st_kriging = SpaceTimeKriging::new(train_coords_spatial, train_t, train_values, vario_spatial, vario_temporal)
        .map_err(|e| extendr_api::Error::Other(format!("Space-time kriging initialization failed: {}", e)))?;

    let result = st_kriging.predict(pred_x, pred_y, pred_t).map_err(|e| {
        extendr_api::Error::Other(format!("Space-time prediction failed: {}", e))
    })?;

    Ok(list!(
        prediction = result.prediction,
        variance = result.variance,
        std_error = result.std_error,
        ci_lower = result.ci_lower,
        ci_upper = result.ci_upper,
    )
    .into())
}

/// Perform spatio-temporal kriging grid prediction (vectorized)
///
/// # Arguments
/// - `train_x`, `train_y`: Training spatial coordinates
/// - `train_t`: Training temporal coordinates
/// - `train_values`: Training data values
/// - `pred_x`, `pred_y`: Grid spatial locations (vectors)
/// - `pred_t`: Grid temporal locations (vector, same length as pred_x)
/// - `family_s`, `nugget_s`, `psill_s`, `range_s`: Spatial variogram parameters
/// - `family_t`, `nugget_t`, `psill_t`, `range_t`: Temporal variogram parameters
///
/// # Returns
/// List with vectors: prediction, variance, std_error, ci_lower, ci_upper
#[cfg(feature = "r")]
#[extendr]
pub fn kriging_predict_grid_spacetime(
    train_x: Vec<f64>,
    train_y: Vec<f64>,
    train_t: Vec<f64>,
    train_values: Vec<f64>,
    pred_x: Vec<f64>,
    pred_y: Vec<f64>,
    pred_t: Vec<f64>,
    family_s: &str,
    nugget_s: f64,
    psill_s: f64,
    range_s: f64,
    family_t: &str,
    nugget_t: f64,
    psill_t: f64,
    range_t: f64,
) -> Result<Robj> {
    if train_x.len() != train_y.len() || train_x.len() != train_t.len() || train_x.len() != train_values.len() {
        return Err(extendr_api::Error::Other(
            "Training x, y, t, and values must have equal length".to_string(),
        ));
    }

    if pred_x.len() != pred_y.len() || pred_x.len() != pred_t.len() {
        return Err(extendr_api::Error::Other(
            "Prediction x, y, and t must have equal length".to_string(),
        ));
    }

    if train_x.len() < 4 {
        return Err(extendr_api::Error::Other(
            "At least 4 spatio-temporal training points required".to_string(),
        ));
    }

    let vario_family_s = match family_s.to_lowercase().as_str() {
        "spherical" => VariogramModelFamily::Spherical,
        "exponential" => VariogramModelFamily::Exponential,
        "gaussian" => VariogramModelFamily::Gaussian,
        _ => {
            return Err(extendr_api::Error::Other(
                "Invalid spatial family. Use: 'spherical', 'exponential', or 'gaussian'".to_string(),
            ))
        }
    };

    let vario_family_t = match family_t.to_lowercase().as_str() {
        "spherical" => VariogramModelFamily::Spherical,
        "exponential" => VariogramModelFamily::Exponential,
        "gaussian" => VariogramModelFamily::Gaussian,
        _ => {
            return Err(extendr_api::Error::Other(
                "Invalid temporal family. Use: 'spherical', 'exponential', or 'gaussian'".to_string(),
            ))
        }
    };

    let vario_spatial = VariogramModel {
        family: vario_family_s,
        nugget: nugget_s,
        partial_sill: psill_s,
        range: range_s,
        wrss: 0.0,
        condition_number: 1.0,
    };

    let vario_temporal = VariogramModel {
        family: vario_family_t,
        nugget: nugget_t,
        partial_sill: psill_t,
        range: range_t,
        wrss: 0.0,
        condition_number: 1.0,
    };

    let train_coords_spatial: Vec<(f64, f64)> = train_x
        .iter()
        .zip(train_y.iter())
        .map(|(&x, &y)| (x, y))
        .collect();

    let st_kriging = SpaceTimeKriging::new(train_coords_spatial, train_t, train_values, vario_spatial, vario_temporal)
        .map_err(|e| extendr_api::Error::Other(format!("Space-time kriging initialization failed: {}", e)))?;

    let results = st_kriging.predict_batch(pred_x.iter().zip(pred_y.iter()).map(|(&x, &y)| (x, y)).collect(), pred_t)
        .map_err(|e| extendr_api::Error::Other(format!("Space-time batch prediction failed: {}", e)))?;

    let predictions: Vec<f64> = results.iter().map(|r| r.prediction).collect();
    let variances: Vec<f64> = results.iter().map(|r| r.variance).collect();
    let std_errors: Vec<f64> = results.iter().map(|r| r.std_error).collect();
    let ci_lowers: Vec<f64> = results.iter().map(|r| r.ci_lower).collect();
    let ci_uppers: Vec<f64> = results.iter().map(|r| r.ci_upper).collect();

    Ok(list!(
        prediction = predictions,
        variance = variances,
        std_error = std_errors,
        ci_lower = ci_lowers,
        ci_upper = ci_uppers,
    )
    .into())
}

// ──────────────────────────────────────────────────────────────────────────
// extendr macros
// ──────────────────────────────────────────────────────────────────────────

#[cfg(feature = "r")]
extendr_module! {
    mod wbspatialstats;
    fn estimate_variogram;
    fn fit_variogram;
    fn kriging_predict;
    fn kriging_predict_grid;
    fn kriging_cross_validate;
}
