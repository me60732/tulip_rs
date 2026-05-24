//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::max::{
    min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::max_simd::{options::Calc, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::ring_buffer::unsync_multi_buffer::multi_buffer::UsizeConstants;
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver for the Maximum In Period (MAX) indicator, processing `N` option-set lanes per scheduling epoch.
struct MaxDriver {}

impl Driver<State, usize> for MaxDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();

        let (look_back, mut i_simd) = {
            let mut look_backs = [0; N];
            let mut i_array = [0; N];
            for (i, option) in options.iter().enumerate() {
                if let Some(&look_back) = option {
                    look_backs[i] = look_back;
                    i_array[i] = look_back;
                }
            }
            (Simd::from_array(look_backs), Simd::from_array(i_array))
        };

        //collect outputs
        let max_line_ptr = crate::extract_output_ptrs!(outputs, N, max_line_ptr);
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let mut state = SimdState::new(&states);

        //println!("start: {:?}, N: {:?}, LEN: {:?}", start, N, real.len());
        for j in 0..len {
            let (max, _) = unsafe { state.calc_unchecked_simd(real_ptrs, i_simd, look_back) };

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                max_line_ptr => max
            );
            i_simd += UsizeConstants::ONE;
        }
        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Maximum In Period (MAX) on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[real]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Unused; MAX has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[max]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let periods: [(usize, usize); N] =
        std::array::from_fn(|i| (options[i][0] as usize, options[i][0] as usize - 1));
    let mut road_train = PrimeMover::<N, State, usize>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for (i, &(_period, look_back)) in periods.iter().enumerate() {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let max_line = {
            let len = inputs[0].len();
            let capacity = output_length(len, options[i]);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::new(
            inputs[0][0], // real
            look_back,
        );

        let mut output_buffer = vec![max_line];

        //let adosc_len = output_buffer[0].len();
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
            look_back,
            look_back,
            state,
            Some(&periods[i].1),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = MaxDriver {};
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(inputs[0], state, periods[i]));
    }
    Ok((output_buffers, states))
}
