#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::vhf::indicator_by_assets;

pub use crate::indicator_types::{TSimdState, TState};
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::vhf::indicator_by_options;

use crate::indicators::vhf::State;
/// SIMD-parallel state for the Vertical Horizontal Filter (VHF) indicator, holding `N` lanes of per-asset state.
use crate::types::Warm;
use std::simd::{num::SimdFloat, Simd};
pub mod assets {
    //! Per-asset road SIMD helpers for the Vertical Horizontal Filter (VHF) indicator.
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::assets::SimdState as MaxSimdState, min_simd::assets::SimdState as MinSimdState,
    };
    use crate::ring_buffer::single_buffer::generic_buffer::{SimdBuffer, SimdRingBuffer};
    pub struct SimdState<const N: usize> {
        buffer: SimdBuffer<N>,
        min_state: MinSimdState<N>,
        max_state: MaxSimdState<N>,
        prev_real: Simd<f64, N>,
        sum: Simd<f64, N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_impl!(
            sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
            scalar: [sum, prev_real],
            buf: [(buffer: SimdBuffer<N>, from_f64_buffers)]
        );
    }
    /// Trait providing the unchecked per-asset SIMD VHF computation.
    
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, [*const f64; N], usize, usize);
        type Outputs = Simd<f64, N>;

        #[inline(always)]
        fn calc(
            &mut self,
            (value, real, look_back, i): Self::Inputs<'_>
        ) -> Self::Outputs {
            let new = (value - self.prev_real).abs();
            self.sum += new - self.buffer.push_with_info(new);
            self.prev_real = value;

            let (min, _) = self
                .min_state
                .calc_w_current::<4>((real, i, look_back, value));
            let (max, _) = self
                .max_state
                .calc_w_current::<4>((real, i, look_back, value));

            (max - min) / self.sum.simd_max(Simd::splat(f64::EPSILON))
        }
    }
}

pub mod options {
    //! Per-option road SIMD helpers for the Vertical Horizontal Filter (VHF) indicator.
    use super::*;
    use crate::indicators::simd_indicators::{
        max_simd::options::SimdState as MaxSimdState, min_simd::options::SimdState as MinSimdState,
    };
    use crate::ring_buffer::unsync_multi_buffer::multi_buffer::UnsyncBuffer;
    pub struct SimdState<const N: usize> {
        buffer: UnsyncBuffer<N, f64, Warm>,
        min_state: MinSimdState<N>,
        max_state: MaxSimdState<N>,
        prev_real: Simd<f64, N>,
        sum: Simd<f64, N>,
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_impl!(
            sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
            scalar: [sum, prev_real],
            buf: [(buffer: UnsyncBuffer<N, f64, Warm>, from_f64_buffers)]
        );
    }
    
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, [*const f64; N], Simd<usize, N>, Simd<usize, N>);
        type Outputs = Simd<f64, N>;

        
        #[inline(always)]
        fn calc(
            &mut self,
            (value, real, look_back, i): Self::Inputs<'_>
        ) -> Self::Outputs {
            let new = (value - self.prev_real).abs();
            self.sum += new - self.buffer.push_with_info(new).0;
            self.prev_real = value;
            let (min, _) = self
                .min_state
                .calc_w_current((real, i, look_back, value));
            let (max, _) = self
                .max_state
                .calc_w_current((real, i, look_back, value));

            (max - min) / self.sum.simd_max(Simd::splat(f64::EPSILON))
        }
    }
}
