pub use crate::indicators::simd_indicators::aroon_simd::SimdState;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::aroonosc::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::aroonosc::indicator_by_options;

use std::simd::Simd;
pub mod assets {
    use super::*;
    use crate::indicators::simd_indicators::aroon_simd::assets::Calc as AroonCalc;
    pub trait Calc<const N: usize> {
        unsafe fn calc_unchecked_simd<const CHUNK_SIZE: usize>(
            self: &mut Self,
            high: [*const f64; N],
            low: [*const f64; N],
            i: usize,
            period: usize,
            multiplier: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    }
    impl<const N: usize> Calc<N> for SimdState<N> {
        #[inline(always)]
        unsafe fn calc_unchecked_simd<const CHUNK_SIZE: usize>(
            self: &mut Self,
            high: [*const f64; N],
            low: [*const f64; N],
            i: usize,
            period: usize,
            multiplier: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let (aroon_down, aroon_up) = AroonCalc::calc_unchecked_simd::<CHUNK_SIZE>(
                self, high, low, i, period, multiplier,
            );

            (aroon_up - aroon_down, aroon_down, aroon_up)
        }
    }
}
pub mod options {
    use super::*;
    pub use crate::indicators::simd_indicators::aroon_simd::options::Calc as AroonCalc;
    pub trait Calc<const N: usize> {
        unsafe fn calc_unchecked_simd(
            self: &mut Self,
            high: [*const f64; N],
            low: [*const f64; N],
            i: Simd<usize, N>,
            period: Simd<usize, N>,
            multiplier: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    }
    impl<const N: usize> Calc<N> for SimdState<N> {
        #[inline(always)]
        unsafe fn calc_unchecked_simd(
            self: &mut Self,
            high: [*const f64; N],
            low: [*const f64; N],
            i: Simd<usize, N>,
            period: Simd<usize, N>,
            multiplier: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let (aroon_down, aroon_up) =
                AroonCalc::calc_unchecked_simd(self, high, low, i, period, multiplier);

            (aroon_up - aroon_down, aroon_down, aroon_up)
        }
    }
}
