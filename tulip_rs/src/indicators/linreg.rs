use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
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

        let (mut linreg_line, mut slope_line, mut intercept_line);
        {
            let capacity = inputs[0].len();
            (slope_line, intercept_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                slope_line: capacity,
                intercept_line: capacity
            );
            linreg_line = crate::uninit_vec!(f64, capacity);
        }

        cycle_linreg(
            &self.real,
            &mut self.state,
            self.period,
            &mut linreg_line,
            (&mut slope_line, &mut intercept_line),
        );
        self.real.drain(..self.real.len() - self.period + 1);

        Ok(vec![linreg_line, slope_line, intercept_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State<S = Cold> {
    pub sum_x: f64,
    pub sum_y: f64,
    pub sum_xy: f64,
    pub per: f64,
    pub n: f64,
    pub(crate) state: std::marker::PhantomData<S>,
}
impl State<Cold> {
    pub fn new(sum_x: f64, sum_y: f64, sum_xy: f64, per: f64, period: usize) -> Self {
        Self {
            sum_x,
            sum_y,
            sum_xy,
            per,
            n: period as f64,
            state: std::marker::PhantomData,
        }
    }

    pub fn init_state(data: &[f64], period: usize) -> State<Warm> {
        let (mut sum_x, mut sum_xx, mut sum_y, mut sum_xy) = (0.0, 0.0, 0.0, 0.0);
        if data.len() >= period - 1 {
            for i in 0..period - 1 {
                let d = unsafe { *data.get_unchecked(i) };
                sum_x += (i + 1) as f64;
                sum_xx += ((i + 1) as f64).powi(2);
                sum_y += d;
                sum_xy += (i + 1) as f64 * d;
            }
        }
        sum_x += period as f64;
        sum_xx += (period * period) as f64;
        let per = multiplier(period, sum_x, sum_xx);
        State {
            sum_x,
            sum_y,
            sum_xy,
            per,
            n: period as f64,
            state: std::marker::PhantomData,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = (f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, (prev_value, value): Self::Inputs<'a>) -> Self::Outputs {
        let (sum_x, mut sum_y, mut sum_xy, per, n) =
            (self.sum_x, self.sum_y, self.sum_xy, self.per, self.n);

        sum_xy += value * n;
        sum_y += value;

        let slope = (n * sum_xy - sum_x * sum_y) * per;
        let intercept = (sum_y - slope * sum_x) / n;
        let linreg = intercept + slope * n;

        sum_xy -= sum_y;
        sum_y -= prev_value;

        (self.sum_y, self.sum_xy) = (sum_y, sum_xy);
        (linreg, slope, intercept)
    }
}

/// Performs the main calculation loop for the LINREG indicator using rolling sums.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `state` - A mutable reference to the current `State`.
/// * `period` - The period for the LINREG calculation.
/// * `linreg_line` - A mutable slice for storing the LINREG output values.
/// * `out_vecs` - A tuple of mutable slices for optional slope and intercept outputs.
fn cycle_linreg(
    real: &[f64],
    state: &mut State<Warm>,
    period: usize,
    linreg_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let (slope_line, intercept_line) = out_vecs;
    let (has_optional, want_slope, want_intercept) =
        crate::calc_want_flags!(slope_line, intercept_line);

    for (j, i) in (period - 1..real.len()).enumerate() {
        let inputs = unsafe { (*real.get_unchecked(j), *real.get_unchecked(i)) };
        let (linreg, slope, intercept) = state.calc(inputs);

        unsafe { *linreg_line.get_unchecked_mut(j) = linreg };
        if has_optional {
            crate::store_optional_outputs!(j,
                want_slope, slope_line => slope,
                want_intercept, intercept_line => intercept
            );
        }
    }
}

/// Calculates the multiplier for the LINREG calculation.
#[inline]
pub fn multiplier(period: usize, sum_x: f64, sum_xx: f64) -> f64 {
    1.0 / (period as f64 * sum_xx - sum_x.powi(2))
}

pub struct Linreg;

impl Indicator<INPUTS, OPTIONS> for Linreg {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "linreg",
        indicator_type: IndicatorType::Trend,
        full_name: "Linear Regression",
        inputs: &["real"],
        options: &["period"],
        outputs: &["linreg"],
        optional_outputs: &["linregslope", "linregintercept"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "linreg_linregintercept",
                label: "Regression",
                display_type: DisplayType::Overlay,
                outputs: &["linreg", "linregintercept"],
            },
            DisplayGroup {
                offset: None,
                id: "linregslope",
                label: "Linear Regression Slope",
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
        let (mut linreg_line, mut slope_line, mut intercept_line);
        {
            let capacity = Self::output_length(real.len(), options);
            (slope_line, intercept_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                slope_line: capacity,
                intercept_line: capacity
            );
            linreg_line = crate::uninit_vec!(f64, capacity);
        }
        let mut state = State::init_state(&real[1..period], period);
        // Perform the main LINREG calculation
        cycle_linreg(
            &real[1..],
            &mut state,
            period,
            &mut linreg_line,
            (&mut slope_line, &mut intercept_line),
        );

        Ok((
            vec![linreg_line, slope_line, intercept_line],
            IndicatorState::new(state, real, period),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::linreg_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Linreg {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS],
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::linreg_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
