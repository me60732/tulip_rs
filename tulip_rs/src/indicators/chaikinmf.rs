use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::simd::Simd;
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 4;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

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
        inputs: &[&[f64]; INPUTS],
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
pub type IndicatorState = State<Warm>;
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub buffer: Buffer<S, Simd<f64, 2>>,
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
impl State<Cold> {
    pub fn new(period: usize) -> State<Cold> {
        Self {
            buffer: Buffer::<Cold, Simd<f64, 2>>::new(period),
            sums: Simd::splat(0.0),
        }
    }

    pub fn init_state(inputs: (&[f64], &[f64], &[f64], &[f64]), period: usize) -> State<Warm> {
            let (high, low, close, volume) = inputs;
            let mut buffer = Buffer::new(period);
            let mut sums = Simd::splat(0.0);
            let mut i = 0;
            while !buffer.is_full() {
                let mfv = calc_mfv((high[i], low[i], close[i], volume[i]));
                let values = Simd::from_array([mfv, volume[i]]);
                buffer.push(values);
                sums += values;
    
                i += 1;
            }
            State {
                buffer: buffer.into_full(),
                sums,
            }
        }
}

impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64, f64);
    type Outputs = f64;

    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        let mfv = calc_mfv(inputs);
        let values = Simd::from_array([mfv, inputs.3]);
        let old_vals = self.buffer.push_with_info(values);
        self.sums += values - old_vals;
        //let [mfv_sum, vol_sum] = self.sums.as_array();
        self.sums[0] / self.sums[1]
    }
}

#[inline(always)]
fn calc_mfv((high, low, close, volume): (f64, f64, f64, f64)) -> f64 {
    ((close - low) - (high - close)) / (high - low).max(f64::EPSILON) * volume
}
/// Performs the main calculation loop for the Chaikin Money Flow indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of `(high, low, close, volume)` slices for the bars to process.
/// * `state` - A mutable reference to the current [`IndicatorState`].
/// * `cmf_line` - A mutable slice for storing the CMF output values.
fn cycle_mfi(
    (high, low, close, volume): (&[f64], &[f64], &[f64], &[f64]),
    state: &mut State<Warm>,
    cmf_line: &mut [f64],
) {
    for i in 0..volume.len() {
        unsafe {
            *cmf_line.get_unchecked_mut(i) = state.calc((
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
                *volume.get_unchecked(i),
            ));
        }
    }
}

pub struct ChaikinMf;
impl Indicator<INPUTS, OPTIONS> for ChaikinMf {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
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

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
        let [high, low, close, volume] = *inputs;
        validate_inputs(inputs, Self::min_data(options))?;
        let mut cmf_line = {
            let len = inputs[0].len();
            let capacity = Self::output_length(len, options);
            crate::uninit_vec!(f64, capacity)
        };

        let mut state = State::init_state((high, low, close, volume), period);
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
}
