use crate::common::validate_inputs;
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
/// Number of input price series required by this indicator.
pub use crate::indicators::ad::INPUTS;
use crate::indicators::ad::{Ad, State as AdState};
use crate::indicators::{
    ema::{multiplier as ema_multiplier, Ema},
    simd_indicators::ema_simd::{multiplier_simd, SimdState as EmaSimdState},
};

use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
use std::simd::Simd;
/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

pub type IndicatorState = State<Warm>;
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub ema_state: EmaSimdState<2>,
    pub ad_state: AdState,
    pub(crate) state: std::marker::PhantomData<S>,
}
impl State<Cold> {
    pub fn init_state(
        inputs: &[&[f64]; INPUTS],
        periods: (usize, usize),
        out_vecs: (&mut [f64], &mut [f64]),
    ) -> State<Warm> {
        let (high, low, close, volume) = (inputs[0], inputs[1], inputs[2], inputs[3]);
        let (short_period, long_period) = periods;
        let (short_ema_line, ad_line) = out_vecs;

        let multipliers = multiplier_simd([short_period, long_period]);
        let (mut ad_state, mut ema_state) = (
            AdState::new(0.0),
            EmaSimdState::new(Simd::splat(0.0), multipliers),
        );

        for i in 0..long_period - 1 {
            let ad = ad_state.calc((high[i], low[i], close[i], volume[i]));
            if i > 0 {
                ema_state.calc(Simd::splat(ad));
            } else {
                ema_state.ema = Simd::splat(ad);
            }
            crate::init_store_optional_outputs!(i, high.len(),
                short_ema_line => ema_state.ema[0],
                ad_line => ad
            );
        }
        State {
            ema_state,
            ad_state,
            state: std::marker::PhantomData,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64, f64);
    type Outputs = f64;
    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        let ad = self.ad_state.calc(inputs);
        let [short_ema, long_ema] = self.ema_state.calc(Simd::splat(ad)).to_array();

        short_ema - long_ema
    }
}
impl TIndicatorState<INPUTS> for IndicatorState {
    /// Calculates the ADOSC indicator, picking up where the previous calculation left off.
    ///
    /// This function is useful for scenarios where indicator data is stored in a database and
    /// you need to continue calculations from the last stored state.
    ///
    /// # Arguments
    ///
    /// * `inputs` - A reference to an array of 4 input slices: high, low, close, and volume.
    /// * `_optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<Vec<f64>>` with the ADOSC line and any additional requested outputs, or an `IndicatorError`.
    //#[inline(always)]
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let capacity = inputs[0].len();
        let mut adosc_line = crate::uninit_vec!(f64, capacity);

        let (mut short_ema_line, mut long_ema_line, mut ad_line) = crate::init_optional_outputs!(
            optional_outputs, &[false, false, false],
            short_ema_line: capacity,
            long_ema_line: capacity,
            ad_line: capacity
        );

        cycle_adosc(
            inputs[0], //high
            inputs[1], //low
            inputs[2], //close
            inputs[3], //volume
            self,
            &mut adosc_line,
            (&mut short_ema_line, &mut long_ema_line, &mut ad_line),
        );

        Ok(vec![adosc_line, short_ema_line, long_ema_line, ad_line])
    }
}

pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

fn cycle_adosc(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    state: &mut State<Warm>,
    adosc_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (short_ema_line, long_ema_line, ad_line) = out_vecs;
    let (has_optional, want_short, want_long, want_ad) =
        crate::calc_want_flags!(short_ema_line, long_ema_line, ad_line);

    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
                *volume.get_unchecked(i),
            )
        };
        unsafe {
            *adosc_line.get_unchecked_mut(i) = state.calc(inputs);
        };
        if has_optional {
            crate::store_optional_outputs!(i,
                want_ad, ad_line => state.ad_state.ad,
                want_short, short_ema_line => state.ema_state.ema[0],
                want_long, long_ema_line => state.ema_state.ema[1]
            );
        }
    }
}

#[inline(always)]
pub fn multiplier(short_period: usize, long_period: usize) -> ((f64, f64), (f64, f64)) {
    (ema_multiplier(short_period), ema_multiplier(long_period))
}

pub struct Adosc;

impl Indicator<INPUTS, OPTIONS> for Adosc {
    const INFO: Info = Info {
        name: "adosc",
        full_name: "Accumulation/Distribution Oscillator (Chaikin Oscillator)",
        indicator_type: IndicatorType::Volume,
        inputs: &["high", "low", "close", "volume"],
        options: &["short_period", "long_period"],
        outputs: &["adosc"],
        optional_outputs: &["short_ema", "long_ema", "ad"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "adosc",
                label: "ADOSC",
                display_type: DisplayType::Indicator,
                outputs: &["adosc"],
            },
            DisplayGroup {
                offset: None,
                id: "Accumulation/Distribution",
                label: "AD EMAs",
                display_type: DisplayType::Indicator,
                outputs: &["short_ema", "long_ema", "ad"],
            },
        ],
    };

    type IndicatorState = IndicatorState;

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

        validate_inputs(inputs, Self::min_data(options))?;

        let adosc_capacity = Self::output_length(inputs[0].len(), options);
        let mut adosc_line = crate::uninit_vec!(f64, adosc_capacity);

        let (mut short_ema_line, mut long_ema_line, mut ad_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false, false],
            short_ema_line: Ema::output_length(inputs[0].len(), &[short_period as f64]),
            long_ema_line: adosc_capacity,
            ad_line: Ad::output_length(inputs[0].len(), &[])
        );

        let mut state = State::init_state(
            inputs,
            (short_period, long_period),
            (&mut short_ema_line, &mut ad_line),
        );
        let optional_outputs = {
            let (short_start, ad_start) =
                crate::slice_outputs_start!(adosc_capacity, short_ema_line, ad_line);
            (
                &mut short_ema_line[short_start..],
                long_ema_line.as_mut_slice(),
                &mut ad_line[ad_start..],
            )
        };
        let (high, low, close, volume) = {
            let from = long_period - 1;
            (
                &inputs[0][from..],
                &inputs[1][from..],
                &inputs[2][from..],
                &inputs[3][from..],
            )
        };

        cycle_adosc(
            high,
            low,
            close,
            volume,
            &mut state,
            &mut adosc_line,
            optional_outputs,
        );

        Ok((
            vec![adosc_line, short_ema_line, long_ema_line, ad_line],
            state,
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::adosc_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Adosc {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::adosc_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
