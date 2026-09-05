//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

use crate::indicators::elderray::{
    Elderray, Indicator, IndicatorState, INPUTS, OPTIONS, State
};
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::elderray_simd::{TSimdState, TState, SimdState};

/// SIMD driver that advances Elder-ray across `N` asset lanes per scheduling epoch.
struct ElderRayDriver {
    want_optional_outputs: bool,
}

impl Driver<State<Warm>> for ElderRayDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// For each bar, computes `bull = high − EMA` and `bear = low − EMA` across all `N`
    /// asset lanes, writes bull and bear to their output buffers, optionally writes the
    /// updated EMA, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Direct array construction
        let mut state = SimdState::<N>::from_states(&mut states);

        // Pre-compute pointers for maximum efficiency
        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high, low, close);
        let (bull_line_ptr, bear_line_ptr, ema_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, bull, bear, ema);
        let want_ema = self.want_optional_outputs;
        // Optimized main loop with minimal overhead
        for i in 0..len {
            let inputs = crate::extract_simd_inputs_at_index!(i, N,
                h @ high_ptrs,
                l @ low_ptrs,
                c @ close_ptrs
            );

            let (bull, bear, emas) = state.calc(inputs);

            crate::write_simd_at_indices!(N, i,
                bull_line_ptr => bull,
                bear_line_ptr => bear
            );
            crate::store_simd_optional_outputs!(i, N,
                want_ema, ema_line_ptr => emas
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates Elder-ray for `N` assets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[high, low, close]` for asset `i`.
/// * `options` - Shared options slice; `options[0]` is the EMA period.
/// * `optional_outputs` - Pass `Some(&[true])` to also populate the EMA line for every asset.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` is `[bull, bear, ema]` for asset `i`
/// (the `ema` vec is empty unless `optional_outputs` enables it) and `states[i]`
/// is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Elderray::min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut want_optional_outputs = false;
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let [high, low, close] = *inputs[i];
        let asset_inputs = vec![high, low, close];
        let (bull_line, bear_line, ema_line) = {
            let capacity = Elderray::output_length(inputs[i][0].len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    ema_line: capacity
                ),
            )
        };
        let state = State::init_state(close, period);
        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(ema_line);
        }
        let mut output_buffer = vec![bull_line, bear_line, ema_line];
        let mut asset_outputs = Vec::with_capacity(output_buffers.len());

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
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = ElderRayDriver {
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
