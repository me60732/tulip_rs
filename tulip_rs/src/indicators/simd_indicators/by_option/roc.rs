//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::roc::{
    min_data, output_length, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::roc_simd::calc_simd;

struct RocDriver {
    want_optional_outputs: bool,
}

impl Driver<bool, usize> for RocDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut _states: Vec<&mut bool>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();
        let mut i_simd = {
            let mut i = [0usize; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&period) = option {
                    i[lane] = period;
                }
            }
            Simd::from_array(i)
        };
        // Optimization 2: Pre-compute all input and output pointers
        let (roc_line_ptr, mom_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, roc_line_ptr, mom_line_ptr);

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let one_splat = Simd::splat(1);
        // Optimization 3: Simplified main loop with pre-computed offsets
        for j in 0..len {
            let new_vals = crate::extract_simd_inputs_at_index_array!(i_simd, N,
                new @ input_ptrs
            );

            let old_vals = crate::extract_simd_inputs_at_index!(j, N,
                old @ input_ptrs
            );

            let (roc, mom) = calc_simd(new_vals, old_vals);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                roc_line_ptr => roc
            );
            crate::store_simd_optional_outputs!(j, N,
                self.want_optional_outputs, mom_line_ptr => mom
            );
            i_simd += one_splat;
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

    let periods: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);
    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, bool, usize>::new();
    let mut want_optional_outputs = false;
    for (i, &period) in periods.iter().enumerate() {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let (roc_line, mom_line) = {
            let capacity = output_length(inputs[0].len(), options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    mom_line: capacity
                ),
            )
        };

        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(mom_line);
        }

        let mut output_buffer = vec![roc_line, mom_line];
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
            false,
            Some(&periods[i]),
        ));
        output_buffers.push(output_buffer);
    }
    
    let mut driver = RocDriver { want_optional_outputs };
    road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for i in 0..N {
        states.push(IndicatorState::new(inputs[0], periods[i]));
    }
    Ok((output_buffers, states))
}
