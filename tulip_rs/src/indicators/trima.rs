use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{TIndicatorState, Indicator, TState, IndicatorResult};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::trima_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::trima_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::trima_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::trima_simd::indicator_by_options as indicator;
}


#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    state: State<Warm>,
    period: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State<Warm>, period: usize) -> Self {
        Self {
            real: real[real.len() - period + 1..].to_vec(),
            state,
            period,
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

        let mut trima_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_trima(
            &self.real,
            self.period,
            &mut trima_line,
            &mut self.state,
        );

        self.real.drain(..self.real.len() - self.period + 1);

        Ok(vec![trima_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State<S = Cold> {
    pub weight_sum: f64,
    pub lead_sum: f64,
    pub trail_sum: f64,
    pub multiplier: f64,
    pub(crate) state: std::marker::PhantomData<S>,
}
impl State {
    pub fn new(weight_sum: f64, lead_sum: f64, trail_sum: f64, period: usize) -> Self {
        Self {
            weight_sum,
            lead_sum,
            trail_sum,
            multiplier: multiplier(period),
            state: std::marker::PhantomData,
        }
    }
    
    pub fn init_state(real: &[f64], period: usize) -> State<Warm> {
        let mut weight_sum = 0.0;
        let mut lead_sum = 0.0;
        let mut trail_sum = 0.0;
        let mut w = 1.0;

        let (lead_period, trail_period) = initialize_periods(period);

        for (i, &value) in real.iter().enumerate().take(period - 1) {
            weight_sum += value * w;
            if i + 1 > period - lead_period {
                lead_sum += value;
            }
            if i < trail_period {
                trail_sum += value;
            }
            if i + 1 < trail_period {
                w += 1.0;
            }
            if i + 1 >= period - lead_period {
                w -= 1.0;
            }
        }
        State {
            weight_sum,
            lead_sum,
            trail_sum,
            multiplier: multiplier(period),
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
        (real, lsi, tsi1, tsi2): Self::Inputs<'a>,
    ) -> Self::Outputs {
        let (mut weight_sum, mut lead_sum, mut trail_sum) =
            (self.weight_sum, self.lead_sum, self.trail_sum);
        weight_sum += real;
        let trima = weight_sum * self.multiplier;
        lead_sum += real;
        weight_sum += lead_sum - trail_sum;
        lead_sum -= lsi;
        trail_sum += tsi1 - tsi2;

        (self.weight_sum, self.lead_sum, self.trail_sum) = (weight_sum, lead_sum, trail_sum);
        trima
    }
}

/// Performs the main calculation loop for the TRIMA indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `period` - The period for the TRIMA calculation.
/// * `multiplier` - Normalization factor applied to produce the final TRIMA value.
/// * `trima_line` - A mutable slice for storing the TRIMA output values.
/// * `state` - A mutable reference to the rolling sums state.
pub fn cycle_trima(
    real: &[f64],
    period: usize,
    trima_line: &mut [f64],
    state: &mut State<Warm>,
) {
    let (mut lsi, mut tsi1) = initialize_counters(period);

    for (j, i) in (period - 1..real.len()).enumerate() {
        unsafe {
            *trima_line.get_unchecked_mut(j) = state.calc((
                *real.get_unchecked(i),
                *real.get_unchecked(lsi),
                *real.get_unchecked(tsi1),
                *real.get_unchecked(j), //tsi2),
            ));
        }

        (lsi, tsi1) = (lsi + 1, tsi1 + 1);
    }
}

/// Determines the 'lead' and 'trail' periods used for TRIMA calculations.
///
/// A Triangular Moving Average splits its period roughly in half, so:
/// - If `period` is odd, `lead_period` is simply `period / 2`.
/// - If `period` is even, `lead_period` becomes `(period / 2) - 1`.
///
/// The `trail_period` is always one more than `lead_period`.
///
/// # Arguments
///
/// * `period` - The TRIMA period.
///
/// # Returns
///
/// `(lead_period, trail_period)`:
/// - `lead_period`: The number of values considered as the 'lead' half.
/// - `trail_period`: The number of values for the trailing half.
#[inline(always)]
fn initialize_periods(period: usize) -> (usize, usize) {
    let lead_period = if period % 2 == 1 {
        period / 2
    } else {
        period / 2 - 1
    };
    let trail_period = lead_period + 1;
    (lead_period, trail_period)
}
/// Calculates the offset indices needed for lead and trail lookups used in the iteration.
///
/// # Arguments
///
/// * `period` - The TRIMA period.
///
/// # Returns
///
/// `(lsi, tsi1, tsi2)`:
/// - `lsi`: How far back we remove from the lead sum.
/// - `tsi1`: How far back we add to the trail sum.
/// - `tsi2`: How far back we remove from the trail sum.
#[inline(always)]
pub fn initialize_counters(period: usize) -> (usize, usize) {
    let (lead_period, trail_period) = initialize_periods(period);
    let lsi = (period - 1) - lead_period + 1;
    let tsi1 = trail_period;
    (lsi, tsi1)
}

/// Computes a multiplier for normalizing the weighted sums in the TRIMA calculation.
///
/// If the period is odd:
///   `multiplier = 1.0 / ((period / 2 + 1) * (period / 2 + 1))`
/// If the period is even:
///   `multiplier = 1.0 / ((period / 2 + 1) * (period / 2))`
///
/// # Arguments
///
/// * `period` - The TRIMA period.
///
/// # Returns
///
/// A `f64` scaling factor applied to produce the final TRIMA value.
pub fn multiplier(period: usize) -> f64 {
    if period % 2 == 1 {
        1.0 / ((period / 2 + 1) * (period / 2 + 1)) as f64
    } else {
        1.0 / ((period / 2 + 1) * (period / 2)) as f64
    }
}

pub struct Trima;

impl Indicator<INPUTS, OPTIONS> for Trima {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "trima",
        full_name: "Triangular Moving Average",
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["period"],
        outputs: &["trima"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "trima",
            label: "TRIMA",
            display_type: DisplayType::Overlay,
            outputs: &["trima"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
        validate_options(options)?;
        validate_inputs(inputs, Self::min_data(options))?;
        let period = options[0] as usize;
        let real = inputs[0];

        let mut trima_line = {
            let capacity = Self::output_length(real.len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        // Initialize rolling sums for the 2 SMA passes in TRIMA.
        // The original TRIMA logic can be performed with a single pass using these sums.
        let mut state = State::init_state(real, period);

        cycle_trima(real, period, &mut trima_line, &mut state);

        Ok((
            vec![trima_line],
            IndicatorState::new(real, state, period),
        ))
    }
}
