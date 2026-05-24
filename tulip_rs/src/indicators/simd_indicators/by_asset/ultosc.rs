use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::ultosc_simd::assets::SimdState;
use crate::indicators::ultosc::{
    min_data, output_length, validate_options, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver that advances the Ultimate Oscillator (ULTOSC) across `N` asset lanes per scheduling epoch.
struct UltoscDriver {
    periods: (usize, usize),
}

impl Driver<State> for UltoscDriver {
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
        let ultosc_line_ptr = crate::extract_output_ptrs!(outputs, N, cvi_line_ptr);

        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);

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

            let ultosc = unsafe { state.calc_unchecked(&high, &low, &close, self.periods) };

            crate::write_simd_at_indices!(N, i,
                ultosc_line_ptr => ultosc
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Ultimate Oscillator (ULTOSC) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// ULTOSC produces no optional outputs. Uses the [`PrimeMover`] scheduler to batch assets into
/// SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close]` for asset `i`.
/// * `options` - `options[0]` is `short_period`, `options[1]` is `medium_period`,
///   `options[2]` is `long_period`.
/// * `_optional_outputs` - Unused; ULTOSC has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the ULTOSC line for asset `i` and
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let periods = (
        options[0] as usize,
        options[1] as usize,
        options[2] as usize,
    );

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // high
            inputs[i][1], // low
            inputs[i][2], // close
        ];

        let mut ultosc_line = {
            let capacity = output_length(inputs[i][0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::init_state(
            inputs[i][0],
            inputs[i][1],
            inputs[i][2],
            periods,
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
            periods.2 + 1,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = UltoscDriver {
        periods: (periods.0, periods.1),
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state, (periods.0, periods.1)));
    }
    Ok((output_buffers, states))
}
