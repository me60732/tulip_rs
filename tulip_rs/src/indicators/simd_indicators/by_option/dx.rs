//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::dx::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::dx_simd::{calc_simd, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::tr::output_length as tr_output_length;
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver for the Directional Movement Index (DX) indicator, processing `N` option-set lanes per scheduling epoch.
struct DxDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State, (f64, f64)> for DxDriver {
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

        let (multiplier_simd, inv_multiplier_simd) = {
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

        let (has_optional, want_atr, want_tr) = self.want_optional_outputs;
        //collect outputs
        let (dx_line_ptr, atr_line_ptr, tr_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, dx_line_ptr, atr_line_ptr, tr_line_ptr);

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

            let (dx, atr, tr) = calc_simd(&mut state, high, low, close, multiplier_simd);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                dx_line_ptr => dx
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_tr, tr_line_ptr => tr
                );
                crate::store_simd_optional_outputs_corrected!(i, N,
                    want_atr, atr_line_ptr => corrected(atr, inv_multiplier_simd)
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Directional Movement Index (DX) on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[high, low, close]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Optional output flags: `[want_atr, want_tr]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[dx, atr?, tr?]`
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
    let mut want_optional_outputs = (false, false, false);
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
            inputs[2], // close
        ];

        let (dx_line, atr_line, mut tr_line);
        {
            let len = inputs[0].len();
            let capacity = output_length(len, options[i]);
            let tr_capacity = tr_output_length(len, options[i]);

            dx_line = crate::uninit_vec!(f64, capacity);

            (atr_line, tr_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                atr_line: capacity,
                tr_line: tr_capacity
            );
        }
        let period = options[i][0] as usize;
        let state = State::init_state(inputs[0], inputs[1], inputs[2], period, &mut tr_line);

        let mut starts = [0; 3];
        starts[2] = crate::slice_outputs_start!(dx_line.len(), tr_line);
        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(atr_line, tr_line);
        }

        let mut output_buffer = vec![dx_line, atr_line, tr_line];

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
            period,
            0,
            state,
            Some(&multipliers[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = DxDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(state, multipliers[i].1));
    }
    Ok((output_buffers, states))
}
