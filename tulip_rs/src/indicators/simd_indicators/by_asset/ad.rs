//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicator_types::TSimdState;
use crate::indicators::ad::{IndicatorState, State, INPUTS, OPTIONS, Ad, Indicator};
use crate::indicators::simd_indicators::ad_simd::{SimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver that advances the Accumulation/Distribution (AD) line across `N` asset lanes
/// per scheduling epoch.
struct AdDriver;

impl Driver<State, ()> for AdDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][field]` (high, low, close, volume), writes the running AD value
    /// to `outputs[asset][0]`, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::<N>::from_states(&mut states);

        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs, close_ptrs, volume_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs, volume_ptrs);

        let ad_line_ptr = crate::extract_output_ptrs!(outputs, N, ad_line_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get new and old values using pre-computed pointers
            let inputs = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs,
                volume @ volume_ptrs
            );

            let ad = state.calc(inputs);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                ad_line_ptr => ad
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Accumulation/Distribution (AD) line for `N` assets simultaneously using SIMD
/// parallelism.
///
/// AD requires no configurable options. Uses the [`PrimeMover`] scheduler to batch assets into
/// SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[high, low, close, volume]` for asset `i`.
/// * `options` - Unused; AD has no configurable options.
/// * `optional_outputs` - Unused; AD produces only the single AD line output.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the AD line for asset `i`
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    _options: &[f64; OPTIONS],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Ad::min_data(_options))?;
    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = inputs[i][0].len();
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // high
            inputs[i][1], // low
            inputs[i][2], // close
            inputs[i][3], // volume
        ];
        let state = State::new(0.0);
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
    let mut driver = AdDriver {};
    let state_vec = road_train.drive(&mut driver);

    Ok((output_buffers, state_vec))
}
