//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::vwap_simd::{SimdState, TSimdState, TState};
use crate::indicators::vwap::{
    Vwap, Indicator, IndicatorState as State, INPUTS, OPTIONS,
};
use crate::types::IndicatorError;
use std::simd::Simd;
/// SIMD driver that advances the VWAP indicator across `N` asset lanes per scheduling epoch.
struct VwapDriver {
    want_optional_outputs: bool,
}

impl Driver<State> for VwapDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let mut state = SimdState::from_states(&mut states);
        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs, close_ptrs, volume_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs, volume_ptrs);

        let (vwap_line_ptr, typprice_line_ptr) = crate::extract_output_ptrs!(outputs, N, vwap, tp);
        let want_tp = self.want_optional_outputs;
        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            let inputs = crate::extract_simd_inputs_at_index!(i, N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs,
                volume @ volume_ptrs
            );

            let (vwap, tp) = state.calc(inputs);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                vwap_line_ptr => vwap
            );
            crate::store_simd_optional_outputs!(i, N,
                want_tp, typprice_line_ptr => tp
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Volume Weighted Average Price (VWAP) for `N` assets simultaneously
/// using SIMD parallelism.
///
/// VWAP takes no configurable options and produces one optional output (`typprice`).
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[high, low, close, volume]` for asset `i`.
/// * `_options` - Unused; VWAP has no configurable options.
/// * `optional_outputs` - Pass `Some(&[true])` to include `typprice` as `outputs[i][1]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the VWAP line for asset `i`,
/// `outputs[i][1]` is `typprice` (empty unless requested), and
/// `states[i]` is the final [`State`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    _options: &[f64; OPTIONS],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<State>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Vwap::min_data(&[]))?;

    let mut road_train = PrimeMover::<N, State>::new();
    let mut want_optional_outputs = false;
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let [high, low, close, volume] = *inputs[i];
        let asset_inputs = vec![high, low, close, volume];

        let (vwap_line, typprice_line) = {
            let capacity = Vwap::output_length(high.len(), &[]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    tp: capacity
                ),
            )
        };

        let state = State::new();

        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(typprice_line);
        }

        let mut output_buffer = vec![vwap_line, typprice_line];

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
            0,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = VwapDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    Ok((output_buffers, states_vec))
}
