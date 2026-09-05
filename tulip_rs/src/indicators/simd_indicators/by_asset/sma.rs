//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::sma::{
    Sma, Indicator, State,INPUTS, OPTIONS, IndicatorState
};
use crate::types::{IndicatorError, Warm};
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::sma_simd::{SimdState, TSimdState, TState};

/// SIMD driver for Simple Moving Average (SMA) across `N` asset lanes per epoch.
/// Holds shared parameters used by [`Driver::next_run`] for each scheduled run.
struct SmaDriver {
    /// The look-back window length (number of bars summed per average).
    period: usize,
}

impl Driver<State<Warm>> for SmaDriver {
    /// Processes `bar_count` bars for `N` assets simultaneously using SIMD.
    /// Reads `inputs[asset][0]` (real prices), writes `outputs[asset][0]` (SMA line),
    /// and updates `states[asset]` with the rolling window sum for subsequent epochs.
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
        let output_ptrs = crate::extract_output_ptrs!(outputs, N, output_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.period..len).enumerate() {
            let inputs = crate::extract_simd_at_indices!(N, input_ptrs,
                new_vals @ i,
                old_vals @ j
            );

            let sma = state.calc(inputs);

            crate::write_simd_at_indices!(N, j,
                output_ptrs => sma
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Simple Moving Average (SMA) for `N` assets simultaneously using SIMD.
///
/// All assets share the same `options` (period). Warms up each asset's rolling sum via
/// [`init_state`], then dispatches to [`SmaDriver::next_run`] through the `PrimeMover` scheduler.
///
/// # Arguments
///
/// * `inputs`           — `N` asset input sets; `inputs[i][0]` is the real-price slice for asset `i`.
/// * `options`          — Shared parameter array: `options[0]` = period.
/// * `_optional_outputs`— Unused; SMA has no optional output lines.
///
/// # Returns
///
/// `Ok((outputs, states))` where:
/// * `outputs[i][0]` — the SMA line for asset `i`.
/// * `states[i]`     — the [`IndicatorState`] (rolling sum + multiplier) for resuming computation.
///
/// # Errors
///
/// Returns [`IndicatorError`] if inputs are too short or options are invalid.
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Sma::min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // real
        ];

        let sma_line = crate::uninit_vec!(f64, Sma::output_length(inputs[i][0].len(), options));

        let state = State::init_state(inputs[i][0], period);

        let mut output_buffer = vec![sma_line];

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

    let mut driver = SmaDriver {
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

