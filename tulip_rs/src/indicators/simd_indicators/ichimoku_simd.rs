use crate::indicators::ichimoku::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::ichimoku::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::ichimoku::indicator_by_options;

use crate::indicators::simd_indicators::{
    max_simd::SimdState as SimdMaxState, min_simd::SimdState as SimdMinState,
};
use std::simd::Simd;

/// SIMD-parallel state for computing the Ichimoku Cloud across `N` assets or option-sets simultaneously.
///
/// Wraps six min/max ring-buffer SIMD states: one pair each for the short (Tenkan-sen),
/// medium (Kijun-sen), and ultra-long (Senkou Span B) lookback windows.
pub struct SimdState<const N: usize> {
    short_min_state: SimdMinState<N>,
    short_max_state: SimdMaxState<N>,
    medium_min_state: SimdMinState<N>,
    medium_max_state: SimdMaxState<N>,
    long_min_state: SimdMinState<N>,
    long_max_state: SimdMaxState<N>,
}

impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single `SimdState`,
    /// packing each min/max ring-buffer field into the corresponding SIMD lane.
    pub fn new(states: &mut [&mut State]) -> Self {
        let mut short_min = Vec::with_capacity(N);
        let mut short_max = Vec::with_capacity(N);
        let mut medium_min = Vec::with_capacity(N);
        let mut medium_max = Vec::with_capacity(N);
        let mut long_min = Vec::with_capacity(N);
        let mut long_max = Vec::with_capacity(N);

        for state in states.iter_mut() {
            short_min.push(&mut state.short_min_state);
            short_max.push(&mut state.short_max_state);
            medium_min.push(&mut state.medium_min_state);
            medium_max.push(&mut state.medium_max_state);
            long_min.push(&mut state.long_min_state);
            long_max.push(&mut state.long_max_state);
        }

        Self {
            short_min_state: SimdMinState::new(&short_min),
            short_max_state: SimdMaxState::new(&short_max),
            medium_min_state: SimdMinState::new(&medium_min),
            medium_max_state: SimdMaxState::new(&medium_max),
            long_min_state: SimdMinState::new(&long_min),
            long_max_state: SimdMaxState::new(&long_max),
        }
    }

    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
    /// avoiding allocation compared to a `to_states` conversion.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut short_min_refs = Vec::with_capacity(N);
        let mut short_max_refs = Vec::with_capacity(N);
        let mut medium_min_refs = Vec::with_capacity(N);
        let mut medium_max_refs = Vec::with_capacity(N);
        let mut long_min_refs = Vec::with_capacity(N);
        let mut long_max_refs = Vec::with_capacity(N);

        for state in states.iter_mut() {
            short_min_refs.push(&mut state.short_min_state);
            short_max_refs.push(&mut state.short_max_state);
            medium_min_refs.push(&mut state.medium_min_state);
            medium_max_refs.push(&mut state.medium_max_state);
            long_min_refs.push(&mut state.long_min_state);
            long_max_refs.push(&mut state.long_max_state);
        }

        self.short_min_state.write_states(&mut short_min_refs);
        self.short_max_state.write_states(&mut short_max_refs);
        self.medium_min_state.write_states(&mut medium_min_refs);
        self.medium_max_state.write_states(&mut medium_max_refs);
        self.long_min_state.write_states(&mut long_min_refs);
        self.long_max_state.write_states(&mut long_max_refs);
    }
}

pub mod assets {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::assets::Calc as CalcMax, min_simd::assets::Calc as CalcMin,
    };

    /// SIMD computation trait for the Ichimoku Cloud, operating on `N` asset lanes simultaneously.
    pub trait Calc<const N: usize> {
        /// Computes all four Ichimoku lines for one bar across `N` asset lanes using SIMD.
        ///
        /// Delegates to six underlying min/max SIMD ring-buffer states (short, medium, ultra-long)
        /// and derives conversion, base, Senkou Span A, and Senkou Span B in parallel.
        ///
        /// `CS`, `CM`, `CL` independently tune the inner ring-buffer SIMD rescan width for
        /// the short, medium (long), and ultra-long windows respectively — matching the three
        /// const generics used by the scalar `cycle::<CS, CM, CL>` dispatch.
        ///
        /// # Safety
        ///
        /// `high[lane]` and `low[lane]` must point to valid memory at `i`, with
        /// `i >= ultra_look_back`. Callers must ensure the ring-buffer states were
        /// initialised (via [`State::init_state`]) before the first call.
        ///
        /// # Returns
        ///
        /// `(conversion, base, span_a, span_b)` as SIMD vectors for all `N` lanes.
        unsafe fn calc_unchecked_simd<const CS: usize, const CM: usize, const CL: usize>(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            i: usize,
            short_look_back: usize,
            long_look_back: usize,
            ultra_look_back: usize,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    }

    impl<const N: usize> Calc<N> for SimdState<N> {
        #[inline(always)]
        unsafe fn calc_unchecked_simd<const CS: usize, const CM: usize, const CL: usize>(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            i: usize,
            short_look_back: usize,
            long_look_back: usize,
            ultra_look_back: usize,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let (short_min, _) =
                self.short_min_state
                    .calc_unchecked_simd::<CS>(low, i, short_look_back);
            let (short_max, _) =
                self.short_max_state
                    .calc_unchecked_simd::<CS>(high, i, short_look_back);
            let (medium_min, _) =
                self.medium_min_state
                    .calc_unchecked_simd::<CM>(low, i, long_look_back);
            let (medium_max, _) =
                self.medium_max_state
                    .calc_unchecked_simd::<CM>(high, i, long_look_back);
            let (long_min, _) =
                self.long_min_state
                    .calc_unchecked_simd::<CL>(low, i, ultra_look_back);
            let (long_max, _) =
                self.long_max_state
                    .calc_unchecked_simd::<CL>(high, i, ultra_look_back);

            let half = Simd::splat(0.5_f64);
            let conversion = half * (short_min + short_max);
            let base = half * (medium_min + medium_max);
            let span_a = half * (conversion + base);
            let span_b = half * (long_min + long_max);

            (conversion, base, span_a, span_b)
        }
    }
}

pub mod options {
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::options::Calc as CalcMax, min_simd::options::Calc as CalcMin,
    };

    /// SIMD computation trait for the Ichimoku Cloud, operating on `N` option-set lanes simultaneously.
    ///
    /// Unlike the `assets` variant, `i`, `short_look_back`, `long_look_back`, and
    /// `ultra_look_back` are SIMD vectors so each lane can be at a different bar position
    /// and use a different pair of periods.
    pub trait Calc<const N: usize> {
        /// Computes all four Ichimoku lines for one output bar across `N` option-set lanes.
        ///
        /// # Safety
        ///
        /// For each lane `k`, `high[k]` and `low[k]` must point to valid memory at `i[k]`,
        /// with `i[k] >= ultra_look_back[k]`. Ring-buffer states must have been initialised
        /// (via [`State::init_state`]) before the first call.
        ///
        /// # Returns
        ///
        /// `(conversion, base, span_a, span_b)` as SIMD vectors for all `N` lanes.
        unsafe fn calc_unchecked_simd(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            i: Simd<usize, N>,
            short_look_back: Simd<usize, N>,
            long_look_back: Simd<usize, N>,
            ultra_look_back: Simd<usize, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    }

    impl<const N: usize> Calc<N> for SimdState<N> {
        #[inline(always)]
        unsafe fn calc_unchecked_simd(
            &mut self,
            high: [*const f64; N],
            low: [*const f64; N],
            i: Simd<usize, N>,
            short_look_back: Simd<usize, N>,
            long_look_back: Simd<usize, N>,
            ultra_look_back: Simd<usize, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let (short_min, _) = self
                .short_min_state
                .calc_unchecked_simd(low, i, short_look_back);
            let (short_max, _) = self
                .short_max_state
                .calc_unchecked_simd(high, i, short_look_back);
            let (medium_min, _) = self
                .medium_min_state
                .calc_unchecked_simd(low, i, long_look_back);
            let (medium_max, _) =
                self.medium_max_state
                    .calc_unchecked_simd(high, i, long_look_back);
            let (long_min, _) = self
                .long_min_state
                .calc_unchecked_simd(low, i, ultra_look_back);
            let (long_max, _) = self
                .long_max_state
                .calc_unchecked_simd(high, i, ultra_look_back);

            let half = Simd::splat(0.5_f64);
            let conversion = half * (short_min + short_max);
            let base = half * (medium_min + medium_max);
            let span_a = half * (conversion + base);
            let span_b = half * (long_min + long_max);

            (conversion, base, span_a, span_b)
        }
    }
}
