//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::stddev_simd::SimdState;
use crate::indicators::stddev::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;

struct StddevDriver {
    want_optional_outputs: bool,
}

impl Driver<State, (usize, f64)> for StddevDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(usize, f64)>>,
    ) {
        let len = outputs[0][0].len();

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::new(&states);

        let (mut i, multiplier_simd) = {
            let mut multipliers = [0.0; N];
            let mut i = [0usize; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&(period, multiplier)) = option {
                    i[lane] = period;
                    multipliers[lane] = multiplier;
                }
            }
            (i, Simd::from_array(multipliers))
        };

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);

        let (stddev_line_ptr, sma_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, stddev_line_ptr, sma_line_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for j in 0..len {
            // Get new and old values using pre-computed pointers
            let old_vals = crate::extract_simd_inputs_at_index!(j, N,
                old @ input_ptrs
            );
            let new_vals = crate::extract_simd_inputs_at_index_array!(i, N,
                new @ input_ptrs
            );

            let (stddev, sma) = state.calc_simd(new_vals, old_vals, multiplier_simd);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                stddev_line_ptr => stddev
            );
            crate::store_simd_optional_outputs!(j, N,
                self.want_optional_outputs, sma_line_ptr => sma
            );

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
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [(usize, f64); N] =
        std::array::from_fn(|i| (options[i][0] as usize, multiplier(options[i][0] as usize)));
    let mut road_train = PrimeMover::<N, State, (usize, f64)>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = false;

    for (i, &(period, _)) in params.iter().enumerate() {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let (stddev_line, sma_line) = {
            let capacity = output_length(inputs[0].len(), options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };

        let state = State::init_state(inputs[0], period);

        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(sma_line);
        }

        let mut output_buffer = vec![stddev_line, sma_line];
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
            period,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = StddevDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            inputs[0],
            state,
            params[i].1,
            params[i].0,
        ));
    }
    Ok((output_buffers, states))
}
