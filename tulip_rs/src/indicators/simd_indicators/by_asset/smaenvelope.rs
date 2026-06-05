//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::smaenvelope::{
    min_data, multiplier, output_length, validate_options, IndicatorState, INPUTS_WIDTH,
    OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::smaenvelope_simd::calc_simd;

/// SIMD driver for the SMA Envelope indicator across `N` asset lanes per scheduling epoch.
/// Holds the shared parameters used by [`Driver::next_run`] for every scheduled run.
struct SmaEnvelopeDriver {
    /// Precomputed `(1/period, percentage/100)` tuple, broadcast to all SIMD lanes.
    multipliers: (f64, f64),
    /// The SMA look-back window length.
    period: usize,
}

impl Driver<f64> for SmaEnvelopeDriver {
    /// Processes one scheduling epoch of output bars for `N` assets simultaneously using SIMD.
    /// Reads `inputs[asset][0]` (real prices), writes `outputs[asset][0..3]`
    /// (lower, middle, upper bands), and updates `states[asset]` with the rolling window
    /// sum for subsequent epochs.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut f64>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut sums = Simd::<f64, N>::from_array(std::array::from_fn(|i| unsafe {
            **states.get_unchecked(i)
        }));

        let multiplier_simd = (
            Simd::splat(self.multipliers.0),
            Simd::splat(self.multipliers.1),
        );

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (lower_line_ptr, middle_line_ptr, upper_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, lower, middle, ipper);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.period..len).enumerate() {
            let (old_vals, new_vals) = crate::extract_simd_at_indices!(N, input_ptrs,
                old_vals @ j,
                new_vals @ i
            );

            let (lower, middle, upper) = calc_simd(&mut sums, new_vals, old_vals, multiplier_simd);

            crate::write_simd_at_indices!(N, j,
                lower_line_ptr => lower,
                middle_line_ptr => middle,
                upper_line_ptr => upper
            );
        }

        // Update states efficiently
        let final_sums = sums.to_array();
        for (i, state) in states.iter_mut().enumerate().take(N) {
            **state = final_sums[i];
        }
    }
}

/// Warms up the SMA Envelope state by accumulating the first `period` bars of each of
/// the `N` input slices into a running sum, and precomputes the shared multipliers.
///
/// # Arguments
///
/// * `inputs` - `N` real-price slices, one per asset.
/// * `period` - The SMA look-back window length.
/// * `percentage` - The envelope width as a percentage of the SMA.
///
/// # Returns
///
/// `(sums, multipliers)` — a `Vec<f64>` of `N` initial window sums and the precomputed
/// `(1/period, percentage/100)` tuple.
pub fn init_state<'a, const N: usize>(
    inputs: &[&'a [f64]; N],
    period: usize,
    percentage: f64,
) -> (Vec<f64>, (f64, f64)) {
    let multiplier = multiplier(period, percentage);
    let mut sums = Simd::<f64, N>::splat(0.0);

    // Optimization: Pre-compute input pointers for the initialization loop
    let input_ptrs: [*const f64; N] = std::array::from_fn(|i| inputs[i].as_ptr());

    for i in 0..period {
        let values = Simd::from_array(std::array::from_fn(|j| unsafe { *input_ptrs[j].add(i) }));
        sums += values;
    }

    (sums.to_array().to_vec(), multiplier)
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
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let percentage = options[1];
    //let real: Vec<&[f64]> = (0..N).map(|i| inputs[i][0]).collect();
    let real: [&[f64]; N] = std::array::from_fn(|i| inputs[i][0]);
    //init ema, sliced inputs and multipliers
    let (sums, multipliers) = init_state(&real, period, percentage);

    let mut road_train = PrimeMover::<N, f64>::new();
    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            let capacity = output_length(inputs[i][0].len(), options);
            vec![
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            ]
        })
        .collect();

    for i in 0..N {
        let asset_inputs = vec![inputs[i][0]];
        unsafe {
            let lower_buf = std::slice::from_raw_parts_mut(
                output_buffers[i][0].as_mut_ptr(),
                output_buffers[i][0].len(),
            );
            let middle_buf = std::slice::from_raw_parts_mut(
                output_buffers[i][1].as_mut_ptr(),
                output_buffers[i][1].len(),
            );
            let upper_buf = std::slice::from_raw_parts_mut(
                output_buffers[i][2].as_mut_ptr(),
                output_buffers[i][2].len(),
            );
            let asset_outputs = vec![lower_buf, middle_buf, upper_buf];

            road_train.add_asset(Asset::new(
                asset_inputs,
                asset_outputs,
                i,
                period,
                period,
                sums[i],
                None,
            ));
        }
    }
    let mut driver = SmaEnvelopeDriver {
        multipliers,
        period,
    };
    let sums = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, &sum) in sums.iter().enumerate() {
        states.push(IndicatorState::new(inputs[i][0], sum, period, multipliers));
    }
    Ok((output_buffers, states))
}
