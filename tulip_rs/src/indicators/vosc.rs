use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::sma::{
    calc as sma_calc, multiplier as sma_multiplier, output_length as sma_output_length,
};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;
/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::vosc_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::vosc_simd::indicator_by_options;

// Sub-module exports with common naming
/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::vosc_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::vosc_simd::indicator_by_options as indicator;
}

pub fn info() -> Info<'static> {
    Info {
        name: "vosc",
        full_name: "Volume Oscillator",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Volume,
        inputs: &["volume"],
        // Two options: short_period and long_period.
        options: &["short_period", "long_period"],
        outputs: &["vosc"],
        optional_outputs: &["short_sma", "long_sma"],
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    volume: Vec<f64>,
    state: State,
    multipliers: (f64, f64),
    periods: (usize, usize),
}
impl IndicatorState {
    pub fn new(
        volume: &[f64],
        state: State,
        multipliers: (f64, f64),
        periods: (usize, usize),
    ) -> Self {
        Self {
            volume: volume[volume.len() - periods.1..].to_vec(),
            state,
            multipliers,
            periods,
        }
    }
}

impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.volume.extend_from_slice(inputs[0]);
        let (mut vosc_line, (mut short_sma_line, mut long_sma_line)) = {
            let len = inputs[0].len();
            (
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    short_sma_line: len,
                    long_sma_line: len
                ),
            )
        };

        cycle(
            &self.volume,
            self.periods,
            self.multipliers,
            &mut self.state,
            &mut vosc_line,
            (&mut short_sma_line, &mut long_sma_line),
        );
        self.volume.drain(..self.volume.len() - self.periods.1);
        Ok(vec![vosc_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub short_sum: f64,
    pub long_sum: f64,
}
impl State {
    pub fn new(short_sum: f64, long_sum: f64) -> Self {
        State {
            short_sum,
            long_sum,
        }
    }
    /// Initializes the VOSC calculation by computing the initial fast and slow sums.
    /// The SMA for each is the sum over period divided by period. We use the last
    /// short_period values from the long window for the fast sum.
    pub fn init_state(
        short_period: usize,
        long_period: usize,
        volume: &[f64],
        short_sma_line: &mut [f64],
    ) -> Self {
        let mut short_sum = 0.0;
        let mut long_sum = 0.0;
        let multiplier = sma_multiplier(short_period);
        // Use the first long_period values.
        for (i, &vol) in volume.iter().enumerate().take(long_period) {
            long_sum += vol;
            if i >= short_period {
                short_sum += vol - volume[i - short_period];
                let short_sma = short_sum * multiplier;
                crate::init_store_optional_outputs!(i, volume.len(),
                    short_sma_line => short_sma
                );
            } else {
                short_sum += vol;
            }
        }
        Self::new(short_sum, long_sum)
    }
    #[inline(always)]
    pub fn calc(
        &mut self,
        vols: (&f64, &f64, &f64),
        short_multiplier: f64,
        long_multiplier: f64,
    ) -> (f64, f64, f64) {
        let fast_sma = sma_calc(&mut self.short_sum, vols.0, vols.1, &short_multiplier);
        let slow_sma = sma_calc(&mut self.long_sum, vols.0, vols.2, &long_multiplier);
        if slow_sma == 0.0 {
            return (0.0, fast_sma, slow_sma);
        }
        ((fast_sma - slow_sma) * 100.0 / slow_sma, fast_sma, slow_sma)
    }
}
/// Returns the minimum number of input bars required to produce accurate results.
///
/// For this indicator accuracy does not depend on decimal precision, so
/// this always returns the same value as [`min_data`].
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options: `[short_period, long_period]`.
/// * `_decimals` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the VOSC indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options: `[short_period, long_period]`.
///
/// # Returns
///
/// The minimum amount of data required (long_period + 1).
pub fn min_data(options: &[f64]) -> usize {
    options[1] as usize + 1
}

/// Calculates the output length based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the VOSC calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Calculates the Volume Oscillator (VOSC) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — `volume`
///
/// # Options
///
/// * `options[0]` — `short_period`
/// * `options[1]` — `long_period`
///
/// # Arguments
///
/// * `inputs` - Array of input slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true, true])` to enable optional outputs
///   `[short_sma, long_sma]`; `None` disables all.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `vosc` and `state`
/// can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let short_period = options[0] as usize;
    let long_period = options[1] as usize;
    let multipliers = multiplier(short_period, long_period);

    validate_inputs(inputs, min_data(options))?;
    let volume = inputs[0];
    let (mut vosc_line, (mut short_sma_line, mut long_sma_line)) = {
        let len = volume.len();
        let capacity = output_length(len, options);
        let short_sma_capacity = sma_output_length(len, &[short_period as f64]);
        (
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                short_sma_line: short_sma_capacity,
                long_sma_line: capacity
            ),
        )
    };
    let start = crate::slice_outputs_start!(vosc_line.len(), short_sma_line);
    // Initialize state.
    let mut state = State::init_state(short_period, long_period, volume, &mut short_sma_line);

    // The very first value is calculated during initialization.

    // Process from index = long_period (first full window is available).
    cycle(
        volume,
        (short_period, long_period),
        multipliers,
        &mut state,
        &mut vosc_line,
        (&mut short_sma_line[start..], &mut long_sma_line),
    );

    Ok((
        vec![vosc_line, short_sma_line, long_sma_line],
        IndicatorState::new(volume, state, multipliers, (short_period, long_period)),
    ))
}

/// Iterates over the volume data and computes VOSC values for each bar.
///
/// # Arguments
///
/// * `volume` - The full input volume slice.
/// * `periods` - A tuple of `(short_period, long_period)`.
/// * `multipliers` - A tuple of `(short_multiplier, long_multiplier)` from `multiplier()`.
/// * `state` - Mutable reference to the rolling `State` (fast and slow sums).
/// * `vosc_line` - Mutable output slice for VOSC values.
/// * `out_vecs` - Mutable output slices for optional outputs: `(short_sma, long_sma)`.
fn cycle(
    volume: &[f64],
    periods: (usize, usize),
    multipliers: (f64, f64),
    state: &mut State,
    vosc_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64]),
) {
    //if long_period <= short_period || volume.len() - long_period != vosc_line.len(){ return }
    let (short_multiplier, long_multiplier) = multipliers;
    let (short_period, long_period) = periods;
    let (short_sma_line, long_sma_line) = out_vecs;
    let (has_optional, want_short_sma, want_long_sma) =
        crate::calc_want_flags!(short_sma_line, long_sma_line);

    for (j, i) in (long_period..volume.len()).enumerate() {
        let (vosc, short_sma, long_sma);
        unsafe {
            (vosc, short_sma, long_sma) = state.calc(
                (
                    volume.get_unchecked(i),
                    volume.get_unchecked(i - short_period),
                    volume.get_unchecked(j),
                ),
                short_multiplier,
                long_multiplier,
            );
            *vosc_line.get_unchecked_mut(j) = vosc;
        }
        if has_optional {
            crate::store_optional_outputs!(j,
                want_short_sma, short_sma_line => short_sma,
                want_long_sma, long_sma_line => long_sma
            );
        }
    }
}

/// Calculates a single VOSC value for one bar, updating the rolling state in place.
///
/// # Arguments
///
/// * `state` - Mutable reference to the rolling `State` (fast and slow SMA sums).
/// * `vols` - A tuple of `(current_volume, prev_short_volume, prev_long_volume)`.
/// * `short_multiplier` - The SMA multiplier for the short period.
/// * `long_multiplier` - The SMA multiplier for the long period.
///
/// # Returns
///
/// A tuple of `(vosc, short_sma, long_sma)`.
#[inline(always)]
pub fn calc(
    state: &mut State,
    vols: (&f64, &f64, &f64),
    short_multiplier: f64,
    long_multiplier: f64,
) -> (f64, f64, f64) {
    state.calc(vols, short_multiplier, long_multiplier)
}

#[inline(always)]
pub fn multiplier(short_period: usize, long_period: usize) -> (f64, f64) {
    (sma_multiplier(short_period), sma_multiplier(long_period))
}
