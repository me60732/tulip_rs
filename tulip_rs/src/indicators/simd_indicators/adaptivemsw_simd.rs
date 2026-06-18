use crate::indicators::adaptivemsw::State;
use crate::indicators::msw;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::adaptivemsw::indicator_by_assets;
use crate::indicators::simd_indicators::homodynediscriminator_simd::SimdState as HdSimdState;
use crate::ring_buffer::fixed_single_buffer::FixedMirrorBuffer;
use std::simd::Simd;

/// SIMD-parallel state for the Adaptive MESA Sine Wave across `N` assets simultaneously.
///
/// The Homodyne Discriminator pipeline runs fully in SIMD across all `N` lanes,
/// producing `N` `SmoothPeriod` values per bar. The DFT step runs per-asset
/// (each asset may have a different adaptive period), but each DFT call uses
/// the existing 8-wide SIMD `calc_rp_ip` internally.
///
/// The price history is stored as `N` independent 50-slot ring buffers (one per
/// asset) rather than a SIMD ring buffer, because the DFT window length varies
/// per asset and the DFT itself is not easy to vectorize across lanes with
/// different window sizes.
pub struct SimdState<const N: usize> {
    /// Embedded Homodyne Discriminator SIMD state — all N lanes computed together.
    pub hd: HdSimdState<N>,
    /// Per-asset price history buffers for the DFT: `view[0]` = oldest, `view[count-1]` = newest.
    /// `get_slice_by_period` returns a contiguous oldest-first slice with zero copying.
    price_bufs: [FixedMirrorBuffer<f64, 50>; N],
}

impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single [`SimdState`].
    pub fn new(states: &mut [&mut State]) -> Self {
        // First pass: clone price_bufs (owned data, no lasting borrow).
        let price_bufs: [FixedMirrorBuffer<f64, 50>; N] =
            std::array::from_fn(|j| states[j].price_buf.clone());

        // Second pass: collect mutable HD references.
        let mut hd_refs: Vec<&mut crate::indicators::homodynediscriminator::State> =
            Vec::with_capacity(N);
        for state in states.iter_mut() {
            hd_refs.push(&mut state.hd);
        }

        let hd = HdSimdState::new(&hd_refs);

        Self { hd, price_bufs }
    }

    /// Scatters the SIMD state back into `N` scalar [`State`] references.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut hd_refs: Vec<&mut crate::indicators::homodynediscriminator::State> =
            Vec::with_capacity(N);
        for (j, state) in states.iter_mut().enumerate() {
            hd_refs.push(&mut state.hd);
            state.price_buf = self.price_bufs[j].clone();
        }
        self.hd.write_states(&mut hd_refs);
    }

    /// Computes one bar of the Adaptive MESA Sine Wave for `N` assets simultaneously.
    ///
    /// The HD runs fully in SIMD across all N lanes. Each asset's DFT then runs
    /// independently (using the 8-wide SIMD `calc_rp_ip` internally), because the
    /// adaptive period may differ per asset.
    ///
    /// Returns `(sine_vec, lead_vec)` — one value per lane.
    ///
    /// # Safety
    ///
    /// All HD ring buffers must be full on entry. Guaranteed after [`State::init_state`]
    /// for every lane before [`SimdState::new`].
    #[inline(always)]
    pub unsafe fn calc_simd_unchecked(
        &mut self,
        real: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        // Run HD for all N lanes simultaneously.
        let dc = self.hd.calc_simd_unchecked(real);
        let dc_arr = dc.to_array();
        let real_arr = real.to_array();

        let mut sine_arr = [0.0_f64; N];
        let mut lead_arr = [0.0_f64; N];

        // Per-lane DFT: each uses its own adaptive period and price history.
        for j in 0..N {
            self.price_bufs[j].push(real_arr[j]);
            let period = ((dc_arr[j] + 0.5) as usize).clamp(6, self.price_bufs[j].len().min(50));
            let multiplier = msw::multiplier(period);

            // get_slice_by_period returns &view[count-period..count]: oldest-first,
            // contiguous — no stack copy or reversal needed.
            let (sine, lead) =
                msw::calc::<8>(self.price_bufs[j].get_slice_by_period(period), multiplier);
            sine_arr[j] = sine;
            lead_arr[j] = lead;
        }

        (Simd::from_array(sine_arr), Simd::from_array(lead_arr))
    }
}
