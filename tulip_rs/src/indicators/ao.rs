use crate::common::validate_inputs;
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::indicators::medprice::calc as calc_medprice;
use crate::indicators::{
    simd_indicators::sma_simd::SimdState as SmaSimdState,
    sma::{calc as sma_calc, multiplier as sma_multiplier, Sma},
};
use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
use std::simd::Simd;
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 2;
//pub type Init = Full;
/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 0;
pub const SHORT_PERIOD: usize = 5;
pub const LONG_PERIOD: usize = 34;

pub type IndicatorState = State<Warm>;
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub buffer: Buffer<S>,
    pub sma_state: SmaSimdState<2>,
}

impl TIndicatorState<2> for IndicatorState {
    #[inline(always)]
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        //let mut ao_line: Vec<f64> = vec![0.0; inputs[0].len()]; //Vec::with_capacity(inputs[0].len());

        let capacity = inputs[0].len();
        let mut ao_line = crate::uninit_vec!(f64, capacity);

        let (mut short_sma_line, mut long_sma_line, mut medprice_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &optional_outputs.unwrap_or(&[false, false, false]),
            short_sma_line: capacity,
            long_sma_line: capacity,
            medprice_line: capacity
        );

        cycle_ao(
            inputs[0], //high
            inputs[1], //low
            self,
            &mut ao_line,
            (&mut short_sma_line, &mut long_sma_line, &mut medprice_line),
        );

        Ok(vec![ao_line, short_sma_line, long_sma_line, medprice_line])
    }
}
impl State<Cold> {
    pub fn new(short_sum: f64, long_sum: f64) -> State<Cold> {
        let multiplier = {
            let multi = multiplier((SHORT_PERIOD, LONG_PERIOD));
            Simd::from_array([multi.0, multi.1])
        };
        State {
            sma_state: SmaSimdState::new(Simd::from_array([short_sum, long_sum]), multiplier),
            buffer: Buffer::new(LONG_PERIOD),
        }
    }
    pub fn init_state(
        inputs: (&[f64], &[f64]),
        medprice_line: &mut [f64],
        short_sma_line: &mut [f64],
    ) -> State<Warm> {
        let (high, low) = inputs;
        let mut state = Self::new(0.0, 0.0);
        let [short_sum, long_sum] = state.sma_state.sum.as_mut_array();
        let (multiplier, _) = multiplier((SHORT_PERIOD, LONG_PERIOD));
        for (i, (&high_val, &low_val)) in high.iter().zip(low.iter()).take(LONG_PERIOD).enumerate()
        {
            let med_price = calc_medprice(high_val, low_val);
            let mut sma = 0.0;
            state.buffer.push(med_price);
            *long_sum += med_price;
            if i >= SHORT_PERIOD {
                let prev_medprice = calc_medprice(high[i - SHORT_PERIOD], low[i - SHORT_PERIOD]);
                sma = sma_calc(short_sum, &med_price, &prev_medprice, &multiplier);
            } else {
                *short_sum += med_price;
            }
            crate::init_store_optional_outputs!(i, high.len(),
                medprice_line => med_price,
                short_sma_line => sma
            );
        }
        State {
            sma_state: state.sma_state,
            buffer: state.buffer.into_full(),
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = (f64, f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, values: Self::Inputs<'a>) -> Self::Outputs {
        let (high, low) = values;

        let med_price = calc_medprice(high, low);

        let long_old = self.buffer.push_with_info(med_price);
        let short_old = self.buffer.get_by_period(SHORT_PERIOD);
        let [short_sma, long_sma] = self
            .sma_state
            .calc((
                Simd::splat(med_price),
                Simd::from_array([short_old, long_old]),
            ))
            .to_array();

        (short_sma - long_sma, short_sma, long_sma, med_price)
    }
}

/// Performs the main calculation loop for the AO indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `multipliers` - The precomputed SMA multipliers for the short and long periods.
/// * `state` - A mutable reference to the current `State` (buffer, short sum, long sum).
/// * `ao_line` - A mutable slice for storing the resulting AO line values.
/// * `out_vecs` - A tuple of mutable slices for optional outputs: short SMA, long SMA, and median price lines.
fn cycle_ao(
    high: &[f64],
    low: &[f64],
    state: &mut State<Warm>,
    ao_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (short_sma_line, long_sma_line, medprice_line) = out_vecs;
    let (has_optional, want_short, want_long, want_medprice) =
        crate::calc_want_flags!(short_sma_line, long_sma_line, medprice_line);

    for i in 0..high.len() {
        let values = unsafe { (*high.get_unchecked(i), *low.get_unchecked(i)) };

        let (ao, short_sma, long_sma, medprice) = state.calc(values);
        unsafe { *ao_line.get_unchecked_mut(i) = ao };

        if has_optional {
            crate::store_optional_outputs!(i,
                want_short, short_sma_line => short_sma,
                want_long, long_sma_line => long_sma,
                want_medprice, medprice_line => medprice
            );
        }
    }
}

#[inline(always)]
pub fn multiplier(periods: (usize, usize)) -> (f64, f64) {
    (sma_multiplier(periods.0), sma_multiplier(periods.1))
}

pub struct Ao;

impl Indicator<INPUTS, OPTIONS> for Ao {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "ao",
        full_name: "Awesome Oscillator",
        indicator_type: IndicatorType::Momentum,
        inputs: &["high", "low"],
        options: &[],
        outputs: &["ao"],
        optional_outputs: &["short_sma", "long_sma", "medprice"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "ao",
                label: "AO",
                display_type: DisplayType::Indicator,
                outputs: &["ao"],
            },
            DisplayGroup {
                offset: None,
                id: "short_sma_long_sma_medprice",
                label: "Median Price",
                display_type: DisplayType::Overlay,
                outputs: &["short_sma", "long_sma", "medprice"],
            },
        ],
    };
    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        35 // long_period
    }
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        _options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_inputs(inputs, Self::min_data(_options))?;

        let high = inputs[0];
        let low = inputs[1];

        let (mut ao_line, (mut short_sma_line, mut long_sma_line, mut medprice_line)) = {
            let capacity = Self::output_length(high.len(), _options);
            let short_capacity = Sma::output_length(high.len(), &[SHORT_PERIOD as f64]);

            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &optional_outputs.unwrap_or(&[false, false, false]),
                    short_sma_line: short_capacity,
                    long_sma_line: capacity,
                    medprice_line: high.len()
                ),
            )
        };

        let mut state = State::init_state((high, low), &mut medprice_line, &mut short_sma_line);
        let optional_outputs = {
            let offsets = crate::slice_outputs_start!(ao_line.len(), medprice_line, short_sma_line);
            (
                &mut short_sma_line[offsets.1..],
                long_sma_line.as_mut_slice(),
                &mut medprice_line[offsets.0..],
            )
        };
        let (high, low) = { (&high[LONG_PERIOD..], &low[LONG_PERIOD..]) };
        cycle_ao(high, low, &mut state, &mut ao_line, optional_outputs);

        Ok((
            vec![ao_line, short_sma_line, long_sma_line, medprice_line],
            state,
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::ao_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
