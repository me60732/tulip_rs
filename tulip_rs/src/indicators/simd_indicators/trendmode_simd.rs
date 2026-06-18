//! SIMD-parallel state structs for the Ehlers TrendMode.
//!
//! Two sub-modules are provided for the two SIMD parallelism modes:
//!
//! - [`assets`] — `N` independent assets with the same α, full HD + CC in SIMD.
//! - [`options`] — 1 asset with `N` different α values; HD is scalar (shared),
//!   CC runs in SIMD across options.
//!
//! Peak-amplitude tracking (`pk`) is per-lane and updated in a scalar loop
//! inside `calc_simd_unchecked` using the simple `cycle.abs() < 0.2 * pk`
//! condition from Ehlers' original EasyLanguage formula.

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::trendmode::indicator_by_assets;
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::trendmode::indicator_by_options;

// ─────────────────────────────────────────────────────────────────────────────
// assets — N assets, same alpha
// ─────────────────────────────────────────────────────────────────────────────

/// SIMD state for `N` assets with the same α (used by `indicator_by_assets`).
pub mod assets {
    use crate::indicators::simd_indicators::cybercycle_simd::SimdState as CcSimdState;
    use crate::indicators::simd_indicators::homodynediscriminator_simd::SimdState as HdSimdState;
    use crate::indicators::trendmode;
    use std::simd::Simd;

    /// SIMD-parallel state for the Ehlers TrendMode across `N` assets simultaneously.
    ///
    /// The Homodyne Discriminator and CyberCycle pipelines run fully in SIMD
    /// across all `N` lanes. Peak-amplitude tracking runs in a scalar loop.
    pub struct SimdState<const N: usize> {
        /// Embedded HD SIMD state — provides `SmoothPeriod` for all N lanes.
        pub hd: HdSimdState<N>,
        /// Embedded CC SIMD state — provides `Cycle` for all N lanes.
        pub cc: CcSimdState<N>,
        /// Per-asset decaying peak amplitude: `max(pk[1] × 0.991, |Cycle|)`.
        pub pk: [f64; N],
    }

    impl<const N: usize> SimdState<N> {
        /// Gathers `N` scalar [`trendmode::State`] references into a single [`SimdState`].
        pub fn new(states: &mut [&mut trendmode::State]) -> Self {
            let pk: [f64; N] = std::array::from_fn(|j| states[j].pk);

            // Gather HD sub-state.
            let hd = {
                let mut hd_refs = Vec::with_capacity(N);
                for state in states.iter_mut() {
                    hd_refs.push(&mut state.hd);
                }
                HdSimdState::new(&hd_refs)
            };

            // Gather CC sub-state.
            let cc = {
                let mut cc_refs = Vec::with_capacity(N);
                for state in states.iter_mut() {
                    cc_refs.push(&mut state.cc);
                }
                CcSimdState::new(&mut cc_refs)
            };

            Self { hd, cc, pk }
        }

        /// Scatters the SIMD state back into `N` scalar [`trendmode::State`] references.
        pub fn write_states(&self, states: &mut [&mut trendmode::State]) {
            // Write HD sub-state.
            {
                let mut hd_refs = Vec::with_capacity(N);
                for state in states.iter_mut() {
                    hd_refs.push(&mut state.hd);
                }
                self.hd.write_states(&mut hd_refs);
            }

            // Write CC sub-state.
            {
                let mut cc_refs = Vec::with_capacity(N);
                for state in states.iter_mut() {
                    cc_refs.push(&mut state.cc);
                }
                self.cc.write_states(&mut cc_refs);
            }

            // Write per-lane peak scalars.
            for j in 0..N {
                states[j].pk = self.pk[j];
            }
        }

        /// Computes one bar of the TrendMode for `N` assets simultaneously.
        ///
        /// The HD and CC stages run in SIMD. Peak-amplitude tracking and mode
        /// detection are computed per lane in a scalar loop.
        ///
        /// After the call:
        /// - `self.cc.cycle_prev` = Cycle (current bar), all lanes
        /// - `self.pk[j]`         = peak amplitude (current bar), lane j
        ///
        /// Returns a SIMD vector of `1.0` (Trend) / `0.0` (Cycle) per lane.
        ///
        /// # Safety
        ///
        /// All HD and CC ring buffers must be full on entry for every lane.
        /// Guaranteed after [`trendmode::State::init_state`] for every lane.
        #[inline(always)]
        pub unsafe fn calc_simd_unchecked(
            &mut self,
            real: Simd<f64, N>,
            multipliers: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
        ) -> Simd<f64, N> {
            self.hd.calc_simd_unchecked(real);
            let cycle_simd = self.cc.calc_simd_unchecked(real, multipliers);
            let cycle_arr = cycle_simd.to_array();
            let mut trendmode_arr = [0.0_f64; N];

            for j in 0..N {
                self.pk[j] = (self.pk[j] * 0.991).max(cycle_arr[j].abs());
                trendmode_arr[j] = if self.pk[j] > 0.0 && cycle_arr[j].abs() < 0.2 * self.pk[j] {
                    1.0
                } else {
                    0.0
                };
            }

            Simd::from_array(trendmode_arr)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// options — 1 asset, N alpha values
// ─────────────────────────────────────────────────────────────────────────────

/// SIMD state for 1 asset with `N` different α values (used by `indicator_by_options`).
pub mod options {
    use crate::indicators::homodynediscriminator;
    use crate::indicators::simd_indicators::cybercycle_simd::SimdState as CcSimdState;
    use crate::indicators::trendmode;
    use std::simd::Simd;

    /// SIMD-parallel state for the Ehlers TrendMode with `N` different α options
    /// applied to a single shared price series.
    ///
    /// The Homodyne Discriminator runs as a single scalar (all lanes see the same
    /// price so they share one HD state). The CyberCycle and peak tracking run per
    /// lane because each lane uses a different α coefficient.
    pub struct SimdState<const N: usize> {
        /// Shared scalar HD state — same price input for all N options.
        pub hd: homodynediscriminator::State,
        /// Per-option CC SIMD state — one lane per α value.
        pub cc: CcSimdState<N>,
        /// Per-option decaying peak amplitude.
        pub pk: [f64; N],
    }

    impl<const N: usize> SimdState<N> {
        /// Gathers `N` scalar [`trendmode::State`] references into a single [`SimdState`].
        ///
        /// All HD states are identical (same price input), so `states[0].hd` is
        /// cloned as the shared scalar HD. CC and peak fields are gathered per lane.
        pub fn new(states: &mut [&mut trendmode::State]) -> Self {
            // All N lanes processed the same price series → identical HD states.
            let hd = states[0].hd.clone();
            let pk: [f64; N] = std::array::from_fn(|j| states[j].pk);

            // Gather CC sub-state (per-option, different α values).
            let cc = {
                let mut cc_refs = Vec::with_capacity(N);
                for state in states.iter_mut() {
                    cc_refs.push(&mut state.cc);
                }
                CcSimdState::new(&mut cc_refs)
            };

            Self { hd, cc, pk }
        }

        /// Scatters the SIMD state back into `N` scalar [`trendmode::State`] references.
        pub fn write_states(&self, states: &mut [&mut trendmode::State]) {
            // Write CC sub-state.
            {
                let mut cc_refs = Vec::with_capacity(N);
                for state in states.iter_mut() {
                    cc_refs.push(&mut state.cc);
                }
                self.cc.write_states(&mut cc_refs);
            }

            // Write shared HD state and per-lane peak scalars.
            for j in 0..N {
                states[j].hd = self.hd.clone();
                states[j].pk = self.pk[j];
            }
        }

        /// Computes one bar of the TrendMode for `N` α options simultaneously.
        ///
        /// The scalar HD advances once (shared price). The CC runs in SIMD with
        /// per-lane multipliers. Peak tracking and mode detection are per-lane.
        ///
        /// After the call:
        /// - `self.hd.smooth_period` = DC period (current bar, shared)
        /// - `self.cc.cycle_prev`    = Cycle (current bar), all lanes
        /// - `self.pk[j]`            = peak amplitude (current bar), lane j
        ///
        /// Returns a SIMD vector of `1.0` (Trend) / `0.0` (Cycle) per lane.
        ///
        /// # Safety
        ///
        /// All HD and CC ring buffers must be full on entry.
        /// Guaranteed after [`trendmode::State::init_state`] for every lane.
        #[inline(always)]
        pub unsafe fn calc_simd_unchecked(
            &mut self,
            price: f64,
            multipliers: (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
        ) -> Simd<f64, N> {
            self.hd.calc_unchecked(price);
            let real_simd = Simd::splat(price);
            let cycle_simd = self.cc.calc_simd_unchecked(real_simd, multipliers);
            let cycle_arr = cycle_simd.to_array();
            let mut trendmode_arr = [0.0_f64; N];

            for j in 0..N {
                self.pk[j] = (self.pk[j] * 0.991).max(cycle_arr[j].abs());
                trendmode_arr[j] = if self.pk[j] > 0.0 && cycle_arr[j].abs() < 0.2 * self.pk[j] {
                    1.0
                } else {
                    0.0
                };
            }

            Simd::from_array(trendmode_arr)
        }
    }
}
