use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::chaikinmf::{
    ChaikinMf, Indicator, IndicatorState, State, INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::chaikinmf_simd::{TState, options::SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};

struct ChaikinMfDriver;

impl Driver<State<Warm>, usize> for ChaikinMfDriver {
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

        let cmf_line_ptr = crate::extract_output_ptrs!(outputs, N, cmf_line_ptr);
        let (high_ptrs, low_ptrs, close_ptrs, volume_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs, volume_ptrs);

        for i in 0..len {
            let inputs = unsafe {
                (
                    *high_ptrs[0].add(i),
                    *low_ptrs[0].add(i),
                    *close_ptrs[0].add(i),
                    *volume_ptrs[0].add(i),
                )
            };

            let cmf = state.calc(inputs);

            crate::write_simd_at_indices!(N, i,
                cmf_line_ptr => cmf
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates Chaikin Money Flow on a single asset with `N` different periods simultaneously
/// using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`),
///   containing `[high, low, close, volume]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `_optional_outputs` - Unused; ChaikinMF has no optional output lines.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the CMF series for option set `i`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, ChaikinMf::min_data)?;
    validate_options(options, None)?;
    let periods: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);

    let mut road_train = PrimeMover::<N, State<Warm>, usize>::new();
    let mut output_buffers = Vec::with_capacity(N);

    let [high, low, close, volume] = *inputs;
    for i in 0..N {
        let asset_inputs = vec![high, low, close, volume];

        let cmf_line = {
            let len = high.len();
            let capacity = ChaikinMf::output_length(len, options[i]);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::init_state((high, low, close, volume), periods[i]);

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
            periods[i],
            0,
            state,
            Some(&periods[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = ChaikinMfDriver;
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
