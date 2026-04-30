//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::vwma_simd::SimdState;
use crate::indicators::vwma::{
    min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;
struct VwmaDriver {}

impl Driver<State, usize> for VwmaDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();
        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::new(&states);
        let mut i = [0usize; N];
        for (lane, option) in options.iter().enumerate() {
            if let Some(&period) = option {
                i[lane] = period;
            }
        }

        // Optimization 2: Pre-compute all input and output pointers
        let (close_ptrs, volume_ptrs) =
            crate::extract_input_ptrs!(inputs, N, close_ptrs, volume_ptrs);

        let output_ptrs = crate::extract_output_ptrs!(outputs, N, output_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for j in 0..len {
            let (prev_close, prev_volume) = crate::extract_simd_inputs_at_index!(j, N,
                pc @ close_ptrs,
                pv @ volume_ptrs
            );
            let (close, volume) = crate::extract_simd_inputs_at_index_array!(i, N,
                pc @ close_ptrs,
                pv @ volume_ptrs
            );

            let vwma = state.calc_simd(close, volume, prev_close, prev_volume);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                output_ptrs => vwma
            );

            for i in i.iter_mut() {
                *i += 1;
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);

    let mut road_train = PrimeMover::<N, State, usize>::new();
    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = output_length(inputs[0].len(), options[i]);
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    for (i, period) in params.iter().enumerate() {
        let state = State::init_state(*period, inputs[0], inputs[1]);
        let asset_inputs = vec![inputs[0], inputs[1]];
        unsafe {
            // Get a mutable reference to the output buffer for this asset
            let output_buffer = &mut output_buffers[i][0];
            let asset_outputs = vec![std::slice::from_raw_parts_mut(
                output_buffer.as_mut_ptr(),
                output_buffer.len(),
            )];

            road_train.add_asset(Asset::new(
                asset_inputs,
                asset_outputs,
                i,
                *period,
                *period,
                state,
                Some(period),
            ));
        }
    }
    let mut driver = VwmaDriver {};
    let states = road_train.drive(&mut driver);

    let mut indicator_states = Vec::with_capacity(N);

    for (state, period) in states.into_iter().zip(params.into_iter()) {
        indicator_states.push(IndicatorState::new(inputs[0], inputs[1], state, period));
    }
    Ok((output_buffers, indicator_states))
}
