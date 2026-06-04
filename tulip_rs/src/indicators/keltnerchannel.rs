use crate::common::{min_process, validate_inputs};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::{
    atr::{init_calc, multiplier as atr_multiplier, State as AtrState},
    ema::{calc as ema_calc, multiplier as ema_multiplier},
    tr::output_length as tr_output_length,
};
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info,
};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::keltnerchannel_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::keltnerchannel_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::keltnerchannel_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::keltnerchannel_simd::indicator_by_options as indicator;
}

/// Returns information about the Keltner Channel indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the Keltner Channel indicator.
pub const INFO: Info = Info {
    name: "keltnerchannel",
    full_name: "Keltner Channel",
    indicator_type: IndicatorType::Volatility,
    inputs: &["high", "low", "close"],
    options: &["period", "step"],
    outputs: &["lower", "middle", "upper"],
    optional_outputs: &["atr", "tr"],
    display_groups: &[
        DisplayGroup {
            id: "keltnerchannel",
            label: "Keltner Channel",
            display_type: DisplayType::Overlay,
            outputs: &["lower", "middle", "upper"],
        },
        DisplayGroup {
            id: "atr_tr",
            label: "True Range",
            display_type: DisplayType::Indicator,
            outputs: &["atr", "tr"],
        },
    ],
};
#[derive(Serialize, Deserialize)]
pub struct State {
    pub atr_state: AtrState,
    pub ema: f64,
}
impl State {
    /// Initialises the Keltner Channel state from the first `period` bars.
    ///
    /// Seeds the ATR with the simple-average true range over `[0, period)` and
    /// seeds the EMA with the exponentially-smoothed close over the same window.
    /// If `tr_line` is non-empty the raw true-range values for bars `[1, period)` are
    /// written into it (index 0 = bar 1).
    ///
    /// # Arguments
    ///
    /// * `high` - High prices; must contain at least `period` elements.
    /// * `low` - Low prices; must contain at least `period` elements.
    /// * `close` - Close prices; must contain at least `period` elements.
    /// * `period` - Lookback period for ATR and EMA initialisation.
    /// * `multipliers` - Smoothing constants `((atr_alpha, atr_1m_alpha), (ema_alpha, ema_1m_alpha))`.
    /// * `tr_line` - Optional output buffer for raw true-range values written during warm-up.
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        multipliers: ((f64, f64), (f64, f64)),
        tr_line: &mut [f64],
    ) -> Self {
        let mut atr = high[0] - low[0];
        let mut ema = close[0];
        let mut tr;
        for i in 1..period {
            let prev_close = close[i - 1];
            (atr, tr) = init_calc(high[i], low[i], prev_close, atr);
            ema = ema_calc(&close[i], ema, multipliers.1);
            if tr_line.len() > 0 {
                tr_line[i - 1] = tr;
            }
        }
        atr /= period as f64;
        Self {
            atr_state: AtrState::new(atr, close[period - 1]),
            ema,
        }
    }
    /// Advances the indicator by one bar and returns the channel values.
    ///
    /// Updates the ATR and EMA states, then computes the lower and upper channel
    /// bands as `EMA ± step × ATR`.
    ///
    /// # Arguments
    ///
    /// * `high` - High price for the current bar.
    /// * `low` - Low price for the current bar.
    /// * `close` - Close price for the current bar.
    /// * `step` - ATR multiplier controlling channel width.
    /// * `multipliers` - Smoothing constants `((atr_alpha, atr_1m_alpha), (ema_alpha, ema_1m_alpha))`.
    ///
    /// # Returns
    ///
    /// A tuple `(lower, middle, upper, atr, tr)` where `middle` is the current EMA.
    #[inline(always)]
    pub fn calc(
        &mut self,
        high: f64,
        low: f64,
        close: f64,
        step: f64,
        multipliers: ((f64, f64), (f64, f64)),
    ) -> (f64, f64, f64, f64, f64) {
        let (atr, tr) = self.atr_state.calc(high, low, close, multipliers.0);
        self.ema = ema_calc(&close, self.ema, multipliers.1);

        let per = atr * step;
        let upper = self.ema + per;
        let lower = self.ema - per;

        (lower, self.ema, upper, atr, tr)
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    multipliers: ((f64, f64), (f64, f64)),
    state: State,
    step: f64,
}
impl IndicatorState {
    /// Creates a new `IndicatorState` for streaming continuation.
    ///
    /// # Arguments
    ///
    /// * `state` - The internal ATR + EMA state after the last computed bar.
    /// * `step` - ATR multiplier controlling channel width.
    /// * `multipliers` - Smoothing constants `((atr_alpha, atr_1m_alpha), (ema_alpha, ema_1m_alpha))`.
    pub fn new(state: State, step: f64, multipliers: ((f64, f64), (f64, f64))) -> Self {
        Self {
            state,
            step,
            multipliers,
        }
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let [high, low, close] = inputs;
        let (mut middle_band, mut upper_band, mut lower_band, (mut atr_line, mut tr_line)) = {
            let len = high.len();
            (
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    atr_line: len,
                    tr_line: len
                ),
            )
        };

        cycle(
            (high, low, close),
            self.step,
            self.multipliers,
            (&mut lower_band, &mut middle_band, &mut upper_band),
            &mut self.state,
            (&mut atr_line, &mut tr_line),
        );

        Ok(vec![lower_band, middle_band, upper_band, atr_line, tr_line])
    }
}
/// Returns the minimum number of input bars required to produce accurate results.
///
/// The EMA (middle band) uses exponential smoothing, so more warm-up bars are needed
/// to achieve higher decimal precision. This function accounts for EMA convergence time.
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options.
/// * `decimals` - The required decimal precision. Higher values increase the returned minimum.
///
/// # Returns
///
/// The minimum number of input bars required to achieve `decimals` of precision.
/// Always ≥ [`min_data`].
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    min_process(
        options,
        Some((decimals, 0)),
        &[multiplier(options[0] as usize).1 .0],
        IndicatorInfoOrInteger::Info(INFO),
        min_data,
    )
}
/// Returns the minimum amount of data required for the Keltner Channel indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the Keltner Channel calculation.
///
/// # Returns
///
/// The minimum amount of data required (`period + 1`).
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length for the Keltner Channel indicator.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the Keltner Channel calculation.
///
/// # Returns
///
/// The number of output values produced by the Keltner Channel calculation (`data_len - period`).
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
/// Validates Keltner Channel options.
///
/// # Errors
///
/// Returns `Err(IndicatorError::InvalidOptions)` if `period < 1` or `step ≤ 0`.
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= 0.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Calculates the Keltner Channel indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
///
/// # Options
///
/// * `options[0]` — period (lookback for ATR and EMA)
/// * `options[1]` — step (ATR multiplier controlling channel width)
///
/// # Outputs
///
/// * `outputs[0]` — `lower` band (`EMA - step × ATR`)
/// * `outputs[1]` — `middle` band (EMA of close)
/// * `outputs[2]` — `upper` band (`EMA + step × ATR`)
///
/// # Optional Outputs
///
/// * `atr` — the Wilder ATR series used in the calculation
/// * `tr`  — the True Range series
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Optional flags `[want_atr, want_tr]` to enable extra outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `lower`, `outputs[1]` is `middle`,
/// `outputs[2]` is `upper`, `outputs[3]` is `atr` (empty unless requested),
/// `outputs[4]` is `tr` (empty unless requested), and `state` can be passed to
/// `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let step = options[1];
    let [high, low, close] = inputs;
    let multipliers = multiplier(period);

    validate_inputs(inputs, min_data(options))?;

    let (mut middle_band, mut upper_band, mut lower_band, (mut atr_line, mut tr_line)) = {
        let len = high.len();
        let capacity = output_length(len, options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                atr_line: capacity,
                tr_line: tr_output_length(len, options)
            ),
        )
    };

    let mut state = State::init_state(high, low, close, period, multipliers, &mut tr_line);
    let (inputs, tr) = {
        let tr_offset = crate::slice_outputs_start!(middle_band.len(), tr_line);
        (
            (&high[period..], &low[period..], &close[period..]),
            &mut tr_line[tr_offset..],
        )
    };
    cycle(
        inputs,
        step,
        multipliers,
        (&mut lower_band, &mut middle_band, &mut upper_band),
        &mut state,
        (&mut atr_line, tr),
    );

    Ok((
        vec![lower_band, middle_band, upper_band, atr_line, tr_line],
        IndicatorState::new(state, step, multipliers),
    ))
}

/// Performs the main calculation loop for the Keltner Channel indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of `(high, low, close)` price slices (starting at the first output bar).
/// * `step` - The ATR multiplier controlling channel width.
/// * `multipliers` - Smoothing constants `((atr_alpha, atr_1m_alpha), (ema_alpha, ema_1m_alpha))`.
/// * `outputs` - A tuple of mutable slices for storing the `(lower, middle, upper)` channel bands.
/// * `state` - A mutable reference to the current indicator state.
/// * `optional_outputs` - A tuple of mutable slices for optional `(atr, tr)` outputs.
fn cycle(
    inputs: (&[f64], &[f64], &[f64]),
    step: f64,
    multipliers: ((f64, f64), (f64, f64)),
    outputs: (&mut [f64], &mut [f64], &mut [f64]),
    state: &mut State,
    optional_outputs: (&mut [f64], &mut [f64]),
) {
    let (lower_band, middle_band, upper_band) = outputs;
    let (high, low, close) = inputs;
    let (atr_line, tr_line) = optional_outputs;
    let (has_optional, want_atr, want_tr) = crate::calc_want_flags!(atr_line, tr_line);
    for i in 0..high.len() {
        let (high, low, close) = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };
        let (lower, middle, upper, atr, tr) = state.calc(high, low, close, step, multipliers);

        unsafe {
            *middle_band.get_unchecked_mut(i) = middle;
            *upper_band.get_unchecked_mut(i) = upper;
            *lower_band.get_unchecked_mut(i) = lower;
        }
        if has_optional {
            crate::store_optional_outputs!(i,
                want_atr, atr_line => atr,
                want_tr, tr_line => tr
            );
        }
    }
}

/// Returns the precomputed smoothing constants for the Keltner Channel.
///
/// # Returns
///
/// A tuple `((atr_alpha, atr_1m_alpha), (ema_alpha, ema_1m_alpha))` where the first pair
/// contains Wilder ATR smoothing constants and the second pair contains EMA smoothing constants.
pub fn multiplier(period: usize) -> ((f64, f64), (f64, f64)) {
    (atr_multiplier(period), ema_multiplier(period))
}
