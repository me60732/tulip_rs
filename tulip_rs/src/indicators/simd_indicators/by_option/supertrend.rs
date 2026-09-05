//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::supertrend_simd::{SimdState, TSimdState, TState};
use crate::indicators::{
    medprice::Medprice,
    supertrend::{
        validate_options as vo, Indicator, IndicatorState, State, SuperTrend, INPUTS, OPTIONS,
    },
    tr::Tr,
};

use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver for the SuperTrend indicator, processing `N` option-set lanes per scheduling epoch.
struct SuperTrendDriver {
    want_optional_outputs: (bool, bool, bool, bool),
}

impl Driver<State<Warm>> for SuperTrendDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    /// Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::from_states(&mut states);
        let len = outputs[0][0].len();

        let (has_optional, want_atr, want_tr, want_medprice) = self.want_optional_outputs;
        //collect outputs
        let (super_line_ptr, atr_line_ptr, tr_line_ptr, medprice_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, st, atr, tr, medprice);

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

            let (st, atr, tr, medprice) = state.calc(inputs);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                super_line_ptr => st
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_tr, tr_line_ptr => tr,
                    want_atr, atr_line_ptr => atr,
                    want_medprice, medprice_line_ptr => medprice
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the SuperTrend indicator on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[high, low, close]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period, step]`.
/// * `optional_outputs` - Optional output flags: `[want_atr, want_tr, want_medprice]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains
/// `[supertrend, atr?, tr?, medprice?]` and `states[i]` is the final
/// [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, SuperTrend::min_data)?;
    validate_options(options, Some(vo))?;

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut want_optional_outputs = (false, false, false, false);
    let mut output_buffers = Vec::with_capacity(N);

    let [high, low, close] = *inputs;
    for i in 0..N {
        let asset_inputs = vec![high, low, close];

        let (st_line, (atr_line, mut tr_line, mut medprice_line)) = {
            let capacity = SuperTrend::output_length(high.len(), options[i]);
            let tr_capacity = Tr::output_length(high.len(), &[]);
            let med_capacity = Medprice::output_length(high.len(), &[]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false, false],
                    atr_line: capacity,
                    tr_line: tr_capacity,
                    medprice_line: med_capacity
                ),
            )
        };

        let (period, step) = (options[i][0] as usize, options[i][1]);
        let state = State::init_state(
            high,
            low,
            close,
            period,
            step,
            &mut tr_line,
            &mut medprice_line,
        );

        let mut starts = [0; 4];
        (starts[1], starts[2], starts[3]) =
            crate::slice_outputs_start!(st_line.len(), atr_line, tr_line, medprice_line);
        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(atr_line, tr_line, medprice_line);
        }

        let mut output_buffer = vec![st_line, atr_line, tr_line, medprice_line];

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

    let mut driver = SuperTrendDriver {
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
