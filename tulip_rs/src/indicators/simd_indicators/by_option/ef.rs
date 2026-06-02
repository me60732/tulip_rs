use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::ef::{
    min_data, output_length, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH, init
};
use crate::indicators::simd_indicators::ef_simd::calc_simd;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver for Kaufman's Adaptive Moving Average (KAMA) indicator, processing `N` option-set lanes per scheduling epoch.
struct EfDriver {}

impl Driver<f64, usize> for EfDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut f64>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();

        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let output_ptrs = crate::extract_output_ptrs!(outputs, N, output_ptrs);

        let (mut i, mut prev, mut old) = {
            let mut i = [0usize; N];
            let mut periods = [0usize; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&period) = option {
                    i[lane] = period + 1;
                    periods[lane] = period;
                }
            }
            (
                i,
                crate::extract_simd_inputs_at_index_array!(Simd::from_array(periods), N,
                    new @ input_ptrs
                ),
                crate::extract_simd_inputs_at_index!(0, N, real @ input_ptrs),
            )
        };
        // Direct array construction
        let mut sums = Simd::<f64, N>::from_array(std::array::from_fn(|i| unsafe {
            **states.get_unchecked(i)
        }));

        // Optimized main loop with minimal overhead
        for j in 0..len {
            let value = crate::extract_simd_inputs_at_index_array!(i, N,
                new @ input_ptrs
            );
            let last = crate::extract_simd_inputs_at_index!(j+1, N, real @ input_ptrs);
            let ef = calc_simd(&mut sums, (value, prev, last, old));
            (prev, old) = (value, last);
            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, j,
                output_ptrs => ef
            );
            for i in i.iter_mut() {
                *i += 1;
            }
        }

        // Update states efficiently
        let final_sums = sums.as_array();
        for (i, state) in states.iter_mut().enumerate().take(N) {
            **state = final_sums[i];
        }
    }
}

/// Calculates Kaufman's Adaptive Moving Average (KAMA) on a single asset with `N` different option
/// sets simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[real]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Unused; KAMA has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[kama]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);
    // Create output buffers OUTSIDE the assets - these will be owned by this function
    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, f64, usize>::new();

    for i in 0..N {
        let len = inputs[0].len();
        let capacity = output_length(len, options[i]);
        let mut ef_line = crate::uninit_vec!(f64, capacity);
        let period = options[i][0] as usize;
        let sum = init(inputs[0], period, &mut ef_line);
        let asset_inputs = vec![inputs[0]];

        let mut output_buffer = vec![ef_line];
        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        unsafe {
            //let slice_len = output_buffer.len() - starts[j];
            // Get a mutable reference to the output buffer for this asset
            let output_buffer = &mut output_buffer[0];
            asset_outputs.push(std::slice::from_raw_parts_mut(
                output_buffer.as_mut_ptr().add(1), //slice from
                output_buffer.len(),               // slice to
            ));
        }
        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period + 1,
            period + 1,
            sum,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = EfDriver {};
    let final_states = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, sum) in final_states.into_iter().enumerate() {
        states.push(IndicatorState::new(inputs[0], sum, params[i]));
    }
    Ok((output_buffers, states))
}
