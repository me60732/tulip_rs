#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::adxr::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::adxr::indicator_by_options;

mod imports {
    pub(crate) use crate::indicators::adxr::IndicatorState;
    pub(crate) use crate::indicators::simd_indicators::{
        adx_simd::SimdState as AdxSimdState, simd_types::F64Constants,
    };
    pub(crate) use std::simd::Simd;
}
pub use crate::indicator_types::{TSimdState, TState};
pub mod assets {
    use super::imports::*;
    pub use super::*;
    use crate::ring_buffer::single_buffer::generic_buffer::{SimdBuffer, SimdRingBuffer};

    /// SIMD-parallel state for computing the Average Directional Movement Rating (ADXR) across
    /// `N` assets simultaneously. Each field is a SIMD vector where lane `i` corresponds to
    /// asset `i`.
    pub struct SimdState<const N: usize> {
        /// Embedded ADX SIMD state for all `N` asset lanes.
        pub adx_state: AdxSimdState<N>,
        /// Ring buffer that retains past ADX values used to compute the ADXR average.
        pub buffer: SimdBuffer<N>,
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        type Outputs = (
            Simd<f64, N>,
            Simd<f64, N>,
            Simd<f64, N>,
            Simd<f64, N>,
            Simd<f64, N>,
        );

        #[inline(always)]
        fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
            let (adx, dx, atr, tr) = self.adx_state.calc(inputs);
            let adxr = F64Constants::HALF * (adx + self.buffer.push_with_info(adx));

            (adxr, adx, dx, atr, tr)
        }
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = IndicatorState;
        crate::simd_state_impl!(
            sub: [(adx_state: AdxSimdState<N>)],
            scalar: [],
            buf: [(buffer: SimdBuffer<N>, from_f64_buffers)]
        );
    }
}

pub mod options {
    use super::imports::*;
    pub use super::*;
    use crate::ring_buffer::single_buffer::generic_buffer::Warm;
    use crate::ring_buffer::unsync_multi_buffer::multi_buffer::UnsyncBuffer;

    /// SIMD-parallel state for computing the ADXR across `N` option lanes simultaneously.
    /// Uses per-lane ring buffers of potentially different periods stored in an `UnsyncBuffer`.
    pub struct SimdState<const N: usize> {
        /// Embedded ADX SIMD state for all `N` option lanes.
        pub adx_state: AdxSimdState<N>,
        /// Per-lane ring buffers with independent periods for each option set.
        pub buffer: UnsyncBuffer<N, f64, Warm>,
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        type Outputs = (
            Simd<f64, N>,
            Simd<f64, N>,
            Simd<f64, N>,
            Simd<f64, N>,
            Simd<f64, N>,
        );
        #[inline(always)]
        fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
            let (adx, dx, atr, tr) = self.adx_state.calc(inputs);
            let (old_adx, _) = self.buffer.push_with_info(adx);
            let adxr = F64Constants::HALF * (adx + old_adx);

            (adxr, adx, dx, atr, tr)
        }
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = IndicatorState;
        crate::simd_state_impl!(
            sub: [(adx_state: AdxSimdState<N>)],
            scalar: [],
            buf: [(buffer: UnsyncBuffer<N, f64, Warm>, from_f64_buffers)]
        );
    }
}
