use crate::common::validate_inputs;
pub use crate::indicator_types::{TIndicatorState, Indicator, IndicatorResult, TState};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::psar_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::psar_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::psar_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::psar_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State<Warm>,
    high: Vec<f64>,
    low: Vec<f64>,
}
impl IndicatorState {
    pub fn new(state: State<Warm>, high: &[f64], low: &[f64]) -> Self {
        Self {
            state,
            high: high[high.len() - 2..].to_vec(),
            low: low[low.len() - 2..].to_vec(),
        }
    }
}
impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.high.extend_from_slice(inputs[0]);
        self.low.extend_from_slice(inputs[1]);

        let mut psar_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_psar(
            (&self.high, &self.low),
            &mut psar_line,
            &mut self.state
        );
        self.high.drain(..self.high.len() - 2);
        self.low.drain(..self.low.len() - 2);
        Ok(vec![psar_line])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State<S = Cold> {
    pub psar: f64,
    pub extream: f64,
    pub accel: f64,
    pub af_step: f64,
    pub max_af: f64,
    pub uptrend: bool,
    pub(crate) state: std::marker::PhantomData<S>,
}
impl State<Cold> {
    pub fn new(high: &[f64], low: &[f64], af_step: f64, max_af: f64) -> Self {
        let (uptrend, extream, psar) = if high[0] + low[0] <= high[1] + low[1] {
            (true, high[0], low[0])
        } else {
            (false, low[0], high[0])
        };
        State {
            psar,
            extream,
            uptrend,
            accel: af_step,
            af_step,
            max_af,
            state: std::marker::PhantomData::<Cold>,
        }
    }
    pub(crate) fn into_warm(self) -> State<Warm> {
        State {
            psar: self.psar,
            extream: self.extream,
            accel: self.accel,
            af_step: self.af_step,
            max_af: self.max_af,
            uptrend: self.uptrend,
            state: std::marker::PhantomData::<Warm>,
        }
    }
    pub fn init_state(high: &[f64], low: &[f64], af_step: f64, max_af: f64, psar_line: &mut [f64] ) -> State<Warm> {
        let mut state = Self::new(high, low, af_step, max_af).into_warm();
        psar_line[0] = state.calc((high, low, 1));
        state
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (&'a [f64], &'a [f64], usize);
    type Outputs = f64;
    
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (high, low, i): Self::Inputs<'a>
    ) -> f64 {
        let (mut psar, mut extream, mut uptrend, mut accel, af_step, max_af) =
            (self.psar, self.extream, self.uptrend, self.accel, self.af_step, self.max_af);
    
        // Use += for potential FMA optimization
        //psar += (extream - psar) * accel;
        psar = accel.mul_add(extream - psar, psar);
        if uptrend {
            // Keep original branch structure for better prediction
            if i >= 2 && psar > low[i - 2] {
                psar = low[i - 2];
            }
            if psar > low[i - 1] {
                psar = low[i - 1];
            }
    
            // Combined condition for extreme and acceleration
            if high[i] > extream {
                extream = high[i];
                accel = (accel + af_step).min(max_af);
            }
        } else {
            if i >= 2 && psar < high[i - 2] {
                psar = high[i - 2];
            }
            if psar < high[i - 1] {
                psar = high[i - 1];
            }
    
            if low[i] < extream {
                extream = low[i];
                accel = (accel + af_step).min(max_af);
            }
        }
    
        if (uptrend && low[i] < psar) || (!uptrend && high[i] > psar) {
            uptrend = !uptrend;
            psar = extream;
            accel = af_step;
            extream = if uptrend { high[i] } else { low[i] };
        }
    
        (self.psar, self.extream, self.uptrend, self.accel) = (psar, extream, uptrend, accel);
        psar
    }
    #[inline(always)]
    unsafe fn calc_unchecked(
        &mut self,
        (high, low, i): (&[f64], &[f64], usize)
    ) -> f64 {
        let (mut psar, mut extream, mut accel, af_step, max_af, mut uptrend) =
            (self.psar, self.extream, self.accel, self.af_step, self.max_af, self.uptrend);
        let (h, prev_high, old_high, l, prev_low, old_low) = {
            let prev = i-1;
            let before = i-2;
            (
                *high.get_unchecked(i),
                *high.get_unchecked(prev),
                *high.get_unchecked(before),
                *low.get_unchecked(i),
                *low.get_unchecked(prev),
                *low.get_unchecked(before)
            )
        };
    
        //psar += (extream - psar) * accel;
        psar = accel.mul_add(extream - psar, psar);

        if uptrend {
            if psar > old_low {
                psar = old_low;
            }
            if psar > prev_low {
                psar = prev_low;
            }
    
            if h > extream {
                extream = h;
                accel = (accel + af_step).min(max_af);
            }
        } else {
            if psar < old_high {
                psar = old_high;
            }
            if psar < prev_high {
                psar = prev_high;
            }
    
            if l < extream {
                extream = l;
                accel = (accel + af_step).min(max_af);
            }
        }
    
        if (uptrend && l < psar) || (!uptrend && h > psar) {
            uptrend = !uptrend;
            psar = extream;
            accel = af_step;
            extream = if uptrend { h } else { l };
        }
    
        (self.psar, self.extream, self.accel, self.uptrend) = (psar, extream, accel, uptrend);
        psar
    }
}
pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] <= 0.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}


/// Iterates over the input data and applies the calc function.
fn cycle_psar(
    (high, low): (&[f64], &[f64]),
    psar_line: &mut [f64],
    state: &mut State<Warm>,
) {

    for (j, i) in (2..high.len()).enumerate() {
        unsafe {
            *psar_line.get_unchecked_mut(j) = state.calc_unchecked((high, low, i));
        }
    }
}

pub struct Psar;
impl Indicator<INPUTS, OPTIONS> for Psar {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "psar",
        full_name: "Parabolic SAR",
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low"],
        options: &["acceleration_factor", "max_acceleration_factor"],
        outputs: &["psar"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "psar",
            label: "PSAR",
            display_type: DisplayType::Overlay,
            outputs: &["psar"],
        }],
    };
    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        2
    }
    
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let af_step = options[0];
        let max_af = options[1];
    
        validate_inputs(inputs, Self::min_data(options))?;
    
        let high = inputs[0];
        let low = inputs[1];
    
        
        let mut psar_line = {
            let capacity = Self::output_length(high.len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let mut state = State::init_state(high, low, af_step, max_af, &mut psar_line);
        
        cycle_psar(
            (high, low),
            &mut psar_line[1..],
            &mut state,
        );
    
        Ok((
            vec![psar_line],
            IndicatorState::new(state, high, low),
        ))
    }
}