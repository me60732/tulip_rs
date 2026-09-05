//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::sma::{Indicator, IndicatorState, Sma, State, INPUTS, OPTIONS};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::sma_simd::{SimdState, TSimdState, TState};

/// SIMD driver for the Simple Moving Average (SMA) indicator, processing `N` option-set lanes per scheduling epoch.
struct SmaDriver {}

impl Driver<State<Warm>, usize> for SmaDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&usize>>,
    ) {
        let output_len = outputs[0][0].len();

        //let mut period_arr = [0usize; N];

        let mut i = {
            let mut i = [0usize; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&period) = option {
                    i[lane] = period;
                }
            }
            i
        };

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::<N>::from_states(&mut states);

        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let sma_line_ptr = crate::extract_output_ptrs!(outputs, N, sma_line_ptr);
        //let mut j = 0;
        // Optimization 3: Simplified main loop with pre-computed offsets

        for j in 0..output_len {
            let old_vals = crate::extract_simd_inputs_at_index!(j, N,
                old @ real_ptrs
            );
            let new_vals = crate::extract_simd_inputs_at_index_array!(i, N,
                new @ real_ptrs
            );

            let sma = state.calc((new_vals, old_vals));

            crate::write_simd_at_indices!(N, j,
                sma_line_ptr => sma
            );
            //i += UsizeConstants::ONE;
            for i in i.iter_mut() {
                *i += 1;
            }
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Simple Moving Average (SMA) indicator for one asset with `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// Applies each of the `N` period configurations to the same shared input series, computing
/// SMA values for all option sets in a single SIMD-accelerated pass via [`PrimeMover`].
///
/// # Arguments
/// * `inputs` - Shared input: `inputs[0]` is the `real` price series.
/// * `options` - An array of `N` option sets; `options[i][0]` is the `period` for lane `i`.
/// * `_optional_outputs` - Unused; SMA has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the `sma` series for option set `i`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Sma::min_data)?;
    validate_options(options, None)?;
    let params: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);

    let mut road_train = PrimeMover::<N, State<Warm>, usize>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for (i, &period) in params.iter().enumerate() {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let sma_line = {
            let len = inputs[0].len();
            let capacity = Sma::output_length(len, options[i]);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::init_state(inputs[0], period);

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
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = SmaDriver {};
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(inputs[0], state, params[i]));
    }
    Ok((output_buffers, states))
}
