use crate::indicators::hilberttransform::{IndicatorState as State, C0, C1, C2, C3};
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::hilberttransform::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::hilberttransform::indicator_by_options;

use crate::indicators::simd_indicators::roofingfilter_simd::SimdState as RfSimdState;
use crate::ring_buffer::fixed_single_buffer::{FixedRingBuffer, FixedSimdRingBuffer};
use std::simd::{Simd, StdFloat};
pub use crate::indicator_types::{TSimdState, TState};
use crate::types::Warm;
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
    pub const GAIN: Simd<f64, N> = Simd::splat(1.0);
}
/// Applies the 7-tap Hilbert kernel to a full ring buffer with an optional gain.
///
/// Returns `(center_tap, hilbert_sum × gain)` — i.e. `(I, Q)` semantics:
/// * `I` = `buf[3]` — the 3-bar-ago in-phase tap (not scaled by gain).
/// * `Q` = `(0.0962·buf[0] + 0.5769·buf[2] − 0.5769·buf[4] − 0.0962·buf[6]) × gain`.
///
/// Pass `gain = 1.0` for a plain (non-adaptive) transform; the compiler folds
/// the `× 1.0` away. Pass `0.075 · Period[1] + 0.54` for the Ehlers adaptive
/// variant used by the Homodyne Discriminator.
///
/// The buffer must be full (`buf.is_full() == true`) before calling.
#[inline(always)]
pub fn ht_kernel<S, const N: usize>(
    buf: &FixedRingBuffer<Simd<f64, N>, 7, S>,
    gain: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>) {
    let q = ht_kernel_base(buf, gain);
    (buf[3], q)
}
#[inline(always)]
pub fn ht_kernel_base<S, const N: usize>(
    buf: &FixedRingBuffer<Simd<f64, N>, 7, S>,
    gain: Simd<f64, N>,
) -> Simd<f64, N> {
    let q = (COEFS::<N>::C0.mul_add(buf[0], COEFS::<N>::C1 * buf[2])
        + COEFS::<N>::C2.mul_add(buf[4], COEFS::<N>::C3 * buf[6]))
        * gain;
    q
}

/// Applies the 7-tap Hilbert kernel to two independent full SIMD ring buffers simultaneously.
///
/// SIMD equivalent of the scalar [`ht_kernel_pair`](crate::indicators::hilberttransform::ht_kernel_pair).
/// Interleaves the four FMAs across `buf_a` and `buf_b` so the CPU sees them as
/// independent operations across both the SIMD width and the two buffers.
///
/// Returns `(I_a, Q_a, I_b, Q_b)` where `I = buf[3]` and `Q = hilbert_sum × gain`.
/// Both buffers must be full before calling.
#[inline(always)]
pub fn ht_kernel_pair<S, const N: usize>(
    buf_a: &FixedRingBuffer<Simd<f64, N>, 7, S>,
    buf_b: &FixedRingBuffer<Simd<f64, N>, 7, S>,
    gain: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
    let qa_hi = COEFS::<N>::C0.mul_add(buf_a[0], COEFS::<N>::C1 * buf_a[2]);
    let qb_hi = COEFS::<N>::C0.mul_add(buf_b[0], COEFS::<N>::C1 * buf_b[2]);
    let qa_lo = COEFS::<N>::C2.mul_add(buf_a[4], COEFS::<N>::C3 * buf_a[6]);
    let qb_lo = COEFS::<N>::C2.mul_add(buf_b[4], COEFS::<N>::C3 * buf_b[6]);
    (
        buf_a[3],
        (qa_hi + qa_lo) * gain,
        buf_b[3],
        (qb_hi + qb_lo) * gain,
    )
}
/// SIMD-parallel state for computing the Hilbert Transform across `N` assets simultaneously.
///
/// Holds two pieces of state:
/// * `buffer` — seven-slot ring buffer of SIMD vectors fed by the roofing filter output;
///   used to compute the in-phase (`I`) and quadrature (`Q`) components.
/// * `rf_state` — cascaded roofing filter (HighPass → SuperSmoother) that band-limits
///   the input before the Hilbert kernel.
pub struct SimdState<const N: usize> {
    rf_state: RfSimdState<N>,
    buffer: FixedRingBuffer<Simd<f64, N>, 7, Warm>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;
    
    /// Gathers `N` scalar [`State`] references into a single [`SimdState`],
    /// packing each asset's ring buffer and roofing-filter state into their
    /// respective SIMD lanes.
    fn from_states(states: &mut [&mut State]) -> Self {
        let mut buffer_refs = Vec::with_capacity(N);
        let mut rf_state = Vec::with_capacity(N);

        for state in states.iter_mut() {
            buffer_refs.push(&state.buffer);
            rf_state.push(&mut state.rf_state);
        }
        let rf_state = RfSimdState::from_states(&mut rf_state);
        let buffer = FixedSimdRingBuffer::from_f64_buffers(&buffer_refs);

        Self { rf_state, buffer }
    }
    /// Writes the SIMD state back into `N` existing mutable scalar [`State`] references in place,
    /// scattering each lane's ring buffer and roofing-filter state back to its corresponding asset.
    fn write_states(&self, states: &mut [&mut State]) {
        let mut rf_refs = Vec::with_capacity(N);
        let buffers = self.buffer.to_f64_buffers();

        // Collect references and values
        for (buffer, state) in buffers.into_iter().zip(states.iter_mut()) {
            state.buffer = buffer;
            rf_refs.push(&mut state.rf_state);
        }
        self.rf_state.write_states(&mut rf_refs);
    }

}
impl<const N: usize> SimdState<N> {
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
        ht_kernel(&self.buffer, COEFS::GAIN)
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
        self.buffer.push(real);
        ht_kernel(&self.buffer, COEFS::<N>::GAIN)
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
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        let (rf, hp) = self.rf_state.calc(real);
        let (i, q) = self.calc_transform_simd_unchecked(rf);
        (i, q, rf, hp)
    }
}

impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = Simd<f64, N>;
    type Outputs = (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>);
    #[inline(always)]
    fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
        unsafe { self.calc_simd_unchecked(real) }
    }
}