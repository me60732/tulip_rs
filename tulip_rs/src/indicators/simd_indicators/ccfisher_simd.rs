//! SIMD-parallel state structs for the Ehlers CyberCycle Fisher.
//!
//! Two sub-modules are provided for the two SIMD parallelism modes:
//!
//! - [`assets`] — `N` independent assets with the same α. Each lane has its own
//!   HD pipeline (`HdSimdState<N>`), its own CC pipeline, and its own price input.
//! - [`options`] — 1 asset with `N` different α values. HD is a single shared
//!   scalar state (all lanes see the same price); CC runs in SIMD with per-lane
//!   multipliers.
//!
//! ## Why two separate `SimdState` structs?
//!
//! The only structural difference is the `hd` field:
//! - `assets` needs `HdSimdState<N>` (N independent HD pipelines).
//! - `options` needs a scalar `homodynediscriminator::State` (one shared HD).
//! Because the field types differ, a single struct cannot cover both cases.
//!
//! ## Shared computation
//!
//! Everything after the CC step — peak envelope, normalise, smooth, clamp, and
//! the Fisher transform — is identical in both modes. This is factored into the
//! module-level [`fisher_pipeline`] function so there is no duplicated code.
//!
//! `pk`, `val1`, and `fish` are `Simd<f64, N>` so the whole pipeline is
//! vectorised. The natural logarithm uses [`crate::math_simd::ln_unchecked`],
//! which is safe because `val1` is clamped to `(−0.999, 0.999)`, making
//! `ln_arg = (1 + val1) / (1 − val1)` always strictly positive.

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::ccfisher::indicator_by_assets;
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::ccfisher::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};

use crate::types::Warm;
use std::simd::{cmp::SimdPartialOrd, num::SimdFloat, Select, Simd, StdFloat};

/// Shared post-CC Fisher pipeline used by both `assets` and `options` modes.
///
/// Given the current `cycle` vector and mutable references to the per-lane
/// state (`pk`, `val1`, `fish`), advances one bar and returns `(fisher, signal)`.
///
/// Steps:
/// 1. Peak envelope: `pk = max(pk × 0.991, |cycle|)`
/// 2. Normalise: `value = cycle / pk` (zero when `pk == 0`)
/// 3. Smooth + clamp: `val1 = clamp(0.65·val1 + 0.35·value, −0.999, 0.999)`
/// 4. Fisher: `fisher = 0.5 × ln((1 + val1) / (1 − val1))`  — via `ln_unchecked`
/// 5. Signal: `signal = fish` (previous bar); `fish = fisher`
///
/// # Safety
///
/// Caller must guarantee `val1` stays in `[−0.999, 0.999]` (enforced by the
/// clamp in step 3), which makes `ln_arg` always strictly positive.
#[inline(always)]
fn fisher_pipeline<const N: usize>(
    cycle: Simd<f64, N>,
    pk: &mut Simd<f64, N>,
    val1: &mut Simd<f64, N>,
    fish: &mut Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>) {
    // 1. Peak envelope
    *pk = (*pk * Simd::splat(0.991)).simd_max(cycle.abs());

    // 2. Normalise — branchless zero-guard
    let value = pk
        .simd_gt(Simd::splat(0.0))
        .select(cycle / *pk, Simd::splat(0.0));

    // 3. Smooth + clamp
    *val1 = Simd::splat(0.35_f64)
        .mul_add(value, Simd::splat(0.65) * *val1)
        .simd_max(Simd::splat(-0.999))
        .simd_min(Simd::splat(0.999));

    // 4. Fisher transform — ln_unchecked safe: val1 ∈ [−0.999, 0.999] → ln_arg > 0
    let ln_arg = (Simd::splat(1.0) + *val1) / (Simd::splat(1.0) - *val1);
    let fisher = Simd::splat(0.5) * unsafe { crate::math_simd::ln_unchecked(ln_arg) };

    // 5. Signal
    let signal = *fish;
    *fish = fisher;

    (fisher, signal)
}

// ─────────────────────────────────────────────────────────────────────────────
// assets — N assets, same alpha
// ─────────────────────────────────────────────────────────────────────────────

/// SIMD state for `N` assets with the same α (used by `indicator_by_assets`).
pub mod assets {
    use super::fisher_pipeline;
    use super::Warm;
    use crate::indicator_types::{TSimdState, TState};
    use crate::indicators::ccfisher::State;
    use crate::indicators::simd_indicators::cybercycle_simd::SimdState as CcSimdState;
    use crate::indicators::simd_indicators::homodynediscriminator_simd::SimdState as HdSimdState;
    use std::simd::Simd;

    /// SIMD state for N assets with a shared α.
    ///
    /// `hd` is `HdSimdState<N>` because each asset has an independent price
    /// history and therefore needs its own HD pipeline. `cc`, `pk`, `val1`, and
    /// `fish` are `Simd<f64, N>` covering all N lanes simultaneously.
    pub struct SimdState<const N: usize> {
        /// N independent HD pipelines — one per asset.
        pub hd: HdSimdState<N>,
        /// N independent CC pipelines — one per asset.
        pub cc: CcSimdState<N>,
        /// Per-asset decaying peak amplitude.
        pub pk: Simd<f64, N>,
        /// Per-asset Fisher-transform smoother (clamped to ±0.999).
        pub val1: Simd<f64, N>,
        /// Per-asset previous Fisher value — becomes `signal` on the next bar.
        pub fish: Simd<f64, N>,
        /// Whether the alpha is adaptive (alpha == 0.0). Drives dispatch in `TState::calc`.
        pub is_adaptive: bool,
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;
        /// Gathers `N` scalar [`ccfisher::State`] references into a `SimdState`.
        fn from_states(states: &mut [&mut Self::ScalarState]) -> Self {
            let pk = Simd::from_array(std::array::from_fn(|j| states[j].pk));
            let val1 = Simd::from_array(std::array::from_fn(|j| states[j].val1));
            let fish = Simd::from_array(std::array::from_fn(|j| states[j].fish));

            let hd = {
                let mut refs: Vec<&mut _> = states.iter_mut().map(|s| &mut s.hd).collect();
                HdSimdState::from_states(&mut refs)
            };
            let cc = {
                let mut refs: Vec<&mut _> = states.iter_mut().map(|s| &mut s.cc).collect();
                CcSimdState::from_states(&mut refs)
            };

            Self {
                hd,
                cc,
                pk,
                val1,
                fish,
                is_adaptive: states[0].alpha == 0.0,
            }
        }

        /// Scatters the SIMD state back into `N` scalar [`ccfisher::State`] references.
        fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
            {
                let mut refs: Vec<&mut _> = states.iter_mut().map(|s| &mut s.hd).collect();
                TSimdState::write_states(&self.hd, &mut refs);
            }
            {
                let mut refs: Vec<&mut _> = states.iter_mut().map(|s| &mut s.cc).collect();
                TSimdState::write_states(&self.cc, &mut refs);
            }
            let pk = self.pk.to_array();
            let val1 = self.val1.to_array();
            let fish = self.fish.to_array();
            for j in 0..N {
                states[j].pk = pk[j];
                states[j].val1 = val1[j];
                states[j].fish = fish[j];
            }
        }
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = Simd<f64, N>;
        type Outputs = (Simd<f64, N>, Simd<f64, N>);
        fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
            self.calc_dispatch(real)
        }
    }
    impl<const N: usize> SimdState<N> {
        fn calc_dispatch(&mut self, real: Simd<f64, N>) -> (Simd<f64, N>, Simd<f64, N>) {
            if self.is_adaptive {
                self.calc_adaptive(real)
            } else {
                self.hd.calc(real);
                let cycle = self.cc.calc(real);
                fisher_pipeline(cycle, &mut self.pk, &mut self.val1, &mut self.fish)
            }
        }

        #[inline(always)]
        pub fn calc_adaptive(&mut self, real: Simd<f64, N>) -> (Simd<f64, N>, Simd<f64, N>) {
            use std::simd::num::SimdFloat;
            self.hd.calc(real);
            let effective_period = self.hd.smooth_period.simd_max(Simd::splat(3.0_f64));
            let alpha = Simd::splat(2.0_f64) / (effective_period + Simd::splat(1.0_f64));
            let one = Simd::splat(1.0_f64);
            let c = one - Simd::splat(0.5_f64) * alpha;
            let b = one - alpha;
            self.cc.coef = c * c;
            self.cc.d1 = Simd::splat(2.0_f64) * b;
            self.cc.d2 = b * b;
            let cycle = self.cc.calc(real);
            fisher_pipeline(cycle, &mut self.pk, &mut self.val1, &mut self.fish)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// options — 1 asset, N alpha values
// ─────────────────────────────────────────────────────────────────────────────

/// SIMD state for 1 asset with `N` different α values (used by `indicator_by_options`).
pub mod options {
    use super::fisher_pipeline;
    use super::Warm;
    use crate::indicator_types::{TSimdState, TState};
    use crate::indicators::ccfisher::State;
    use crate::indicators::homodynediscriminator;
    use crate::indicators::simd_indicators::cybercycle_simd::SimdState as CcSimdState;
    use std::simd::Simd;

    /// SIMD state for 1 asset with N different α values.
    ///
    /// `hd` is a single scalar state because all N option lanes process the same
    /// price series — they share one HD output. `cc` runs in SIMD with per-lane
    /// multipliers; `pk`, `val1`, `fish` are `Simd<f64, N>` as in the assets case.
    pub struct SimdState<const N: usize> {
        /// Single shared HD state — same price input for all N lanes.
        pub hd: homodynediscriminator::State<Warm>,
        /// N CC pipelines with per-lane α multipliers.
        pub cc: CcSimdState<N>,
        /// Per-lane decaying peak amplitude.
        pub pk: Simd<f64, N>,
        /// Per-lane Fisher-transform smoother (clamped to ±0.999).
        pub val1: Simd<f64, N>,
        /// Per-lane previous Fisher value — becomes `signal` on the next bar.
        pub fish: Simd<f64, N>,
        /// Fixed alpha values per lane (used when lane is not adaptive).
        pub fixed_alphas: Simd<f64, N>,
        /// Mask: `true` for lanes where alpha is adaptive (alpha == 0.0).
        pub adaptive_mask: std::simd::Mask<i64, N>,
        /// Whether any lane uses adaptive alpha.
        pub is_adaptive: bool,
    }
    impl<const N: usize> SimdState<N> {
        fn calc_dispatch(&mut self, real: Simd<f64, N>) -> (Simd<f64, N>, Simd<f64, N>) {
            use crate::indicators::cybercycle::adaptive_alpha;
            use std::simd::Select;
            if self.is_adaptive {
                let smooth_period = self.advance_hd(real[0]);
                let adap_a = Simd::splat(adaptive_alpha(smooth_period));
                let effective_alpha = self.adaptive_mask.select(adap_a, self.fixed_alphas);
                let one = Simd::splat(1.0_f64);
                let c = one - Simd::splat(0.5_f64) * effective_alpha;
                let b = one - effective_alpha;
                let bar_mults = (c * c, Simd::splat(2.0_f64) * b, b * b);
                self.advance_cc(real, bar_mults)
            } else {
                self.hd.calc(real[0]);
                let cycle = self.cc.calc(real);
                fisher_pipeline(cycle, &mut self.pk, &mut self.val1, &mut self.fish)
            }
        }
        #[inline(always)]
        fn advance_hd(&mut self, price: f64) -> f64 {
            self.hd.calc(price);
            self.hd.smooth_period
        }

        /// Advances CC and the Fisher pipeline for one bar with per-lane `multipliers`.
        ///
        /// Complements [`advance_hd`]: the caller computes per-lane SIMD multipliers
        /// (e.g. via adaptive mask+select) and passes them here.
        ///
        /// # Safety
        /// CC ring buffers must be full on entry.
        #[inline(always)]
        fn advance_cc(
            &mut self,
            real: Simd<f64, N>,
            multipliers: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
        ) -> (Simd<f64, N>, Simd<f64, N>) {
            self.cc.coef = multipliers.0;
            self.cc.d1 = multipliers.1;
            self.cc.d2 = multipliers.2;
            let cycle = self.cc.calc(real);
            fisher_pipeline(cycle, &mut self.pk, &mut self.val1, &mut self.fish)
        }
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;

        /// Gathers `N` scalar [`ccfisher::State`] references into a `SimdState`.
        ///
        /// All N lanes have identical HD states (same price), so `states[0].hd`
        /// is cloned as the shared scalar HD.
        fn from_states(states: &mut [&mut Self::ScalarState]) -> Self {
            let hd = states[0].hd.clone();
            let pk = Simd::from_array(std::array::from_fn(|j| states[j].pk));
            let val1 = Simd::from_array(std::array::from_fn(|j| states[j].val1));
            let fish = Simd::from_array(std::array::from_fn(|j| states[j].fish));
            let cc = {
                let mut refs: Vec<&mut _> = states.iter_mut().map(|s| &mut s.cc).collect();
                CcSimdState::from_states(&mut refs)
            };
            let alpha_arr: [f64; N] = std::array::from_fn(|j| states[j].alpha);
            let is_adaptive_arr: [bool; N] = std::array::from_fn(|j| states[j].alpha == 0.0);
            let fixed_alphas = Simd::from_array(alpha_arr);
            let adaptive_mask = std::simd::Mask::from_array(is_adaptive_arr);
            let has_adaptive = is_adaptive_arr.iter().any(|&b| b);
            Self {
                hd,
                cc,
                pk,
                val1,
                fish,
                fixed_alphas,
                adaptive_mask,
                is_adaptive: has_adaptive,
            }
        }

        /// Scatters the SIMD state back into `N` scalar [`ccfisher::State`] references.
        fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
            {
                let mut refs: Vec<&mut _> = states.iter_mut().map(|s| &mut s.cc).collect();
                TSimdState::write_states(&self.cc, &mut refs);
            }
            let pk = self.pk.to_array();
            let val1 = self.val1.to_array();
            let fish = self.fish.to_array();
            for j in 0..N {
                states[j].hd = self.hd.clone();
                states[j].pk = pk[j];
                states[j].val1 = val1[j];
                states[j].fish = fish[j];
            }
        }
    }

    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = Simd<f64, N>;
        type Outputs = (Simd<f64, N>, Simd<f64, N>);

        fn calc<'a>(&mut self, real: Simd<f64, N>) -> (Simd<f64, N>, Simd<f64, N>) {
            self.calc_dispatch(real)
        }
    }
}
