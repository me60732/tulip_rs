//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::aroon::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::aroon_simd::{assets::Calc, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;
/// SIMD driver that advances the Aroon Indicator (AROON) across `N` asset lanes per scheduling
/// epoch.
struct AroonDriver {
    /// The look-back period used to find the highest high and lowest low.
    period: usize,
    /// Pre-computed `100.0 / period` scaling factor applied to Aroon values.
    multiplier: f64,
}

impl Driver<State> for AroonDriver {
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
        let len = inputs[0][0].len();

        //collect outputs
        let (aroon_down_ptr, aroon_up_ptr) =
            crate::extract_output_ptrs!(outputs, N, aroon_down_ptr, aroon_up_ptr);
        let (high_ptrs, low_ptrs) = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs);
        let mut state = SimdState::new(&mut states);
        let multiplier = Simd::splat(self.multiplier);
        //let current: Vec<Simd<f64, N>> = crate::create_simd_vec_from_inputs!(real_ptrs, N, len);
        match self.period {
            1..=14 => {
                for (j, i) in (self.period..len).enumerate() {
                    let (aroon_down, aroon_up) = unsafe {
                        state.calc_unchecked_simd::<1>(
                            high_ptrs,
                            low_ptrs,
                            i,
                            self.period,
                            multiplier,
                        )
                    };

                    // Store results using pre-computed pointers
                    crate::write_simd_at_indices!(N, j,
                        aroon_down_ptr => aroon_down,
                        aroon_up_ptr => aroon_up
                    );
                }
            }
            _ => {
                for (j, i) in (self.period..len).enumerate() {
                    let (aroon_down, aroon_up) = unsafe {
                        state.calc_unchecked_simd::<8>(
                            high_ptrs,
                            low_ptrs,
                            i,
                            self.period,
                            multiplier,
                        )
                    };

                    // Store results using pre-computed pointers
                    crate::write_simd_at_indices!(N, j,
                        aroon_down_ptr => aroon_down,
                        aroon_up_ptr => aroon_up
                    );
                }
            }
        }
        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Aroon Indicator (AROON) for `N` assets simultaneously using SIMD parallelism.
///
/// The Aroon Indicator measures how many periods ago the highest high and lowest low were recorded
/// within the look-back window. All assets share the same `options`. Uses the [`PrimeMover`]
/// scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low]` for asset `i`.
/// * `options` - Shared options applied to all `N` assets: `[period]`.
/// * `_optional_outputs` - Unused; AROON has no optional output lines.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[aroon_down, aroon_up]`
/// for asset `i` and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let multiplier = multiplier(period);
    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // high
            inputs[i][1], // low
        ];

        let (aroon_down_line, aroon_up_line) = {
            let len = inputs[i][0].len();
            let capacity = output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let state = State::new(
            inputs[i][1][0], // low
            period,
            inputs[i][0][0], // high
            period,
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
            period,
            period,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = AroonDriver { period, multiplier };
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            inputs[i][0],
            inputs[i][1],
            state,
            period,
            multiplier,
        ));
    }
    Ok((output_buffers, states))
}
