//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::kvo_simd::{calc_simd, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::{
    ema::output_length as ema_output_length,
    kvo::{
        min_data, multiplier, output_length, validate_options as vo, IndicatorState, State,
        INPUTS_WIDTH, OPTIONS_WIDTH,
    },
};
use crate::types::IndicatorError;
use std::simd::Simd;

struct KvoDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State, ((f64, f64), (f64, f64))> for KvoDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&((f64, f64), (f64, f64))>>,
    ) {
        let len = outputs[0][0].len();

        // Direct array construction
        let mut simd_state = SimdState::new(&states);

        let multipliers_simd = {
            let mut multipliers = (([0.0; N], [0.0; N]), ([0.0; N], [0.0; N]));
            for (lane, option) in options.iter().enumerate() {
                if let Some(&multiplier) = option {
                    multipliers.0 .0[lane] = multiplier.0 .0;
                    multipliers.0 .1[lane] = multiplier.0 .1;
                    multipliers.1 .0[lane] = multiplier.1 .0;
                    multipliers.1 .1[lane] = multiplier.1 .1;
                }
            }
            (
                (
                    Simd::from_array(multipliers.0 .0),
                    Simd::from_array(multipliers.0 .1),
                ),
                (
                    Simd::from_array(multipliers.1 .0),
                    Simd::from_array(multipliers.1 .1),
                ),
            )
        };
        let (has_optional, want_short_ema, want_long_ema) = self.want_optional_outputs;
        // Pre-compute pointers for maximum efficiency
        let (high_ptrs, low_ptrs, close_ptrs, volume_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs, volume_ptrs);
        let (kvo_line_ptr, short_ema_line_ptr, long_ema_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            kvo_line_ptr,
            short_ema_line_ptr,
            long_ema_line_ptr
        );

        // Optimized main loop with minimal overhead
        for i in 0..len {
            let inputs = crate::extract_simd_inputs_at_index_splat!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs,
                volume @ volume_ptrs
            );

            let kvo = calc_simd(&mut simd_state, inputs, multipliers_simd);

            crate::write_simd_at_indices!(N, i,
                kvo_line_ptr => kvo
            );

            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_short_ema, short_ema_line_ptr => simd_state.short_ema,
                    want_long_ema, long_ema_line_ptr => simd_state.long_ema
                );
            }
        }

        simd_state.write_states(&mut states);
    }
}

pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, Some(vo))?;

    let params: [((f64, f64), (f64, f64)); N] =
        std::array::from_fn(|i| multiplier(options[i][0] as usize, options[i][1] as usize));

    let mut want_optional_outputs = (false, false, false);
    // Create output buffers OUTSIDE the assets - these will be owned by this function
    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, State, ((f64, f64), (f64, f64))>::new();

    for i in 0..N {
        let short_period = options[i][0] as usize;
        let long_period = options[i][1] as usize;
        let len = inputs[0].len();
        let capacity = output_length(len, options[i]);
        let short_capacity = ema_output_length(len, &[short_period as f64]);
        let kvo_line = crate::uninit_vec!(f64, capacity);

        let (mut short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            short_ema_line: short_capacity,
            long_ema_line: capacity
        );

        let state = State::init_state(
            (inputs[0], inputs[1], inputs[2], inputs[3]),
            &kvo_line,
            (short_period, long_period),
            &mut short_ema_line,
        );
        let input_start = len - capacity;
        let asset_inputs = vec![inputs[0], inputs[1], inputs[2], inputs[3]];

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(short_ema_line, long_ema_line);
        }
        let mut starts = [0; 3];
        starts[1] = crate::slice_outputs_start!(capacity, short_ema_line);
        let mut output_buffer = vec![kvo_line, short_ema_line, long_ema_line];
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
            input_start,
            0,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = KvoDriver {
        want_optional_outputs,
    };
    let final_states = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, multipliers) in final_states.into_iter().zip(params) {
        states.push(IndicatorState::new(multipliers, state));
    }
    Ok((output_buffers, states))
}
