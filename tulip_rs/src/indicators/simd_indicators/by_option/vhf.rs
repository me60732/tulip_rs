//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::vhf_simd::{options::Calc, SimdState};
use crate::indicators::vhf::{
    init_state, min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;
/// SIMD driver for the Vertical Horizontal Filter (VHF) indicator, processing `N` option-set lanes per scheduling epoch.
struct VhfDriver {}

impl Driver<State, usize> for VhfDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();

        let (mut i_simd, mut p_simd, look_back) = {
            let mut period = [0; N];
            let mut i_array = [0; N];
            let mut look_back = [0; N];
            for (i, option) in options.iter().enumerate() {
                if let Some(&p) = option {
                    period[i] = p;
                    i_array[i] = p + 1;
                    look_back[i] = p - 1;
                }
            }
            (
                Simd::from_array(i_array),
                Simd::from_array(period),
                Simd::from_array(look_back),
            )
        };

        //collect outputs
        let vhf_ptr = crate::extract_output_ptrs!(outputs, N, vhf_ptr);

        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        let mut state = SimdState::new(&mut states);

        for j in 0..len {
            let (value, prev) = crate::extract_simd_at_indices_array!(N, real_ptrs,
                current @ i_simd,
                prev @ p_simd
            );
            let (old, drop) = crate::extract_simd_at_indices!(N, real_ptrs,
                old @ j,
                drop @ j + 1
            );
            let vhf = unsafe {
                state.calc_unchecked_simd((value, prev, old, drop), real_ptrs, look_back, i_simd)
            };

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                vhf_ptr => vhf
            );
            p_simd = i_simd;

            //i_simd += one_splat;
            for i in i_simd.as_mut_array().iter_mut() {
                *i += 1;
            }
        }
        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Vertical Horizontal Filter (VHF) for one shared asset across `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch option sets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - Shared input data: `inputs[0]` is `&[f64]` containing `real` (price series).
/// * `options` - An array of `N` option sets; `options[i]` is `&[f64; OPTIONS_WIDTH]` containing
///   `[period]` for option set `i`.
/// * `optional_outputs` - Unused; VHF has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `vhf` for option set `i`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or any option set is invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);
    let mut road_train = PrimeMover::<N, State, usize>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let mut vhf_line = {
            let len = inputs[0].len();
            let capacity = output_length(len, options[i]);
            crate::uninit_vec!(f64, capacity)
        };

        let state = init_state(inputs[0], params[i], &mut vhf_line);

        let mut output_buffer = vec![vhf_line];

        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(1), //slice from
                    output_buffer.len(),               // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            params[i] + 1,
            params[i] + 1,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = VhfDriver {};
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(state, inputs[0], params[i]));
    }
    Ok((output_buffers, states))
}
