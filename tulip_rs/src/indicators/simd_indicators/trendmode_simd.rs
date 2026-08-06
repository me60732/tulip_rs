//! SIMD-parallel state structs for the Ehlers TrendMode.
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
//!
//! ## Shared computation
//!
//! Everything after the CC step — peak envelope update and TrendMode classification
//! — is identical in both modes and is factored into the module-level
//! [`trendmode_pipeline`] function.
//!
//! `pk` is `Simd<f64, N>` so the peak update and mode detection are fully
//! vectorised with no per-lane scalar loop.

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::trendmode::indicator_by_assets;
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::trendmode::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};

use std::simd::{cmp::SimdPartialOrd, num::SimdFloat, Select, Simd};
use crate::types::Warm;
/// Shared post-CC TrendMode pipeline used by both `assets` and `options` modes.
///
/// Updates the decaying peak envelope and returns the TrendMode classification
/// vector — all in SIMD, no per-lane loop.
///
/// Steps:
/// 1. `pk = max(pk × 0.991, |cycle|)`
/// 2. `trendmode = 1.0` if `pk > 0` and `|cycle| < 0.2 × pk`, else `0.0`
#[inline(always)]
fn trendmode_pipeline<const N: usize>(cycle: Simd<f64, N>, pk: &mut Simd<f64, N>) -> Simd<f64, N> {
    *pk = (*pk * Simd::splat(0.991)).simd_max(cycle.abs());
    let pk_positive = pk.simd_gt(Simd::splat(0.0));
    let small_cycle = cycle.abs().simd_lt(Simd::splat(0.2) * *pk);
    (pk_positive & small_cycle).select(Simd::splat(1.0_f64), Simd::splat(0.0_f64))
}

// ─────────────────────────────────────────────────────────────────────────────
// assets — N assets, same alpha
// ─────────────────────────────────────────────────────────────────────────────

/// SIMD state for `N` assets with the same α (used by `indicator_by_assets`).
pub mod assets {
    use super::trendmode_pipeline;
    use crate::indicator_types::{TSimdState, TState};
    use crate::indicators::simd_indicators::cybercycle_simd::SimdState as CcSimdState;
    use crate::indicators::simd_indicators::homodynediscriminator_simd::SimdState as HdSimdState;
    use crate::indicators::trendmode::IndicatorState as State;
    use std::simd::{num::SimdFloat, Simd};

    /// SIMD state for N assets with a shared α.
    ///
    /// `hd` is `HdSimdState<N>` because each asset has an independent price
    /// history requiring its own HD pipeline. `pk` is `Simd<f64, N>` — the peak
    /// envelope update and mode detection are fully vectorised.
    pub struct SimdState<const N: usize> {
        /// N independent HD pipelines — one per asset.
        pub hd: HdSimdState<N>,
        /// N independent CC pipelines — one per asset.
        pub cc: CcSimdState<N>,
        /// Per-asset decaying peak amplitude: `max(pk[1] × 0.991, |Cycle|)`.
        pub pk: Simd<f64, N>,
        /// Alpha values for all N lanes (identical across lanes in assets mode).
        /// `alpha[0] == 0.0` signals adaptive mode.
        pub alpha: Simd<f64, N>,
    }

    impl<const N: usize> SimdState<N> {
   

        /// One bar of TrendMode for N assets simultaneously (fixed α).
        ///
        /// HD and CC run in SIMD using the multipliers already on `self.cc` (gathered
        /// from scalar states at construction time); post-CC peak + classification via
        /// [`trendmode_pipeline`] — no scalar loop.
        ///
        /// Returns `Simd<f64, N>` of `1.0` (Trend) / `0.0` (Cycle) per lane.
        ///
        /// # Safety
        ///
        /// All HD and CC ring buffers must be full. Guaranteed after
        /// [`trendmode::State::init_state`] for every lane.
        #[inline(always)]
        fn calc_simd(&mut self, real: Simd<f64, N>) -> Simd<f64, N> {
            self.hd.calc(real);
            let cycle = self.cc.calc(real);
            trendmode_pipeline(cycle, &mut self.pk)
        }

        /// One bar of TrendMode for N assets using **adaptive alpha per lane**.
        ///
        /// HD runs in SIMD — each asset lane has its own `smooth_period`. The per-lane
        /// adaptive alpha is derived via `2 / (smooth_period.max(3) + 1)`, then
        /// per-lane multipliers are computed and fed into CC and `trendmode_pipeline`.
        ///
        /// # Safety
        /// All HD and CC ring buffers must be full. Guaranteed after
        /// [`trendmode::State::init_state`] for every lane.
        #[inline(always)]
        fn calc_adaptive(&mut self, real: Simd<f64, N>) -> Simd<f64, N> {
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
            trendmode_pipeline(cycle, &mut self.pk)
        }
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State;
        crate::simd_state_impl!(
            sub: [(hd: HdSimdState<N>), (cc: CcSimdState<N>)],
            scalar: [pk, alpha]
        );
    }

    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = Simd<f64, N>;
        type Outputs = Simd<f64, N>;
        #[inline(always)]
        fn calc<'a>(&mut self, real: Simd<f64, N>) -> Simd<f64, N> {
            if self.alpha[0] == 0.0 {
                self.calc_adaptive(real)
            } else {
                self.calc_simd(real)
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// options — 1 asset, N alpha values
// ─────────────────────────────────────────────────────────────────────────────

/// SIMD state for 1 asset with `N` different α values (used by `indicator_by_options`).
pub mod options {
    use super::Warm;
    use super::trendmode_pipeline;
    use crate::indicator_types::{TSimdState, TState};
    use crate::indicators::homodynediscriminator;
    use crate::indicators::simd_indicators::cybercycle_simd::SimdState as CcSimdState;
    use crate::indicators::trendmode::IndicatorState as State;
    use std::simd::{Mask, Simd};

    /// SIMD state for 1 asset with N different α values.
    ///
    /// `hd` is a single scalar state because all N option lanes process the same
    /// price series — they share one HD output. `pk` is `Simd<f64, N>` as in the
    /// assets case.
    pub struct SimdState<const N: usize> {
        /// Single shared HD state — same price input for all N lanes.
        pub hd: homodynediscriminator::State<Warm>,
        /// N CC pipelines with per-lane α multipliers.
        pub cc: CcSimdState<N>,
        /// Per-lane decaying peak amplitude.
        pub pk: Simd<f64, N>,
        /// Fixed alpha values per lane (0.0 for adaptive lanes).
        pub fixed_alphas: Simd<f64, N>,
        /// Mask selecting which lanes are adaptive (alpha == 0.0).
        pub adaptive_mask: Mask<i64, N>,
        /// Whether any lane uses adaptive alpha.
        pub has_adaptive: bool,
    }

    impl<const N: usize> SimdState<N> {

        /// One bar of TrendMode for N α-option lanes simultaneously (fixed α).
        ///
        /// HD advances once (shared price, via `real[0]`). CC runs in SIMD using the
        /// multipliers already on `self.cc` (constant for fixed α; set per-bar for
        /// adaptive lanes via [`advance_hd`] + [`advance_cc`]). Post-CC via
        /// [`trendmode_pipeline`].
        ///
        /// Returns `Simd<f64, N>` of `1.0` (Trend) / `0.0` (Cycle) per lane.
        ///
        /// # Safety
        ///
        /// All HD and CC ring buffers must be full. Guaranteed after
        /// [`trendmode::State::init_state`] for every lane.
        #[inline(always)]
        fn calc_simd(&mut self, real: Simd<f64, N>) -> Simd<f64, N> {
            // All lanes share the same price — use lane 0 for the scalar HD.
            self.hd.calc(real[0]);
            let cycle = self.cc.calc(real);
            trendmode_pipeline(cycle, &mut self.pk)
        }

        /// Advances the shared scalar HD one bar and returns the updated `smooth_period`.
        ///
        /// Call this before [`advance_cc`] when computing per-bar adaptive multipliers.
        ///
        /// # Safety
        /// All HD ring buffers must be full on entry.
        #[inline(always)]
        fn advance_hd(&mut self, price: f64) -> f64 {
            self.hd.calc(price);
            self.hd.smooth_period
        }

        /// Advances CC and the peak pipeline for one bar with per-lane `multipliers`.
        ///
        /// Complements [`advance_hd`]: the caller computes the per-lane SIMD multipliers
        /// (e.g. via adaptive mask+select) and passes them here.
        ///
        /// # Safety
        /// CC ring buffers must be full on entry.
        #[inline(always)]
        fn advance_cc(
            &mut self,
            real: Simd<f64, N>,
            multipliers: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
        ) -> Simd<f64, N> {
            self.cc.coef = multipliers.0;
            self.cc.d1 = multipliers.1;
            self.cc.d2 = multipliers.2;
            let cycle = self.cc.calc(real);
            trendmode_pipeline(cycle, &mut self.pk)
        }
    }

    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State;

        fn from_states(states: &mut [&mut State]) -> Self {
            let hd = states[0].hd.clone();
            let pk = Simd::from_array(std::array::from_fn(|j| states[j].pk));
            let cc = {
                let mut refs: Vec<&mut _> = states.iter_mut().map(|s| &mut s.cc).collect();
                CcSimdState::from_states(&mut refs)
            };
            let alpha_arr: [f64; N] = std::array::from_fn(|j| states[j].alpha);
            let is_adaptive_arr: [bool; N] = std::array::from_fn(|j| states[j].is_adaptive);
            let fixed_alphas = Simd::from_array(alpha_arr);
            let adaptive_mask = Mask::from_array(is_adaptive_arr);
            let has_adaptive = is_adaptive_arr.iter().any(|&b| b);
            Self {
                hd,
                cc,
                pk,
                fixed_alphas,
                adaptive_mask,
                has_adaptive,
            }
        }

        fn write_states(&self, states: &mut [&mut State]) {
            {
                let mut refs: Vec<&mut _> = states.iter_mut().map(|s| &mut s.cc).collect();
                TSimdState::write_states(&self.cc, &mut refs);
            }
            let pk = self.pk.to_array();
            for j in 0..N {
                states[j].hd = self.hd.clone();
                states[j].pk = pk[j];
            }
        }
    }

    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = Simd<f64, N>;
        type Outputs = Simd<f64, N>;

        fn calc<'a>(&mut self, real: Simd<f64, N>) -> Simd<f64, N> {
            use crate::indicators::cybercycle::adaptive_alpha;
            use std::simd::Select;
            if self.has_adaptive {
                let smooth_period = self.advance_hd(real[0]);
                let adap_a = Simd::splat(adaptive_alpha(smooth_period));
                let effective_alpha = self.adaptive_mask.select(adap_a, self.fixed_alphas);
                let one = Simd::splat(1.0_f64);
                let c = one - Simd::splat(0.5_f64) * effective_alpha;
                let b = one - effective_alpha;
                let bar_mults = (c * c, Simd::splat(2.0_f64) * b, b * b);
                self.advance_cc(real, bar_mults)
            } else {
                self.calc_simd(real)
            }
        }
    }
}
