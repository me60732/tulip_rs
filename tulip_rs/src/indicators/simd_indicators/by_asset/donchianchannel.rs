//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::donchianchannel::{
    min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::donchianchannel_simd::{assets::Calc, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
/// SIMD driver that advances the Donchian Channel indicator across `N` asset lanes per scheduling
/// epoch.
struct DonchianChannelDriver {
    look_back: usize,
}

impl Driver<State> for DonchianChannelDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][field]` (high, low), writes to `outputs[asset][output]`,
    /// and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let data_len = inputs[0][0].len();

        //collect outputs
        let outputs = crate::extract_output_ptrs!(outputs, N, lower_ptr, middle_ptr, upper_ptr);
        let inputs = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs);
        let mut state = SimdState::new(&mut states);

        match self.look_back {
            1..=20 => {
                self.cycle::<N, 1>(inputs, outputs, data_len, &mut state);
            }
            /*6..=50 => {
                self.cycle::<N, 4>(inputs, outputs, data_len, &mut state);
            }*/
            _ => {
                self.cycle::<N, 8>(inputs, outputs, data_len, &mut state);
            }
        }
        // Update states efficiently
        state.write_states(&mut states);
    }
}
impl DonchianChannelDriver {
    /// Inner SIMD loop for one epoch of the Donchian Channel computation.
    ///
    /// Iterates over `look_back..data_len` bars, calling the unchecked SIMD `calc`
    /// for all `N` asset lanes at each bar, and writing lower/middle/upper
    /// to the pre-computed output pointers.
    ///
    /// `CHUNK_SIZE` is the const-generic SIMD prefetch hint passed down to the min/max helpers.
    fn cycle<const N: usize, const CHUNK_SIZE: usize>(
        &self,
        inputs: ([*const f64; N], [*const f64; N]),
        outputs: ([*mut f64; N], [*mut f64; N], [*mut f64; N]),
        data_len: usize,
        state: &mut SimdState<N>,
    ) {
        let (high_ptrs, low_ptrs) = inputs;
        let (lower_line_ptr, middle_line_ptr, upper_line_ptr) = outputs;

        for (j, i) in (self.look_back..data_len).enumerate() {
            let (lower, middle, upper) = unsafe {
                state.calc_unchecked_simd::<CHUNK_SIZE>(high_ptrs, low_ptrs, i, self.look_back)
            };

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                lower_line_ptr => lower,
                middle_line_ptr => middle,
                upper_line_ptr => upper
            );
        }
    }
}

/// Calculates the Donchian Channel indicator for `N` assets simultaneously using SIMD parallelism.
///
/// All assets share the same `options`. Uses the [`PrimeMover`] scheduler to batch assets
/// into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low]` for asset `i`.
/// * `options` - Shared options applied to all `N` assets: `[period]`.
/// * `_optional_outputs` - Unused; pass `None`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[lower, middle, upper]`
/// for asset `i` and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let periods = (options[0] as usize, options[0] as usize - 1);

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let [high, low] = *inputs[i];
        let asset_inputs = vec![high, low];

        let (lower_line, middle_line, upper_line) = {
            let len = inputs[i][0].len();
            let capacity = output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let state = State::new(high, low, periods);

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
            periods.1,
            periods.1,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = DonchianChannelDriver {
        look_back: periods.1,
    };
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            state,
            inputs[i][0],
            inputs[i][1],
            periods,
        ));
    }
    Ok((output_buffers, states))
}
