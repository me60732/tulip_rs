use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub(crate) sum: f64,
    pub(crate) multiplier: f64,
    pub(crate) state: std::marker::PhantomData<S>,
}
impl State<Cold> {
    pub fn new(sum: f64, period: usize) -> Self {
        let multiplier = multiplier(period);
        Self {
            sum,
            multiplier,
            state: std::marker::PhantomData,
        }
    }
    pub(crate) fn into_warm(self) -> State<Warm> {
        State {
            sum: self.sum,
            multiplier: self.multiplier,
            state: std::marker::PhantomData,
        }
    }
    pub fn init_state(real: &[f64], period: usize) -> State<Warm> {
        let mut sum = 0.0;
        for i in 0..period {
            sum += real[i];
        }
        State {
            sum,
            multiplier: multiplier(period),
            state: std::marker::PhantomData,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = f64;
    #[inline(always)]
    fn calc<'a>(&mut self, (real, prev_real): Self::Inputs<'a>) -> Self::Outputs {
        self.sum += real - prev_real;
        self.sum * self.multiplier
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    //state: State,
    state: State<Warm>,
    period: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State<Warm>, period: usize) -> Self {
        Self {
            real: real[real.len() - period..].to_vec(),
            //state: State::new(sum, multiplier),
            state,
            period,
        }
    }
}
impl TIndicatorState<INPUTS> for IndicatorState {
    /// Continues the Simple Moving Average (SMA) calculation from the stored state.
    ///
    /// # Arguments
    ///
    /// * `inputs` - An array of one input slice: `[real]`.
    /// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
    ///
    /// # Returns
    ///
    /// `Result<Vec<Vec<f64>>, IndicatorError>` — a vector of output vectors containing the SMA line.
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let mut sma_line: Vec<f64> = crate::uninit_vec!(f64, inputs[0].len());
        self.real.extend_from_slice(inputs[0]);
        cycle_sma(&self.real, self.period, &mut sma_line, &mut self.state);
        self.real.drain(..self.real.len() - self.period);

        Ok(vec![sma_line])
    }
}
pub struct Sma;
impl Indicator<INPUTS, OPTIONS> for Sma {
    type IndicatorState = IndicatorState;
    /// Returns information about the Simple Moving Average (SMA) indicator.
    ///
    /// # Returns
    ///
    /// An `Info` struct containing metadata about the SMA indicator.
    const INFO: Info = Info {
        name: "sma",
        full_name: "Simple Moving Average",
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["period"],
        outputs: &["sma"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "sma",
            label: "SMA",
            display_type: DisplayType::Overlay,
            outputs: &["sma"],
        }],
    };
    /// Calculates the Simple Moving Average (SMA) indicator over the full input dataset.
    ///
    /// # Inputs
    ///
    /// * `inputs[0]` — real (source) values
    ///
    /// # Options
    ///
    /// * `options[0]` — period
    ///
    /// # Arguments
    ///
    /// * `inputs` - Array of input price slices (see Inputs above).
    /// * `options` - Array of indicator options (see Options above).
    /// * `_optional_outputs` - Unused; this indicator has no optional outputs.
    ///
    /// # Returns
    ///
    /// `Ok((outputs, state))` where:
    /// - `outputs[0]` — `sma`
    ///
    /// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
    /// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;

        let real = inputs[0];
        let mut state = State::init_state(real, period);
        let mut sma_line = {
            let capacity = Self::output_length(real.len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        cycle_sma(real, period, &mut sma_line, &mut state);

        Ok((vec![sma_line], IndicatorState::new(real, state, period)))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::sma_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Sma {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::sma_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

/// Performs the main calculation loop for the SMA indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `period` - The period for the SMA calculation.
/// * `sma_line` - A mutable slice for storing the SMA output values.
/// * `sum` - A mutable reference to the running sum of the window values.
/// * `multiplier` - A reference to the precomputed multiplier (1/period).
fn cycle_sma(real: &[f64], period: usize, sma_line: &mut [f64], state: &mut State<Warm>) {
    //let multiplier = &multiplier(period);
    for (j, i) in (period..real.len()).enumerate() {
        let sma = unsafe { state.calc((*real.get_unchecked(i), *real.get_unchecked(j))) };
        unsafe { *sma_line.get_unchecked_mut(j) = sma };
    }
}
/// Calculates the current value of the Simple Moving Average (SMA) indicator.
///
/// # Arguments
///
/// * `sum` - A mutable reference to the running sum of the window values.
/// * `value` - The current input value entering the window.
/// * `prev_value` - The oldest input value leaving the window.
/// * `multiplier` - A reference to the precomputed multiplier (1/period).
///
/// # Returns
///
/// The current SMA value.
#[inline(always)]
pub(crate) fn calc(sum: &mut f64, value: &f64, prev_value: &f64, multiplier: &f64) -> f64 {
    let mut s = *sum;
    s = s + (value - prev_value);
    *sum = s;
    s * multiplier
}
/// Calculates the multiplier for the Simple Moving Average (SMA) indicator.
///
/// # Arguments
///
/// * `period` - The period for the SMA calculation.
///
/// # Returns
///
/// The multiplier for the SMA calculation.
#[inline(always)]
pub fn multiplier(period: usize) -> f64 {
    1.0 / period as f64
}
