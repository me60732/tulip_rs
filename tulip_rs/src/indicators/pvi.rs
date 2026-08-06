use crate::common::validate_inputs;
pub use crate::indicator_types::{TIndicatorState, Indicator, IndicatorResult, TState};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 0;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::pvi_simd::indicator_by_assets;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::pvi_simd::indicator_by_assets as indicator;
}
pub type IndicatorState = State;
#[derive(Serialize, Deserialize)]
pub struct State {
    pub pvi: f64,
    pub close: f64,
    pub volume: f64,
}
impl State {
    #[inline(always)]
    pub fn new(pvi: f64, close: f64, volume: f64) -> Self {
        Self { pvi, close, volume }
    }
    
    fn cycle(&mut self, close: &[f64], volume: &[f64], pvi_line: &mut [f64]) {
        for i in 0..close.len() {
            unsafe {
                *pvi_line.get_unchecked_mut(i) =
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
        if volume > self.volume {
            //return pvi + (close - prev_close) / prev_close * pvi
            self.pvi = close / self.close * self.pvi;
        }
        (self.close, self.volume) = (close, volume);
        self.pvi
    }
}
impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let close = inputs[0];
        let volume = inputs[1];

        let mut pvi_line = crate::uninit_vec!(f64, close.len());

        self.cycle(&close, &volume, &mut pvi_line);

        Ok(vec![pvi_line])
    }
}


/// Iterates over the input data and applies the calc function.
fn cycle(close: &[f64], volume: &[f64], pvi_line: &mut [f64], mut pvi: f64) {
    for (j, i) in (1..close.len()).enumerate() {
        unsafe {
            pvi = calc(
                close.get_unchecked(i),
                close.get_unchecked(j),
                volume.get_unchecked(i),
                volume.get_unchecked(j),
                pvi,
            );
            *pvi_line.get_unchecked_mut(j) = pvi;
        }
    }
}

/// Performs the core calculation for the Positive Volume Index (PVI) indicator.
#[inline(always)]
pub fn calc(close: &f64, prev_close: &f64, volume: &f64, prev_volume: &f64, pvi: f64) -> f64 {
    if volume > prev_volume {
        //return pvi + (close - prev_close) / prev_close * pvi
        return close / prev_close * pvi;
    }

    pvi
}

pub struct Pvi;

impl Indicator<INPUTS, OPTIONS> for Pvi {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "pvi",
        full_name: "Positive Volume Index",
        indicator_type: IndicatorType::Volume,
        inputs: &["close", "volume"],
        options: &[],
        outputs: &["pvi"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "pvi",
            label: "PVI",
            display_type: DisplayType::Indicator,
            outputs: &["pvi"],
        }],
    };

    /// Returns the minimum amount of data required for the pvi indicator.
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
        let mut pvi_line = {
            let capacity = Self::output_length(close.len(), _options);
            crate::uninit_vec!(f64, capacity)
        };
    
        cycle(close, volume, &mut pvi_line, 1000.0);
        let pvi = pvi_line[pvi_line.len() - 1];
        Ok((
            vec![pvi_line],
            IndicatorState {
                pvi,
                close: close[close.len() - 1],
                volume: volume[volume.len() - 1],
            },
        ))
    }
}