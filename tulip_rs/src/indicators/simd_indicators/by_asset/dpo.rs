//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::dpo::{
    min_data, output_length, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::{by_asset::sma::init_state, dpo_simd::calc_simd};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver that advances the Detrended Price Oscillator (DPO) across `N` asset lanes
/// per scheduling epoch.
struct DpoDriver {
    multiplier: f64,
    period: usize,
    dpo_period: usize,
    want_sma: bool,
}

impl Driver<f64> for DpoDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][0]` (real), writes the DPO to `outputs[asset][0]`,
    /// optional SMA to `outputs[asset][1]`, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut f64>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let want_sma = self.want_sma;
        // Optimization 1: Direct array construction instead of collect+try_into
        let mut sums = Simd::<f64, N>::from_array(std::array::from_fn(|i| unsafe {
            **states.get_unchecked(i)
        }));

        let multiplier_simd = Simd::splat(self.multiplier);

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs: [*const f64; N] =
            std::array::from_fn(|j| unsafe { inputs.get_unchecked(j).get_unchecked(0).as_ptr() });

        let (dpo_line_ptrs, sma_line_ptrs) =
            crate::extract_output_ptrs!(outputs, N, dpo_ptrs, sma_ptrs);
        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.period..len).enumerate() {
            let (old_vals, dpo_vals, new_vals) = crate::extract_simd_at_indices!(N, input_ptrs,
                old_vals @ j,
                dpo_vals @ i - self.dpo_period,
                new_vals @ i
            );

            let (dpo, sma) = calc_simd(new_vals, &mut sums, (old_vals, dpo_vals), multiplier_simd);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                dpo_line_ptrs => dpo
            );
            crate::store_simd_optional_outputs!(j, N,
                want_sma, sma_line_ptrs => sma
            );
        }

        // Update states efficiently
        let final_sums = sums.to_array();
        for (i, state) in states.iter_mut().enumerate().take(N) {
            **state = final_sums[i];
        }
    }
}

/// Calculates the Detrended Price Oscillator (DPO) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[real]` for asset `i`.
/// * `options` - Shared options slice; `options[0]` is the period.
/// * `optional_outputs` - Optional slice selecting extra outputs: index `0` = `sma`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the DPO line for asset `i`,
/// `outputs[i][1]` is the optional SMA, and `states[i]` is the final [`IndicatorState`]
/// for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let dpo_period = period / 2 + 1;
    //let real: Vec<&[f64]> = (0..N).map(|i| inputs[i][0]).collect();
    let real: [&[f64]; N] = std::array::from_fn(|i| inputs[i][0]);
    //init ema, sliced inputs and multipliers
    let (sums, multiplier) = init_state(&real, period);

    let mut road_train = PrimeMover::<N, f64>::new();

    let mut want_sma = false;
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![inputs[i][0]];
        let (dpo_line, sma_line) = {
            let len = inputs[i][0].len();
            let capacity = output_length(len, options);

            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };
        if i == 0 {
            (_, want_sma) = crate::calc_want_flags!(sma_line);
        }
        let mut output_buffer = vec![dpo_line, sma_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(0), //slice from
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
            sums[i],
            None,
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = DpoDriver {
        multiplier,
        period,
        dpo_period,
        want_sma,
    };
    let sums = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, &sum) in sums.iter().enumerate() {
        states.push(IndicatorState::new(
            unsafe { inputs.get_unchecked(i).get_unchecked(0) },
            sum,
            multiplier,
            period,
            dpo_period,
        ));
    }
    Ok((output_buffers, states))
}
