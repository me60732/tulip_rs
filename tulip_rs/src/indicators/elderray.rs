use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;

use crate::indicators::ema::{calc as calc_ema, init_state};
pub use crate::indicators::ema::{min_data, min_data_accuracy, multiplier, output_length};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
//use wide::*;

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::elderray_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::elderray_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::elderray_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::elderray_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    multipliers: (f64, f64),
    ema: f64,
}
impl IndicatorState {
    pub fn new(ema: f64, multipliers: (f64, f64)) -> Self {
        Self { ema, multipliers }
    }
    pub fn get_ema(&self) -> f64 {
        self.ema
    }
    pub fn get_multipliers(&self) -> (f64, f64) {
        self.multipliers
    }
    pub fn set_ema(&mut self, ema: f64) {
        self.ema = ema;
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let inputs = (inputs[0], inputs[1], inputs[2]);

        let (mut bull_line, mut bear_line, mut ema_line) = {
            let len = inputs.0.len();
            (
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    ema_line: len
                ),
            )
        };
        cycle(
            inputs,
            &mut self.ema,
            self.multipliers,
            (&mut bull_line, &mut bear_line, &mut ema_line),
        );
        Ok(vec![bull_line, bear_line, ema_line])
    }
}
/// Metadata describing the Elder-ray indicator.
pub const INFO: Info = Info {
    name: "elderray",
    full_name: "Elder-ray",
    indicator_type: IndicatorType::Trend,
    inputs: &["high", "low", "close"],
    options: &["period"],
    outputs: &["bull", "bear"],
    optional_outputs: &["ema"],
    display_groups: &[
        DisplayGroup {
            id: "elderray",
            label: "Elder-ray",
            display_type: DisplayType::Indicator,
            outputs: &["bull", "bear"],
        },
        DisplayGroup {
            id: "ema",
            label: "EMA",
            display_type: DisplayType::Overlay,
            outputs: &["ema"],
        },
    ],
};

/// Calculates the Elder-ray indicator for an entire dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
///
/// # Options
///
/// * `options[0]` — EMA period
///
/// # Outputs
///
/// * `outputs[0]` — `bull` power line (`high − EMA`)
/// * `outputs[1]` — `bear` power line (`low − EMA`)
/// * `outputs[2]` — `ema` line (optional; populate by passing `Some(&[true])`)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true])` to also return the EMA line.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the bull line, `outputs[1]` is
/// the bear line, and `outputs[2]` is the optional EMA line. `state` can be
/// passed to `IndicatorState::batch_indicator` for streaming updates.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    validate_inputs(inputs, min_data(options))?;

    let (mut bull_line, mut bear_line, mut ema_line, inputs, multipliers, mut ema) = {
        let capacity = output_length(inputs[0].len(), options);
        let multipliers = multiplier(options[0] as usize);
        let period = options[0] as usize;
        (
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false],
                ema_line: capacity
            ),
            (
                &inputs[0][period..],
                &inputs[1][period..],
                &inputs[2][period..],
            ),
            multipliers,
            init_state(inputs[2], period, multipliers),
        )
    };

    cycle(
        inputs,
        &mut ema,
        multipliers,
        (&mut bull_line, &mut bear_line, &mut ema_line),
    );
    Ok((
        vec![bull_line, bear_line, ema_line],
        IndicatorState { ema, multipliers },
    ))
}
fn cycle(
    inputs: (&[f64], &[f64], &[f64]),
    ema: &mut f64,
    multipliers: (f64, f64),
    outputs: (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (high, low, close) = inputs;
    let (bull_line, bear_line, ema_line) = outputs;
    let (_, want_ema) = crate::calc_want_flags!(ema_line);
    let mut prev_ema = *ema;

    for i in 0..high.len() {
        let (high, low, close) = unsafe {
            (
                high.get_unchecked(i),
                low.get_unchecked(i),
                close.get_unchecked(i),
            )
        };
        let (bull, bear);
        (bull, bear, prev_ema) = calc(high, low, close, prev_ema, multipliers);
        unsafe {
            *bull_line.get_unchecked_mut(i) = bull;
            *bear_line.get_unchecked_mut(i) = bear;
        };

        crate::store_optional_outputs!(i,
            want_ema, ema_line => prev_ema
        );
    }
    *ema = prev_ema;
}

#[inline(always)]
pub fn calc(
    high: &f64,
    low: &f64,
    close: &f64,
    prev_ema: f64,
    multipliers: (f64, f64),
) -> (f64, f64, f64) {
    let ema = calc_ema(close, prev_ema, multipliers);

    (high - ema, low - ema, ema)
}
