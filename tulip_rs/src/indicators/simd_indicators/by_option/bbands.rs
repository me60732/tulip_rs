//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::bbands::{
    validate_options as vo, BBands, Indicator, IndicatorState, State, INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::bbands_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver for the Bollinger Bands (BBANDS) indicator, processing `N` option-set lanes per scheduling epoch.
struct BbandsDriver {}

impl Driver<State<Warm>, usize> for BbandsDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();

        let mut i = [0usize; N];
        for (lane, option) in options.iter().enumerate() {
            if let Some(&period) = option {
                i[lane] = period;
            }
        }

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::from_states(&mut states);

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);

        let (lower_band_ptr, middle_band_ptr, upper_band_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            lower_band_ptr,
            middle_band_ptr,
            upper_band_ptr
        );

        // Optimization 3: Simplified main loop with pre-computed offsets
        for j in 0..len {
            let old_vals = crate::extract_simd_inputs_at_index!(j, N,
                old @ input_ptrs
            );
            let new_vals = crate::extract_simd_inputs_at_index_array!(i, N,
                new @ input_ptrs
            );

            let (lower_band, middle_band, upper_band) = state.calc((new_vals, old_vals));

            crate::write_simd_at_indices!(N, j,
                lower_band_ptr => lower_band,
                middle_band_ptr => middle_band,
                upper_band_ptr => upper_band
            );

            for i in i.iter_mut() {
                *i += 1;
            }
        }

        state.write_states(&mut states);
    }
}

/// Calculates Bollinger Bands (BBANDS) on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[real]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period, std_dev]`.
/// * `optional_outputs` - Unused; Bollinger Bands has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[bbands_lower, bbands_middle, bbands_upper]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, BBands::min_data)?;
    validate_options(options, Some(vo))?;
    let mut road_train = PrimeMover::<N, State<Warm>, usize>::new();

    let params: [(usize, f64); N] = std::array::from_fn(|i| {
        let period = options[i][0] as usize;
        (period, options[i][1])
    });

    let mut output_buffers = Vec::with_capacity(N);

    for (i, &(period, std_dev)) in params.iter().enumerate() {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let (middle_band, upper_band, lower_band) = {
            let capacity = BBands::output_length(inputs[0].len(), options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let state = State::init_state(inputs[0], period, std_dev);

        let mut output_buffer = vec![lower_band, middle_band, upper_band];

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

    let mut driver = BbandsDriver {};
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        let (period, _) = params[i];
        states.push(IndicatorState::new(inputs[0], state, period));
    }
    Ok((output_buffers, states))
}
