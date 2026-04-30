//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::kvo_simd::{calc_simd, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::{
    ema::output_length as ema_output_length,
    kvo::{
        min_data, multiplier, output_length, validate_options, IndicatorState, State, INPUTS_WIDTH,
        OPTIONS_WIDTH,
    },
};
use crate::types::IndicatorError;
use std::simd::Simd;

struct KvoDriver {
    multipliers: ((f64, f64), (f64, f64)),
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State> for KvoDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Direct array construction
        let mut simd_state = SimdState::new(&states);

        let multipliers_simd = (
            (
                Simd::splat(self.multipliers.0 .0),
                Simd::splat(self.multipliers.0 .1),
            ),
            (
                Simd::splat(self.multipliers.1 .0),
                Simd::splat(self.multipliers.1 .1),
            ),
        );
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
            let inputs = crate::extract_simd_inputs_at_index!(
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

pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let short_period = options[0] as usize;
    let long_period = options[1] as usize;
    let multipliers = multiplier(short_period, long_period);
    let mut want_optional_outputs = (false, false, false);
    // Create output buffers OUTSIDE the assets - these will be owned by this function
    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, State>::new();

    for i in 0..N {
        let len = inputs[i][0].len();
        let capacity = output_length(len, options);
        let short_capacity = ema_output_length(len, &[short_period as f64]);
        let kvo_line = crate::uninit_vec!(f64, capacity);

        let (mut short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            short_ema_line: short_capacity,
            long_ema_line: capacity
        );

        let state = State::init_state(
            (inputs[i][0], inputs[i][1], inputs[i][2], inputs[i][3]),
            &kvo_line,
            (short_period, long_period),
            &mut short_ema_line,
        );
        let input_start = len - capacity;
        let asset_inputs = vec![
            &inputs[i][0][input_start..],
            &inputs[i][1][input_start..],
            &inputs[i][2][input_start..],
            &inputs[i][3][input_start..],
        ];

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
            0,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = KvoDriver {
        multipliers: multiplier(short_period, long_period),
        want_optional_outputs,
    };
    let final_states = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in final_states.into_iter() {
        states.push(IndicatorState::new(multipliers, state));
    }
    Ok((output_buffers, states))
}
