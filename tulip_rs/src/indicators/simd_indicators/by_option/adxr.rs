//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

use crate::indicators::adxr::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::adxr_simd::options::{calc_unchecked_simd, SimdState};
use crate::indicators::{
    adx::output_length as adx_output_length, dx::output_length as dx_output_length,
    tr::output_length as tr_output_length,
};

/// SIMD driver for the Average Directional Movement Rating (ADXR) indicator, processing `N` option-set lanes per scheduling epoch.
struct AdxrDriver {
    want_optional_outputs: (bool, bool, bool, bool, bool),
}

impl Driver<State, (f64, f64)> for AdxrDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
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
            let mut multipliers = [0.0; N];
            let mut inv_multipliers = [0.0; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&multiplier) = option {
                    //println!("{:?}", outputs[lane][0].len());
                    multipliers[lane] = multiplier.0;
                    inv_multipliers[lane] = multiplier.1;
                }
            }
            (
                Simd::from_array(multipliers),
                Simd::from_array(inv_multipliers),
            )
        };
        let (has_optional, want_adx, want_dx, want_atr, want_tr) = self.want_optional_outputs;
        //collect outputs
        let (adxr_line_ptr, adx_line_ptr, dx_line_ptr, atr_line_ptr, tr_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            adxr_line_ptr,
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

            let (adxr, adx, dx, atr, tr) =
                unsafe { calc_unchecked_simd(&mut state, high, low, close, multipliers) };
            //unsafe { calc_simd(&mut state, high, low, close, multiplier) };
            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                adxr_line_ptr => adxr
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_adx, adx_line_ptr => adx,
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

/// Calculates the Average Directional Movement Rating (ADXR) on a single asset with `N` different option
/// sets simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[high, low, close]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Optional output flags: `[want_adx, want_dx, want_atr, want_tr]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[adxr, adx?, dx?, atr?, tr?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;

    let params: [(f64, f64); N] = std::array::from_fn(|i| multiplier(options[i][0] as usize));

    let mut road_train = PrimeMover::<N, State, (f64, f64)>::new();
    let mut want_optional_outputs = (false, false, false, false, false);
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let period = options[i][0] as usize;
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
            inputs[2], // close
        ];

        let (adxr_line, (mut adx_line, mut dx_line, mut atr_line, mut tr_line), start) = {
            let len = inputs[0].len();
            let adxr_capacity = output_length(len, options[i]);
            let adx_capacity = adx_output_length(len, options[i]);
            let dx_capacity = dx_output_length(len, options[i]);
            let tr_capacity = tr_output_length(len, options[i]);

            (
                crate::uninit_vec!(f64, adxr_capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false, false, false],
                    adx_line: adx_capacity,
                    dx_line: dx_capacity,
                    atr_line: dx_capacity,
                    tr_line: tr_capacity
                ),
                len - adxr_capacity,
            )
        };

        let state = State::init_state(
            inputs[0],
            inputs[1],
            inputs[2],
            period,
            (&mut adx_line, &mut dx_line, &mut atr_line, &mut tr_line),
        );

        let mut starts = [0; 5];
        (starts[1], starts[2], starts[3], starts[4]) =
            crate::slice_outputs_start!(adxr_line.len(), adx_line, dx_line, atr_line, tr_line);
        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(adx_line, dx_line, atr_line, tr_line);
        }

        let mut output_buffer = vec![adxr_line, adx_line, dx_line, atr_line, tr_line];

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
            start,
            0,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = AdxrDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, multipliers) in states_vec.into_iter().zip(params.into_iter()) {
        states.push(IndicatorState::new(state, multipliers));
    }
    Ok((output_buffers, states))
}
