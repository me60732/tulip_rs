use crate::common::{min_process, validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::ema::multiplier;
use crate::indicators::{
    ema::{calc as calc_ema, output_length as ema_output_length},
    tr::{calc as calc_tr, output_length as tr_output_length},
};
pub use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, RingBuffer};
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info,
};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::trvi_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::trvi_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::trvi_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::trvi_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub buffer: Buffer,
    pub prev_close: f64,
}
impl State {
    pub fn new(prev_close: f64, period: usize) -> Self {
        Self {
            prev_close,
            buffer: Buffer::new(period),
        }
    }
    pub fn init_state(
        inputs: &[&[f64]; INPUTS_WIDTH],
        period: usize,
        tr_line: &mut [f64],
        ema_line: &mut [f64],
    ) -> Self {
        let mut state = State::new(inputs[2][0], period);

        let [high, low, close] = *inputs;
        let multiplier = multiplier(period);

        for i in 1..period * 2 - 1 {
            let (h, l, c) = (high[i], low[i], close[i]);
            let (tr, ema);
            if i < period {
                tr = calc_tr(h, l, state.prev_close);
                let base = state.buffer.back().unwrap_or(tr);

                ema = calc_ema(&tr, base, multiplier);
                state.buffer.push(ema);
            } else {
                (_, tr, ema) = state.calc(h, l, c, multiplier);
            }
            state.prev_close = c;
            crate::init_store_optional_outputs!(i, high.len(),
                tr_line => tr,
                ema_line => ema
            );
        }

        state
    }
    #[inline]
    pub fn calc(
        &mut self,
        high: f64,
        low: f64,
        close: f64,
        multiplier: (f64, f64),
    ) -> (f64, f64, f64) {
        let prev_ema = self.buffer.back().unwrap();
        let old_ema = self.buffer.front().unwrap();
        let tr = calc_tr(high, low, self.prev_close);
        let ema = calc_ema(&tr, prev_ema, multiplier);
        self.buffer.push(ema);
        self.prev_close = close;
        if old_ema.abs() < f64::EPSILON {
            (0.0, tr, ema)
        } else {
            ((ema - old_ema) / old_ema * 100.0, tr, ema)
        }
    }
    #[inline(always)]
    pub(crate) unsafe fn calc_unchecked(
        &mut self,
        high: f64,
        low: f64,
        close: f64,
        multiplier: (f64, f64),
    ) -> (f64, f64, f64) {
        let prev_ema = self.buffer.back_unchecked();
        let old_ema = self.buffer.front_unchecked();
        let tr = calc_tr(high, low, self.prev_close);
        self.prev_close = close;
        let ema = calc_ema(&tr, prev_ema, multiplier);
        self.buffer.push_unchecked(ema);

        ((ema - old_ema) / old_ema * 100.0, tr, ema)
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    multiplier: (f64, f64),
}
impl IndicatorState {
    pub fn new(state: State, multiplier: (f64, f64)) -> Self {
        Self { state, multiplier }
    }
}
impl TIndicatorState<INPUTS_WIDTH> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut trvi_line, (mut tr_line, mut ema_line)) = {
            let len = inputs[0].len();
            (
                crate::uninit_vec!(f64, inputs[0].len()),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    tr_line: len,
                    ema_line: len
                ),
            )
        };
        let [high, low, close] = inputs;
        cycle(
            (high, low, close),
            self.multiplier,
            &mut self.state,
            &mut trvi_line,
            (&mut tr_line, &mut ema_line),
        );

        Ok(vec![trvi_line, tr_line, ema_line])
    }
}
/// Returns information about the True Range Volatility Indicator (TRVI).
///
/// TRVI is structurally identical to the Chaikin Volatility Indicator (CVI),
/// but substitutes **True Range** for the simple high-low spread. This makes
/// the indicator more sensitive to overnight price gaps and unusually large
/// bars, both of which inflate TR without widening the intraday high-low range.
///
/// # Returns
///
/// An `Info` struct containing metadata about the TRVI indicator.
pub const INFO: Info = Info {
    name: "trvi",
    indicator_type: IndicatorType::Volatility,
    full_name: "True Range Volatility Indicator",
    inputs: &["high", "low", "close"],
    options: &["period"],
    outputs: &["trvi"],
    optional_outputs: &["tr", "ema"],
    display_groups: &[
        DisplayGroup {
            offset: None,
            id: "trvi",
            label: "True Range Volatility Indicator",
            display_type: DisplayType::Indicator,
            outputs: &["trvi"],
        },
        DisplayGroup {
            offset: None,
            id: "tr",
            label: "True Range",
            display_type: DisplayType::Indicator,
            outputs: &["tr", "ema"],
        },
    ],
};
/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// For indicators with exponential smoothing the seed value's influence
/// must decay below the requested precision, so this value grows with
/// `decimals`. Internally uses `min_process` with the smoothing
/// multiplier to calculate the required lookback.
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options (e.g. period).
/// * `decimals` - The number of decimal places of accuracy required.
///
/// # Returns
///
/// The minimum number of input bars needed for the requested accuracy.
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    min_process(
        options,
        Some((decimals, 0)),
        &[multiplier(options[0] as usize).0],
        IndicatorInfoOrInteger::Integer(1),
        min_data,
    )
}
/// Returns the minimum amount of data required for the TRVI indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the TRVI calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    (options[0] * 2.0) as usize
}

/// Returns the number of output values given an input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the TRVI calculation.
///
/// # Returns
///
/// The number of output values (`data_len - min_data(options) + 1`).
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the True Range Volatility Indicator (TRVI) over the full input dataset.
///
/// TRVI is the Chaikin Volatility Indicator (CVI) computed on True Range rather
/// than the simple high-low spread, making it more responsive to overnight gaps
/// and unusually large bars.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices (used to compute True Range via the previous close)
///
/// # Options
///
/// * `options[0]` — period (EMA window used to smooth the True Range)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true, true])` to also emit the `tr` and `ema`
///   intermediate series; `None` or `Some(&[false, false])` disables them.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `trvi`, `outputs[1]` is `tr`
/// (optional), `outputs[2]` is the EMA of TR (optional), and `state` can be
/// passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;
    let [high, low, close] = *inputs;
    let (mut trvi_line, (mut tr_line, mut ema_line)) = {
        let capacity = output_length(high.len(), options);
        let tr_capacity = tr_output_length(high.len(), options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                tr_line: tr_capacity,
                ema_line: ema_output_length(tr_capacity, options)
            ),
        )
    };

    let mut state = State::init_state(inputs, period, &mut tr_line, &mut ema_line);

    let multiplier = multiplier(period);
    let (high, low, close) = {
        let from = period * 2 - 1;
        (&high[from..], &low[from..], &close[from..])
    };
    let (tr, ema) = {
        let (tr_offset, ema_offset) =
            crate::slice_outputs_start!(trvi_line.len(), tr_line, ema_line);
        (&mut tr_line[tr_offset..], &mut ema_line[ema_offset..])
    };
    cycle(
        (high, low, close),
        multiplier,
        &mut state,
        &mut trvi_line,
        (tr, ema),
    );

    Ok((
        vec![trvi_line, tr_line, ema_line],
        IndicatorState { state, multiplier },
    ))
}

/// Performs the main calculation loop for the TRVI indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of (high, low, close) price slices.
/// * `multiplier` - A tuple `(multiplier, inv_multiplier)` derived from the EMA period.
/// * `state` - Mutable reference to the indicator state (EMA ring buffer + prev close).
/// * `trvi_line` - Mutable slice to write the TRVI output values into.
/// * `optional_outputs` - Mutable slices for the optional TR and EMA outputs.
fn cycle(
    inputs: (&[f64], &[f64], &[f64]),
    multiplier: (f64, f64),
    state: &mut State,
    trvi_line: &mut [f64],
    optional_outputs: (&mut [f64], &mut [f64]),
) {
    let (high, low, close) = inputs;
    let (tr_line, ema_line) = optional_outputs;
    let (has_optional, want_tr, want_ema) = crate::calc_want_flags!(tr_line, ema_line);

    for i in 0..high.len() {
        let (trvi, tr, ema);
        unsafe {
            (trvi, tr, ema) = state.calc_unchecked(
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
                multiplier,
            );
            *trvi_line.get_unchecked_mut(i) = trvi;
        }
        if has_optional {
            crate::store_optional_outputs!(i,
                want_tr, tr_line => tr,
                want_ema, ema_line => ema
            );
        }
    }
}
