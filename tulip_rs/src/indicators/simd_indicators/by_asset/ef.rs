use crate::indicators::ef::{
    Ef, Indicator, IndicatorState, INPUTS, OPTIONS, State
};
use crate::indicators::simd_indicators::ef_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Efficiency Ratio (EF) indicator across `N` asset
/// lanes per scheduling epoch.
struct EfDriver {
    period: usize,
}

impl Driver<State<Warm>> for EfDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][0]` (real prices), writes the EF line to `outputs[asset][0]`,
    /// and updates `states[asset]` (the rolling absolute-movement sum) in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Direct array construction
        let mut state = SimdState::from_states(&mut states);
        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let output_ptrs = crate::extract_output_ptrs!(outputs, N, output_ptrs);

        // Optimized main loop with minimal overhead
        for (j, i) in (self.period..len).enumerate() {
            let inputs = crate::extract_simd_at_indices!(N, input_ptrs,
                value @ i,
                last_value @ j
            );

            let ef = state.calc(inputs);
            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, j,
                output_ptrs => ef
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Efficiency Ratio (EF) indicator for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[real]` for asset `i`.
/// * `options` - Shared options slice; `options[0]` is the period.
/// * `_optional_outputs` - Unused; EF has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the EF line for asset `i`
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Ef::min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    // Create output buffers OUTSIDE the assets - these will be owned by this function
    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, State<Warm>>::new();

    for i in 0..N {
        let len = inputs[i][0].len();
        let capacity = Ef::output_length(len, options);
        let mut ef_line = crate::uninit_vec!(f64, capacity);

        let state = State::init_state(inputs[i][0], period, &mut ef_line);
        let asset_inputs = vec![inputs[i][0]];

        let mut output_buffer = vec![ef_line];
        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        unsafe {
            //let slice_len = output_buffer.len() - starts[j];
            // Get a mutable reference to the output buffer for this asset
            let output_buffer = &mut output_buffer[0];
            asset_outputs.push(std::slice::from_raw_parts_mut(
                output_buffer.as_mut_ptr().add(1), //slice from
                output_buffer.len() - 1,           // slice to
            ));
        }
        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period+1,
            period,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = EfDriver { period };
    let final_states = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in final_states.into_iter().enumerate() {
        states.push(IndicatorState::new(inputs[i][0], state, period));
    }
    Ok((output_buffers, states))
}
