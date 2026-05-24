//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::volatility_simd::assets::SimdState;
use crate::indicators::volatility::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Volatility Indicator across `N` asset lanes per scheduling epoch.
struct VolatilityDriver {
    multiplier: f64,
}

impl Driver<State> for VolatilityDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::new(&mut states);
        let len = inputs[0][0].len();

        let multiplier = Simd::splat(self.multiplier);

        //collect outputs
        let volatility_line_ptr = crate::extract_output_ptrs!(outputs, N, volatility_line_ptr);

        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let real = crate::extract_simd_inputs_at_index!(
                i,
                N,
                real @ real_ptrs
            );

            let volatility = unsafe { state.calc_unchecked_simd(real, multiplier) };

            crate::write_simd_at_indices!(N, i,
                volatility_line_ptr => volatility
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Volatility Indicator for `N` assets simultaneously using SIMD parallelism.
///
/// This indicator produces no optional outputs. Uses the [`PrimeMover`] scheduler to batch
/// assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[real]` for asset `i`.
/// * `options` - `options[0]` is the `period`.
/// * `_optional_outputs` - Unused; this indicator has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the volatility line for asset `i` and
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let multiplier = multiplier(period);

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // real
        ];

        let volatility_line = {
            let capacity = output_length(inputs[i][0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::init_state(inputs[i][0], period);

        let mut output_buffer = vec![volatility_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(0), //slice from
                    output_buffer.len(),               // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period + 1,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = VolatilityDriver { multiplier };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state, multiplier));
    }
    Ok((output_buffers, states))
}
