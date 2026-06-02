//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

use crate::indicators::adx::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::adx_simd::{calc_simd, SimdState};
use crate::indicators::{
    dx::output_length as dx_output_length, tr::output_length as tr_output_length,
};

/// SIMD driver that advances the Average Directional Index (ADX) across `N` option-set lanes
/// per scheduling epoch.
struct AdxDriver {
    /// Optional output flags: `(has_optional, want_dx, want_atr, want_tr)`.
    want_optional_outputs: (bool, bool, bool, bool),
}

impl Driver<State, (f64, f64)> for AdxDriver {
    /// Processes one epoch of bars for `N` option lanes simultaneously using SIMD.
    ///
    /// Reads from `inputs[field]` (shared single asset's high, low, close), writes to
    /// `outputs[lane][output]`, and updates `states[lane]` in place.
    /// Per-lane Wilder multipliers are loaded from `options[lane]` at the start of each epoch.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(f64, f64)>>,
    ) {
        let mut state = SimdState::<N>::new(&mut states);
        let len = outputs[0][0].len();
        let multipliers = {
            let mut multipliers = ([0.0; N], [0.0; N]);
            for (lane, option) in options.iter().enumerate() {
                if let Some(&multiplier) = option {
                    //println!("{:?}", outputs[lane][0].len());
                    multipliers.0[lane] = multiplier.0;
                    multipliers.1[lane] = multiplier.1;
                }
            }
            (
                Simd::from_array(multipliers.0),
                Simd::from_array(multipliers.1),
            )
        };

        let (has_optional, want_dx, want_atr, want_tr) = self.want_optional_outputs;
        //collect outputs
        let (adx_line_ptr, dx_line_ptr, atr_line_ptr, tr_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            adx_line_ptr,
            dx_line_ptr,
            atr_line_ptr,
            tr_line_ptr
        );

        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let (high, low, close) = crate::extract_simd_inputs_at_index_splat!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs
            );

            let (adx, dx, atr, tr) = calc_simd(&mut state, high, low, close, multipliers);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                adx_line_ptr => adx
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_dx, dx_line_ptr => dx,
                    want_tr, tr_line_ptr => tr
                );
                crate::store_simd_optional_outputs_corrected!(i, N,
                    want_atr, atr_line_ptr => corrected(atr, multipliers.1)
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Average Directional Index (ADX) on a single asset with `N` different option
/// sets simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[high, low, close]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Optional output flags: `[want_dx, want_atr, want_tr]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[adx, dx?, atr?, tr?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;

    let multipliers: [(f64, f64); N] = std::array::from_fn(|i| multiplier(options[i][0] as usize));

    let mut road_train = PrimeMover::<N, State, (f64, f64)>::new();
    let mut want_optional_outputs = (false, false, false, false);
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
            inputs[2], // close
        ];

        let (adx_line, mut dx_line, mut atr_line, mut tr_line);
        {
            let len = inputs[0].len();
            let dx_capacity = dx_output_length(len, options[i]);
            let adx_capacity = output_length(len, options[i]);
            let tr_capacity = tr_output_length(len, &[]);
            adx_line = crate::uninit_vec!(f64, adx_capacity);

            (dx_line, atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                dx_line: dx_capacity,
                atr_line: dx_capacity,
                tr_line: tr_capacity
            );
        }
        let period = options[i][0] as usize;
        let state = State::init_state(
            inputs[0],
            inputs[1],
            inputs[2],
            period,
            (&mut dx_line, &mut atr_line, &mut tr_line),
        );

        let mut starts = [0; 4];
        (starts[1], starts[2], starts[3]) =
            crate::slice_outputs_start!(adx_line.len(), dx_line, atr_line, tr_line);
        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(dx_line, atr_line, tr_line);
        }

        let mut output_buffer = vec![adx_line, dx_line, atr_line, tr_line];

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
            period * 2 - 1,
            0,
            state,
            Some(&multipliers[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = AdxDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, &multipliers) in states_vec.into_iter().zip(multipliers.iter()) {
        states.push(IndicatorState::new(state, multipliers));
    }
    Ok((output_buffers, states))
}
