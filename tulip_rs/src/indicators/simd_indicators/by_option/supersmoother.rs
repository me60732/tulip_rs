use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::supersmoother_simd::SimdState;
use crate::indicators::supersmoother::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver for the Ehlers Super Smoother indicator, processing `N` option-set lanes
/// per scheduling epoch using a shared input series.
struct SuperSmootherDriver;

impl Driver<State, (f64, f64, f64)> for SuperSmootherDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    ///
    /// Reads the shared real input, assembles per-lane coefficient vectors from `options`,
    /// advances the filter state, and writes one output bar per lane.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(f64, f64, f64)>>,
    ) {
        let len = outputs[0][0].len();

        let real_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let super_line = crate::extract_output_ptrs!(outputs, N, super_line);

        let multipliers_simd = {
            let mut multipliers = ([0.0; N], [0.0; N], [0.0; N]);
            for (lane, option) in options.iter().enumerate() {
                if let Some(&multiplier) = option {
                    multipliers.0[lane] = multiplier.0;
                    multipliers.1[lane] = multiplier.1;
                    multipliers.2[lane] = multiplier.2;
                }
            }
            (
                Simd::from_array(multipliers.0),
                Simd::from_array(multipliers.1),
                Simd::from_array(multipliers.2),
            )
        };

        let mut state = SimdState::new(&states);

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index_splat!(i, N,
                new @ real_ptrs
            );

            let super_smoother = state.calc_simd(real, multipliers_simd);

            crate::write_simd_at_indices!(N, i,
                super_line => super_smoother
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Ehlers Super Smoother on a single asset with `N` different option
/// sets simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[real]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `_optional_outputs` - Unused; SuperSmoother has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` contains the `supersmoother` line
/// for option set `i`, and `states[i]` is the final [`IndicatorState`] for that lane.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [(f64, f64, f64); N] = std::array::from_fn(|i| {
        let period = options[i][0] as usize;
        multiplier(period)
    });

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State, (f64, f64, f64)>::new();

    for i in 0..N {
        let super_line = {
            let capacity = output_length(inputs[0].len(), options[i]);
            crate::uninit_vec!(f64, capacity)
        };
        let period = options[i][0] as usize;
        let state = State::init_state(inputs[0], period, params[i]);
        let asset_inputs = vec![inputs[0]];

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
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = SuperSmootherDriver;
    let final_states = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in final_states.into_iter().enumerate() {
        states.push(IndicatorState::new(state, params[i]));
    }
    Ok((output_buffers, states))
}
