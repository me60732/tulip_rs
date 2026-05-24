//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::mass::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::mass_simd::option::SimdState;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;

/// SIMD driver for the Mass Index (MASS) indicator, processing `N` option-set lanes per scheduling epoch.
struct MassDriver {
    multipliers: (f64, f64),
}

impl Driver<State, usize> for MassDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&usize>>,
    ) {
        let periods: [usize; N] = std::array::from_fn(|i| *options[i].unwrap());
        let mut state = SimdState::<N>::new(&mut states, periods);
        /*for i in 0..N {
            println!("{},  Buffer Len: {:?}, Inputs Len: {:?}",i, states[i].buffer.capacity, inputs[i][0].len());
        }*/
        let len = outputs[0][0].len();

        let multipliers = (self.multipliers.0, self.multipliers.1);

        //collect outputs
        let mass_line_ptr = crate::extract_output_ptrs!(outputs, N, mass_line_ptr);

        let (high_ptrs, low_ptrs) = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let (high, low) = unsafe { (*high_ptrs[0].add(i), *low_ptrs[0].add(i)) };

            let mass = unsafe { state.calc_unchecked(high, low, multipliers) };

            crate::write_simd_at_indices!(N, i,
                mass_line_ptr => mass
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Mass Index (MASS) on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[high, low]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Unused; Mass Index has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[mass]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    //let params: [(f64, f64); N] = std::array::from_fn(|i| multiplier(9));
    let multipliers = multiplier();
    let mut road_train = PrimeMover::<N, State, usize>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let periods: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
        ];

        let mut mass_line = {
            let capacity = output_length(inputs[0].len(), options[i]);
            crate::uninit_vec!(f64, capacity)
        };

        let (start, state) = State::init_state(
            inputs[0],
            inputs[1],
            periods[i],
            multipliers,
            &mut mass_line,
        );

        let mut output_buffer = vec![mass_line];

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
            start,
            0,
            state,
            Some(&periods[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = MassDriver { multipliers };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state, multipliers));
    }
    Ok((output_buffers, states))
}
