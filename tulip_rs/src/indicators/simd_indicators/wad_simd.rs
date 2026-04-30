#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::wad::indicator_by_assets;
use crate::indicators::{simd_indicators::simd_types::F64Constants, wad::IndicatorState as State};
use std::simd::{cmp::SimdPartialOrd, num::SimdFloat, *};

pub struct SimdState<const N: usize> {
    pub prev_close: Simd<f64, N>,
    pub wad: Simd<f64, N>,
}

impl<const N: usize> SimdState<N> {
    pub fn new(states: &[&mut State]) -> Self {
        let mut prev_close = [0.0; N];
        let mut wad = [0.0; N];

        for i in 0..N {
            prev_close[i] = states[i].prev_close;
            wad[i] = states[i].wad;
        }
        Self {
            prev_close: Simd::from_array(prev_close),
            wad: Simd::from_array(wad),
        }
    }
    pub fn to_states(&self) -> [State; N] {
        let prev_close = self.prev_close.to_array();
        let wad = self.wad.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(prev_close[i], wad[i]));

        states
    }
    pub fn write_states(&self, states: &mut [&mut State]) {
        let prev_close = self.prev_close.to_array();
        let wad = self.wad.to_array();

        for i in 0..N {
            states[i].prev_close = prev_close[i];
            states[i].wad = wad[i];
        }
    }
    #[inline(always)]
    pub fn calc_simd(
        &mut self,
        high: Simd<f64, N>,
        low: Simd<f64, N>,
        close: Simd<f64, N>,
    ) -> Simd<f64, N> {
        // Create masks for different conditions
        let close_gt_prev = close.simd_gt(self.prev_close);
        let close_lt_prev = close.simd_lt(self.prev_close);

        // Only calculate increments where needed using masks
        // For up trend: close - min(prev_close, low)
        let up_increment =
            close_gt_prev.select(close - self.prev_close.simd_min(low), F64Constants::ZERO);

        // For down trend: close - max(prev_close, high)
        let down_increment =
            close_lt_prev.select(close - self.prev_close.simd_max(high), F64Constants::ZERO);

        // Combine the increments (only one will be non-zero per lane)
        let increment = up_increment + down_increment;

        self.wad += increment;
        self.prev_close = close;

        self.wad
    }
}
