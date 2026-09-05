//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

use crate::indicators::simd_indicators::keltnerchannel_simd::{SimdState, TSimdState, TState};
use crate::indicators::{
    keltnerchannel::{
        validate_options as vo, Indicator, IndicatorState, KeltnerChannel, State, INPUTS, OPTIONS,
    },
    tr::Tr,
};

/// SIMD driver that advances the Keltner Channel indicator across `N` option-set lanes
/// per scheduling epoch.
struct KeltnerChannelDriver {
    /// Optional output flags: `(has_optional, want_atr, want_tr)`.
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State<Warm>> for KeltnerChannelDriver {
    /// Processes one epoch of bars for `N` option lanes simultaneously using SIMD.
    ///
    /// Reads from `inputs[field]` (shared single asset's high, low, close), writes to
    /// `outputs[lane][output]`, and updates `states[lane]` in place.
    /// Per-lane Wilder multipliers are loaded from `options[lane]` at the start of each epoch.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::from_states(&mut states);
        let len = outputs[0][0].len();

        let (has_optional, want_atr, want_tr) = self.want_optional_outputs;
        //collect outputs
        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);
        let (lower_band_ptr, middle_band_ptr, upper_band_ptr, atr_line_ptr, tr_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            lower_band_ptr,
            middle_band_ptr,
            upper_band_ptr,
            atr_line_ptr,
            tr_line_ptr
        );

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

            let (lower_band, middle_band, upper_band, atr, tr) = state.calc(inputs);

            crate::write_simd_at_indices!(N, i,
                lower_band_ptr => lower_band,
                middle_band_ptr => middle_band,
                upper_band_ptr => upper_band
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_atr, atr_line_ptr => atr,
                    want_tr, tr_line_ptr => tr
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Keltner Channel indicator on a single asset with `N` different option
/// sets simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[high, low, close]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period, step]`.
/// * `optional_outputs` - Optional output flags: `[want_atr, want_tr]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[lower, middle, upper, atr?, tr?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, KeltnerChannel::min_data)?;
    validate_options(options, Some(vo))?;

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut want_optional_outputs = (false, false, false);
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
            inputs[2], // close
        ];
        let (period, step) = (options[i][0] as usize, options[i][1]);
        let (middle_band, upper_band, lower_band, (atr_line, mut tr_line)) = {
            let len = inputs[0].len();
            let capacity = KeltnerChannel::output_length(len, options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    atr_line: capacity,
                    tr_line: Tr::output_length(len, &[])
                ),
            )
        };

        let state = State::init_state(inputs[0], inputs[1], inputs[2], period, step, &mut tr_line);

        let mut starts = [0; 5];
        starts[4] = crate::slice_outputs_start!(upper_band.len(), tr_line);
        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(atr_line, tr_line);
        }

        let mut output_buffer = vec![lower_band, middle_band, upper_band, atr_line, tr_line];

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

    let mut driver = KeltnerChannelDriver {
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
