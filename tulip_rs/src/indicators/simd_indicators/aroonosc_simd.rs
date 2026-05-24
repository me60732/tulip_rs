/// Re-uses [`aroon_simd::SimdState`] as the state for the Aroon Oscillator since both
/// indicators track the same rolling min/max windows.
pub use crate::indicators::simd_indicators::aroon_simd::SimdState;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::aroonosc::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::aroonosc::indicator_by_options;

use std::simd::Simd;
pub mod assets {
    use super::*;
    use crate::indicators::simd_indicators::aroon_simd::assets::Calc as AroonCalc;

    /// SIMD computation trait for the Aroon Oscillator, operating on `N` asset lanes simultaneously.
    pub trait Calc<const N: usize> {
        /// Computes the Aroon Oscillator (plus Aroon Down and Up) for one bar across `N` asset lanes.
        ///
        /// Delegates to [`AroonCalc::calc_unchecked_simd`] to obtain `(aroon_down, aroon_up)`,
        /// then returns the oscillator as `aroon_up - aroon_down` along with both components.
        ///
        /// # Safety
        ///
        /// Same constraints as [`aroon_simd::assets::Calc::calc_unchecked_simd`]: `high[lane]`
        /// and `low[lane]` must point to valid memory at index `i`, with `i >= period`.
        ///
        /// # Returns
        ///
        /// A tuple `(aroonosc, aroon_down, aroon_up)` of SIMD vectors for all `N` lanes.
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

    /// SIMD computation trait for the Aroon Oscillator, operating on `N` option lanes simultaneously.
    ///
    /// `i` and `period` are SIMD vectors so each lane can be at a different bar position and
    /// use a different lookback period.
    pub trait Calc<const N: usize> {
        /// Computes the Aroon Oscillator (plus Aroon Down and Up) for one bar across `N` option lanes.
        ///
        /// Delegates to [`AroonCalc::calc_unchecked_simd`] and returns
        /// `(aroon_up - aroon_down, aroon_down, aroon_up)`.
        ///
        /// # Safety
        ///
        /// Callers must ensure that `high[lane]` and `low[lane]` point to valid memory at
        /// `i[lane]`, and that `i[lane] >= period[lane]` for every lane.
        ///
        /// # Returns
        ///
        /// A tuple `(aroonosc, aroon_down, aroon_up)` of SIMD vectors for all `N` lanes.
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
