//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::ema::output_length as ema_output_length;
use crate::indicators::ppo::{
    min_data, multiplier, output_length, validate_options, IndicatorState, State, INPUTS_WIDTH,
    OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::ppo_simd::SimdState;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

struct PpoDriver {
    multipliers: ((f64, f64), (f64, f64)),
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State> for PpoDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::new(&states);
        let len = inputs[0][0].len();
        let multipliers = (
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
        // Optimization 1: Direct array construction instead of collect+try_into

        //collect outputs
        let (ppo_line_ptr, short_ema_line_ptr, long_ema_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            ppo_line_ptr,
            short_ema_line_ptr,
            long_ema_line_ptr
        );

        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let real = crate::extract_simd_inputs_at_index!(
                i,
                N,
                real @ real_ptrs
            );

            let ppo = state.calc_simd(real, multipliers);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                ppo_line_ptr => ppo
            );

            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_short_ema, short_ema_line_ptr => state.short_ema,
                    want_long_ema, long_ema_line_ptr => state.long_ema
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
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

    let mut road_train = PrimeMover::<N, State>::new();
    let mut want_optional_outputs = (false, false, false);
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![inputs[i][0]];

        let ppo_capacity = output_length(inputs[i][0].len(), options);
        let ppo_line = crate::uninit_vec!(f64, ppo_capacity);

        let (mut short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            short_ema_line: ema_output_length(inputs[i][0].len(), &[short_period as f64]),
            long_ema_line: ppo_capacity
        );

        let state = State::init_state(
            inputs[i][0],
            (short_period, long_period),
            &mut short_ema_line,
        );

        let mut starts = [0; 3];
        starts[1] = crate::slice_outputs_start!(ppo_capacity, short_ema_line);
        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(short_ema_line, long_ema_line);
        }

        let mut output_buffer = vec![ppo_line, short_ema_line, long_ema_line];

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
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = PpoDriver {
        multipliers,
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state, multipliers));
    }
    Ok((output_buffers, states))
}
