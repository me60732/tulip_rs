//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::dm::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::dm_simd::{calc_simd, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver that advances the Directional Movement (DM) across `N` asset lanes per
/// scheduling epoch.
struct DmDriver {
    /// Pre-computed Wilder smoothing multiplier for the given period.
    multiplier: f64,
}

impl Driver<State> for DmDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][field]` (high, low), writes to `outputs[asset][output]`,
    /// and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::new(&states);
        let len = inputs[0][0].len();
        let multipliers = Simd::splat(self.multiplier);

        //collect outputs
        let (plus_dm_line_ptr, minus_dm_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, plus_dm_line_ptr, minus_dm_line_ptr);

        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs) = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let (high, low) = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs
            );

            let (plus_dm, minus_dm) = calc_simd(&mut state, high, low, multipliers);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                plus_dm_line_ptr => plus_dm,
                minus_dm_line_ptr => minus_dm
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Directional Movement (DM) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// DM computes the smoothed Plus and Minus Directional Movement (+DM and -DM) over a rolling
/// period. All assets share the same `options`. Uses the [`PrimeMover`] scheduler to batch
/// assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low]` for asset `i`.
/// * `options` - Shared options applied to all `N` assets: `[period]`.
/// * `_optional_outputs` - Unused; DM has no optional output lines.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[plus_dm, minus_dm]`
/// for asset `i` and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.
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
            inputs[i][0], // high
            inputs[i][1], // low
        ];

        let (plus_dm_line, minus_dm_line) = {
            let capacity: usize = output_length(inputs[i][0].len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let state = State::init_state(inputs[i][0], inputs[i][1], period);

        let mut output_buffer = vec![plus_dm_line, minus_dm_line];

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
            period,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = DmDriver { multiplier };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state, multiplier));
    }
    Ok((output_buffers, states))
}
