//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;

use crate::indicators::fisher::{
    min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::fisher_simd::assets::SimdState;
use std::simd::Simd;
/// SIMD driver that advances the Fisher Transform (fisher) across `N` asset lanes
/// per scheduling epoch.
struct FisherDriver {
    period: usize,
}

impl Driver<State> for FisherDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][field]` (high, low), writes the Fisher Transform to
    /// `outputs[asset][0]`, the signal line to `outputs[asset][1]`, and updates
    /// `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        //collect outputs
        let (fisher_line_ptr, signal_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, fisher_line_ptr, signal_line_ptr);
        let (high_ptrs, low_ptrs) = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptr);
        let mut state = SimdState::new(&mut states);

        //let current: Vec<Simd<f64, N>> = crate::create_simd_vec_from_inputs!(real_ptrs, N, len);
        match self.period {
            1..=14 => {
                for i in 0..len {
                    let (high, low) = crate::extract_simd_inputs_at_index!(i, N,
                        high @ high_ptrs,
                        low @ low_ptrs
                    );
                    let (fisher, signal) = state.calc_simd::<1>(high, low, self.period);

                    // Store results using pre-computed pointers
                    crate::write_simd_at_indices!(N, i,
                        fisher_line_ptr => fisher,
                        signal_line_ptr => signal
                    );
                }
            }

            _ => {
                for i in 0..len {
                    let (high, low) = crate::extract_simd_inputs_at_index!(i, N,
                        high @ high_ptrs,
                        low @ low_ptrs
                    );
                    let (fisher, signal) = state.calc_simd::<8>(high, low, self.period);

                    // Store results using pre-computed pointers
                    crate::write_simd_at_indices!(N, i,
                        fisher_line_ptr => fisher,
                        signal_line_ptr => signal
                    );
                }
            }
        }
        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Fisher Transform (fisher) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low]` for asset `i`.
/// * `options` - Shared options slice; `options[0]` is the period.
/// * `_optional_outputs` - Unused; Fisher Transform has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the `fisher` line for asset `i`,
/// `outputs[i][1]` is the `fisher_signal` line, and `states[i]` is the final
/// [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // high
            inputs[i][1], // low
        ];

        let (mut fisher_line, mut signal_line) = {
            let len = inputs[i][0].len();
            let capacity = output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let state = State::init_state(
            inputs[i][0], // high
            inputs[i][1], // low
            period,
            &mut fisher_line,
            &mut signal_line,
        );

        let mut output_buffer = vec![fisher_line, signal_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(1), //slice from
                    output_buffer.len(),               // slice to
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

    let mut driver = FisherDriver { period };
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state, period));
    }
    Ok((output_buffers, states))
}
