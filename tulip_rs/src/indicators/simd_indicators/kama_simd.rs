use crate::indicators::kama::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::kama::indicator_by_assets;
use crate::indicators::simd_indicators::simd_types::F64Constants;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::kama::indicator_by_options;

use std::simd::{cmp::SimdPartialEq, num::SimdFloat, Select, Simd, StdFloat};

/// SIMD-parallel state for computing the Kaufman Adaptive Moving Average (KAMA) across `N` assets simultaneously.
/// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    /// Current KAMA value per asset lane.
    pub kama: Simd<f64, N>,
    /// Rolling sum of absolute bar-to-bar price changes over the period window per lane.
    pub sum: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single `SimdState`, packing each field into a SIMD lane.
    pub fn new(states: &[&mut State]) -> Self {
        let mut kama = [0.0; N];
        let mut sum = [0.0; N];

        for i in 0..N {
            kama[i] = states[i].kama;
            sum[i] = states[i].sum;
        }

        Self {
            kama: Simd::from_array(kama),
            sum: Simd::from_array(sum),
        }
    }
    /// Scatters the SIMD state back into an array of `N` scalar [`State`] values.
    pub fn to_states(&self) -> [State; N] {
        let kama = self.kama.to_array();
        let sum = self.sum.to_array();

        let states: [State; N] = std::array::from_fn(|i| State::new(kama[i], sum[i]));

        states
    }
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let kama = self.kama.to_array();
        let sum = self.sum.to_array();

        for (i, state) in states.iter_mut().enumerate() {
            state.kama = kama[i];
            state.sum = sum[i];
        }
    }
}

/// Computes one KAMA step across `N` asset/option lanes using SIMD parallelism.
///
/// Calculates the Efficiency Ratio (|net change| / |total path|) and uses it to
/// blend the fast and slow EMA smoothing constants. When `sum == 0` (perfectly
/// efficient or flat market) the smoothing constant defaults to `1.0` (full tracking).
/// FMA instructions are used throughout to maximise throughput.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    state: &mut SimdState<N>,
    values: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
    multipliers: (Simd<f64, N>, Simd<f64, N>),
) -> Simd<f64, N> {
    let (value, prev_value, last_value, old_value) = values;
    let (fast_ema, slow_ema) = multipliers;
    let (mut kama, mut sum) = (state.kama, state.sum);
    let mask = sum.simd_ne(F64Constants::ZERO);
    sum += (value - prev_value).abs() - (last_value - old_value).abs();

    let efficiency_ratio = mask.select(
        (value - last_value).abs() / sum, // When sum != 0.0
        F64Constants::ONE,                // When sum == 0.0, use 1.0
    );

    let smoothing_constant = {
        let temp = (fast_ema - slow_ema).mul_add(efficiency_ratio, slow_ema);
        temp * temp // Square it by multiplying by itself
    };

    // Optimized calculation using C-style EMA pattern
    let per1 = F64Constants::ONE - smoothing_constant;
    //kama = kama * per1 + value * smoothing_constant;
    kama = kama.mul_add(per1, value * smoothing_constant);
    (state.kama, state.sum) = (kama, sum);
    kama
}
