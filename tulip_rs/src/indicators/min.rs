use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
//use std::slice::
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

use std::{
    f64,
    simd::{
        cmp::{SimdPartialEq, SimdPartialOrd},
        num::SimdFloat,
        Simd,
    },
};
#[derive(Serialize, Deserialize)]
pub struct State<S = Cold> {
    pub min: f64,
    pub trail: usize,
    pub(crate) state: std::marker::PhantomData<S>,
}

impl State<Cold> {
    pub fn new(min: f64, trail: usize) -> Self {
        State {
            min,
            trail,
            state: std::marker::PhantomData,
        }
    }
    pub(crate) fn into_warm(self) -> State<Warm> {
        State {
            min: self.min,
            trail: self.trail,
            state: std::marker::PhantomData,
        }
    }
    pub fn init_state(real: &[f64], look_back: usize) -> State<Warm> {
        let mut min = real[0];
        let mut trail = 0;
        for &bar in real.iter().take(look_back).skip(1) {
            if bar <= min {
                min = bar;
                trail = 0;
                continue;
            }
            trail += 1;
        }

        State {
            min,
            trail,
            state: std::marker::PhantomData,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (&'a [f64], usize, (usize, usize));
    type Outputs = (f64, usize);
    #[inline(always)]
    fn calc<'a>(&mut self, (real, i, (period, look_back)): Self::Inputs<'a>) -> Self::Outputs {
        let (mut min, mut trail) = (self.min, self.trail);
        trail += 1;

        if period <= trail {
            let search_start = i - look_back;
            let window = &real[search_start..=i];

            let (min_val, min_idx) = find_min_simd::<4>(window);
            min = min_val;
            trail = i - (search_start + min_idx);
        } else {
            let current = real[i];
            if current <= min {
                min = current;
                trail = 0;
            }
        }

        self.min = min;
        self.trail = trail;
        (min, trail)
    }
    #[inline(always)]
    unsafe fn calc_unchecked(&mut self, inputs: Self::Inputs<'_>) -> Self::Outputs {
        self.calc_chuncked_unchecked::<4>(inputs)
    }
}
impl State<Warm> {
    #[inline(always)]
    pub unsafe fn calc_chuncked_unchecked<const N: usize>(
        &mut self,
        (real, i, (period, look_back)): (&[f64], usize, (usize, usize)),
    ) -> (f64, usize) {
        let (mut min, mut trail) = (self.min, self.trail);
        trail += 1;

        if period <= trail {
            let search_start = i - look_back;
            let window = real.get_unchecked(search_start..=i);

            let (min_val, min_idx) = match N {
                1 => find_min_scalar(window),
                _ => find_min_simd::<N>(window),
            };

            min = min_val;
            trail = i - (search_start + min_idx);
        } else {
            let current = *real.get_unchecked(i);
            if current <= min {
                min = current;
                trail = 0;
            }
        }

        self.min = min;
        self.trail = trail;
        (min, trail)
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    pub real: Vec<f64>,
    pub state: State<Warm>,
    pub periods: (usize, usize),
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State<Warm>, periods: (usize, usize)) -> Self {
        Self {
            real: real[real.len() - periods.1..].to_vec(),
            state,
            periods,
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

        let mut min_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_min(&self.real, self.periods, &mut min_line, &mut self.state);

        self.real.drain(..self.real.len() - self.periods.1);

        Ok(vec![min_line])
    }
}

/// Performs the main calculation loop for the min indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `periods` - A tuple of `(period, look_back)` for the min calculation.
/// * `min_line` - A mutable slice for storing the min output values.
/// * `state` - A mutable reference to the current `State`.
fn cycle_min(real: &[f64], periods: (usize, usize), min_line: &mut [f64], state: &mut State<Warm>) {
    for (j, i) in (periods.0 - 1..real.len()).enumerate() {
        unsafe {
            *min_line.get_unchecked_mut(j) = state.calc_unchecked((real, i, periods)).0;
        }
    }
}

#[inline(always)]
pub(crate) fn find_min_scalar(window: &[f64]) -> (f64, usize) {
    let end = window.len() - 1;
    let mut min_val = unsafe { *window.get_unchecked(end) };
    let mut min_idx = end;
    let mut i = end;

    while i > 0 {
        i -= 1;
        let val = unsafe { *window.get_unchecked(i) };
        if val < min_val {
            min_val = val;
            min_idx = i;
        }
    }

    (min_val, min_idx)
}

pub(crate) fn find_min_simd<const N: usize>(window: &[f64]) -> (f64, usize) {
    let mut global_min = Simd::<f64, N>::splat(unsafe { *window.get_unchecked(0) });
    let mut min_idx = 0;

    let search_window = unsafe { window.get_unchecked(1..) };

    let mut best_values = Simd::<f64, N>::splat(0.0);
    let mut best_start = usize::MAX;

    for (chunk_idx, chunk) in search_window.chunks_exact(N).enumerate() {
        let values = Simd::<f64, N>::from_slice(chunk);
        let mask = values.simd_le(global_min);
        if mask.any() {
            global_min = Simd::splat(values.reduce_min());
            best_values = values;
            best_start = chunk_idx;
        }
    }

    if best_start != usize::MAX {
        let i = if N <= 4 {
            best_values.simd_eq(global_min).to_bitmask().ilog2() as usize
        } else {
            let eq_mask = best_values.simd_eq(global_min);
            let mut i = N;
            while i > 0 {
                i -= 1;
                if unsafe { eq_mask.test_unchecked(i) } {
                    break;
                }
            }
            i
        };
        min_idx = best_start * N + 1 + i;
    }

    let mut global_min = global_min[0];
    let processed_len = (search_window.len() / N) * N;
    let remainder = &search_window[processed_len..];
    if !remainder.is_empty() {
        let (rem_min, rem_idx) = find_min_scalar(remainder);
        if rem_min <= global_min {
            global_min = rem_min;
            min_idx = processed_len + 1 + rem_idx;
        }
    }

    (global_min, min_idx)
}

pub struct Min;

impl Indicator<INPUTS, OPTIONS> for Min {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "min",
        full_name: "minimum",
        indicator_type: IndicatorType::Price,
        inputs: &["real"],
        options: &["period"],
        outputs: &["min"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "min",
            label: "MIN",
            display_type: DisplayType::Overlay,
            outputs: &["min"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let periods = (options[0] as usize, options[0] as usize - 1);

        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];

        let mut min_line = {
            let capacity = Self::output_length(inputs[0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let mut state = State::init_state(real, periods.0);
        cycle_min(real, periods, &mut min_line, &mut state);

        Ok((
            vec![min_line],
            Self::IndicatorState::new(real, state, periods),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::min_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Min {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::min_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
