pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::homodynediscriminator::IndicatorState as State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::homodynediscriminator::indicator_by_assets;

use crate::indicators::simd_indicators::hilberttransform_simd::{ht_kernel, ht_kernel_pair};
use crate::math_simd::trig::simd_atan;
use crate::ring_buffer::fixed_single_buffer::{FixedRingBuffer, FixedSimdRingBuffer};
use std::simd::{cmp::SimdPartialEq, num::SimdFloat, Select, Simd, StdFloat};
use crate::types::Warm;
/// SIMD-parallel state for the Ehlers Homodyne Discriminator across `N` assets simultaneously.
///
/// Mirrors [`State`] but packs `N` independent assets into each SIMD vector,
/// enabling the four-stage HT cascade and homodyne discriminator to be computed
/// for all assets in a single pass through the ring buffers.
///
/// Stage 3 uses two separate `i1_buf` / `q1_buf` SIMD ring buffers (one per component)
/// rather than the scalar state's `iq1_buf: FixedRingBuffer<Simd<f64, 2>, 7>` trick,
/// because each slot already carries `N` lanes of asset data and a second SIMD
/// dimension would not reduce the number of memory loads.
///
/// Uses [`ht_kernel_pair`] to compute jI and jQ for all `N` assets in a single
/// interleaved FMA pass across both buffers, and [`simd_atan`] for the per-lane
/// `atan(Im/Re)` computation in [`apply_discriminator_simd`].
pub struct SimdState<const N: usize> {
    // Stage 0: 4-bar Hann-weighted smooth
    pub price_buf: FixedRingBuffer<Simd<f64, N>, 4, Warm>,
    // Stage 1: smooth → Detrender
    smooth_buf: FixedRingBuffer<Simd<f64, N>, 7, Warm>,
    // Stage 2: Detrender → I1, Q1
    detrender_buf: FixedRingBuffer<Simd<f64, N>, 7, Warm>,
    // Stage 3: separate I1 and Q1 histories for the 90° phase-advance kernel
    i1_buf: FixedRingBuffer<Simd<f64, N>, 7, Warm>,
    q1_buf: FixedRingBuffer<Simd<f64, N>, 7, Warm>,
    // IIR state for the homodyne discriminator
    i2_prev: Simd<f64, N>,
    q2_prev: Simd<f64, N>,
    re_prev: Simd<f64, N>,
    im_prev: Simd<f64, N>,
    // Period tracking
    period: Simd<f64, N>,
    pub(crate) smooth_period: Simd<f64, N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;

    /// Gathers `N` scalar [`State`] references into a single [`SimdState`],
    /// packing each asset's ring buffers and IIR scalars into SIMD lanes.
    ///
    /// The scalar `iq1_buf` (which packs i1/q1 into a `Simd<f64, 2>` per slot) is
    /// split back into two separate per-asset `FixedRingBuffer<f64, 7>` instances,
    /// then merged across assets into `i1_buf` and `q1_buf` via
    /// [`FixedSimdRingBuffer::from_f64_buffers`] (which normalises the ring head to `index = 0`).
    fn from_states(states: &mut [&mut State]) -> Self {
        // Each iq1_buf slot is Simd<f64, 2>: lane 0 = i1, lane 1 = q1.
        // Extract them into two separate scalar ring buffers per asset.
        let i1_scalar: [FixedRingBuffer<f64, 7, Warm>; N] = std::array::from_fn(|j| FixedRingBuffer {
            vals: std::array::from_fn(|k| states[j].iq1_buf.vals[k].to_array()[0]),
            index: states[j].iq1_buf.index,
            count: states[j].iq1_buf.count,
            state: std::marker::PhantomData::<Warm>
        });
        let q1_scalar: [FixedRingBuffer<f64, 7, Warm>; N] = std::array::from_fn(|j| FixedRingBuffer {
            vals: std::array::from_fn(|k| states[j].iq1_buf.vals[k].to_array()[1]),
            index: states[j].iq1_buf.index,
            count: states[j].iq1_buf.count,
            state: std::marker::PhantomData::<Warm>
        });

        let price_refs: [&FixedRingBuffer<f64, 4, Warm>; N] =
            std::array::from_fn(|j| &states[j].price_buf);
        let smooth_refs: [&FixedRingBuffer<f64, 7, Warm>; N] =
            std::array::from_fn(|j| &states[j].smooth_buf);
        let detrender_refs: [&FixedRingBuffer<f64, 7, Warm>; N] =
            std::array::from_fn(|j| &states[j].detrender_buf);
        let i1_refs: [&FixedRingBuffer<f64, 7, Warm>; N] = std::array::from_fn(|j| &i1_scalar[j]);
        let q1_refs: [&FixedRingBuffer<f64, 7, Warm>; N] = std::array::from_fn(|j| &q1_scalar[j]);

        Self {
            price_buf: FixedSimdRingBuffer::from_f64_buffers(&price_refs),
            smooth_buf: FixedSimdRingBuffer::from_f64_buffers(&smooth_refs),
            detrender_buf: FixedSimdRingBuffer::from_f64_buffers(&detrender_refs),
            i1_buf: FixedSimdRingBuffer::from_f64_buffers(&i1_refs),
            q1_buf: FixedSimdRingBuffer::from_f64_buffers(&q1_refs),
            i2_prev: Simd::from_array(std::array::from_fn(|j| states[j].i2_prev)),
            q2_prev: Simd::from_array(std::array::from_fn(|j| states[j].q2_prev)),
            re_prev: Simd::from_array(std::array::from_fn(|j| states[j].re_prev)),
            im_prev: Simd::from_array(std::array::from_fn(|j| states[j].im_prev)),
            period: Simd::from_array(std::array::from_fn(|j| states[j].period)),
            smooth_period: Simd::from_array(std::array::from_fn(|j| states[j].smooth_period)),
        }
    }

    /// Writes the SIMD state back into `N` scalar [`State`] references,
    /// scattering each lane's ring buffers and IIR scalars to the corresponding asset.
    ///
    /// The SIMD `i1_buf`/`q1_buf` are scattered to per-asset `FixedRingBuffer<f64, 7>`
    /// instances via [`FixedRingBuffer::to_f64_buffers`], then repacked into each
    /// scalar state's `iq1_buf: FixedRingBuffer<Simd<f64, 2>, 7>` (lane 0 = i1, lane 1 = q1).
    fn write_states(&self, states: &mut [&mut State]) {
        let price_bufs = self.price_buf.to_f64_buffers();
        let smooth_bufs = self.smooth_buf.to_f64_buffers();
        let detrender_bufs = self.detrender_buf.to_f64_buffers();
        let i1_bufs = self.i1_buf.to_f64_buffers();
        let q1_bufs = self.q1_buf.to_f64_buffers();
        let i2_arr = self.i2_prev.to_array();
        let q2_arr = self.q2_prev.to_array();
        let re_arr = self.re_prev.to_array();
        let im_arr = self.im_prev.to_array();
        let period_arr = self.period.to_array();
        let sp_arr = self.smooth_period.to_array();

        for (j, state) in states.iter_mut().enumerate() {
            state.price_buf = price_bufs[j].clone();
            state.smooth_buf = smooth_bufs[j].clone();
            state.detrender_buf = detrender_bufs[j].clone();
            // Repack i1 and q1 back into iq1_buf (lane 0 = i1, lane 1 = q1)
            state.iq1_buf = FixedRingBuffer {
                vals: std::array::from_fn(|k| {
                    Simd::from_array([i1_bufs[j].vals[k], q1_bufs[j].vals[k]])
                }),
                index: i1_bufs[j].index,
                count: i1_bufs[j].count,
                state: std::marker::PhantomData::<Warm>
            };
            state.i2_prev = i2_arr[j];
            state.q2_prev = q2_arr[j];
            state.re_prev = re_arr[j];
            state.im_prev = im_arr[j];
            state.period = period_arr[j];
            state.smooth_period = sp_arr[j];
        }
    }
}

impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = Simd<f64, N>;
    type Outputs = Simd<f64, N>;

    /// Safe single-bar update — delegates to `calc_simd_unchecked`.
    ///
    /// Only call once all ring buffers are full (guaranteed by the driver
    /// infrastructure after [`TSimdState::from_states`]).
    #[inline(always)]
    fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
        self.price_buf.push(real);
        let ab = Simd::splat(4.0).mul_add(self.price_buf[0], Simd::splat(3.0) * self.price_buf[1]);
        let cd = Simd::splat(2.0).mul_add(self.price_buf[2], self.price_buf[3]);
        let smooth = (ab + cd) * Simd::splat(0.1);

        // ── Adaptive gain from previous period estimate ───────────────────────
        let gain = Simd::splat(0.075).mul_add(self.period, Simd::splat(0.54));

        // ── Stage 1: Detrender ────────────────────────────────────────────────
        self.smooth_buf.push(smooth);
        let (_, detrender) = ht_kernel(&self.smooth_buf, gain);

        // ── Stage 2: I1, Q1 ──────────────────────────────────────────────────
        self.detrender_buf.push(detrender);
        let (i1, q1) = ht_kernel(&self.detrender_buf, gain);

        // ── Stage 3: jI and jQ via interleaved FMAs across both buffers ───────
        // ht_kernel_pair returns (I_a, Q_a, I_b, Q_b); we need Q_a = jI, Q_b = jQ.
        self.i1_buf.push(i1);
        self.q1_buf.push(q1);
        let (_, j_i, _, j_q) = ht_kernel_pair(&self.i1_buf, &self.q1_buf, gain);

        self.apply_discriminator_simd(i1, q1, j_i, j_q)
    }
}

impl<const N: usize> SimdState<N> {
    /// One-bar update returning `(smooth_period, i1, q1)` for all `N` lanes.
    ///
    /// Identical to [`calc_simd_unchecked`](Self::calc_simd_unchecked) but also returns the
    /// intermediate I1 / Q1 values from stage 2, allowing callers (e.g. MAMA) to compute
    /// per-lane phase and adaptive alpha without re-running the pipeline.
    ///
    /// # Safety
    ///
    /// All five ring buffers must be full on entry.
    #[inline(always)]
    pub fn calc_with_iq(
        &mut self,
        real: Simd<f64, N>,
    ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
        self.price_buf.push(real);
        let ab = Simd::splat(4.0).mul_add(self.price_buf[0], Simd::splat(3.0) * self.price_buf[1]);
        let cd = Simd::splat(2.0).mul_add(self.price_buf[2], self.price_buf[3]);
        let smooth = (ab + cd) * Simd::splat(0.1);

        let gain = Simd::splat(0.075).mul_add(self.period, Simd::splat(0.54));

        self.smooth_buf.push(smooth);
        let (_, detrender) = ht_kernel(&self.smooth_buf, gain);

        self.detrender_buf.push(detrender);
        let (i1, q1) = ht_kernel(&self.detrender_buf, gain);

        self.i1_buf.push(i1);
        self.q1_buf.push(q1);
        let (_, j_i, _, j_q) = ht_kernel_pair(&self.i1_buf, &self.q1_buf, gain);

        let smooth_period = self.apply_discriminator_simd(i1, q1, j_i, j_q);
        (smooth_period, i1, q1)
    }

    /// Applies the homodyne discriminator and period-smoothing logic for all `N` lanes.
    ///
    /// Mirrors [`State::apply_discriminator`] exactly, with scalar operations replaced
    /// by SIMD equivalents:
    /// - `f64::atan` → `simd_atan`
    /// - `if im != 0.0 && re != 0.0` → `(im.simd_ne(zero) & re.simd_ne(zero)).select(...)`
    /// - `f64::min/max/clamp` → `Simd::simd_min/simd_max`
    /// - `f64::mul_add` → `Simd::mul_add` (via `StdFloat`)
    #[inline(always)]
    fn apply_discriminator_simd(
        &mut self,
        i1: Simd<f64, N>,
        q1: Simd<f64, N>,
        j_i: Simd<f64, N>,
        j_q: Simd<f64, N>,
    ) -> Simd<f64, N> {
        let zero = Simd::splat(0.0_f64);
        let pt2 = Simd::splat(0.2_f64);
        let pt8 = Simd::splat(0.8_f64);

        // ── Phasor rotation (adds 90° to disambiguate quadrant) ───────────────
        let i2_raw = i1 - j_q;
        let q2_raw = q1 + j_i;

        // ── IIR smooth I2, Q2 (α = 0.2) ──────────────────────────────────────
        let i2 = pt2.mul_add(i2_raw, pt8 * self.i2_prev);
        let q2 = pt2.mul_add(q2_raw, pt8 * self.q2_prev);

        // ── Homodyne discriminator: dot / cross with 1-bar-delayed conjugate ──
        // Re = I2·I2[1] + Q2·Q2[1]   (dot product → cosine of phase difference)
        // Im = I2·Q2[1] − Q2·I2[1]   (cross product → sine of phase difference)
        let re_raw = i2.mul_add(self.i2_prev, q2 * self.q2_prev);
        let im_raw = i2.mul_add(self.q2_prev, -(q2 * self.i2_prev));

        self.i2_prev = i2;
        self.q2_prev = q2;

        // ── IIR smooth Re, Im (α = 0.2) ───────────────────────────────────────
        let re = pt2.mul_add(re_raw, pt8 * self.re_prev);
        let im = pt2.mul_add(im_raw, pt8 * self.im_prev);

        self.re_prev = re;
        self.im_prev = im;

        // ── Period from instantaneous phase-change per bar ────────────────────
        // 2π / atan(Im/Re). Fall back to previous period when Im or Re is zero.
        // simd_atan is safe for ±inf (returns ±π/2); the select discards that
        // result for any lane where either Im or Re is zero.
        let both_nonzero = im.simd_ne(zero) & re.simd_ne(zero);
        let period_from_atan = Simd::splat(std::f64::consts::TAU) / simd_atan(im / re);
        let mut period = both_nonzero.select(period_from_atan, self.period);

        // ── Rate-of-change clamp: limit to ±50% change per bar ───────────────
        period = period
            .simd_min(Simd::splat(1.5) * self.period)
            .simd_max(Simd::splat(0.67) * self.period);

        // ── Absolute clamp [6, 50] bars ───────────────────────────────────────
        period = period
            .simd_max(Simd::splat(6.0))
            .simd_min(Simd::splat(50.0));

        // ── Final smoothing (two IIR passes) ─────────────────────────────────
        period = Simd::splat(0.2).mul_add(period, pt8 * self.period);
        self.smooth_period =
            Simd::splat(0.33).mul_add(period, Simd::splat(0.67) * self.smooth_period);
        self.period = period;

        self.smooth_period
    }
}
