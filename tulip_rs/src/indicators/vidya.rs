use crate::common::{min_process, validate_inputs};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::{
    sma::calc as sma_calc,
    stddev::{
        calc as stddev_calc, multiplier as stddev_multiplier,
        output_length as stddev_output_length, State as StddevState,
    },
};
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};

pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 3;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::vidya_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::vidya_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::vidya_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::vidya_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    real: Vec<f64>,
    periods: (usize, usize),
    multipliers: (f64, f64),
    alpha: f64,
}
impl IndicatorState {
    pub fn new(
        real: &[f64],
        state: State,
        periods: (usize, usize),
        multipliers: (f64, f64),
        alpha: f64,
    ) -> Self {
        Self {
            real: real[real.len() - periods.1..].to_vec(),
            state,
            periods,
            multipliers,
            alpha,
        }
    }
}

impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
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
            self.multipliers,
            self.alpha,
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
pub struct State {
    pub short_state: StddevState,
    pub long_state: StddevState,
    pub prev_vidya: f64,
}
impl State {
    pub fn new(short_state: (f64, f64), long_state: (f64, f64), prev_vidya: f64) -> Self {
        Self {
            short_state: StddevState::new(short_state.0, short_state.1),
            long_state: StddevState::new(long_state.0, long_state.1),
            prev_vidya,
        }
    }

    pub fn init_state(
        short_period: usize,
        long_period: usize,
        real: &[f64],
        alpha: f64,
        vidya_line: &mut [f64],
        out_vecs: (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
    ) -> Self {
        let mut sum_short: f64 = 0.0;
        let mut sum_sq_short: f64 = 0.0;
        let mut sum_long: f64 = 0.0;
        let mut sum_sq_long: f64 = 0.0;
        let (short_sma_line, long_sma_line, short_sd_line, long_sd_line) = out_vecs;
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
        Self::new((sum_short, sum_sq_short), (sum_long, sum_sq_long), vidya)
    }
    #[inline(always)]
    pub fn calc(
        &mut self,
        value: &f64,
        prev_values: (&f64, &f64),
        alpha: f64,
        multipliers: (f64, f64),
    ) -> (f64, f64, f64, f64, f64) {
        // Compute short-term STDDEV.
        let (multiplier_short, multiplier_long) = multipliers;
        let (prev_short, prev_long) = prev_values;

        let (sd_short, sma_short) = self.short_state.calc(value, &prev_short, multiplier_short);

        // Compute long-term STDDEV.
        let (sd_long, sma_long) = self.long_state.calc(value, &prev_long, multiplier_long);

        let mut k = sd_short / sd_long;
        k *= alpha;

        self.prev_vidya = (value - self.prev_vidya) * k + self.prev_vidya;
        (self.prev_vidya, sma_short, sma_long, sd_short, sd_long)
    }
}
pub fn info() -> Info<'static> {
    Info {
        name: "vidya",
        full_name: "Variable Index Dynamic Average",
        display_type: DisplayType::Overlay,
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        // Three options: short_period, long_period, alpha.
        options: &["short_period", "long_period", "alpha"],
        outputs: &["vidya"],
        // Optional outputs: sma_fast and sma_slow are taken from the STDDEV calc.
        optional_outputs: &["short_sma", "long_sma", "short_sdtdev", "long_sdtdev"],
    }
}
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    let (short_multiplier, long_multiplier) = multiplier(options[0] as usize, options[1] as usize);
    if options[1] >= 10.0 {
        min_process(
            options,
            Some((decimals, 0)),
            &[long_multiplier],
            IndicatorInfoOrInteger::Info(&info()),
            min_data,
        )
    } else {
        min_process(
            options,
            Some((decimals, 0)),
            &[long_multiplier, short_multiplier],
            IndicatorInfoOrInteger::Info(&info()),
            min_data,
        )
    }
}
/// Returns the minimum required data points (using the long period).
pub fn min_data(options: &[f64]) -> usize {
    options[1] as usize
}

/// Returns the output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[2] <= 0.0 || options[2] >= 1.0 || options[0] < 1.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Full-indicator calculation for VIDYA.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let short_period = options[0] as usize;
    let long_period = options[1] as usize;
    let alpha = options[2];
    let multipliers = multiplier(short_period, long_period);

    validate_inputs(inputs, min_data(options))?;

    let real = inputs[0];

    let (
        mut vidya_line,
        mut short_sma_line,
        mut long_sma_line,
        mut short_sd_line,
        mut long_sd_line,
        mut state,
        outputs
    );
    {
        let capacity = output_length(real.len(), options);
        let long_capacity = stddev_output_length(real.len(), &[long_period as f64]);
        let short_capacity = stddev_output_length(real.len(), &[short_period as f64]);

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
        multipliers,
        alpha,
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
        IndicatorState::new(real, state, (short_period, long_period), multipliers, alpha),
    ))
}

/// Loop through the data calling calc() for each bar.
fn cycle(
    real: &[f64],
    periods: (usize, usize),
    multipliers: (f64, f64),
    alpha: f64,
    state: &mut State,
    vidya_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
) {
    let (short_period, long_period) = periods;
    let (short_sma_line, long_sma_line, short_sd_line, long_sd_line) = out_vecs;
    let (has_optional, want_short_sma, want_long_sma, want_short_sd, want_long_sd) =
        crate::calc_want_flags!(short_sma_line, long_sma_line, short_sd_line, long_sd_line);

    for (j, i) in (long_period..real.len()).enumerate() {
        let (value, prev_values) = unsafe {
            (
                real.get_unchecked(i),
                (
                    real.get_unchecked(i - short_period),
                    real.get_unchecked(j),
                ),
            )
        };
        let (vidya, sma_short, sma_long, sd_short, sd_long) =
            calc(state, value, prev_values, alpha, multipliers);
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

/// Calculation for a single bar of VIDYA.
/// All computations—including calling stddev_calc—are done here.
///
/// Parameters:
/// - real: full data slice.
/// - i: current index (must be at least as high as both short_period and long_period).
/// - short_period: period for the short volatility.
/// - long_period: period for the long volatility.
/// - alpha: scaling constant.
/// - multiplier_short: multiplier for the short_period.
/// - multiplier_long: multiplier for the long_period.
/// - prev_vidya: previous VIDYA value.
/// - sum_short, sum_sq_short: rolling state for the short period.
/// - sum_long, sum_sq_long: rolling state for the long period.
///
/// Returns a tuple:
/// (new_vidya, new_sum_short, new_sum_sq_short, new_sum_long, new_sum_sq_long, sma_short, sma_long)
#[inline(always)]
pub fn calc(
    state: &mut State,
    value: &f64,
    prev_values: (&f64, &f64),
    alpha: f64,
    multipliers: (f64, f64),
) -> (f64, f64, f64, f64, f64) {
    // Compute short-term STDDEV.
    let (multiplier_short, multiplier_long) = multipliers;
    let (prev_short, prev_long) = prev_values;

    let (sd_short, sma_short) =
        stddev_calc(&mut state.short_state, value, &prev_short, multiplier_short);

    // Compute long-term STDDEV.
    let (sd_long, sma_long) =
        stddev_calc(&mut state.long_state, value, &prev_long, multiplier_long);

    let mut k = sd_short / sd_long;
    k *= alpha;

    //state.prev_vidya = (value - state.prev_vidya) * k + state.prev_vidya;
    state.prev_vidya = (value - state.prev_vidya).mul_add(k, state.prev_vidya);
    (state.prev_vidya, sma_short, sma_long, sd_short, sd_long)
}
#[inline(always)]
pub fn multiplier(short_period: usize, long_period: usize) -> (f64, f64) {
    (
        stddev_multiplier(short_period),
        stddev_multiplier(long_period),
    )
}
