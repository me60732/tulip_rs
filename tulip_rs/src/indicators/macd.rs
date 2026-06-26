use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::{
    ema::{State as EmaState, multiplier as ema_multiplier, output_length as ema_output_length},
    simd_indicators::ema_simd::{SimdState as EmaSimdState, multiplier_simd}
};
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info,
};
use std::simd::Simd;
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 3;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::macd_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::macd_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::macd_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::macd_simd::indicator_by_options as indicator;
}

/// Returns information about the Moving Average Convergence Divergence (MACD) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the MACD indicator.
pub const INFO: Info = Info {
    name: "macd",
    full_name: "Moving Average Convergence Divergence",
    indicator_type: IndicatorType::Trend,
    inputs: &["real"],
    options: &["short_period", "long_period", "signal_period"],
    outputs: &["macd_line", "signal_line", "histogram"],
    optional_outputs: &["short_ema", "long_ema"],
    display_groups: &[
        DisplayGroup {
            offset: None,
            id: "macd",
            label: "MACD",
            display_type: DisplayType::Indicator,
            outputs: &["macd_line", "signal_line", "histogram"],
        },
        DisplayGroup {
            offset: None,
            id: "short_ema_long_ema",
            label: "EMAs",
            display_type: DisplayType::Overlay,
            outputs: &["short_ema", "long_ema"],
        },
    ],
};
pub type IndicatorState = State;

impl TIndicatorState<1> for IndicatorState {
    #[inline(always)]
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut macd_line, mut signal_line, mut histogram, mut short_ema_line, mut long_ema_line);
        {
            let capacity = inputs[0].len();

            // Pre-allocate the result vectors with the calculated capacities
            macd_line = crate::uninit_vec!(f64, capacity);
            signal_line = crate::uninit_vec!(f64, capacity);
            histogram = crate::uninit_vec!(f64, capacity);

            (short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
            short_ema_line: capacity,
            long_ema_line: capacity
            );
        }
        cycle_macd(
            inputs[0],
            self,
            (&mut macd_line, &mut signal_line, &mut histogram),
            (&mut short_ema_line, &mut long_ema_line),
        );
        Ok(vec![
            macd_line,
            signal_line,
            histogram,
            short_ema_line,
            long_ema_line,
        ])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub ema_state: EmaSimdState<2>,
    pub signal_state: EmaState,
}
impl State {
    pub fn new(short_ema: f64, long_ema: f64, signal: f64, periods: (usize, usize, usize)) -> Self {
        let (multipliers, signal_mul) = (multiplier_simd([periods.0, periods.1]), ema_multiplier(periods.2));
        Self {
            ema_state: EmaSimdState::new(Simd::from_array([short_ema, long_ema]), multipliers),
            signal_state: EmaState::new(signal, signal_mul),
        }
    }
    pub fn init_state(
        real: &[f64],
        periods: (usize, usize, usize),
        macd_line: &mut [f64],
        out_vecs: (&mut [f64], &mut [f64]),
    ) -> Self {
        let (_, long_period, signal_period) = periods;
        let mut state = Self::new(real[0], real[0], 0.0, periods);
        let (short_ema_line, long_ema_line) = out_vecs;
        let (has_optional, _, _) = crate::calc_want_flags!(short_ema_line, long_ema_line);
        let mut count = 0;
        for i in 1..long_period + signal_period - 2 {
            let (macd, _, _) = state.calc(real[i]);
            if i == long_period - 1 {
                state.signal_state.ema = macd;
            }
            if i >= long_period - 1 {
                macd_line[count] = macd;
                count += 1;
            }
            if has_optional {
                crate::init_store_optional_outputs!(i, real.len(),
                    short_ema_line => state.ema_state.ema[0],
                    long_ema_line => state.ema_state.ema[1]
                );
            }
        }

        state
    }
    /// Calculates the current MACD value.
    ///
    /// # Arguments
    ///
    /// * `state` - A mutable reference to the current `State` holding EMA values.
    /// * `value` - The current input price value.
    /// * `multipliers` - A tuple of three EMA multiplier pairs for short, long, and signal periods.
    ///
    /// # Returns
    ///
    /// A tuple containing the MACD line value, signal line value, and histogram value.
    #[inline(always)]
    pub fn calc(
        &mut self,
        real: f64,
    ) -> (f64, f64, f64) {
        let [short_ema, long_ema] = self.ema_state.calc_simd(Simd::splat(real)).to_array();
    
        let macd_value = short_ema - long_ema;
        let signal = self.signal_state.calc(macd_value);
    
        (macd_value, signal, macd_value - signal)
    }
}
pub fn output_length(data_len: usize, options: &[f64]) -> (usize, usize, usize) {
    //let min_data = min_data(&options);
    let long_period = options[1] as usize;
    let signal_period = options[2] as usize;

    let macd_capacity = data_len - long_period + 1;
    let signal_capacity = macd_capacity - signal_period + 1;
    let histogram_capacity = signal_capacity;
    (macd_capacity, signal_capacity, histogram_capacity)
}

pub fn min_data(options: &[f64]) -> usize {
    (options[1] + options[2]) as usize - 1
}
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] || options[2] < 1.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Calculates the Moving Average Convergence Divergence (MACD) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real (close) prices
///
/// # Options
///
/// * `options[0]` — short_period
/// * `options[1]` — long_period
/// * `options[2]` — signal_period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true, false])` to enable optional outputs
///   (`short_ema`, `long_ema`); `None` disables all optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where:
/// - `outputs[0]` — `macd_line`
/// - `outputs[1]` — `signal_line`
/// - `outputs[2]` — `histogram`
/// - `outputs[3]` — `short_ema` (empty if not requested)
/// - `outputs[4]` — `long_ema` (empty if not requested)
///
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    validate_inputs(inputs, min_data(options))?;

    let (
        mut macd_line,
        mut signal_line,
        mut histogram,
        mut short_ema_line,
        mut long_ema_line,
        mut state,
        real,
    );
    {
        let short_period = options[0] as usize;
        let long_period = options[1] as usize;
        let signal_period = options[2] as usize;
        // Calculate capacities
        let len = inputs[0].len();
        let (macd_capacity, signal_capacity, histogram_capacity) = output_length(len, options);

        let short_ema_capacity = ema_output_length(len, &[short_period as f64]);
        let long_ema_capacity = ema_output_length(len, &[long_period as f64]);
        // Pre-allocate the result vectors with the calculated capacities
        macd_line = crate::uninit_vec!(f64, macd_capacity);
        signal_line = crate::uninit_vec!(f64, signal_capacity);
        histogram = crate::uninit_vec!(f64, histogram_capacity);

        (short_ema_line, long_ema_line) = crate::init_optional_outputs!(
            optional_outputs, &[false, false],
            short_ema_line: short_ema_capacity,
            long_ema_line: long_ema_capacity
        );
        state = State::init_state(
            inputs[0],
            (short_period, long_period, signal_period),
            &mut macd_line,
            (&mut short_ema_line, &mut long_ema_line),
        );
        let start = long_period + signal_period - 2;
        real = &inputs[0][start..]
    }
    let (macd_offset, short_offset, long_offset) =
        crate::slice_outputs_start!(signal_line.len(), macd_line, short_ema_line, long_ema_line);
    cycle_macd(
        real,
        &mut state,
        (
            &mut macd_line[macd_offset..],
            &mut signal_line,
            &mut histogram,
        ),
        (
            &mut short_ema_line[short_offset..],
            &mut long_ema_line[long_offset..],
        ),
    );

    Ok((
        vec![
            macd_line,
            signal_line,
            histogram,
            short_ema_line,
            long_ema_line,
        ],
        state,
    ))
}

//#[inline(always)]
fn cycle_macd(
    real: &[f64],
    state: &mut State,
    outputs: (&mut [f64], &mut [f64], &mut [f64]),
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let (macd_line, signal_line, histogram_line) = outputs;

    let (short_ema_line, long_ema_line) = out_vecs;
    let (has_optional, want_short, want_long) =
        crate::calc_want_flags!(short_ema_line, long_ema_line);

    for i in 0..real.len() {
        unsafe {
            (
                *macd_line.get_unchecked_mut(i),
                *signal_line.get_unchecked_mut(i),
                *histogram_line.get_unchecked_mut(i),
            ) = state.calc(*real.get_unchecked(i));
        }
        if has_optional {
            let [short_ema, long_ema] = state.ema_state.ema.to_array();
            crate::store_optional_outputs!(i,
                want_short, short_ema_line => short_ema,
                want_long, long_ema_line => long_ema
            );
        }
    }
}

pub fn multiplier(
    short_period: usize,
    long_period: usize,
    signal_period: usize,
) -> ((f64, f64), (f64, f64), (f64, f64)) {
    (
        ema_multiplier(short_period),
        ema_multiplier(long_period),
        ema_multiplier(signal_period),
    )
}
