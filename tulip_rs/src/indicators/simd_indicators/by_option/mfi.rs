//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::mfi::{
    Mfi, Indicator, State, INPUTS, OPTIONS, IndicatorState
};
use crate::indicators::simd_indicators::mfi_simd::{TState, options::SimdState};

/// SIMD driver for the Money Flow Index (MFI) indicator, processing `N` option-set lanes per scheduling epoch.
struct MfiDriver {
    want_optional_outputs: bool,
}

impl Driver<State<Warm>, usize> for MfiDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&usize>>,
    ) {
        let periods: [usize; N] = std::array::from_fn(|i| *options[i].unwrap());
        let mut state = SimdState::<N>::from_states(&mut states, periods);
        let len = outputs[0][0].len();

        let want_typprice = self.want_optional_outputs;

        //collect outputs
        let (mfi_line_ptr, typprice_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, mfi_line_ptr, typprice_line_ptr);

        let (high_ptrs, low_ptrs, close_ptrs, volume_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs, volume_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let inputs = unsafe {
                (
                    *high_ptrs[0].add(i),
                    *low_ptrs[0].add(i),
                    *close_ptrs[0].add(i),
                    *volume_ptrs[0].add(i),
                )
            };

            let mfi = state.calc(inputs);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                mfi_line_ptr => mfi
            );
            crate::store_simd_optional_outputs!(i, N,
                want_typprice, typprice_line_ptr => Simd::<f64, N>::splat(state.typprice)
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Money Flow Index (MFI) on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[high, low, close, volume]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Optional output flags: `[want_typprice]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[mfi, typprice?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Mfi::min_data)?;
    validate_options(options, None)?;
    let periods: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);
    let mut road_train = PrimeMover::<N, State<Warm>, usize>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = false;
    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
            inputs[2], // close
            inputs[3], // volume
        ];

        let (mfi_line, mut typprice_line) = {
            let len = inputs[0].len();
            let capacity = Mfi::output_length(len, options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    typprice_line: len
                ),
            )
        };

        let state = State::init_state(
            (inputs[0], inputs[1], inputs[2], inputs[3]),
            periods[i],
            &mut typprice_line,
        );

        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(typprice_line);
        }
        let mut starts = [0; 2];
        starts[1] = crate::slice_outputs_start!(mfi_line.len(), typprice_line);

        let mut output_buffer = vec![mfi_line, typprice_line];

        //let adosc_len = output_buffer[0].len();
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
            periods[i],
            0,
            state,
            Some(&periods[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = MfiDriver {
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
