use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;

use crate::indicators::{
    min::{calc as calc_min, calc_unchecked as calc_min_unchecked, State as MinState},
    max::{calc as calc_max, calc_unchecked as calc_max_unchecked, State as MaxState},
    atr::{output_length as atr_output_length, State as AtrState, multiplier as atr_multiplier},
    tr::output_length as tr_output_length,
};

use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
/*#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::aroon_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::aroon_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::aroon_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::aroon_simd::indicator_by_options as indicator;
}*/

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    high: Vec<f64>,
    low: Vec<f64>,
    state: State,
    periods: (usize, usize),
    multipliers: (f64, (f64, f64))
}
impl IndicatorState {
    pub fn new(high: &[f64], low: &[f64], state: State, periods: (usize, usize), multipliers: (f64, (f64, f64))) -> Self {
        Self {
            high: high[high.len() - periods.0..].to_vec(),
            low: low[low.len() - periods.0..].to_vec(),
            state,
            periods,
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

        let periods = self.periods;
        self.high.extend_from_slice(inputs[0]);
        self.low.extend_from_slice(inputs[1]);
        let close = inputs[2];
        let (mut long_line, mut short_line, (mut atr_line, mut tr_line)) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    atr_line: capacity,
                    tr_line: capacity
                )
            )
        };
        match periods.0 {
            1..=4 => {
                cycle::<1>(
                    (&self.high, &self.low, close),
                    periods,
                    self.multipliers,
                    (&mut long_line, &mut short_line),
                    &mut self.state,
                    (&mut atr_line, &mut tr_line)
                );
            }
            5..25 => {
                cycle::<4>(
                    (&self.high, &self.low, close),
                    periods,
                    self.multipliers,
                    (&mut long_line, &mut short_line),
                    &mut self.state,
                    (&mut atr_line, &mut tr_line)
                );
            }
            _ => {
                cycle::<8>(
                    (&self.high, &self.low, close),
                    periods,
                    self.multipliers,
                    (&mut long_line, &mut short_line),
                    &mut self.state,
                    (&mut atr_line, &mut tr_line)
                );
            }
        }

        self.high.drain(..self.high.len() - periods.0);
        self.low.drain(..self.low.len() - periods.0);

        Ok(vec![long_line, short_line, atr_line, tr_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub min_state: MinState,
    pub max_state: MaxState,
    pub atr_state: AtrState,
}
impl State {
    pub fn new(high: &[f64], low: &[f64], close: &[f64], period: usize, tr_line: &mut [f64]) -> Self {
        let min_state = MinState::new(low[0], period);
        let max_state = MaxState::new(high[0], period);
        let atr_state = AtrState::init_state(high, low, close, period, tr_line, false);
        State {
            min_state,
            max_state,
            atr_state,
        }
    }
}
/// Returns information about the Aroon indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the Aroon indicator.
pub const INFO: Info = Info {
    name: "chandelierexit",
    full_name: "Chandelier Exit",
    indicator_type: IndicatorType::Trend,
    inputs: &["high", "low", "close"],
    options: &["period", "multiplier"],
    outputs: &["long", "short"],
    optional_outputs: &["atr", "tr"],
    display_groups: &[
        DisplayGroup {
            id: "long_short",
            label: "Exit Positions",
            display_type: DisplayType::Overlay,
            outputs: &["long", "short"],
        },
        DisplayGroup {
            id: "atr_tr",
            label: "True Range",
            display_type: DisplayType::Indicator,
            outputs: &["atr", "tr"],
        }
    ]
};
/// Returns the minimum number of input bars required to produce accurate results.
///
/// For this indicator accuracy does not depend on decimal precision, so
/// this always returns the same value as [`min_data`].
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options.
/// * `_decimals` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the Aroon indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the Aroon calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length for the Aroon indicator.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the Aroon calculation.
///
/// # Returns
///
/// The number of output values produced by the Aroon calculation.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Aroon indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
///
/// # Options
///
/// * `options[0]` — period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; Aroon has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `aroon_down`, `outputs[1]` is `aroon_up`,
/// and `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    validate_inputs(inputs, min_data(options))?;

    let periods = (options[0] as usize, options[1] as usize - 1);
    let multipliers = (options[1], atr_multiplier(periods.0));
    let [high, low, close] = inputs;

    /*let (mut aroon_up_line, mut aroon_down_line) = {
        let capacity = output_length(high.len(), options);
        (crate::uninit_vec!(f64, capacity), crate::uninit_vec!(f64, capacity))
    };*/
    let (mut long_line, mut short_line, (mut atr_line, mut tr_line)) = {
        let len = high.len();
        let capacity = output_length(high.len(), options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                atr_line: atr_output_length(len, &[options[0]]),
                tr_line: tr_output_length(len, &[])
            )
        )
    };
    
    let mut state = State::new(high, low, close, periods.0, &mut tr_line);
    let tr = {
      let tr_offset = crate::slice_outputs_start!(atr_line.len(), tr_line);
      &mut tr_line[tr_offset..]
    };
    match periods.0 {
        1..=10 => {
            cycle::<1>(
                (high, low, &close[periods.0..]),
                periods,
                multipliers,
                (&mut long_line, &mut short_line),
                &mut state,
                (&mut atr_line, tr)
            );
        }
        11..=25 => {
            cycle::<4>(
                (high, low, &close[periods.0..]),
                periods,
                multipliers,
                (&mut long_line, &mut short_line),
                &mut state,
                (&mut atr_line, tr)
            );
        }
        _ => {
            cycle::<8>(
                (high, low, &close[periods.0..]),
                periods,
                multipliers,
                (&mut long_line, &mut short_line),
                &mut state,
                (&mut atr_line, tr)
            );
        }
    }
    Ok((
        vec![long_line, short_line, atr_line, tr_line],
        IndicatorState::new(high, low, state, periods, multipliers),
    ))
}

/// Performs the main calculation loop for the Aroon indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of high and low price slices.
/// * `period` - The period for the Aroon calculation.
/// * `multiplier` - The multiplier used to scale Aroon values (100 / period).
/// * `output_lines` - A tuple of mutable slices for storing the Aroon down and Aroon up lines.
/// * `state` - A mutable reference to the current indicator state.
fn cycle<const N: usize>(
    inputs: (&[f64], &[f64], &[f64]),
    periods: (usize, usize),
    multipliers: (f64, (f64, f64)),
    output_lines: (&mut [f64], &mut [f64]),
    state: &mut State,
    optional_outputs: (&mut [f64], &mut [f64])
) {
    let (high, low, close) = inputs;
    let (long_line, short_line) = output_lines;
    let (atr_line, tr_line) = optional_outputs;
    let (has_optional, want_atr, want_tr) = crate::calc_want_flags!(atr_line, tr_line);
    for (j, i) in (periods.0..inputs.0.len()).enumerate() {
        let (long, short, atr, tr);
        unsafe {
            (long, short, atr, tr) = calc_unchecked::<N>(state, (high, low, *close.get_unchecked(j)), i, periods, multipliers);
            *long_line.get_unchecked_mut(j) = long;
            *short_line.get_unchecked_mut(j) = short;
        }
        if has_optional {
            crate::store_optional_outputs!(j,
                want_atr, atr_line => atr
            );
            crate::store_optional_outputs!(j,
                want_tr, tr_line => tr
            );
        }
    }

}
#[inline(always)]
pub fn calc(
    state: &mut State,
    inputs: (&[f64], &[f64], f64),
    i: usize,
    periods: (usize, usize),
    multipliers: (f64, (f64, f64)),
) -> (f64, f64, f64, f64) {
    let (high, low, close) = inputs;
    let (step, atr_multipliers) = multipliers;
    let (min, _) = calc_min(&mut state.min_state, low, i, periods);
    let (max, _) = calc_max(&mut state.max_state, high, i, periods);

    let (atr, tr) = state.atr_state.calc(high[i], low[i], close, atr_multipliers);

    //let per = atr * step;   
    let long = atr.mul_add(-step, max);
    //let long = max - per;
    //let short = min + per;
    let short = atr.mul_add(step, min);
    (long, short, atr, tr)
}
#[inline(always)]
pub(crate) unsafe fn calc_unchecked<const N: usize>(
    state: &mut State,
    inputs: (&[f64], &[f64], f64),
    i: usize,
    periods: (usize, usize),
    multipliers: (f64, (f64, f64)),
) -> (f64, f64, f64, f64) {
    let (high, low, close) = inputs;
    let (step, atr_multipliers) = multipliers;
    let (min, _) = calc_min_unchecked::<N>(&mut state.min_state, low, i, periods);
    let (max, _) = calc_max_unchecked::<N>(&mut state.max_state, high, i, periods);

    let (atr, tr) = state.atr_state.calc(*high.get_unchecked(i), *low.get_unchecked(i), close, atr_multipliers);
    let long = atr.mul_add(-step, max);
    let short = atr.mul_add(step, min);
    (long, short, atr, tr)
}

