use crate::indicators::hilberttransform::{State, C0, C1, C2, C3};
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::hilberttransform::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::hilberttransform::indicator_by_options;

use crate::indicators::simd_indicators::roofingfilter_simd::SimdState as RfSimdState;
use crate::ring_buffer::fixed_single_buffer::{FixedRingBuffer, FixedSimdRingBuffer};
use std::simd::{Simd, StdFloat};

/// Pre-broadcast SIMD constants for the seven-tap Hilbert kernel.
///
/// Stores [`C0`]–[`C3`] as `Simd::splat` values so that per-bar coefficient
/// multiplies are pure vector operations with no scalar-to-vector broadcast cost.
pub struct COEFS<const N: usize>;
impl<const N: usize> COEFS<N> {
    pub const C0: Simd<f64, N> = Simd::splat(C0);
    pub const C1: Simd<f64, N> = Simd::splat(C1);
    pub const C2: Simd<f64, N> = Simd::splat(C2);
    pub const C3: Simd<f64, N> = Simd::splat(C3);
}

/// SIMD-parallel state for computing the Hilbert Transform across `N` assets simultaneously.
///
/// Holds two pieces of state:
/// * `buffer` — seven-slot ring buffer of SIMD vectors fed by the roofing filter output;
///   used to compute the in-phase (`I`) and quadrature (`Q`) components.
/// * `rf_state` — cascaded roofing filter (HighPass → SuperSmoother) that band-limits
///   the input before the Hilbert kernel.
pub struct SimdState<const N: usize> {
    buffer: FixedRingBuffer<Simd<f64, N>, 7>,
    rf_state: RfSimdState<N>,
}

impl<const N: usize> SimdState<N> {
    /// Gathers `N` scalar [`State`] references into a single [`SimdState`],
    /// packing each asset's ring buffer and roofing-filter state into their
    /// respective SIMD lanes.
    pub fn new(states: &mut [&mut State]) -> Self {
        let mut buffer_refs = Vec::with_capacity(N);
        let mut rf_state = Vec::with_capacity(N);

        for state in states.iter_mut() {
            buffer_refs.push(&state.buffer);
            rf_state.push(&mut state.rf_state);
        }
        let rf_state = RfSimdState::new(&mut rf_state);
        let buffer = FixedSimdRingBuffer::from_f64_buffers(&buffer_refs);

        Self { rf_state, buffer }
    }
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
    /// scattering each lane's ring buffer and roofing-filter state back to its corresponding asset.
    pub fn write_states(&self, states: &mut [&mut State]) {
        let mut rf_refs = Vec::with_capacity(N);
        let buffers = self.buffer.to_f64_buffers();

        // Collect references and values
        for (buffer, state) in buffers.into_iter().zip(states.iter_mut()) {
            state.buffer = buffer;
            rf_refs.push(&mut state.rf_state);
        }
        self.rf_state.write_states(&mut rf_refs);
    }
    /// Applies the seven-tap Hilbert kernel across all `N` assets simultaneously.
    ///
    /// Pushes `real` (the roofing-filter output) into the ring buffer, then computes:
    /// * **Q** (quadrature) — two independent FMAs on the even-offset taps
    ///   (`buf[0]`, `buf[2]`, `buf[4]`, `buf[6]`) with coefficients `C0`–`C3`.
    /// * **I** (in-phase) — the center tap `buf[3]` (a 3-bar-ago sample).
    ///
    /// # Returns
    ///
    /// `(in_phase, quadrature)` as a pair of `Simd<f64, N>` vectors, one value per asset lane.
    #[inline(always)]
    pub fn calc_transform_simd(&mut self, real: Simd<f64, N>) -> (Simd<f64, N>, Simd<f64, N>) {
        self.buffer.push(real);
        let q_hi = COEFS::C0.mul_add(self.buffer[0], COEFS::C1 * self.buffer[2]); // 0.0962*x[t]   + 0.5769*x[t-2]
        let q_lo = COEFS::C2.mul_add(self.buffer[4], COEFS::C3 * self.buffer[6]); // -0.5769*x[t-4] + -0.0962*x[t-6]
        let q = q_hi + q_lo;
        let i = self.buffer[3];
        (i, q)
    }
    /// Unsafe variant of [`calc_transform`](Self::calc_transform) that skips the
    /// ring-buffer fullness check on push.
    ///
    /// # Safety
    ///
    /// The caller must ensure the buffer is full (`buffer.len() == 7`) before calling.
    #[inline(always)]
    pub unsafe fn calc_transform_simd_unchecked(
        &mut self,
        real: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        self.buffer.push_unchecked(real);
        let q_hi = COEFS::C0.mul_add(self.buffer[0], COEFS::C1 * self.buffer[2]); // 0.0962*x[t]   + 0.5769*x[t-2]
        let q_lo = COEFS::C2.mul_add(self.buffer[4], COEFS::C3 * self.buffer[6]); // -0.5769*x[t-4] + -0.0962*x[t-6]
        let q = q_hi + q_lo;
        let i = self.buffer[3];
        (i, q)
    }
    /// Advances the full Hilbert Transform pipeline by one bar across all `N` assets simultaneously.
    ///
    /// Applies the roofing filter (HighPass → SuperSmoother) to `real`, then passes
    /// the band-limited result through the Hilbert kernel, matching the scalar `State::calc` logic.
    ///
    /// # Arguments
    ///
    /// * `real` - SIMD vector of current input prices, one per asset lane.
    /// * `multipliers` - Pre-broadcast roofing-filter coefficients `((ss_a1, ss_a2, ss_b0), (hp_a1, hp_a2))`.
    ///
    /// # Returns
    ///
    /// `(in_phase, quadrature, roofing, highpass)` — four `Simd<f64, N>` vectors, one value per asset lane.
    #[inline(always)]
    pub fn calc_simd(
        &mut self,
        real: Simd<f64, N>,
        multipliers: (
            (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
            (Simd<f64, N>, Simd<f64, N>),
        ),
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let (rf, hp) = self.rf_state.calc_simd(real, multipliers);
        let (i, q) = self.calc_transform_simd(rf);
        (i, q, rf, hp)
    }
    /// Unsafe variant of [`calc`](Self::calc) that skips the ring-buffer fullness check.
    ///
    /// # Safety
    ///
    /// The caller must ensure the buffer is full (`buffer.len() == 7`) before calling.
    #[inline(always)]
    pub unsafe fn calc_simd_unchecked(
        &mut self,
        real: Simd<f64, N>,
        multipliers: (
            (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>),
            (Simd<f64, N>, Simd<f64, N>),
        ),
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let (rf, hp) = self.rf_state.calc_simd(real, multipliers);
        let (i, q) = self.calc_transform_simd_unchecked(rf);
        (i, q, rf, hp)
    }
}
