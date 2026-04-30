use crate::common::{min_process, validate_inputs};
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::ad::calc as calc_ad;
use crate::indicators::ad::output_length as ad_output_length;
pub use crate::indicators::ad::INPUTS_WIDTH;
use crate::indicators::ema::{
    calc as calc_ema, multiplier as ema_multiplier, output_length as ema_output_length,
};
use crate::types::{DisplayType, IndicatorError, IndicatorInfoOrInteger, IndicatorType, Info};
use serde::{Deserialize, Serialize};
pub const OPTIONS_WIDTH: usize = 2;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::adosc_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::adosc_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::adosc_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::adosc_simd::indicator_by_options as indicator;
}
/// Returns information about the Accumulation/Distribution Oscillator (ADOSC) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the ADOSC indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "adosc",
        full_name: "Accumulation/Distribution Oscillator",
        indicator_type: IndicatorType::Trend,
        display_type: DisplayType::Indicator,
        inputs: &["high", "low", "close", "volume"],
        options: &["short_period", "long_period"],
        outputs: &["adosc"],
        optional_outputs: &["short_ema", "long_ema", "ad"],
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    multipliers: ((f64, f64), (f64, f64)),
    state: State,
}
impl IndicatorState {
    pub fn new(state: State, multipliers: ((f64, f64), (f64, f64))) -> Self {
        Self { state, multipliers }
    }
}
impl TIndicatorState<4> for IndicatorState {
    /// Calculates the Accumulation/Distribution Oscillator (ADOSC) indicator, picking up where the previous calculation left off.
    ///
    /// This function is useful for scenarios where indicator data is stored in a database and you need to continue calculations from the last stored state.
    ///
    /// # Arguments
    ///
    /// * `inputs` - A slice of vectors containing the high, low, close prices, and volume.
    /// * `options` - A slice containing the short and long periods for the ADOSC calculation.
    /// * `indicator_state` - An `IndicatorState` struct containing necessary input values.
    /// * `optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
    ///
    /// # Returns
    ///
    /// A vector of vectors containing the ADOSC line and any additional requested outputs.
    //#[inline(always)]
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
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
            inputs[3],  //volume
            self.multipliers,
            &mut self.state,
            &mut adosc_line,
            (&mut short_ema_line, &mut long_ema_line, &mut ad_line),
        );

        Ok(vec![adosc_line, short_ema_line, long_ema_line, ad_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub ad: f64,
    pub short_ema: f64,
    pub long_ema: f64,
}
impl State {
    pub fn new(ad: f64, short_ema: f64, long_ema: f64) -> Self {
        Self {
            ad,
            short_ema,
            long_ema,
        }
    }

    pub fn init_state(
        inputs: &[&[f64]; INPUTS_WIDTH],
        periods: (usize, usize),
        out_vecs: (&mut [f64], &mut [f64]),
    ) -> State {
        let (high, low, close, volume) = (inputs[0], inputs[1], inputs[2], inputs[3]);
        let (short_period, long_period) = periods;
        let (short_ema_line, ad_line) = out_vecs;

        let (mut ad, mut short_ema, mut long_ema) = (0.0, 0.0, 0.0);
        let (short_per, long_per) = multiplier(short_period, long_period);

        for i in 0..long_period - 1 {
            ad = calc_ad(ad, high[i], low[i], close[i], volume[i]);
            if i > 0 {
                short_ema = calc_ema(&ad, short_ema, short_per);
                long_ema = calc_ema(&ad, long_ema, long_per);
            } else {
                short_ema = ad;
                long_ema = ad;
            }
            crate::init_store_optional_outputs!(i, high.len(),
                short_ema_line => short_ema,
                ad_line => ad
            );
        }
        State {
            short_ema,
            long_ema,
            ad,
        }
    }
}
/// Returns the minimum amount of data required for the ADOSC indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the ADOSC calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[1] as usize // long_period
}
pub fn min_data_accuracy(options: &[f64], decimals: usize) -> usize {
    let (_short_multiplier, long_multiplier) = multiplier(options[0] as usize, options[1] as usize);
    min_process(
        options,
        Some((decimals, 0)),
        &[long_multiplier.0],
        IndicatorInfoOrInteger::Info(&info()),
        min_data,
    )
}
/// Calculates the output length based on the data length, options, and an optional recent-only parameter.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the ADOSC calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
///
/// # Returns
///
/// The output length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) ->  Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
/// Calculates the Accumulation/Distribution Oscillator (ADOSC) indicator for an entire dataset or a slice of it.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the high, low, close prices, and volume.
/// * `options` - A slice containing the short and long periods for the ADOSC calculation.
/// * `recent_only` - An optional tuple indicating whether to calculate only the most recent values and the length of recent data.
/// * `optional_outputs` - An optional slice of booleans indicating which additional outputs to generate.
///
/// # Returns
///
/// A vector of vectors containing the ADOSC line and any additional requested outputs.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    
    validate_options(options)?;
    
    let short_period = options[0] as usize;
    let long_period = options[1] as usize;
    let multipliers = multiplier(short_period, long_period);
    
    validate_inputs(inputs, min_data(options))?;

    let adosc_capacity = output_length(inputs[0].len(), options);
    let mut adosc_line = crate::uninit_vec!(f64, adosc_capacity);

    let (mut short_ema_line, mut long_ema_line, mut ad_line) = crate::init_optional_outputs_eff!(
        optional_outputs, &[false, false, false],
        short_ema_line: ema_output_length(inputs[0].len(), &[short_period as f64]),
        long_ema_line: adosc_capacity,
        ad_line: ad_output_length(inputs[0].len(), options)
    );

    let mut state = {
        State::init_state(
            inputs,
            (short_period, long_period),
            (&mut short_ema_line, &mut ad_line),
        )
    };
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
        let from = long_period -1;
        (&inputs[0][from..], &inputs[1][from..], &inputs[2][from..], &inputs[3][from..])
    };

    cycle_adosc(
        high, low, close, volume,
        multipliers,
        &mut state,
        &mut adosc_line,
        optional_outputs
    );

    Ok((
        vec![adosc_line, short_ema_line, long_ema_line, ad_line],
        IndicatorState {
            state: state,
            multipliers: multipliers,
        },
    ))
}

/// Performs the main calculation loop for the ADOSC indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `volume` - A slice of volume data.
/// * `adosc_line` - A mutable reference to a vector for storing the ADOSC line.
/// * `output_vectors` - A mutable reference to an array of optional vectors for storing additional outputs.
/// * `short_period` - The short period for the ADOSC calculation.
/// * `long_period` - The long period for the ADOSC calculation.
fn cycle_adosc(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    multipliers: ((f64, f64), (f64, f64)),
    state: &mut State,
    adosc_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
) {
    //let (high, low, close, volume) = (inputs[0], inputs[1], inputs[2], inputs[3]);

    let (short_ema_line, long_ema_line, ad_line) = out_vecs;
    let (has_optional, want_short, want_long, want_ad) =
        crate::calc_want_flags!(short_ema_line, long_ema_line, ad_line);

    //calculate offsets
    //let short_offset = crate::calc_output_offsets!(high.len(), short_ema_line);

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
            *adosc_line.get_unchecked_mut(i) = calc(state, inputs, multipliers);
        };
        if has_optional {
            crate::store_optional_outputs!(i,
                want_ad, ad_line => state.ad,
                want_short, short_ema_line => state.short_ema,
                want_long, long_ema_line => state.long_ema
            );
        }
    }
}

/// Calculates the current value of the Accumulation/Distribution Oscillator (ADOSC) indicator.
///
/// # Arguments
///
/// * `high` - The current high price.
/// * `low` - The current low price.
/// * `close` - The current close price.
/// * `volume` - The current volume.
/// * `prev_ad` - The previous AD value.
/// * `short_ema` - The previous short EMA value.
/// * `long_ema` - The previous long EMA value.
/// * `short_period` - The short period for the ADOSC calculation.
/// * `long_period` - The long period for the ADOSC calculation.
///
/// # Returns
///
/// A tuple containing the current ADOSC value, the updated AD value, the updated short EMA value, and the updated long EMA value.
#[inline(always)]
pub fn calc(
    state: &mut State,
    inputs: (f64, f64, f64, f64),
    multipliers: ((f64, f64), (f64, f64)),
) -> f64 {
    let (high, low, close, volume) = inputs;
    let (short_multiplier, long_multiplier) = multipliers;

    state.ad = calc_ad(state.ad, high, low, close, volume);
    state.short_ema = calc_ema(&state.ad, state.short_ema, short_multiplier);
    state.long_ema = calc_ema(&state.ad, state.long_ema, long_multiplier);

    state.short_ema - state.long_ema
}

#[inline(always)]
pub fn multiplier(short_period: usize, long_period: usize) -> ((f64, f64), (f64, f64)) {
    (ema_multiplier(short_period), ema_multiplier(long_period))
}
