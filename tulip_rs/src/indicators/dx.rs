use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
pub use crate::indicators::di::State as DiState;
use crate::indicators::tr::Tr;
pub use crate::indicators::wilders::multiplier;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::dx_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::dx_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::dx_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::dx_simd::indicator_by_options as indicator;
}
pub type IndicatorState = State<Warm>;
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
#[repr(transparent)]
pub struct State<S = Cold>(pub DiState<S>);
impl<S> Deref for State<S> {
    type Target = DiState<S>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<S> DerefMut for State<S> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        let (_, _, atr, tr) = self.calc_diup_didown(inputs);

        let dx = self.calc_dx();

        (dx, atr, tr)
    }
}
impl State<Cold> {
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        tr_line: &mut [f64],
    ) -> State<Warm> {
        State(DiState::init_state(high, low, close, period, tr_line))
    }
}
impl State<Warm> {
    #[inline(always)]
    pub fn calc_dx(&mut self) -> f64 {
        let di_up = self.di_state.dmup; // / state.atr_state.atr;
        let di_down = self.di_state.dmdown; // / state.atr_state.atr;

        let dm_diff = (di_up - di_down).abs();
        let dm_sum = di_up + di_down;
        (dm_diff * 100.0 / dm_sum).max(0.0)
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        //let mut dx_line = vec![0.0; capacity];
        let (mut dx_line, mut atr_line, mut tr_line);
        {
            let capacity = inputs[0].len();
            dx_line = crate::uninit_vec!(f64, capacity);
            (atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                atr_line: capacity,
                tr_line: capacity
            );
        }
        let [high, low, close] = inputs;
        cycle(
            high,
            low,
            close,
            self,
            (&mut dx_line, &mut atr_line, &mut tr_line),
        );
        Ok(vec![dx_line, atr_line, tr_line])
    }
}

/// Performs the main calculation loop for the DX indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `close` - A slice of close prices.
/// * `state` - A mutable reference to the indicator state.
/// * `inv_multiplier` - The inverse smoothing multiplier used for ATR calculation.
/// * `out_vecs` - A tuple of mutable output slices: `(dx_line, atr_line, tr_line)`.
fn cycle(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    state: &mut State<Warm>,
    out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (dx_line, atr_line, tr_line) = out_vecs;
    let (has_optional, want_atr, want_tr) = crate::calc_want_flags!(atr_line, tr_line);

    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };

        let (dx, atr, tr) = TState::calc(state, inputs);
        unsafe {
            *dx_line.get_unchecked_mut(i) = dx;
        }
        if has_optional {
            crate::store_optional_outputs_corrected!(i,
                want_atr, atr_line => corrected(atr, state.atr_state.wilders_state.inv_multiplier)
            );
            crate::store_optional_outputs!(i,
                want_tr, tr_line => tr
            );
        }
    }
}

pub struct Dx;

impl Indicator<INPUTS, OPTIONS> for Dx {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "dx",
        full_name: "Directional Movement Index",
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low", "close"],
        options: &["period"],
        outputs: &["dx"],
        optional_outputs: &["atr", "tr"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "dx",
                label: "DX",
                display_type: DisplayType::Indicator,
                outputs: &["dx"],
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

        let capacity = Self::output_length(inputs[0].len(), options);
        let tr_capacity = Tr::output_length(inputs[0].len(), &[]);
        let (mut dx_line, mut atr_line, mut tr_line);
        {
            dx_line = crate::uninit_vec!(f64, capacity);
            (atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                atr_line: capacity,
                tr_line: tr_capacity
            );
        }

        let mut state = State::init_state(inputs[0], inputs[1], inputs[2], period, &mut tr_line);
        let tr = {
            let offset = crate::slice_outputs_start!(dx_line.len(), tr_line);
            &mut tr_line[offset..]
        };
        let (high, low, close) = (
            &inputs[0][period..],
            &inputs[1][period..],
            &inputs[2][period..],
        );
        cycle(
            high,
            low,
            close,
            &mut state,
            (&mut dx_line, &mut atr_line, tr),
        );

        Ok((vec![dx_line, atr_line, tr_line], state))
    }
}
