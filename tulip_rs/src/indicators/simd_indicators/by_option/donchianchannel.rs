use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::donchianchannel::{
    min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::donchianchannel_simd::{options::Calc, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;
/// SIMD driver for the Donchian Channel indicator, processing `N` option-set lanes per scheduling epoch.
struct DonchianChannelDriver;

impl Driver<State, usize> for DonchianChannelDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    ///
    /// Reads the shared input, applies each lane's period, writes lower/middle/upper outputs,
    /// and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&usize>>,
    ) {
        let len = outputs[0][0].len();

        let (look_back, mut i_simd) = {
            let mut look_back = [0; N];
            let mut i_array = [0; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&l) = option {
                    look_back[lane] = l;
                    i_array[lane] = l;
                }
            }
            (Simd::from_array(look_back), Simd::from_array(i_array))
        };

        //collect outputs
        let (lower_line_ptr, middle_line_ptr, upper_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, lower, middle, upper);

        let (high_ptrs, low_ptrs) = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs);

        let mut state = SimdState::new(&mut states);
        let one_splat = Simd::splat(1);

        for j in 0..len {
            let (lower, middle, upper) =
                unsafe { state.calc_unchecked_simd(high_ptrs, low_ptrs, i_simd, look_back) };

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                lower_line_ptr => lower,
                middle_line_ptr => middle,
                upper_line_ptr => upper
            );

            i_simd += one_splat;
        }
        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Donchian Channel indicator on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[high, low]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `_optional_outputs` - Unused; pass `None`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[lower, middle, upper]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let periods: [(usize, usize); N] = std::array::from_fn(|i| {
        let period = options[i][0] as usize;
        (period, period - 1)
    });
    let mut road_train = PrimeMover::<N, State, usize>::new();
    let mut output_buffers = Vec::with_capacity(N);

    let [high, low] = *inputs;
    for i in 0..N {
        let asset_inputs = vec![high, low];

        let (lower_line, middle_line, upper_line) = {
            let len = high.len();
            let capacity = output_length(len, options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };
        let state = State::new(high, low, periods[i]);

        let mut output_buffer = vec![lower_line, middle_line, upper_line];

        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr(), //slice from
                    output_buffer.len(),               // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            periods[i].1,
            periods[i].1,
            state,
            Some(&periods[i].1),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = DonchianChannelDriver;

    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (state, &periods) in states_vec.into_iter().zip(periods.iter()) {
        states.push(IndicatorState::new(state, high, low, periods));
    }
    Ok((output_buffers, states))
}
