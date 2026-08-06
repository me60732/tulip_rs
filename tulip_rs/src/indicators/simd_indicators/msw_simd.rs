pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::msw::State;
use std::simd::Simd;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::msw::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::msw::indicator_by_options;

/// SIMD-parallel SDFT state for the MSW by-options path.
///
/// Packs `N` scalar [`State`]s (one per period/option lane) into SIMD vectors so that
/// the per-bar SDFT recurrence can be applied to all lanes in a single vectorized step.
/// Mirrors the `SimdState` pattern used by `adosc_simd`, `adaptivemsw_simd`, etc.
pub struct SimdState<const N: usize> {
    /// SDFT real accumulator — one per lane.
    pub rp: Simd<f64, N>,
    /// SDFT imaginary accumulator — one per lane.
    pub ip: Simd<f64, N>,
    /// Rotation phasor real part `cos(2π/period)` — constant per lane.
    pub wr: Simd<f64, N>,
    /// Rotation phasor imaginary part `sin(2π/period)` — constant per lane.
    pub wi: Simd<f64, N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;

    fn from_states(states: &mut [&mut State]) -> Self {
        Self {
            rp: Simd::from_array(std::array::from_fn(|i| states[i].rp)),
            ip: Simd::from_array(std::array::from_fn(|i| states[i].ip)),
            wr: Simd::from_array(std::array::from_fn(|i| states[i].wr)),
            wi: Simd::from_array(std::array::from_fn(|i| states[i].wi)),
        }
    }

    fn write_states(&self, states: &mut [&mut State]) {
        let rp = self.rp.to_array();
        let ip = self.ip.to_array();
        for i in 0..N {
            states[i].rp = rp[i];
            states[i].ip = ip[i];
        }
    }
}

impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>); // (new_sample, old_sample)
    type Outputs = (Simd<f64, N>, Simd<f64, N>); // (sine, lead_sine)

    #[inline(always)]
    fn calc<'a>(&mut self, (new_sample, old_sample): Self::Inputs<'a>) -> Self::Outputs {
        options::calc_sdft(self, new_sample, old_sample)
    }
}

pub mod imports {
    //! Shared imports, constants and helpers for the Mesa Sine Wave (MSW) indicator.
    pub(crate) use crate::indicators::msw::MSWConstants;
    pub(crate) use crate::indicators::simd_indicators::simd_types::F64Constants;
    pub(crate) use crate::math_simd::trig::{simd_atan, simd_sin};
    use std::f64::consts::PI;
    pub(crate) use std::simd::{cmp::SimdPartialOrd, num::SimdFloat, Select, Simd, StdFloat};
    /// Trait exposing SIMD-splat constants for MSW angle calculations.
    pub(crate) trait Constants<const N: usize> {
        const HPI: Simd<f64, N> = Simd::splat(PI * 0.5);
        const QPI: Simd<f64, N> = Simd::splat(PI * 0.25);
        const THRESHOLD: Simd<f64, N> = Simd::splat(0.001);
        const PI: Simd<f64, N> = Simd::splat(PI);
    }
    impl<const N: usize> Constants<N> for MSWConstants<N> {}

    /// Computes the sine-wave and lead-line phases from the real (RP) and imaginary (IP) parts
    /// of the Hilbert transform for `N` lanes simultaneously.
    ///
    /// Returns `(sine, lead_sine)` where `lead_sine` is phase-shifted by `π/4`.
    #[inline(always)]
    pub(crate) fn calc_msw<const N: usize>(
        rp: Simd<f64, N>,
        ip: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let phase = rp.abs().simd_gt(MSWConstants::THRESHOLD).select(
            simd_atan(ip / rp),
            MSWConstants::PI
                * ip.simd_lt(F64Constants::ZERO)
                    .select(F64Constants::NEG_ONE, F64Constants::ONE),
        );

        let mut phase = rp
            .simd_lt(F64Constants::ZERO)
            .select(phase + MSWConstants::PI, phase);
        phase += MSWConstants::HPI;
        phase = phase
            .simd_lt(F64Constants::ZERO)
            .select(phase + MSWConstants::TPI, phase);

        phase = phase
            .simd_gt(MSWConstants::TPI)
            .select(phase - MSWConstants::TPI, phase);

        (simd_sin(phase), simd_sin(phase + MSWConstants::QPI))
    }
}

pub mod options {
    use super::{imports::*, SimdState};

    /// Advances the Sliding DFT by one bar for `N` option lanes simultaneously.
    ///
    /// Applies the O(1) SDFT recurrence vectorized across all lanes:
    /// ```text
    /// rp_new = wr·rp − wi·ip + (new_sample − old_sample)
    /// ip_new = wr·ip + wi·rp
    /// ```
    /// Updates `state.rp` and `state.ip` in-place and returns `(sine, lead_sine)`.
    #[inline(always)]
    pub fn calc_sdft<const N: usize>(
        state: &mut SimdState<N>,
        new_sample: Simd<f64, N>,
        old_sample: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let diff = new_sample - old_sample;
        let rp_new = state
            .wr
            .mul_add(state.rp, (-state.wi).mul_add(state.ip, diff));
        let ip_new = state.wr.mul_add(state.ip, state.wi * state.rp); // uses OLD rp
        state.rp = rp_new;
        state.ip = ip_new;
        calc_msw(state.rp, state.ip)
    }
}

pub mod assets {
    use super::imports::*;

    /// Per-bar inner loop for the by-assets path — zero trig, pure SIMD FMA.
    ///
    /// `prev_slice` has one `Simd<f64, N>` per window position (N asset prices packed
    /// together). Each scalar twiddle is broadcast across all N lanes.
    #[inline(always)]
    pub fn calc_simd_precomputed<const N: usize>(
        prev_slice: &[Simd<f64, N>],
        cos_twiddles: &[f64],
        sin_twiddles: &[f64],
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let mut rp = Simd::<f64, N>::splat(0.0);
        let mut ip = Simd::<f64, N>::splat(0.0);

        for (k, &weight) in prev_slice.iter().enumerate() {
            rp = Simd::<f64, N>::splat(cos_twiddles[k]).mul_add(weight, rp);
            ip = Simd::<f64, N>::splat(sin_twiddles[k]).mul_add(weight, ip);
        }

        calc_msw(rp, ip)
    }
}
