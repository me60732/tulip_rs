use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::typprice::calc as calc_typprice;
pub use crate::indicators::typprice::{min_data, min_data_accuracy, output_length};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 4;
/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 0;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::vwap_simd::indicator_by_assets;

// Sub-module exports with common naming
/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::vwap_simd::indicator_by_assets as indicator;
}

/// Metadata describing the VWAP indicator's name, inputs, options, and outputs.
pub const INFO: Info = Info {
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
/// Running state for the Volume Weighted Average Price (VWAP) indicator.
///
/// Holds the cumulative price-volume sum (`pv_sum`) and cumulative volume sum
/// (`vol_sum`) needed to compute the running VWAP at each new bar.
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub pv_sum: f64,
    pub vol_sum: f64,
}
impl IndicatorState {
    /// Creates a new zeroed `IndicatorState` ready for the first bar of a session.
    pub fn new() -> Self {
        Self {
            pv_sum: 0.0,
            vol_sum: 0.0,
        }
    }
    /// Advances the state by one bar and returns `(vwap, typprice)`.
    ///
    /// `typprice = (high + low + close) / 3`. Accumulates `typprice × volume`
    /// and `volume` into the running sums, then divides to produce the cumulative VWAP.
    #[inline(always)]
    pub fn calc(&mut self, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64) {
        let tp = calc_typprice(&high, &low, &close);
        self.pv_sum += tp * volume;
        self.vol_sum += volume;
        (self.pv_sum / self.vol_sum, tp)
    }
}
impl TIndicatorState<INPUTS_WIDTH> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
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

/// Calculates the Volume Weighted Average Price (VWAP) over the full input dataset.
///
/// VWAP is the cumulative ratio of `(typical_price × volume)` to `volume` from
/// the first bar to each bar, making it a session-level running average.
/// `typical_price = (high + low + close) / 3`.
///
/// # Inputs
///
/// * `inputs[0]` — `high`
/// * `inputs[1]` — `low`
/// * `inputs[2]` — `close`
/// * `inputs[3]` — `volume`
///
/// # Arguments
///
/// * `inputs` - Array of input slices (see Inputs above).
/// * `_options` - Unused; VWAP takes no options.
/// * `optional_outputs` - Pass `Some(&[true])` to include `typprice` as `outputs[1]`.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `vwap` and `state`
/// can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    _options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    // Expecting four inputs: High, Low, Close, Volume.
    validate_inputs(inputs, min_data(_options))?;

    let [high, low, close, volume] = *inputs;
    let (mut vwap_line, mut typprice_line) = {
        let capacity = output_length(high.len(), &[]);
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
            (*vwap_line.get_unchecked_mut(i), tp) = state.calc(
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
                *volume.get_unchecked(i),
            );
        }
        crate::store_optional_outputs!(i,
            want_tp, typprice_line => tp
        );
    }
}
/// Calculates a single VWAP value for one bar, updating the running state in place.
///
/// # Arguments
///
/// * `high` - The current bar's high price.
/// * `low` - The current bar's low price.
/// * `close` - The current bar's close price.
/// * `volume` - The current bar's volume.
/// * `state` - Mutable reference to the `IndicatorState` (running `pv_sum` and `vol_sum`).
///
/// # Returns
///
/// `(vwap, typprice)` — the cumulative VWAP and the typical price for this bar.
#[inline(always)]
pub fn calc(
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    state: &mut IndicatorState,
) -> (f64, f64) {
    state.calc(high, low, close, volume)
}
