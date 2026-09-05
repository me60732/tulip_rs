use crate::indicators::kvo::State;
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::kvo::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::kvo::indicator_by_options;
pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::simd_indicators::{
    ema_simd::SimdState as EmaSimdState, simd_types::F64Constants,
};
use crate::types::Warm;
use std::simd::{
    cmp::SimdPartialOrd,
    num::SimdFloat,
    *,
};
/// SIMD-parallel state for computing the Klinger Volume Oscillator (KVO) across `N` assets/options simultaneously.
/// Each field is a SIMD vector where lane `i` corresponds to asset/option `i`.
pub struct SimdState<const N: usize> {
    pub short_ema: EmaSimdState<N>,
    pub long_ema: EmaSimdState<N>,
    pub cm: Simd<f64, N>,
    pub trend: Mask<i64, N>,
    pub prev_hlc: Simd<f64, N>,
    pub prev_hl: Simd<f64, N>,

}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_impl!(
         sub: [(short_ema: EmaSimdState<N>), (long_ema: EmaSimdState<N>)],
         scalar: [cm, prev_hlc, prev_hl],
         buf: [],
         mask: [trend]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);

    #[inline(always)]
    fn calc<'a>(
        &mut self,
        inputs: Self::Inputs<'a>,
    ) -> Self::Outputs {
        // Extract multipliers once (minor optimization)

        let vf = self.calc_vf_simd(inputs);
        let short_ema = self.short_ema.calc(vf);
        let long_ema = self.long_ema.calc(vf);
        (short_ema - long_ema, short_ema, long_ema)
    }
}
impl<const N: usize> SimdState<N> {
    /// Computes the Volume Force (VF) component of KVO across `N` lanes using SIMD parallelism.
    ///
    /// Detects trend changes by comparing the current HLC sum to the previous bar's value.
    /// On a trend reversal, the cumulative money flow (`cm`) is seeded with the previous bar's
    /// high-low range. `cm` is then #[inline(always)]
    #[inline(always)]
    fn calc_vf_simd(
        &mut self,
        (high, low, close, volume): (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
    ) -> Simd<f64, N> {
        let hlc = high + low + close;
        let dm  = high - low;
    
        // true  = was DOWN, now going UP → flip to UP
        // false = was UP,   now going DOWN → flip to DOWN
        let flip_to_up   = !self.trend & hlc.simd_gt(self.prev_hlc);
        let flip_to_down =  self.trend & hlc.simd_lt(self.prev_hlc);
        let changed      = flip_to_up | flip_to_down;
    
        // Reset CM only on direction change
        self.cm = changed.select(self.prev_hl, self.cm);

        self.trend ^= changed;
        
        self.cm += dm.simd_max(F64Constants::EPSILON);
        self.prev_hlc  = hlc;
        self.prev_hl = dm;
    
        // Mask → sign: true (UP) = +1.0, false (DOWN) = -1.0
        let sign = self.trend.select(
            Simd::splat( 1.0_f64),
            Simd::splat(-1.0_f64),
        );
    
        volume
            * (dm / self.cm)
                .mul_add(F64Constants::TWO, F64Constants::NEG_ONE)
                .abs()
            * F64Constants::HUNDRED
            * sign
    }

}
