//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
//use std::simd::cmp::SimdPartialOrd;
use std::simd::Simd;

use crate::indicators::dema::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::ema::output_length as ema_output_length;
use crate::indicators::simd_indicators::dema_simd::{calc_simd, SimdState};

/// SIMD driver that advances the Double Exponential Moving Average (DEMA) across `N` asset
/// lanes per scheduling epoch.
struct DemaDriver {
    /// EMA smoothing multiplier `2.0 / (period + 1)`.
    multiplier: f64,
    /// Complement smoothing factor `1.0 - multiplier`.
    inv_multiplier: f64,
    /// Whether to also emit the intermediate EMA output.
    want_optional_outputs: bool,
}

impl Driver<State> for DemaDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][0]` (real prices), writes to `outputs[asset][output]`,
    /// and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let mut state = SimdState::new_mut_ref(&states);

        let multipliers_simd = (
            Simd::splat(self.multiplier),
            Simd::splat(self.inv_multiplier),
        );

        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (dema_line_ptr, ema_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, dema_line_ptr, ema_line_ptr);

        // Optimized main loop with minimal overhead
        for i in 0..len {
            let values = crate::extract_simd_inputs_at_index!(i, N, values @ input_ptrs);

            let (dema, ema) = calc_simd(&mut state, values, multipliers_simd);

            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, i,
                dema_line_ptr => dema
            );
            crate::store_simd_optional_outputs!(i, N,
                self.want_optional_outputs, ema_line_ptr => ema
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Double Exponential Moving Average (DEMA) for `N` assets simultaneously
/// using SIMD parallelism.
///
/// DEMA reduces lag by computing `2 * EMA - EMA(EMA)`. All assets share the same `options`.
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing the real price series for asset `i`.
/// * `options` - Shared options applied to all `N` assets: `[period]`.
/// * `optional_outputs` - Optional output flags: `[want_ema]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[dema, ema?]`
/// for asset `i` and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let (multiplier, inv_multiplier) = multiplier(period);
    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, State>::new();
    let mut want_optional_outputs = false;
    for i in 0..N {
        let dema_capacity = output_length(inputs[i][0].len(), options);
        let dema_line = crate::uninit_vec!(f64, dema_capacity);
        let ema_capacity = ema_output_length(inputs[i][0].len(), options);
        let mut ema_line = crate::init_optional_outputs_eff!(
            optional_outputs, &[false],
            ema_line: ema_capacity
        );

        let state = State::init_state(inputs[i][0], dema_capacity, period, &mut ema_line);
        let asset_inputs = vec![inputs[i][0]];
        let mut starts = [0; 2];
        starts[1] = crate::slice_outputs_start!(dema_capacity, ema_line);

        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(ema_line);
        }
        let mut output_buffer = vec![dema_line, ema_line];
        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(starts[j]), //slice from
                    output_buffer.len(),                       // slice to
                ));
            }
        }
        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period * 2 - 2,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = DemaDriver {
        multiplier,
        inv_multiplier,
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec {
        states.push(IndicatorState::new(state, (multiplier, inv_multiplier)));
    }
    Ok((output_buffers, states))
}
