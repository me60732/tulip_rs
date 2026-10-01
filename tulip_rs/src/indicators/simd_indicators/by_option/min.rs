//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::min::{Indicator, IndicatorState, Min, State, INPUTS, OPTIONS};
use crate::indicators::simd_indicators::min_simd::{options::SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::ring_buffer::unsync_multi_buffer::multi_buffer::UsizeConstants;
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver for the Minimum Value (MIN) indicator, processing `N` option-set lanes per scheduling epoch.
struct MinDriver {}

impl Driver<State<Warm>, usize> for MinDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();

        let (look_back, mut i_simd) = {
            let mut look_backs = [0; N];
            let mut i_array = [0; N];
            for (i, option) in options.iter().enumerate() {
                if let Some(&look_back) = option {
                    look_backs[i] = look_back;
                    i_array[i] = look_back;
                }
            }
            (Simd::from_array(look_backs), Simd::from_array(i_array))
        };

        //collect outputs
        let min_line_ptr = crate::extract_output_ptrs!(outputs, N, min_line_ptr);
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let mut state = SimdState::<N>::from_states(&mut states);

        //println!("start: {:?}, N: {:?}, LEN: {:?}", start, N, real.len());
        for j in 0..len {
            let (min, _) = state.calc((real_ptrs, i_simd, look_back));

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                min_line_ptr => min
            );
            i_simd += UsizeConstants::ONE;
        }
        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Minimum Value (MIN) indicator for one asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// Applies each of the `N` period configurations to the same shared input series, computing
/// rolling minimum values for all option sets in a single SIMD-accelerated pass via
/// [`PrimeMover`].
///
/// # Arguments
/// * `inputs` - Shared input: `inputs[0]` is the `real` price series.
/// * `options` - An array of `N` option sets; `options[i][0]` is the `period` for lane `i`.
/// * `_optional_outputs` - Unused; MIN has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the `min` series for option set `i`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Min::min_data)?;
    validate_options(options, None)?;

    // Build periods array and create sorting permutation
    let periods_unsorted: [(usize, usize); N] =
        std::array::from_fn(|i| (options[i][0] as usize, options[i][0] as usize - 1));

    // Compute a permutation that sorts lanes by period ascending
    // Store original indices sorted by period value
    let mut sorted_indices: [usize; N] = std::array::from_fn(|i| i);
    sorted_indices.sort_by(|&a, &b| periods_unsorted[a].0.cmp(&periods_unsorted[b].0));

    // Build the sorted periods array
    let mut periods: [(usize, usize); N] = std::array::from_fn(|_| (0, 0));
    for (sorted_pos, &orig_idx) in sorted_indices.iter().enumerate() {
        periods[sorted_pos] = periods_unsorted[orig_idx];
    }

    let mut road_train = PrimeMover::<N, State<Warm>, usize>::new();
    // Track output buffers in sorted order
    let mut output_buffers_sorted: Vec<Vec<Vec<f64>>> = Vec::with_capacity(N);

    for (sorted_pos, &(_period, look_back)) in periods.iter().enumerate() {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let min_line = {
            let len = inputs[0].len();
            let capacity = Min::output_length(len, options[sorted_indices[sorted_pos]]);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::init_state(inputs[0], look_back);

        let mut output_buffer = vec![min_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr(), //slice from
                    output_buffer.len(),        // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            sorted_pos, // Use sorted position for lane index in SIMD state
            look_back,
            look_back,
            state,
            Some(&periods[sorted_pos].1),
        ));
        output_buffers_sorted.push(output_buffer);
    }

    let mut driver = MinDriver {};
    let states_vec = road_train.drive(&mut driver);

    // Permute results back to original caller-supplied order
    let mut output_buffers: Vec<Vec<Vec<f64>>> = Vec::with_capacity(N);
    output_buffers.resize(N, vec![vec![0.0; 0]; 1]);

    for (sorted_pos, &orig_idx) in sorted_indices.iter().enumerate() {
        output_buffers[orig_idx] = std::mem::take(&mut output_buffers_sorted[sorted_pos]);
    }

    let mut states = Vec::with_capacity(N);
    for (sorted_pos, state) in states_vec.into_iter().enumerate() {
        let orig_idx = sorted_indices[sorted_pos];
        states.push(IndicatorState::new(
            inputs[0],
            state,
            periods_unsorted[orig_idx],
        ));
    }
    Ok((output_buffers, states))
}
