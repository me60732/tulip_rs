//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::cvi::{Cvi, Indicator, IndicatorState, State, INPUTS, OPTIONS};
use crate::indicators::simd_indicators::cvi_simd::{options::SimdState, TSimdState, TState};

use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};

/// SIMD driver for the Chaikin Volatility Indicator (CVI) indicator, processing `N` option-set lanes per scheduling epoch.
struct CviDriver {}

impl Driver<State<Warm>> for CviDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::from_states(&mut states);
        let len = outputs[0][0].len();

        //collect outputs
        let cvi_line_ptr = crate::extract_output_ptrs!(outputs, N, cvi_line_ptr);

        let (high_ptrs, low_ptrs) = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let inputs = unsafe { (*high_ptrs[0].add(i), *low_ptrs[0].add(i)) };

            let cvi = state.calc(inputs);

            crate::write_simd_at_indices!(N, i,
                cvi_line_ptr => cvi
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Chaikin Volatility Indicator (CVI) on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[high, low]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Unused; CVI has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[cvi]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Cvi::min_data)?;
    validate_options(options, None)?;

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let period = options[i][0] as usize;
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
        ];

        let cvi_line = {
            let capacity = Cvi::output_length(inputs[0].len(), options[i]);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::init_state(inputs, period);

        let mut output_buffer = vec![cvi_line];

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
            period * 2 - 1,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = CviDriver {};
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
