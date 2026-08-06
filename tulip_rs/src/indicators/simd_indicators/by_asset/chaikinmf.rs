use crate::indicators::chaikinmf::{
    ChaikinMf, Indicator, IndicatorState, State, INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::chaikinmf_simd::{TSimdState, TState, assets::SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

struct ChaikinMfDriver;

impl Driver<State<Warm>> for ChaikinMfDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::from_states(&mut states);
        let len = inputs[0][0].len();

        let cmf_line_ptr = crate::extract_output_ptrs!(outputs, N, cmf_line_ptr);
        let (high_ptrs, low_ptrs, close_ptrs, volume_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs, volume_ptrs);

        for i in 0..len {
            let inputs = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs,
                volume @ volume_ptrs
            );

            let cmf = state.calc(inputs);

            crate::write_simd_at_indices!(N, i,
                cmf_line_ptr => cmf
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates Chaikin Money Flow for `N` assets simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[high, low, close, volume]` for asset `i`.
/// * `options` - Shared options slice; `options[0]` is the period.
/// * `_optional_outputs` - Unused; ChaikinMF has no optional output lines.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the CMF series for asset `i`
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N],
    options: &[f64; OPTIONS],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, ChaikinMf::min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let [high, low, close, volume] = *inputs[i];
        let asset_inputs = vec![high, low, close, volume];

        let cmf_line = {
            let len = high.len();
            let capacity = ChaikinMf::output_length(len, options);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::init_state((high, low, close, volume), period);

        let mut output_buffer = vec![cmf_line];
        let mut asset_outputs = Vec::with_capacity(1);
        unsafe {
            let out = &mut output_buffer[0];
            asset_outputs.push(std::slice::from_raw_parts_mut(out.as_mut_ptr(), out.len()));
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = ChaikinMfDriver;
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
