use crate::common_simd::assets::validate_inputs;
use crate::indicators::apo::{
    validate_options, IndicatorState, INPUTS,
    OPTIONS, State, Apo, Indicator
};
use crate::indicators::ema::Ema;
use crate::indicators::simd_indicators::apo_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver that advances the Absolute Price Oscillator (APO) across `N` asset lanes per
/// scheduling epoch.
struct ApoDriver {
    /// Optional output flags: `(has_optional, want_short_ema, want_long_ema)`.
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State<Warm>> for ApoDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][field]` (price series), writes to `outputs[asset][output]`,
    /// and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::from_states(&mut states);
        let len = inputs[0][0].len();
        let (has_optional, want_short_ema, want_long_ema) = self.want_optional_outputs;
        // Optimization 1: Direct array construction instead of collect+try_into

        //collect outputs
        let (apo_line_ptr, short_ema_line_ptr, long_ema_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            apo_line_ptr,
            short_ema_line_ptr,
            long_ema_line_ptr
        );

        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let real = crate::extract_simd_inputs_at_index!(i, N, real @ real_ptrs);

            let apo = state.calc(real);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                apo_line_ptr => apo
            );

            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_short_ema, short_ema_line_ptr => state.short_ema.ema,
                    want_long_ema, long_ema_line_ptr => state.long_ema.ema
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Absolute Price Oscillator (APO) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// All assets share the same `options`. Uses the [`PrimeMover`] scheduler to batch assets into
/// SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing the price series for asset `i`.
/// * `options` - Shared options applied to all `N` assets: `[short_period, long_period]`.
/// * `optional_outputs` - Optional output flags: `[want_short_ema, want_long_ema]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[apo, short_ema?, long_ema?]`
/// for asset `i` and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Apo::min_data(options))?;
    validate_options(options)?;
    let short_period = options[0] as usize;
    let long_period = options[1] as usize;

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut want_optional_outputs = (false, false, false);
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // real
        ];

        let apo_capacity = Apo::output_length(inputs[i][0].len(), options);
        let apo_line = crate::uninit_vec!(f64, apo_capacity);

        let (mut short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false],
            short_ema_line: Ema::output_length(inputs[i][0].len(), &[short_period as f64]),
            long_ema_line: apo_capacity
        );

        let state = State::init_state(inputs[i][0], short_period, long_period, &mut short_ema_line);

        let mut starts = [0; 3];
        starts[1] = crate::slice_outputs_start!(apo_capacity, short_ema_line);
        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(short_ema_line, long_ema_line);
        }

        let mut output_buffer = vec![apo_line, short_ema_line, long_ema_line];

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
            long_period - 1,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = ApoDriver {
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
