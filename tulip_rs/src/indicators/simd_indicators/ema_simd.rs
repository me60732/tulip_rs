#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::ema::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::ema::indicator_by_options;
use std::simd::{Simd, StdFloat};

/// Computes the EMA multiplier pair for `N` lanes with potentially different periods.
///
/// Returns `(per, 1 - per)` where `per = 2.0 / (period + 1.0)` for each lane,
/// suitable for use with [`calc_simd`].
///
/// # Arguments
///
/// * `periods` - Array of per-lane EMA periods.
///
/// # Returns
///
/// A tuple `(multiplier, inv_multiplier)` as SIMD vectors.
#[inline(always)]
pub fn multiplier_simd<const N: usize>(periods: [usize; N]) -> (Simd<f64, N>, Simd<f64, N>) {
    // Convert usize array to f64 array
    let mut f64_periods = [0.0; N];
    for i in 0..N {
        f64_periods[i] = periods[i] as f64;
    }

    // Create SIMD vectors
    let periods_simd = Simd::<f64, N>::from_array(f64_periods);
    let two = Simd::<f64, N>::splat(2.0);
    let one = Simd::<f64, N>::splat(1.0);

    // Calculate: 2.0 / (period + 1.0)
    let per = two / (periods_simd + one);
    (per, one - per)
}

/// Computes one bar of the Exponential Moving Average (EMA) for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Applies the standard EMA formula: `prev_ema * inv_multiplier + value * multiplier`.
///
/// # Arguments
///
/// * `value` - Current prices for this bar.
/// * `prev_ema` - Previous EMA values for each lane.
/// * `multipliers` - Tuple `(multiplier, inv_multiplier)` from [`multiplier_simd`].
///
/// # Returns
///
/// Updated EMA values for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    value: Simd<f64, N>,
    prev_ema: Simd<f64, N>,
    multipliers: (Simd<f64, N>, Simd<f64, N>),
) -> Simd<f64, N> {
    let (multiplier, inv_multiplier) = multipliers;
    //prev_ema * inv_multiplier + value * multiplier
    prev_ema.mul_add(inv_multiplier, value * multiplier)
}
