//use crate::common::validate_inputs;
use crate::indicators::adx::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::adx_simd::{calc_simd, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::{
    dx::output_length as dx_output_length, tr::output_length as tr_output_length,
};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Average Directional Index (ADX) across `N` asset lanes per
/// scheduling epoch.
struct AdxDriver {
    multipliers: (f64, f64),
    /// Optional output flags: `(has_optional, want_dx, want_atr, want_tr)`.
    want_optional_outputs: (bool, bool, bool, bool),
}

impl Driver<State> for AdxDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][field]` (high, low, close), writes to
    /// `outputs[asset][output]`, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::new(&mut states);
        let len = inputs[0][0].len();
        let multipliers = (Simd::splat(self.multipliers.0), Simd::splat(self.multipliers.1));
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
            let (high, low, close) = crate::extract_simd_inputs_at_index!(
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

/// Calculates the Average Directional Index (ADX) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// All assets share the same `options`. Uses the [`PrimeMover`] scheduler to batch assets into
/// SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close]` for asset `i`.
/// * `options` - Shared options applied to all `N` assets: `[period]`.
/// * `optional_outputs` - Optional output flags: `[want_dx, want_atr, want_tr]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[adx, dx?, atr?, tr?]`
/// for asset `i` and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;

    let multipliers = multiplier(period);

    let mut road_train = PrimeMover::<N, State>::new();
    let mut want_optional_outputs = (false, false, false, false);
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // high
            inputs[i][1], // low
            inputs[i][2], // close
        ];

        let (adx_line, mut dx_line, mut atr_line, mut tr_line);
        {
            let len = inputs[i][0].len();
            let dx_capacity = dx_output_length(len, options);
            let adx_capacity = output_length(len, options);
            let tr_capacity = tr_output_length(len, &[]);
            adx_line = crate::uninit_vec!(f64, adx_capacity);

            (dx_line, atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                dx_line: dx_capacity,
                atr_line: dx_capacity,
                tr_line: tr_capacity
            );
        }

        let state = State::init_state(
            inputs[i][0],
            inputs[i][1],
            inputs[i][2],
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
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = AdxDriver {
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
