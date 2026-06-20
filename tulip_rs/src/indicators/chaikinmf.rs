use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
use crate::ring_buffer::single_buffer::generic_buffer::{RingBuffer, SimdBuffer};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::simd::Simd;
/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 4;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::chaikinmf_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::chaikinmf_simd::indicator_by_options;

#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::chaikinmf_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::chaikinmf_simd::indicator_by_options as indicator;
}

/// Returns information about the Chaikin Money Flow (CMF) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the CMF indicator.
pub const INFO: Info = Info {
    name: "chaikinmf",
    indicator_type: IndicatorType::Volume,
    full_name: "Chaikin Money Flow",
    inputs: &["high", "low", "close", "volume"],
    options: &["period"],
    outputs: &["cmf"],
    optional_outputs: &[],
    display_groups: &[DisplayGroup {
        offset: None,
        id: "cmf",
        label: "Chaikin Money Flow",
        display_type: DisplayType::Indicator,
        outputs: &["cmf"],
    }],
};

impl TIndicatorState<4> for IndicatorState {
    /// Runs the Chaikin Money Flow calculation over a new batch of input bars,
    /// updating the rolling state in place.
    ///
    /// # Arguments
    /// * `inputs` - `[high, low, close, volume]` slices for the new bars.
    /// * `_optional_outputs` - Unused; CMF has no optional output lines.
    ///
    /// # Returns
    /// `Ok(outputs)` where `outputs[0]` is the CMF series for the batch.
    /// Returns `Err(IndicatorError)` if any input slice is empty.
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let [high, low, close, volume] = inputs;
        let mut cmf_line = {
            let capacity = inputs[0].len();
            crate::uninit_vec!(f64, capacity)
        };
        cycle_mfi((high, low, close, volume), self, &mut cmf_line);

        Ok(vec![cmf_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub buffer: SimdBuffer<2>,
    #[serde(
        serialize_with = "serialize_f64x2",
        deserialize_with = "deserialize_f64x2"
    )]
    pub sums: Simd<f64, 2>,
}
fn serialize_f64x2<S>(data: &Simd<f64, 2>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    data.to_array().serialize(serializer)
}

fn deserialize_f64x2<'de, D>(deserializer: D) -> Result<Simd<f64, 2>, D::Error>
where
    D: Deserializer<'de>,
{
    let array = <[f64; 2]>::deserialize(deserializer)?;
    Ok(Simd::from_array(array))
}
impl IndicatorState {
    pub fn new(period: usize) -> Self {
        Self {
            buffer: SimdBuffer::new(period),
            sums: Simd::splat(0.0),
        }
    }
    pub fn init_state(inputs: (&[f64], &[f64], &[f64], &[f64]), period: usize) -> Self {
        let (high, low, close, volume) = inputs;
        let mut state = Self::new(period);

        let mut i = 0;
        while !state.buffer.is_full() {
            state.calc(high[i], low[i], close[i], volume[i]);
            i += 1;
        }
        state
    }
    /// Calculates Chaikin Money Flow for the current data point, updating the
    /// rolling ring buffer and running sums.
    ///
    /// # Arguments
    ///
    /// * `high` - The current high price.
    /// * `low` - The current low price.
    /// * `close` - The current close price.
    /// * `volume` - The current volume.
    ///
    /// # Returns
    ///
    /// The CMF value: `mfv_sum / vol_sum` over the look-back window,
    /// where `mfv = ((close - low) - (high - close)) / (high - low) * volume`.
    #[inline(always)]
    pub fn calc(&mut self, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let mfv = ((close - low) - (high - close)) / (high - low) * volume;
        let values = Simd::from_array([mfv, volume]);
        if let Some(old_vals) = self.buffer.push_with_info(values) {
            self.sums += values - old_vals;
        } else {
            self.sums += values;
        }
        //let [mfv_sum, vol_sum] = self.sums.as_array();
        self.sums[0] / self.sums[1]
    }
    /// Calculates Chaikin Money Flow for the current data point without bounds
    /// checking, assuming the ring buffer is already full.
    ///
    /// # Arguments
    ///
    /// * `high` - The current high price.
    /// * `low` - The current low price.
    /// * `close` - The current close price.
    /// * `volume` - The current volume.
    ///
    /// # Returns
    ///
    /// The CMF value: `mfv_sum / vol_sum` over the look-back window.
    ///
    /// # Safety
    /// Caller must ensure the ring buffer has been fully seeded (i.e., `is_full()` is true).
    #[inline(always)]
    pub unsafe fn calc_unchecked(&mut self, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let mfv = ((close - low) - (high - close)) / (high - low).max(f64::EPSILON) * volume;
        let values = Simd::from_array([mfv, volume]);
        let old_vals = self.buffer.push_with_info_unchecked(values);
        self.sums += values - old_vals;
        //let [mfv_sum, vol_sum] = self.sums.as_array();
        self.sums[0] / self.sums[1]
    }
}
/// Returns the minimum number of input bars required for the Chaikin Money Flow indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options (`[period]`).
///
/// # Returns
///
/// `period + 1` — one bar is consumed seeding the ring buffer, then each
/// subsequent bar produces one output value.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Calculates the number of output values produced by the Chaikin Money Flow indicator.
///
/// # Arguments
///
/// * `data_len` - The number of input bars.
/// * `options` - A slice containing the indicator options (`[period]`).
///
/// # Returns
///
/// `data_len - min_data(options) + 1`.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates Chaikin Money Flow over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
/// * `inputs[3]` — volume
///
/// # Options
///
/// * `options[0]` — look-back period
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `_optional_outputs` - Unused; CMF has no optional output lines.
///
/// # Returns
///
/// `Ok((outputs, state))` where:
/// - `outputs[0]` — CMF series (`mfv_sum / vol_sum` for each bar)
///
/// `state` can be passed to [`IndicatorState::batch_indicator`] for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let [high, low, close, volume] = *inputs;
    validate_inputs(inputs, min_data(options))?;
    let mut cmf_line = {
        let len = inputs[0].len();
        let capacity = output_length(len, options);
        crate::uninit_vec!(f64, capacity)
    };

    let mut state = IndicatorState::init_state((high, low, close, volume), period);
    // Perform the main MFI calculation
    cycle_mfi(
        (
            &high[period..],
            &low[period..],
            &close[period..],
            &volume[period..],
        ),
        &mut state,
        &mut cmf_line,
    );

    Ok((vec![cmf_line], state))
}

/// Performs the main calculation loop for the Chaikin Money Flow indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of `(high, low, close, volume)` slices for the bars to process.
/// * `state` - A mutable reference to the current [`IndicatorState`].
/// * `cmf_line` - A mutable slice for storing the CMF output values.
fn cycle_mfi(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    state: &mut IndicatorState,
    cmf_line: &mut [f64],
) {
    let (high, low, close, volume) = inputs;

    for i in 0..volume.len() {
        unsafe {
            *cmf_line.get_unchecked_mut(i) = state.calc_unchecked(
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
                *volume.get_unchecked(i),
            );
        }
    }
}
