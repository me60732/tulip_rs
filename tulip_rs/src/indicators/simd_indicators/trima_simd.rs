#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::trima::indicator_by_assets;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::trima::State;
use crate::ring_buffer::single_buffer::generic_buffer::{SimdBuffer, SimdRingBuffer};
use crate::types::Warm;
use std::simd::Simd;

/// SIMD-parallel state for computing the Triangular Moving Average (TRIMA) across `N` assets simultaneously.
/// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    /// Ring buffer holding SMA1 values per lane
    pub sma1_ring: SimdBuffer<N>,
    /// Running sum for first SMA (m1-period window) per lane
    pub sum1: Simd<f64, N>,
    /// Running sum for second SMA (m2-period window of first SMA outputs) per lane
    pub sum2: Simd<f64, N>,
    /// 1/m1 per lane (from scalar state)
    pub inv_m1: Simd<f64, N>,
    /// 1/m2 per lane (from scalar state)
    pub inv_m2: Simd<f64, N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;

    crate::simd_state_from_state!(
        sub: [],
        scalar: [sum1, sum2, inv_m1, inv_m2],
        buf: [(sma1_ring: SimdBuffer<N>, from_f64_buffers)]
    );

    crate::simd_state_write!(
        sub: [],
        scalar: [sum1, sum2, inv_m1, inv_m2],
        buf: [(sma1_ring: SimdBuffer<N>, from_f64_buffers)]
    );
}

impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = Simd<f64, N>;

    /// Calculates one bar of TRIMA per lane using the SMA-of-SMA formulation.
    ///
    /// Inputs: `(x[i], x[i - m1])` where `m1 = (p+1)/2`.
    #[inline(always)]
    fn calc<'a>(&mut self, (x_i, x_i_minus_m1): Self::Inputs<'a>) -> Self::Outputs {
        self.sum1 += x_i - x_i_minus_m1;
        let s1 = self.sum1 * self.inv_m1;
        let old_s1 = self.sma1_ring.push_with_info(s1);
        self.sum2 += s1 - old_s1;
        self.sum2 * self.inv_m2
    }
}

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::trima::indicator_by_options;

/// Option-parallel state for TRIMA: `N` different periods on ONE shared price series.
///
/// Unlike by-assets (uniform `m2`), each lane evicts its own `m2_i` bars back and the
/// pushed `s1` series is per-lane (m1 differs), so a single scalar ring cannot serve all
/// lanes. Uses an `UnsyncBuffer` — `N` independent scalar rings with per-lane capacities,
/// one vectorized push per bar — the same pattern as `adxr_simd::options`.
pub mod option {
    use super::Simd;
    use crate::indicator_types::{TSimdState, TState};
    use crate::indicators::trima::State;
    use crate::ring_buffer::unsync_multi_buffer::multi_buffer::UnsyncBuffer;
    use crate::types::Warm;

    pub struct SimdState<const N: usize> {
        pub sma1_ring: UnsyncBuffer<N, f64, Warm>,
        pub sum1: Simd<f64, N>,
        pub sum2: Simd<f64, N>,
        pub inv_m1: Simd<f64, N>,
        pub inv_m2: Simd<f64, N>,
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        crate::simd_state_impl!(
            sub: [],
            scalar: [sum1, sum2, inv_m1, inv_m2],
            buf: [(sma1_ring: UnsyncBuffer<N, f64, Warm>, from_f64_buffers)]
        );
    }

    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
        type Outputs = Simd<f64, N>;

        #[inline(always)]
        fn calc<'a>(&mut self, (x_i, x_i_minus_m1): Self::Inputs<'a>) -> Self::Outputs {
            self.sum1 += x_i - x_i_minus_m1;
            let s1 = self.sum1 * self.inv_m1;
            // One vectorized push; lane i evicts its own ring's oldest value.
            let old_s1 = self.sma1_ring.push_with_info(s1);
            self.sum2 += s1 - old_s1;
            self.sum2 * self.inv_m2
        }
    }
}
