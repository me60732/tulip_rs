use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{TIndicatorState, Indicator, IndicatorResult, TState};
//use crate::indicators::linreg::State as LinregState;
use crate::indicators::tsf::{Tsf, State as TsfState};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::fosc_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::fosc_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::fosc_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::fosc_simd::indicator_by_options as indicator;
}

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
        inputs: &[&[f64]; 1],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.real.extend_from_slice(inputs[0]);

        let (mut fosc_line, mut tsf_line, mut linreg_line, mut slope_line, mut intercept_line);
        {
            let capacity = inputs[0].len();
            (tsf_line, linreg_line, slope_line, intercept_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false, false],
                tsf_line: capacity,
                linreg_line: capacity,
                slope_line: capacity,
                intercept_line: capacity
            );
            fosc_line = crate::uninit_vec!(f64, capacity);
        }
        //let mut fosc_line = Vec::<f64>::with_capacity(capacity); //vec![0.0; capacity];
        // Perform the main FOSC calculation
        cycle_fosc(
            &self.real,
            &mut self.state,
            self.period,
            (
                &mut fosc_line,
                &mut tsf_line,
                &mut linreg_line,
                &mut slope_line,
                &mut intercept_line,
            ),
        );

        self.real.drain(..self.real.len() - self.period + 1);

        Ok(vec![
            fosc_line,
            tsf_line,
            linreg_line,
            slope_line,
            intercept_line,
        ])
    }
}
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub tsf_state: TsfState<S>,
    pub tsf: f64,
}
impl State<Cold> {
    /*pub fn new(tsf: f64, sum_x: f64, sum_y: f64, sum_xy: f64, per: f64) -> Self {
        Self {
            tsf,
            tsf_state: TsfState::new(sum_x, sum_y, sum_xy, per),
        }
    }*/
    pub fn init_state(
        real: &[f64],
        period: usize,
        (tsf_line, linreg_line, slope_line, intercept_line): (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
    ) -> State<Warm> {
        let (has_optional, _, _, _, _) =
            crate::calc_want_flags!(tsf_line, linreg_line, slope_line, intercept_line);

        let mut tsf_state = TsfState::init_state(&real[1..period], period);
        let (tsf, linreg, slope, intercept) = tsf_state.calc((real[1], real[period]));
        
        if has_optional {
            crate::init_store_optional_outputs!(period, real.len(),
                tsf_line => tsf,
                linreg_line => linreg,
                slope_line => slope,
                intercept_line => intercept
            );
        }
        State {
            tsf_state,
            tsf,
        }
    }

}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = (f64, f64, f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (prev_value, value): Self::Inputs<'a>
    ) ->  Self::Outputs {
        let fosc = 100.0 * (value - self.tsf) / value; //.max(f64::EPSILON);
    
        let (tsf, linreg, slope, intercept) =
            self.tsf_state.calc((prev_value, value));
        self.tsf = tsf;
        (fosc, tsf, linreg, slope, intercept)
    }
}

/// Performs the main calculation loop for the FOSC indicator using rolling sums.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `state` - A mutable reference to the indicator state.
/// * `period` - The period for the FOSC calculation.
/// * `start` - The starting index within `real` for the calculation.
/// * `out_vecs` - A tuple of mutable output slices:
///   `(fosc_line, tsf_line, linreg_line, slope_line, intercept_line)`.
//#[inline(always)]
fn cycle_fosc(
    real: &[f64],
    state: &mut State<Warm>,
    period: usize,
    (fosc_line, tsf_line, linreg_line, slope_line, intercept_line): (&mut [f64], &mut [f64], &mut [f64], &mut [f64], &mut [f64]),
) {
    let (has_optional, want_tsf, want_linreg, want_slope, want_intercept) =
        crate::calc_want_flags!(tsf_line, linreg_line, slope_line, intercept_line);

    //for (i, &value) in real.iter().enumerate().skip(start) {
    for (j, i) in (period-1..real.len()).enumerate() {
        let inputs = unsafe {( 
            *real.get_unchecked(j), 
            *real.get_unchecked(i)
        )};
        let (fosc, tsf, linreg, slope, intercept) = state.calc(inputs);

        unsafe { *fosc_line.get_unchecked_mut(j) = fosc };

        if has_optional {
            crate::store_optional_outputs!(j,
                want_tsf, tsf_line => tsf,
                want_linreg, linreg_line => linreg,
                want_slope, slope_line => slope,
                want_intercept, intercept_line => intercept
            );
        }
    }
}

pub struct Fosc;
impl Indicator<INPUTS, OPTIONS> for Fosc {
    type IndicatorState = IndicatorState;
    /// Returns information about the Forecast Oscillator (FOSC) indicator.
    ///
    /// # Returns
    ///
    /// An `Info` struct containing metadata about the FOSC indicator.
    const INFO: Info = Info {
        name: "fosc",
        indicator_type: IndicatorType::Trend,
        full_name: "Forecast Oscillator",
        inputs: &["real"],
        options: &["period"],
        outputs: &["fosc"],
        optional_outputs: &["tsf", "linreg", "linregslope", "linregintercept"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "fosc",
                label: "FOSC",
                display_type: DisplayType::Indicator,
                outputs: &["fosc"],
            },
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
    /// Returns the minimum amount of data required for the FOSC indicator.
    ///
    /// # Arguments
    ///
    /// * `options` - A slice containing the options for the FOSC calculation.
    ///
    /// # Returns
    ///
    /// The minimum amount of data required.
    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize + 2
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
    
        validate_inputs(inputs, Self::min_data(options))?;
    
        let real = inputs[0];
        let (mut fosc_line, mut tsf_line, mut linreg_line, mut slope_line, mut intercept_line);
        {
            let capacity = Self::output_length(real.len(), options);
            let tsf_capacity = Tsf::output_length(real.len(), options);
            (tsf_line, linreg_line, slope_line, intercept_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false, false],
                tsf_line: tsf_capacity,
                linreg_line: tsf_capacity,
                slope_line: tsf_capacity,
                intercept_line: tsf_capacity
            );
    
            fosc_line = crate::uninit_vec!(f64, capacity);
        }
        let mut state = State::init_state(
            real,
            period,
            (
                &mut tsf_line,
                &mut linreg_line,
                &mut slope_line,
                &mut intercept_line,
            ),
        );
        let outputs = {
            let offsets = crate::slice_outputs_start!(
                fosc_line.len(),
                tsf_line,
                linreg_line,
                slope_line,
                intercept_line
            );
            (
                fosc_line.as_mut_slice(),
                &mut tsf_line[offsets.0..],
                &mut linreg_line[offsets.1..],
                &mut slope_line[offsets.2..],
                &mut intercept_line[offsets.3..],
            )
        };
    
        // Perform the main FOSC calculation
        cycle_fosc(&real[2..], &mut state, period, outputs);
    
        Ok((
            vec![fosc_line, tsf_line, linreg_line, slope_line, intercept_line],
            IndicatorState::new(state, real, period),
        ))
    }
}
