#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::cci::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::cci::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
mod imports {
    pub(crate) use crate::indicators::cci::State;
    pub(crate) use crate::indicators::simd_indicators::md_simd::assets::SimdState as MdSimdState;
    pub(crate) use crate::indicators::simd_indicators::{
        simd_types::F64Constants, typprice_simd::calc_simd as typprice_calc_simd,
    };
    pub(crate) use std::simd::Simd;
}
use crate::types::Warm;
pub mod asset {
    use super::imports::*;
    use super::*;

    use crate::ring_buffer::single_buffer::generic_buffer::{SimdBuffer, SimdRingBuffer};

    /// SIMD-parallel state for computing the Commodity Channel Index (CCI) across `N` assets
    /// simultaneously. Each field is a SIMD vector where lane `i` corresponds to asset `i`.
    pub struct SimdState<const N: usize> {
        /// Ring buffer of recent typical prices, used to compute mean deviation.
        buffer: SimdBuffer<N>,
        /// Rolling sum of typical prices for the SMA calculation.
        md_state: MdSimdState<N>,
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;

        crate::simd_state_impl!(
             sub: [(md_state: MdSimdState<N>)],
             scalar: [],
             buf: [(buffer: SimdBuffer<N>, from_f64_buffers)]
        );
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        #[inline(always)]
        fn calc<'a>(&mut self, (high, low, close): Self::Inputs<'a>) -> Self::Outputs {
            let typprice = typprice_calc_simd(high, low, close);

            let old = self.buffer.push_with_info(typprice);

            let (md, sma) = self.md_state.calc((typprice, old, self.buffer.get_slice()));

            let cci = (typprice - sma) / (F64Constants::ZERO15 * md);
            (cci, sma, md, typprice)
        }
    }
}

pub(crate) mod options {
    use super::imports::*;
    use super::*;
    use crate::indicators::{md::calc_md_simd, typprice::calc as typprice_calc};
    use crate::ring_buffer::unsync_multi_buffer::multi_buffer::UnsyncBuffer;

    /// SIMD-parallel state for computing the CCI across `N` option lanes simultaneously.
    /// Uses per-lane ring buffers of potentially different periods stored in an `UnsyncBuffer`.
    pub struct SimdState<const N: usize> {
        /// Per-lane ring buffers with independent periods for each option set.
        buffer: UnsyncBuffer<N, f64, Warm>,
        /// Rolling sums of typical prices, one per option lane.
        md_state: MdSimdState<N>,
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;

        crate::simd_state_impl!(
             sub: [(md_state: MdSimdState<N>)],
             scalar: [],
             buf: [(buffer: UnsyncBuffer<N, f64, Warm>, from_f64_buffers)]
        );
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (f64, f64, f64);
        type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
        /// Advances the CCI by one bar for `N` option lanes simultaneously (unchecked variant).
        ///
        /// Takes a single scalar `(high, low, close)` that is broadcast to all lanes, then
        /// applies each lane's independent period to compute the SMA and mean deviation.
        /// Returns `(cci, sma, mean_deviation, typical_price)`.
        ///
        /// # Safety
        ///
        /// The caller must guarantee all per-lane ring buffers are fully warmed up.
        #[inline(always)]
        fn calc<'a>(&mut self, (high, low, close): Self::Inputs<'a>) -> Self::Outputs {
            let typprice = typprice_calc(high, low, close);
            let typprice = Simd::splat(typprice);
            let (old, _) = self.buffer.push_with_info(typprice);

            let sma = self.md_state.0.calc((typprice, old));
            let mut md = Simd::splat(0.0);
            let sma_ref = sma.as_array();
            let md_ref = md.as_mut_array();
            let slices = self.buffer.raw_slice();
            for (i, &multiplier) in self.md_state.multiplier.as_array().iter().enumerate() {
                md_ref[i] = calc_md_simd::<4>(&slices[i], sma_ref[i], multiplier);
            }

            let cci = (typprice - sma) / (F64Constants::ZERO15 * md);
            (cci, sma, md, typprice)
        }
    }
}
