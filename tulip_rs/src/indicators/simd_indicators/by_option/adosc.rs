//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

use crate::indicators::ad::output_length as ad_output_length;
use crate::indicators::adosc::{
    min_data, multiplier, output_length, validate_options as vo, IndicatorState, State,
    INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::ema::output_length as ema_output_length;
use crate::indicators::simd_indicators::adosc_simd::{calc_simd, SimdState};

/// SIMD driver that advances the Chaikin AD Oscillator (ADOSC) across `N` option-set lanes
/// per scheduling epoch.
struct AdoscDriver {
    /// Optional output flags: `(has_optional, want_short_ema, want_long_ema, want_ad)`.
    want_optional_outputs: (bool, bool, bool, bool),
}

impl Driver<State, ((f64, f64), (f64, f64))> for AdoscDriver {
    /// Processes one epoch of bars for `N` option lanes simultaneously using SIMD.
    ///
    /// Reads from `inputs[field]` (shared single asset's high, low, close, volume), writes to
    /// `outputs[lane][output]`, and updates `states[lane]` in place.
    /// Per-lane EMA multipliers are loaded from `options[lane]` at the start of each epoch.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&((f64, f64), (f64, f64))>>,
    ) {
        let mut state = SimdState::<N>::new(&states);
        let len = outputs[0][0].len();
        let mut multipliers = (([0.0; N], [0.0; N]), ([0.0; N], [0.0; N]));

        for (lane, option) in options.iter().enumerate() {
            if let Some(&multiplier) = option {
                (
                    (multipliers.0 .0[lane], multipliers.0 .1[lane]),
                    (multipliers.1 .0[lane], multipliers.1 .1[lane]),
                ) = multiplier;
            }
        }
        let multipliers = (
            (
                Simd::from_array(multipliers.0 .0),
                Simd::from_array(multipliers.0 .1),
            ),
            (
                Simd::from_array(multipliers.1 .0),
                Simd::from_array(multipliers.1 .1),
            ),
        );
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
            let (high, low, close, volume) = crate::extract_simd_inputs_at_index_splat!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs,
                volume @ volume_ptrs
            );

            let adosc = calc_simd(&mut state, (high, low, close, volume), multipliers);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                adosc_line_ptr => adosc
            );

            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_short_ema, short_ema_line_ptr => state.short_ema,
                    want_long_ema, long_ema_line_ptr => state.long_ema,
                    want_ad, ad_line_ptr => state.ad
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Chaikin AD Oscillator (ADOSC) on a single asset with `N` different option
/// sets simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[high, low, close, volume]`.
/// * `options` - An array of `N` option sets, one per SIMD lane:
///   `[short_period, long_period]`.
/// * `optional_outputs` - Optional output flags:
///   `[want_short_ema, want_long_ema, want_ad]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[adosc, short_ema?, long_ema?, ad?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, Some(vo))?;

    let mut multipliers = [((0.0, 0.0), (0.0, 0.0)); N];
    let mut road_train = PrimeMover::<N, State, ((f64, f64), (f64, f64))>::new();
    let mut want_optional_outputs = (false, false, false, false);
    let mut output_buffers = Vec::with_capacity(N);
    // First pass: calculate all multipliers
    for i in 0..N {
        let short_period = options[i][0] as usize;
        let long_period = options[i][1] as usize;
        multipliers[i] = multiplier(short_period, long_period);
    }
    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
            inputs[2], // close
            inputs[3], // volume
        ];
        let short_period = options[i][0] as usize;
        let long_period = options[i][1] as usize;

        let adosc_capacity = output_length(inputs[0].len(), options[i]);
        let adosc_line = crate::uninit_vec!(f64, adosc_capacity);

        let (mut short_ema_line, long_ema_line, mut ad_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false, false],
            short_ema_line: ema_output_length(inputs[0].len(), &[short_period as f64]),
            long_ema_line: adosc_capacity,
            ad_line: ad_output_length(inputs[0].len(), options[i])
        );

        let state = State::init_state(
            inputs,
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
                    output_buffer.len(),                       // slice to
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
            Some(&multipliers[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = AdoscDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(state, multipliers[i]));
    }
    Ok((output_buffers, states))
}
