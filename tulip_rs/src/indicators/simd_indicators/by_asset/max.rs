//use crate::common::validate_inputs;
use crate::indicators::max::{
    Max, Indicator, IndicatorState, State, INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::max_simd::{assets::SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use crate::{common::validate_options, common_simd::assets::validate_inputs};

/// SIMD driver that advances the Maximum Value (max) across `N` asset lanes per scheduling
/// epoch.
struct MaxDriver {
    look_back: usize,
}

impl Driver<State<Warm>> for MaxDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][0]` (real), writes the rolling maximum to
    /// `outputs[asset][0]`, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        //collect outputs
        let max_line_ptr = crate::extract_output_ptrs!(outputs, N, max_line_ptr);
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let mut state = SimdState::<N>::from_states(&mut states);

        for (j, i) in (self.look_back..len).enumerate() {
            let (max, _) = state.calc((real_ptrs, i, self.look_back));

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                max_line_ptr => max
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Maximum Value (max) for `N` assets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[real]` for asset `i`.
/// * `options` - Shared options slice; `options[0]` is the period.
/// * `_optional_outputs` - Unused; max has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the rolling maximum for asset `i`
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Max::min_data(options))?;
    validate_options(options)?;
    let periods = (options[0] as usize, options[0] as usize - 1);

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // real
        ];

        let max_line = {
            let len = inputs[i][0].len();
            let capacity = Max::output_length(len, options);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::init_state(inputs[i][0], periods.1);

        let mut output_buffer = vec![max_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr(), //slice from
                    output_buffer.len(),        // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            periods.1,
            periods.1,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = MaxDriver { look_back: periods.1 };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(inputs[i][0], state, periods));
    }
    Ok((output_buffers, states))
}
