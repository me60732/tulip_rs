use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::ema::Ema;
use crate::indicators::macd::{
    validate_options as vo, Indicator, IndicatorState, Macd, State, INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::macd_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver for the Moving Average Convergence Divergence (MACD) indicator, processing `N` option-set lanes per scheduling epoch.
struct MacdDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State<Warm>> for MacdDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        let mut state = SimdState::<N>::from_states(&mut states);

        let (has_optional, want_short_ema, want_long_ema) = self.want_optional_outputs;
        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (
            macd_line_ptr,
            signal_line_ptr,
            histogram_line_ptr,
            short_ema_line_ptr,
            long_ema_line_ptr,
        ) = crate::extract_output_ptrs!(
            outputs,
            N,
            macd_line_ptr,
            signal_line_ptr,
            histogram_line_ptr,
            short_ema_line_ptr,
            long_ema_line_ptr
        );

        // Optimized main loop with minimal overhead
        for i in 0..len {
            let inputs = crate::extract_simd_inputs_at_index_splat!(i, N, values @ input_ptrs);

            let (macd, signal, histogram, short_ema, long_ema) = state.calc(inputs);

            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, i,
                macd_line_ptr => macd,
                signal_line_ptr => signal,
                histogram_line_ptr => histogram
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_short_ema, short_ema_line_ptr => short_ema,
                    want_long_ema, long_ema_line_ptr => long_ema
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Moving Average Convergence Divergence (MACD) on a single asset with `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[real]`.
/// * `options` - An array of `N` option sets, one per SIMD lane:
///   `[short_period, long_period, signal_period]`.
/// * `optional_outputs` - Optional output flags: `[want_short_ema, want_long_ema]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains
/// `[macd_line, signal_line, histogram, short_ema?, long_ema?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Macd::min_data)?;
    validate_options(options, Some(vo))?;

    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut want_optional_outputs = (false, false, false);

    for i in 0..N {
        let short_period = options[i][0] as usize;
        let long_period = options[i][1] as usize;
        let signal_period = options[i][2] as usize;

        let len = inputs[0].len();
        let caps = Macd::slot_lengths(len, options[i]);

        let short_ema_capacity = Ema::output_length(len, &[short_period as f64]);
        let long_ema_capacity = Ema::output_length(len, &[long_period as f64]);
        // Pre-allocate the result vectors with the calculated capacities
        let mut macd_line = crate::uninit_vec!(f64, caps[0]);
        let signal_line = crate::uninit_vec!(f64, caps[1]);
        let histogram = crate::uninit_vec!(f64, caps[2]);

        let (mut short_ema_line, mut long_ema_line) = crate::init_optional_outputs!(
            optional_outputs, &[false, false],
            short_ema_line: short_ema_capacity,
            long_ema_line: long_ema_capacity
        );

        let state = State::init_state(
            inputs[0],
            (short_period, long_period, signal_period),
            &mut macd_line,
            (&mut short_ema_line, &mut long_ema_line),
        );
        let asset_inputs = vec![inputs[0]];
        let mut starts = [0; 5];
        (starts[0], starts[3], starts[4]) =
            crate::slice_outputs_start!(caps[1], macd_line, short_ema_line, long_ema_line);

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(short_ema_line, long_ema_line);
        }
        let mut output_buffer = vec![
            macd_line,
            signal_line,
            histogram,
            short_ema_line,
            long_ema_line,
        ];
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
        let start = long_period + signal_period - 2;
        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            start,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = MacdDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    Ok((output_buffers, states_vec))
}
