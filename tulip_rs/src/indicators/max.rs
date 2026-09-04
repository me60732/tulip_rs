use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

use std::simd::{
    cmp::{SimdPartialEq, SimdPartialOrd},
    num::SimdFloat,
    Simd,
};

#[derive(Serialize, Deserialize)]
pub struct State<S = Cold> {
    pub max: f64,
    pub trail: usize,
    pub(crate) state: std::marker::PhantomData<S>,
}

impl State<Cold> {
    pub fn new(max: f64, trail: usize) -> Self {
        Self {
            max,
            trail,
            state: std::marker::PhantomData,
        }
    }
    pub(crate) fn into_warm(self) -> State<Warm> {
        State {
            max: self.max,
            trail: self.trail,
            state: std::marker::PhantomData,
        }
    }
    pub fn init_state(real: &[f64], look_back: usize) -> State<Warm> {
        let mut max = real[0];
        let mut trail = 0;

        for &bar in real.iter().take(look_back).skip(1) {
            if bar >= max {
                max = bar;
                trail = 0;
                continue;
            }
            trail += 1;
        }

        State {
            max,
            trail,
            state: std::marker::PhantomData,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (&'a [f64], usize, (usize, usize));
    type Outputs = (f64, usize);

    #[inline(always)]
    fn calc<'a>(&mut self, (real, i, (period, look_back)): Self::Inputs<'a>) -> (f64, usize) {
        let (mut max, mut trail) = (self.max, self.trail);
        trail += 1;

        if period <= trail {
            let search_start = i - look_back;
            let window = &real[search_start..=i];

            let (max_val, max_idx) = find_max_simd::<4>(window);
            max = max_val;
            trail = i - (search_start + max_idx);
        } else {
            let current = real[i];
            if current >= max {
                // >= to handle equal values correctly
                max = current;
                trail = 0;
            }
        }

        self.max = max;
        self.trail = trail;
        (max, trail)
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
        let (mut max, mut trail) = (self.max, self.trail);
        trail += 1;

        if period <= trail {
            let search_start = i - look_back;
            let window = real.get_unchecked(search_start..=i);

            let (max_val, max_idx) = match N {
                1 => find_max_scalar(window),
                _ => find_max_simd::<N>(window),
            };

            max = max_val;
            trail = i - (search_start + max_idx);
        } else {
            let current = *real.get_unchecked(i);
            if current >= max {
                max = current;
                trail = 0;
            }
        }

        self.max = max;
        self.trail = trail;
        (max, trail)
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

        let mut max_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle_max(&self.real, self.periods, &mut max_line, &mut self.state);

        self.real.drain(..self.real.len() - self.periods.1);

        Ok(vec![max_line])
    }
}

/// Performs the main calculation loop for the max indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `periods` - A tuple of `(period, look_back)` for the max calculation.
/// * `max_line` - A mutable slice for storing the max output values.
/// * `state` - A mutable reference to the current `State`.
fn cycle_max(real: &[f64], periods: (usize, usize), max_line: &mut [f64], state: &mut State<Warm>) {
    for (j, i) in (periods.1..real.len()).enumerate() {
        unsafe {
            *max_line.get_unchecked_mut(j) = state.calc_unchecked((real, i, periods)).0;
        }
    }
}

#[inline(always)]
pub(crate) fn find_max_scalar(window: &[f64]) -> (f64, usize) {
    let mut max_val = window[0];
    let mut max_idx = 0;

    for i in 1..window.len() {
        if window[i] >= max_val {
            // >= to get last position
            max_val = window[i];
            max_idx = i;
        }
    }
    (max_val, max_idx)
}

pub(crate) fn find_max_simd<const N: usize>(window: &[f64]) -> (f64, usize) {
    let mut global_max = Simd::<f64, N>::splat(unsafe { *window.get_unchecked(0) });
    let mut max_idx = 0;

    let search_window = unsafe { window.get_unchecked(1..) };

    let mut best_values = Simd::<f64, N>::splat(0.0);
    let mut best_start = usize::MAX; // sentinel: no chunk has updated yet
                                     // Process chunks with SIMD - direct iteration
    for (chunk_idx, chunk) in search_window.chunks_exact(N).enumerate() {
        let values = Simd::<f64, N>::from_slice(chunk);
        let mask = values.simd_ge(global_max);
        if mask.any() {
            global_max = Simd::splat(values.reduce_max());
            best_values = values; // save the chunk that holds the max
            best_start = chunk_idx;
        }
    }

    // Position finding done once outside the loop
    if best_start != usize::MAX {
        let i = if N <= 4 {
            best_values.simd_eq(global_max).to_bitmask().ilog2() as usize
        } else {
            let eq_mask = best_values.simd_eq(global_max);
            let mut i = N;
            while i > 0 {
                i -= 1;
                if unsafe { eq_mask.test_unchecked(i) } {
                    break;
                }
            }
            i
        };
        max_idx = best_start * N + 1 + i;
    }

    let mut global_max = global_max[0];
    // Handle remainder using find_max_scalar - calculate slice directly
    let processed_len = (search_window.len() / N) * N;
    let remainder = &search_window[processed_len..];
    if !remainder.is_empty() {
        let (rem_max, rem_idx) = find_max_scalar(remainder);
        if rem_max >= global_max {
            global_max = rem_max;
            max_idx = processed_len + 1 + rem_idx; // +1 for search_window offset
        }
    }

    (global_max, max_idx)
}

pub struct Max;

impl Indicator<INPUTS, OPTIONS> for Max {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "max",
        full_name: "maximum",
        indicator_type: IndicatorType::Price,
        inputs: &["real"],
        options: &["period"],
        outputs: &["max"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "max",
            label: "MAX",
            display_type: DisplayType::Overlay,
            outputs: &["max"],
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

        let mut max_line = {
            let capacity = Self::output_length(inputs[0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let mut state = State::init_state(real, periods.1);

        cycle_max(real, periods, &mut max_line, &mut state);

        Ok((vec![max_line], IndicatorState::new(real, state, periods)))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::max_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Max {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS],
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::max_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
