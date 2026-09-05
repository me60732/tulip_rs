//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::vosc_simd::{SimdState, TSimdState, TState};
use crate::indicators::{
    sma::{Indicator, Sma},
    vosc::{validate_options as vo, IndicatorState, State, Vosc, INPUTS, OPTIONS},
};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver for the Volume Oscillator (VOSC) indicator, processing `N` option-set lanes per scheduling epoch.
struct VoscDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State<Warm>, (usize, usize)> for VoscDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&(usize, usize)>>,
    ) {
        let len = outputs[0][0].len();

        let mut i = [0usize; N];
        let mut short = [0usize; N];
        for (lane, option) in options.iter().enumerate() {
            if let Some(&(short_period, long_period)) = option {
                short[lane] = long_period - short_period;
                i[lane] = long_period;
            }
        }

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::<N>::from_states(&mut states);
        let (has_optional, want_short_sma, want_long_sma) = self.want_optional_outputs;

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (vosc_line_ptr, short_sma_line_ptr, long_sma_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            vosc_line_ptr,
            short_sma_line_ptr,
            long_sma_line_ptr
        );

        // Optimization 3: Simplified main loop with pre-computed offsets
        for j in 0..len {
            let long_volume = crate::extract_simd_inputs_at_index!(j, N, long @ input_ptrs);

            let (volume, short_volume) = crate::extract_simd_at_indices_array!(N, input_ptrs,
                value @ i,
                short_value @ short
            );

            let (vosc, short_sma, long_sma) = state.calc((volume, short_volume, long_volume));

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                vosc_line_ptr => vosc
            );

            if has_optional {
                crate::store_simd_optional_outputs!(j, N,
                    want_short_sma, short_sma_line_ptr => short_sma,
                    want_long_sma, long_sma_line_ptr => long_sma
                );
            }

            for (i, short) in i.iter_mut().zip(short.iter_mut()) {
                *i += 1;
                *short += 1;
            }
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Volume Oscillator (VOSC) for one shared asset across `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch option sets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - Shared input data: `inputs[0]` is `&[f64]` containing `volume`.
/// * `options` - An array of `N` option sets; `options[i]` is `&[f64; OPTIONS]` containing
///   `[short_period, long_period]` for option set `i`.
/// * `optional_outputs` - Optional slice controlling extra output series;
///   index 0 enables `short_sma`, index 1 enables `long_sma`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `vosc`, `outputs[i][1]` is `short_sma`
/// (empty unless requested), and `outputs[i][2]` is `long_sma` (empty unless requested) for option set `i`,
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or any option set is invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Vosc::min_data)?;
    validate_options(options, Some(vo))?;
    let params: [(usize, usize); N] =
        std::array::from_fn(|i| (options[i][0] as usize, options[i][1] as usize));

    let mut road_train = PrimeMover::<N, State<Warm>, (usize, usize)>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = (false, false, false);

    for (i, param) in params.iter().enumerate() {
        let asset_inputs = vec![inputs[0]];
        let (vosc_line, (mut short_sma_line, long_sma_line)) = {
            let len = inputs[0].len();
            let capacity = Vosc::output_length(len, options[i]);
            let short_capacity = Sma::output_length(len, &[param.0 as f64]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    short_sma_line: short_capacity,
                    long_sma_line: capacity
                ),
            )
        };

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(short_sma_line, long_sma_line);
        }
        let mut starts = [0; N];
        starts[1] = crate::slice_outputs_start!(vosc_line.len(), short_sma_line);

        let state = State::init_state(param.0, param.1, inputs[0], &mut short_sma_line);

        let mut output_buffer = vec![vosc_line, short_sma_line, long_sma_line];

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
            param.1,
            param.1,
            state,
            Some(&param),
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = VoscDriver {
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    let mut indicator_states = Vec::with_capacity(N);
    for (state, param) in states.into_iter().zip(params.into_iter()) {
        indicator_states.push(IndicatorState::new(inputs[0], state, (param.0, param.1)));
    }
    Ok((output_buffers, indicator_states))
}
