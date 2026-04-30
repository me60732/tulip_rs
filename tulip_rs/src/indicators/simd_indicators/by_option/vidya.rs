//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::vidya_simd::SimdState;
use crate::indicators::stddev::output_length as stddev_output_length;
use crate::indicators::vidya::{
    min_data, multiplier, output_length, validate_options as vo, IndicatorState, State,
    INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;
struct Params {
    multipliers: (f64, f64),
    periods: (usize, usize),
    alpha: f64,
}
struct VidyaDriver {
    want_optional_outputs: (bool, bool, bool, bool, bool),
}

impl Driver<State, Params> for VidyaDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&Params>>,
    ) {
        let len = outputs[0][0].len();

        let mut state = SimdState::new(&mut states);

        let mut i = [0usize; N];
        let mut short = [0usize; N];
        let (multipliers_simd, alpha_simd) = {
            let mut multipliers = ([0.0; N], [0.0; N]);
            let mut alpha = [0.0; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(param) = option {
                    short[lane] = param.periods.1 - param.periods.0;
                    i[lane] = param.periods.1;
                    multipliers.0[lane] = param.multipliers.0;
                    multipliers.1[lane] = param.multipliers.1;
                    alpha[lane] = param.alpha;
                }
            }
            (
                (
                    Simd::from_array(multipliers.0),
                    Simd::from_array(multipliers.1),
                ),
                Simd::from_array(alpha),
            )
        };

        let (has_optional, want_short_sma, want_long_sma, want_short_sd, want_long_sd) =
            self.want_optional_outputs;
        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (
            vidya_line_ptr,
            short_sma_line_ptr,
            long_sma_line_ptr,
            short_sd_line_ptr,
            long_sd_line_ptr,
        ) = crate::extract_output_ptrs!(
            outputs,
            N,
            vidya_line_ptr,
            short_sma_line_ptr,
            long_sma_line_ptr,
            short_sd_line_ptr,
            long_sd_line_ptr
        );

        // Optimized main loop with minimal overhead
        for j in 0..len {
            let long_value = crate::extract_simd_inputs_at_index!(j, N, long @ input_ptrs);

            let (value, short_value) = crate::extract_simd_at_indices_array!(N, input_ptrs,
                value @ i,
                short_value @ short
            );

            let (vidya, short_sma, long_sma, short_sd, long_sd) =
                state.calc_simd(value, short_value, long_value, alpha_simd, multipliers_simd);

            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, j,
                vidya_line_ptr => vidya
            );

            if has_optional {
                crate::store_simd_optional_outputs!(j, N,
                    want_short_sma, short_sma_line_ptr => short_sma,
                    want_long_sma, long_sma_line_ptr => long_sma,
                    want_short_sd, short_sd_line_ptr => short_sd,
                    want_long_sd, long_sd_line_ptr => long_sd
                );
            }

            for (i, short) in i.iter_mut().zip(short.iter_mut()) {
                *i += 1;
                *short += 1;
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
    validate_options(options, Some(vo))?;
    let params: [Params; N] = std::array::from_fn(|i| Params {
        periods: (options[i][0] as usize, options[i][1] as usize),
        multipliers: multiplier(options[i][0] as usize, options[i][1] as usize),
        alpha: options[i][2],
    });

    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, State, Params>::new();
    let mut want_optional_outputs = (false, false, false, false, false);

    for (i, &option) in options.iter().enumerate() {
        let len = inputs[0].len();
        let capacity = output_length(len, option);
        let short_period = option[0] as usize;
        let long_period = option[1] as usize;
        let alpha = option[2];

        let (
            mut vidya_line,
            mut short_sma_line,
            mut long_sma_line,
            mut short_sd_line,
            mut long_sd_line,
        );
        {
            let short_capacity = stddev_output_length(len, &[option[0]]);
            let long_capacity = stddev_output_length(len, &[option[1]]);
            vidya_line = crate::uninit_vec!(f64, capacity);
            (short_sma_line, long_sma_line, short_sd_line, long_sd_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false, false],
                short_sma_line: short_capacity,
                long_sma_line: long_capacity,
                short_sd_line: short_capacity,
                long_sd_line: long_capacity
            );
        }

        let state = State::init_state(
            short_period,
            long_period,
            inputs[0],
            alpha,
            &mut vidya_line,
            (
                &mut short_sma_line,
                &mut long_sma_line,
                &mut short_sd_line,
                &mut long_sd_line,
            ),
        );

        let asset_inputs = vec![inputs[0]];
        let mut starts = [1; 5];
        (starts[1], starts[2], starts[3], starts[4]) = crate::slice_outputs_start!(
            capacity - 1,
            short_sma_line,
            long_sma_line,
            short_sd_line,
            long_sd_line
        ); //capacity - 1 because vidya_line recieve 1 output bar in init_state

        if i == 0 {
            want_optional_outputs =
                crate::calc_want_flags!(short_sma_line, long_sma_line, short_sd_line, long_sd_line);
        }
        let mut output_buffer = vec![
            vidya_line,
            short_sma_line,
            long_sma_line,
            short_sd_line,
            long_sd_line,
        ];
        //let adosc_len = output_buffer[0].len();
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
            long_period,
            long_period,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = VidyaDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, param) in states_vec.into_iter().zip(params.into_iter()) {
        states.push(IndicatorState::new(
            inputs[0],
            state,
            param.periods,
            param.multipliers,
            param.alpha,
        ));
    }
    Ok((output_buffers, states))
}
