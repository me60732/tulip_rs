//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::stochrsi_simd::assets::SimdState;
use crate::indicators::{
    rsi::{multiplier, output_length as rsi_output_length},
    stochrsi::{min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH},
};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Stochastic RSI (STOCHRSI) across `N` asset lanes per scheduling epoch.
struct StochrsiDriver {
    want_optional_outputs: bool,
    period: usize,
    multipliers: (f64, f64),
}

impl Driver<State> for StochrsiDriver {
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
        let multipliers_simd = (
            Simd::splat(self.multipliers.0),
            Simd::splat(self.multipliers.1),
        );
        let want_rsi = self.want_optional_outputs;
        // Optimization 1: Direct array construction instead of collect+try_into

        //collect outputs
        let (stochrsi_line_ptr, rsi_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, stochrsi_line_ptr, rsi_line_ptr);

        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        match self.period {
            1..=14 => {
                for i in 0..len {
                    // Get inputs arrays for stocks
                    let real = crate::extract_simd_inputs_at_index!(
                        i,
                        N,
                        real @ real_ptrs
                    );

                    let (stochrsi, rsi) = state.calc_simd::<1>(real, multipliers_simd, self.period);

                    // Store results using pre-computed pointers
                    crate::write_simd_at_indices!(N, i,
                        stochrsi_line_ptr => stochrsi
                    );
                    crate::store_simd_optional_outputs!(i, N,
                        want_rsi, rsi_line_ptr => rsi
                    );
                }
            }
            _ => {
                for i in 0..len {
                    // Get inputs arrays for stocks
                    let real = crate::extract_simd_inputs_at_index!(
                        i,
                        N,
                        real @ real_ptrs
                    );

                    let (stochrsi, rsi) = state.calc_simd::<8>(real, multipliers_simd, self.period);

                    // Store results using pre-computed pointers
                    crate::write_simd_at_indices!(N, i,
                        stochrsi_line_ptr => stochrsi
                    );
                    crate::store_simd_optional_outputs!(i, N,
                        want_rsi, rsi_line_ptr => rsi
                    );
                }
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Stochastic RSI (STOCHRSI) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// STOCHRSI applies the Stochastic Oscillator formula to RSI values, normalising RSI
/// relative to its own high/low range over the look-back period.
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[real]` for asset `i`.
/// * `options` - `[period]` — the look-back period for both the inner RSI and
///   the Stochastic normalisation.
/// * `optional_outputs` - Optional slice of booleans enabling extra outputs:
///   `[0]` → `rsi`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the STOCHRSI line and
/// `outputs[i][1]` is the RSI line (empty unless requested) for asset `i`.
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let (period, multipliers) = {
        let period = options[0] as usize;
        (period, multiplier(period))
    };

    let mut road_train = PrimeMover::<N, State>::new();
    let mut want_optional_outputs = false;
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // real
        ];

        let (stochrsi_line, mut rsi_line);
        {
            let len = inputs[i][0].len();
            let capacity = output_length(len, options);
            stochrsi_line = crate::uninit_vec!(f64, capacity);
            let rsi_capacity = rsi_output_length(len, options);
            rsi_line = crate::init_optional_outputs_eff!(
                optional_outputs, &[false],
                rsi_line: rsi_capacity
            );
        }
        let state = State::init_state(&inputs[i][0], period, &mut rsi_line);

        if i == 0 {
            (want_optional_outputs, _) = crate::calc_want_flags!(rsi_line);
        }

        let mut starts = [0; 2];
        starts[1] = crate::slice_outputs_start!(stochrsi_line.len(), rsi_line);

        let mut output_buffer = vec![stochrsi_line, rsi_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(starts[j]), //slice from
                    output_buffer.len(),                       // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period * 2,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = StochrsiDriver {
        period,
        want_optional_outputs,
        multipliers,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state, period, multipliers));
    }
    Ok((output_buffers, states))
}
