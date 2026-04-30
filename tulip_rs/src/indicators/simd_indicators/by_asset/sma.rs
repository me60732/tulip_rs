//use crate::common::validate_inputs;
use crate::indicators::sma::{
    min_data, multiplier, output_length, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::sma_simd::calc_simd;

struct SmaDriver {
    multiplier: f64,
    period: usize,
}

impl Driver<f64> for SmaDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut f64>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut sums = Simd::<f64, N>::from_array(std::array::from_fn(|i| unsafe {
            **states.get_unchecked(i)
        }));

        let multiplier_simd = Simd::splat(self.multiplier);

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let output_ptrs = crate::extract_output_ptrs!(outputs, N, output_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.period..len).enumerate() {

            let (old_vals, new_vals) = crate::extract_simd_at_indices!(N, input_ptrs,
                old_vals @ j,
                new_vals @ i
            );

            let sma = calc_simd(&mut sums, new_vals, old_vals, multiplier_simd);

            crate::write_simd_at_indices!(N, j,
                output_ptrs => sma
            );
        }

        // Update states efficiently
        let final_sums = sums.to_array();
        for (i, state) in states.iter_mut().enumerate().take(N) {
            **state = final_sums[i];
        }
    }
}

pub fn init_state<'a, const N: usize>(inputs: &[&'a [f64]; N], period: usize) -> (Vec<f64>, f64) {
    let multiplier = multiplier(period);
    let mut sums = Simd::<f64, N>::splat(0.0);

    // Optimization: Pre-compute input pointers for the initialization loop
    let input_ptrs: [*const f64; N] = std::array::from_fn(|i| inputs[i].as_ptr());

    for i in 0..period {
        let values = Simd::from_array(std::array::from_fn(|j| unsafe { *input_ptrs[j].add(i) }));
        sums += values;
    }

    (sums.to_array().to_vec(), multiplier)
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
    let (sums, multiplier) = init_state(&real, period);

    let mut road_train = PrimeMover::<N, f64>::new();
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
                sums[i],
                None,
            ));
        }
    }
    let mut driver = SmaDriver { multiplier, period };
    let sums = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, &sum) in sums.iter().enumerate() {
        states.push(IndicatorState::new(
            unsafe { inputs.get_unchecked(i).get_unchecked(0) },
            sum,
            multiplier,
            period,
        ));
    }
    Ok((output_buffers, states))
}
