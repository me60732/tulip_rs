use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 2;
/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    close: Vec<f64>,
    volume: Vec<f64>,
    state: State<Warm>,
    period: usize,
}
impl IndicatorState {
    pub fn new(close: &[f64], volume: &[f64], state: State<Warm>, period: usize) -> Self {
        Self {
            close: close[close.len() - period..].to_vec(),
            volume: volume[volume.len() - period..].to_vec(),
            state,
            period,
        }
    }
}

impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        self.close.extend_from_slice(inputs[0]);
        self.volume.extend_from_slice(inputs[1]);

        let mut vwma_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(
            &self.close,
            &self.volume,
            self.period,
            &mut self.state,
            &mut vwma_line,
        );

        self.close.drain(..self.close.len() - self.period);
        self.volume.drain(..self.volume.len() - self.period);

        Ok(vec![vwma_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State<S = Cold> {
    pub sum: f64,
    pub vol_sum: f64,
    pub(crate) state: std::marker::PhantomData<S>,
}
impl State<Cold> {
    pub fn new(sum: f64, vol_sum: f64) -> Self {
        State {
            sum,
            vol_sum,
            state: std::marker::PhantomData,
        }
    }
    /// Initializes VWMA by computing the initial numerator and denominator sums over the first period,
    /// then computing the first VWMA value.
    pub fn init_state(period: usize, close: &[f64], volume: &[f64]) -> State<Warm> {
        let mut sum = 0.0;
        let mut vol_sum = 0.0;
        for i in 0..period {
            sum += close[i] * volume[i];
            vol_sum += volume[i];
        }
        State {
            sum,
            vol_sum,
            state: std::marker::PhantomData,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64, f64);
    type Outputs = f64;

    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (close, volume, prev_close, prev_volume): Self::Inputs<'a>,
    ) -> Self::Outputs {
        // Add new bar's contribution.
        self.sum += (close * volume) - (prev_close * prev_volume);
        self.vol_sum += volume - prev_volume;

        if self.vol_sum == 0.0 {
            return 0.0;
        }
        self.sum / self.vol_sum
    }
}

/// Iterates over the close and volume arrays and writes VWMA values into `vwma_line`.
///
/// # Arguments
///
/// * `close` - The full close price input slice.
/// * `volume` - The full volume input slice.
/// * `period` - The period for the VWMA calculation.
/// * `state` - Mutable reference to the rolling `State` (weighted sum and volume sum).
/// * `vwma_line` - Mutable output slice for VWMA values.
fn cycle(
    close: &[f64],
    volume: &[f64],
    period: usize,
    state: &mut State<Warm>,
    vwma_line: &mut [f64],
) {
    for (j, i) in (period..close.len()).enumerate() {
        unsafe {
            *vwma_line.get_unchecked_mut(j) = state.calc((
                *close.get_unchecked(i),
                *volume.get_unchecked(i),
                *close.get_unchecked(j),
                *volume.get_unchecked(j),
            ))
        };
    }
}

pub struct Vwma;
impl Indicator<INPUTS, OPTIONS> for Vwma {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "vwma",
        full_name: "Volume Weighted Moving Average",
        indicator_type: IndicatorType::Trend,
        // Two inputs: close and volume.
        inputs: &["close", "volume"],
        // One option: period.
        options: &["period"],
        outputs: &["vwma"],
        // No optional outputs.
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "vwma",
            label: "VWMA",
            display_type: DisplayType::Overlay,
            outputs: &["vwma"],
        }],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;

        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let close = inputs[0];
        let volume = inputs[1];

        let mut vwma_line = {
            let capacity = Self::output_length(close.len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        // Initialize state.
        let mut state = State::init_state(period, close, volume);

        // Process from index = period (first full window is available).
        cycle(close, volume, period, &mut state, &mut vwma_line);

        Ok((
            vec![vwma_line],
            IndicatorState::new(close, volume, state, period),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::vwma_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Vwma {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::vwma_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
