use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::medprice::calc as calc_medprice;
use crate::indicators::sma::{
    calc as sma_calc, multiplier as sma_multiplier, output_length as sma_output_length,
};
use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, RingBuffer};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 0;
pub const SHORT_PERIOD: usize = 5;
pub const LONG_PERIOD: usize = 34;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::ao_simd::indicator_by_assets;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::ao_simd::indicator_by_assets as indicator;
}
/// Returns information about the Awesome Oscillator (AO) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the AO indicator.
pub const INFO: Info = Info {
    name: "ao",
    full_name: "Awesome Oscillator",
    indicator_type: IndicatorType::Momentum,
    inputs: &["high", "low"],
    options: &[],
    outputs: &["ao"],
    optional_outputs: &["short_sma", "long_sma", "medprice"],
    display_groups: &[
        DisplayGroup {
            id: "ao",
            label: "AO",
            display_type: DisplayType::Indicator,
            outputs: &["ao"],
        },
        DisplayGroup {
            id: "short_sma_long_sma_medprice",
            label: "Median Price",
            display_type: DisplayType::Overlay,
            outputs: &["short_sma", "long_sma", "medprice"],
        },
    ],
};

#[derive(Serialize, Deserialize)]
pub struct State {
    pub buffer: Buffer,
    pub short_sum: f64,
    pub long_sum: f64,
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    multipliers: (f64, f64),
}
impl IndicatorState {
    pub fn new(state: State, multipliers: (f64, f64)) -> Self {
        Self { state, multipliers }
    }
}
impl TIndicatorState<2> for IndicatorState {
    #[inline(always)]
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        //let mut ao_line: Vec<f64> = vec![0.0; inputs[0].len()]; //Vec::with_capacity(inputs[0].len());

        let capacity = inputs[0].len();
        let mut ao_line = crate::uninit_vec!(f64, capacity);

        let (mut short_sma_line, mut long_sma_line, mut medprice_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &optional_outputs.unwrap_or(&[false, false, false]),
            short_sma_line: capacity,
            long_sma_line: capacity,
            medprice_line: capacity
        );

        cycle_ao(
            inputs[0], //high
            inputs[1], //low
            self.multipliers,
            &mut self.state,
            &mut ao_line,
            (&mut short_sma_line, &mut long_sma_line, &mut medprice_line),
        );

        Ok(vec![ao_line, short_sma_line, long_sma_line, medprice_line])
    }
}
impl State {
    pub fn new(short_sum: f64, long_sum: f64) -> Self {
        State {
            short_sum,
            long_sum,
            buffer: Buffer::new(LONG_PERIOD),
        }
    }
    pub fn init_state(
        inputs: (&[f64], &[f64]),
        medprice_line: &mut [f64],
        short_sma_line: &mut [f64],
    ) -> Self {
        let (high, low) = inputs;
        let mut state = Self::new(0.0, 0.0);
        let (multiplier, _) = multiplier((SHORT_PERIOD, LONG_PERIOD));
        for (i, (&high_val, &low_val)) in high.iter().zip(low.iter()).take(LONG_PERIOD).enumerate()
        {
            let med_price = calc_medprice(high_val, low_val);
            let mut sma = 0.0;
            state.buffer.push(med_price);
            state.long_sum += med_price;
            if i >= SHORT_PERIOD {
                let prev_medprice = calc_medprice(high[i - SHORT_PERIOD], low[i - SHORT_PERIOD]);
                sma = sma_calc(
                    &mut state.short_sum,
                    &med_price,
                    &prev_medprice,
                    &multiplier,
                );
            } else {
                state.short_sum += med_price;
            }
            crate::init_store_optional_outputs!(i, high.len(),
                medprice_line => med_price,
                short_sma_line => sma
            );
        }
        state
    }
    #[inline(always)]
    pub unsafe fn calc_unchecked(
        &mut self,
        values: (f64, f64),
        multipliers: (f64, f64),
    ) -> (f64, f64, f64, f64) {
        let (short_multiplier, long_multiplier) = multipliers;

        let (high, low) = values;

        let med_price = calc_medprice(high, low);

        let long_sma = sma_calc(
            &mut self.long_sum,
            &med_price,
            &self.buffer.push_with_info_unchecked(med_price),
            &long_multiplier,
        );
        let short_sma = sma_calc(
            &mut self.short_sum,
            &med_price,
            &self.buffer.get_by_period(SHORT_PERIOD),
            &short_multiplier,
        );

        (short_sma - long_sma, short_sma, long_sma, med_price)
    }
    #[inline(always)]
    pub fn calc(&mut self, values: (f64, f64), multipliers: (f64, f64)) -> (f64, f64, f64, f64) {
        let (short_multiplier, long_multiplier) = multipliers;

        let (high, low) = values;

        let med_price = calc_medprice(high, low);

        let long_sma = if let Some(prev) = self.buffer.push_with_info(med_price) {
            sma_calc(&mut self.long_sum, &med_price, &prev, &long_multiplier)
        } else {
            0.0
        };

        let short_sma = sma_calc(
            &mut self.short_sum,
            &med_price,
            &self.buffer.get_by_period(SHORT_PERIOD),
            &short_multiplier,
        );

        (short_sma - long_sma, short_sma, long_sma, med_price)
    }
}
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
/// Returns the minimum amount of data required for the AO indicator.
///
/// # Arguments
///
/// * `_options` - A slice containing the options for the AO calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(_options: &[f64]) -> usize {
    35 // long_period
}

/// Calculates the output length for the AO indicator based on the data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the AO calculation (unused; AO has no configurable options).
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Awesome Oscillator (AO) indicator over the full input dataset.
///
/// Uses fixed periods of 5 (short SMA) and 34 (long SMA) applied to the median price.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
///
/// # Arguments
///
/// * `inputs` - Array of 2 input price slices (see Inputs above).
/// * `_options` - Unused; AO has no configurable options.
/// * `optional_outputs` - Pass `Some(&[true, false, false])` to enable individual
///   optional outputs (`short_sma`, `long_sma`, `medprice`); `None` disables all.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the `ao` line,
/// `outputs[1]` is the optional `short_sma` line, `outputs[2]` is the optional `long_sma` line,
/// and `outputs[3]` is the optional `medprice` line (each empty if not requested).
/// `state` can be passed to `IndicatorState::batch_indicator` to continue streaming.
///
/// Returns `Err(IndicatorError)` if inputs are too short.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_inputs(inputs, min_data(_options))?;

    let high = inputs[0];
    let low = inputs[1];

    let (mut ao_line, (mut short_sma_line, mut long_sma_line, mut medprice_line)) = {
        let capacity = output_length(high.len(), _options);
        let short_capacity = sma_output_length(high.len(), &[SHORT_PERIOD as f64]);

        (
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &optional_outputs.unwrap_or(&[false, false, false]),
                short_sma_line: short_capacity,
                long_sma_line: capacity,
                medprice_line: high.len()
            ),
        )
    };
    let multipliers = multiplier((SHORT_PERIOD, LONG_PERIOD));

    let mut state = State::init_state((high, low), &mut medprice_line, &mut short_sma_line);
    let optional_outputs = {
        let offsets = crate::slice_outputs_start!(ao_line.len(), medprice_line, short_sma_line);
        (
            &mut short_sma_line[offsets.1..],
            long_sma_line.as_mut_slice(),
            &mut medprice_line[offsets.0..],
        )
    };
    let (high, low) = { (&high[LONG_PERIOD..], &low[LONG_PERIOD..]) };
    cycle_ao(
        high,
        low,
        multipliers,
        &mut state,
        &mut ao_line,
        optional_outputs,
    );

    Ok((
        vec![ao_line, short_sma_line, long_sma_line, medprice_line],
        IndicatorState { state, multipliers },
    ))
}

/// Performs the main calculation loop for the AO indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `multipliers` - The precomputed SMA multipliers for the short and long periods.
/// * `state` - A mutable reference to the current `State` (buffer, short sum, long sum).
/// * `ao_line` - A mutable slice for storing the resulting AO line values.
/// * `out_vecs` - A tuple of mutable slices for optional outputs: short SMA, long SMA, and median price lines.
fn cycle_ao(
    high: &[f64],
    low: &[f64],
    multipliers: (f64, f64),
    state: &mut State,
    ao_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (short_sma_line, long_sma_line, medprice_line) = out_vecs;
    let (has_optional, want_short, want_long, want_medprice) =
        crate::calc_want_flags!(short_sma_line, long_sma_line, medprice_line);

    for i in 0..high.len() {
        let values = unsafe { (*high.get_unchecked(i), *low.get_unchecked(i)) };

        let (ao, short_sma, long_sma, medprice) =
            unsafe { state.calc_unchecked(values, multipliers) };
        unsafe { *ao_line.get_unchecked_mut(i) = ao };

        if has_optional {
            crate::store_optional_outputs!(i,
                want_short, short_sma_line => short_sma,
                want_long, long_sma_line => long_sma,
                want_medprice, medprice_line => medprice
            );
        }
    }
}
#[inline(always)]
pub fn calc(
    state: &mut State,
    values: (f64, f64),
    multipliers: (f64, f64),
) -> (f64, f64, f64, f64) {
    state.calc(values, multipliers)
}
#[inline(always)]
pub unsafe fn calc_unchecked(
    state: &mut State,
    values: (f64, f64),
    multipliers: (f64, f64),
) -> (f64, f64, f64, f64) {
    state.calc_unchecked(values, multipliers)
}

#[inline(always)]
pub fn multiplier(periods: (usize, usize)) -> (f64, f64) {
    (sma_multiplier(periods.0), sma_multiplier(periods.1))
}
