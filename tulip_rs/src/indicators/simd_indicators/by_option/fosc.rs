//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::fosc::{
    min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
pub use crate::indicators::simd_indicators::fosc_simd::{calc_simd, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::tsf::output_length as tsf_output_length;
use crate::types::IndicatorError;
use std::simd::Simd;
struct FoscDriver {
    want_optional_outputs: (bool, bool, bool, bool, bool),
}

impl Driver<State, usize> for FoscDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&usize>>,
    ) {
        let mut state = SimdState::<N>::new(&states);
        let len = outputs[0][0].len();
        let (mut i, period_simd) = {
            let mut i = [0usize; N];
            let mut periods = [0.0; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&period) = option {
                    i[lane] = period;
                    periods[lane] = period as f64;
                }
            }
            (i, Simd::from_array(periods))
        };
        let (has_optional, want_tsf, want_linreg, want_slope, want_intercept) =
            self.want_optional_outputs;
        // Optimization 1: Direct array construction instead of collect+try_into
        //collect outputs
        let (fosc_line_ptr, tsf_line_ptr, linreg_line_ptr, slope_line_ptr, intercept_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            fosc_line_ptr,
            tsf_line_ptr,
            linreg_line_ptr,
            slope_line_ptr,
            intercept_line_ptr
        );

        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for j in 0..len {
            // Get inputs arrays for stocks
            let real = crate::extract_simd_inputs_at_index_array!(i, N,
                new @ real_ptrs
            );
            let prev_real = crate::extract_simd_inputs_at_index!(j+1, N, real @ real_ptrs);
            let (fosc, tsf, linreg, slope, intercept) =
                calc_simd(&mut state, prev_real, real, period_simd);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                fosc_line_ptr => fosc
            );

            if has_optional {
                crate::store_simd_optional_outputs!(j, N,
                    want_tsf, tsf_line_ptr => tsf,
                    want_linreg, linreg_line_ptr => linreg,
                    want_slope, slope_line_ptr => slope,
                    want_intercept, intercept_line_ptr => intercept
                );
            }
            for i in i.iter_mut() {
                *i += 1;
            }
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
    let params: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);

    let mut road_train = PrimeMover::<N, State, usize>::new();
    let mut want_optional_outputs = (false, false, false, false, false);
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let capacity = output_length(inputs[0].len(), options[i]);
        let fosc_line = crate::uninit_vec!(f64, capacity);
        let (mut tsf_line, mut linreg_line, mut slope_line, mut intercept_line) = {
            let tsf_capacity = tsf_output_length(inputs[0].len(), options[i]);

            crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false, false],
                tsf_line: tsf_capacity,
                linreg_line: tsf_capacity,
                slope_line: tsf_capacity,
                intercept_line: tsf_capacity
            )
        };
        let period = options[i][0] as usize;
        let state = State::init_state(
            &inputs[0],
            period,
            (
                &mut tsf_line,
                &mut linreg_line,
                &mut slope_line,
                &mut intercept_line,
            ),
        );

        if i == 0 {
            want_optional_outputs =
                crate::calc_want_flags!(tsf_line, linreg_line, slope_line, intercept_line);
        }

        let mut starts = [0; 5];
        (starts[1], starts[2], starts[3], starts[4]) = crate::slice_outputs_start!(
            capacity,
            tsf_line,
            linreg_line,
            slope_line,
            intercept_line
        );

        let mut output_buffer = vec![fosc_line, tsf_line, linreg_line, slope_line, intercept_line];
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());
        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(starts[j]), //slice from
                    output_buffer.len(),                       // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period + 1,
            period,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = FoscDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            state,
            unsafe { inputs.get_unchecked(0) },
            params[i],
        ));
    }
    Ok((output_buffers, states))
}
