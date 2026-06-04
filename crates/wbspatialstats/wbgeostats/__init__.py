"""
wbgeostats - Production-grade geostatistics kriging library

Python API for kriging, variography, and spatial inference.

Example:
    >>> from wbgeostats import OrdinaryKriging, estimate_variogram, fit_variogram
    >>> 
    >>> coords = [(0,0), (1,0), (0,1), (1,1)]
    >>> values = [10.0, 20.0, 15.0, 25.0]
    >>>
    >>> vario = estimate_variogram(coords, values)
    >>> fitted_model = fit_variogram(vario['lags'], vario['semivariances'], 'spherical')
    >>> 
    >>> kriging = OrdinaryKriging(coords, values, fitted_model)
    >>> prediction = kriging.predict(0.5, 0.5)
    >>> print(f"Predicted: {prediction.prediction:.2f} ± {prediction.std_error:.2f}")
"""

__version__ = "0.1.0"
__author__ = "Whitebox Geospatial Inc."
__license__ = "AGPL-3.0-or-later"

try:
    # Import compiled PyO3 extension
    from ._core import (
        VariogramModel,
        KrigingResult,
        OrdinaryKriging,
        estimate_variogram,
        fit_variogram,
        cross_validate_kriging,
    )
    
    __all__ = [
        "VariogramModel",
        "KrigingResult",
        "OrdinaryKriging",
        "estimate_variogram",
        "fit_variogram",
        "cross_validate_kriging",
    ]
    
except ImportError as e:
    raise ImportError(
        "wbgeostats Python bindings not installed. "
        "Build with: maturin develop"
    ) from e
