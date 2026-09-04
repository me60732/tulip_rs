use crate::common::validate_inputs;
pub use crate::indicator_types::{Indicator, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState};
use crate::indicators::medprice::calc as calc_medprice;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 0;

pub type IndicatorState = State<Warm>;
#[derive(Serialize, Deserialize)]
pub struct State<S = Cold> {
    pub prev_medprice: f64,
    pub(crate) state: std::marker::PhantomData<S>,
}
impl State<Cold> {
    pub fn init_state(high: &[f64], low: &[f64]) -> State<Warm> {
        State {
            prev_medprice: calc_medprice(high[0], low[0]),
            state: std::marker::PhantomData,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, volume): Self::Inputs<'a>) -> Self::Outputs {
        let medprice = calc_medprice(high, low);
        let distance_moved = medprice - self.prev_medprice;
        let hl_diff = (high - low).max(f64::EPSILON);
        self.prev_medprice = medprice;

        (
            distance_moved * 10000.0 * hl_diff / volume.max(f64::EPSILON),
            medprice,
        )
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut emv_line, mut medprice_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    medprice_line: capacity
                ),
            )
        };
        let [high, low, volume] = inputs;
        // Perform the main EMV calculation
        cycle_emv(high, low, volume, self, &mut emv_line, &mut medprice_line);

        Ok(vec![emv_line, medprice_line])
    }
}

/// Performs the main calculation loop for the EMV indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `volume` - A slice of volume data.
/// * `prev_medprice` - A mutable reference to the previous median price value.
/// * `emv_line` - A mutable slice for storing the EMV output values.
/// * `medprice_line` - A mutable slice for storing the optional median price output.
fn cycle_emv(
    high: &[f64],
    low: &[f64],
    volume: &[f64],
    state: &mut State<Warm>,
    emv_line: &mut [f64],
    medprice_line: &mut [f64],
) {
    let (_, want_medprice) = crate::calc_want_flags!(medprice_line);

    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *volume.get_unchecked(i),
            )
        };
        let (emv, medprice) = state.calc(inputs);
        unsafe {
            *emv_line.get_unchecked_mut(i) = emv;
        }
        crate::store_optional_outputs!(i,
            want_medprice, medprice_line => medprice);
    }
}

pub struct Emv;

impl Indicator<INPUTS, OPTIONS> for Emv {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "emv",
        indicator_type: IndicatorType::Volume,
        full_name: "Ease of Movement",
        inputs: &["high", "low", "volume"],
        options: &[],
        outputs: &["emv"],
        optional_outputs: &["medprice"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "emv",
                label: "EMV",
                display_type: DisplayType::Indicator,
                outputs: &["emv"],
            },
            DisplayGroup {
                offset: None,
                id: "medprice",
                label: "Median Price",
                display_type: DisplayType::Overlay,
                outputs: &["medprice"],
            },
        ],
    };

    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        2 // The EMV calculation requires at least two data points
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        _options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
        validate_inputs(inputs, Self::min_data(_options))?;

        let [high, low, volume] = inputs;
        let mut state = State::init_state(high, low);

        let (mut emv_line, mut medprice_line);
        {
            let capacity = Self::output_length(high.len(), _options);
            let medprice_capacity = high.len();
            emv_line = crate::uninit_vec!(f64, capacity);
            medprice_line = crate::init_optional_outputs_eff!(
                optional_outputs, &[false],
                medprice_line: medprice_capacity
            );
            crate::init_store_optional_outputs!(0, medprice_capacity,
                medprice_line => state.prev_medprice
            );
        }
        let medprice = {
            let offset = crate::slice_outputs_start!(emv_line.len(), medprice_line);
            &mut medprice_line[offset..]
        };
        let (high, low, volume) = (&high[1..], &low[1..], &volume[1..]);
        // Perform the main EMV calculation
        cycle_emv(high, low, volume, &mut state, &mut emv_line, medprice);

        Ok((vec![emv_line, medprice_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::emv_simd::indicator_by_assets::<N>(inputs, options, optional_outputs)
    }
}
