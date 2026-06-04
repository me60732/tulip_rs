#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::ef::indicator_by_assets;
use crate::indicators::simd_indicators::simd_types::F64Constants;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::ef::indicator_by_options;

use std::simd::{cmp::SimdPartialEq, num::SimdFloat, Select, Simd};
/// Computes one EF (Efficiency Ratio) step across `N` asset/option lanes using SIMD parallelism.
///
/// Calculates the Efficiency Ratio `|net change| / |total path|` for each lane.
/// When `sum == 0` (price did not move, or every up-move was exactly cancelled by a
/// down-move) the ER is defined as `0.0`, matching the scalar implementation.
/// Forcing `1.0` here would be incorrect: it would signal a perfect trend during
/// stagnant or noisy conditions, inverting the intended behaviour of adaptive
/// indicators such as KAMA that consume this value.
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
        F64Constants::ZERO,                // When sum == 0.0, return 0.0 (no efficiency)
    )
}
