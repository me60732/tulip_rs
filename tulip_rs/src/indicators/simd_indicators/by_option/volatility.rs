//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::volatility_simd::options::SimdState;
use crate::indicators::volatility::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;

struct VolatilityDriver {}

impl Driver<State, (usize, f64)> for VolatilityDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(usize, f64)>>,
    ) {
        let mut periods = [0usize; N];
        let multiplier_simd = {
            let mut multipliers = [0.0; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&(period, multiplier)) = option {
                    periods[lane] = period;
                    multipliers[lane] = multiplier;
                }
            }
            Simd::from_array(multipliers)
        };

        let mut state = SimdState::<N>::new(&mut states, periods);
        let len = inputs[0][0].len();

        //collect outputs
        let volatility_line_ptr = crate::extract_output_ptrs!(outputs, N, volatility_line_ptr);

        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let real = unsafe { *real_ptrs[0].add(i) };

            let volatility = unsafe { state.calc_unchecked_simd(real, multiplier_simd) };

            crate::write_simd_at_indices!(N, i,
                volatility_line_ptr => volatility
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [(usize, f64); N] = std::array::from_fn(|i| {
        let period = options[i][0] as usize;
        (period, multiplier(period))
    });
    let mut road_train = PrimeMover::<N, State, (usize, f64)>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let volatility_line = {
            let capacity = output_length(inputs[0].len(), options[i]);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::init_state(inputs[0], params[i].0);

        let mut output_buffer = vec![volatility_line];

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
            params[i].0 + 1,
            0,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = VolatilityDriver {};
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, param) in states_vec.into_iter().zip(params.into_iter()) {
        states.push(IndicatorState::new(state, param.1));
    }
    Ok((output_buffers, states))
}
