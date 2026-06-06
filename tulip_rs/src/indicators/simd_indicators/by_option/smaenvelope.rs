//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::smaenvelope::{
    init_state, min_data, multiplier, output_length, validate_options as vo, IndicatorState,
    INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::smaenvelope_simd::calc_simd;

/// SIMD driver for the SMA Envelope indicator, processing `N` option-set lanes per scheduling epoch.
struct SmaEnvelope {}

impl Driver<f64, (usize, (f64, f64))> for SmaEnvelope {
    /// Processes one scheduling epoch of output bars for `N` option-set lanes simultaneously
    /// using SIMD. Each lane may have a different period and percentage, so look-back offsets
    /// are tracked independently per lane.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut f64>,
        options: Vec<Option<&(usize, (f64, f64))>>,
    ) {
        let output_len = outputs[0][0].len();

        //let mut period_arr = [0usize; N];

        let (multiplier_simd, mut i) = {
            let mut i = [0usize; N];
            let mut multipliers = ([0.0; N], [0.0; N]);
            for (lane, option) in options.iter().enumerate() {
                if let Some(&(period, multiplier)) = option {
                    i[lane] = period;
                    multipliers.0[lane] = multiplier.0;
                    multipliers.1[lane] = multiplier.1;
                }
            }
            (
                (
                    Simd::from_array(multipliers.0),
                    Simd::from_array(multipliers.1),
                ),
                i,
            ) //Simd::from_array(i))
        };

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut sums = Simd::<f64, N>::from_array(std::array::from_fn(|i| unsafe {
            **states.get_unchecked(i)
        }));

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

            let (lower, middle, upper) = calc_simd(&mut sums, new_vals, old_vals, multiplier_simd);

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

        // Update states efficiently
        let final_sums = sums.to_array();
        for (i, state) in states.iter_mut().enumerate().take(N) {
            **state = final_sums[i];
        }
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
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, Some(vo))?;
    let params: [(usize, (f64, f64)); N] = std::array::from_fn(|i| {
        (
            options[i][0] as usize,
            multiplier(options[i][0] as usize, options[i][1]),
        )
    });

    let mut road_train = PrimeMover::<N, f64, (usize, (f64, f64))>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for (i, &(period, _)) in params.iter().enumerate() {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let (lower_line, middle_line, upper_line) = {
            let len = inputs[0].len();
            let capacity = output_length(len, options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let sum = init_state(inputs[0], period);

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
                    output_buffer.len(),               // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period,
            period,
            sum,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = SmaEnvelope {};
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, sum) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            inputs[0],
            sum,
            params[i].0,
            params[i].1,
        ));
    }
    Ok((output_buffers, states))
}
