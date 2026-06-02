#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::ef::indicator_by_assets;
use crate::indicators::simd_indicators::simd_types::F64Constants;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::ef::indicator_by_options;

use std::simd::{cmp::SimdPartialEq, num::SimdFloat, Select, Simd};
/// Computes one KAMA step across `N` asset/option lanes using SIMD parallelism.
///
/// Calculates the Efficiency Ratio (|net change| / |total path|) and uses it to
/// blend the fast and slow EMA smoothing constants. When `sum == 0` (perfectly
/// efficient or flat market) the smoothing constant defaults to `1.0` (full tracking).
/// FMA instructions are used throughout to maximise throughput.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    sum: &mut Simd<f64, N>,
    values: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
) -> Simd<f64, N> {
    let (value, prev_value, last_value, old_value) = values;
    let mask = sum.simd_ne(F64Constants::ZERO);
    *sum += (value - prev_value).abs() - (last_value - old_value).abs();

    mask.select(
        (value - last_value).abs() / *sum, // When sum != 0.0
        F64Constants::ONE,                // When sum == 0.0, use 1.0
    )
}
