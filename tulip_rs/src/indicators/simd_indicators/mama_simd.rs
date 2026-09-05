pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::mama::IndicatorState as State;
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::mama::indicator_by_assets;
#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::mama::indicator_by_options;
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
    pub hd: HdSimdState<N>,
    prev_phase: Simd<f64, N>,
    mama: Simd<f64, N>,
    fama: Simd<f64, N>,
    fast_limit: Simd<f64, N>,
    slow_limit: Simd<f64, N>,
    pub alpha: Simd<f64, N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;
    crate::simd_state_from_state!(
         sub: [(hd: HdSimdState<N>)],
         scalar: [prev_phase, mama, fama, fast_limit, slow_limit, alpha]
    );
    crate::simd_state_write!(
         sub: [(hd: HdSimdState<N>)],
         scalar: [prev_phase, mama, fama, alpha]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = Simd<f64, N>;
    type Outputs = (Simd<f64, N>, Simd<f64, N>);
    
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        real: Self::Inputs<'a>,
    ) -> Self::Outputs {
        let (_, i1, q1) = self.hd.calc_with_iq(real);
        self.apply_mama_simd(real, i1, q1);
        (self.mama, self.fama)
    }
}
impl<const N: usize> SimdState<N> {

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
        self.alpha = (self.fast_limit / delta_phase)
            .simd_min(self.fast_limit)
            .simd_max(self.slow_limit);

        // MAMA — EMA with adaptive alpha.
        self.mama = self
            .alpha
            .mul_add(real, (Simd::splat(1.0) - self.alpha) * self.mama);

        // FAMA — EMA at half the alpha.
        let half_alpha = Simd::splat(0.5) * self.alpha;
        self.fama = half_alpha.mul_add(self.mama, (Simd::splat(1.0) - half_alpha) * self.fama);
    }
}

