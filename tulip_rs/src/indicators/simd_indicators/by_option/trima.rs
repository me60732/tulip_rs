//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::trima_simd::option::SimdState;
use crate::indicators::simd_indicators::trima_simd::{TSimdState, TState};
use crate::indicators::trima::{Indicator, IndicatorState, State, Trima, INPUTS, OPTIONS};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver for the Triangular Moving Average (TRIMA) indicator, processing `N` option-set lanes per scheduling epoch.
struct TrimaDriver;

impl Driver<State<Warm>, usize> for TrimaDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();
        let mut state = SimdState::from_states(&mut states);

        // Pre-sliced input convention (matches `Asset::new`'s `start_offset = m1` per lane):
        // the shared lookback bar sits at slice index `j`, the current bar at
        // `m1_lane + j` — same shape as by-assets trima / by-option sma, no per-bar
        // subtraction in the hot loop.
        let mut i = {
            let mut i = [0usize; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&m1) = option {
                    i[lane] = m1;
                }
            }
            i
        };

        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let trima_line_ptr = crate::extract_output_ptrs!(outputs, N, trima_line_ptr);

        // Optimized main loop with minimal overhead
        for j in 0..len {
            let x_minus_m1_simd = crate::extract_simd_inputs_at_index!(j, N,
                x @ input_ptrs
            );
            let real = crate::extract_simd_inputs_at_index_array!(i, N,
                r @ input_ptrs
            );

            let trima = state.calc((real, x_minus_m1_simd));

            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, j,
                trima_line_ptr => trima
            );
            for i in i.iter_mut() {
                *i += 1;
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Triangular Moving Average (TRIMA) for one shared asset across `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch option sets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - Shared input data: `inputs[0]` is `&[f64]` containing `real` (price series).
/// * `options` - An array of `N` option sets; `options[i]` is `&[f64; OPTIONS]` containing
///   `[period]` for option set `i`.
/// * `optional_outputs` - Unused; TRIMA has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `trima` for option set `i`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or any option set is invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Trima::min_data)?;
    validate_options(options, None)?;

    let params: [usize; N] = std::array::from_fn(|i| (options[i][0] as usize + 1) / 2);

    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = Trima::output_length(inputs[0].len(), options[i]);
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    let mut road_train = PrimeMover::<N, State<Warm>, usize>::new();
    for i in 0..N {
        let period = options[i][0] as usize;
        let state = State::init_state(inputs[0], period);
        let asset_inputs = vec![inputs[0]];

        unsafe {
            // Get a mutable reference to the output buffer for this asset
            let output_buffer = &mut output_buffers[i][0];
            let asset_outputs = vec![std::slice::from_raw_parts_mut(
                output_buffer.as_mut_ptr(),
                output_buffer.len(),
            )];
            road_train.add_asset(Asset::new(
                asset_inputs,
                asset_outputs,
                i,
                period - 1,
                params[i],
                state,
                Some(&params[i]),
            ));
        }
    }

    let mut driver = TrimaDriver {};
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(inputs[0], state));
    }
    Ok((output_buffers, states))
}
