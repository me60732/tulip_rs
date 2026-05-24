//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::dm::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::dm_simd::{calc_simd, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver for the Directional Movement (DM) indicator, processing `N` option-set lanes per scheduling epoch.
struct DmDriver {}

impl Driver<State, f64> for DmDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&f64>>,
    ) {
        let mut state = SimdState::<N>::new(&states);
        let len = outputs[0][0].len();
        let multiplier_simd = {
            let mut multipliers = [0.0; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&multiplier) = option {
                    multipliers[lane] = multiplier
                }
            }
            Simd::from_array(multipliers)
        };

        //collect outputs
        let (plus_dm_line_ptr, minus_dm_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, plus_dm_line_ptr, minus_dm_line_ptr);

        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs) = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let (high, low) = crate::extract_simd_inputs_at_index_splat!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs
            );

            let (plus_dm, minus_dm) = calc_simd(&mut state, high, low, multiplier_simd);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                plus_dm_line_ptr => plus_dm,
                minus_dm_line_ptr => minus_dm
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Directional Movement (DM) on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[high, low]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Unused; DM has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[plus_dm, minus_dm]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let multipliers: [f64; N] = std::array::from_fn(|i| multiplier(options[i][0] as usize));

    let mut road_train = PrimeMover::<N, State, f64>::new();

    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
        ];

        let (plus_dm_line, minus_dm_line) = {
            let capacity: usize = output_length(inputs[0].len(), options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };
        let period = options[i][0] as usize;

        let state = State::init_state(inputs[0], inputs[1], period);

        let mut output_buffer = vec![plus_dm_line, minus_dm_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(0), //slice from
                    output_buffer.len(),               // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period,
            0,
            state,
            Some(&multipliers[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = DmDriver {};
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state));
    }
    Ok((output_buffers, states))
}
