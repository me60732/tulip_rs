use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::atr::{min_data, min_data_accuracy, multiplier, output_length};

use crate::indicators::tr::output_length as tr_output_length;
use crate::indicators::{
    atr::State as AtrState,
    medprice::{calc as calc_medprice, output_length as medprice_output_length},
};

use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;
/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::supertrend_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::supertrend_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::supertrend_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::supertrend_simd::indicator_by_options as indicator;
}

pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= 0.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Returns information about the SuperTrend indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the DI indicator.
pub const INFO: Info = Info {
    name: "supertrend",
    full_name: "Super Trend",
    indicator_type: IndicatorType::Trend,
    inputs: &["high", "low", "close"],
    options: &["period", "step"],
    outputs: &["supertrend"],
    optional_outputs: &["atr", "tr", "medprice"],
    display_groups: &[
        DisplayGroup {
            offset: None,
            id: "supertrend",
            label: "Super Trend",
            display_type: DisplayType::Overlay,
            outputs: &["supertrend", "medprice"],
        },
        DisplayGroup {
            offset: None,
            id: "atr_tr",
            label: "True Range",
            display_type: DisplayType::Indicator,
            outputs: &["atr", "tr"],
        },
    ],
};
#[derive(Serialize, Deserialize)]
pub struct State {
    pub atr_state: AtrState,
    pub prev_st: f64,
    pub prev_ub: f64,
    pub prev_lb: f64,
    pub trend: bool,
}
impl State {
    pub fn new(atr_state: AtrState) -> Self {
        Self {
            atr_state,
            prev_st: 0.0,
            prev_lb: 0.0,
            prev_ub: 0.0,
            trend: false,
        }
    }
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        step: f64,
        tr_line: &mut [f64],
        medprice_line: &mut [f64],
    ) -> State {
        let mut state = Self::new(AtrState::init_state(
            high, low, close, period, tr_line, false,
        ));
        if medprice_line.len() > 0 {
            for i in 0..period - 1 {
                medprice_line[i] = calc_medprice(high[i], low[i]);
            }
        }
        let step = step * state.atr_state.atr;
        let (_, medprice) =
            state.calc_st(high[period - 1], low[period - 1], close[period - 1], step);
        if medprice_line.len() > 0 {
            medprice_line[period - 1] = medprice;
        }

        state
    }
    #[inline(always)]
    pub fn calc(
        &mut self,
        high: f64,
        low: f64,
        close: f64,
        step: f64,
        multipliers: (f64, f64),
    ) -> (f64, f64, f64, f64) {
        let (atr, tr) = self.atr_state.calc(high, low, close, multipliers);
        let step = step * atr;
        let (st, medprice) = self.calc_st(high, low, close, step);
        (st, atr, tr, medprice)
    }

    #[inline(always)]
    fn calc_st(&mut self, high: f64, low: f64, close: f64, step: f64) -> (f64, f64) {
        let medprice = calc_medprice(high, low);
        let mut ub = medprice + step;
        let mut lb = medprice - step;

        // Branchless trend update
        let crosses_up = close > self.prev_st;
        let crosses_down = close < self.prev_st;
        self.trend = crosses_up | (self.trend & !crosses_down);

        let st = if self.trend {
            lb = self.prev_lb.max(lb);
            lb
        } else {
            ub = self.prev_ub.min(ub);
            ub
        };

        (self.prev_lb, self.prev_ub, self.prev_st) = (lb, ub, st);

        (st, medprice)
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    multipliers: (f64, f64),
    step: f64,
}
impl IndicatorState {
    pub fn new(state: State, step: f64, multipliers: (f64, f64)) -> Self {
        Self {
            state,
            multipliers,
            step,
        }
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut st_line, (mut atr_line, mut tr_line, mut medprice_line)) = {
            let capacity = inputs[0].len();

            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false, false],
                    atr_line: capacity,
                    tr_line: capacity,
                    medprice_line: capacity
                ),
            )
        };
        let [high, low, close] = inputs;
        cycle_calc(
            (high, low, close),
            &mut self.state,
            self.step,
            self.multipliers,
            &mut st_line,
            (&mut atr_line, &mut tr_line, &mut medprice_line),
        );

        Ok(vec![st_line, atr_line, tr_line, medprice_line])
    }
}

/// Calculates the SuperTrend indicator over the full input dataset.
///
/// SuperTrend uses a Wilder-smoothed ATR to build upper and lower bands around
/// the median price. The trend flips when the close crosses the active band, and
/// the band ratchets in the trend direction each bar so it can never widen.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
///
/// # Options
///
/// * `options[0]` — period (Wilder smoothing window for the ATR)
/// * `options[1]` — step (ATR multiplier used to set the band width)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true, false, false])` to enable `atr`,
///   `Some(&[false, true, false])` to enable `tr`,
///   `Some(&[false, false, true])` to enable `medprice`, or any combination;
///   `None` disables all optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is the SuperTrend line,
/// `outputs[1]` is `atr`, `outputs[2]` is `tr`, and `outputs[3]` is `medprice`
/// (each empty unless requested). `state` can be passed to
/// `IndicatorState::batch_indicator` for streaming updates.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let multipliers = multiplier(period);
    let step = options[1];

    validate_inputs(inputs, min_data(options))?;
    let [high, low, close] = *inputs;

    let (mut st_line, (mut atr_line, mut tr_line, mut medprice_line)) = {
        let capacity = output_length(high.len(), options);
        let tr_capacity = tr_output_length(high.len(), options);
        let med_capacity = medprice_output_length(high.len(), options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                atr_line: capacity,
                tr_line: tr_capacity,
                medprice_line: med_capacity
            ),
        )
    };
    let mut state = State::init_state(
        high,
        low,
        close,
        period,
        step,
        &mut tr_line,
        &mut medprice_line,
    );
    let (tr, med) = {
        let (tr, med) = crate::slice_outputs_start!(st_line.len(), tr_line, medprice_line);
        (&mut tr_line[tr..], &mut medprice_line[med..])
    };
    cycle_calc(
        (&high[period..], &low[period..], &close[period..]),
        &mut state,
        step,
        multipliers,
        &mut st_line,
        (&mut atr_line, tr, med),
    );

    Ok((
        vec![st_line, atr_line, tr_line, medprice_line],
        IndicatorState::new(state, step, multipliers),
    ))
}

/// Performs the main calculation loop for the SuperTrend indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of `(high, low, close)` price slices.
/// * `state` - Mutable reference to the SuperTrend [`State`].
/// * `step` - The ATR multiplier (band half-width = `step × atr`).
/// * `multipliers` - Wilder's smoothing multipliers `(alpha, 1-alpha)`.
/// * `st_line` - Mutable output slice for the SuperTrend values.
/// * `out_vecs` - A tuple of `(atr_line, tr_line, medprice_line)` for optional outputs.
fn cycle_calc(
    inputs: (&[f64], &[f64], &[f64]),
    state: &mut State,
    step: f64,
    multipliers: (f64, f64),
    st_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (high, low, close) = inputs;
    let (atr_line, tr_line, medprice_line) = out_vecs;
    let (has_optional, want_atr, want_tr, want_medprice) =
        crate::calc_want_flags!(atr_line, tr_line, medprice_line);

    for i in 0..high.len() {
        let (h, l, c) = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };

        let (st, atr, tr, medprice) = state.calc(h, l, c, step, multipliers);

        unsafe {
            *st_line.get_unchecked_mut(i) = st;
        }
        if has_optional {
            crate::store_optional_outputs!(i,
                want_tr, tr_line => tr,
                want_atr, atr_line => atr,
                want_medprice, medprice_line => medprice
            );
        }
    }
}
