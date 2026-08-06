//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::vhf_simd::{TSimdState, assets::SimdState, TState};
use crate::indicators::vhf::{
    Vhf, Indicator, IndicatorState, State, INPUTS, OPTIONS,
};
use crate::types::{IndicatorError, Warm};
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;
/// SIMD driver that advances the Vertical Horizontal Filter (VHF) across `N` asset lanes per scheduling epoch.
struct VhfDriver {
    period: usize,
}

impl Driver<State<Warm>> for VhfDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        //collect outputs
        let vhf_line_ptr = crate::extract_output_ptrs!(outputs, N, vhf_line_ptr);
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let mut state = SimdState::from_states(&mut states);
        cycle::<N>(real_ptrs, len, self.period, &mut state, vhf_line_ptr);
        
        // Update states efficiently
        state.write_states(&mut states);
    }
}
fn cycle<const N: usize>(
    real_ptrs: [*const f64; N],
    len: usize,
    period: usize,
    state: &mut SimdState<N>,
    vhf_line_ptr: [*mut f64; N],
) {
    let look_back = period - 1;
    for (j, i) in (period + 1..len).enumerate() {
        let cur_vals = crate::extract_simd_at_indices!(N, real_ptrs,
            cur_vals @ i
        );

        let vhf = state.calc((cur_vals, real_ptrs, look_back, i));

        // Store results using pre-computed pointers
        crate::write_simd_at_indices!(N, j,
            vhf_line_ptr => vhf
        );
    }
}
/// Calculates the Vertical Horizontal Filter (VHF) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// VHF produces no optional outputs. Uses the [`PrimeMover`] scheduler to batch assets into
/// SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[real]` for asset `i`.
/// * `options` - `options[0]` is the `period`.
/// * `_optional_outputs` - Unused; VHF has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the VHF line for asset `i` and
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Vhf::min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // real
        ];

        let mut vhf_line = {
            let len = inputs[i][0].len();
            let capacity = Vhf::output_length(len, options);
            crate::uninit_vec!(f64, capacity)
        };
        let state = State::init_state(inputs[i][0], period, &mut vhf_line);

        let mut output_buffer = vec![vhf_line];

        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(1), //slice from
                    output_buffer.len() - 1,           // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period + 1,
            period + 1,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = VhfDriver { period };
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(state, inputs[i][0], period));
    }
    Ok((output_buffers, states))
}
