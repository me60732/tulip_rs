//use crate::common::validate_inputs;
use crate::indicators::cmo::{output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH, min_data};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::cmo_simd::{calc_simd, SimdState};
use crate::types::IndicatorError;
use crate::common_simd::options::{validate_inputs, validate_options};
use std::simd::{Simd, };

struct CmoDriver;

impl Driver<State, usize> for CmoDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();

        let mut i = [0usize; N];
        let mut prev_i = [0usize; N];
        for (lane, option) in options.iter().enumerate() {
            if let Some(&period) = option {
                i[lane] = period + 1;
                prev_i[lane] = period;
            }
        }

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::new(&states);

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let cmo_line_ptr = crate::extract_output_ptrs!(outputs, N, cmo_line_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for j in 0..len {
            // Get new and old values using pre-computed pointers
            let (current, prev) = crate::extract_simd_at_indices_array!(N, input_ptrs,
                current @ i,
                prev @ prev_i
            );
            let (prev_before, prev_period) = crate::extract_simd_at_indices!(N, input_ptrs,
                prev_before @ j,
                prev_period @ j + 1
            );
            let cmo = calc_simd(&mut state, prev_before, prev_period, current, prev);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                cmo_line_ptr => cmo
            );
            
            prev_i = i;
            
            for i in i.iter_mut() {
                *i += 1;
            }
        }

        state.write_states(&mut states);
    }
}

pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError>
{
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let mut road_train = PrimeMover::<N, State, usize>::new();

    let mut params = [0usize; N];
    for i in 0..N {
        params[i] = options[i][0] as usize;
    }
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let period = options[i][0] as usize;
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let cmo_line = {
            let capacity = output_length(inputs[0].len(), options[i]);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::init_state(inputs[0], period);

        let mut output_buffer = vec![cmo_line];

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
            period + 1,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = CmoDriver {};
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(inputs[0], state, params[i]));
    }
    Ok((output_buffers, states))
}
