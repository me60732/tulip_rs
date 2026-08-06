use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::kama::{Indicator, IndicatorState, Kama, State, INPUTS, OPTIONS};
use crate::indicators::simd_indicators::kama_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver for Kaufman's Adaptive Moving Average (KAMA) indicator, processing `N` option-set lanes per scheduling epoch.
struct KamaDriver {
    want_optional_outputs: bool,
}

impl Driver<State<Warm>, usize> for KamaDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();

        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (kama_line_ptr, ef_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, kama_line_ptr, ef_line_ptr);

        let mut i = [0usize; N];
        for (lane, option) in options.iter().enumerate() {
            if let Some(&period) = option {
                i[lane] = period;
            }
        }

        // Direct array construction
        let mut state = SimdState::from_states(&mut states);

        // Optimized main loop with minimal overhead
        for j in 0..len {
            let value = crate::extract_simd_inputs_at_index_array!(i, N,
                new @ input_ptrs
            );
            let last = crate::extract_simd_inputs_at_index!(j, N, real @ input_ptrs);
            let (kama, ef) = state.calc((value, last));

            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, j,
                kama_line_ptr => kama
            );
            crate::store_simd_optional_outputs!(j, N,
                self.want_optional_outputs, ef_line_ptr => ef
            );

            for i in i.iter_mut() {
                *i += 1;
            }
        }

        state.write_states(&mut states);
    }
}

/// Calculates Kaufman's Adaptive Moving Average (KAMA) on a single asset with `N` different option
/// sets simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[real]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Unused; KAMA has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[kama]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Kama::min_data)?;
    validate_options(options, None)?;
    let params: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);
    // Create output buffers OUTSIDE the assets - these will be owned by this function
    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, State<Warm>, usize>::new();
    let mut want_optional_outputs = false;

    for i in 0..N {
        let (mut kama_line, mut ef_line) = {
            let capacity = Kama::output_length(inputs[0].len(), options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs!(
                    optional_outputs, &[false],
                    ef_line: capacity
                ),
            )
        };
        let period = options[i][0] as usize;
        let state = State::init_state(inputs[0], period, &mut kama_line, &mut ef_line);
        let asset_inputs = vec![inputs[0]];
        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(ef_line);
        }
        let mut starts = [1; 2];
        starts[1] = if !want_optional_outputs { 0 } else { starts[1] };
        let mut output_buffer = vec![kama_line, ef_line];
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(starts[j]), //slice from
                    output_buffer.len() - starts[j],           // slice to
                ));
            }
        }
        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period + 1,
            period,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = KamaDriver {
        want_optional_outputs,
    };
    let final_states = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in final_states.into_iter().enumerate() {
        states.push(IndicatorState::new(inputs[0], params[i], state));
    }
    Ok((output_buffers, states))
}
