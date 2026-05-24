#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::sma::{indicator_by_assets, init_state};

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::sma::indicator_by_options;
use std::simd::Simd;

/// Advances one bar of the Simple Moving Average (SMA) for `N` asset lanes simultaneously.
///
/// The SMA is maintained as a running sum. Each step adds the new value and removes the value
/// that is dropping off the window, then multiplies by `1 / period` to get the average.
///
/// # Arguments
///
/// * `sum`       — Mutable reference to the SIMD vector holding the running window sum for each lane.
/// * `value`     — The incoming bar value for each lane.
/// * `prev_value`— The value leaving the window (i.e. `real[i - period]`) for each lane.
/// * `multiplier`— `1.0 / period` broadcast to all lanes.
///
/// # Returns
///
/// The SMA for the current bar across all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    sum: &mut Simd<f64, N>,
    value: Simd<f64, N>,
    prev_value: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> Simd<f64, N> {
    *sum += value - prev_value;
    *sum * multiplier
}
