//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::stochrsi_simd::options::SimdState;
use crate::indicators::{
    rsi::{multiplier, output_length as rsi_output_length},
    stochrsi::{min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH},
};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver for the Stochastic RSI (STOCHRSI) indicator, processing `N` option-set lanes per scheduling epoch.
struct StochrsiDriver {
    want_optional_outputs: bool,
}

impl Driver<State, (usize, f64)> for StochrsiDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(usize, f64)>>,
    ) {
        let len = outputs[0][0].len();
        let (period, multiplier) = {
            let mut period = [0usize; N];
            let mut multiplier = [0.0; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&(lookback, multi)) = option {
                    period[lane] = lookback;
                    multiplier[lane] = multi;
                }
            }
            (Simd::from_array(period), Simd::from_array(multiplier))
        };
        let mut state = SimdState::<N>::new(&mut states);
        let want_rsi = self.want_optional_outputs;
        //collect outputs
        let (stochrsi_line_ptr, rsi_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, stochrsi_line_ptr, rsi_line_ptr);

        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index_splat!(i, N,
                real @ real_ptrs
            );
            let (stochrsi, rsi) = unsafe { state.calc_simd_unchecked(real, multiplier, period) };

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                stochrsi_line_ptr => stochrsi
            );
            crate::store_simd_optional_outputs!(i, N,
                want_rsi, rsi_line_ptr => rsi
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Stochastic RSI (STOCHRSI) indicator for one asset with `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// Applies each of the `N` period configurations to the same shared input series, computing
/// Stochastic RSI values for all option sets in a single SIMD-accelerated pass via [`PrimeMover`].
///
/// # Arguments
/// * `inputs` - Shared input: `inputs[0]` is the `real` price series.
/// * `options` - An array of `N` option sets; `options[i][0]` is the `period` for lane `i`.
/// * `optional_outputs` - Optional slice of booleans enabling extra outputs per lane:
///   `[0]` → `rsi`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `stochrsi` and `outputs[i][1]` is `rsi`
/// (empty if not requested) for option set `i`, and `states[i]` is the final
/// [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [(usize, f64); N] = std::array::from_fn(|i| {
        let period = options[i][0] as usize;
        (period, multiplier(period))
    });
    let mut road_train = PrimeMover::<N, State, (usize, f64)>::new();
    let mut want_optional_outputs = false;
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let (stochrsi_line, mut rsi_line);
        {
            let len = inputs[0].len();
            let capacity = output_length(len, options[i]);
            stochrsi_line = crate::uninit_vec!(f64, capacity);
            let rsi_capacity = rsi_output_length(len, options[i]);
            rsi_line = crate::init_optional_outputs_eff!(
                optional_outputs, &[false],
                rsi_line: rsi_capacity
            );
        }
        let state = State::init_state(&inputs[0], params[i].0, &mut rsi_line);

        if i == 0 {
            (want_optional_outputs, _) = crate::calc_want_flags!(rsi_line);
        }

        let mut starts = [0; 2];
        starts[1] = crate::slice_outputs_start!(stochrsi_line.len(), rsi_line);

        let mut output_buffer = vec![stochrsi_line, rsi_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(starts[j]), //slice from
                    output_buffer.len(),                       // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            params[i].0 * 2,
            0,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = StochrsiDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, param) in states_vec.into_iter().zip(params.iter()) {
        states.push(IndicatorState::new(state, param.0, param.1));
    }
    Ok((output_buffers, states))
}
