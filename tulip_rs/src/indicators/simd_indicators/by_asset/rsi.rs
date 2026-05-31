//use crate::common::validate_inputs;
use crate::indicators::rsi::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::rsi_simd::SimdState;
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Relative Strength Index (RSI) across `N` asset lanes per scheduling epoch.
struct RsiDriver {
    multipliers: (f64, f64),
}

impl Driver<State> for RsiDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        //let output_len = len - self.period;

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::new(&states);
        let multipliers_simd = (
            Simd::splat(self.multipliers.0),
            Simd::splat(self.multipliers.1),
        );
        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        let rsi_line_ptr = crate::extract_output_ptrs!(outputs, N, rsi_line_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get new and old values using pre-computed pointers
            let current = crate::extract_simd_inputs_at_index!(i, N,
                current @ real_ptrs
            );

            let rsi = state.calc_simd(current, multipliers_simd);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                rsi_line_ptr => rsi
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Relative Strength Index (RSI) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// RSI is a momentum oscillator that measures the speed and magnitude of price changes
/// on a scale from 0 to 100. Uses the [`PrimeMover`] scheduler to batch assets into
/// SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[real]` for asset `i`.
/// * `options` - `[period]` — the smoothing period for the RSI calculation.
/// * `_optional_outputs` - Unused; RSI produces no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the RSI line for asset `i`
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    //let real: Vec<&[f64]> = (0..N).map(|i| inputs[i][0]).collect();
    let real: [&[f64]; N] = std::array::from_fn(|i| inputs[i][0]);
    //init ema, sliced inputs and multipliers
    let simd_state = SimdState::init_state(&real, period);
    let states = simd_state.to_states();
    let multipliers = multiplier(period);
    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for (i, state) in states.into_iter().enumerate() {
        let asset_inputs = vec![inputs[i][0]];
        let rsi_line = {
            let capacity = output_length(inputs[i][0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };
        let mut output_buffer = vec![rsi_line];

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
    let mut driver = RsiDriver { multipliers };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state, multipliers));
    }
    Ok((output_buffers, states))
}
