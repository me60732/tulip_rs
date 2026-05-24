#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::md::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::md::indicator_by_options;

pub mod imports {
    pub(crate) use crate::indicators::simd_indicators::{
        simd_types::F64Constants, sma_simd::calc_simd as sma_calc_simd,
    };
    pub(crate) use std::simd::{num::SimdFloat, Simd};
}
pub mod assets {
    use super::imports::*;
    /// Computes one Mean Deviation step across `N` asset lanes using SIMD parallelism.
    ///
    /// Updates the SMA via a sliding-window sum and then computes the mean absolute
    /// deviation of all values in the current window from the new SMA.
    #[inline(always)]
    pub fn calc_simd<const N: usize>(
        value: Simd<f64, N>,
        prev_value: Simd<f64, N>,
        slice: &[Simd<f64, N>],
        sum: &mut Simd<f64, N>,
        multiplier: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let sma = sma_calc_simd(sum, value, prev_value, multiplier);

        let md = calc_md_simd(slice, sma, multiplier);
        (md, sma)
    }
    /// Computes the per-lane mean absolute deviation from `sma` over the values in `slice`.
    ///
    /// Sums `|x - sma|` for each value and multiplies by `multiplier` (= 1/period).
    /// Clamps the result to `EPSILON` to avoid exact-zero returns.
    #[inline(always)]
    pub fn calc_md_simd<const N: usize>(
        slice: &[Simd<f64, N>],
        sma: Simd<f64, N>,
        multiplier: Simd<f64, N>,
    ) -> Simd<f64, N> {
        (slice.iter().map(|&x| (x - sma).abs()).sum::<Simd<f64, N>>() * multiplier)
            .simd_max(F64Constants::EPSILON)
    }
}
pub mod options {
    use super::imports::*;
    use crate::indicators::md::calc_md_simd;
    /// Computes one Mean Deviation step for `N` option lanes (different periods) on a single asset.
    ///
    /// Advances the shared SMA, then for each lane reads its own period-length window from
    /// the raw pointer to compute its individual mean deviation.
    #[inline(always)]
    pub fn calc_simd<const N: usize>(
        value: Simd<f64, N>,
        prev_value: Simd<f64, N>,
        real: [*const f64; N],
        sum: &mut Simd<f64, N>,
        multiplier: Simd<f64, N>,
        periods: [usize; N],
        i: [usize; N],
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let sma = sma_calc_simd(sum, value, prev_value, multiplier);
        let mut md = [0.0; N];
        let sma_ref = sma.as_array();
        let multiplier_ref = multiplier.as_array();

        //let take = (i + Simd::splat(1)) - start;
        for (lane, (&i, &period)) in i.iter().zip(periods.iter()).enumerate() {
            let start = i + 1 - period;
            let slice = unsafe { std::slice::from_raw_parts(real[lane].add(start), period) };
            md[lane] = calc_md_simd::<4>(slice, sma_ref[lane], multiplier_ref[lane]);
        }
        (Simd::from_array(md), sma)
    }
}
