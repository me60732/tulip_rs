use crate::common::{min_process, validate_inputs};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::ema::{
    calc as calc_ema, multiplier as ema_multiplier, output_length as ema_output_length,
};
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info,
};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::ppo_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::ppo_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::ppo_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::ppo_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    multipliers: ((f64, f64), (f64, f64)),
    state: State,
}
impl IndicatorState {
    pub fn new(state: State, multipliers: ((f64, f64), (f64, f64))) -> Self {
        Self { state, multipliers }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let real = inputs[0];

        let (mut ppo_line, mut short_ema_line, mut long_ema_line);
        {
            let capacity = real.len();
            ppo_line = crate::uninit_vec!(f64, capacity);

            (short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                short_ema_line: capacity,
                long_ema_line: capacity
            );
        }
        cycle_ppo(
            real,
            self.multipliers,
            &mut ppo_line,
            &mut self.state,
            (&mut short_ema_line, &mut long_ema_line),
        );

        Ok(vec![ppo_line, short_ema_line, long_ema_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub short_ema: f64,
    pub long_ema: f64,
}
impl State {
    pub fn new(short_ema: f64, long_ema: f64) -> Self {
        State {
            short_ema,
            long_ema,
        }
    }
    pub fn init_state(real: &[f64], periods: (usize, usize), short_ema_line: &mut [f64]) -> Self {
        let (short_multiplier, long_multiplier) = multiplier(periods.0, periods.1);
        let (_, long_period) = periods;
        let (mut short_ema, mut long_ema) = (real[0], real[0]);
        for i in 1..long_period {
            short_ema = calc_ema(&real[i], short_ema, short_multiplier);
            long_ema = calc_ema(&real[i], long_ema, long_multiplier);
            crate::init_store_optional_outputs!(i, real.len(),
                short_ema_line => short_ema
            );
        }

        Self {
            short_ema,
            long_ema,
        }
    }
}
/// Returns information about the Percentage Price Oscillator (PPO) indicator.
pub const INFO: Info = Info {
    name: "ppo",
    full_name: "Percentage Price Oscillator",
    indicator_type: IndicatorType::Momentum,
    inputs: &["real"],
    options: &["short_period", "long_period"],
    outputs: &["ppo"],
    optional_outputs: &["short_ema", "long_ema"],
    display_groups: &[
        DisplayGroup {
            id: "ppo",
            label: "PPO",
            display_type: DisplayType::Indicator,
            outputs: &["ppo"],
        },
        DisplayGroup {
            id: "short_ema_long_ema",
            label: "EMAs",
            display_type: DisplayType::Overlay,
            outputs: &["short_ema", "long_ema"],
        },
    ],
};
/// Returns the minimum number of input bars required to produce results
/// accurate to `decimals` decimal places.
///
/// For indicators with exponential smoothing the seed value's influence
/// must decay below the requested precision, so this value grows with
/// `decimals`. Internally uses `min_process` with the long-period EMA
/// smoothing multiplier to calculate the required lookback.
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options (e.g. `short_period`, `long_period`).
/// * `decimals` - The number of decimal places of accuracy required.
///
/// # Returns
///
/// The minimum number of input bars needed for the requested accuracy.
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    let (_short_multiplier, long_multiplier) = multiplier(options[0] as usize, options[1] as usize);
    min_process(
        options,
        Some((decimals, 0)),
        &[long_multiplier.0],
        IndicatorInfoOrInteger::Integer(0),
        min_data,
    )
}
/// Returns the minimum amount of data required for the PPO indicator.
pub fn min_data(options: &[f64]) -> usize {
    options[1] as usize + 1
}

/// Returns the output length for the PPO indicator.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Calculates the Percentage Price Oscillator (PPO) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — real (a price series, e.g. close)
///
/// # Options
///
/// * `options[0]` — short_period
/// * `options[1]` — long_period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Optional slice of booleans enabling extra outputs:
///   `[0]` → `short_ema`, `[1]` → `long_ema`.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `ppo` line,
/// `outputs[1]` is the `short_ema` line (empty if not requested), and
/// `outputs[2]` is the `long_ema` line (empty if not requested). `state` can be
/// passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    validate_inputs(inputs, min_data(options))?;

    let real = inputs[0];

    let (mut ppo_line, mut short_ema_line, mut long_ema_line, mut state, long_period, multipliers);
    {
        let short_period = options[0] as usize;
        long_period = options[1] as usize;
        multipliers = multiplier(short_period, long_period);
        let capacity = output_length(real.len(), options);
        let short_ema_capacity = ema_output_length(real.len(), &[short_period as f64]);

        ppo_line = crate::uninit_vec!(f64, capacity);

        (short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            short_ema_line: short_ema_capacity,
            long_ema_line: capacity
        );

        state = State::init_state(real, (short_period, long_period), &mut short_ema_line);
    }
    let optional_outputs = {
        let offset = crate::slice_outputs_start!(ppo_line.len(), short_ema_line);
        (&mut short_ema_line[offset..], long_ema_line.as_mut_slice())
    };

    cycle_ppo(
        &real[long_period..],
        multipliers,
        &mut ppo_line,
        &mut state,
        optional_outputs,
    );

    Ok((
        vec![ppo_line, short_ema_line, long_ema_line],
        IndicatorState::new(state, multipliers),
    ))
}

/// Iterates over the input data and applies the calc function.
fn cycle_ppo(
    real: &[f64],
    multipliers: ((f64, f64), (f64, f64)),
    ppo_line: &mut [f64],
    state: &mut State,
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let (short_ema_line, long_ema_line) = out_vecs;
    let (has_optional, want_short, want_long) =
        crate::calc_want_flags!(short_ema_line, long_ema_line);

    for i in 0..real.len() {
        let value = unsafe { real.get_unchecked(i) };

        let ppo = calc(state, value, multipliers);

        unsafe { *ppo_line.get_unchecked_mut(i) = ppo };

        if has_optional {
            crate::store_optional_outputs!(i,
                want_short, short_ema_line => state.short_ema,
                want_long, long_ema_line => state.long_ema
            );
        }
    }
}

/// Performs the core calculation for the Percentage Price Oscillator (PPO) indicator.
#[inline(always)]
pub fn calc(state: &mut State, real: &f64, multipliers: ((f64, f64), (f64, f64))) -> f64 {
    let (short_multiplier, long_multiplier) = multipliers;
    let (mut short_ema, mut long_ema) = (state.short_ema, state.long_ema);

    short_ema = calc_ema(real, short_ema, short_multiplier);
    long_ema = calc_ema(real, long_ema, long_multiplier);

    (state.short_ema, state.long_ema) = (short_ema, long_ema);

    let long_ema_safe = long_ema.max(f64::EPSILON);
    (short_ema - long_ema) * 100.0 / long_ema_safe
}

#[inline(always)]
pub fn multiplier(short_period: usize, long_period: usize) -> ((f64, f64), (f64, f64)) {
    (ema_multiplier(short_period), ema_multiplier(long_period))
}
