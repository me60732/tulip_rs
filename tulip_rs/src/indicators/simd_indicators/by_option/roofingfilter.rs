use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::roofingfilter_simd::{SimdState, TSimdState, TState};
use crate::indicators::{
    highpass::HighPass,
    roofingfilter::{Indicator, IndicatorState, RoofingFilter, State, INPUTS, OPTIONS},
};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver for the Ehlers Roofing Filter indicator, processing `N` option-set lanes
/// per scheduling epoch using a shared input series.
struct RoofingDriver {
    want_optional_outputs: bool,
}

impl Driver<State> for RoofingDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    ///
    /// Reads the shared real input, assembles per-lane coefficient vectors from `options`,
    /// advances the RoofingFilter state, and writes the roofing output (and optional highpass)
    /// for each lane.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = outputs[0][0].len();

        let real_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (rf_line, hp_line) = crate::extract_output_ptrs!(outputs, N, rf, hp);

        let mut state = SimdState::<N>::from_states(&mut states);
        let want_hp = self.want_optional_outputs;

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index_splat!(i, N,
                new @ real_ptrs
            );

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

/// Calculates the Ehlers Roofing Filter on a single asset with `N` different option
/// sets simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[real]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[ss_period, hp_period]`.
/// * `optional_outputs` - Pass `Some(&[true])` to also emit the `highpass` line per lane.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` contains the `roofing` line
/// and `outputs[i][1]` contains the optional `highpass` line (empty if not requested)
/// for option set `i`, and `states[i]` is the final [`IndicatorState`] for that lane.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, RoofingFilter::min_data)?;
    validate_options(options, None)?;

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State>::new();
    let mut want_optional_outputs = false;
    for i in 0..N {
        let (rf_line, mut hp_line) = {
            let len = inputs[0].len();
            let capacity = RoofingFilter::output_length(len, options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    hp_line: HighPass::output_length(len, &[options[i][1]])
                ),
            )
        };
        let periods = (options[i][0] as usize, options[i][1] as usize);
        let state = State::init_state(inputs[0], periods, &mut hp_line);
        let asset_inputs = vec![inputs[0]];
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

    let mut driver = RoofingDriver {
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
