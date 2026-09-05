//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;
pub use crate::indicator_types::{TSimdState, TState, Indicator};
use crate::indicators::simd_indicators::trvi_simd::assets::SimdState;
use crate::indicators::{
    ema::Ema,
    tr::Tr,
    trvi::{Trvi, IndicatorState, State, INPUTS, OPTIONS},
};

/// SIMD driver that advances the True Range Volatility Indicator (TRVI) across `N` asset lanes
/// per scheduling epoch.
struct TrviDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State<Warm>> for TrviDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][field]` (high, low, close), writes TRVI values to
    /// `outputs[asset][0]`, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::from_states(&mut states);
        let len = inputs[0][0].len();
        
        //collect outputs
        let (trvi_line_ptr, tr_line_ptr, ema_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, trvi, tr, ema);

        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high, low, close);
        let (has_optional, want_tr, want_ema) = self.want_optional_outputs;
        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let inputs = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs
            );

            let (trvi, tr, ema) = state.calc(inputs);

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
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
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
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Trvi::min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = (false, false, false);
    for i in 0..N {
        let [high, low, close] = *inputs[i];
        let asset_inputs = vec![high, low, close];

        let (trvi_line, (mut tr_line, mut ema_line)) = {
            let capacity = Trvi::output_length(high.len(), options);
            let tr_capacity = Tr::output_length(high.len(), &[]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    tr_line: tr_capacity,
                    ema_line: Ema::output_length(tr_capacity, options)
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
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
