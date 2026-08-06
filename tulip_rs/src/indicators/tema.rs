use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{TIndicatorState, Indicator, TState, IndicatorResult};
use crate::indicators::dema::{
    Dema, State as DemaState,
};
pub use crate::indicators::ema::multiplier;
use crate::indicators::ema::{calc as calc_ema, Ema};
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold
};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::tema_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::tema_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::tema_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::tema_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
#[serde(bound="")]
pub struct State<S = Cold> {
    pub dema_state: DemaState<S>,
    pub ema3: f64,
}
impl<S> Deref for State<S> {
    type Target = DemaState<S>;
    fn deref(&self) -> &Self::Target { &self.dema_state }
}
impl<S> DerefMut for State<S> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.dema_state }
}
pub type IndicatorState = State<Warm>;
impl State<Cold> {
    
    pub fn init_state(
        real: &[f64],
        period: usize,
        (dema_line, ema_line): (&mut [f64], &mut [f64]),
    ) -> State<Warm> {

        // Phase 1+2: initialize ema1 and ema2 via DEMA init
        // processes real[0..=(period*2-3)]
        let mut dema_state = DemaState::init_state(real, period, ema_line);

        // Transition: advance ema1/ema2 one step, then seed ema3 from the updated ema2
        let seed_idx = /*real.len() - dema_capacity; / =*/ period*2-2;
        let (dema_val, ema_val) = dema_state.calc(real[seed_idx]);

        let mut state = State::<Warm> {
            ema3: dema_state.ema2,
            dema_state,
        };

        crate::init_store_optional_outputs!(seed_idx, real.len(),
            dema_line => dema_val,
            ema_line => ema_val
        );

        // Phase 3: full TEMA calc for remaining init bars
        //let remaining = real.len() - tema_capacity; // = period*3-3
        for i in (seed_idx + 1)..period*3-3 {
            let (_, dema, ema) = state.calc(real[i]);
            crate::init_store_optional_outputs!(i, real.len(),
                dema_line => dema,
                ema_line => ema
            );
        }

        state
    }

}

impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = (f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, value: Self::Inputs<'a>) -> Self::Outputs {
        let (dema, ema) = self.dema_state.calc(value);
        self.ema3 = calc_ema(self.dema_state.ema2, self.ema3, self.multiplier, self.inv_multiplier);
        (
            ema.mul_add(3.0, self.dema_state.ema2.mul_add(-3.0, self.ema3)),
            dema,
            ema,
        )
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut tema_line, mut dema_line, mut ema_line);
        {
            let capacity = inputs[0].len();
            tema_line = crate::uninit_vec!(f64, capacity);
            (dema_line, ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                dema_line: capacity,
                ema_line: capacity
            );
        }
        cycle_tema(
            inputs[0],
            self,
            &mut tema_line,
            (&mut dema_line, &mut ema_line),
        );
        Ok(vec![tema_line, dema_line, ema_line])
    }
}


/// Performs the main calculation loop for the TEMA indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `multipliers` - A tuple of EMA smoothing factors `(multiplier, inv_multiplier)`.
/// * `state` - A mutable reference to the current indicator state.
/// * `tema_line` - A mutable slice for storing the TEMA output values.
/// * `out_vecs` - A tuple of mutable slices for optional outputs `(dema_line, ema_line)`.
fn cycle_tema(
    real: &[f64],
    state: &mut State<Warm>,
    tema_line: &mut [f64],
    (dema_line, ema_line): (&mut [f64], &mut [f64]),
) {
    let (has_optional, want_dema, want_ema) = crate::calc_want_flags!(dema_line, ema_line);

    for i in 0..real.len() {
        let value = unsafe { *real.get_unchecked(i) };
        let (tema, dema, ema) = state.calc(value);
        unsafe { *tema_line.get_unchecked_mut(i) = tema };

        if has_optional {
            crate::store_optional_outputs!(i,
                want_dema, dema_line => dema,
                want_ema, ema_line => ema
            );
        }
    }
}


pub struct Tema;

impl Indicator<INPUTS, OPTIONS> for Tema {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "tema",
        indicator_type: IndicatorType::Trend,
        full_name: "Triple Exponential Moving Average",
        inputs: &["real"],
        options: &["period"],
        outputs: &["tema"],
        optional_outputs: &["dema", "ema"],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "tema",
            label: "EMAs",
            display_type: DisplayType::Overlay,
            outputs: &["tema", "dema", "ema"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize * 3 - 2
    }
    
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
    
        validate_inputs(inputs, Self::min_data(options))?;
        let (mut tema_line, mut dema_line, mut ema_line, mut state, real);
        {
            let len = inputs[0].len();
            let capacity = Self::output_length(len, options);
            let ema_capacity = Ema::output_length(len, options);
            let dema_capacity = Dema::output_length(len, options);
    
            tema_line = crate::uninit_vec!(f64, capacity);
    
            (dema_line, ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                dema_line: dema_capacity,
                ema_line: ema_capacity
            );
            let period = options[0] as usize;
            state = State::init_state(inputs[0], period, /*capacity,*/ (&mut dema_line, &mut ema_line));
            let start = len - capacity;
            real = &inputs[0][start..];
        }
        let optional_outputs = {
            let offsets = crate::slice_outputs_start!(tema_line.len(), dema_line, ema_line);
            (&mut dema_line[offsets.0..], &mut ema_line[offsets.1..])
        };
    
        // Perform the main TEMA calculation
        cycle_tema(
            real,
            &mut state,
            &mut tema_line,
            optional_outputs,
        );
    
        Ok((
            vec![tema_line, dema_line, ema_line],
            state,
        ))
    }
}