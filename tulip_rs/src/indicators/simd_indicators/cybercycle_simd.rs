use crate::indicators::cybercycle::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::cybercycle::indicator_by_assets;
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::cybercycle::indicator_by_options;
use crate::indicators::simd_indicators::simd_types::F64Constants;
use crate::ring_buffer::fixed_single_buffer::FixedRingBuffer;
use std::simd::{Simd, StdFloat};

/// SIMD-parallel state for the Ehlers CyberCycle across `N` assets simultaneously.
///
/// Mirrors [`State`] but packs `N` independent assets into each SIMD vector,
/// enabling the 6-tap smooth and 2-pole IIR to be computed for all assets in a
/// single pass through the ring buffers.
///
/// Gather ([`new`](SimdState::new)) and scatter ([`write_states`](SimdState::write_states))
/// use `to_ordered_vec` / `to_f64_buffers` to pack/unpack the ring buffers.
pub struct SimdState<const N: usize> {
    /// 4-bar price ring buffer, one SIMD lane per asset.
    pub price_buf: FixedRingBuffer<Simd<f64, N>, 4>,
    /// 3-bar smooth ring buffer, one SIMD lane per asset.
    pub smooth_buf: FixedRingBuffer<Simd<f64, N>, 3>,
    /// Cycle[1] — one-bar lag, one SIMD lane per asset.
    pub cycle_prev: Simd<f64, N>,
    /// Cycle[2] — two-bar lag, one SIMD lane per asset.
    pub cycle_prev2: Simd<f64, N>,
}

impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single [`SimdState`].
    ///
    /// Calls `to_ordered_vec()` on each asset's ring buffers to obtain owned
    /// data (oldest→newest), then builds the SIMD ring buffers with `index: 0`
    /// so that `buf[0]` = newest element. Scalar cycle scalars are packed last.
    pub fn new(states: &mut [&mut State]) -> Self {
        let mut cycle_prev_arr = [0.0_f64; N];
        let mut cycle_prev2_arr = [0.0_f64; N];
        let mut price_ordered: [Vec<f64>; N] = std::array::from_fn(|_| Vec::new());
        let mut smooth_ordered: [Vec<f64>; N] = std::array::from_fn(|_| Vec::new());
        let mut price_count = 0_usize;
        let mut smooth_count = 0_usize;

        for (i, state) in states.iter_mut().enumerate() {
            cycle_prev_arr[i] = state.cycle_prev;
            cycle_prev2_arr[i] = state.cycle_prev2;
            price_ordered[i] = state.price_buf.to_ordered_vec();
            smooth_ordered[i] = state.smooth_buf.to_ordered_vec();
            if i == 0 {
                price_count = state.price_buf.count;
                smooth_count = state.smooth_buf.count;
            }
        }

        // Build SIMD ring buffers from owned ordered vecs.
        // `to_ordered_vec()` returns oldest-first; with `index: 0` the ring-buffer
        // indexing `buf[k] = vals[CAP-1-k]` maps slot CAP-1 → newest ✓
        let price_buf = FixedRingBuffer {
            vals: std::array::from_fn(|slot| {
                Simd::from_array(std::array::from_fn(|lane| {
                    price_ordered[lane].get(slot).copied().unwrap_or(0.0)
                }))
            }),
            index: 0,
            count: price_count,
        };
        let smooth_buf = FixedRingBuffer {
            vals: std::array::from_fn(|slot| {
                Simd::from_array(std::array::from_fn(|lane| {
                    smooth_ordered[lane].get(slot).copied().unwrap_or(0.0)
                }))
            }),
            index: 0,
            count: smooth_count,
        };

        Self {
            price_buf,
            smooth_buf,
            cycle_prev: Simd::from_array(cycle_prev_arr),
            cycle_prev2: Simd::from_array(cycle_prev2_arr),
        }
    }

    /// Scatters the SIMD state back into `N` scalar [`State`] references.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let price_bufs = self.price_buf.to_f64_buffers();
        let smooth_bufs = self.smooth_buf.to_f64_buffers();
        let cycle_prev_arr = self.cycle_prev.to_array();
        let cycle_prev2_arr = self.cycle_prev2.to_array();

        for (j, state) in states.iter_mut().enumerate() {
            state.price_buf = price_bufs[j].clone();
            state.smooth_buf = smooth_bufs[j].clone();
            state.cycle_prev = cycle_prev_arr[j];
            state.cycle_prev2 = cycle_prev2_arr[j];
        }
    }

    /// Computes one bar of the CyberCycle for `N` assets simultaneously.
    ///
    /// Mirrors the scalar `calc_unchecked` FMA chain in SIMD arithmetic.
    ///
    /// After the call:
    /// - `self.cycle_prev`  = Cycle (current bar), all lanes
    /// - `self.cycle_prev2` = Cycle[1] (previous bar), all lanes  — this is `Trigger`
    ///
    /// # Safety
    ///
    /// Both `price_buf` and `smooth_buf` must be full on entry.
    /// Guaranteed for every lane after [`State::init_state`].
    #[inline(always)]
    pub unsafe fn calc_simd_unchecked(
        &mut self,
        real: Simd<f64, N>,
        multipliers: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
    ) -> Simd<f64, N> {
        // ── Stage 1: 6-tap smooth ──────────────────────────────────────────
        // ab = 2·P[1] + P   cd = 2·P[2] + P[3]   smooth = (ab+cd)/6
        self.price_buf.push_unchecked(real);
        let ab = F64Constants::<N>::TWO.mul_add(self.price_buf[1], self.price_buf[0]);
        let cd = F64Constants::<N>::TWO.mul_add(self.price_buf[2], self.price_buf[3]);
        let smooth = (ab + cd) * Simd::splat(1.0_f64 / 6.0);

        // ── Stage 2: 2-pole high-pass IIR ─────────────────────────────────
        // Cycle = coeff·(S−2·S[1]+S[2]) + d1·C[1] − d2·C[2]
        self.smooth_buf.push_unchecked(smooth);
        let (coeff, d1, d2) = multipliers;
        let smooth_diff =
            (-F64Constants::<N>::TWO).mul_add(self.smooth_buf[1], smooth) + self.smooth_buf[2];
        let cycle = coeff.mul_add(
            smooth_diff,
            d1.mul_add(self.cycle_prev, -d2 * self.cycle_prev2),
        );

        self.cycle_prev2 = self.cycle_prev;
        self.cycle_prev = cycle;
        cycle
    }
}
