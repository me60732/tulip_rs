//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::hma::{Hma, Indicator, IndicatorState, State, INPUTS, OPTIONS};
use crate::indicators::simd_indicators::hma_simd::{options::SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver for the Hull Moving Average (HMA) indicator, processing `N` option-set lanes per scheduling epoch.
struct HmaDriver;

impl Driver<State<Warm>, (usize, usize)> for HmaDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&(usize, usize)>>,
    ) {
        let mut state = SimdState::<N>::from_states(&mut states);
        let len = outputs[0][0].len();
        //multipliers: ,
        let mut i = [0usize; N];
        let mut i2 = [0usize; N];

        for (lane, &option) in options.iter().enumerate() {
            if let Some(&(period, period2)) = option {
                i[lane] = period;
                i2[lane] = period - period2;
            }
        }

        //collect outputs
        let hma_line_ptr = crate::extract_output_ptrs!(outputs, N, hma_line_ptr);

        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        // Optimization 3: Simplified main loop with pre-computed offsets
        for j in 0..len {
            // Get inputs arrays for stocks
            let prev_real = crate::extract_simd_inputs_at_index!(j, N,
                prev_real @ real_ptrs
            );
            let (real, prev_real2) = crate::extract_simd_at_indices_array!(N, real_ptrs,
                real @ i,
                prev_real2 @ i2
            );

            let hma = state.calc((real, prev_real, prev_real2));
            //unsafe { calc_simd(&mut state, high, low, close, multiplier) };
            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                hma_line_ptr => hma
            );

            for (i, i2) in i.iter_mut().zip(i2.iter_mut()) {
                *i += 1;
                *i2 += 1;
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Hull Moving Average (HMA) on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[real]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Unused; HMA has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[hma]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Hma::min_data)?;
    validate_options(options, None)?;
    let params: [(usize, usize); N] = std::array::from_fn(|i| {
        let period = options[i][0] as usize;
        (period, period / 2)
    });
    let mut road_train = PrimeMover::<N, State<Warm>, (usize, usize)>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let period = options[i][0] as usize;
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let hma_line = {
            let capacity = Hma::output_length(inputs[0].len(), options[i]);
            crate::uninit_vec!(f64, capacity)
        };

        let (start, state) = State::init_state(inputs[0], period);

        let mut output_buffer = vec![hma_line];

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
            start,
            period,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = HmaDriver {};
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, (period, period2)) in states_vec.into_iter().zip(params.into_iter()) {
        states.push(IndicatorState::new(inputs[0], state, period, period2));
    }
    Ok((output_buffers, states))
}
