use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::highpass_simd::SimdState;
use crate::indicators::highpass::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Ehlers High Pass filter across `N` asset lanes per scheduling epoch.
struct HighPassDriver {
    multipliers: (f64, f64),
}

impl Driver<State> for HighPassDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][0]` (real), writes the HighPass output to
    /// `outputs[asset][0]`, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        let mut state = SimdState::new(&states);

        let multipliers_simd = (
            Simd::splat(self.multipliers.0),
            Simd::splat(self.multipliers.1)
        );

        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real);
        let highpass_line_ptr = crate::extract_output_ptrs!(outputs, N, super_line);

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index!(i, N, values @ real_ptrs);

            let hp = state.calc_simd(real, multipliers_simd);

            crate::write_simd_at_indices!(N, i,
                highpass_line_ptr => hp
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Ehlers High Pass filter for `N` assets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[real]` for asset `i`.
/// * `options` - Shared options slice; `options[0]` is the period.
/// * `_optional_outputs` - Unused; HighPass has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the HighPass line for asset `i`
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let multipliers = multiplier(period);

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State>::new();

    for i in 0..N {
        let asset_inputs = vec![inputs[i][0]];
        let highpass_line = {
            let capacity = output_length(inputs[i][0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::init_state(inputs[i][0], period, multipliers);

        let mut output_buffer = vec![highpass_line];
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

    let mut driver = HighPassDriver { multipliers };
    let final_states = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in final_states.into_iter() {
        states.push(IndicatorState::new(state, driver.multipliers));
    }
    Ok((output_buffers, states))
}
