//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::zlema_simd::SimdState;
use crate::indicators::zlema::{
    min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;

struct ZlemaDriver {}

impl Driver<State, usize> for ZlemaDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();
        let mut state = SimdState::new(&states);
        let mut i = [0usize; N];
        for (lane, option) in options.iter().enumerate() {
            if let Some(&lag) = option {
                i[lane] = lag;
            }
        }
        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let zlema_line_ptr = crate::extract_output_ptrs!(outputs, N, zlema_line_ptr);

        // Optimized main loop with minimal overhead
        for j in 0..len {
            let lagged = crate::extract_simd_inputs_at_index!(j, N, old @ input_ptrs);
            let current = crate::extract_simd_inputs_at_index_array!(i, N, current @ input_ptrs);
            let zlema = state.calc_simd(current, lagged);

            crate::write_simd_at_indices!(N, j,
                zlema_line_ptr => zlema
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
    let params: [usize; N] =
        std::array::from_fn(|i| (((options[i][0] as usize).saturating_sub(1)) / 2).max(1));

    let mut road_train = PrimeMover::<N, State, usize>::new();
    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = output_length(inputs[0].len(), options[i]);
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    for i in 0..N {
        let period = options[i][0] as usize;
        let lag = ((period.saturating_sub(1)) / 2).max(1);
        let state = State::new(inputs[0], lag, period);

        let asset_inputs = vec![inputs[0]];
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
                lag,
                lag,
                state,
                Some(&params[i]),
            ));
        }
    }

    let mut driver = ZlemaDriver {};
    let states = road_train.drive(&mut driver);

    let mut indicator_states = Vec::with_capacity(N);
    for (state, &lag) in states.into_iter().zip(params.iter()) {
        indicator_states.push(IndicatorState::new(inputs[0], state, lag));
    }
    Ok((output_buffers, indicator_states))
}
