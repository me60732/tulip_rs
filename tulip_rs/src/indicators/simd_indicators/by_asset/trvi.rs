//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

use crate::indicators::simd_indicators::trvi_simd::assets::SimdState;
use crate::indicators::{
    ema::output_length as ema_output_length,
    tr::output_length as tr_output_length,
    trvi::{
        min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
    },
};

/// SIMD driver that advances the True Range Volatility Indicator (TRVI) across `N` asset lanes
/// per scheduling epoch.
struct TrviDriver {
    /// Pre-computed EMA smoothing factors `(multiplier, inv_multiplier)` for the given period.
    multiplier: (f64, f64),
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State> for TrviDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][field]` (high, low, close), writes TRVI values to
    /// `outputs[asset][0]`, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::new(&mut states);
        let len = inputs[0][0].len();

        let multiplier = (
            Simd::splat(self.multiplier.0),
            Simd::splat(self.multiplier.1),
        );

        //collect outputs
        let (trvi_line_ptr, tr_line_ptr, ema_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, trvi, tr, ema);

        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high, low, close);
        let (has_optional, want_tr, want_ema) = self.want_optional_outputs;
        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let (high, low, close) = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs
            );

            let (trvi, tr, ema) =
                unsafe { state.calc_unchecked_simd(high, low, close, multiplier) };

            crate::write_simd_at_indices!(N, i,
                trvi_line_ptr => trvi
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_tr, tr_line_ptr => tr,
                    want_ema, ema_line_ptr => ema
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the True Range Volatility Indicator (TRVI) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// TRVI is structurally identical to the Chaikin Volatility Indicator (CVI) but uses True Range
/// instead of the simple high-low spread, making it more sensitive to overnight gaps and
/// unusually large bars. All assets share the same `options`. Uses the [`PrimeMover`] scheduler
/// to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close]` for asset `i`.
/// * `options` - Shared options applied to all `N` assets: `[period]`.
/// * `optional_outputs` - Pass `Some(&[true, true])` to also emit the `tr` and `ema`
///   intermediate series for every asset; `None` disables all optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the TRVI series for asset `i`,
/// `outputs[i][1]` is `tr` (optional), `outputs[i][2]` is the EMA of TR (optional),
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let multiplier = multiplier(period);

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = (false, false, false);
    for i in 0..N {
        let [high, low, close] = *inputs[i];
        let asset_inputs = vec![high, low, close];

        let (trvi_line, (mut tr_line, mut ema_line)) = {
            let capacity = output_length(high.len(), options);
            let tr_capacity = tr_output_length(high.len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    tr_line: tr_capacity,
                    ema_line: ema_output_length(tr_capacity, options)
                ),
            )
        };

        let state = State::init_state(inputs[i], period, &mut tr_line, &mut ema_line);
        let mut starts = [0; 3];
        (starts[1], starts[2]) = crate::slice_outputs_start!(trvi_line.len(), tr_line, ema_line);

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(tr_line, ema_line);
        }
        let mut output_buffer = vec![trvi_line, tr_line, ema_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(starts[j]), //slice from
                    output_buffer.len() - starts[j],           // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period * 2 - 1,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = TrviDriver {
        multiplier,
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state, multiplier));
    }
    Ok((output_buffers, states))
}
