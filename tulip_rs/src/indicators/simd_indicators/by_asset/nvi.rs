//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::nvi::{
    min_data, output_length, IndicatorState as State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::nvi_simd::SimdState;

/// SIMD driver that advances the Negative Volume Index (NVI) across `N` asset lanes per scheduling epoch.
struct NviDriver;

impl Driver<State> for NviDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::new(&states);

        // Optimization 2: Pre-compute all input and output pointers
        let (close_ptrs, volume_ptrs) =
            crate::extract_input_ptrs!(inputs, N, close_ptrs, volume_ptrs);

        let output_ptrs = crate::extract_output_ptrs!(outputs, N, output_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            let (close, volume) = crate::extract_simd_inputs_at_index!(i, N,
                close @ close_ptrs,
                volume @ volume_ptrs
            );

            let nvi = state.calc_simd(close, volume);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                output_ptrs => nvi
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Negative Volume Index (NVI) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// NVI accumulates price-change contributions only on bars where volume declines.
/// It requires no configurable options. Uses the [`PrimeMover`] scheduler to batch
/// assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[close, volume]` for asset `i`.
/// * `_options` - Unused; NVI has no configurable options.
/// * `_optional_outputs` - Unused; NVI produces only the single NVI line output.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the NVI line for asset `i`
/// and `states[i]` is the final [`State`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<State>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(_options))?;
    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = output_length(inputs[i][0].len(), &[]);
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    for i in 0..N {
        let state = State::new(1000.0, inputs[i][0][0], inputs[i][1][0]);
        let asset_inputs = vec![&inputs[i][0][1..], &inputs[i][1][1..]];
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
                0,
                0,
                state,
                None,
            ));
        }
    }
    let mut driver = NviDriver;
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
