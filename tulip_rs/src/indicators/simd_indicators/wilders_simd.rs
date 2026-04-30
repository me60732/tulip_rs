use crate::indicators::simd_indicators::simd_types::F64Constants;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::wilders::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::wilders::indicator_by_options;

use std::simd::{Simd, StdFloat};

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

#[inline(always)]
pub fn calc_simd<const N: usize>(
    prev_wilders: Simd<f64, N>,
    value: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> Simd<f64, N> {
    prev_wilders.mul_add(multiplier, value * (F64Constants::ONE - multiplier))
}

#[inline(always)]
pub fn partial_calc_simd<const N: usize>(
    prev_wilders: Simd<f64, N>,
    value: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> Simd<f64, N> {
    prev_wilders.mul_add(multiplier, value)
}
