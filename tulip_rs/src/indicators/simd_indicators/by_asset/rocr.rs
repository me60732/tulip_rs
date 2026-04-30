//use crate::common::validate_inputs;
use crate::indicators::rocr::{
    min_data, output_length, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::rocr_simd::calc_simd;

struct RocrDriver {
    period: usize,
}

impl Driver<bool> for RocrDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut _states: Vec<&mut bool>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Optimization 2: Pre-compute all input and output pointers
        let output_ptrs = crate::extract_output_ptrs!(
            outputs,
            N,
            output_ptr
        );

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        

        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.period..len).enumerate() {

            let (old_vals, new_vals) = crate::extract_simd_at_indices!(N, input_ptrs,
                old_vals @ j,
                new_vals @ i
            );

            let rocr = calc_simd(new_vals, old_vals);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                output_ptrs => rocr
            );
        }
    }
}

pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;

    let mut road_train = PrimeMover::<N, bool>::new();
    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = output_length(inputs[i][0].len(), options);
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    for i in 0..N {
        let asset_inputs = vec![inputs[i][0]];
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
                period,
                false,
                None,
            ));
        }
    }
    let mut driver = RocrDriver { period };
    road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for i in 0..N {
        states.push(IndicatorState::new(
            unsafe { inputs.get_unchecked(i).get_unchecked(0) },
            period,
        ));
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
