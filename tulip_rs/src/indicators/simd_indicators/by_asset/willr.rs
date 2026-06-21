//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::willr_simd::{assets::Calc, SimdState, CHUNK_1};
use crate::indicators::{
    max::output_length as max_output_length,
    willr::{min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH},
};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;
/// SIMD driver that advances Williams %R (WILLR) across `N` asset lanes per scheduling epoch.
struct WillrDriver {
    period: usize,
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State> for WillrDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        //collect outputs
        let output_ptrs = crate::extract_output_ptrs!(outputs, N, willr, min, max);
        let inputs = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);
        let mut state = SimdState::new(&mut states);
        //let look_back = self.period - 1;

        if CHUNK_1.contains(&self.period) {
            self.cycle::<N, 1>(inputs, &mut state, output_ptrs, len);
        } else {
            self.cycle::<N, 4>(inputs, &mut state, output_ptrs, len);
        }
        // Update states efficiently
        state.write_states(&mut states);
    }
}
impl WillrDriver {
    fn cycle<const N: usize, const CHUNK_SIZE: usize>(
        &self,
        inputs: ([*const f64; N], [*const f64; N], [*const f64; N]),
        state: &mut SimdState<N>,
        output_ptrs: ([*mut f64; N], [*mut f64; N], [*mut f64; N]),
        len: usize,
    ) {
        let (willr_line_ptr, min_line_ptr, max_line_ptr) = output_ptrs;
        let look_back = self.period - 1;
        let (high_ptrs, low_ptrs, close_ptrs) = inputs;
        let (has_optional, want_min, want_max) = self.want_optional_outputs;
        for (j, i) in (self.period..len).enumerate() {
            let close = crate::extract_simd_inputs_at_index!(i, N, close @ close_ptrs);

            let (willr, min, max) = unsafe {
                state.calc_unchecked_simd::<CHUNK_SIZE>(high_ptrs, low_ptrs, close, i, look_back)
            };

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                willr_line_ptr => willr
            );

            if has_optional {
                crate::store_simd_optional_outputs!(j, N,
                    want_min, min_line_ptr => min,
                    want_max, max_line_ptr => max
                );
            }
        }
    }
}
/// Calculates Williams %R (WILLR) for `N` assets simultaneously using SIMD parallelism.
///
/// WILLR produces no optional outputs. Uses the [`PrimeMover`] scheduler to batch assets into
/// SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close]` for asset `i`.
/// * `options` - `options[0]` is the `period`.
/// * `_optional_outputs` - Unused; WILLR has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the Williams %R line for asset `i` and
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = (false, false, false);
    for i in 0..N {
        let [high, low, close] = *inputs[i];
        let asset_inputs = vec![high, low, close];

        let (willr_line, (mut min_line, mut max_line)) = {
            let len = high.len();
            let capacity = output_length(len, options);
            let min_max_capacity = max_output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    min_line: min_max_capacity,
                    max_line: min_max_capacity
                ),
            )
        };
        let state = State::init_state(high, low, period, (&mut min_line, &mut max_line));
        let mut starts = [0; 3];
        (starts[1], starts[2]) = crate::slice_outputs_start!(willr_line.len(), min_line, max_line);

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(min_line, max_line);
        }
        let mut output_buffer = vec![willr_line, min_line, max_line];

        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(starts[j]), //slice from
                    output_buffer.len() - starts[j],           // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period,
            period,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = WillrDriver {
        period,
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            state,
            inputs[i][0],
            inputs[i][1],
            period,
        ));
    }
    Ok((output_buffers, states))
}
