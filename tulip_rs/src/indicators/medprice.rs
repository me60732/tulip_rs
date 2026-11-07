use crate::common::validate_inputs;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 0;

pub struct State;

impl TState for State {
    type Inputs<'a> = (f64, f64);
    type Outputs = f64;
    #[inline(always)]
    fn calc(&mut self, (high, low): Self::Inputs<'_>) -> Self::Outputs {
        0.5 * (high + low)
    }
}
#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState;

impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        process(inputs)
    }
}

#[inline(always)]
fn process(inputs: &[&[f64]; INPUTS]) -> Result<Vec<Vec<f64>>, IndicatorError> {
    validate_inputs(inputs, 1)?;
    let high = inputs[0];
    let low = inputs[1];

    let mut medprice_line = crate::uninit_vec!(f64, high.len());
    let mut state = State;
    for (i, (&high_value, &low_value)) in high.iter().zip(low.iter()).enumerate() {
        unsafe { *medprice_line.get_unchecked_mut(i) = state.calc((high_value, low_value)) };
    }

    Ok(vec![medprice_line])
}

/// Calculates the median price.
#[inline(always)]
pub fn calc(high: f64, low: f64) -> f64 {
    0.5 * (high + low)
}

pub struct Medprice;
impl Indicator<INPUTS, OPTIONS> for Medprice {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "medprice",
        full_name: "Median Price",
        indicator_type: IndicatorType::Price,
        inputs: &["high", "low"],
        options: &[],
        outputs: &["medprice"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "medprice",
            label: "MEDPRICE",
            display_type: DisplayType::Overlay,
            outputs: &["medprice"],
        }],
    };

    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        1 // Only one data point is needed to calculate the median price
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
        inputs: &[&[&[f64]; INPUTS]; N],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::medprice_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

// ── planner recipe (cse) ─────────────────────────────────────────────

impl crate::cse::Planable for Medprice {
    /// The graph is leaf data — `crate::cse::recipes::medprice`
    /// (`0.5 · (high + low)`, stateless, every bar publishes).
    fn recipe(_options: &[f64]) -> crate::cse::Recipe {
        crate::cse::recipes::medprice(_options)
    }
}

#[cfg(feature = "simd_options")]
impl crate::indicator_types::IndicatorByOptions<INPUTS, OPTIONS> for Medprice {
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

/// Fully stateless — `medprice::IndicatorState` is a zero-field unit
/// struct, and the recipe carries no `tail_inputs` chain.
impl crate::cse::Restorable<INPUTS, OPTIONS> for Medprice {
    const KIND: &'static str = "medprice";

    fn restore_state(
        v: Vec<crate::cse::ChainState>,
        _windows: &[&[f64]],
    ) -> Option<Self::IndicatorState> {
        if !v.is_empty() {
            return None;
        }
        Some(IndicatorState)
    }
}
