use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::typprice::calc as calc_typprice;
use crate::ring_buffer::multi_buffer::multi_buffer::{MultiBuffer as Buffer, RingBuffer};
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
pub const INPUTS_WIDTH: usize = 4;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::mfi_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::mfi_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::mfi_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::mfi_simd::indicator_by_options as indicator;
}

/// Returns information about the Money Flow Index (MFI) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the MFI indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "mfi",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Volume,
        full_name: "Money Flow Index",
        inputs: &["high", "low", "close", "volume"],
        options: &["period"],
        outputs: &["mfi"],
        optional_outputs: &["typprice"],
    }
}
/*#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
}*/
impl TIndicatorState<4> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut mfi_line, mut typprice_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    typprice_line: capacity
                ),
            )
        };

        cycle_mfi(
            (inputs[0], inputs[1], inputs[2], inputs[3]),
            self,
            &mut mfi_line,
            &mut typprice_line,
        );

        Ok(vec![mfi_line, typprice_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub buffer: Buffer<2>,
    pub typprice: f64,
    pub pos_sum: f64,
    pub neg_sum: f64,
}
impl IndicatorState {
    pub fn new(buffer: Buffer<2>, typprice: f64, pos_sum: f64, neg_sum: f64) -> Self {
        Self {
            buffer,
            typprice,
            pos_sum,
            neg_sum,
        }
    }

    pub fn init_state(
        inputs: (&[f64], &[f64], &[f64], &[f64]),
        period: usize,
        typprice_line: &mut [f64],
    ) -> Self {
        let (high, low, close, volume) = inputs;
        let mut state = Self {
            typprice: calc_typprice(&high[0], &low[0], &close[0]),
            pos_sum: 0.0,
            neg_sum: 0.0,
            buffer: Buffer::new(period),
        };

        for i in 0..period {
            state.calc(&high[i], &low[i], &close[i], &volume[i]);
            crate::init_store_optional_outputs!(i, high.len(),
                typprice_line => state.typprice
            );
        }
        state
    }
    /// Calculates the Money Flow Index (MFI) for the current data point.
    ///
    /// # Arguments
    ///
    /// * `high` - The current high price.
    /// * `low` - The current low price.
    /// * `close` - The current close price.
    /// * `volume` - The current volume data.
    /// * `prev_high` - The previous high price.
    /// * `prev_low` - The previous low price.
    /// * `prev_close` - The previous close price.
    /// * `positive_flow` - The previous positive money flow.
    /// * `negative_flow` - The previous negative money flow.
    /// * `period` - The period for the MFI calculation.
    /// * `index` - The current index in the loop.
    ///
    /// # Returns
    ///
    /// A tuple containing the calculated MFI, new positive money flow, and new negative money flow values.
    #[inline(always)]
    pub fn calc(&mut self, high: &f64, low: &f64, close: &f64, volume: &f64) -> f64 {
        let prev_typprice = self.typprice;
        self.typprice = calc_typprice(high, low, close);

        let price_change = self.typprice - prev_typprice;

        let (pos_flow, neg_flow) = if price_change > 0.0 {
            (self.typprice * volume, 0.0)
        } else if price_change < 0.0 {
            (0.0, self.typprice * volume)
        } else {
            (0.0, 0.0)
        };

        if let Some([pos_flow_old, neg_flow_old]) = self.buffer.push_with_info([pos_flow, neg_flow])
        {
            self.pos_sum += pos_flow - pos_flow_old;
            self.neg_sum += neg_flow - neg_flow_old;
        } else {
            self.pos_sum += pos_flow;
            self.neg_sum += neg_flow
        }

        self.pos_sum / (self.pos_sum + self.neg_sum).max(f64::EPSILON) * 100.0
    }
    #[inline(always)]
    pub unsafe fn calc_unchecked(
        &mut self,
        high: &f64,
        low: &f64,
        close: &f64,
        volume: &f64,
    ) -> f64 {
        let prev_typprice = self.typprice;
        self.typprice = calc_typprice(high, low, close);

        let price_change = self.typprice - prev_typprice;
        let money_flow = self.typprice * volume;

        let (pos_flow, neg_flow) = if price_change > 0.0 {
            (money_flow, 0.0)
        } else if price_change < 0.0 {
            (0.0, money_flow)
        } else {
            (0.0, 0.0)
        };

        let [pos_flow_old, neg_flow_old] =
            self.buffer.push_with_info_unchecked([pos_flow, neg_flow]);
        self.pos_sum += pos_flow - pos_flow_old;
        self.neg_sum += neg_flow - neg_flow_old;

        self.pos_sum / (self.pos_sum + self.neg_sum).max(f64::EPSILON) * 100.0
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the MFI indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the MFI calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the MFI calculation.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Money Flow Index (MFI) for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `options` - A slice containing the options for the MFI calculation.
///
/// # Returns
///
/// A vector of vectors containing the MFI line.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;

    validate_inputs(inputs, min_data(options))?;
    let (mut mfi_line, mut typprice_line) = {
        let len = inputs[0].len();
        let capacity = output_length(len, options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false],
                typprice_line: len
            ),
        )
    };
    let offset = crate::slice_outputs_start!(mfi_line.len(), typprice_line);
    let mut state = IndicatorState::init_state(
        (inputs[0], inputs[1], inputs[2], inputs[3]),
        period,
        &mut typprice_line,
    );
    // Perform the main MFI calculation
    cycle_mfi(
        (&inputs[0][period..], &inputs[1][period..], &inputs[2][period..], &inputs[3][period..]),
        &mut state,
        &mut mfi_line,
        &mut typprice_line[offset..],
    );

    Ok((vec![mfi_line, typprice_line], state))
}

/// Performs the main calculation loop for the MFI indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `volume` - A slice of volume data.
/// * `period` - The period for the MFI calculation.
/// * `mfi_line` - A mutable reference to a vector for storing the MFI line.
/// * `output_vectors` - A mutable reference to an array of optional output vectors.
/// * `prev_state` - An optional tuple containing the previous state values.
/// * `start` - The start index for the main loop.
fn cycle_mfi(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &mut IndicatorState,
    mfi_line: &mut [f64],
    typprice_line: &mut [f64],
) {
    let (high, low, close, volume) = inputs;
    let (_, want_typprice) = crate::calc_want_flags!(typprice_line);

    for i in 0..high.len() {
        unsafe {
            *mfi_line.get_unchecked_mut(i) = state.calc_unchecked(
                high.get_unchecked(i),
                low.get_unchecked(i),
                close.get_unchecked(i),
                volume.get_unchecked(i),
            );
        }
        crate::store_optional_outputs!(i,
            want_typprice, typprice_line => state.typprice
        );
    }
}
