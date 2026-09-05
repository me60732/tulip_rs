//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::rsi::{Indicator, IndicatorState, Rsi, State, INPUTS, OPTIONS};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::rsi_simd::{SimdState, TSimdState, TState};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver for the Relative Strength Index (RSI) indicator, processing `N` option-set lanes per scheduling epoch.
struct RsiDriver;

impl Driver<State<Warm>> for RsiDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = outputs[0][0].len();

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::<N>::from_states(&mut states);
        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        let rsi_line_ptr = crate::extract_output_ptrs!(outputs, N, rsi_line_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get new and old values using pre-computed pointers
            let current = crate::extract_simd_inputs_at_index_splat!(i, N,
                current @ real_ptrs
            );

            let rsi = state.calc(current);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                rsi_line_ptr => rsi
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Relative Strength Index (RSI) indicator for one asset with `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// Applies each of the `N` period configurations to the same shared input series, computing
/// RSI values for all option sets in a single SIMD-accelerated pass via [`PrimeMover`].
///
/// # Arguments
/// * `inputs` - Shared input: `inputs[0]` is the `real` price series.
/// * `options` - An array of `N` option sets; `options[i][0]` is the `period` for lane `i`.
/// * `_optional_outputs` - Unused; RSI has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the `rsi` series for option set `i`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Rsi::min_data)?;
    validate_options(options, None)?;

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for (i, &option) in options.iter().enumerate() {
        let period = option[0] as usize;
        let asset_inputs = vec![inputs[0]];
        let rsi_line = {
            let capacity = Rsi::output_length(inputs[0].len(), option);
            crate::uninit_vec!(f64, capacity)
        };
        let mut output_buffer = vec![rsi_line];

        let state = State::init_state(inputs[0], period);

        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr(), //slice from
                    output_buffer.len(),        // slice to
                ));
            }
        }
        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period + 1,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = RsiDriver {};
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
