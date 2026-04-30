//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
//use std::simd::cmp::SimdPartialOrd;
use std::simd::Simd;

use crate::indicators::ema::{
    min_data, multiplier, output_length, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH,
};
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::ema_simd::calc_simd;

struct EmaDriver {
    multiplier: f64,
    inv_multiplier: f64,
}

impl Driver<f64> for EmaDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut f64>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Direct array construction
        let mut emas = Simd::<f64, N>::from_array(std::array::from_fn(|i| unsafe {
            **states.get_unchecked(i)
        }));

        let multipliers_simd = (
            Simd::splat(self.multiplier),
            Simd::splat(self.inv_multiplier),
        );

        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let output_ptrs = crate::extract_output_ptrs!(outputs, N, output_ptrs);

        // Optimized main loop with minimal overhead
        for i in 0..len {
            let values = crate::extract_simd_inputs_at_index!(i, N, values @ input_ptrs);
            emas = calc_simd(values, emas, multipliers_simd);

            crate::write_simd_at_indices!(N, i,
                output_ptrs => emas
            );
        }

        // Update states efficiently
        let final_emas = emas.to_array();
        for (i, state) in states.iter_mut().enumerate() {
            **state = final_emas[i];
        }
    }
}

fn init_state<'a, const N: usize>(
    inputs: &[&'a [f64]; N],
    period: usize,
) -> (Vec<f64>, (f64, f64)) {
    let multipliers = multiplier(period);
    let multipliers_simd = (
        Simd::<f64, N>::splat(multipliers.0),
        Simd::splat(multipliers.1),
    );

    let input_ptrs: [*const f64; N] = std::array::from_fn(|i| inputs[i].as_ptr());
    let mut emas = Simd::from_array(std::array::from_fn(|j| unsafe { *input_ptrs[j].add(0) }));

    for i in 1..period {
        //let vals = Simd::from_array(get_vals(inputs, i));
        let values = Simd::from_array(std::array::from_fn(|j| unsafe { *input_ptrs[j].add(i) }));
        emas = calc_simd(values, emas, multipliers_simd);
    }

    (emas.to_array().to_vec(), multipliers)
}

pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    //let real: Vec<&[f64]> = (0..N).map(|i| inputs[i][0]).collect();
    let real: [&[f64]; N] = std::array::from_fn(|i| inputs[i][0]);
    //init ema, sliced inputs and multipliers
    let (emas, (multiplier, inv_multiplier)) = init_state(&real, period);

    // Create output buffers OUTSIDE the assets - these will be owned by this function
    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = output_length(inputs[i][0].len(), options);
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    let mut road_train = PrimeMover::<N, f64>::new();

    for i in 0..N {
        let asset_inputs = vec![real[i]]; //vec![sliced_inputs[i]];
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
                emas[i],
                None,
            ));
        }
    }
    let mut driver = EmaDriver {
        multiplier,
        inv_multiplier,
    };
    let emas = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for ema in emas {
        states.push(IndicatorState::new(ema, (multiplier, inv_multiplier)));
    }
    Ok((output_buffers, states))
}

pub fn indicator_by_assets_from_state<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N],
    states: &mut [IndicatorState; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<[Vec<Vec<f64>>; N], IndicatorError> {
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
    let mut ema_lines: [Vec<Vec<f64>>; N] =
        std::array::from_fn(|_| vec![crate::uninit_vec!(f64, len)]);

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
}
