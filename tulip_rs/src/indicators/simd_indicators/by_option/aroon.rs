//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::aroon::{Aroon, Indicator, IndicatorState, State, INPUTS, OPTIONS};
use crate::indicators::simd_indicators::aroon_simd::{options::SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;
/// SIMD driver for the Aroon (AROON) indicator, processing `N` option-set lanes per scheduling epoch.
struct AroonDriver {}

impl Driver<State<Warm>, usize> for AroonDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();

        let (period, mut i_simd) = {
            let mut period = [0; N];
            let mut i_array = [0; N];
            for (i, option) in options.iter().enumerate() {
                if let Some(&p) = option {
                    period[i] = p;
                    i_array[i] = p;
                }
            }
            (Simd::from_array(period), Simd::from_array(i_array))
        };

        //collect outputs
        let (aroon_down_ptr, aroon_up_ptr) =
            crate::extract_output_ptrs!(outputs, N, aroon_down_ptr, aroon_up_ptr);

        let (high_ptrs, low_ptrs) = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs);

        let mut state = SimdState::<N>::from_states(&mut states);
        let one_splat = Simd::splat(1);
        //println!("start: {:?}, N: {:?}, LEN: {:?}", start, N, real.len());
        for j in 0..len {
            let (aroon_down, aroon_up) = state.calc((high_ptrs, low_ptrs, i_simd, period));

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
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[high, low]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Unused; Aroon has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[aroon_down, aroon_up]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Aroon::min_data)?;
    validate_options(options, None)?;
    let periods: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);
    let mut road_train = PrimeMover::<N, State<Warm>, usize>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let [high, low] = *inputs;
        let asset_inputs = vec![high, low];

        let (aroon_down_line, aroon_up_line) = {
            let len = inputs[0].len();
            let capacity = Aroon::output_length(len, options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let state = State::init_state(high, low, periods[i]);

        let mut output_buffer = vec![aroon_down_line, aroon_up_line];

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
            periods[i],
            periods[i],
            state,
            Some(&periods[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = AroonDriver {};
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (state, &option) in states_vec.into_iter().zip(options.iter()) {
        let period = option[0] as usize;
        states.push(IndicatorState::new(inputs[0], inputs[1], state, period));
    }
    Ok((output_buffers, states))
}
