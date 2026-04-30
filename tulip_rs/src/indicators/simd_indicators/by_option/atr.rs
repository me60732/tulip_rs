//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::atr::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::atr_simd::SimdState;
use crate::indicators::tr::output_length as tr_output_length;
use crate::types::IndicatorError;
use std::simd::Simd;

struct AtrDriver {
    want_optional_outputs: bool,
}

impl Driver<State, f64> for AtrDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&f64>>,
    ) {
        let mut state = SimdState::<N>::new(&states);
        let len = outputs[0][0].len();
        let multipliers_simd = {
            let mut multipliers = [0.0; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&multiplier) = option {
                    //println!("{:?}", outputs[lane][0].len());
                    multipliers[lane] = multiplier;
                }
            }
            Simd::from_array(multipliers)
        };

        //collect outputs
        let (atr_line_ptr, tr_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, atr_line_ptr, tr_line_ptr);

        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let (high, low, close) = crate::extract_simd_inputs_at_index_splat!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs
            );

            let (atr, tr) = state.calc_simd(high, low, close, multipliers_simd);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                atr_line_ptr => atr
            );

            crate::store_simd_optional_outputs!(i, N,
                self.want_optional_outputs, tr_line_ptr => tr
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
    let multipliers: [f64; N] = std::array::from_fn(|i| multiplier(options[i][0] as usize).0);

    let mut road_train = PrimeMover::<N, State, f64>::new();
    let mut want_optional_outputs = false;
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
            inputs[2], // close
        ];
        let len = inputs[0].len();
        let atr_capacity = output_length(len, options[i]);
        let atr_line = crate::uninit_vec!(f64, atr_capacity);

        let mut tr_line = crate::init_optional_outputs_eff!(
            optional_outputs, &[false],
            tr_line: tr_output_length(len, options[i])
        );
        let period = options[i][0] as usize;
        let state = State::init_state(inputs[0], inputs[1], inputs[2], period, &mut tr_line, false);

        let mut starts = [0; 2];
        starts[1] = crate::slice_outputs_start!(atr_capacity, tr_line);
        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(tr_line);
        }

        let mut output_buffer = vec![atr_line, tr_line];

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
            period,
            0,
            state,
            Some(&multipliers[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = AtrDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state));
    }
    Ok((output_buffers, states))
}
