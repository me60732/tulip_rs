use crate::common::validate_inputs;
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

pub use crate::indicators::atr::{multiplier, Atr};

use crate::indicators::tr::Tr;
use crate::indicators::{
    atr::State as AtrState,
    medprice::{calc as calc_medprice, Medprice},
};

use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;
/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= 0.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub atr_state: AtrState<S>,
    pub prev_st: f64,
    pub prev_ub: f64,
    pub prev_lb: f64,
    pub step: f64,
    pub trend: bool,
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64, f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        let (atr, tr) = self.atr_state.calc(inputs);
        let step = self.step * atr;
        let (st, medprice) = self.calc_st(inputs, step);
        (st, atr, tr, medprice)
    }
}
impl State<Cold> {
    pub fn new(atr_state: AtrState, step: f64) -> Self {
        Self {
            atr_state,
            prev_st: 0.0,
            prev_lb: 0.0,
            prev_ub: 0.0,
            step,
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
    ) -> State<Warm> {
        let mut state = State::<Warm> {
            atr_state: AtrState::init_state(high, low, close, period, tr_line, false),
            prev_st: 0.0,
            prev_lb: 0.0,
            prev_ub: 0.0,
            step,
            trend: false,
        };
        if medprice_line.len() > 0 {
            for i in 0..period - 1 {
                medprice_line[i] = calc_medprice(high[i], low[i]);
            }
        }
        let step = step * state.atr_state.wilders_state.wilders;
        let (_, medprice) =
            state.calc_st((high[period - 1], low[period - 1], close[period - 1]), step);
        if medprice_line.len() > 0 {
            medprice_line[period - 1] = medprice;
        }

        state
    }
}
impl State<Warm> {
    #[inline(always)]
    fn calc_st(&mut self, (high, low, close): (f64, f64, f64), step: f64) -> (f64, f64) {
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
pub type IndicatorState = State<Warm>;

impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
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
            self,
            &mut st_line,
            (&mut atr_line, &mut tr_line, &mut medprice_line),
        );

        Ok(vec![st_line, atr_line, tr_line, medprice_line])
    }
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
    (high, low, close): (&[f64], &[f64], &[f64]),
    state: &mut State<Warm>,
    st_line: &mut [f64],
    (atr_line, tr_line, medprice_line): (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (has_optional, want_atr, want_tr, want_medprice) =
        crate::calc_want_flags!(atr_line, tr_line, medprice_line);

    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };

        let (st, atr, tr, medprice) = state.calc(inputs);

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

pub struct SuperTrend;

impl Indicator<INPUTS, OPTIONS> for SuperTrend {
    type IndicatorState = IndicatorState;

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize + 1 // period
    }
    const INFO: Info = Info {
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

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
        let step = options[1];

        validate_inputs(inputs, Self::min_data(options))?;
        let [high, low, close] = *inputs;

        let (mut st_line, (mut atr_line, mut tr_line, mut medprice_line)) = {
            let capacity = Self::output_length(high.len(), options);
            let tr_capacity = Tr::output_length(high.len(), &[]);
            let med_capacity = Medprice::output_length(high.len(), &[]);
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
            &mut st_line,
            (&mut atr_line, tr, med),
        );

        Ok((vec![st_line, atr_line, tr_line, medprice_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::supertrend_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for SuperTrend {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::supertrend_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
