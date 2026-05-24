//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::ultosc_simd::options::SimdState;
use crate::indicators::ultosc::{
    min_data, output_length, validate_options as vo, IndicatorState, State, INPUTS_WIDTH,
    OPTIONS_WIDTH,
};
use crate::types::IndicatorError;

/// SIMD driver for the Ultimate Oscillator (ULTOSC) indicator, processing `N` option-set lanes per scheduling epoch.
struct UltoscDriver;

impl Driver<State, (usize, usize, usize)> for UltoscDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(usize, usize, usize)>>,
    ) {
        let len = outputs[0][0].len();
        let mut state = {
            let mut short_periods = [0usize; N];
            let mut medium_periods = [0usize; N];
            let mut long_periods = [0usize; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&(short_period, medium_period, long_period)) = option {
                    short_periods[lane] = short_period;
                    medium_periods[lane] = medium_period;
                    long_periods[lane] = long_period;
                }
            }
            SimdState::<N>::new(&mut states, (short_periods, medium_periods, long_periods))
        };
        //collect outputs
        let ultosc_line_ptr = crate::extract_output_ptrs!(outputs, N, cvi_line_ptr);

        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let (high, low, close) = unsafe {
                (
                    *high_ptrs[0].add(i),
                    *low_ptrs[0].add(i),
                    *close_ptrs[0].add(i),
                )
            };

            let ultosc = unsafe { state.calc_unchecked(high, low, close) };

            crate::write_simd_at_indices!(N, i,
                ultosc_line_ptr => ultosc
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Ultimate Oscillator (ULTOSC) for one shared asset across `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch option sets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - Shared input data: `inputs[0]` is `&[f64]` for `high`, `inputs[1]` for `low`,
///   `inputs[2]` for `close`.
/// * `options` - An array of `N` option sets; `options[i]` is `&[f64; OPTIONS_WIDTH]` containing
///   `[short_period, medium_period, long_period]` for option set `i`.
/// * `optional_outputs` - Unused; ULTOSC has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `ultosc` for option set `i`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or any option set is invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, Some(vo))?;
    let periods: [(usize, usize, usize); N] = std::array::from_fn(|i| {
        (
            options[i][0] as usize,
            options[i][1] as usize,
            options[i][2] as usize,
        )
    });
    let mut road_train = PrimeMover::<N, State, (usize, usize, usize)>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
            inputs[2], // close
        ];

        let mut ultosc_line = {
            let capacity = output_length(inputs[0].len(), options[i]);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::init_state(
            inputs[0],
            inputs[1],
            inputs[2],
            periods[i],
            &mut ultosc_line,
        );

        let mut output_buffer = vec![ultosc_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(1), //slice from
                    output_buffer.len(),               // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            periods[i].2 + 1,
            0,
            state,
            Some(&periods[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = UltoscDriver;

    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, periods) in states_vec.into_iter().zip(periods.into_iter()) {
        states.push(IndicatorState::new(state, (periods.0, periods.1)));
    }
    Ok((output_buffers, states))
}
