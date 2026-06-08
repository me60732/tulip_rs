use crate::common_simd::assets::validate_inputs;
use crate::common::validate_options;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::vortex_simd::assets::SimdState;
use crate::indicators::{
    vortex::{
        min_data, output_length, IndicatorState as State, INPUTS_WIDTH, OPTIONS_WIDTH,
    },
    tr::output_length as tr_output_length
};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver that advances the Vortex indicator across `N` asset lanes per scheduling epoch.
struct VortexDriver {
    want_optional_outputs: bool
}

impl Driver<State> for VortexDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::new(&mut states);
        let len = inputs[0][0].len();

        //collect outputs
        let (vi_up_line_ptr, vi_down_line_ptr, tr_line_ptr) = crate::extract_output_ptrs!(outputs, N, vi_up, vi_down, tr);

        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);
        let want_tr = self.want_optional_outputs;
        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let (high, low, close) = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs
            );

            let (vi_up, vi_down, tr) = unsafe { state.calc_unchecked(high, low, close) };

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

/// Calculates the Vortex indicator for `N` assets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
/// Supports the optional TR output via `optional_outputs`.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close]` for asset `i`.
/// * `options` - `options[0]` is `period`.
/// * `optional_outputs` - `Some(&[true])` to enable the optional TR output for all assets.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `vi_up`, `outputs[i][1]` is `vi_down`,
/// `outputs[i][2]` is the optional TR line for asset `i`, and `states[i]` is the final
/// [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<State>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = false;
    for i in 0..N {
        let [high, low, close] = *inputs[i];
        let asset_inputs = vec![
            high, low, close
        ];

        let (vi_up_line, vi_down_line, mut tr_line) = {
            let len = high.len();
            let capacity = output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    tr_line_line: tr_output_length(len, options)
                )
            )
        };

        let state = State::init_state(
            high,
            low,
            close,
            period,
            &mut tr_line,
        );
        
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
                    output_buffer.len() - starts[j],               // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period + 1,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = VortexDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    Ok((output_buffers, states_vec))
}
