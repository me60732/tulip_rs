use crate::common::validate_inputs;
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.

use crate::indicators::typprice::calc as calc_typprice;
pub use crate::indicators::typprice::Typprice;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 4;
/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 0;

/// Running state for the Volume Weighted Average Price (VWAP) indicator.
///
/// Holds the cumulative price-volume sum (`pv_sum`) and cumulative volume sum
/// (`vol_sum`) needed to compute the running VWAP at each new bar.
pub type IndicatorState = State;
#[derive(Serialize, Deserialize)]
pub struct State {
    pub pv_sum: f64,
    pub vol_sum: f64,
}
impl State {
    /// Creates a new zeroed `IndicatorState` ready for the first bar of a session.
    pub fn new() -> Self {
        Self {
            pv_sum: 0.0,
            vol_sum: 0.0,
        }
    }
}
impl TState for State {
    type Inputs<'a> = (f64, f64, f64, f64);
    type Outputs = (f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, close, volume): Self::Inputs<'a>) -> Self::Outputs {
        let tp = calc_typprice(high, low, close);
        self.pv_sum += tp * volume;
        self.vol_sum += volume;
        (self.pv_sum / self.vol_sum, tp)
    }
}
impl TIndicatorState<INPUTS> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let [high, low, close, volume] = *inputs;
        let (mut vwap_line, mut typprice_line) = {
            let len = high.len();
            (
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    tp: len
                ),
            )
        };

        cycle(
            high,
            low,
            close,
            volume,
            self,
            &mut vwap_line,
            &mut typprice_line,
        );

        Ok(vec![vwap_line, typprice_line])
    }
}

/// Iterates over the high, low, close, and volume slices and computes VWAP values for each bar.
///
/// # Arguments
///
/// * `high` - Input high price slice.
/// * `low` - Input low price slice.
/// * `close` - Input close price slice.
/// * `volume` - Input volume slice.
/// * `state` - Mutable reference to the `IndicatorState` (running `pv_sum` and `vol_sum`).
/// * `vwap_line` - Mutable output slice for VWAP values.
/// * `typprice_line` - Mutable output slice for the optional typical-price values.
fn cycle(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    state: &mut IndicatorState,
    vwap_line: &mut [f64],
    typprice_line: &mut [f64],
) {
    let (_, want_tp) = crate::calc_want_flags!(typprice_line);
    for i in 0..close.len() {
        let tp;
        unsafe {
            (*vwap_line.get_unchecked_mut(i), tp) = state.calc((
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
                *volume.get_unchecked(i),
            ));
        }
        crate::store_optional_outputs!(i,
            want_tp, typprice_line => tp
        );
    }
}

pub struct Vwap;
impl Indicator<INPUTS, OPTIONS> for Vwap {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "vwap",
        full_name: "Volume Weighted Average Price",
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low", "close", "volume"],
        options: &[],
        outputs: &["vwap"],
        optional_outputs: &["typprice"],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "vwap",
            label: "Price",
            display_type: DisplayType::Overlay,
            outputs: &["vwap", "typprice"],
        }],
    };
    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        Typprice::min_data(_options)
    }
    fn output_length(data_len: usize, _options: &[f64; OPTIONS]) -> usize {
        Typprice::output_length(data_len, _options)
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        _options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
        // Expecting four inputs: High, Low, Close, Volume.
        validate_inputs(inputs, Typprice::min_data(_options))?;

        let [high, low, close, volume] = *inputs;
        let (mut vwap_line, mut typprice_line) = {
            let capacity = Typprice::output_length(high.len(), &[]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    tp: capacity
                ),
            )
        };

        let mut state = IndicatorState::new();

        cycle(
            high,
            low,
            close,
            volume,
            &mut state,
            &mut vwap_line,
            &mut typprice_line,
        );

        // State holds running pv_sum and vol_sum for incremental updates.
        Ok((vec![vwap_line, typprice_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::vwap_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
