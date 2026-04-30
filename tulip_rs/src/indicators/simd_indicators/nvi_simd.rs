use crate::indicators::nvi::IndicatorState as State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::nvi::indicator_by_assets;
use std::simd::{cmp::SimdPartialOrd, *};

pub struct SimdState<const N: usize> {
    nvi: Simd<f64, N>,
    close: Simd<f64, N>,
    volume: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new(states: &[&mut State]) -> Self {
        let mut nvi = [0.0; N];
        let mut close = [0.0; N];
        let mut volume = [0.0; N];

        for i in 0..N {
            nvi[i] = states[i].nvi;
            close[i] = states[i].close;
            volume[i] = states[i].volume;
        }
        Self {
            nvi: Simd::from_array(nvi),
            close: Simd::from_array(close),
            volume: Simd::from_array(volume),
        }
    }
    /*pub fn to_states(&self) -> [State; N] {
        let nvi = self.nvi.to_array();
        let close = self.close.to_array();
        let volume = self.volume.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(nvi[i], close[i], volume[i]));

        states
    }*/
    pub fn write_states(&self, states: &mut [&mut State]) {
        let nvi = self.nvi.to_array();
        let close = self.close.to_array();
        let volume = self.volume.to_array();

        for i in 0..N {
            states[i].nvi = nvi[i];
            states[i].close = close[i];
            states[i].volume = volume[i];
        }
    }
    #[inline(always)]
    pub fn calc_simd(&mut self, close: Simd<f64, N>, volume: Simd<f64, N>) -> Simd<f64, N> {
        // Create a mask for where volume < state.volume
        let mask = volume.simd_lt(self.volume);

        // Calculate the new NVI value conditionally using SIMD select
        self.nvi = mask.select(close / self.close * self.nvi, self.nvi);

        (self.close, self.volume) = (close, volume);
        self.nvi
    }
}

