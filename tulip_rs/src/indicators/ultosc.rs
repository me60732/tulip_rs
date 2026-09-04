use crate::common::validate_inputs;
use crate::common_simd::{deserialize_f64x2, serialize_f64x2};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
use crate::indicators::tr::Tr;
use crate::ring_buffer::multi_buffer::multi_buffer::MultiBuffer as Buffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
//use wide::*;
use std::simd::{num::SimdFloat, Simd};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 3;

const MULTIPLIERS: Simd<f64, 2> = Simd::from_array([4.0, 2.0]);

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State<Warm>,
    periods: (usize, usize),
}
impl IndicatorState {
    pub fn new(state: State<Warm>, periods: (usize, usize)) -> Self {
        Self { state, periods }
    }
}

impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let [high, low, close] = *inputs;
        let (mut ultosc_line, (mut tr_line, mut bp_line)) = {
            let len = high.len();
            (
                crate::uninit_vec!(f64, inputs[0].len()),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    tr_line: len,
                    bp_line: len
                ),
            )
        };

        cycle(
            high,
            low,
            close,
            self.periods,
            &mut self.state,
            &mut ultosc_line,
            (&mut tr_line, &mut bp_line),
        );

        Ok(vec![ultosc_line, tr_line, bp_line])
    }
}
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub buffer: Buffer<2, f64, S>,

    #[serde(
        serialize_with = "serialize_f64x2",
        deserialize_with = "deserialize_f64x2"
    )]
    pub bp_sums_2x: Simd<f64, 2>,

    #[serde(
        serialize_with = "serialize_f64x2",
        deserialize_with = "deserialize_f64x2"
    )]
    pub tr_sums_2x: Simd<f64, 2>,
    pub bp_long_sum: f64,
    pub tr_long_sum: f64,
    pub prev_close: f64,
}

impl State<Cold> {
    pub fn new(long_period: usize, prev_close: f64) -> Self {
        Self {
            buffer: Buffer::new(long_period),
            bp_long_sum: 0.0,
            bp_sums_2x: Simd::<f64, 2>::splat(0.0),
            tr_long_sum: 0.0,
            tr_sums_2x: Simd::<f64, 2>::splat(0.0),
            prev_close,
        }
    }
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        periods: (usize, usize, usize),
        ultosc_line: &mut [f64],
        tr_line: &mut [f64],
        bp_line: &mut [f64],
    ) -> State<Warm> {
        let long_period = periods.2;
        let mut state = Self::new(long_period, close[0]);
        let (has_optional, want_tr, want_bp) = crate::calc_want_flags!(tr_line, bp_line);
        for (i, ((&high_val, &low_val), &close_val)) in high
            .iter()
            .zip(low.iter())
            .zip(close.iter())
            .enumerate()
            .skip(1)
            .take(long_period)
        {
            let (ult, tr, bp) = state.calc((high_val, low_val, close_val, periods.0, periods.1));
            if i == long_period {
                ultosc_line[0] = ult;
            }
            if has_optional {
                crate::store_optional_outputs!(i-1,
                    want_tr, tr_line => tr,
                    want_bp, bp_line => bp
                );
            }
        }
        State {
            buffer: state.buffer.into_full(),
            bp_sums_2x: state.bp_sums_2x,
            tr_sums_2x: state.tr_sums_2x,
            bp_long_sum: state.bp_long_sum,
            tr_long_sum: state.tr_long_sum,
            prev_close: state.prev_close,
        }
    }
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (high, low, close, short_period, medium_period): (f64, f64, f64, usize, usize),
    ) -> (f64, f64, f64) {
        const DIV: f64 = 100.0 / 7.0;

        let true_low = low.min(self.prev_close);
        let true_high = high.max(self.prev_close);
        let bp = close - true_low;
        let tr = true_high - true_low;

        if let Some([old_bp, old_tr]) = self.buffer.push_with_info([bp, tr]) {
            self.bp_long_sum += bp - old_bp;
            self.tr_long_sum += tr - old_tr;
        } else {
            self.bp_long_sum += bp;
            self.tr_long_sum += tr;
        }

        let (bp_x2, tr_x2) = (Simd::<f64, 2>::splat(bp), Simd::<f64, 2>::splat(tr));
        let (bp_r, tr_r) = {
            let [bp, tr] = self
                .buffer
                .get_by_periods::<2>([short_period, medium_period]);
            (
                Simd::<f64, 2>::from_array(bp),
                Simd::<f64, 2>::from_array(tr),
            )
        };

        self.bp_sums_2x += bp_x2 - bp_r;
        self.tr_sums_2x += tr_x2 - tr_r;
        self.prev_close = close;

        if self.buffer.is_full() {
            let weight_sum = (MULTIPLIERS * self.bp_sums_2x / self.tr_sums_2x).reduce_sum();

            let third = self.bp_long_sum / self.tr_long_sum;
            return (((weight_sum + third) * DIV), tr, bp);
        }
        (0.0, tr, bp)
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64, usize, usize);
    type Outputs = (f64, f64, f64);

    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (high, low, close, short_period, medium_period): Self::Inputs<'a>,
    ) -> Self::Outputs {
        const DIV: f64 = 100.0 / 7.0;

        let true_low = low.min(self.prev_close);
        let true_high = high.max(self.prev_close);
        let bp = close - true_low;
        let tr = true_high - true_low;

        let [old_bp, old_tr] = self.buffer.push_with_info([bp, tr]);
        self.bp_long_sum += bp - old_bp;
        self.tr_long_sum += tr - old_tr;

        let (bp_x2, tr_x2) = (Simd::<f64, 2>::splat(bp), Simd::<f64, 2>::splat(tr));
        let (bp_r, tr_r) = {
            let [bp, tr] = self
                .buffer
                .get_by_periods::<2>([short_period, medium_period]);
            (
                Simd::<f64, 2>::from_array(bp),
                Simd::<f64, 2>::from_array(tr),
            )
        };

        self.bp_sums_2x += bp_x2 - bp_r;
        self.tr_sums_2x += tr_x2 - tr_r;

        let weight_sum = (MULTIPLIERS * self.bp_sums_2x / self.tr_sums_2x).reduce_sum();
        //let weight_sum = first_second.reduce_add();
        let third = self.bp_long_sum / self.tr_long_sum;
        self.prev_close = close;
        (((weight_sum + third) * DIV), tr, bp)
    }
}

pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] < options[0] || options[2] < options[1] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

/// Performs the main calculation loop for the ULTOSC indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `periods` - A tuple `(short_period, medium_period)` used for the weighted sums.
/// * `state` - A mutable reference to the current indicator state.
/// * `ultosc_line` - A mutable slice for storing the ULTOSC output values.
fn cycle(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    periods: (usize, usize),
    state: &mut State<Warm>,
    ultosc_line: &mut [f64],
    optional_outputs: (&mut [f64], &mut [f64]),
) {
    let (tr_line, bp_line) = optional_outputs;
    let (has_optional, want_tr, want_bp) = crate::calc_want_flags!(tr_line, bp_line);

    for i in 0..high.len() {
        let (ultosc, tr, bp);
        let (high, low, close) = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };

        (ultosc, tr, bp) = state.calc((high, low, close, periods.0, periods.1));
        unsafe { *ultosc_line.get_unchecked_mut(i) = ultosc };

        if has_optional {
            crate::store_optional_outputs!(i,
                want_tr, tr_line => tr,
                want_bp, bp_line => bp
            );
        }
    }
}

pub struct Ultosc;

impl Indicator<INPUTS, OPTIONS> for Ultosc {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "ultosc",
        full_name: "Ultimate Oscillator",
        indicator_type: IndicatorType::Momentum,
        // Inputs are expected to be: high, low, close
        inputs: &["high", "low", "close"],
        // Options: short_period, medium_period, long_period
        options: &["short_period", "medium_period", "long_period"],
        outputs: &["ultosc", "tr", "bp"],
        optional_outputs: &[],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "ultosc",
                label: "ULTOSC",
                display_type: DisplayType::Indicator,
                outputs: &["ultosc"],
            },
            DisplayGroup {
                offset: None,
                id: "tr_bp",
                label: "Buying Pressure",
                display_type: DisplayType::Indicator,
                outputs: &["tr", "bp"],
            },
        ],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let periods = (
            options[0] as usize,
            options[1] as usize,
            options[2] as usize,
        );

        validate_inputs(inputs, Self::min_data(options))?;
        let [high, low, close] = inputs;

        let (mut ultosc_line, (mut tr_line, mut bp_line)) = {
            let capacity = Self::output_length(high.len(), options);
            let tr_capacity = Tr::output_length(high.len(), &[]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    tr_line: tr_capacity,
                    bp_line: tr_capacity
                ),
            )
        };

        let mut state = State::init_state(
            high,
            low,
            close,
            periods,
            &mut ultosc_line,
            &mut tr_line,
            &mut bp_line,
        );
        let (tr, bp) = {
            let mut offset = crate::slice_outputs_start!(ultosc_line.len(), tr_line);
            if offset > 0 {
                offset += 1;
            }
            (&mut tr_line[offset..], &mut bp_line[offset..])
        };
        // Single-pass calculation loop.
        cycle(
            &high[periods.2 + 1..],
            &low[periods.2 + 1..],
            &close[periods.2 + 1..],
            (periods.0, periods.1),
            &mut state,
            &mut ultosc_line[1..],
            (tr, bp),
        );

        Ok((
            vec![ultosc_line, tr_line, bp_line],
            IndicatorState {
                periods: (periods.0, periods.1),
                state,
            },
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::ultosc_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Ultosc {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::ultosc_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
