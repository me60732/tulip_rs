use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::ema::output_length as ema_output_length;
use crate::indicators::macd::{
    min_data, multiplier, output_length, validate_options as vo, IndicatorState, State,
    INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::macd_simd::{calc_simd, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver for the Moving Average Convergence Divergence (MACD) indicator, processing `N` option-set lanes per scheduling epoch.
struct MacdDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State, ((f64, f64), (f64, f64), (f64, f64))> for MacdDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&((f64, f64), (f64, f64), (f64, f64))>>,
    ) {
        let len = inputs[0][0].len();

        let mut state = SimdState::new(&states);

        let multipliers_simd = {
            let mut multipliers = (
                ([0.0; N], [0.0; N]),
                ([0.0; N], [0.0; N]),
                ([0.0; N], [0.0; N]),
            );
            for (lane, option) in options.iter().enumerate() {
                if let Some(&multiplier) = option {
                    //println!("{:?}", outputs[lane][0].len());
                    multipliers.0 .0[lane] = multiplier.0 .0;
                    multipliers.0 .1[lane] = multiplier.0 .1;
                    multipliers.1 .0[lane] = multiplier.1 .0;
                    multipliers.1 .1[lane] = multiplier.1 .1;
                    multipliers.2 .0[lane] = multiplier.2 .0;
                    multipliers.2 .1[lane] = multiplier.2 .1;
                }
            }
            (
                (
                    Simd::from_array(multipliers.0 .0),
                    Simd::from_array(multipliers.0 .1),
                ),
                (
                    Simd::from_array(multipliers.1 .0),
                    Simd::from_array(multipliers.1 .1),
                ),
                (
                    Simd::from_array(multipliers.2 .0),
                    Simd::from_array(multipliers.2 .1),
                ),
            )
        };
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
            let values = crate::extract_simd_inputs_at_index_splat!(i, N, values @ input_ptrs);

            let (macd, signal, histogram) = calc_simd(&mut state, values, multipliers_simd);

            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, i,
                macd_line_ptr => macd,
                signal_line_ptr => signal,
                histogram_line_ptr => histogram
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

/// Calculates the Moving Average Convergence Divergence (MACD) on a single asset with `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
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
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, Some(vo))?;
    let params: [((f64, f64), (f64, f64), (f64, f64)); N] = std::array::from_fn(|i| {
        multiplier(
            options[i][0] as usize,
            options[i][1] as usize,
            options[i][2] as usize,
        )
    });
    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, State, ((f64, f64), (f64, f64), (f64, f64))>::new();
    let mut want_optional_outputs = (false, false, false);

    for i in 0..N {
        let short_period = options[i][0] as usize;
        let long_period = options[i][1] as usize;
        let signal_period = options[i][2] as usize;

        let len = inputs[0].len();
        let (macd_capacity, signal_capacity, histogram_capacity) = output_length(len, options[i]);

        let short_ema_capacity = ema_output_length(len, &[short_period as f64]);
        let long_ema_capacity = ema_output_length(len, &[long_period as f64]);
        // Pre-allocate the result vectors with the calculated capacities
        let mut macd_line = crate::uninit_vec!(f64, macd_capacity);
        let signal_line = crate::uninit_vec!(f64, signal_capacity);
        let histogram = crate::uninit_vec!(f64, histogram_capacity);

        let (mut short_ema_line, mut long_ema_line) = crate::init_optional_outputs!(
            optional_outputs, &[false, false],
            short_ema_line: short_ema_capacity,
            long_ema_line: long_ema_capacity
        );

        let state = State::init_state(
            inputs[0],
            (short_period, long_period, signal_period),
            params[i],
            &mut macd_line,
            (&mut short_ema_line, &mut long_ema_line),
        );
        let asset_inputs = vec![inputs[0]];
        let mut starts = [0; 5];
        (starts[0], starts[3], starts[4]) =
            crate::slice_outputs_start!(signal_capacity, macd_line, short_ema_line, long_ema_line);

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
                    output_buffer.len(),                       // slice to
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
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = MacdDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, &multipliers) in states_vec.into_iter().zip(params.iter()) {
        states.push(IndicatorState::new(multipliers, state));
    }
    Ok((output_buffers, states))
}
