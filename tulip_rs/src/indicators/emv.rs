use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::medprice::calc as calc_medprice;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 3;
pub const OPTIONS_WIDTH: usize = 0;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::emv_simd::indicator_by_assets;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::emv_simd::indicator_by_assets as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    prev_medprice: f64,
}
impl IndicatorState {
    pub fn new(prev_medprice: f64) -> Self {
        Self { prev_medprice }
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut emv_line, mut medprice_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    medprice_line: capacity
                ),
            )
        };
        let [high, low, volume] = inputs;
        // Perform the main EMV calculation
        cycle_emv(
            high,
            low,
            volume,
            &mut self.prev_medprice,
            &mut emv_line,
            &mut medprice_line,
        );

        Ok(vec![emv_line, medprice_line])
    }
}
/// Returns information about the Ease of Movement (EMV) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the EMV indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "emv",
        display_type: DisplayType::Indicator,
        indicator_type: IndicatorType::Momentum,
        full_name: "Ease of Movement",
        inputs: &["high", "low", "volume"],
        options: &[],
        outputs: &["emv"],
        optional_outputs: &["medprice"],
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the EMV indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the EMV calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(_options: &[f64]) -> usize {
    2 // The EMV calculation requires at least two data points
}

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the EMV calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Ease of Movement (EMV) for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `options` - A slice containing the options for the EMV calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the EMV line.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_inputs(inputs, min_data(_options))?;

    let [high, low, volume] = inputs;
    let mut prev_medprice = calc_medprice(high[0], low[0]);

    let (mut emv_line, mut medprice_line);
    {
        let capacity = output_length(high.len(), _options);
        let medprice_capacity = high.len();
        emv_line = crate::uninit_vec!(f64, capacity);
        medprice_line = crate::init_optional_outputs_eff!(
            optional_outputs, &[false],
            medprice_line: medprice_capacity
        );
        crate::init_store_optional_outputs!(0, medprice_capacity,
            medprice_line => prev_medprice
        );
    }
    let medprice = {
        let offset = crate::slice_outputs_start!(emv_line.len(), medprice_line);
        &mut medprice_line[offset..]
    };
    let (high, low, volume) = (&high[1..], &low[1..], &volume[1..]);
    // Perform the main EMV calculation
    cycle_emv(
        high,
        low,
        volume,
        &mut prev_medprice,
        &mut emv_line,
        medprice,
    );

    Ok((
        vec![emv_line, medprice_line],
        IndicatorState { prev_medprice },
    ))
}

/// Performs the main calculation loop for the EMV indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `volume` - A slice of volume data.
/// * `period` - The period for the EMV calculation.
/// * `start` - The starting index for the calculation.
/// * `emv_line` - A mutable reference to a vector for storing the EMV line.
fn cycle_emv(
    high: &[f64],
    low: &[f64],
    volume: &[f64],
    prev_medprice: &mut f64,
    emv_line: &mut [f64],
    medprice_line: &mut [f64],
) {
    let (_, want_medprice) = crate::calc_want_flags!(medprice_line);

    for i in 0..high.len() {
        unsafe {
            *emv_line.get_unchecked_mut(i) = calc(*high.get_unchecked(i), *low.get_unchecked(i), *volume.get_unchecked(i), prev_medprice);
        }
        crate::store_optional_outputs!(i,
            want_medprice, medprice_line => *prev_medprice);
    }
}

/// Calculates the Ease of Movement (EMV) for the current data point.
///
/// # Arguments
///
/// * `high` - The current high price.
/// * `low` - The current low price.
/// * `volume` - The current volume.
/// * `period` - The period for the EMV calculation.
///
/// # Returns
///
/// The calculated EMV value.
///
#[inline(always)]
pub fn calc(high: f64, low: f64, volume: f64, prev_medprice: &mut f64) -> f64 {
    let medprice = calc_medprice(high, low);
    let distance_moved = medprice - *prev_medprice;
    let hl_diff = (high - low).max(f64::EPSILON);
    let volume_safe = volume.max(f64::EPSILON);
    *prev_medprice = medprice;

    distance_moved * 10000.0 * hl_diff / volume_safe
}
