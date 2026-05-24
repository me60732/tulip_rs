//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::vosc_simd::SimdState;
use crate::indicators::{
    sma::output_length as sma_output_length,
    vosc::{
        min_data, multiplier, output_length, validate_options, IndicatorState, State, INPUTS_WIDTH,
        OPTIONS_WIDTH,
    },
};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver that advances the Volume Oscillator (VOSC) across `N` asset lanes per scheduling epoch.
struct VoscDriver {
    multipliers: (f64, f64),
    long_period: usize,
    short_period: usize,
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State> for VoscDriver {
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
        let (has_optional, want_short_sma, want_long_sma) = self.want_optional_outputs;
        let short_multiplier = Simd::splat(self.multipliers.0);
        let long_multiplier = Simd::splat(self.multipliers.1);

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (vosc_line_ptr, short_sma_line_ptr, long_sma_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            vosc_line_ptr,
            short_sma_line_ptr,
            long_sma_line_ptr
        );

        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.long_period..len).enumerate() {
            let vols = crate::extract_simd_at_indices!(N, input_ptrs,
                volume @ i,
                short_volume @ i-self.short_period,
                long_volume @ j
            );

            let (vosc, short_sma, long_sma) =
                state.calc_simd(vols, short_multiplier, long_multiplier);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                vosc_line_ptr => vosc
            );

            if has_optional {
                crate::store_simd_optional_outputs!(j, N,
                    want_short_sma, short_sma_line_ptr => short_sma,
                    want_long_sma, long_sma_line_ptr => long_sma
                );
            }
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Volume Oscillator (VOSC) for `N` assets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[volume]` for asset `i`.
/// * `options` - `options[0]` is `short_period`, `options[1]` is `long_period`.
/// * `optional_outputs` - `optional_outputs[0] = true` enables `short_sma`,
///   `optional_outputs[1] = true` enables `long_sma`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the VOSC line for asset `i`,
/// `outputs[i][1]` is `short_sma` (empty unless requested),
/// `outputs[i][2]` is `long_sma` (empty unless requested), and
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

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = (false, false, false);
    let multipliers = multiplier(short_period, long_period);

    for i in 0..inputs.len() {
        let asset_inputs = vec![inputs[i][0]];
        let (vosc_line, (mut short_sma_line, long_sma_line)) = {
            let len = inputs[i][0].len();
            let capacity = output_length(len, options);
            let short_capacity = sma_output_length(len, &[short_period as f64]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    short_sma_line: short_capacity,
                    long_sma_line: capacity
                ),
            )
        };

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(short_sma_line, long_sma_line);
        }
        let mut starts = [0; N];
        starts[1] = crate::slice_outputs_start!(vosc_line.len(), short_sma_line);

        let state = State::init_state(short_period, long_period, inputs[i][0], &mut short_sma_line);

        let mut output_buffer = vec![vosc_line, short_sma_line, long_sma_line];

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
    let mut driver = VoscDriver {
        long_period,
        short_period,
        multipliers,
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    let mut indicator_states = Vec::with_capacity(N);
    for (i, state) in states.into_iter().enumerate() {
        indicator_states.push(IndicatorState::new(
            inputs[i][0],
            state,
            multipliers,
            (short_period, long_period),
        ));
    }
    Ok((output_buffers, indicator_states))
}
