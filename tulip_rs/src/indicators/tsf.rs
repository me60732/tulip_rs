use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
use crate::indicators::linreg::State as LinregState;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
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

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
#[repr(transparent)]
pub struct State<S = Cold>(pub LinregState<S>);
impl<S> Deref for State<S> {
    type Target = LinregState<S>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<S> DerefMut for State<S> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl State<Cold> {
    pub fn init_state(data: &[f64], period: usize) -> State<Warm> {
        State(LinregState::init_state(data, period))
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = (f64, f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        let (linreg, slope, intercept);
        (linreg, slope, intercept) = self.0.calc(inputs);
        //let tsf = intercept + slope * (period + 1) as f64;
        let tsf = slope.mul_add(self.n + 1.0, intercept);
        (tsf, linreg, slope, intercept)
    }
}
impl IndicatorState {
    pub fn new(state: State<Warm>, real: &[f64], period: usize) -> Self {
        Self {
            state,
            real: real[real.len() - period + 1..].to_vec(),
            period,
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        self.real.extend_from_slice(inputs[0]);

        let (mut tsf_line, mut linreg_line, mut slope_line, mut intercept_line);
        {
            let capacity = inputs[0].len();
            (linreg_line, slope_line, intercept_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                linreg_line: capacity,
                slope_line: capacity,
                intercept_line: capacity
            );
            tsf_line = crate::uninit_vec!(f64, capacity);
        }
        cycle_tsf(
            &self.real,
            &mut self.state,
            self.period,
            &mut tsf_line,
            (&mut linreg_line, &mut slope_line, &mut intercept_line),
        );

        self.real.drain(..self.real.len() - self.period + 1);

        Ok(vec![tsf_line, linreg_line, slope_line, intercept_line])
    }
}

/// Performs the main calculation loop for the TSF indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `state` - A mutable reference to the current linear regression state.
/// * `period` - The period for the TSF calculation.
/// * `tsf_line` - A mutable slice for storing the TSF output values.
/// * `out_vecs` - A tuple of mutable slices for optional outputs `(linreg_line, slope_line, intercept_line)`.
fn cycle_tsf(
    real: &[f64],
    state: &mut State<Warm>,
    period: usize,
    tsf_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (linreg_line, slope_line, intercept_line) = out_vecs;
    let (has_optional, want_linreg, want_slope, want_intercept) =
        crate::calc_want_flags!(linreg_line, slope_line, intercept_line);

    for (j, i) in (period - 1..real.len()).enumerate() {
        let inputs = unsafe { (*real.get_unchecked(j), *real.get_unchecked(i)) };
        let (tsf, linreg, slope, intercept) = state.calc(inputs);

        unsafe { *tsf_line.get_unchecked_mut(j) = tsf };

        if has_optional {
            crate::store_optional_outputs!(j,
                want_linreg, linreg_line => linreg,
                want_slope, slope_line => slope,
                want_intercept, intercept_line => intercept
            );
        }
    }
}

pub struct Tsf;
impl Indicator<INPUTS, OPTIONS> for Tsf {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "tsf",
        indicator_type: IndicatorType::Trend,
        full_name: "Time Series Forecast",
        inputs: &["real"],
        options: &["period"],
        outputs: &["tsf"],
        optional_outputs: &["linreg", "linregslope", "linregintercept"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "tsf_linreg_linregintercept",
                label: "Regression",
                display_type: DisplayType::Overlay,
                outputs: &["tsf", "linreg", "linregintercept"],
            },
            DisplayGroup {
                offset: None,
                id: "linregslope",
                label: "LinReg Slope",
                display_type: DisplayType::Indicator,
                outputs: &["linregslope"],
            },
        ],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;

        let real = inputs[0];
        let (mut tsf_line, mut linreg_line, mut slope_line, mut intercept_line);
        {
            let capacity = Self::output_length(real.len(), options);
            (linreg_line, slope_line, intercept_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                linreg_line: capacity,
                slope_line: capacity,
                intercept_line: capacity
            );
            tsf_line = crate::uninit_vec!(f64, capacity); //Vec::with_capacity(capacity);
        }
        let mut state = State::init_state(&real[1..period], period);

        // Perform the main TSF calculation
        cycle_tsf(
            &real[1..],
            &mut state,
            period,
            &mut tsf_line,
            (&mut linreg_line, &mut slope_line, &mut intercept_line),
        );

        Ok((
            vec![tsf_line, linreg_line, slope_line, intercept_line],
            IndicatorState::new(state, real, period),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::tsf_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Tsf {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::tsf_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
