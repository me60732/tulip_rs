//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::zlema_simd::SimdState;
use crate::indicators::zlema::{
    min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Zero Lag Exponential Moving Average (ZLEMA) across `N` asset lanes per scheduling epoch.
struct ZlemaDriver {
    lag: usize,
}

impl Driver<State> for ZlemaDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let mut state = SimdState::new(&states);

        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let zlema_line_ptr = crate::extract_output_ptrs!(outputs, N, zlema_line_ptr);

        // Optimized main loop with minimal overhead
        for (j, i) in (self.lag..len).enumerate() {
            let (lagged, current) = crate::extract_simd_at_indices!(N, input_ptrs,
                lagged @ j,
                current @ i
            );

            let zlema = state.calc_simd(current, lagged);

            crate::write_simd_at_indices!(N, j,
                zlema_line_ptr => zlema
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Zero Lag Exponential Moving Average (ZLEMA) for `N` assets simultaneously
/// using SIMD parallelism.
///
/// ZLEMA produces no optional outputs. Uses the [`PrimeMover`] scheduler to batch assets into
/// SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[real]` for asset `i`.
/// * `options` - `options[0]` is the `period`.
/// * `_optional_outputs` - Unused; ZLEMA has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the ZLEMA line for asset `i` and
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let lag = ((period.saturating_sub(1)) / 2).max(1);

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = output_length(inputs[i][0].len(), options);
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    for i in 0..N {
        let state = State::new(inputs[i][0], lag, period);

        let asset_inputs = vec![inputs[i][0]];
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
                lag,
                lag,
                state,
                None,
            ));
        }
    }

    let mut driver = ZlemaDriver { lag };
    let states = road_train.drive(&mut driver);

    let mut indicator_states = Vec::with_capacity(N);
    for (i, state) in states.into_iter().enumerate() {
        indicator_states.push(IndicatorState::new(inputs[i][0], state, lag));
    }
    Ok((output_buffers, indicator_states))
}
