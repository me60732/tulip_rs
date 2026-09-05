//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::smaenvelope::{
    validate_options as vo, Indicator, IndicatorState, SmaEnvelope, State, INPUTS, OPTIONS,
};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::smaenvelope_simd::{SimdState, TSimdState, TState};

/// SIMD driver for the SMA Envelope indicator, processing `N` option-set lanes per scheduling epoch.
struct SmaEnvelopeDriver;

impl Driver<State<Warm>, usize> for SmaEnvelopeDriver {
    /// Processes one scheduling epoch of output bars for `N` option-set lanes simultaneously
    /// using SIMD. Each lane may have a different period and percentage, so look-back offsets
    /// are tracked independently per lane.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&usize>>,
    ) {
        let output_len = outputs[0][0].len();

        //let mut period_arr = [0usize; N];

        let mut i = [0usize; N];
        for (lane, option) in options.iter().enumerate() {
            if let Some(&period) = option {
                i[lane] = period;
            }
        }

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::<N>::from_states(&mut states);

        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let (lower_line_ptr, middle_line_ptr, upper_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, lower, middle, upper);
        //let mut j = 0;
        // Optimization 3: Simplified main loop with pre-computed offsets

        for j in 0..output_len {
            let old_vals = crate::extract_simd_inputs_at_index!(j, N,
                old @ real_ptrs
            );
            let new_vals = crate::extract_simd_inputs_at_index_array!(i, N,
                new @ real_ptrs
            );

            let (lower, middle, upper) = state.calc((new_vals, old_vals));

            crate::write_simd_at_indices!(N, j,
                lower_line_ptr => lower,
                middle_line_ptr => middle,
                upper_line_ptr => upper
            );
            //i += UsizeConstants::ONE;
            for i in i.iter_mut() {
                *i += 1;
            }
        }

        state.write_states(&mut states);
    }
}

/// Calculates the SMA Envelope indicator for one asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// Applies each of the `N` (period, percentage) configurations to the same shared input
/// series, computing lower, middle, and upper envelope bands for all option sets in a
/// single SIMD-accelerated pass via [`PrimeMover`].
///
/// # Arguments
///
/// * `inputs`            — Shared input: `inputs[0]` is the real price series.
/// * `options`           — Array of `N` option sets; `options[i][0]` is the period and
///                         `options[i][1]` is the percentage for lane `i`.
/// * `_optional_outputs` — Unused; SMA Envelope has no optional outputs.
///
/// # Returns
///
/// `Ok((outputs, states))` where for each option-set lane `i`:
/// * `outputs[i][0]` — the lower band series.
/// * `outputs[i][1]` — the middle band (SMA) series.
/// * `outputs[i][2]` — the upper band series.
/// * `states[i]`     — the final [`IndicatorState`] for resuming streaming computation.
///
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, SmaEnvelope::min_data)?;
    validate_options(options, Some(vo))?;
    let params: [(usize, f64); N] =
        std::array::from_fn(|i| (options[i][0] as usize, options[i][1]));

    let mut road_train = PrimeMover::<N, State<Warm>, usize>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for (i, &(period, percentage)) in params.iter().enumerate() {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let (lower_line, middle_line, upper_line) = {
            let len = inputs[0].len();
            let capacity = SmaEnvelope::output_length(len, options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let state = State::init_state(inputs[0], period, percentage);

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
            Some(&params[i].0),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = SmaEnvelopeDriver {};
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(inputs[0], state, params[i].0));
    }
    Ok((output_buffers, states))
}
