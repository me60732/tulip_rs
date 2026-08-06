//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::wad_simd::{SimdState, TSimdState, TState};
use crate::indicators::wad::{
   Wad, Indicator, IndicatorState as State, INPUTS, OPTIONS,
};
use crate::types::IndicatorError;
use std::simd::Simd;
/// SIMD driver that advances the WAD Indicator across `N` asset lanes per scheduling epoch.
struct WadDriver;

impl Driver<State> for WadDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let mut state = SimdState::from_states(&mut states);
        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);

        let output_ptrs = crate::extract_output_ptrs!(outputs, N, output_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            let inputs = crate::extract_simd_inputs_at_index!(i, N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs
            );

            let wad = state.calc(inputs);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                output_ptrs => wad
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the WAD Indicator for `N` assets simultaneously using SIMD parallelism.
///
/// WAD requires no configurable options and produces no optional outputs. Uses the
/// [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[high, low, close]` for asset `i`.
/// * `_options` - Unused; WAD has no configurable options.
/// * `_optional_outputs` - Unused; WAD has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the WAD line for asset `i` and
/// `states[i]` is the final state for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    _options: &[f64; OPTIONS],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<State>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Wad::min_data(_options))?;
    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = Wad::output_length(inputs[i][0].len(), _options);
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    for i in 0..N {
        let [high, low, close] = *inputs[i];
        let asset_inputs = vec![high, low, close];
        let state = State::new(close[0], 0.0);
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
                1,
                0,
                state,
                None,
            ));
        }
    }
    let mut driver = WadDriver;
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
