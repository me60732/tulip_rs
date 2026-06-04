//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::{
    keltnerchannel::{
        min_data, output_length, validate_options, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH, multiplier
    },
    tr::output_length as tr_output_length
};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::keltnerchannel_simd::SimdState;
use crate::types::IndicatorError;
use std::simd::Simd;
/*pub use crate::indicators::simd::{
    bbands_simd::calc_simd,
    stddev_simd::{calc_simd as calc_stddev_simd, SimdState},
};*/

/// SIMD driver that advances the Bollinger Bands (BBANDS) across `N` asset lanes per scheduling
/// epoch.
struct KeltnerChannelDriver {
    multipliers: ((f64, f64), (f64, f64)),
    step: f64,
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State> for KeltnerChannelDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][0]` (real prices), writes `[lower_band, middle_band, upper_band]`
    /// to `outputs[asset]`, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let step = Simd::splat(self.step);
        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::new(&mut states);

        let multipliers = (
            (
                Simd::splat(self.multipliers.0.0),
                Simd::splat(self.multipliers.0.1)
            ),
            (
                Simd::splat(self.multipliers.1.0),
                Simd::splat(self.multipliers.1.1)
            )
        );

        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs, close_ptrs) = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);
        let (lower_band_ptr, middle_band_ptr, upper_band_ptr, atr_line_ptr, tr_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            lower_band_ptr,
            middle_band_ptr,
            upper_band_ptr,
            atr_line_ptr,
            tr_line_ptr
        );
        let (has_optional, want_atr, want_tr) = self.want_optional_outputs;
        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get new and old values using pre-computed pointers
            let (high, low, close) = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs
            );

            let (lower_band, middle_band, upper_band, atr, tr) = state.calc_simd(high, low, close, step, multipliers);

            crate::write_simd_at_indices!(N, i,
                lower_band_ptr => lower_band,
                middle_band_ptr => middle_band,
                upper_band_ptr => upper_band
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_atr, atr_line_ptr => atr,
                    want_tr, tr_line_ptr => tr
                );
            }
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Bollinger Bands (BBANDS) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// Bollinger Bands consist of a middle SMA band and upper/lower bands placed a configurable
/// number of standard deviations away. All assets share the same `options`. Uses the
/// [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing the real price series for asset `i`.
/// * `options` - Shared options applied to all `N` assets: `[period, std_dev_multiplier]`.
/// * `_optional_outputs` - Unused; BBANDS has no optional output lines.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[lower_band, middle_band, upper_band]`
/// for asset `i` and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let step = options[1];
    let multipliers = multiplier(period);

    let mut road_train = PrimeMover::<N, State>::new();
    let mut want_optional_outputs = (false, false, false);
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // high
            inputs[i][1], // low
            inputs[i][2], // close
        ];

        let (middle_band, upper_band, lower_band, (atr_line, mut tr_line)) = {
            let len = inputs[i][0].len();
            let capacity = output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    atr_line: capacity,
                    tr_line: tr_output_length(len, options)
                ),
            )
        };

        let state = State::init_state(inputs[i][0], inputs[i][1], inputs[i][2], period, multipliers, &mut tr_line);

        let mut starts = [0; 5];
        starts[4] = crate::slice_outputs_start!(atr_line.len(), tr_line);
        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(atr_line, tr_line);
        }

        let mut output_buffer = vec![lower_band, middle_band, upper_band, atr_line, tr_line];

        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(starts[j]), //slice from
                    output_buffer.len() - starts[j],                       // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = KeltnerChannelDriver {
        multipliers,
        want_optional_outputs,
        step
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state, step, multipliers));
    }
    Ok((output_buffers, states))
}
