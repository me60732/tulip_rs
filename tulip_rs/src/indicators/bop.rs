use crate::common::validate_inputs;
use crate::cse::{Planable, Recipe};
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 4;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 0;

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState;

impl TIndicatorState<4> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        process(inputs)
    }
}

//#[inline(always)]
fn process(inputs: &[&[f64]]) -> Result<Vec<Vec<f64>>, IndicatorError> {
    validate_inputs(inputs, 1)?;

    let open = inputs[0];
    let high = inputs[1];
    let low = inputs[2];
    let close = inputs[3];
    let len = open.len();
    let mut bop_line = crate::uninit_vec!(f64, len);

    open.iter()
        .zip(high.iter())
        .zip(low.iter())
        .zip(close.iter())
        .enumerate()
        .for_each(|(i, (((&o, &h), &l), &c))| unsafe {
            *bop_line.get_unchecked_mut(i) = calc(o, h, l, c);
        });

    Ok(vec![bop_line])
}

/// Calculates the Balance of Power (BOP) value.
///
/// # Arguments
///
/// * `open` - The open price.
/// * `high` - The high price.
/// * `low` - The low price.
/// * `close` - The close price.
///
/// # Returns
///
/// The BOP value.
#[inline(always)]
pub fn calc(open: f64, high: f64, low: f64, close: f64) -> f64 {
    let hl_diff = (high - low).max(f64::EPSILON);
    (close - open) / hl_diff
}

pub struct Bop;

impl Indicator<INPUTS, OPTIONS> for Bop {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "bop",
        full_name: "Balance of Power",
        indicator_type: IndicatorType::Momentum,
        inputs: &["open", "high", "low", "close"],
        options: &[],
        outputs: &["bop"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "bop",
            label: "BOP",
            display_type: DisplayType::Indicator,
            outputs: &["bop"],
        }],
    };

    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        1
    }

    fn output_length(data_len: usize, _options: &[f64; OPTIONS]) -> usize {
        data_len
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        _options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        let outputs = process(inputs)?;

        Ok((outputs, IndicatorState))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::bop_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

impl Planable for Bop {
    /// The graph is leaf data — `crate::cse::recipes::bop`:
    /// `Sub{close, open} / Max{Sub{high, low}, EPSILON}`, over the `ohlc()`
    /// column set, stateless. The `Max` IS the hand's flat-bar guard — a
    /// `Where*` select pins `0.0` where the hand prints `(close - open) /
    /// f64::EPSILON`, so the floor must stay a `Max` (pinned by
    /// `bop_lone_floor_bit_exact`'s flat-bar block).
    fn recipe(_options: &[f64]) -> Recipe {
        crate::cse::recipes::bop(_options)
    }
}

impl crate::cse::Restorable<INPUTS, OPTIONS> for Bop {
    const KIND: &'static str = "bop";

    fn restore_state(
        _v: Vec<crate::cse::ChainState>,
        _windows: &[&[f64]],
    ) -> Option<Self::IndicatorState> {
        Some(IndicatorState)
    }
}

#[cfg(feature = "simd_options")]
impl crate::indicator_types::IndicatorByOptions<INPUTS, OPTIONS> for Bop {
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
