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

pub type IndicatorState = State;
#[derive(Serialize, Deserialize)]
pub struct State {
    pub nvi: f64,
    pub close: f64,
    pub volume: f64,
}
impl State {
    #[inline(always)]
    pub fn new(nvi: f64, close: f64, volume: f64) -> Self {
        Self { nvi, close, volume }
    }

    fn cycle(&mut self, close: &[f64], volume: &[f64], nvi_line: &mut [f64]) {
        for i in 0..close.len() {
            unsafe {
                *nvi_line.get_unchecked_mut(i) =
                    self.calc((*close.get_unchecked(i), *volume.get_unchecked(i)));
            }
        }
    }
}
impl TState for State {
    type Inputs<'a> = (f64, f64);
    type Outputs = f64;
    #[inline(always)]
    fn calc<'a>(&mut self, (close, volume): Self::Inputs<'a>) -> Self::Outputs {
        if volume < self.volume {
            //return nvi + (close - prev_close) / prev_close * nvi
            self.nvi = close / self.close * self.nvi;
        }
        (self.close, self.volume) = (close, volume);
        self.nvi
    }
}
impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut nvi_line = crate::uninit_vec!(f64, inputs[0].len());

        self.cycle(inputs[0], inputs[1], &mut nvi_line);

        Ok(vec![nvi_line])
    }
}

/// Iterates over the input data and applies the calc function.
fn cycle(close: &[f64], volume: &[f64], nvi_line: &mut [f64], mut nvi: f64) {
    for (j, i) in (1..close.len()).enumerate() {
        unsafe {
            nvi = calc(
                close.get_unchecked(i),
                close.get_unchecked(j),
                volume.get_unchecked(i),
                volume.get_unchecked(j),
                nvi,
            );
            *nvi_line.get_unchecked_mut(j) = nvi;
        }
    }
}

/// Performs the core calculation for the Negative Volume Index (NVI) indicator.
#[inline(always)]
pub fn calc(close: &f64, prev_close: &f64, volume: &f64, prev_volume: &f64, nvi: f64) -> f64 {
    if volume < prev_volume {
        //return nvi + (close - prev_close) / prev_close * nvi
        return close / prev_close * nvi;
    }

    nvi
}

pub struct Nvi;
impl Indicator<INPUTS, OPTIONS> for Nvi {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "nvi",
        full_name: "Negative Volume Index",
        indicator_type: IndicatorType::Volume,
        inputs: &["close", "volume"],
        options: &[],
        outputs: &["nvi"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "nvi",
            label: "NVI",
            display_type: DisplayType::Indicator,
            outputs: &["nvi"],
        }],
    };

    /// Returns the minimum amount of data required for the NVI indicator.
    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        2
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        _options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_inputs(inputs, Self::min_data(_options))?;

        let close = inputs[0];
        let volume = inputs[1];

        let mut nvi_line = {
            let capacity = Self::output_length(close.len(), _options);
            crate::uninit_vec!(f64, capacity)
        };

        cycle(close, volume, &mut nvi_line, 1000.0);
        let nvi = nvi_line[nvi_line.len() - 1];
        Ok((
            vec![nvi_line],
            IndicatorState {
                nvi,
                close: close[close.len() - 1],
                volume: volume[volume.len() - 1],
            },
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::nvi_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
