use crate::common::{validate_inputs, validate_options};
use crate::common_simd::{deserialize_f64x2, serialize_f64x2};
pub use crate::indicator_types::{TIndicatorState, Indicator, TState, IndicatorResult};
use crate::indicators::tr::{Tr, State as TrState};
use crate::ring_buffer::multi_type_buffer::MultiTypeBuffer;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};

use serde::{Deserialize, Serialize};
//use wide::*;
use std::simd::{num::SimdFloat, Simd};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::vortex_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::vortex_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::vortex_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    pub use crate::indicators::simd_indicators::vortex_simd::indicator_by_options as indicator;
}


impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let [high, low, close] = inputs;
        let (mut vi_up_line, mut vi_down_line, mut tr_line) = {
            let len = high.len();
            (
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    tr_line_line: len
                ),
            )
        };

        cycle(
            (high, low, close),
            self,
            (&mut vi_up_line, &mut vi_down_line),
            &mut tr_line,
        );

        Ok(vec![vi_up_line, vi_down_line, tr_line])
    }
}
pub type IndicatorState = State<Warm>;

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub buffer: MultiTypeBuffer<(f64, Simd<f64, 2>), S>,

    #[serde(
        serialize_with = "serialize_f64x2",
        deserialize_with = "deserialize_f64x2"
    )]
    pub vm_sums: Simd<f64, 2>,
    #[serde(
        serialize_with = "serialize_f64x2",
        deserialize_with = "deserialize_f64x2"
    )]
    pub prev_low_high: Simd<f64, 2>,

    pub tr_state: TrState,
    pub tr_sum: f64,
}

impl State<Cold> {
    pub fn new(period: usize, prev_high: f64, prev_low: f64, prev_close: f64) -> Self {
        Self {
            buffer: MultiTypeBuffer::new(period),
            prev_low_high: Simd::from_array([prev_low, prev_high]),
            tr_state: TrState::new(prev_close),
            tr_sum: 0.0,
            vm_sums: Simd::splat(0.0),
        }
    }
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        tr_line: &mut [f64],
    ) -> State<Warm> {
        let mut state = Self::new(period, high[0], low[0], close[0]);
        let mut i = 1;

        while !state.buffer.is_full() {
            let tr = state.init_calc((high[i], low[i], close[i]));
            if tr_line.len() > 0 {
                tr_line[i - 1] = tr;
            }
            i += 1;
        }
        State {
            buffer: state.buffer.into_full(),
            vm_sums: state.vm_sums,
            prev_low_high: state.prev_low_high,
            tr_state: state.tr_state,
            tr_sum: state.tr_sum,
        }
    }
    #[inline(always)]
    fn init_calc(&mut self, (high, low, close):  (f64, f64, f64)) -> f64 {
        let tr = self.tr_state.calc((high, low, close));
        let high_low_simd = Simd::from_array([high, low]);
        /*let vm_up = (high - self.prev_low).abs();
        let vm_down = (low - self.prev_high).abs();*/
        let vm_simd = (high_low_simd - self.prev_low_high).abs();

        self.prev_low_high = high_low_simd.reverse();

        self.buffer.push((tr, vm_simd));
        
        self.tr_sum += tr;
        self.vm_sums += vm_simd;

        tr
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, close): Self::Inputs<'a>) -> Self::Outputs {
        let tr = self.tr_state.calc((high, low, close));
        let high_low_simd = Simd::from_array([high, low]);
        let vm_simd = (high_low_simd - self.prev_low_high).abs();
        
        self.prev_low_high = high_low_simd.reverse();

        let (old_tr, old_vm) = self.buffer.push_with_info((tr, vm_simd));
        self.tr_sum += tr - old_tr;
        let tr_sum_simd = Simd::splat(self.tr_sum);

        self.vm_sums += vm_simd - old_vm;
        let vi_simd = self.vm_sums / tr_sum_simd;

        (vi_simd[0], vi_simd[1], tr)
    }
}


/// Performs the main calculation loop for the Vortex indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of `(high, low, close)` price slices.
/// * `state` - A mutable reference to the current indicator state.
/// * `outputs` - A tuple of `(vi_up_line, vi_down_line)` output slices.
/// * `tr_line` - Output slice for the optional TR values (empty if not requested).
fn cycle(
    inputs: (&[f64], &[f64], &[f64]),
    state: &mut State<Warm>,
    outputs: (&mut [f64], &mut [f64]),
    tr_line: &mut [f64],
) {
    let (high, low, close) = inputs;
    let (vi_up_line, vi_down_line) = outputs;
    let (_, want_tr) = crate::calc_want_flags!(tr_line);

    for i in 0..high.len() {
        let tr;
        unsafe {
            let (vi_up, vi_down);
            (vi_up, vi_down, tr) = state.calc((
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            ));
            *vi_up_line.get_unchecked_mut(i) = vi_up;
            *vi_down_line.get_unchecked_mut(i) = vi_down;
        }

        crate::store_optional_outputs!(i,
            want_tr, tr_line => tr
        );
    }
}

pub struct Vortex;

impl Indicator<INPUTS, OPTIONS> for Vortex {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "vortex",
        full_name: "Vortex",
        indicator_type: IndicatorType::Trend,
        // Inputs are expected to be: high, low, close
        inputs: &["high", "low", "close"],
        // Options: period
        options: &["period"],
        outputs: &["vi_up", "vi_down"],
        optional_outputs: &["tr"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "vortex",
                label: "Vortex",
                display_type: DisplayType::Indicator,
                outputs: &["vi_up", "vi_down"],
            },
            DisplayGroup {
                offset: None,
                id: "tr",
                label: "True Range",
                display_type: DisplayType::Indicator,
                outputs: &["tr"],
            },
        ],
    };

    fn output_length(data_len: usize, options: &[f64; OPTIONS]) -> usize {
        data_len - Self::min_data(options) // + 1
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
    
        validate_inputs(inputs, Self::min_data(options))?;
        let [high, low, close] = inputs;
    
        let (mut vi_up_line, mut vi_down_line, mut tr_line) = {
            let len = high.len();
            let capacity = Self::output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    tr_line_line: Tr::output_length(len, &[])
                ),
            )
        };
    
        let mut state = State::init_state(high, low, close, period, &mut tr_line);
        let inputs = (
            &high[period + 1..],
            &low[period + 1..],
            &close[period + 1..],
        );
        let tr = {
            let offset = crate::slice_outputs_start!(vi_up_line.len(), tr_line);
            &mut tr_line[offset..]
        };
        // Single-pass calculation loop.
        cycle(inputs, &mut state, (&mut vi_up_line, &mut vi_down_line), tr);
    
        Ok((vec![vi_up_line, vi_down_line, tr_line], state))
    }
}