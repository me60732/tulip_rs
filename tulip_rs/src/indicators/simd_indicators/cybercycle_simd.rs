pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::cybercycle::IndicatorState as State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::cybercycle::indicator_by_assets;
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::cybercycle::indicator_by_options;
use crate::indicators::simd_indicators::simd_types::F64Constants;
use crate::ring_buffer::fixed_single_buffer::{FixedRingBuffer, FixedSimdRingBuffer};
use std::simd::{Simd, StdFloat};
use crate::types::Warm;
/// SIMD-parallel state for the Ehlers CyberCycle across `N` assets simultaneously.
///
/// Mirrors [`State`] but packs `N` independent assets into each SIMD vector,
/// enabling the 6-tap smooth and 2-pole IIR to be computed for all assets in a
/// single pass through the ring buffers.  Coefficients (`coef`, `d1`, `d2`) are
/// gathered from each lane's scalar state at construction and stored here so the
/// hot path (`calc_simd_unchecked` / `calc`) needs no external parameters.
pub struct SimdState<const N: usize> {
    /// 4-bar price ring buffer, one SIMD lane per asset.
    pub price_buf: FixedRingBuffer<Simd<f64, N>, 4, Warm>,
    /// 3-bar smooth ring buffer, one SIMD lane per asset.
    pub smooth_buf: FixedRingBuffer<Simd<f64, N>, 3, Warm>,
    /// Cycle[1] — one-bar lag, one SIMD lane per asset.
    pub cycle_prev: Simd<f64, N>,
    /// Cycle[2] — two-bar lag, one SIMD lane per asset.
    pub cycle_prev2: Simd<f64, N>,
    /// Feedforward gain: `(1 − α/2)²` per lane.
    pub coef: Simd<f64, N>,
    /// First IIR feedback coefficient: `2·(1 − α)` per lane.
    pub d1: Simd<f64, N>,
    /// Second IIR feedback coefficient: `(1 − α)²` per lane.
    pub d2: Simd<f64, N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;

    /// Gathers `N` scalar [`State`] references into a single [`SimdState`].
    ///
    /// Ring buffers are packed via [`FixedSimdRingBuffer::from_f64_buffers`];
    /// scalar fields are packed into `[f64; N]` arrays then wrapped with
    /// `Simd::from_array`.  Read-only coefficients are gathered along with
    /// mutable filter history.
    fn from_states(states: &mut [&mut State]) -> Self {
        let mut cycle_prev_arr = [0.0_f64; N];
        let mut cycle_prev2_arr = [0.0_f64; N];
        let mut coef_arr = [0.0_f64; N];
        let mut d1_arr = [0.0_f64; N];
        let mut d2_arr = [0.0_f64; N];
        let mut price_refs: Vec<&FixedRingBuffer<f64, 4, Warm>> = Vec::with_capacity(N);
        let mut smooth_refs: Vec<&FixedRingBuffer<f64, 3, Warm>> = Vec::with_capacity(N);

        for (i, state) in states.iter_mut().enumerate() {
            cycle_prev_arr[i] = state.cycle_prev;
            cycle_prev2_arr[i] = state.cycle_prev2;
            coef_arr[i] = state.coef;
            d1_arr[i] = state.d1;
            d2_arr[i] = state.d2;
            price_refs.push(&state.price_buf);
            smooth_refs.push(&state.smooth_buf);
        }

        Self {
            price_buf: FixedRingBuffer::<Simd<f64, N>, 4, Warm>::from_f64_buffers(&price_refs),
            smooth_buf: FixedRingBuffer::<Simd<f64, N>, 3, Warm>::from_f64_buffers(&smooth_refs),
            cycle_prev: Simd::from_array(cycle_prev_arr),
            cycle_prev2: Simd::from_array(cycle_prev2_arr),
            coef: Simd::from_array(coef_arr),
            d1: Simd::from_array(d1_arr),
            d2: Simd::from_array(d2_arr),
        }
    }

    /// Scatters filter history and coefficients back into `N` scalar [`State`] references.
    ///
    /// `coef`, `d1`, `d2` are written back because an adaptive-alpha driver (e.g.
    /// `trendmode`, `ccfisher`) recomputes them every bar — omitting them would
    /// leave each scalar [`State`] with stale coefficients after the epoch.
    fn write_states(&self, states: &mut [&mut State]) {
        let price_bufs = self.price_buf.to_f64_buffers();
        let smooth_bufs = self.smooth_buf.to_f64_buffers();
        let cycle_prev_arr = self.cycle_prev.to_array();
        let cycle_prev2_arr = self.cycle_prev2.to_array();
        let coef_arr = self.coef.to_array();
        let d1_arr = self.d1.to_array();
        let d2_arr = self.d2.to_array();

        for (j, state) in states.iter_mut().enumerate() {
            state.price_buf = price_bufs[j].clone();
            state.smooth_buf = smooth_bufs[j].clone();
            state.cycle_prev = cycle_prev_arr[j];
            state.cycle_prev2 = cycle_prev2_arr[j];
            state.coef = coef_arr[j];
            state.d1 = d1_arr[j];
            state.d2 = d2_arr[j];
        }
    }
}

impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = Simd<f64, N>;
    type Outputs = Simd<f64, N>;

    /// Safe single-bar update — delegates to `calc_simd_unchecked`.
    ///
    /// Only call once the state has been fully warmed up via [`State::init_state`]
    /// for every lane (which is guaranteed by the SIMD driver infrastructure).
    #[inline(always)]
    fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
        self.price_buf.push(real);
        let ab = F64Constants::<N>::TWO.mul_add(self.price_buf[1], self.price_buf[0]);
        let cd = F64Constants::<N>::TWO.mul_add(self.price_buf[2], self.price_buf[3]);
        let smooth = (ab + cd) * Simd::splat(1.0_f64 / 6.0);

        // ── Stage 2: 2-pole high-pass IIR ─────────────────────────────────
        // Cycle = coef·(S−2·S[1]+S[2]) + d1·C[1] − d2·C[2]
        self.smooth_buf.push(smooth);
        let smooth_diff =
            (-F64Constants::<N>::TWO).mul_add(self.smooth_buf[1], smooth) + self.smooth_buf[2];
        let cycle = self.coef.mul_add(
            smooth_diff,
            self.d1
                .mul_add(self.cycle_prev, -self.d2 * self.cycle_prev2),
        );

        self.cycle_prev2 = self.cycle_prev;
        self.cycle_prev = cycle;
        cycle
    }
}

