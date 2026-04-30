//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::aroonosc::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::aroonosc_simd::{options::Calc, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;
struct AroonoscDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State, (usize, f64)> for AroonoscDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(usize, f64)>>,
    ) {
        let len = outputs[0][0].len();

        let (period, multiplier, mut i_simd) = {
            let mut period = [0; N];
            let mut i_array = [0; N];
            let mut multiplier = [0.0; N];
            for (i, option) in options.iter().enumerate() {
                if let Some(&(p, m)) = option {
                    period[i] = p;
                    i_array[i] = p;
                    multiplier[i] = m;
                }
            }
            (
                Simd::from_array(period),
                Simd::from_array(multiplier),
                Simd::from_array(i_array),
            )
        };
        let (want_optional, want_aroon_down, want_aroon_up) = self.want_optional_outputs;
        //collect outputs
        let (aroonosc_ptr, aroon_down_ptr, aroon_up_ptr) =
            crate::extract_output_ptrs!(outputs, N, aroonosc_ptr, aroon_down_ptr, aroon_up_ptr);

        let (high_ptrs, low_ptrs) = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs);

        let mut state = SimdState::new(&mut states);
        let one_splat = Simd::splat(1);

        for j in 0..len {
            let (aroonosc, aroon_down, aroon_up) = unsafe {
                state.calc_unchecked_simd(high_ptrs, low_ptrs, i_simd, period, multiplier)
            };
            crate::write_simd_at_indices!(N, j,
                aroonosc_ptr => aroonosc
            );
            if want_optional {
                crate::store_simd_optional_outputs!(j, N,
                    want_aroon_down, aroon_down_ptr => aroon_down,
                    want_aroon_up, aroon_up_ptr => aroon_up
                );
            }
            i_simd += one_splat;
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
    let params: [(usize, f64); N] = std::array::from_fn(|i| {
        let period = options[i][0] as usize;
        (period, multiplier(period))
    });
    let mut road_train = PrimeMover::<N, State, (usize, f64)>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = (false, false, false);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
        ];

        let (aroonosc_line, (aroon_down_line, aroon_up_line)) = {
            let capacity = output_length(inputs[0].len(), options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    aroon_up_line: capacity,
                    aroon_down_line: capacity
                ),
            )
        };
        let state = State::init_state(inputs[0], inputs[1], params[i].0);

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(aroon_down_line, aroon_up_line);
        }
        let mut output_buffer = vec![aroonosc_line, aroon_down_line, aroon_up_line];

        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
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
            params[i].0,
            params[i].0,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = AroonoscDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (state, &(period, multiplier)) in states_vec.into_iter().zip(params.iter()) {
        states.push(IndicatorState::new(
            inputs[0], inputs[1], state, period, multiplier,
        ));
    }
    Ok((output_buffers, states))
}
