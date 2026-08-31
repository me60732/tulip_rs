//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicator_types::TSimdState;
use crate::indicators::ad::{Ad, Indicator};
use crate::indicators::adosc::{
    Adosc, IndicatorState, State, INPUTS,
    OPTIONS, validate_options
};
use crate::indicators::ema::Ema;
use crate::indicators::simd_indicators::adosc_simd::{SimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver that advances the Chaikin AD Oscillator (ADOSC) across `N` asset lanes per
/// scheduling epoch.
struct AdoscDriver {
    /// Optional output flags: `(has_optional, want_short_ema, want_long_ema, want_ad)`.
    want_optional_outputs: (bool, bool, bool, bool),
}

impl Driver<State<Warm>, ()> for AdoscDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][field]` (high, low, close, volume), writes to
    /// `outputs[asset][output]`, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::from_states(&mut states);
        let len = inputs[0][0].len();

        let (has_optional, want_short_ema, want_long_ema, want_ad) = self.want_optional_outputs;
        // Optimization 1: Direct array construction instead of collect+try_into

        //collect outputs
        let (adosc_line_ptr, short_ema_line_ptr, long_ema_line_ptr, ad_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            adosc_line_ptr,
            short_ema_line_ptr,
            long_ema_line_ptr,
            ad_line_ptr
        );

        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs, close_ptrs, volume_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs, volume_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let inputs = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs,
                volume @ volume_ptrs
            );

            let adosc = state.calc(inputs);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                adosc_line_ptr => adosc
            );

            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_short_ema, short_ema_line_ptr => state.short_ema.ema,
                    want_long_ema, long_ema_line_ptr => state.long_ema.ema,
                    want_ad, ad_line_ptr => state.ad.ad
                );
            }
            
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Chaikin AD Oscillator (ADOSC) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// All assets share the same `options`. Uses the [`PrimeMover`] scheduler to batch assets into
/// SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[high, low, close, volume]` for asset `i`.
/// * `options` - Shared options applied to all `N` assets: `[short_period, long_period]`.
/// * `optional_outputs` - Optional output flags: `[want_short_ema, want_long_ema, want_ad]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[adosc, short_ema?, long_ema?, ad?]`
/// for asset `i` and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Adosc::min_data(options))?;
    validate_options(options)?;
    let short_period = options[0] as usize;
    let long_period = options[1] as usize;

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut want_optional_outputs = (false, false, false, false);
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // high
            inputs[i][1], // low
            inputs[i][2], // close
            inputs[i][3], // volume
        ];

        let adosc_capacity = Adosc::output_length(inputs[i][0].len(), options);
        let adosc_line = crate::uninit_vec!(f64, adosc_capacity);

        let (mut short_ema_line, long_ema_line, mut ad_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false, false],
            short_ema_line: Ema::output_length(inputs[i][0].len(), &[short_period as f64]),
            long_ema_line: adosc_capacity,
            ad_line: Ad::output_length(inputs[i][0].len(), &[])
        );

        let state = State::init_state(
            inputs[i],
            (short_period, long_period),
            (&mut short_ema_line, &mut ad_line),
        );

        let mut starts = [0; 4];
        (starts[1], starts[3]) =
            crate::slice_outputs_start!(adosc_capacity, short_ema_line, ad_line);
        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(short_ema_line, long_ema_line, ad_line);
        }

        let mut output_buffer = vec![adosc_line, short_ema_line, long_ema_line, ad_line];

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

    let mut driver = AdoscDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    Ok((output_buffers, states_vec))
}
