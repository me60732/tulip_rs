use crate::indicators::mama::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::mama::indicator_by_assets;
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::mama::indicator_by_options;
use crate::indicators::simd_indicators::homodynediscriminator_simd::SimdState as HdSimdState;
use crate::math_simd::trig::simd_atan;
use std::simd::{cmp::SimdPartialEq, num::SimdFloat, Select, Simd, StdFloat};

/// SIMD-parallel state for the Ehlers MESA Adaptive Moving Average (MAMA) and
/// Following Adaptive Moving Average (FAMA) across `N` assets simultaneously.
///
/// Composes [`HdSimdState`](super::homodynediscriminator_simd::SimdState) as the `hd` field —
/// the full four-stage Hilbert Transform cascade and homodyne discriminator — and adds four
/// MAMA-specific SIMD fields on top, exactly mirroring how the scalar [`mama::State`] composes
/// [`homodynediscriminator::State`](crate::indicators::homodynediscriminator::State).
///
/// The gather (`new`) and scatter (`write_states`) methods delegate to the nested HD state's
/// own `new` / `write_states`, collecting `hd` references and the MAMA scalars in a single
/// loop pass — the same pattern used in `adx_simd::SimdState` and
/// `hilberttransform_simd::SimdState`.
pub struct SimdState<const N: usize> {
    /// Embedded Homodyne Discriminator SIMD state — runs stages 0–3 (Hann smooth,
    /// Detrender, I1/Q1, jI/jQ) and the homodyne discriminator IIR.
    pub hd: HdSimdState<N>,
    /// Instantaneous phase (degrees) from the previous bar, one per lane.
    prev_phase: Simd<f64, N>,
    /// Current MAMA value, one per lane.
    mama: Simd<f64, N>,
    /// Current FAMA value, one per lane.
    fama: Simd<f64, N>,
    /// Last computed adaptive alpha (stored for optional output), one per lane.
    pub alpha: Simd<f64, N>,
}

impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single [`SimdState`].
    ///
    /// Delegates the HD sub-state gather to [`HdSimdState::new`] and packs the
    /// MAMA-specific scalars into SIMD lanes, all in a single loop pass.
    pub fn new(states: &mut [&mut State]) -> Self {
        let mut hd_refs = Vec::with_capacity(N);
        let mut prev_phase = [0.0_f64; N];
        let mut mama = [0.0_f64; N];
        let mut fama = [0.0_f64; N];
        let mut alpha = [0.0_f64; N];

        for (i, state) in states.iter_mut().enumerate() {
            hd_refs.push(&mut state.hd);
            prev_phase[i] = state.prev_phase;
            mama[i] = state.mama;
            fama[i] = state.fama;
            alpha[i] = state.alpha;
        }

        // HdSimdState::new takes &[&mut HdState]
        let hd = HdSimdState::new(&hd_refs);

        Self {
            hd,
            prev_phase: Simd::from_array(prev_phase),
            mama: Simd::from_array(mama),
            fama: Simd::from_array(fama),
            alpha: Simd::from_array(alpha),
        }
    }

    /// Scatters the SIMD state back into `N` scalar [`State`] references.
    ///
    /// Delegates the HD sub-state scatter to [`HdSimdState::write_states`] and unpacks
    /// the MAMA-specific SIMD lanes back to scalars, all in a single loop pass.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut hd_refs = Vec::with_capacity(N);
        let prev_phase = self.prev_phase.to_array();
        let mama = self.mama.to_array();
        let fama = self.fama.to_array();
        let alpha = self.alpha.to_array();

        for (i, state) in states.iter_mut().enumerate() {
            hd_refs.push(&mut state.hd);
            state.prev_phase = prev_phase[i];
            state.mama = mama[i];
            state.fama = fama[i];
            state.alpha = alpha[i];
        }

        // HdSimdState::write_states takes &mut [&mut HdState]
        self.hd.write_states(&mut hd_refs);
    }

    /// Computes one bar of MAMA / FAMA for `N` assets simultaneously.
    ///
    /// Delegates the full HD pipeline to [`HdSimdState::calc_simd_unchecked_with_iq`]
    /// which returns `(smooth_period, i1, q1)`, then runs the MAMA-specific stage
    /// (`simd_atan`-based phase, delta-phase, alpha, EMA) via [`apply_mama_simd`].
    ///
    /// # Safety
    ///
    /// All HD ring buffers must be full on entry. Guaranteed after [`State::init_state`]
    /// has been called for every lane before [`SimdState::new`].
    #[inline(always)]
    pub unsafe fn calc_simd_unchecked(
        &mut self,
        real: Simd<f64, N>,
        fast_limits: Simd<f64, N>,
        slow_limits: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let (_, i1, q1) = self.hd.calc_simd_unchecked_with_iq(real);
        self.apply_mama_simd(real, i1, q1, fast_limits, slow_limits);
        (self.mama, self.fama)
    }

    /// Applies the MAMA-specific stage for all `N` lanes.
    ///
    /// Mirrors [`mama::State::apply_mama`](crate::indicators::mama::State::apply_mama) exactly:
    /// - `if i1 != 0.0` guard → `i1.simd_ne(zero).select(simd_atan(q1/i1) * RAD_TO_DEG, zero)`
    /// - `f64::max(1.0)` → `Simd::simd_max(Simd::splat(1.0))`
    /// - `f64::clamp(slow, fast)` → `.simd_min(fast_limits).simd_max(slow_limits)`
    /// - `f64::mul_add` → `Simd::mul_add` (via `StdFloat`)
    #[inline(always)]
    fn apply_mama_simd(
        &mut self,
        real: Simd<f64, N>,
        i1: Simd<f64, N>,
        q1: Simd<f64, N>,
        fast_limits: Simd<f64, N>,
        slow_limits: Simd<f64, N>,
    ) {
        let zero = Simd::splat(0.0_f64);
        let rad_to_deg = Simd::splat(180.0 / std::f64::consts::PI);

        // Instantaneous phase in degrees. Guard I1 = 0 with a branchless select.
        let phase = i1
            .simd_ne(zero)
            .select(simd_atan(q1 / i1) * rad_to_deg, zero);

        // DeltaPhase = prev − current (phase advances as cycles progress).
        // Floor at 1° to prevent ÷0 and runaway alpha.
        let delta_phase = (self.prev_phase - phase).simd_max(Simd::splat(1.0));
        self.prev_phase = phase;

        // Adaptive alpha, clamped to [slow_limits, fast_limits].
        self.alpha = (fast_limits / delta_phase)
            .simd_min(fast_limits)
            .simd_max(slow_limits);

        // MAMA — EMA with adaptive alpha.
        self.mama = self
            .alpha
            .mul_add(real, (Simd::splat(1.0) - self.alpha) * self.mama);

        // FAMA — EMA at half the alpha.
        let half_alpha = Simd::splat(0.5) * self.alpha;
        self.fama = half_alpha.mul_add(self.mama, (Simd::splat(1.0) - half_alpha) * self.fama);
    }
}
