//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
//use std::simd::cmp::SimdPartialOrd;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::dema::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::ema::output_length as ema_output_length;
use crate::indicators::simd_indicators::dema_simd::{calc_simd, SimdState};
use std::simd::Simd;

struct DemaDriver {
    want_optional_outputs: bool,
}

impl Driver<State, (f64, f64)> for DemaDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(f64, f64)>>,
    ) {
        let len = outputs[0][0].len();

        let multipliers_simd = {
            let mut multipliers = ([0.0; N], [0.0; N]);
            for (lane, option) in options.iter().enumerate() {
                if let Some(&multiplier) = option {
                    multipliers.0[lane] = multiplier.0;
                    multipliers.1[lane] = multiplier.1;
                }
            }
            (
                Simd::from_array(multipliers.0),
                Simd::from_array(multipliers.1),
            )
        };

        let mut state = SimdState::new_mut_ref(&states);

        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (dema_line_ptr, ema_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, dema_line_ptr, ema_line_ptr);

        // Optimized main loop with minimal overhead
        for j in 0..len {
            let values = crate::extract_simd_inputs_at_index_splat!(j, N, values @ input_ptrs);

            let (dema, ema) = calc_simd(&mut state, values, multipliers_simd);

            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, j,
                dema_line_ptr => dema
            );
            crate::store_simd_optional_outputs!(j, N,
                self.want_optional_outputs, ema_line_ptr => ema
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let mut output_buffers = Vec::with_capacity(N);
    let multipliers: [(f64, f64); N] = std::array::from_fn(|i| multiplier(options[i][0] as usize));

    let mut road_train = PrimeMover::<N, State, (f64, f64)>::new();
    let mut want_optional_outputs = false;
    for i in 0..N {
        let period = options[i][0] as usize;
        let len = inputs[0].len();
        let dema_capacity = output_length(len, options[i]);
        let dema_line = crate::uninit_vec!(f64, dema_capacity);
        let ema_capacity = ema_output_length(len, options[i]);
        let mut ema_line = crate::init_optional_outputs_eff!(
            optional_outputs, &[false],
            ema_line: ema_capacity
        );

        let state = State::init_state(inputs[0], dema_capacity, period, &mut ema_line);
        let asset_inputs = vec![inputs[0]];
        let mut starts = [0; 2];
        starts[1] = crate::slice_outputs_start!(dema_capacity, ema_line);

        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(ema_line);
        }
        let mut output_buffer = vec![dema_line, ema_line];
        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(starts[j]), //slice from
                    output_buffer.len(),                       // slice to
                ));
            }
        }
        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period * 2 - 2,
            0,
            state,
            Some(&multipliers[i]),
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = DemaDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            state,
            (multipliers[i].0, multipliers[i].1),
        ));
    }
    Ok((output_buffers, states))
}
