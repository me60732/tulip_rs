use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::apo::{
    min_data, multiplier, output_length, validate_options as vo, IndicatorState, INPUTS_WIDTH,
    OPTIONS_WIDTH,
};
use crate::indicators::ema::output_length as ema_output_length;
use crate::indicators::simd_indicators::apo_simd::{calc_simd, SimdState, State};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver for the Absolute Price Oscillator (APO) indicator, processing `N` option-set lanes per scheduling epoch.
struct ApoDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State, ((f64, f64), (f64, f64))> for ApoDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&((f64, f64), (f64, f64))>>,
    ) {
        let mut state = SimdState::<N>::new(&states);
        let len = outputs[0][0].len();
        let mut multipliers = (([0.0; N], [0.0; N]), ([0.0; N], [0.0; N]));

        for (lane, option) in options.iter().enumerate() {
            if let Some(&multiplier) = option {
                (
                    (multipliers.0 .0[lane], multipliers.0 .1[lane]),
                    (multipliers.1 .0[lane], multipliers.1 .1[lane]),
                ) = multiplier;
            }
        }
        let multipliers = (
            (
                Simd::from_array(multipliers.0 .0),
                Simd::from_array(multipliers.0 .1),
            ),
            (
                Simd::from_array(multipliers.1 .0),
                Simd::from_array(multipliers.1 .1),
            ),
        );

        let (has_optional, want_short_ema, want_long_ema) = self.want_optional_outputs;
        // Optimization 1: Direct array construction instead of collect+try_into

        //collect outputs
        let (apo_line_ptr, short_ema_line_ptr, long_ema_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            apo_line_ptr,
            short_ema_line_ptr,
            long_ema_line_ptr
        );

        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let real = crate::extract_simd_inputs_at_index_splat!(i, N, real @ real_ptrs);

            let apo = calc_simd(&mut state, real, multipliers);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                apo_line_ptr => apo
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

/// Calculates the Absolute Price Oscillator (APO) on a single asset with `N` different option
/// sets simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[close]`.
/// * `options` - An array of `N` option sets, one per SIMD lane:
///   `[short_period, long_period]`.
/// * `optional_outputs` - Optional output flags: `[want_short_ema, want_long_ema]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[apo, short_ema?, long_ema?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, Some(vo))?;

    let mut multipliers = [((0.0, 0.0), (0.0, 0.0)); N];
    for i in 0..N {
        let short_period = options[i][0] as usize;
        let long_period = options[i][1] as usize;
        multipliers[i] = multiplier(short_period, long_period);
    }

    let mut road_train = PrimeMover::<N, State, ((f64, f64), (f64, f64))>::new();
    let mut want_optional_outputs = (false, false, false);
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let short_period = options[i][0] as usize;
        let long_period = options[i][1] as usize;
        let asset_inputs = vec![
            inputs[0], // real
        ];
        let len = inputs[0].len();
        let apo_capacity = output_length(len, options[i]);
        let apo_line = crate::uninit_vec!(f64, apo_capacity);

        let (mut short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            short_ema_line: ema_output_length(len, &[short_period as f64]),
            long_ema_line: apo_capacity
        );

        let state = State::init_state(inputs[0], short_period, long_period, &mut short_ema_line);

        let mut starts = [0; 3];
        starts[1] = crate::slice_outputs_start!(apo_capacity, short_ema_line);
        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(short_ema_line, long_ema_line);
        }

        let mut output_buffer = vec![apo_line, short_ema_line, long_ema_line];

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
            long_period - 1,
            0,
            state,
            Some(&multipliers[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = ApoDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(state, multipliers[i]));
    }
    Ok((output_buffers, states))
}
