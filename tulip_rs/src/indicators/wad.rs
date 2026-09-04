use crate::common::validate_inputs;
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;
/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 0;

pub type IndicatorState = State;
#[derive(Serialize, Deserialize)]
pub struct State {
    pub prev_close: f64,
    pub wad: f64,
}
impl State {
    pub fn new(prev_close: f64, wad: f64) -> Self {
        Self { prev_close, wad }
    }
}
impl TState for State {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = f64;

    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, close): Self::Inputs<'a>) -> Self::Outputs {
        self.wad += if close > self.prev_close {
            close - self.prev_close.min(low)
        } else if close < self.prev_close {
            close - self.prev_close.max(high)
        } else {
            0.0
        };

        self.prev_close = close;

        self.wad
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut wad_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(inputs[0], inputs[1], inputs[2], self, &mut wad_line);

        Ok(vec![wad_line])
    }
}

/// Iterates over the high, low, and close slices and computes WAD values for each bar.
///
/// # Arguments
///
/// * `high` - Input high price slice.
/// * `low` - Input low price slice.
/// * `close` - Input close price slice.
/// * `state` - Mutable reference to the `IndicatorState` (previous close and cumulative WAD).
/// * `wad_line` - Mutable output slice for WAD values.
fn cycle(high: &[f64], low: &[f64], close: &[f64], state: &mut State, wad_line: &mut [f64]) {
    for i in 0..close.len() {
        unsafe {
            *wad_line.get_unchecked_mut(i) = state.calc((
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            ));
        }
    }
}

pub struct Wad;

impl Indicator<INPUTS, OPTIONS> for Wad {
    type IndicatorState = State;

    const INFO: Info = Info {
        name: "wad",
        full_name: "WAD Indicator",
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low", "close"],
        options: &[],
        outputs: &["wad"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "wad",
            label: "WAD",
            display_type: DisplayType::Indicator,
            outputs: &["wad"],
        }],
    };
    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        2
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        _options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
        // Expecting three inputs: High, Low, Close.
        validate_inputs(inputs, Self::min_data(_options))?;
        let [high, low, close] = *inputs;
        let mut wad_line: Vec<f64> = {
            let capacity = Self::output_length(high.len(), _options);
            crate::uninit_vec!(f64, capacity)
        };

        let mut state = IndicatorState::new(inputs[2][0], 0.0);

        cycle(
            &high[1..],
            &low[1..],
            &close[1..],
            &mut state,
            &mut wad_line,
        );

        // Store last used close and sum for incremental updates.
        Ok((vec![wad_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::wad_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
