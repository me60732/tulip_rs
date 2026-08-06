//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::bbands::{
    BBands, Indicator, validate_options, IndicatorState, State, INPUTS, OPTIONS,
};

use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::bbands_simd::{SimdState, TSimdState, TState};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver that advances the Bollinger Bands (BBANDS) across `N` asset lanes per scheduling
/// epoch.
struct BbandsDriver {
    period: usize,
}

impl Driver<State<Warm>> for BbandsDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][0]` (real prices), writes `[lower_band, middle_band, upper_band]`
    /// to `outputs[asset]`, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::from_states(&mut states);

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let (lower_band_ptr, middle_band_ptr, upper_band_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            lower_band_ptr,
            middle_band_ptr,
            upper_band_ptr
        );

        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.period..len).enumerate() {
            // Get new and old values using pre-computed pointers
            let inputs = crate::extract_simd_at_indices!(N, input_ptrs,
                new_vals @ i,
                old_vals @ j
            );

            let (lower_band, middle_band, upper_band) =
                state.calc(inputs);

            crate::write_simd_at_indices!(N, j,
                lower_band_ptr => lower_band,
                middle_band_ptr => middle_band,
                upper_band_ptr => upper_band
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Bollinger Bands (BBANDS) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// Bollinger Bands consist of a middle SMA band and upper/lower bands placed a configurable
/// number of standard deviations away. All assets share the same `options`. Uses the
/// [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing the real price series for asset `i`.
/// * `options` - Shared options applied to all `N` assets: `[period, std_dev_multiplier]`.
/// * `_optional_outputs` - Unused; BBANDS has no optional output lines.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[lower_band, middle_band, upper_band]`
/// for asset `i` and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, BBands::min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let std_dev = options[1];
    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // real
        ];

        let (middle_band, upper_band, lower_band) = {
            let capacity = BBands::output_length(inputs[i][0].len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let state = State::init_state(inputs[i][0], period, std_dev);

        let mut output_buffer = vec![middle_band, upper_band, lower_band];

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
            period,
            period,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = BbandsDriver {
        period
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            unsafe { inputs.get_unchecked(i).get_unchecked(0) },
            state,
            period,
        ));
    }
    Ok((output_buffers, states))
}

