//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::vosc_simd::SimdState;
use crate::indicators::{
    sma::output_length as sma_output_length,
    vosc::{
        min_data, multiplier, output_length, validate_options as vo, IndicatorState, State,
        INPUTS_WIDTH, OPTIONS_WIDTH,
    },
};
use crate::types::IndicatorError;
use std::simd::Simd;
struct Params {
    multipliers: (f64, f64),
    long_period: usize,
    short_period: usize,
}
struct VoscDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State, Params> for VoscDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&Params>>,
    ) {
        let len = outputs[0][0].len();

        let mut i = [0usize; N];
        let mut short = [0usize; N];
        let (short_multiplier_simd, long_multiplier_simd) = {
            let mut short_multiplier = [0.0; N];
            let mut long_multiplier = [0.0; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(param) = option {
                    short[lane] = param.long_period - param.short_period;
                    i[lane] = param.long_period;
                    short_multiplier[lane] = param.multipliers.0;
                    long_multiplier[lane] = param.multipliers.1;
                }
            }
            (
                Simd::from_array(short_multiplier),
                Simd::from_array(long_multiplier),
            )
        };
        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::new(&states);
        let (has_optional, want_short_sma, want_long_sma) = self.want_optional_outputs;

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (vosc_line_ptr, short_sma_line_ptr, long_sma_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            vosc_line_ptr,
            short_sma_line_ptr,
            long_sma_line_ptr
        );

        // Optimization 3: Simplified main loop with pre-computed offsets
        for j in 0..len {
            let long_volume = crate::extract_simd_inputs_at_index!(j, N, long @ input_ptrs);

            let (volume, short_volume) = crate::extract_simd_at_indices_array!(N, input_ptrs,
                value @ i,
                short_value @ short
            );

            let (vosc, short_sma, long_sma) = state.calc_simd(
                (volume, short_volume, long_volume),
                short_multiplier_simd,
                long_multiplier_simd,
            );

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                vosc_line_ptr => vosc
            );

            if has_optional {
                crate::store_simd_optional_outputs!(j, N,
                    want_short_sma, short_sma_line_ptr => short_sma,
                    want_long_sma, long_sma_line_ptr => long_sma
                );
            }

            for (i, short) in i.iter_mut().zip(short.iter_mut()) {
                *i += 1;
                *short += 1;
            }
        }

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
        short_period: options[i][0] as usize,
        long_period: options[i][1] as usize,
        multipliers: multiplier(options[i][0] as usize, options[i][1] as usize),
    });

    let mut road_train = PrimeMover::<N, State, Params>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = (false, false, false);

    for (i, param) in params.iter().enumerate() {
        let asset_inputs = vec![inputs[0]];
        let (vosc_line, (mut short_sma_line, long_sma_line)) = {
            let len = inputs[0].len();
            let capacity = output_length(len, options[i]);
            let short_capacity = sma_output_length(len, &[param.short_period as f64]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    short_sma_line: short_capacity,
                    long_sma_line: capacity
                ),
            )
        };

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(short_sma_line, long_sma_line);
        }
        let mut starts = [0; N];
        starts[1] = crate::slice_outputs_start!(vosc_line.len(), short_sma_line);

        let state = State::init_state(
            param.short_period,
            param.long_period,
            inputs[0],
            &mut short_sma_line,
        );

        let mut output_buffer = vec![vosc_line, short_sma_line, long_sma_line];

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
            param.long_period,
            param.long_period,
            state,
            Some(&param),
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = VoscDriver {
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    let mut indicator_states = Vec::with_capacity(N);
    for (state, param) in states.into_iter().zip(params.into_iter()) {
        indicator_states.push(IndicatorState::new(
            inputs[0],
            state,
            param.multipliers,
            (param.short_period, param.long_period),
        ));
    }
    Ok((output_buffers, indicator_states))
}
