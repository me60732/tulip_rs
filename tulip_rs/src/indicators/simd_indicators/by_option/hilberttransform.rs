use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::hilberttransform_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::{
    highpass::HighPass,
    hilberttransform::{HilbertTransform, Indicator, IndicatorState, State, INPUTS, OPTIONS},
    roofingfilter::RoofingFilter,
};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;
/// SIMD driver for the Hilbert Transform indicator, processing `N` option-set lanes
/// per scheduling epoch using a shared input series.
struct HilbertDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State<Warm>> for HilbertDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    ///
    /// Reads the shared real input, assembles per-lane coefficient vectors from `options`,
    /// advances the Hilbert Transform state, and writes `in_phase`, `quadrature` and the
    /// optional `roofing` / `highpass` outputs for each lane.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = outputs[0][0].len();

        let real_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (p_line, q_line, rf_line, hp_line) =
            crate::extract_output_ptrs!(outputs, N, p, q, rf, hp);

        let mut state = SimdState::<N>::from_states(&mut states);
        let (has_optional, want_rf, want_hp) = self.want_optional_outputs;

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index_splat!(i, N,
                new @ real_ptrs
            );

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

/// Calculates the Hilbert Transform on a single asset with `N` different option
/// sets simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[real]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[ss_period, hp_period]`.
/// * `optional_outputs` - Pass `Some(&[true, false])` for the roofing line, `Some(&[false, true])`
///   for the highpass line, `Some(&[true, true])` for both, or `None` for neither.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `in_phase`, `outputs[i][1]` is
/// `quadrature`, `outputs[i][2]` is `roofing` (empty unless requested), and
/// `outputs[i][3]` is `highpass` (empty unless requested) for option set `i`.
/// `states[i]` is the final [`IndicatorState`] for that lane.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, HilbertTransform::min_data)?;
    validate_options(options, None)?;

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut want_optional_outputs = (false, false, false);
    for i in 0..N {
        let (p_line, q_line, (mut rf_line, mut hp_line)) = {
            let len = inputs[0].len();
            let capacity = HilbertTransform::output_length(len, options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    rf_line: RoofingFilter::output_length(len, options[i]),
                    hp_line: HighPass::output_length(len, &[options[i][1]])
                ),
            )
        };
        let periods = (options[i][0] as usize, options[i][1] as usize);
        let state = State::init_state(inputs[0], periods, (&mut rf_line, &mut hp_line));
        let asset_inputs = vec![inputs[0]];
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
