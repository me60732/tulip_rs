//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::wilders_simd::{SimdState, TSimdState};
use crate::indicators::wilders::{
    Wilders, Indicator, IndicatorState, INPUTS, OPTIONS, State, TState
};
use crate::types::{IndicatorError, Warm};
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances Wilder's Smoothing (WILDERS) across `N` asset lanes per scheduling epoch.
struct WildersDriver;

impl Driver<State<Warm>> for WildersDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        let mut state = SimdState::<N>::from_states(&mut states);

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let output_ptrs = crate::extract_output_ptrs!(outputs, N, sma_line_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            let real = crate::extract_simd_at_indices!(N, input_ptrs,
                real @ i
            );

            let wilders = state.calc(real);

            crate::write_simd_at_indices!(N, i,
                output_ptrs => wilders
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates Wilder's Smoothing (WILDERS) for `N` assets simultaneously using SIMD parallelism.
///
/// WILDERS produces no optional outputs. Uses the [`PrimeMover`] scheduler to batch assets into
/// SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[real]` for asset `i`.
/// * `options` - `options[0]` is the `period`.
/// * `_optional_outputs` - Unused; WILDERS has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the Wilder's Smoothing line for asset `i` and
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Wilders::min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;


    let mut road_train = PrimeMover::<N, State<Warm>>::new();

    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // close
        ];
        let capacity = Wilders::output_length(inputs[i][0].len(), options);
        let wilders_line = crate::uninit_vec!(f64, capacity);

        let state = State::init_state(
            inputs[i][0],
            period,
        );

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
                    output_buffer.len(),           // slice to
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

    let mut driver = WildersDriver;
    let states_vec = road_train.drive(&mut driver);

    Ok((output_buffers, states_vec))
}
