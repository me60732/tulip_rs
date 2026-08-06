#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::zlema::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::zlema::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::types::Warm;
use crate::indicators::zlema::State;
use std::simd::{Simd, StdFloat};

/// SIMD-parallel state for the Zero-Lag Exponential Moving Average (ZLEMA) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub zlema: Simd<f64, N>,
    pub per: Simd<f64, N>,
    pub multiplier: Simd<f64, N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_from_state!(
         sub: [],
         scalar: [zlema, per, multiplier]
    );
    crate::simd_state_write!(
         sub: [],
         scalar: [zlema]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = Simd<f64, N>;
    
    #[inline(always)]
    fn calc<'a>(&mut self, (current, lagged): Self::Inputs<'a>) -> Self::Outputs {
        let adjusted = current + (current - lagged);
        self.zlema = self.zlema.mul_add(self.per, adjusted * self.multiplier);
        //self.zlema = self.zlema * self.per + adjusted * self.multiplier;
        self.zlema
    }
}

