//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::smaenvelope::{
    SmaEnvelope, Indicator, State, validate_options, IndicatorState, INPUTS,
    OPTIONS,
};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::smaenvelope_simd::{SimdState, TSimdState, TState};

/// SIMD driver for the SMA Envelope indicator across `N` asset lanes per scheduling epoch.
/// Holds the shared parameters used by [`Driver::next_run`] for every scheduled run.
struct SmaEnvelopeDriver {
    /// The SMA look-back window length.
    period: usize,
}

impl Driver<State<Warm>> for SmaEnvelopeDriver {
    /// Processes one scheduling epoch of output bars for `N` assets simultaneously using SIMD.
    /// Reads `inputs[asset][0]` (real prices), writes `outputs[asset][0..3]`
    /// (lower, middle, upper bands), and updates `states[asset]` with the rolling window
    /// sum for subsequent epochs.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::<N>::from_states(&mut states);

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (lower_line_ptr, middle_line_ptr, upper_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, lower, middle, upper);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.period..len).enumerate() {
            let inputs = crate::extract_simd_at_indices!(N, input_ptrs,
                new_vals @ i,
                old_vals @ j
            );

            let (lower, middle, upper) = state.calc(inputs);

            crate::write_simd_at_indices!(N, j,
                lower_line_ptr => lower,
                middle_line_ptr => middle,
                upper_line_ptr => upper
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the SMA Envelope indicator for `N` assets simultaneously using SIMD parallelism.
///
/// All assets share the same `options` (period, percentage). Warms up each asset's rolling
/// sum via [`init_state`], then dispatches to [`SmaEnvelopeDriver::next_run`] through the
/// `PrimeMover` scheduler.
///
/// # Arguments
///
/// * `inputs`            — `N` asset input sets; `inputs[i][0]` is the real-price slice for asset `i`.
/// * `options`           — Shared parameter array: `options[0]` = period, `options[1]` = percentage.
/// * `_optional_outputs` — Unused; SMA Envelope has no optional output lines.
///
/// # Returns
///
/// `Ok((outputs, states))` where:
/// * `outputs[i][0]` — the lower band for asset `i`.
/// * `outputs[i][1]` — the middle band (SMA) for asset `i`.
/// * `outputs[i][2]` — the upper band for asset `i`.
/// * `states[i]`     — the [`IndicatorState`] (rolling sum + multipliers) for resuming computation.
///
/// # Errors
///
/// Returns [`IndicatorError`] if inputs are too short or options are invalid.
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, SmaEnvelope::min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let percentage = options[1];
    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // real
        ];

        let (lower_line, middle_line, upper_line) = {
            let len = inputs[i][0].len();
            let capacity = SmaEnvelope::output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let state = State::init_state(&inputs[i][0], period, percentage);

        let mut output_buffer = vec![lower_line, middle_line, upper_line];

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

    let mut driver = SmaEnvelopeDriver {
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
