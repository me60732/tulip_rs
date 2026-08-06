use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::roofingfilter_simd::{SimdState, TSimdState, TState};
use crate::indicators::{
    roofingfilter::{RoofingFilter, Indicator, IndicatorState, State, INPUTS, OPTIONS},
    highpass::HighPass
};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Ehlers Roofing Filter across `N` asset lanes per scheduling epoch.
struct RoofingDriver {
    want_optional_outputs: bool
}

impl Driver<State> for RoofingDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][0]` (real), writes the RoofingFilter output to
    /// `outputs[asset][0]` and the optional HighPass output to `outputs[asset][1]`,
    /// and updates `states[asset]` in place.
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
        let (rf_line, hp_line) = crate::extract_output_ptrs!(outputs, N, rf, hp);
        let want_hp = self.want_optional_outputs;
        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index!(i, N, values @ real_ptrs);

            let (rf, hp) = state.calc(real);

            crate::write_simd_at_indices!(N, i,
                rf_line => rf
            );
            crate::store_simd_optional_outputs!(i, N,
                want_hp, hp_line => hp
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Ehlers Roofing Filter for `N` assets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[real]` for asset `i`.
/// * `options` - Shared options slice; `options[0]` is `ss_period`, `options[1]` is `hp_period`.
/// * `optional_outputs` - Pass `Some(&[true])` to also emit the `highpass` line per asset.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the `roofing` line for asset `i`,
/// `outputs[i][1]` is the optional `highpass` line (empty if not requested), and
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N],
    options: &[f64; OPTIONS],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, RoofingFilter::min_data(options))?;
    validate_options(options)?;
    let periods = (options[0] as usize, options[1] as usize);

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State>::new();
    let mut want_optional_outputs = false;
    for i in 0..N {
        let asset_inputs = vec![inputs[i][0]];
        let (rf_line, mut hp_line) = {
            let len = inputs[i][0].len();
            let capacity = RoofingFilter::output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    hp_line: HighPass::output_length(len, &[options[1]])
                )
            )
        };

        let state = State::init_state(inputs[i][0], periods, &mut hp_line);
        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(hp_line);
        }
        let mut starts = [0; 2];
        starts[1] = crate::slice_outputs_start!(rf_line.len(), hp_line);
        let mut output_buffer = vec![rf_line, hp_line];
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(starts[j]),
                    output_buffer.len() - starts[j],
                ));
            }
        }
        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            periods.0.max(periods.1),
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = RoofingDriver { want_optional_outputs };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
