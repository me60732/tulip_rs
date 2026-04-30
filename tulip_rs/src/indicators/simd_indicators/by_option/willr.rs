//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::willr_simd::{options::Calc, SimdState};
use crate::indicators::willr::{
    min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;
struct WillrDriver {}

impl Driver<State, usize> for WillrDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();

        let (look_back, mut i_simd) = {
            let mut look_back = [0; N];
            let mut i_array = [0; N];
            for (i, option) in options.iter().enumerate() {
                if let Some(&p) = option {
                    look_back[i] = p - 1;
                    i_array[i] = p;
                }
            }
            (Simd::from_array(look_back), Simd::from_array(i_array))
        };

        //collect outputs
        let willr_ptr = crate::extract_output_ptrs!(outputs, N, willr_ptr);

        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);

        let mut state = SimdState::new(&mut states);
        let one_splat = Simd::splat(1);
        //println!("start: {:?}, N: {:?}, LEN: {:?}", start, N, real.len());
        for j in 0..len {
            let close = crate::extract_simd_inputs_at_index_splat!(i_simd[0], N,
                close @ close_ptrs
            );
            let willr =
                unsafe { state.calc_unchecked_simd(high_ptrs, low_ptrs, close, i_simd, look_back) };

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                willr_ptr => willr
            );
            i_simd += one_splat;
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
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
            inputs[2], // close
        ];

        let willr_line = {
            let len = inputs[0].len();
            let capacity = output_length(len, options[i]);
            crate::uninit_vec!(f64, capacity)
        };

        let mut output_buffer = vec![willr_line];

        let state = State::init_state(inputs[0], inputs[1], params[i]);

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
            params[i],
            params[i],
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = WillrDriver {};
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(state, inputs[0], inputs[1], params[i]));
    }
    Ok((output_buffers, states))
}
