#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::stddev::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::stddev::indicator_by_options;

use crate::indicators::simd_indicators::{
    simd_types::F64Constants, sma_simd::calc_simd as sma_calc_simd,
};
pub use crate::indicators::stddev::{multiplier, State};
use std::simd::{num::SimdFloat, Simd, StdFloat};

/// SIMD-parallel state for computing the Standard Deviation indicator across `N` assets simultaneously.
/// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    /// Running sum of values within the current window (`Σ x`) for each asset lane.
    pub sum: Simd<f64, N>,
    /// Running sum of squared values within the current window (`Σ x²`) for each asset lane.
    pub sum_sq: Simd<f64, N>,
}

impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single `SimdState`, packing each field into a SIMD lane.
    pub fn new(states: &[&mut State]) -> Self {
        let mut sum = [0.0; N];
        let mut sum_sq = [0.0; N];

        for i in 0..N {
            sum[i] = states[i].sum;
            sum_sq[i] = states[i].sum_sq;
        }
        Self {
            sum: Simd::from_array(sum),
            sum_sq: Simd::from_array(sum_sq),
        }
    }

    /// Scatters the SIMD state back into an array of `N` scalar [`State`] values.
    pub fn to_states(&self) -> [State; N] {
        let sum = self.sum.to_array();
        let sum_sq = self.sum_sq.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(sum[i], sum_sq[i]));

        states
    }

    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let sum = self.sum.to_array();
        let sum_sq = self.sum_sq.to_array();

        for i in 0..N {
            states[i].sum = sum[i];
            states[i].sum_sq = sum_sq[i];
        }
    }

    /// Initialises the `SimdState` by summing the first `period` bars of each of the `N` input
    /// slices, producing the window `sum` and `sum_sq` needed to begin the rolling computation.
    /// Also returns the shared `multiplier` (`1 / period`) used by subsequent [`calc_simd`] calls.
    pub fn init_state<'a>(inputs: &[&'a [f64]; N], period: usize) -> (SimdState<N>, f64) {
        let multiplier_val = multiplier(period);
        let mut sums = Simd::splat(0.0);
        let mut sums_sq = Simd::splat(0.0);
        // Optimization: Pre-compute input pointers for the initialization loop
        let input_ptrs: [*const f64; N] = std::array::from_fn(|i| inputs[i].as_ptr());

        for i in 0..period {
            let values =
                Simd::from_array(std::array::from_fn(|j| unsafe { *input_ptrs[j].add(i) }));
            sums += values;
            sums_sq += values * values;
        }
        (
            SimdState::<N> {
                sum: sums,
                sum_sq: sums_sq,
            },
            multiplier_val,
        )
    }

    /// Advances one bar of the Standard Deviation computation for `N` lanes simultaneously.
    ///
    /// Updates the rolling `sum` (via [`sma_calc_simd`]) and `sum_sq`, then computes
    /// `sd = sqrt(sum_sq * multiplier - sma²)`, clamped to at least [`f64::EPSILON`].
    ///
    /// # Returns
    ///
    /// `(sd, sma)` — the standard deviation and the simple moving average for the current bar.
    #[inline(always)]
    pub fn calc_simd(
        &mut self,
        value: Simd<f64, N>,
        prev_value: Simd<f64, N>,
        multiplier: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let sma = sma_calc_simd(&mut self.sum, value, prev_value, multiplier);

        self.sum_sq += value.mul_add(value, -(prev_value * prev_value));
        //let mut sd = (state.sum_sq * multiplier) - (sma * sma);
        let mut sd = self.sum_sq.mul_add(multiplier, -(sma * sma));
        sd = sd.sqrt().simd_max(F64Constants::<N>::EPSILON);

        (sd, sma)
    }
}

/// Advances one bar of the Standard Deviation computation for `N` lanes simultaneously.
///
/// Delegates to [`SimdState::calc_simd`]. Returns `(sd, sma)` for each lane.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    value: Simd<f64, N>,
    prev_value: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>) {
    state.calc_simd(value, prev_value, multiplier)
}
