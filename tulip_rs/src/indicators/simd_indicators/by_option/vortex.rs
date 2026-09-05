//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::vortex_simd::options::SimdState;
use crate::indicators::{
    tr::Tr,
    vortex::{Indicator, IndicatorState, State, TState, Vortex, INPUTS, OPTIONS},
};
use crate::types::{IndicatorError, Warm};
/// SIMD driver for the Vortex indicator, processing `N` option-set (period) lanes per scheduling epoch.
struct VortexDriver {
    want_optional_outputs: bool,
}

impl Driver<State<Warm>, usize> for VortexDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();
        let mut state = {
            let mut periods = [0usize; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&period) = option {
                    periods[lane] = period;
                }
            }
            SimdState::<N>::from_states(&mut states, periods)
        };
        //collect outputs
        let (vi_up_line_ptr, vi_down_line_ptr, tr_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, vi_up, vi_down, tr);

        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);
        let want_tr = self.want_optional_outputs;
        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let inputs = unsafe {
                (
                    *high_ptrs[0].add(i),
                    *low_ptrs[0].add(i),
                    *close_ptrs[0].add(i),
                )
            };

            let (vi_up, vi_down, tr) = state.calc(inputs);

            crate::write_simd_at_indices!(N, i,
                vi_up_line_ptr => vi_up,
                vi_down_line_ptr => vi_down
            );
            crate::store_simd_optional_outputs!(i, N,
                want_tr, tr_line_ptr => tr
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Vortex indicator for one shared asset across `N` different period options
/// simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch option-set lanes into SIMD-width groups.
/// Supports the optional TR output via `optional_outputs`.
///
/// # Arguments
/// * `inputs` - Shared input data: `inputs[0]` is `high`, `inputs[1]` is `low`,
///   `inputs[2]` is `close`.
/// * `options` - An array of `N` option sets; `options[i]` is `&[f64; OPTIONS]`
///   containing `[period]` for lane `i`.
/// * `optional_outputs` - `Some(&[true])` to enable the optional TR output for all lanes.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `vi_up`, `outputs[i][1]` is `vi_down`,
/// `outputs[i][2]` is the optional TR line for option lane `i`, and `states[i]` is the final
/// [`IndicatorState`] for lane `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or any option set is invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Vortex::min_data)?;
    validate_options(options, None)?;
    let period: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);
    let mut road_train = PrimeMover::<N, State<Warm>, usize>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let [high, low, close] = *inputs;
    let mut want_optional_outputs = false;
    for i in 0..N {
        let asset_inputs = vec![high, low, close];

        let (vi_up_line, vi_down_line, mut tr_line) = {
            let len = high.len();
            let capacity = Vortex::output_length(len, options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    tr_line_line: Tr::output_length(len, &[])
                ),
            )
        };

        let state = State::init_state(high, low, close, period[i], &mut tr_line);

        let mut starts = [0; 3];
        starts[2] = crate::slice_outputs_start!(vi_up_line.len(), tr_line);

        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(tr_line);
        }
        let mut output_buffer = vec![vi_up_line, vi_down_line, tr_line];

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
            period[i] + 1,
            0,
            state,
            Some(&period[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = VortexDriver {
        want_optional_outputs,
    };

    let states_vec = road_train.drive(&mut driver);

    Ok((output_buffers, states_vec))
}
