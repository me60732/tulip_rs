use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::indicators::max::State as MaxState;
use crate::indicators::min::State as MinState;
use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State<Warm>,
    real: Vec<f64>,
    period: usize,
}
impl IndicatorState {
    pub fn new(state: State<Warm>, real: &[f64], period: usize) -> Self {
        Self {
            state,
            period,
            real: real[real.len() - period - 1..].to_vec(),
        }
    }
}

impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.real.extend_from_slice(inputs[0]);

        let mut vhf_line = crate::uninit_vec!(f64, inputs[0].len());
        cycle(&self.real, self.period, &mut self.state, &mut vhf_line);

        self.real.drain(..self.real.len() - self.period - 1);
        Ok(vec![vhf_line])
    }
}
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub buffer: Buffer<S>,
    pub min_state: MinState<S>,
    pub max_state: MaxState<S>,
    pub prev_real: f64,
    pub sum: f64,
}

impl TState for State<Warm> {
    type Inputs<'a> = (f64, (&'a [f64], usize, (usize, usize)));
    type Outputs = f64;
    #[inline(always)]
    fn calc(&mut self, (value, inputs): Self::Inputs<'_>) -> f64 {
        let new = (value - self.prev_real).abs();
        let old = self.buffer.push_with_info(new);
        self.sum += new - old;
        self.prev_real = value;
        let (min, _) = self.min_state.calc(inputs);
        let (max, _) = self.max_state.calc(inputs);
        (max - min) / self.sum.max(f64::EPSILON)
    }
    #[inline(always)]
    unsafe fn calc_unchecked<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        self.calc_chuncked_unchecked::<4>(inputs)
    }
}
impl State<Warm> {
    #[inline(always)]
    pub unsafe fn calc_chuncked_unchecked<const N: usize>(
        &mut self,
        (value, inputs): (f64, (&[f64], usize, (usize, usize))),
    ) -> f64 {
        let new = (value - self.prev_real).abs();
        let old = self.buffer.push_with_info(new);
        self.sum += new - old;
        self.prev_real = value;
        let (min, _) = self.min_state.calc_chuncked_unchecked::<N>(inputs);
        let (max, _) = self.max_state.calc_chuncked_unchecked::<N>(inputs);
        (max - min) / self.sum.max(f64::EPSILON)
    }
}
impl State<Cold> {
    pub fn new(
        min: (f64, usize),
        max: (f64, usize),
        sum: f64,
        prev_real: f64,
        period: usize,
    ) -> Self {
        State {
            min_state: MinState::new(min.0, min.1),
            max_state: MaxState::new(max.0, max.1),
            prev_real,
            sum,
            buffer: Buffer::new(period),
        }
    }

    pub fn init_state(real: &[f64], period: usize, indicator_line: &mut [f64]) -> State<Warm> {
        // trail = period forces full window scan on first calc()
        let mut min_state = MinState::new(real[0], period).into_warm();
        let mut max_state = MaxState::new(real[0], period).into_warm();
        let mut sum = 0.0;
        let mut prev_real = real[0];
        let mut buffer = Buffer::new(period);

        // Build sum and buffer FIRST, with abs diffs
        for i in 1..=period {
            let abs_diff = (real[i] - prev_real).abs();
            buffer.push(abs_diff);
            sum += abs_diff;
            prev_real = real[i];
        }

        // THEN advance min/max states to i=period
        let min = min_state.calc((real, period, (period, period - 1))).0;
        let max = max_state.calc((real, period, (period, period - 1))).0;

        indicator_line[0] = (max - min) / sum.max(f64::EPSILON);
        State {
            buffer: buffer.into_full(),
            min_state,
            max_state,
            prev_real,
            sum,
        }
    }
}

/// Performs the main calculation loop for the VHF indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `period` - The period for the VHF calculation.
/// * `state` - A mutable reference to the current indicator state.
/// * `indicator_line` - A mutable slice for storing the VHF output values.
fn cycle(real: &[f64], period: usize, state: &mut State<Warm>, indicator_line: &mut [f64]) {
    let periods = (period, period - 1);

    for (j, i) in (period + 1..real.len()).enumerate() {
        unsafe {
            *indicator_line.get_unchecked_mut(j) =
                state.calc_unchecked((*real.get_unchecked(i), (real, i, periods)));
        }
    }
}

pub struct Vhf;

impl Indicator<INPUTS, OPTIONS> for Vhf {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "vhf",
        full_name: "Vertical Horizontal Filter",
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["period"],
        outputs: &["vhf"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "vhf",
            label: "VHF",
            display_type: DisplayType::Indicator,
            outputs: &["vhf"],
        }],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        // Validate options and minimal input data.
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;

        // Determine the start index for processing.
        let real = inputs[0];
        // Prepare the main output vector.
        let mut vhf_line = {
            let capacity = Self::output_length(real.len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let mut state = State::init_state(real, period, &mut vhf_line);

        cycle(real, period, &mut state, &mut vhf_line[1..]);

        Ok((vec![vhf_line], IndicatorState::new(state, real, period)))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::vhf_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Vhf {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::vhf_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
