use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::supersmoother_simd::{SimdState, TSimdState, TState};
use crate::indicators::supersmoother::{
    SuperSmoother, Indicator, IndicatorState, State, INPUTS, OPTIONS,
};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Ehlers Super Smoother across `N` asset lanes per scheduling epoch.
struct SuperSmootherDriver;

impl Driver<State> for SuperSmootherDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][0]` (real), writes the SuperSmoother output to
    /// `outputs[asset][0]`, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        let mut state = SimdState::<N>::from_states(&mut states);


        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real);
        let super_line_ptr = crate::extract_output_ptrs!(outputs, N, super_line);

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index!(i, N, values @ real_ptrs);

            let super_smoother = state.calc(real);

            crate::write_simd_at_indices!(N, i,
                super_line_ptr => super_smoother
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Ehlers Super Smoother for `N` assets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[real]` for asset `i`.
/// * `options` - Shared options slice; `options[0]` is the period.
/// * `_optional_outputs` - Unused; SuperSmoother has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the SuperSmoother line for asset `i`
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N],
    options: &[f64; OPTIONS],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, SuperSmoother::min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State>::new();

    for i in 0..N {
        let asset_inputs = vec![inputs[i][0]];
        let super_line = {
            let capacity = SuperSmoother::output_length(inputs[i][0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::init_state(inputs[i][0], period);

        let mut output_buffer = vec![super_line];
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr(),
                    output_buffer.len(),
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

    let mut driver = SuperSmootherDriver;
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
