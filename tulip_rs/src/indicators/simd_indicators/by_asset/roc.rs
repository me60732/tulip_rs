//use crate::common::validate_inputs;
use crate::indicators::roc::{
    min_data, output_length, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::roc_simd::calc_simd;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
struct RocDriver {
    period: usize,
    want_optional_outputs: bool,
}

impl Driver<bool> for RocDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut _states: Vec<&mut bool>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        let (roc_line_ptr, mom_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, roc_line_ptr, mom_line_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.period..len).enumerate() {

            let (old_vals, new_vals) = crate::extract_simd_at_indices!(N, input_ptrs,
                old_vals @ j,
                new_vals @ i
            );

            let (roc, mom) = calc_simd(new_vals, old_vals);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                roc_line_ptr => roc
            );
            crate::store_simd_optional_outputs!(j, N,
                self.want_optional_outputs, mom_line_ptr => mom
            );
        }
    }
}

pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;

    let mut road_train = PrimeMover::<N, bool>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = false;

    for i in 0..inputs.len() {
        let asset_inputs = vec![inputs[i][0]];
        let (roc_line, mom_line) = {
            let capacity = output_length(inputs[i][0].len(), options);
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
            None,
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = RocDriver {
        period,
        want_optional_outputs,
    };
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
