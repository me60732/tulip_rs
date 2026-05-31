#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::wilders::indicator_by_assets;
use crate::indicators::simd_indicators::simd_types::F64Constants;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::wilders::indicator_by_options;

use std::simd::{Simd, StdFloat};

/// Initialises the Wilder's Smoothing SIMD state from raw input slices.
///
/// Computes the simple average of the first `period` values for each lane
/// as the seed for subsequent exponential smoothing.
///
/// # Arguments
///
/// * `inputs` - Per-lane input slices; must each contain at least `period` values.
/// * `period` - Number of bars to average for the initial smoothed value.
///
/// # Returns
///
/// SIMD vector containing the initial Wilder's smoothed value for each lane.
pub fn init_state<'a, const N: usize>(inputs: &[&'a [f64]; N], period: usize) -> Simd<f64, N> {
    let input_ptrs: [*const f64; N] = std::array::from_fn(|i| inputs[i].as_ptr());
    let mut wilders = Simd::splat(0.0);
    for i in 0..period {
        let values = Simd::from_array(std::array::from_fn(|j| unsafe { *input_ptrs[j].add(i) }));
        wilders += values;
    }

    wilders /= Simd::splat(period as f64);

    wilders
}

/// Computes one bar of Wilder's Smoothing for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Applies `prev_wilders * multiplier + value * (1 - multiplier)` for each lane.
///
/// # Arguments
///
/// * `prev_wilders` - Previous smoothed values for each lane.
/// * `value` - New input values for this bar.
/// * `multiplier` - Per-lane decay factor `(period - 1) / period`.
///
/// # Returns
///
/// Updated Wilder's smoothed values for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    prev_wilders: Simd<f64, N>,
    value: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> Simd<f64, N> {
    prev_wilders.mul_add(multiplier, value * (F64Constants::ONE - multiplier))
}
#[inline(always)]
pub fn calc_simd_full<const N: usize>(prev_wilders: Simd<f64, N>, value: Simd<f64, N>, multipliers: (Simd<f64, N>, Simd<f64, N>)) -> Simd<f64, N> {
    prev_wilders.mul_add(multipliers.0, value * multipliers.1)
}
/// Computes a partial Wilder's Smoothing step without subtracting the decay residual.
///
/// Applies `prev_wilders * multiplier + value` for each lane, omitting the
/// `(1 - multiplier)` weight on `value`. Used internally for already-scaled inputs.
///
/// # Arguments
///
/// * `prev_wilders` - Previous smoothed values for each lane.
/// * `value` - Pre-scaled new input values for this bar.
/// * `multiplier` - Per-lane decay factor `(period - 1) / period`.
///
/// # Returns
///
/// Partially updated smoothed values for all `N` lanes.
#[inline(always)]
pub fn partial_calc_simd<const N: usize>(
    prev_wilders: Simd<f64, N>,
    value: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> Simd<f64, N> {
    prev_wilders.mul_add(multiplier, value)
}
