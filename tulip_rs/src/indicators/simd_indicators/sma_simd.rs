#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::sma::{indicator_by_assets, init_state};

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::sma::indicator_by_options;
use std::simd::Simd;

/*#[inline(always)]
pub fn multiplier_simd<const N: usize>(periods: [usize; N]) -> Simd<f64, N> {
    // Convert usize array to f64 array
    let mut f64_periods = [0.0; N];
    for i in 0..N {
        f64_periods[i] = periods[i] as f64;
    }

    // Create SIMD vectors
    let periods_simd = Simd::<f64, N>::from_array(f64_periods);
    let one = Simd::<f64, N>::splat(1.0);

    // Calculate: 1.0 / period
    let per = one / periods_simd;
    per
}*/

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
