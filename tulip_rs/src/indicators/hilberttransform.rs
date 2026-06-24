//! # Ehlers Hilbert Transform (with Roofing Filter)
//!
//! **Source:** John Ehlers, *Cycle Analytics for Traders* (2013), Chapter 3.
//!
//! Computes the In-Phase (I) and Quadrature (Q) phasor components of the
//! dominant market cycle by applying a 7-tap discrete Hilbert Transform kernel
//! to the Roofing Filter output. The Roofing Filter (High Pass → Super Smoother)
//! band-limits the signal before the kernel runs, eliminating trend and noise
//! that would otherwise corrupt the phase estimate.
//!
//! ## Pipeline
//!
//! ```text
//! Price → Roofing Filter (hp_period, ss_period) → 7-tap Hilbert kernel → (I, Q)
//! ```
//!
//! ## 7-tap Hilbert kernel
//!
//! Applied with unit gain (`gain = 1.0`) to the roofed signal `x`:
//!
//! ```text
//! Q = (0.0962·x[0] + 0.5769·x[2] − 0.5769·x[4] − 0.0962·x[6])
//! I = x[3]   (the 3-bar-delayed centre tap — the 90° complement of Q)
//! ```

use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::roofingfilter::multiplier;
use crate::indicators::{
    highpass::output_length as hp_output_length,
    roofingfilter::{min_data as rf_min_data, output_length as rf_output_length, State as RfSate},
};
use crate::ring_buffer::fixed_single_buffer::FixedRingBuffer;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

pub(crate) const C0: f64 = 0.0962;
pub(crate) const C1: f64 = 0.5769;
pub(crate) const C2: f64 = -0.5769;
pub(crate) const C3: f64 = -0.0962;

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
pub(crate) fn ht_kernel(buf: &FixedRingBuffer<f64, 7>, gain: f64) -> (f64, f64) {
    let q = (C0.mul_add(buf[0], C1 * buf[2]) + C2.mul_add(buf[4], C3 * buf[6])) * gain;
    (buf[3], q)
}

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::hilberttransform_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::hilberttransform_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::hilberttransform_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::hilberttransform_simd::indicator_by_options as indicator;
}

/// Metadata describing the Hilbert Transform indicator.
pub const INFO: Info = Info {
    name: "hilberttransform",
    indicator_type: IndicatorType::Math,
    full_name: "Hilbert Transform",
    inputs: &["real"],
    options: &["ss_period", "hp_period"],
    outputs: &["in_phase", "quadrature"],
    optional_outputs: &["roofing", "highpass"],
    display_groups: &[DisplayGroup {
        offset: None,
        id: "hilberttransform",
        label: "Hilbert Transform",
        display_type: DisplayType::Indicator,
        outputs: &["in_phase", "quadrature", "roofing", "highpass"],
    }],
};
#[derive(Serialize, Deserialize)]
pub struct State {
    pub buffer: FixedRingBuffer<f64, 7>,
    pub rf_state: RfSate,
}
impl State {
    pub fn init_state(
        real: &[f64],
        periods: (usize, usize),
        multipliers: ((f64, f64, f64), (f64, f64)),
        optional_outputs: (&mut [f64], &mut [f64]),
    ) -> State {
        let (rf_line, hp_line) = optional_outputs;
        let mut buffer = FixedRingBuffer::new();
        let mut rf_state = RfSate::init_state(real, periods, multipliers, hp_line);
        let mut i = periods.0.max(periods.1);

        while buffer.len() < buffer.capacity() {
            let (rf, hp) = rf_state.calc(real[i], multipliers);
            buffer.push(rf);
            crate::init_store_optional_outputs!(i, real.len(),
                rf_line => rf,
                hp_line => hp
            );
            i += 1;
        }

        Self { buffer, rf_state }
    }
    #[inline(always)]
    pub fn calc_transform(&mut self, real: f64) -> (f64, f64) {
        self.buffer.push(real);
        ht_kernel(&self.buffer, 1.0)
    }
    #[inline(always)]
    pub unsafe fn calc_transform_unchecked(&mut self, real: f64) -> (f64, f64) {
        self.buffer.push_unchecked(real);
        ht_kernel(&self.buffer, 1.0)
    }
    #[inline(always)]
    pub fn calc(
        &mut self,
        real: f64,
        multipliers: ((f64, f64, f64), (f64, f64)),
    ) -> (f64, f64, f64, f64) {
        let (rf, hp) = self.rf_state.calc(real, multipliers);
        let (i, q) = self.calc_transform(rf);
        (i, q, rf, hp)
    }
    #[inline(always)]
    pub unsafe fn calc_unchecked(
        &mut self,
        real: f64,
        multipliers: ((f64, f64, f64), (f64, f64)),
    ) -> (f64, f64, f64, f64) {
        let (rf, hp) = self.rf_state.calc(real, multipliers);
        let (i, q) = self.calc_transform_unchecked(rf);
        (i, q, rf, hp)
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    multipliers: ((f64, f64, f64), (f64, f64)),
}
impl IndicatorState {
    pub fn new(state: State, multipliers: ((f64, f64, f64), (f64, f64))) -> Self {
        Self { state, multipliers }
    }
}
impl TIndicatorState<INPUTS_WIDTH> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut p_line, mut q_line, (mut rf_line, mut hp_line)) = {
            let len = inputs[0].len();
            (
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    rf_line: len,
                    hp_line: len
                ),
            )
        };

        cycle(
            inputs[0],
            &mut self.state,
            self.multipliers,
            &mut p_line,
            &mut q_line,
            (&mut rf_line, &mut hp_line),
        );

        Ok(vec![p_line, q_line, rf_line, hp_line])
    }
}

/// Returns the minimum number of input bars required for the Hilbert Transform.
///
/// Equals `rf_min_data(options) + 7` — the roofing filter warm-up plus the
/// seven-tap Hilbert kernel window.
///
/// # Arguments
///
/// * `options` - `[ss_period, hp_period]`.
pub fn min_data(options: &[f64]) -> usize {
    rf_min_data(options) + 7
}

/// Returns the number of output values given an input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - `[ss_period, hp_period]`.
///
/// # Returns
///
/// `data_len - min_data(options) + 1`.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Hilbert Transform (in-phase and quadrature) over the full input dataset.
///
/// The input is first passed through a roofing filter (HighPass → SuperSmoother) to
/// isolate the dominant cycle band, then the seven-tap Hilbert kernel is applied to
/// produce the in-phase (`I`) and quadrature (`Q`) components.
///
/// # Inputs
///
/// * `inputs[0]` — real (price) series
///
/// # Options
///
/// * `options[0]` — `ss_period`: SuperSmoother period (low-cut for the roofing filter)
/// * `options[1]` — `hp_period`: HighPass period (high-cut for the roofing filter)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - `[ss_period, hp_period]` (see Options above).
/// * `optional_outputs` - Pass `Some(&[true, false])` to enable the roofing-filter
///   output, `Some(&[false, true])` for the high-pass output, `Some(&[true, true])`
///   for both, or `None` to disable all optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `in_phase`, `outputs[1]` is
/// `quadrature`, `outputs[2]` is `roofing` (empty unless requested), and
/// `outputs[3]` is `highpass` (empty unless requested).
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let periods = (options[0] as usize, options[1] as usize);
    let multipliers = multiplier(periods);

    validate_inputs(inputs, min_data(options))?;
    let (mut p_line, mut q_line, (mut rf_line, mut hp_line)) = {
        let len = inputs[0].len();
        let capacity = output_length(len, options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                rf_line: rf_output_length(len, options),
                hp_line: hp_output_length(len, &[periods.1 as f64])
            ),
        )
    };

    let mut state = State::init_state(
        inputs[0],
        periods,
        multipliers,
        (&mut rf_line, &mut hp_line),
    );
    let optional_outputs = {
        let offset = crate::slice_outputs_start!(p_line.len(), rf_line, hp_line);
        (&mut rf_line[offset.0..], &mut hp_line[offset.1..])
    };
    let real = {
        let from = periods.0.max(periods.1) + 7;
        &inputs[0][from..]
    };

    cycle(
        real,
        &mut state,
        multipliers,
        &mut p_line,
        &mut q_line,
        optional_outputs,
    );

    Ok((
        vec![p_line, q_line, rf_line, hp_line],
        IndicatorState::new(state, multipliers),
    ))
}

/// Performs the main calculation loop for the Hilbert Transform.
///
/// # Arguments
///
/// * `real` - Input price slice (already offset to the first computable bar).
/// * `state` - Mutable indicator state (roofing filter + Hilbert ring buffer).
/// * `multipliers` - Pre-computed roofing-filter coefficients.
/// * `p_line` - Output slice for the in-phase (`I`) values.
/// * `q_line` - Output slice for the quadrature (`Q`) values.
/// * `optional_outputs` - `(rf_line, hp_line)` slices for optional roofing / highpass outputs.
fn cycle(
    real: &[f64],
    state: &mut State,
    multipliers: ((f64, f64, f64), (f64, f64)),
    p_line: &mut [f64],
    q_line: &mut [f64],
    optional_outputs: (&mut [f64], &mut [f64]),
) {
    let (rf_line, hp_line) = optional_outputs;
    let (has_optional, want_rf, want_hp) = crate::calc_want_flags!(rf_line, hp_line);

    //high.iter().zip(low.iter()).zip(close.iter()).skip(start).enumerate().for_each(|(i, ((h, l), c))| {
    for i in 0..real.len() {
        let (p, q, rf, hp) = unsafe { state.calc_unchecked(*real.get_unchecked(i), multipliers) };

        unsafe {
            *p_line.get_unchecked_mut(i) = p;
            *q_line.get_unchecked_mut(i) = q;
        };
        if has_optional {
            crate::store_optional_outputs!(i,
                want_rf, rf_line => rf,
                want_hp, hp_line => hp
            );
        }
    }
}
