//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::vidya_simd::SimdState;
use crate::indicators::stddev::output_length as stddev_output_length;
use crate::indicators::vidya::{
    min_data, multiplier, output_length, validate_options, IndicatorState, State, INPUTS_WIDTH,
    OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver that advances the Variable Index Dynamic Average (VIDYA) across `N` asset lanes per scheduling epoch.
struct VidyaDriver {
    multipliers: (f64, f64),
    periods: (usize, usize),
    want_optional_outputs: (bool, bool, bool, bool, bool),
    alpha: f64,
}

impl Driver<State> for VidyaDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let (short_period, long_period) = self.periods;
        let mut state = SimdState::new(&mut states);

        let (multipliers_simd, alpha) = (
            (
                Simd::splat(self.multipliers.0),
                Simd::splat(self.multipliers.1),
            ),
            Simd::splat(self.alpha),
        );
        let (has_optional, want_short_sma, want_long_sma, want_short_sd, want_long_sd) =
            self.want_optional_outputs;
        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (
            vidya_line_ptr,
            short_sma_line_ptr,
            long_sma_line_ptr,
            short_sd_line_ptr,
            long_sd_line_ptr,
        ) = crate::extract_output_ptrs!(
            outputs,
            N,
            vidya_line_ptr,
            short_sma_line_ptr,
            long_sma_line_ptr,
            short_sd_line_ptr,
            long_sd_line_ptr
        );

        // Optimized main loop with minimal overhead
        for (j, i) in (long_period..len).enumerate() {
            let (value, short_value, long_value) = crate::extract_simd_at_indices!(N, input_ptrs,
                value @ i,
                short_value @ i-short_period,
                long_value @ j
            );
            let (vidya, short_sma, long_sma, short_sd, long_sd) =
                state.calc_simd(value, short_value, long_value, alpha, multipliers_simd);

            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, j,
                vidya_line_ptr => vidya
            );

            if has_optional {
                crate::store_simd_optional_outputs!(j, N,
                    want_short_sma, short_sma_line_ptr => short_sma,
                    want_long_sma, long_sma_line_ptr => long_sma,
                    want_short_sd, short_sd_line_ptr => short_sd,
                    want_long_sd, long_sd_line_ptr => long_sd
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Variable Index Dynamic Average (VIDYA) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[real]` for asset `i`.
/// * `options` - `options[0]` is `short_period`, `options[1]` is `long_period`,
///   `options[2]` is `alpha`.
/// * `optional_outputs` - `optional_outputs[0] = true` enables `short_sma`,
///   `optional_outputs[1] = true` enables `long_sma`,
///   `optional_outputs[2] = true` enables `short_sdtdev`,
///   `optional_outputs[3] = true` enables `long_sdtdev`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the VIDYA line for asset `i`,
/// `outputs[i][1]` is `short_sma` (empty unless requested),
/// `outputs[i][2]` is `long_sma` (empty unless requested),
/// `outputs[i][3]` is `short_sdtdev` (empty unless requested),
/// `outputs[i][4]` is `long_sdtdev` (empty unless requested), and
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let short_period = options[0] as usize;
    let long_period = options[1] as usize;
    let alpha = options[2];
    let multipliers = multiplier(short_period, long_period);

    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, State>::new();
    let mut want_optional_outputs = (false, false, false, false, false);

    for i in 0..N {
        let len = inputs[i][0].len();
        let capacity = output_length(len, options);
        let (
            mut vidya_line,
            mut short_sma_line,
            mut long_sma_line,
            mut short_sd_line,
            mut long_sd_line,
        );
        {
            let short_capacity = stddev_output_length(len, &[short_period as f64]);
            let long_capacity = stddev_output_length(len, &[long_period as f64]);
            vidya_line = crate::uninit_vec!(f64, capacity);
            (short_sma_line, long_sma_line, short_sd_line, long_sd_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false, false],
                short_sma_line: short_capacity,
                long_sma_line: long_capacity,
                short_sd_line: short_capacity,
                long_sd_line: long_capacity
            );
        }

        let state = State::init_state(
            short_period,
            long_period,
            inputs[i][0],
            alpha,
            &mut vidya_line,
            (
                &mut short_sma_line,
                &mut long_sma_line,
                &mut short_sd_line,
                &mut long_sd_line,
            ),
        );

        let asset_inputs = vec![inputs[i][0]];
        let mut starts = [1; 5];
        (starts[1], starts[2], starts[3], starts[4]) = crate::slice_outputs_start!(
            capacity - 1,
            short_sma_line,
            long_sma_line,
            short_sd_line,
            long_sd_line
        ); //capacity - 1 because vidya_line recieve 1 output bar in init_state

        if i == 0 {
            want_optional_outputs =
                crate::calc_want_flags!(short_sma_line, long_sma_line, short_sd_line, long_sd_line);
        }
        let mut output_buffer = vec![
            vidya_line,
            short_sma_line,
            long_sma_line,
            short_sd_line,
            long_sd_line,
        ];
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
            long_period,
            long_period,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = VidyaDriver {
        multipliers,
        periods: (short_period, long_period),
        alpha,
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            inputs[i][0],
            state,
            (short_period, long_period),
            multipliers,
            alpha,
        ));
    }
    Ok((output_buffers, states))
}
