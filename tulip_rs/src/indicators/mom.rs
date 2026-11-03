use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState {
    real: Vec<f64>,
    period: usize,
}
impl IndicatorState {
    pub fn new(real: &[f64], period: usize) -> Self {
        Self {
            period,
            real: real[real.len() - period..].to_vec(),
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.real.extend_from_slice(inputs[0]);

        let mut mom_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_mom(&self.real, self.period, &mut mom_line);

        self.real.drain(..self.real.len() - self.period);

        Ok(vec![mom_line])
    }
}

/// Iterates over the input data and applies the calc function.
fn cycle_mom(real: &[f64], period: usize, mom_line: &mut [f64]) {
    for (j, i) in (period..real.len()).enumerate() {
        unsafe {
            *mom_line.get_unchecked_mut(j) = calc(*real.get_unchecked(i), *real.get_unchecked(j))
        };
    }
}

/// Performs the core calculation for the Momentum (MOM) indicator.
#[inline(always)]
pub fn calc(real: f64, prev_real: f64) -> f64 {
    real - prev_real
}

pub struct Mom;
impl Indicator<INPUTS, OPTIONS> for Mom {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "mom",
        full_name: "Momentum",
        indicator_type: IndicatorType::Momentum,
        inputs: &["real"],
        options: &["period"],
        outputs: &["mom"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "mom",
            label: "MOM",
            display_type: DisplayType::Indicator,
            outputs: &["mom"],
        }],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];

        let mut mom_line = {
            let capacity = Self::output_length(real.len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        cycle_mom(real, period, &mut mom_line);

        Ok((vec![mom_line], IndicatorState::new(real, period)))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::mom_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Mom {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::mom_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

// ── planner recipe (cse) ─────────────────────────────────────────────

/// The engine's recipe wiring for this indicator — the graph lives in
/// `cse::recipes::mom`, so the module keeps its math and the engine keeps
/// its blindness to indicator identity.
///
/// `MOM(p) = real[i] − real[i − p]`, and its `Lag` depth is a VALUE
/// (`Values::lag_n`), not structure. The hand's `cycle_mom` pairs bar `i` with
/// bar `i − p` and publishes from bar `p` (`min_data = p + 1`,
/// `output_length = len − p`), so this is the pattern where a steady-loop read
/// of the WRONG depth shows up in the published row — `cmo`'s `Lag{…, period}`
/// never could, because there the lag only feeds a `Mirror` (state).
impl crate::cse::Planable for Mom {
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        crate::cse::recipes::mom(options)
    }
}

/// `mom::IndicatorState` carries no engine chain at all — its hand state is
/// a raw window of the last `p` `real` values (see the recipe's own doc:
/// `tail_inputs` stays empty). `period` isn't recoverable from `v` (there's
/// nothing to destructure), but the host-side window IS exactly `period`
/// bars long by construction (`IndicatorState::new` stores
/// `real[real.len() - period..]`), so `windows[0].len()` IS `period`.
impl crate::cse::Restorable<INPUTS, OPTIONS> for Mom {
    const KIND: &'static str = "mom";

    fn restore_state(
        v: Vec<crate::cse::ChainState>,
        windows: &[&[f64]],
    ) -> Option<Self::IndicatorState> {
        if !v.is_empty() {
            return None;
        }
        let period = windows[0].len();
        if period == 0 {
            return None;
        }
        Some(IndicatorState::new(windows[0], period))
    }
}
