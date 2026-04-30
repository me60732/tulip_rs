//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::stoch_simd::options::SimdState;
use crate::indicators::stoch::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;
struct StochDriver {}

impl Driver<State, (usize, (f64, f64))> for StochDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(usize, (f64, f64))>>,
    ) {
        let len = outputs[0][0].len();
        let (look_back, multipliers, mut i_simd) = {
            let mut look_back = [0usize; N];
            let mut i = [0usize; N];
            let mut multipliers = ([0.0; N], [0.0; N]);
            for (lane, option) in options.iter().enumerate() {
                if let Some(&(k_period, multi)) = option {
                    i[lane] = k_period;
                    look_back[lane] = k_period - 1;
                    multipliers.0[lane] = multi.0;
                    multipliers.1[lane] = multi.1;
                }
            }
            (
                Simd::from_array(look_back),
                (
                    Simd::from_array(multipliers.0),
                    Simd::from_array(multipliers.1),
                ),
                Simd::from_array(i),
            )
        };

        //collect outputs
        let (k_line_ptr, d_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, k_line_ptr, d_line_ptr);
        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);
        let mut state = SimdState::new(&mut states);
        //let look_back = self.period - 1;

        let one_splat = Simd::splat(1);
        //println!("start: {:?}, N: {:?}, LEN: {:?}", start, N, real.len());
        for j in 0..len {
            let close = crate::extract_simd_inputs_at_index_splat!(i_simd[0], N,
                close @ close_ptrs
            );
            let (k, d) = unsafe {
                state.calc_unchecked_simd(
                    high_ptrs,
                    low_ptrs,
                    close,
                    i_simd,
                    look_back,
                    multipliers,
                )
            };

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                k_line_ptr => k,
                d_line_ptr => d
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
    let mut road_train = PrimeMover::<N, State, (usize, (f64, f64))>::new();
    let mut output_buffers = Vec::with_capacity(N);

    let params: [(usize, (f64, f64)); N] = std::array::from_fn(|i| {
        (
            options[i][0] as usize,
            multiplier(options[i][1] as usize, options[i][2] as usize),
        )
    });

    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
            inputs[2], // close
        ];
        let mut starts = [0; 2];
        let (mut k_line, d_line, state, start);
        {
            let (k_capacity, d_capacity) = output_length(inputs[0].len(), options[i]);
            k_line = crate::uninit_vec!(f64, k_capacity);
            d_line = crate::uninit_vec!(f64, d_capacity);

            let k_slow = options[i][1] as usize;
            let d_period = options[i][2] as usize;
            (state, starts[0], start) = State::init_state(
                (inputs[0], inputs[1], inputs[2]),
                params[i].0,
                k_slow,
                d_period,
                &mut k_line,
            );
        }

        let mut output_buffer = vec![k_line, d_line];

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
            start,
            params[i].0,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = StochDriver {};
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            state,
            inputs[0],
            inputs[1],
            params[i].1, //multipliers
            params[i].0, //k_period
        ));
    }
    Ok((output_buffers, states))
}
