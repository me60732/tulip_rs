#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::bbands::indicator_by_assets;
/// Re-uses [`stddev_simd::SimdState`] as the state for Bollinger Bands since the rolling
/// standard deviation and SMA are the core calculations needed.
use crate::indicators::simd_indicators::stddev_simd::SimdState as StddevSimdState;
pub use crate::indicator_types::{TState, TSimdState};
use crate::indicators::bbands::State;
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::bbands::indicator_by_options;
use crate::types::Warm;
use std::simd::{Simd, StdFloat};
pub struct SimdState<const N: usize> {
    pub stddev_state: StddevSimdState<N>,
    pub std_dev: Simd<f64, N>
}

impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        inputs: Self::Inputs<'a>
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let (sd, sma) = self.stddev_state.calc(inputs);
    
        //let upper_band = sma + std_dev * sd;
        let upper_band = self.std_dev.mul_add(sd, sma);
        //let lower_band = sma - std_dev * sd;
        let lower_band = (-self.std_dev).mul_add(sd, sma);
        (lower_band, sma, upper_band)
    }
}   
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_from_state!(
         sub: [(stddev_state: StddevSimdState<N>)],
         scalar: [std_dev]
    );
    crate::simd_state_write!(
         sub: [(stddev_state: StddevSimdState<N>)],
         scalar: []
    );
}