use crate::common_simd::assets::validate_inputs;
use crate::indicators::homodynediscriminator::{
    HomodyneDiscriminator, Indicator, IndicatorState, State, INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::homodynediscriminator_simd::{
    SimdState, TSimdState, TState,
};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver that advances the Homodyne Discriminator across `N` asset lanes per scheduling epoch.
struct HomodyneDriver;

impl Driver<State<Warm>> for HomodyneDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Gathers per-asset states into a [`SimdState`], runs the four-stage HT pipeline
    /// and homodyne discriminator for every bar in the epoch, then scatters the
    /// updated state back into the per-asset scalar states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let mut simd_state = SimdState::<N>::from_states(&mut states);

        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let dc_period_ptrs = crate::extract_output_ptrs!(outputs, N, dc_period_ptrs);

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index!(i, N, real @ real_ptrs);
            let dc = simd_state.calc(real);
            crate::write_simd_at_indices!(N, i, dc_period_ptrs => dc);
        }

        simd_state.write_states(&mut states);
    }
}

/// Calculates the Ehlers Homodyne Discriminator for `N` assets simultaneously using
/// SIMD parallelism.
///
/// Each asset's state is independently warmed up via [`State::init_state`] (consuming
/// the first 22 bars), then all assets are batched by the [`PrimeMover`] scheduler
/// and advanced together through the SIMD pipeline starting at bar 22.
///
/// # Arguments
/// * `inputs` — `N` asset input sets; `inputs[i]` is `[&[f64]; 1]` containing `[real]`
///   for asset `i`.
/// * `options` — Unused; the Homodyne Discriminator has no configurable parameters.
///   Pass `&[]`.
/// * `_optional_outputs` — Unused; the indicator produces no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `dc_period` (SmoothPeriod) for asset `i`
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError::NotEnoughData)` if any input is shorter than
/// [`min_data`] (23 bars).
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N],
    options: &[f64; OPTIONS],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, HomodyneDiscriminator::min_data(options))?;

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State<Warm>>::new();

    for i in 0..N {
        // Warm up this asset's scalar state: processes bars 0..21 (22 bars)
        // until all five ring buffers are full.
        let state = State::init_state(inputs[i][0]);

        let dc_period_line = {
            let capacity = HomodyneDiscriminator::output_length(inputs[i][0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };
        let mut output_buffer = vec![dc_period_line];

        let asset_outputs = unsafe {
            let buf = &mut output_buffer[0];
            vec![std::slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.len())]
        };

        road_train.add_asset(Asset::new(
            vec![inputs[i][0]],
            asset_outputs,
            i,
            // inputs_idx = 22 = min_data - 1: init_state consumed bars 0..21,
            // so the driver's first slice starts at bar 22.
            HomodyneDiscriminator::min_data(options) - 1,
            // start_offset = 0: no warm-up prepend needed — the state is already hot.
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = HomodyneDriver;
    let final_states = road_train.drive(&mut driver);

    Ok((output_buffers, final_states))
}
