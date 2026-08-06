use crate::common_simd::assets::validate_inputs;
use crate::indicators::instantaneoustrendline::{
    InstantaneousTrendline as IT, Indicator, IndicatorState, State, INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::instantaneoustrendline_simd::{
    SimdState, TSimdState, TState,
};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver that advances the Instantaneous Trendline across `N` asset lanes per epoch.
struct ItDriver {
    /// `(has_optional, want_trigger, want_dc_period, want_alpha)`
    want_optional_outputs: (bool, bool, bool, bool),
}

impl Driver<State<Warm>> for ItDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let mut simd_state = SimdState::from_states(&mut states);

        let (has_optional, want_trigger, want_dc, want_alpha) = self.want_optional_outputs;

        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let (trendline_ptrs, trigger_ptrs, dc_period_ptrs, alpha_ptrs) = crate::extract_output_ptrs!(
            outputs,
            N,
            trendline_ptrs,
            trigger_ptrs,
            dc_period_ptrs,
            alpha_ptrs
        );

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index!(i, N, real @ real_ptrs);
            let it = simd_state.calc(real);
            crate::write_simd_at_indices!(N, i, trendline_ptrs => it);
            if has_optional {
                // trigger = 2·IT − IT[1] = 2·it_prev − it_prev2 (after state update)
                let trigger = Simd::splat(2.0_f64) * simd_state.it_prev - simd_state.it_prev2;
                crate::store_simd_optional_outputs!(i, N,
                    want_trigger, trigger_ptrs    => trigger,
                    want_dc,      dc_period_ptrs  => simd_state.hd.smooth_period,
                    want_alpha,   alpha_ptrs       => simd_state.alpha
                );
            }
        }

        simd_state.write_states(&mut states);
    }
}

/// Calculates the Ehlers Instantaneous Trendline for `N` assets simultaneously using
/// SIMD parallelism.
///
/// Each asset's state is independently warmed up via [`State::init_state`] (consuming
/// the first 23 bars, writing bar 22's output to index 0), then all assets are batched
/// by the [`PrimeMover`] scheduler and advanced together through the SIMD pipeline
/// starting at bar 23.
///
/// # Arguments
/// * `inputs` — `N` asset input sets; `inputs[i]` is `[&[f64]; 1]` containing `[real]`
///   for asset `i`.
/// * `options` — Unused; the IT has no configurable parameters. Pass `&[]`.
/// * `optional_outputs` — Optional flags: index `0` = `trigger`, `1` = `dc_period`,
///   `2` = `alpha`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `trendline`, `outputs[i][1]` is
/// `trigger` (empty unless requested), `outputs[i][2]` is `dc_period`, `outputs[i][3]`
/// is `alpha`, and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError::NotEnoughData)` if any input is shorter than 23 bars.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N],
    options: &[f64; OPTIONS],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, IT::min_data(options))?;

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut want_optional_outputs = (false, false, false, false);

    for i in 0..N {
        let len = inputs[i][0].len();
        let capacity = IT::output_length(len, options);

        let mut trendline_line = crate::uninit_vec!(f64, capacity);
        let (mut trigger_line, mut dc_period_line, mut alpha_line) = crate::init_optional_outputs!(
            optional_outputs, &[false, false, false],
            trigger_line: capacity,
            dc_period_line: capacity,
            alpha_line: capacity
        );

        // init_state warms up the HD pipeline, seeds IIR from seeding formula,
        // processes bar min_data−1 = 22 (0-indexed), and writes outputs[0].
        let state = State::init_state(
            inputs[i][0],
            &mut trendline_line,
            &mut trigger_line,
            &mut dc_period_line,
            &mut alpha_line,
        );

        if i == 0 {
            want_optional_outputs =
                crate::calc_want_flags!(trigger_line, dc_period_line, alpha_line);
        }

        // init_state wrote index 0; the driver writes indices 1..capacity.
        let mut output_buffer = vec![trendline_line, trigger_line, dc_period_line, alpha_line];
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());
        for j in 0..output_buffer.len() {
            unsafe {
                let buf = &mut output_buffer[j];
                let buf_len = buf.len();
                let start = if buf_len > 0 { 1 } else { 0 };
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    buf.as_mut_ptr().add(start),
                    buf_len.saturating_sub(start),
                ));
            }
        }

        road_train.add_asset(Asset::new(
            vec![inputs[i][0]],
            asset_outputs,
            i,
            // init_state consumed bars 0..22 inclusive, so driver starts at bar 23 = min_data.
            IT::min_data(options),
            0,
            state,
            None,
        ));

        output_buffers.push(output_buffer);
    }

    let mut driver = ItDriver {
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
