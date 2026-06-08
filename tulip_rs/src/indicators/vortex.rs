use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::tr::{calc as calc_tr, output_length as tr_output_length};
use crate::ring_buffer::multi_type_buffer::MultiTypeBuffer;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
//use wide::*;
use std::simd::{num::SimdFloat, Simd};
/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::vortex_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::vortex_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::vortex_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::vortex_simd::indicator_by_options as indicator;
}
/// Returns information about the Vortex indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the Vortex indicator.
pub const INFO: Info = Info {
    name: "vortex",
    full_name: "Vortex",
    indicator_type: IndicatorType::Trend,
    // Inputs are expected to be: high, low, close
    inputs: &["high", "low", "close"],
    // Options: period
    options: &["period"],
    outputs: &["vi_up", "vi_down"],
    optional_outputs: &["tr"],
    display_groups: &[
        DisplayGroup {
            id: "vortex",
            label: "Vortex",
            display_type: DisplayType::Indicator,
            outputs: &["vi_up", "vi_down"],
        },
        DisplayGroup {
            id: "tr",
            label: "True Range",
            display_type: DisplayType::Indicator,
            outputs: &["tr"],
        },
    ],
};

impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let [high, low, close] = inputs;
        let (mut vi_up_line, mut vi_down_line, mut tr_line) = {
            let len = high.len();
            (
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    tr_line_line: len
                ),
            )
        };

        cycle(
            (high, low, close),
            self,
            (&mut vi_up_line, &mut vi_down_line),
            &mut tr_line,
        );

        Ok(vec![vi_up_line, vi_down_line, tr_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub buffer: MultiTypeBuffer<(f64, Simd<f64, 2>)>,

    #[serde(
        serialize_with = "serialize_f64x2",
        deserialize_with = "deserialize_f64x2"
    )]
    pub vm_sums: Simd<f64, 2>,
    #[serde(
        serialize_with = "serialize_f64x2",
        deserialize_with = "deserialize_f64x2"
    )]
    pub prev_low_high: Simd<f64, 2>,

    pub prev_close: f64,
    pub tr_sum: f64,
    /*pub vm_up_sum: f64,
    pub vm_down_sum: f64,*/
}
fn serialize_f64x2<S>(data: &Simd<f64, 2>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    data.to_array().serialize(serializer)
}

fn deserialize_f64x2<'de, D>(deserializer: D) -> Result<Simd<f64, 2>, D::Error>
where
    D: Deserializer<'de>,
{
    let array = <[f64; 2]>::deserialize(deserializer)?;
    Ok(Simd::from_array(array))
}
impl IndicatorState {
    pub fn new(period: usize, prev_high: f64, prev_low: f64, prev_close: f64) -> Self {
        Self {
            buffer: MultiTypeBuffer::new(period),
            prev_low_high: Simd::from_array([prev_low, prev_high]),
            prev_close,
            tr_sum: 0.0,
            vm_sums: Simd::splat(0.0),
            /*vm_up_sum: 0.0,
            vm_down_sum: 0.0,*/
        }
    }
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        tr_line: &mut [f64],
    ) -> Self {
        let mut state = Self::new(period, high[0], low[0], close[0]);
        let mut i = 1;

        while !state.buffer.is_full() {
            let (_, _, tr) = state.calc(high[i], low[i], close[i]);
            if tr_line.len() > 0 {
                tr_line[i-1] = tr;
            }
            i += 1;
        }

        state
    }

    #[inline(always)]
    pub fn calc(&mut self, high: f64, low: f64, close: f64) -> (f64, f64, f64) {
        let tr = calc_tr(high, low, self.prev_close);
        let high_low_simd = Simd::from_array([high, low]);
        /*let vm_up = (high - self.prev_low).abs();
        let vm_down = (low - self.prev_high).abs();*/
        let vm_simd = (high_low_simd - self.prev_low_high).abs();

        self.prev_close = close;
        self.prev_low_high = high_low_simd.reverse();

        if let Some((old_tr, old_vm)) = self.buffer.push_with_info((tr, vm_simd)) {
            self.tr_sum += tr - old_tr;
            self.vm_sums += vm_simd - old_vm;
            let tr_sum_simd = Simd::splat(self.tr_sum);
            /*self.vm_up_sum += vm_up - old_vm_up;
            self.vm_down_sum += vm_down - old_vm_down;*/
            let vi_simd = self.vm_sums / tr_sum_simd;
            return (vi_simd[0], vi_simd[1], tr);
        }

        self.tr_sum += tr;
        self.vm_sums += vm_simd;
        /*self.vm_up_sum += vm_up;
        self.vm_down_sum += vm_down;*/
        (0.0, 0.0, tr)
    }
    #[inline(always)]
    pub unsafe fn calc_unchecked(&mut self, high: f64, low: f64, close: f64) -> (f64, f64, f64) {
        let tr = calc_tr(high, low, self.prev_close);
        let high_low_simd = Simd::from_array([high, low]);
        let vm_simd = (high_low_simd - self.prev_low_high).abs();

        self.prev_close = close;
        self.prev_low_high = high_low_simd.reverse();

        let (old_tr, old_vm) = self.buffer.push_with_info_unchecked((tr, vm_simd));
        self.tr_sum += tr - old_tr;
        let tr_sum_simd = Simd::splat(self.tr_sum);

        self.vm_sums += vm_simd - old_vm;
        let vi_simd = self.vm_sums / tr_sum_simd;

        (vi_simd[0], vi_simd[1], tr)
    }
}
/// Returns the minimum number of input bars required to produce accurate results.
///
/// For the Vortex indicator accuracy does not depend on decimal precision, so
/// this always returns the same value as [`min_data`].
///
/// # Arguments
///
/// * `options` - A slice containing `[period]`.
/// * `_decimals` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the Vortex indicator.
///
/// # Arguments
///
/// * `options` - A slice containing `[period]`.
///
/// # Returns
///
/// The minimum amount of data required (`period + 1`).
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}
/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing `[period]`.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) // + 1
}

/// Calculates the Vortex indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
///
/// # Options
///
/// * `options[0]` — period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - `Some(&[true])` to enable the optional TR output.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `vi_up`, `outputs[1]` is `vi_down`,
/// `outputs[2]` is the optional TR line, and `state` can be passed to
/// `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;
    let [high, low, close] = inputs;

    let (mut vi_up_line, mut vi_down_line, mut tr_line) = {
        let len = high.len();
        let capacity = output_length(len, options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false],
                tr_line_line: tr_output_length(len, options)
            ),
        )
    };

    let mut state = IndicatorState::init_state(high, low, close, period, &mut tr_line);
    let inputs = (
        &high[period + 1..],
        &low[period + 1..],
        &close[period + 1..],
    );
    let tr = {
        let offset = crate::slice_outputs_start!(vi_up_line.len(), tr_line);
        &mut tr_line[offset..]
    };
    // Single-pass calculation loop.
    cycle(inputs, &mut state, (&mut vi_up_line, &mut vi_down_line), tr);

    Ok((vec![vi_up_line, vi_down_line, tr_line], state))
}

/// Performs the main calculation loop for the Vortex indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of `(high, low, close)` price slices.
/// * `state` - A mutable reference to the current indicator state.
/// * `outputs` - A tuple of `(vi_up_line, vi_down_line)` output slices.
/// * `tr_line` - Output slice for the optional TR values (empty if not requested).
fn cycle(
    inputs: (&[f64], &[f64], &[f64]),
    state: &mut IndicatorState,
    outputs: (&mut [f64], &mut [f64]),
    tr_line: &mut [f64],
) {
    let (high, low, close) = inputs;
    let (vi_up_line, vi_down_line) = outputs;
    let (_, want_tr) = crate::calc_want_flags!(tr_line);

    for i in 0..high.len() {
        let tr;
        unsafe {
            let (vi_up, vi_down);
            (vi_up, vi_down, tr) = state.calc_unchecked(
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            );
            *vi_up_line.get_unchecked_mut(i) = vi_up;
            *vi_down_line.get_unchecked_mut(i) = vi_down;
        }

        crate::store_optional_outputs!(i,
            want_tr, tr_line => tr
        );
    }
}
