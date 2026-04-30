//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::wilders_simd::calc_simd;
use crate::indicators::wilders::{
    init_state, min_data, multiplier, output_length, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;

struct WildersDriver {}

impl Driver<f64, f64> for WildersDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut f64>,
        options: Vec<Option<&f64>>,
    ) {
        let len = outputs[0][0].len();

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut wilders = Simd::<f64, N>::from_array(std::array::from_fn(|i| unsafe {
            **states.get_unchecked(i)
        }));

        let multiplier_simd = {
            let mut multipliers = [0.0; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&multiplier) = option {
                    multipliers[lane] = multiplier;
                }
            }
            Simd::from_array(multipliers)
        };

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let output_ptrs = crate::extract_output_ptrs!(outputs, N, sma_line_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index_splat!(i, N,
                new @ input_ptrs
            );

            wilders = calc_simd(wilders, real, multiplier_simd);

            crate::write_simd_at_indices!(N, i,
                output_ptrs => wilders
            );
        }

        // Update states efficiently
        let final_wilders = wilders.to_array();
        for (i, state) in states.iter_mut().enumerate().take(N) {
            **state = final_wilders[i];
        }
    }
}

pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [(f64, f64); N] = std::array::from_fn(|i| multiplier(options[i][0] as usize));

    let mut road_train = PrimeMover::<N, f64, f64>::new();
    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = output_length(inputs[0].len(), options[i]);
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    for (i, option) in options.iter().enumerate() {
        let period = option[0] as usize;
        let asset_inputs = vec![inputs[0]];

        let (state, _) = init_state(inputs[0], period);
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
                period,
                0,
                state,
                Some(&params[i].0),
            ));
        }
    }
    let mut driver = WildersDriver {};
    let wilders = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (&wilder, multipliers) in wilders.iter().zip(params.into_iter()) {
        states.push(IndicatorState::new(wilder, multipliers));
    }
    Ok((output_buffers, states))
}

/*pub fn indicator_by_assets_from_state<const N: usize>(
    inputs: &[ &[ &[f64]; INPUTS_WIDTH]; N],
    states: &mut [IndicatorState; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<[Vec<Vec<f64>>; N], IndicatorError>
{
    let len = inputs[0][0].len();

    // Validate all inputs have same length
    for i in 0..N {
        if inputs[i][0].len() != len {
            return Err(IndicatorError::InvalidInputs);
        }
    }

    // Extract EMAs and multipliers from states
    let mut emas = Simd::from_array(std::array::from_fn(|i| states[i].get_ema()));
    let multipliers = states[0].get_multipliers();
    let multipliers_simd = (Simd::splat(multipliers.0), Simd::splat(multipliers.1));

    // Create output arrays and process directly
    let mut ema_lines: [Vec<Vec<f64>>; N] = std::array::from_fn(|_| {
        vec![crate::uninit_vec!(f64, len)]
    });

    for i in 0..len {
        //let values: [f64; N] = (0..N).map(|j| inputs[j][0][i]).collect::<Vec<_>>().try_into().unwrap();
        let values: [f64; N] = std::array::from_fn(|j| inputs[j][0][i]);

        let vals = Simd::from_array(values);
        emas = calc_simd(vals, emas, multipliers_simd);
        let outputs = emas.to_array();
        for j in 0..N {
            unsafe { *ema_lines[j].get_unchecked_mut(0).get_unchecked_mut(i) = outputs[j] }
        }
    }

    // Update states with final EMA values
    let final_emas = emas.to_array();
    for i in 0..N {
        states[i].set_ema(final_emas[i]);
    }

    Ok(ema_lines)
}*/
