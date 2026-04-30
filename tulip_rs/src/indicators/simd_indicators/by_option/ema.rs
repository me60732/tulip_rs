//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::ema::{
    init_state, min_data, multiplier, output_length, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::ema_simd::calc_simd;

struct EmaDriver {}

impl Driver<f64, (f64, f64)> for EmaDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut f64>,
        options: Vec<Option<&(f64, f64)>>,
    ) {
        let len = outputs[0][0].len();
        //println!("N: {:?}", N);
        //let mut period_arr = [0usize; N];
        let multipliers_simd = {
            let mut multipliers = ([0.0; N], [0.0; N]);
            for (lane, option) in options.iter().enumerate() {
                if let Some(&multiplier) = option {
                    //println!("{:?}", outputs[lane][0].len());
                    multipliers.0[lane] = multiplier.0;
                    multipliers.1[lane] = multiplier.1;
                }
            }
            (
                Simd::from_array(multipliers.0),
                Simd::from_array(multipliers.1),
            )
        };

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut ema = Simd::<f64, N>::from_array(std::array::from_fn(|i| unsafe {
            **states.get_unchecked(i)
        }));

        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let ema_line_ptr = crate::extract_output_ptrs!(outputs, N, ema_line_ptr);
        //let mut j = 0;
        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            let new_vals = crate::extract_simd_inputs_at_index_splat!(i, N,
                new @ real_ptrs
            );

            ema = calc_simd(new_vals, ema, multipliers_simd);

            crate::write_simd_at_indices!(N, i,
                ema_line_ptr => ema
            );
        }

        // Update states efficiently
        let final_ema = ema.to_array();
        for (i, state) in states.iter_mut().enumerate().take(N) {
            **state = final_ema[i];
        }
    }
}

pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let periods: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);

    let multipliers: [(f64, f64); N] = std::array::from_fn(|i| multiplier(options[i][0] as usize));

    let mut road_train = PrimeMover::<N, f64, (f64, f64)>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for (i, (&period, multipliers)) in periods.iter().zip(multipliers.iter()).enumerate() {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let ema_line = {
            let len = inputs[0].len();
            let capacity = output_length(len, options[i]);
            crate::uninit_vec!(f64, capacity)
        };

        let state = init_state(inputs[0], period, *multipliers);

        let mut output_buffer = vec![ema_line];

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
            0,
            state,
            Some(multipliers),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = EmaDriver {};
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(state, multipliers[i]));
    }
    Ok((output_buffers, states))
}
