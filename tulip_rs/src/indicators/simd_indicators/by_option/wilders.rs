//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::wilders_simd::{SimdState, TSimdState, TState};
use crate::indicators::wilders::{Indicator, IndicatorState, State, Wilders, INPUTS, OPTIONS};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver for the Wilder's Smoothing (WILDERS) indicator, processing `N` option-set lanes per scheduling epoch.
struct WildersDriver {}

impl Driver<State<Warm>> for WildersDriver {
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
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let output_ptrs = crate::extract_output_ptrs!(outputs, N, sma_line_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index_splat!(i, N,
                new @ input_ptrs
            );

            let wilders = state.calc(real);

            crate::write_simd_at_indices!(N, i,
                output_ptrs => wilders
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates Wilder's Smoothing (WILDERS) for one shared asset across `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch option sets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - Shared input data: `inputs[0]` is `&[f64]` containing `real` (price series).
/// * `options` - An array of `N` option sets; `options[i]` is `&[f64; OPTIONS]` containing
///   `[period]` for option set `i`.
/// * `optional_outputs` - Unused; WILDERS has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `wilders` for option set `i`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or any option set is invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Wilders::min_data)?;
    validate_options(options, None)?;

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let wilders_line = {
            let len = inputs[0].len();
            let capacity = Wilders::output_length(len, options[i]);
            crate::uninit_vec!(f64, capacity)
        };
        let period = options[i][0] as usize;
        let state = State::init_state(inputs[0], period);

        let mut output_buffer = vec![wilders_line];

        //let adosc_len = output_buffer[0].len();
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
            period,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = WildersDriver {};
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
