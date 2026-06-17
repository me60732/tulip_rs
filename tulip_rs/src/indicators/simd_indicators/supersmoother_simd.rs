#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::supersmoother::indicator_by_assets;
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::supersmoother::indicator_by_options;
use crate::indicators::supersmoother::State;

use std::simd::{Simd, StdFloat};

/// SIMD-parallel state for computing the Ehlers Super Smoother across `N` assets simultaneously.
/// Each field is a SIMD vector where lane `i` holds the filter state for asset `i`.
pub struct SimdState<const N: usize> {
    pub y1: Simd<f64, N>,        // y[t-1] for each asset
    pub y2: Simd<f64, N>,        // y[t-2] for each asset
    pub prev_real: Simd<f64, N>, // x[t-1] for Ehlers input averaging
}

impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single [`SimdState`],
    /// packing `y1`, `y2`, and `prev_real` from each asset into their respective SIMD lanes.
    pub fn new(states: &[&mut State]) -> Self {
        let mut y1 = [0.0; N];
        let mut y2 = [0.0; N];
        let mut prev_real = [0.0; N];

        for i in 0..N {
            y1[i] = states[i].y1;
            y2[i] = states[i].y2;
            prev_real[i] = states[i].prev_real;
        }

        Self {
            y1: Simd::from_array(y1),
            y2: Simd::from_array(y2),
            prev_real: Simd::from_array(prev_real),
        }
    }

    /// Scatters the SIMD state back into `N` scalar [`State`] references,
    /// writing each lane's `y1`, `y2`, and `prev_real` back to its corresponding asset state.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let y1 = self.y1.to_array();
        let y2 = self.y2.to_array();
        let prev_real = self.prev_real.to_array();

        for (i, state) in states.iter_mut().enumerate() {
            state.y1 = y1[i];
            state.y2 = y2[i];
            state.prev_real = prev_real[i];
        }
    }

    /// Advances the filter by one bar across all `N` assets simultaneously.
    ///
    /// Computes Ehlers' `(b0/2)·(real + prev_real) + a1·y1 + a2·y2`,
    /// then shifts `prev_real ← real`, `y2 ← y1`, `y1 ← y`.
    ///
    /// # Arguments
    ///
    /// * `real` - SIMD vector of current input prices, one per asset lane.
    /// * `multipliers` - Tuple of SIMD coefficient vectors `(a1, a2, b0)`,
    ///   broadcast from [`crate::indicators::supersmoother::multiplier`].
    ///
    /// # Returns
    ///
    /// A SIMD vector of filtered output values, one per asset lane.
    #[inline(always)]
    pub fn calc_simd(
        &mut self,
        real: Simd<f64, N>,
        multipliers: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
    ) -> Simd<f64, N> {
        let (a1, a2, b0) = multipliers;
        // Ehlers: (b0/2) * (real + prev_real) + a1*y1 + a2*y2
        let half = Simd::splat(0.5);
        let y = (b0 * half).mul_add(real + self.prev_real, a1.mul_add(self.y1, a2 * self.y2));
        self.y2 = self.y1;
        self.y1 = y;
        self.prev_real = real;
        y
    }
}
