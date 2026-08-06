use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{TIndicatorState, Indicator, TState, IndicatorResult};
use crate::indicators::atr::State as AtrState;
use crate::indicators::dm::State as DMState;
use crate::indicators::tr::Tr;
pub use crate::indicators::wilders::multiplier;
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold
};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::di_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::di_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::di_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::di_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
#[serde(bound="")]
pub struct State<S = Cold> {
    pub di_state: DMState<S>,
    pub atr_state: AtrState<S>,
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        inputs: Self::Inputs<'a>
    ) -> Self::Outputs {
        let (dmup, dmdown, atr, tr) = self.calc_diup_didown(inputs);
    
        let atr_inv = 100.0 / atr;
        let mut pdi = dmup * atr_inv; // multiplication
        let mut mdi = dmdown * atr_inv;
        pdi = if pdi.is_nan() { 0.0 } else { pdi };
        mdi = if mdi.is_nan() { 0.0 } else { mdi };
        (pdi, mdi, atr, tr)
    }
}
impl State<Cold> {
    /*pub fn new(dm_state: (f64, f64, f64, f64), atr_state: (f64, f64)) -> Self {
        Self {
            atr_state: AtrState::new(atr_state.0, atr_state.1),
            di_state: DMState::new(dm_state.0, dm_state.1, dm_state.2, dm_state.3),
        }
    }*/
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        tr_line: &mut [f64],
    ) -> State<Warm> {
        let atr_state = AtrState::init_state(high, low, close, period, tr_line, true);
        let di_state = DMState::init_state(high, low, period);

        State {
            atr_state,
            di_state,
        }
    }
}
impl State<Warm> {
    #[inline(always)]
    pub fn calc_diup_didown(
        &mut self,
        (high, low, close): (f64, f64, f64),
    ) -> (f64, f64, f64, f64) {
        let (dmup, dmdown) = self.di_state.calc((high, low));
        let (atr, tr) = self.atr_state.partial_calc((high, low, close));
        (dmup, dmdown, atr, tr)
    }
}
pub type IndicatorState = State<Warm>;

impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut plus_di_line, mut minus_di_line, mut atr_line, mut tr_line);
        {
            let capacity = inputs[0].len();
            plus_di_line = crate::uninit_vec!(f64, capacity);
            minus_di_line = crate::uninit_vec!(f64, capacity);

            (atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                atr_line: capacity,
                tr_line: capacity
            );
        }
        let [high, low, close] = inputs;
        cycle_calc(
            high,
            low,
            close,
            self,
            (&mut plus_di_line, &mut minus_di_line),
            (&mut atr_line, &mut tr_line),
        );

        Ok(vec![plus_di_line, minus_di_line, atr_line, tr_line])
    }
}


/// Performs the main calculation loop for the DI indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `state` - Mutable reference to the DI state (DM and ATR sub-states).
/// * `inv_multiplier` - The inverse Wilder's multiplier used to scale ATR output.
/// * `outputs` - A tuple of `(plus_di_line, minus_di_line)` output slices.
/// * `out_vecs` - A tuple of `(atr_line, tr_line)` for optional outputs.
fn cycle_calc(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    state: &mut State<Warm>,
    outputs: (&mut [f64], &mut [f64]),
    out_vecs: (&mut [f64], &mut [f64]),
) {
    let (plus_di_line, minus_di_line) = outputs;
    let (atr_line, tr_line) = out_vecs;
    let (has_optional, want_atr, want_tr) = crate::calc_want_flags!(atr_line, tr_line);

    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };

        let (pdi, mdi, atr, tr) = state.calc(inputs);

        unsafe {
            *plus_di_line.get_unchecked_mut(i) = pdi;
            *minus_di_line.get_unchecked_mut(i) = mdi;
        }
        if has_optional {
            crate::store_optional_outputs!(i,
                want_tr, tr_line => tr
            );
            crate::store_optional_outputs_corrected!(i,
                want_atr, atr_line => corrected(atr, state.atr_state.wilders_state.inv_multiplier)
            );
        }
    }
}

pub struct Di;
impl Indicator<INPUTS, OPTIONS> for Di {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "di",
        full_name: "Directional Indicator",
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["plus_di", "minus_di"],
        optional_outputs: &["atr", "tr"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "di",
                label: "DI",
                display_type: DisplayType::Indicator,
                outputs: &["plus_di", "minus_di"],
            },
            DisplayGroup {
                offset: None,
                id: "atr_tr",
                label: "True Range",
                display_type: DisplayType::Indicator,
                outputs: &["atr", "tr"],
            },
        ],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
    
        validate_inputs(inputs, Self::min_data(options))?;
        let high = inputs[0];
        let low = inputs[1];
        let close = inputs[2];
    
        let (mut plus_di_line, mut minus_di_line, mut atr_line, mut tr_line);
        {
            let capacity = Self::output_length(high.len(), options);
            let tr_capacity = Tr::output_length(high.len(), &[]);
    
            plus_di_line = crate::uninit_vec!(f64, capacity);
            minus_di_line = crate::uninit_vec!(f64, capacity);
    
            (atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                atr_line: capacity,
                tr_line: tr_capacity
            );
        }
        let mut state = State::init_state(high, low, close, period, &mut tr_line);
        let tr = {
            let offsets = crate::slice_outputs_start!(plus_di_line.len(), tr_line);
            &mut tr_line[offsets..]
        };
        let (high, low, close) = { (&high[period..], &low[period..], &close[period..]) };
        cycle_calc(
            high,
            low,
            close,
            &mut state,
            (&mut plus_di_line, &mut minus_di_line),
            (&mut atr_line, tr),
        );
    
        Ok((
            vec![plus_di_line, minus_di_line, atr_line, tr_line],
            state,
        ))
    }
}