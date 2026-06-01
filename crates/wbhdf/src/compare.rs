use crate::error::{WbhdfError, WbhdfResult};

#[derive(Debug, Clone, PartialEq)]
pub struct F32ComparisonSummary {
    pub compared_len: usize,
    pub mismatches: usize,
    pub max_abs_diff: f32,
    pub first_mismatch_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct F64ComparisonSummary {
    pub compared_len: usize,
    pub mismatches: usize,
    pub max_abs_diff: f64,
    pub first_mismatch_index: Option<usize>,
}

pub fn compare_f32_exact(actual: &[f32], expected: &[f32]) -> WbhdfResult<F32ComparisonSummary> {
    compare_f32_with_tolerance(actual, expected, 0.0)
}

pub fn compare_f32_with_tolerance(
    actual: &[f32],
    expected: &[f32],
    abs_tolerance: f32,
) -> WbhdfResult<F32ComparisonSummary> {
    if actual.len() != expected.len() {
        return Err(WbhdfError::InvalidInput(format!(
            "array length mismatch: actual={} expected={}",
            actual.len(),
            expected.len()
        )));
    }
    if abs_tolerance.is_sign_negative() || !abs_tolerance.is_finite() {
        return Err(WbhdfError::InvalidInput(
            "abs_tolerance must be finite and >= 0".to_string(),
        ));
    }

    let mut mismatches = 0usize;
    let mut max_abs_diff = 0.0_f32;
    let mut first_mismatch_index = None;

    for (idx, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        let diff = (a - e).abs();
        if diff > max_abs_diff {
            max_abs_diff = diff;
        }
        if diff > abs_tolerance {
            mismatches += 1;
            if first_mismatch_index.is_none() {
                first_mismatch_index = Some(idx);
            }
        }
    }

    Ok(F32ComparisonSummary {
        compared_len: actual.len(),
        mismatches,
        max_abs_diff,
        first_mismatch_index,
    })
}

pub fn compare_f64_exact(actual: &[f64], expected: &[f64]) -> WbhdfResult<F64ComparisonSummary> {
    compare_f64_with_tolerance(actual, expected, 0.0)
}

pub fn compare_f64_with_tolerance(
    actual: &[f64],
    expected: &[f64],
    abs_tolerance: f64,
) -> WbhdfResult<F64ComparisonSummary> {
    if actual.len() != expected.len() {
        return Err(WbhdfError::InvalidInput(format!(
            "array length mismatch: actual={} expected={}",
            actual.len(),
            expected.len()
        )));
    }
    if abs_tolerance.is_sign_negative() || !abs_tolerance.is_finite() {
        return Err(WbhdfError::InvalidInput(
            "abs_tolerance must be finite and >= 0".to_string(),
        ));
    }

    let mut mismatches = 0usize;
    let mut max_abs_diff = 0.0_f64;
    let mut first_mismatch_index = None;

    for (idx, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        let diff = (a - e).abs();
        if diff > max_abs_diff {
            max_abs_diff = diff;
        }
        if diff > abs_tolerance {
            mismatches += 1;
            if first_mismatch_index.is_none() {
                first_mismatch_index = Some(idx);
            }
        }
    }

    Ok(F64ComparisonSummary {
        compared_len: actual.len(),
        mismatches,
        max_abs_diff,
        first_mismatch_index,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        compare_f32_exact, compare_f32_with_tolerance, compare_f64_exact,
        compare_f64_with_tolerance,
    };

    #[test]
    fn reports_exact_match_without_mismatches() {
        let summary = compare_f32_exact(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]).unwrap();
        assert_eq!(summary.compared_len, 3);
        assert_eq!(summary.mismatches, 0);
        assert_eq!(summary.max_abs_diff, 0.0);
        assert_eq!(summary.first_mismatch_index, None);
    }

    #[test]
    fn reports_toleranced_match_when_diffs_are_small() {
        let summary = compare_f32_with_tolerance(
            &[1.0, 2.001, 3.0],
            &[1.0, 2.0, 2.9995],
            0.01,
        )
        .unwrap();
        assert_eq!(summary.mismatches, 0);
        assert!(summary.max_abs_diff > 0.0);
    }

    #[test]
    fn reports_mismatches_and_first_index() {
        let summary = compare_f32_with_tolerance(
            &[1.0, 2.5, 3.0, 4.25],
            &[1.0, 2.0, 3.0, 4.0],
            0.1,
        )
        .unwrap();
        assert_eq!(summary.mismatches, 2);
        assert_eq!(summary.first_mismatch_index, Some(1));
        assert!(summary.max_abs_diff >= 0.25);
    }

    #[test]
    fn rejects_mismatched_lengths() {
        let err = compare_f32_exact(&[1.0], &[1.0, 2.0]).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("array length mismatch"));
    }

    #[test]
    fn rejects_negative_tolerance() {
        let err = compare_f32_with_tolerance(&[1.0], &[1.0], -0.001).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("abs_tolerance"));
    }

    #[test]
    fn reports_f64_exact_match_without_mismatches() {
        let summary = compare_f64_exact(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]).unwrap();
        assert_eq!(summary.compared_len, 3);
        assert_eq!(summary.mismatches, 0);
        assert_eq!(summary.max_abs_diff, 0.0);
        assert_eq!(summary.first_mismatch_index, None);
    }

    #[test]
    fn reports_f64_toleranced_match_when_diffs_are_small() {
        let summary = compare_f64_with_tolerance(
            &[1.0, 2.0000001, 3.0],
            &[1.0, 2.0, 2.9999999],
            1e-6,
        )
        .unwrap();
        assert_eq!(summary.mismatches, 0);
        assert!(summary.max_abs_diff > 0.0);
    }

    #[test]
    fn reports_f64_mismatches_and_first_index() {
        let summary = compare_f64_with_tolerance(
            &[1.0, 2.5, 3.0, 4.25],
            &[1.0, 2.0, 3.0, 4.0],
            0.1,
        )
        .unwrap();
        assert_eq!(summary.mismatches, 2);
        assert_eq!(summary.first_mismatch_index, Some(1));
        assert!(summary.max_abs_diff >= 0.25);
    }

    #[test]
    fn rejects_f64_negative_tolerance() {
        let err = compare_f64_with_tolerance(&[1.0], &[1.0], -0.001).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("abs_tolerance"));
    }
}
