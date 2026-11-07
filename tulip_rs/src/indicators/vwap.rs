use crate::common::validate_inputs;
use crate::cse::{Planable, Recipe};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

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
#[derive(Serialize, Deserialize, Clone)]
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

impl Planable for Vwap {
    /// The graph is leaf data — `crate::cse::recipes::vwap`: the typical-price
    /// subgraph `((high + low) + close) * (1/3)` spelled node-for-node as
    /// `recipes::typprice` does (so `intern` merges them — see
    /// `vwap_typprice_shared`), then `Div` of two `Accum` runs, `tp*volume` over
    /// `volume`. The hand computes `tp` by CALLING `typprice::calc`, which is why
    /// the association must be copied rather than re-derived.
    fn recipe(_options: &[f64]) -> Recipe {
        crate::cse::recipes::vwap(_options)
    }
}

impl crate::cse::Restorable<INPUTS, OPTIONS> for Vwap {
    const KIND: &'static str = "vwap";

    fn restore_state(
        v: Vec<crate::cse::ChainState>,
        _windows: &[&[f64]],
    ) -> Option<Self::IndicatorState> {
        let [crate::cse::ChainState::Mirror { value: pv_sum }, crate::cse::ChainState::Mirror { value: vol_sum }] =
            v.as_slice()
        else {
            return None;
        };
        Some(State {
            pv_sum: *pv_sum,
            vol_sum: *vol_sum,
        })
    }
}

#[cfg(feature = "simd_options")]
impl crate::indicator_types::IndicatorByOptions<INPUTS, OPTIONS> for Vwap {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS],
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> crate::indicator_types::SimdIndicatorResult<Vec<Self::IndicatorState>> {
        let mut all_outputs = Vec::with_capacity(N);
        let mut all_states = Vec::with_capacity(N);

        for option in options.iter() {
            let (outputs, state) = Self::indicator(inputs, option, optional_outputs)?;
            all_outputs.push(outputs);
            all_states.push(state);
        }

        Ok((all_outputs, all_states))
    }
}
