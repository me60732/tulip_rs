//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
//use std::simd::cmp::SimdPartialOrd;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::dema::{Dema, Indicator, IndicatorState, State, INPUTS, OPTIONS};
use crate::indicators::ema::Ema;
use crate::indicators::simd_indicators::dema_simd::{SimdState, TSimdState, TState};
use std::simd::Simd;

/// SIMD driver for the Double Exponential Moving Average (DEMA) indicator, processing `N` option-set lanes per scheduling epoch.
struct DemaDriver {
    want_optional_outputs: bool,
}

impl Driver<State<Warm>> for DemaDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = outputs[0][0].len();

        let mut state = SimdState::<N>::from_states(&mut states);

        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (dema_line_ptr, ema_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, dema_line_ptr, ema_line_ptr);

        // Optimized main loop with minimal overhead
        for j in 0..len {
            let values = crate::extract_simd_inputs_at_index_splat!(j, N, values @ input_ptrs);

            let (dema, ema) = state.calc(values);

            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, j,
                dema_line_ptr => dema
            );
            crate::store_simd_optional_outputs!(j, N,
                self.want_optional_outputs, ema_line_ptr => ema
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Double Exponential Moving Average (DEMA) on a single asset with `N` different option
/// sets simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[real]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Optional output flags: `[want_ema]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[dema, ema?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Dema::min_data)?;
    validate_options(options, None)?;
    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut want_optional_outputs = false;
    for i in 0..N {
        let period = options[i][0] as usize;
        let len = inputs[0].len();
        let dema_capacity = Dema::output_length(len, options[i]);
        let dema_line = crate::uninit_vec!(f64, dema_capacity);
        let ema_capacity = Ema::output_length(len, options[i]);
        let mut ema_line = crate::init_optional_outputs_eff!(
            optional_outputs, &[false],
            ema_line: ema_capacity
        );

        let state = State::init_state(inputs[0], /*dema_capacity,*/ period, &mut ema_line);
        let asset_inputs = vec![inputs[0]];
        let mut starts = [0; 2];
        starts[1] = crate::slice_outputs_start!(dema_capacity, ema_line);

        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(ema_line);
        }
        let mut output_buffer = vec![dema_line, ema_line];
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
            period * 2 - 2,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = DemaDriver {
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
