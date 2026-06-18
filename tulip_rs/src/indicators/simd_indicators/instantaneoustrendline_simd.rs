use crate::indicators::instantaneoustrendline::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::instantaneoustrendline::indicator_by_assets;
use crate::indicators::simd_indicators::homodynediscriminator_simd::SimdState as HdSimdState;
use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::{Simd, StdFloat};

/// SIMD-parallel state for the Ehlers Instantaneous Trendline across `N` assets simultaneously.
///
/// Composes [`HdSimdState`] as the `hd` field — the full four-stage HT cascade and
/// homodyne discriminator — and adds IT-specific SIMD fields on top, exactly mirroring
/// how the scalar [`State`] composes [`homodynediscriminator::State`].
///
/// The gather (`new`) and scatter (`write_states`) methods delegate the HD sub-state
/// to [`HdSimdState::new`] / [`HdSimdState::write_states`] and pack/unpack the
/// IT-specific scalars in a single loop pass.
pub struct SimdState<const N: usize> {
    /// Embedded Homodyne Discriminator SIMD state.
    /// Its `price_buf[0..2]` holds the 3 most-recent raw prices used by the IIR.
    pub hd: HdSimdState<N>,
    /// IT[1] — previous trendline (IIR feedback), one lane per asset.
    pub it_prev: Simd<f64, N>,
    /// IT[2] — two-bar-ago trendline (IIR feedback), one lane per asset.
    pub it_prev2: Simd<f64, N>,
    /// Last computed adaptive α, one lane per asset (for optional output).
    pub alpha: Simd<f64, N>,
}

impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single [`SimdState`].
    ///
    /// Delegates the HD sub-state gather to [`HdSimdState::new`] and packs the
    /// IT-specific scalars and price ring buffer into SIMD lanes.
    pub fn new(states: &mut [&mut State]) -> Self {
        let mut it_prev_arr = [0.0_f64; N];
        let mut it_prev2_arr = [0.0_f64; N];
        let mut alpha_arr = [0.0_f64; N];

        // First pass: collect IT-specific scalars.
        for (i, state) in states.iter_mut().enumerate() {
            it_prev_arr[i] = state.it_prev;
            it_prev2_arr[i] = state.it_prev2;
            alpha_arr[i] = state.alpha;
        }

        // Second pass: collect mutable hd references for HdSimdState construction.
        let mut hd_refs: Vec<&mut crate::indicators::homodynediscriminator::State> =
            Vec::with_capacity(N);
        for state in states.iter_mut() {
            hd_refs.push(&mut state.hd);
        }

        let hd = HdSimdState::new(&hd_refs);

        Self {
            hd,
            it_prev: Simd::from_array(it_prev_arr),
            it_prev2: Simd::from_array(it_prev2_arr),
            alpha: Simd::from_array(alpha_arr),
        }
    }

    /// Scatters the SIMD state back into `N` scalar [`State`] references.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut hd_refs: Vec<&mut crate::indicators::homodynediscriminator::State> =
            Vec::with_capacity(N);
        let it_prev_arr = self.it_prev.to_array();
        let it_prev2_arr = self.it_prev2.to_array();
        let alpha_arr = self.alpha.to_array();

        for (j, state) in states.iter_mut().enumerate() {
            hd_refs.push(&mut state.hd);
            state.it_prev = it_prev_arr[j];
            state.it_prev2 = it_prev2_arr[j];
            state.alpha = alpha_arr[j];
        }

        self.hd.write_states(&mut hd_refs);
    }

    /// Computes one bar of the Instantaneous Trendline for `N` assets simultaneously.
    ///
    /// Runs the full HD pipeline via [`HdSimdState::calc_simd_unchecked`] to get
    /// `smooth_period` (DC), then applies the 2-pole IIR using SIMD arithmetic.
    ///
    /// After the call:
    /// - `self.it_prev`  = IT (current bar), all lanes
    /// - `self.it_prev2` = IT[1] (previous bar), all lanes
    /// - `self.alpha`    = α used this bar, all lanes
    /// - trigger = `Simd::splat(2.0) * self.it_prev - self.it_prev2`
    ///
    /// # Safety
    ///
    /// All HD ring buffers must be full on entry. Guaranteed after [`State::init_state`]
    /// for every lane before [`SimdState::new`].
    #[inline(always)]
    pub unsafe fn calc_simd_unchecked(&mut self, real: Simd<f64, N>) -> Simd<f64, N> {
        let dc = self.hd.calc_simd_unchecked(real);

        let alpha = F64Constants::<N>::TWO / (dc + F64Constants::<N>::ONE);
        self.alpha = alpha;
        let a2 = alpha * alpha;
        let beta = F64Constants::<N>::ONE - alpha;

        // 4 FMAs: same chain as the scalar hot path.
        let it = (F64Constants::<N>::TWO * beta).mul_add(
            self.it_prev,
            (-(beta * beta)).mul_add(
                self.it_prev2,
                (alpha - a2 * F64Constants::<N>::QUATER).mul_add(
                    self.hd.price_buf[0],
                    (a2 * F64Constants::<N>::HALF).mul_add(
                        self.hd.price_buf[1],
                        -(alpha - a2 * Simd::splat(0.75_f64)) * self.hd.price_buf[2],
                    ),
                ),
            ),
        );

        self.it_prev2 = self.it_prev;
        self.it_prev = it;
        it
    }
}
