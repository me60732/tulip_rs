use crate::indicators::simd_indicators::hilberttransform_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::{
    highpass::HighPass,
    hilberttransform::{
        HilbertTransform, Indicator, IndicatorState, State, INPUTS, OPTIONS,
    },
    roofingfilter::RoofingFilter,
};
use crate::types::{IndicatorError, Warm};
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Hilbert Transform across `N` asset lanes per scheduling epoch.
struct HilbertDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State<Warm>> for HilbertDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][0]` (real), applies the roofing filter then the
    /// Hilbert kernel, writes `in_phase` to `outputs[asset][0]`, `quadrature` to
    /// `outputs[asset][1]`, and the optional `roofing` / `highpass` outputs to
    /// `outputs[asset][2]` / `outputs[asset][3]`. Updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        let mut state = SimdState::from_states(&mut states);


        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real);
        let (p_line, q_line, rf_line, hp_line) =
            crate::extract_output_ptrs!(outputs, N, p, q, rf, hp);

        let (has_optional, want_rf, want_hp) = self.want_optional_outputs;

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index!(i, N, values @ real_ptrs);

            let (p, q, rf, hp) = state.calc(real);

            crate::write_simd_at_indices!(N, i,
                p_line => p,
                q_line => q
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_rf, rf_line => rf,
                    want_hp, hp_line => hp
                );
            }
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Hilbert Transform for `N` assets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[real]` for asset `i`.
/// * `options` - Shared options `[ss_period, hp_period]`.
/// * `optional_outputs` - Pass `Some(&[true, false])` for the roofing line, `Some(&[false, true])`
///   for the highpass line, `Some(&[true, true])` for both, or `None` for neither.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `in_phase`, `outputs[i][1]` is
/// `quadrature`, `outputs[i][2]` is `roofing` (empty unless requested), and
/// `outputs[i][3]` is `highpass` (empty unless requested) for asset `i`.
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N],
    options: &[f64; OPTIONS],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, HilbertTransform::min_data(options))?;
    validate_options(options)?;
    let periods = (options[0] as usize, options[1] as usize);

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut want_optional_outputs = (false, false, false);
    for i in 0..N {
        let asset_inputs = vec![inputs[i][0]];
        let (p_line, q_line, (mut rf_line, mut hp_line)) = {
            let len = inputs[i][0].len();
            let capacity = HilbertTransform::output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    rf_line: RoofingFilter::output_length(len, options),
                    hp_line: HighPass::output_length(len, &[periods.1 as f64])
                ),
            )
        };

        let state = State::init_state(
            inputs[i][0],
            periods,
            (&mut rf_line, &mut hp_line),
        );
        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(rf_line, hp_line);
        }
        let mut starts = [0; 4];
        (starts[2], starts[3]) = crate::slice_outputs_start!(p_line.len(), rf_line, hp_line);

        let mut output_buffer = vec![p_line, q_line, rf_line, hp_line];
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
            periods.0.max(periods.1) + 7,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = HilbertDriver {
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
