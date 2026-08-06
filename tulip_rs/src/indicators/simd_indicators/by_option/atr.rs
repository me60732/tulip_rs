//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicator_types::TSimdState;
use crate::indicators::atr::{Atr, Indicator, IndicatorState, State, INPUTS, OPTIONS};
use crate::indicators::simd_indicators::atr_simd::{SimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::tr::Tr;
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver for the Average True Range (ATR) indicator, processing `N` option-set lanes per scheduling epoch.
struct AtrDriver {
    want_optional_outputs: bool,
}

impl Driver<State<Warm>> for AtrDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::from_states(&mut states);
        let len = outputs[0][0].len();

        //collect outputs
        let (atr_line_ptr, tr_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, atr_line_ptr, tr_line_ptr);

        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let inputs = crate::extract_simd_inputs_at_index_splat!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs
            );

            let (atr, tr) = state.calc(inputs);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                atr_line_ptr => atr
            );

            crate::store_simd_optional_outputs!(i, N,
                self.want_optional_outputs, tr_line_ptr => tr
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Average True Range (ATR) on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[high, low, close]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Optional output flags: `[want_tr]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[atr, tr?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Atr::min_data)?;
    validate_options(options, None)?;

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut want_optional_outputs = false;
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
            inputs[2], // close
        ];
        let len = inputs[0].len();
        let atr_capacity = Atr::output_length(len, options[i]);
        let atr_line = crate::uninit_vec!(f64, atr_capacity);

        let mut tr_line = crate::init_optional_outputs_eff!(
            optional_outputs, &[false],
            tr_line: Tr::output_length(len, &[])
        );
        let period = options[i][0] as usize;
        let state = State::init_state(inputs[0], inputs[1], inputs[2], period, &mut tr_line, false);

        let mut starts = [0; 2];
        starts[1] = crate::slice_outputs_start!(atr_capacity, tr_line);
        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(tr_line);
        }

        let mut output_buffer = vec![atr_line, tr_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(starts[j]), //slice from
                    output_buffer.len() - starts[j],           // slice to
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
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = AtrDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    Ok((output_buffers, states_vec))
}
