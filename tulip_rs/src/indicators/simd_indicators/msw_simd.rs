#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::msw::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::msw::indicator_by_options;

pub mod imports {
    //! Shared imports, constants and helpers for the Mesa Sine Wave (MSW) indicator.
    pub(crate) use crate::indicators::msw::MSWConstants;
    pub(crate) use crate::indicators::simd_indicators::simd_types::F64Constants;
    pub(crate) use crate::math_simd::trig::{simd_atan, simd_sin, simd_sin_cos};
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

pub mod assets {
    //! Per-asset road compute function for the Mesa Sine Wave (MSW) indicator.
    use super::imports::*;
    /// Computes one bar of the Mesa Sine Wave (MSW) indicator for `N` assets simultaneously
    /// using SIMD parallelism.
    ///
    /// Accumulates the cosine/sine Hilbert transform over `prev_slice` using the given
    /// frequency `multiplier`, then derives the sine-wave and lead-sine outputs.
    ///
    /// # Arguments
    ///
    /// * `prev_slice` - Ordered window of recent prices as SIMD vectors (one per bar).
    /// * `multiplier` - Per-lane frequency factor `2π / period`.
    ///
    /// # Returns
    ///
    /// A tuple `(sine, lead_sine)` for all `N` lanes.
    #[inline(always)]
    pub fn calc_simd<const N: usize>(
        prev_slice: &[Simd<f64, N>],
        multiplier: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let mut rp = Simd::splat(0.0);
        let mut ip = Simd::splat(0.0);
        let len = prev_slice.len();

        // Pre-compute reciprocal to avoid repeated division
        let angle_factor = MSWConstants::TPI * multiplier;

        // Pre-compute len-1 to avoid repeated subtraction
        let len_minus_1 = (len - 1) as f64;

        // Accumulate rp and ip
        for (idx, &weight) in prev_slice.iter().enumerate() {
            let j_vals = Simd::splat(len_minus_1 - idx as f64);
            let angle = angle_factor * j_vals;
            let (sin_vals, cos_vals) = simd_sin_cos(angle);

            // Use FMA if available for better performance and accuracy
            rp = cos_vals.mul_add(weight, rp); //
            ip = sin_vals.mul_add(weight, ip);
        }

        calc_msw(rp, ip)
    }
}

pub mod options {
    //! Per-option road compute function for the Mesa Sine Wave (MSW) indicator.
    use super::imports::*;
    use crate::indicators::msw::calc_rp_ip;
    /// Computes one bar of the Mesa Sine Wave (MSW) indicator for `N` option lanes simultaneously
    /// using SIMD parallelism.
    ///
    /// Each lane may have a different period; the function computes its own Hilbert transform
    /// from raw price data and derives the sine-wave and lead-sine outputs.
    ///
    /// # Arguments
    ///
    /// * `real` - Per-lane raw price pointers.
    /// * `periods` - Per-lane look-back periods.
    /// * `multiplier` - Per-lane frequency factors `2π / period`.
    /// * `i` - Per-lane current bar indices.
    ///
    /// # Returns
    ///
    /// A tuple `(sine, lead_sine)` for all `N` lanes.
    #[inline(always)]
    pub fn calc_simd<const N: usize>(
        real: [*const f64; N],
        periods: [usize; N],
        multiplier: [f64; N],
        i: [usize; N],
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let (mut rp, mut ip) = ([0.0; N], [0.0; N]);
        for (lane, (&i, &period)) in i.iter().zip(periods.iter()).enumerate() {
            let start = i + 1 - period;
            let slice = unsafe { std::slice::from_raw_parts(real[lane].add(start), period) };
            (rp[lane], ip[lane]) = calc_rp_ip::<N>(slice, multiplier[lane])
        }

        calc_msw(Simd::from_array(rp), Simd::from_array(ip))
    }
}
