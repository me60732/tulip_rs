use crate::common::validate_inputs;
pub use crate::indicator_types::{TIndicatorState, TState, Indicator, IndicatorResult};
use crate::indicators::{
    sma::calc as sma_calc,
    stddev::{
        StdDev, State as StddevState, multiplier as stddev_multiplier
    },
};
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold
};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;
/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 3;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::vidya_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::vidya_simd::indicator_by_options;

// Sub-module exports with common naming
/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::vidya_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::vidya_simd::indicator_by_options as indicator;
}

pub fn multiplier(short_period: usize, long_period: usize) -> (f64, f64) {
    (stddev_multiplier(short_period), stddev_multiplier(long_period))
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State<Warm>,
    real: Vec<f64>,
    periods: (usize, usize),
}
impl IndicatorState {
    pub fn new(
        real: &[f64],
        state: State<Warm>,
        periods: (usize, usize),
    ) -> Self {
        Self {
            real: real[real.len() - periods.1..].to_vec(),
            state,
            periods,
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

        let (
            mut vidya_line,
            mut short_sma_line,
            mut long_sma_line,
            mut short_sd_line,
            mut long_sd_line,
        );
        {
            let capacity = inputs[0].len();
            vidya_line = crate::uninit_vec!(f64, capacity);

            (short_sma_line, long_sma_line, short_sd_line, long_sd_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false, false],
                short_sma_line: capacity,
                long_sma_line: capacity,
                short_sd_line: capacity,
                long_sd_line: capacity
            );
        }
        cycle(
            &self.real,
            self.periods,
            &mut self.state,
            &mut vidya_line,
            (
                &mut short_sma_line,
                &mut long_sma_line,
                &mut short_sd_line,
                &mut long_sd_line,
            ),
        );

        self.real.drain(..self.real.len() - self.periods.1);

        Ok(vec![
            vidya_line,
            short_sma_line,
            long_sma_line,
            short_sd_line,
            long_sd_line,
        ])
    }
}
#[derive(Serialize, Deserialize)]
#[serde(bound="")]
pub struct State<S = Cold> {
    pub short_state: StddevState<S>,
    pub long_state: StddevState<S>,
    pub alpha: f64,
    pub prev_vidya: f64,
}
impl State {
    pub fn new(short_state: (f64, f64), long_state: (f64, f64), alpha: f64, prev_vidya: f64, (short_period, long_period): (usize, usize)) -> Self {
        Self {
            short_state: StddevState::new(short_state.0, short_state.1, short_period),
            long_state: StddevState::new(long_state.0, long_state.1, long_period),
            prev_vidya,
            alpha
        }
    }
    pub fn init_state(
        short_period: usize,
        long_period: usize,
        real: &[f64],
        alpha: f64,
        vidya_line: &mut [f64],
        (short_sma_line, long_sma_line, short_sd_line, long_sd_line): (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
    ) -> State<Warm> {
        let mut sum_short: f64 = 0.0;
        let mut sum_sq_short: f64 = 0.0;
        let mut sum_long: f64 = 0.0;
        let mut sum_sq_long: f64 = 0.0;
        
        let (short_multiplier, long_multiplier) = multiplier(short_period, long_period);
        for (i, &value) in real.iter().enumerate().take(long_period) {
            sum_long += value;
            sum_sq_long += value * value;
            if i >= short_period {
                let prev_value = real[i - short_period];
                let short_sma = sma_calc(&mut sum_short, &value, &prev_value, &short_multiplier);
                sum_sq_short += (value * value) - (prev_value * prev_value);
                let short_stddev = (sum_sq_short * short_multiplier
                    - short_sma * (sum_short * short_multiplier))
                    .sqrt();
                crate::init_store_optional_outputs!(i, real.len(),
                    short_sma_line => short_sma,
                    short_sd_line => short_stddev
                );
            } else {
                sum_short += value;
                sum_sq_short += value * value;
            }
        }
        let short_sma = sum_short * short_multiplier;
        let short_stddev =
            (sum_sq_short * short_multiplier - short_sma * (sum_short * short_multiplier)).sqrt();
        let long_sma = sum_long * long_multiplier;
        let long_stddev =
            (sum_sq_long * long_multiplier - long_sma * (sum_long * long_multiplier)).sqrt();
        let mut k = if long_stddev.abs() < f64::EPSILON {
            0.0
        } else {
            short_stddev / long_stddev
        };
        if k.is_nan() {
            k = 0.0;
        }
        k *= alpha;
        let vidya = (real[long_period - 1] - real[long_period - 2]) * k + real[long_period - 2];
        vidya_line[0] = vidya;

        crate::init_store_optional_outputs!(long_period-1, real.len(),
            /*short_sma_line => short_sma,
            short_sd_line => short_stddev,*/
            long_sma_line => long_sma,
            long_sd_line => long_stddev
        );
        State {
            short_state: StddevState::new(sum_short, sum_sq_short, short_period).into_warm(),
            long_state: StddevState::new(sum_long, sum_sq_long, long_period).into_warm(),
            alpha,
            prev_vidya: vidya
        }
    }
    
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64, f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (value, prev_short, prev_long): Self::Inputs<'a>
    ) -> Self::Outputs {

        let (sd_short, sma_short) = self.short_state.calc((value, prev_short));

        // Compute long-term STDDEV.
        let (sd_long, sma_long) = self.long_state.calc((value, prev_long));

        let mut k = sd_short / sd_long;
        k *= self.alpha;
        self.prev_vidya = (value - self.prev_vidya).mul_add(k, self.prev_vidya);
        //self.prev_vidya = (value - self.prev_vidya) * k + self.prev_vidya;
        (self.prev_vidya, sma_short, sma_long, sd_short, sd_long)
    }
}

pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[2] <= 0.0 || options[2] >= 1.0 || options[0] < 1.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}


/// Iterates over the real data slice and computes VIDYA values for each bar.
///
/// # Arguments
///
/// * `real` - The full input data slice.
/// * `periods` - A tuple of `(short_period, long_period)`.
/// * `multipliers` - A tuple of `(short_multiplier, long_multiplier)` from `multiplier()`.
/// * `alpha` - The smoothing constant.
/// * `state` - Mutable reference to the rolling calculation state.
/// * `vidya_line` - Mutable output slice for VIDYA values.
/// * `out_vecs` - Mutable output slices for optional outputs:
///   `(short_sma, long_sma, short_sd, long_sd)`.
fn cycle(
    real: &[f64],
    (short_period, long_period): (usize, usize),
    state: &mut State<Warm>,
    vidya_line: &mut [f64],
    (short_sma_line, long_sma_line, short_sd_line, long_sd_line): (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
) {
    let (has_optional, want_short_sma, want_long_sma, want_short_sd, want_long_sd) =
        crate::calc_want_flags!(short_sma_line, long_sma_line, short_sd_line, long_sd_line);

    for (j, i) in (long_period..real.len()).enumerate() {
        let inputs = unsafe {
            (
                *real.get_unchecked(i),
                *real.get_unchecked(i - short_period), 
                *real.get_unchecked(j),
            )
        };
        let (vidya, sma_short, sma_long, sd_short, sd_long) =
            state.calc(inputs);
        unsafe { *vidya_line.get_unchecked_mut(j) = vidya };

        if has_optional {
            crate::store_optional_outputs!(j,
                want_long_sma, long_sma_line => sma_long,
                want_long_sd, long_sd_line => sd_long,
                want_short_sma, short_sma_line => sma_short,
                want_short_sd, short_sd_line => sd_short
            );
        }
    }
}

pub struct Vidya;

impl Indicator<INPUTS, OPTIONS> for Vidya {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "vidya",
        full_name: "Variable Index Dynamic Average",
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        // Three options: short_period, long_period, alpha.
        options: &["short_period", "long_period", "alpha"],
        outputs: &["vidya"],
        // Optional outputs: sma_fast and sma_slow are taken from the STDDEV calc.
        optional_outputs: &["short_sma", "long_sma", "short_stddev", "long_stddev"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "vidya",
                label: "VIDYA",
                display_type: DisplayType::Overlay,
                outputs: &["vidya"],
            },
            DisplayGroup {
                offset: None,
                id: "short_sma_long_sma",
                label: "SMAs",
                display_type: DisplayType::Overlay,
                outputs: &["short_sma", "long_sma"],
            },
            DisplayGroup {
                offset: None,
                id: "short_stddev_long_stddev",
                label: "Standard Deviation",
                display_type: DisplayType::Indicator,
                outputs: &["short_stddev", "long_stddev"],
            },
        ],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[1] as usize
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let short_period = options[0] as usize;
        let long_period = options[1] as usize;
        let alpha = options[2];
    
        validate_inputs(inputs, Self::min_data(options))?;
    
        let real = inputs[0];
    
        let (
            mut vidya_line,
            mut short_sma_line,
            mut long_sma_line,
            mut short_sd_line,
            mut long_sd_line,
            mut state,
            outputs,
        );
        {
            let capacity = Self::output_length(real.len(), options);
            let long_capacity = StdDev::output_length(real.len(), &[long_period as f64]);
            let short_capacity = StdDev::output_length(real.len(), &[short_period as f64]);
    
            vidya_line = crate::uninit_vec!(f64, capacity);
            (short_sma_line, long_sma_line, short_sd_line, long_sd_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false, false],
                short_sma_line: short_capacity,
                long_sma_line: long_capacity,
                short_sd_line: short_capacity,
                long_sd_line: long_capacity
            );
    
            // Start processing at the max period for a full window.
            state = State::init_state(
                short_period,
                long_period,
                real,
                alpha,
                &mut vidya_line,
                (
                    &mut short_sma_line,
                    &mut long_sma_line,
                    &mut short_sd_line,
                    &mut long_sd_line,
                ),
            );
            let start = crate::slice_outputs_start!(
                capacity - 1,
                short_sma_line,
                long_sma_line,
                short_sd_line,
                long_sd_line
            ); //capacity - 1 because vidya_line recieve 1 output bar in init_state
            outputs = (
                &mut short_sma_line[start.0..],
                &mut long_sma_line[start.1..],
                &mut short_sd_line[start.2..],
                &mut long_sd_line[start.3..],
            )
        }
    
        cycle(
            real,
            (short_period, long_period),
            &mut state,
            &mut vidya_line[1..],
            outputs,
        );
    
        Ok((
            vec![
                vidya_line,
                short_sma_line,
                long_sma_line,
                short_sd_line,
                long_sd_line,
            ],
            IndicatorState::new(real, state, (short_period, long_period)),
        ))
    }
}