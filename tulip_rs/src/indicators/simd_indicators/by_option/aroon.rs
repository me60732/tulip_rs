//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::aroon::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::aroon_simd::{options::Calc, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;
/// SIMD driver for the Aroon (AROON) indicator, processing `N` option-set lanes per scheduling epoch.
struct AroonDriver {}

impl Driver<State, (usize, f64)> for AroonDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(usize, f64)>>,
    ) {
        let len = outputs[0][0].len();

        let (period, multiplier, mut i_simd) = {
            let mut period = [0; N];
            let mut i_array = [0; N];
            let mut multiplier = [0.0; N];
            for (i, option) in options.iter().enumerate() {
                if let Some(&(p, m)) = option {
                    period[i] = p;
                    i_array[i] = p;
                    multiplier[i] = m;
                }
            }
            (
                Simd::from_array(period),
                Simd::from_array(multiplier),
                Simd::from_array(i_array),
            )
        };

        //collect outputs
        let (aroon_down_ptr, aroon_up_ptr) =
            crate::extract_output_ptrs!(outputs, N, aroon_down_ptr, aroon_up_ptr);

        let (high_ptrs, low_ptrs) = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs);

        let mut state = SimdState::new(&mut states);
        let one_splat = Simd::splat(1);
        //println!("start: {:?}, N: {:?}, LEN: {:?}", start, N, real.len());
        for j in 0..len {
            let (aroon_down, aroon_up) = unsafe {
                state.calc_unchecked_simd(high_ptrs, low_ptrs, i_simd, period, multiplier)
            };

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                aroon_down_ptr => aroon_down,
                aroon_up_ptr => aroon_up
            );
            i_simd += one_splat;
        }
        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Aroon (AROON) indicator on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[high, low]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Unused; Aroon has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[aroon_down, aroon_up]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [(usize, f64); N] = std::array::from_fn(|i| {
        let period = options[i][0] as usize;
        (period, multiplier(period))
    });
    let mut road_train = PrimeMover::<N, State, (usize, f64)>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
        ];

        let (aroon_down_line, aroon_up_line) = {
            let len = inputs[0].len();
            let capacity = output_length(len, options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let state = State::new(
            inputs[1][0], // low
            params[i].0,
            inputs[0][0], // high
            params[i].0,
        );

        let mut output_buffer = vec![aroon_down_line, aroon_up_line];

        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(0), //slice from
                    output_buffer.len(),               // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            params[i].0,
            params[i].0,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = AroonDriver {};
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (state, &(period, multiplier)) in states_vec.into_iter().zip(params.iter()) {
        states.push(IndicatorState::new(
            inputs[0], inputs[1], state, period, multiplier,
        ));
    }
    Ok((output_buffers, states))
}
