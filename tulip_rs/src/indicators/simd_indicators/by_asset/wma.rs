//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::wma_simd::SimdState;
use crate::indicators::wma::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Weighted Moving Average (WMA) across `N` asset lanes per scheduling epoch.
struct WmaDriver {
    multipliers: (f64, f64, f64),
    period: usize,
    want_optional_outputs: bool,
}

impl Driver<State> for WmaDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::new(&states);

        let multiplier_simd = (
            Simd::splat(self.multipliers.0),
            Simd::splat(self.multipliers.1),
            Simd::splat(self.multipliers.2),
        );

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let (wma_line_ptr, sma_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, wma_line_ptr, sma_line_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.period..len).enumerate() {
            // Get new and old values using pre-computed pointers
            let (new_vals, old_vals) = crate::extract_simd_at_indices!(N, input_ptrs,
                new_vals @ i,
                old_vals @ j
            );

            let (wma, sma) = state.calc_simd(old_vals, new_vals, multiplier_simd);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                wma_line_ptr => wma
            );
            crate::store_simd_optional_outputs!(j, N,
                self.want_optional_outputs, sma_line_ptr => sma
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Weighted Moving Average (WMA) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[real]` for asset `i`.
/// * `options` - `options[0]` is the `period`.
/// * `optional_outputs` - `optional_outputs[0] = true` enables the optional `sma` output.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the WMA line for asset `i`,
/// `outputs[i][1]` is `sma` (empty unless requested), and
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    //let real: Vec<&[f64]> = (0..N).map(|i| inputs[i][0]).collect();
    let real: [&[f64]; N] = std::array::from_fn(|i| inputs[i][0]);
    //init ema, sliced inputs and multipliers
    let simd_state = SimdState::init_state(&real, period);
    let multipliers = multiplier(period);
    let states = simd_state.to_states();

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = false;
    for (i, state) in states.into_iter().enumerate() {
        let asset_inputs = vec![inputs[i][0]];
        let (wma_line, sma_line) = {
            let capacity = output_length(inputs[i][0].len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };
        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(sma_line);
        }
        let mut output_buffer = vec![wma_line, sma_line];

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
            period,
            period,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = WmaDriver {
        multipliers,
        period,
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut indicator_states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        indicator_states.push(IndicatorState::new(
            unsafe { inputs.get_unchecked(i).get_unchecked(0) },
            multipliers,
            state,
            period,
        ));
    }
    Ok((output_buffers, indicator_states))
}
