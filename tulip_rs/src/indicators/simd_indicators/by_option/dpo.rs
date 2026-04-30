//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::dpo::{
    init_state, min_data, multiplier, output_length, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::dpo_simd::calc_simd;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

struct DpoDriver {
    want_sma: bool,
}

impl Driver<f64, (usize, usize, f64)> for DpoDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut f64>,
        options: Vec<Option<&(usize, usize, f64)>>,
    ) {
        let len = outputs[0][0].len();

        let want_sma = self.want_sma;
        // Optimization 1: Direct array construction instead of collect+try_into
        let mut sums = Simd::<f64, N>::from_array(std::array::from_fn(|i| unsafe {
            **states.get_unchecked(i)
        }));
        let mut i = [0usize; N];
        let mut dpo_idx = [0usize; N];
        let multiplier_simd = {
            let mut multipliers = [0.0; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&(period, dpo_period, multiplier)) = option {
                    dpo_idx[lane] = period - dpo_period;
                    i[lane] = period;
                    multipliers[lane] = multiplier;
                }
            }
            Simd::from_array(multipliers)
        };

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs: [*const f64; N] =
            std::array::from_fn(|j| unsafe { inputs.get_unchecked(j).get_unchecked(0).as_ptr() });

        let (dpo_line_ptrs, sma_line_ptrs) =
            crate::extract_output_ptrs!(outputs, N, dpo_ptrs, sma_ptrs);
        // Optimization 3: Simplified main loop with pre-computed offsets
        for j in 0..len {
            let (new_vals, dpo_vals) = crate::extract_simd_at_indices_array!(N, input_ptrs,
                current @ i,
                dpo @ dpo_idx
            );
            let old_vals = crate::extract_simd_inputs_at_index!(j, N,
                old @ input_ptrs
            );

            let (dpo, sma) = calc_simd(new_vals, &mut sums, (old_vals, dpo_vals), multiplier_simd);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                dpo_line_ptrs => dpo
            );
            crate::store_simd_optional_outputs!(j, N,
                want_sma, sma_line_ptrs => sma
            );

            for (i, dpo_idx) in i.iter_mut().zip(dpo_idx.iter_mut()) {
                *i += 1;
                *dpo_idx += 1;
            }
        }

        // Update states efficiently
        let final_sums = sums.to_array();
        for (i, state) in states.iter_mut().enumerate().take(N) {
            **state = final_sums[i];
        }
    }
}

pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [(usize, usize, f64); N] = std::array::from_fn(|i| {
        (
            options[i][0] as usize,
            options[i][0] as usize / 2 + 1,
            multiplier(options[i][0] as usize),
        )
    });

    let mut road_train = PrimeMover::<N, f64, (usize, usize, f64)>::new();

    let mut want_sma = false;
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![inputs[0]];
        let (dpo_line, sma_line) = {
            let len = inputs[0].len();
            let capacity = output_length(len, options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };
        let period = options[i][0] as usize;
        let state = init_state(inputs[0], period);

        if i == 0 {
            (_, want_sma) = crate::calc_want_flags!(sma_line);
        }
        let mut output_buffer = vec![dpo_line, sma_line];

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
            period,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = DpoDriver { want_sma };
    let sums = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, &sum) in sums.iter().enumerate() {
        let (period, dpo_period, multiplier) = params[i];
        states.push(IndicatorState::new(
            unsafe { inputs.get_unchecked(0) },
            sum,
            multiplier,
            period,
            dpo_period,
        ));
    }
    Ok((output_buffers, states))
}
