//! Forward quantization for JPEG 2000 encoding.
//!
//! - Lossless (reversible 5-3): No quantization, just sign/magnitude conversion
//! - Lossy (irreversible 9-7): Scalar deadzone quantization with step sizes
//!   derived from the DWT subband gain norms.

use alloc::vec;
use alloc::vec::Vec;

/// Quantization parameters for a single subband.
#[derive(Debug, Clone, Copy)]
pub(crate) struct QuantStepSize {
    pub(crate) exponent: u16,
    pub(crate) mantissa: u16,
}

impl QuantStepSize {
    /// Compute the actual step size Δ = 2^(exp - guard_bits) × (1 + mantissa/2048).
    fn delta(&self, guard_bits: u8) -> f32 {
        let rb = self.exponent as i32 - guard_bits as i32;
        let base = if rb >= 0 {
            (1u32 << rb) as f32
        } else {
            1.0 / (1u32 << (-rb)) as f32
        };
        base * (1.0 + self.mantissa as f32 / 2048.0)
    }
}

/// Compute default quantization step sizes for the irreversible 9-7 transform.
///
/// The step sizes are derived from the DWT 9-7 subband gain norms (Table E.1 in T.800).
/// For lossless mode, step sizes are not used (exponents store bit depth info only).
pub(crate) fn compute_step_sizes(
    bit_depth: u8,
    num_decompositions: u8,
    reversible: bool,
    guard_bits: u8,
) -> Vec<QuantStepSize> {
    let mut step_sizes = Vec::new();

    if reversible {
        // For reversible 5-3, QCD stores the subband exponent only.
        // The decoder reconstructs the number of bitplanes as:
        //   Mb = guard_bits + exponent - 1
        // For lossless coding we therefore need exponents that reproduce the
        // reversible subband dynamic range:
        //   LL => bit_depth + 0
        //   HL/LH => bit_depth + 1
        //   HH => bit_depth + 2
        // This gain depends on subband orientation, not decomposition level.
        step_sizes.push(QuantStepSize {
            exponent: bit_depth as u16,
            mantissa: 0,
        });

        for _ in 0..num_decompositions {
            step_sizes.push(QuantStepSize {
                exponent: bit_depth as u16 + 1,
                mantissa: 0,
            });
            step_sizes.push(QuantStepSize {
                exponent: bit_depth as u16 + 1,
                mantissa: 0,
            });
            step_sizes.push(QuantStepSize {
                exponent: bit_depth as u16 + 2,
                mantissa: 0,
            });
        }
    } else {
        // For irreversible 9-7: compute step sizes from norm gains
        // Base step size for the image
        let base_step = 1.0 / (1u32 << bit_depth) as f32;

        // DWT 9-7 analysis gain norms (squared), per Table E.1
        // These are the L2 norms of the analysis basis functions.
        let ll_gain = 1.0f64;
        let hl_gains: Vec<f64> = (0..num_decompositions)
            .map(|l| dwt97_subband_gain(l, false, true))
            .collect();
        let lh_gains: Vec<f64> = (0..num_decompositions)
            .map(|l| dwt97_subband_gain(l, true, false))
            .collect();
        let hh_gains: Vec<f64> = (0..num_decompositions)
            .map(|l| dwt97_subband_gain(l, true, true))
            .collect();

        // LL subband
        step_sizes.push(step_from_gain(
            base_step as f64 / ll_gain,
            guard_bits,
            bit_depth,
        ));

        for level in 0..num_decompositions as usize {
            step_sizes.push(step_from_gain(
                base_step as f64 / hl_gains[level],
                guard_bits,
                bit_depth,
            ));
            step_sizes.push(step_from_gain(
                base_step as f64 / lh_gains[level],
                guard_bits,
                bit_depth,
            ));
            step_sizes.push(step_from_gain(
                base_step as f64 / hh_gains[level],
                guard_bits,
                bit_depth,
            ));
        }
    }

    step_sizes
}

/// Approximate DWT 9-7 subband gain for a given decomposition level.
fn dwt97_subband_gain(level: u8, high_row: bool, high_col: bool) -> f64 {
    // Approximate gains based on analysis filter norms
    // Low-pass analysis gain ≈ 1.0 per level (normalized)
    // High-pass analysis gain increases with level
    let low = 1.0f64;
    let high = 2.0f64;

    let row_gain = if high_row {
        high * (1u64 << level) as f64
    } else {
        low
    };
    let col_gain = if high_col {
        high * (1u64 << level) as f64
    } else {
        low
    };

    row_gain * col_gain
}

fn step_from_gain(step: f64, guard_bits: u8, bit_depth: u8) -> QuantStepSize {
    if step <= 0.0 {
        return QuantStepSize {
            exponent: guard_bits as u16 + bit_depth as u16,
            mantissa: 0,
        };
    }

    let log2_step = step.log2();
    let exponent = -(log2_step.floor() as i32) + guard_bits as i32;
    let exponent = exponent.clamp(0, 31) as u16;

    // mantissa = (step / 2^(-exponent+guard_bits) - 1) * 2048
    let reconstructed_base = 2.0f64.powi(-(exponent as i32) + guard_bits as i32);
    let mantissa = if reconstructed_base > 0.0 {
        ((step / reconstructed_base - 1.0) * 2048.0).round() as u16
    } else {
        0
    };

    QuantStepSize {
        exponent,
        mantissa: mantissa.min(2047),
    }
}

/// Quantize wavelet coefficients for a single subband.
///
/// For lossless: converts f32 to i32 (round to nearest integer).
/// For lossy: applies scalar deadzone quantization.
///
/// Returns (magnitude, sign) pairs packed as i32 values.
pub(crate) fn quantize_subband(
    coefficients: &[f32],
    step_size: &QuantStepSize,
    guard_bits: u8,
    reversible: bool,
) -> Vec<i32> {
    if reversible {
        // No quantization: round to nearest integer
        coefficients.iter().map(|&c| c.round() as i32).collect()
    } else {
        let delta = step_size.delta(guard_bits);
        if delta <= 0.0 {
            return vec![0i32; coefficients.len()];
        }
        let inv_delta = 1.0 / delta;

        coefficients
            .iter()
            .map(|&c| {
                // Deadzone quantization: q = sign(c) * floor(|c| / Δ)
                let sign = if c < 0.0 { -1 } else { 1 };
                let magnitude = (c.abs() * inv_delta).floor() as i32;
                sign * magnitude
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lossless_quantize() {
        let coeffs = vec![10.0, -5.0, 3.7, -8.2, 0.0];
        let step = QuantStepSize {
            exponent: 12,
            mantissa: 0,
        };
        let result = quantize_subband(&coeffs, &step, 1, true);
        assert_eq!(result, vec![10, -5, 4, -8, 0]);
    }

    #[test]
    fn test_lossy_quantize() {
        let coeffs = vec![10.0, -5.0, 0.3, -0.1];
        let step = QuantStepSize {
            exponent: 1,
            mantissa: 0,
        };
        let delta = step.delta(1);
        assert!((delta - 1.0).abs() < 0.01);

        let result = quantize_subband(&coeffs, &step, 1, false);
        assert_eq!(result[0], 10);
        assert_eq!(result[1], -5);
        assert_eq!(result[2], 0); // Below deadzone
        assert_eq!(result[3], 0); // Below deadzone
    }

    #[test]
    fn test_compute_step_sizes_reversible() {
        let steps = compute_step_sizes(8, 3, true, 1);
        // 1 LL + 3 levels × 3 subbands = 10
        assert_eq!(steps.len(), 10);
        // All mantissas should be 0 for reversible
        assert!(steps.iter().all(|s| s.mantissa == 0));
        let exponents: Vec<u16> = steps.iter().map(|s| s.exponent).collect();
        assert_eq!(exponents, vec![8, 9, 9, 10, 9, 9, 10, 9, 9, 10]);
    }

    #[test]
    fn test_compute_step_sizes_irreversible() {
        let steps = compute_step_sizes(8, 3, false, 1);
        assert_eq!(steps.len(), 10);
    }
}
