use crate::common_simd::assets::validate_inputs;
use crate::indicators::mama::{
    Mama, Indicator, validate_options, IndicatorState, State, INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::mama_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver that advances MAMA / FAMA across `N` asset lanes per scheduling epoch.
struct MamaDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State<Warm>> for MamaDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Gathers per-asset states into a [`SimdState`], runs the full HD + MAMA pipeline
    /// for every bar in the epoch, writes `mama` and `fama` (and optionally
    /// `dc_period` / `alpha`) for each asset, then scatters the updated state back.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let mut simd_state = SimdState::from_states(&mut states);

        let (has_optional, want_dc, want_alpha) = self.want_optional_outputs;

        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let (mama_line_ptr, fama_line_ptr, dc_period_line_ptr, alpha_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            mama_line_ptr,
            fama_line_ptr,
            dc_period_line_ptr,
            alpha_line_ptr
        );

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index!(i, N, real @ real_ptrs);
            let (mama, fama) = simd_state.calc(real);
            crate::write_simd_at_indices!(N, i,
                mama_line_ptr => mama,
                fama_line_ptr => fama
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_dc,    dc_period_line_ptr => simd_state.hd.smooth_period,
                    want_alpha, alpha_line_ptr     => simd_state.alpha
                );
            }
        }

        simd_state.write_states(&mut states);
    }
}

/// Calculates MAMA and FAMA for `N` assets simultaneously using SIMD parallelism.
///
/// Each asset's state is independently warmed up via [`State::init_state`] (consuming
/// the first 23 bars, writing bar 22's output to index 0), then all assets are batched
/// by the [`PrimeMover`] scheduler and advanced together through the SIMD pipeline
/// starting at bar 23.
///
/// # Arguments
/// * `inputs` — `N` asset input sets; `inputs[i]` is `[&[f64]; 1]` containing `[real]`
///   for asset `i`.
/// * `options` — Shared options: `[fast_limit, slow_limit]`.
/// * `optional_outputs` — Optional flags: index `0` = `dc_period`, index `1` = `alpha`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `mama`, `outputs[i][1]` is `fama`,
/// `outputs[i][2]` is `dc_period` (empty unless requested), `outputs[i][3]` is `alpha`
/// (empty unless requested), and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError::NotEnoughData)` if any input is shorter than
/// [`min_data`] (23 bars), or `Err(IndicatorError::InvalidOptions)` if options are invalid.
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N],
    options: &[f64; OPTIONS],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Mama::min_data(options))?;
    validate_options(options)?;

    let fast_limit = options[0];
    let slow_limit = options[1];

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut want_optional_outputs = (false, false, false);

    for i in 0..N {
        let len = inputs[i][0].len();
        let capacity = Mama::output_length(len, options);

        let mut mama_line = crate::uninit_vec!(f64, capacity);
        let mut fama_line = crate::uninit_vec!(f64, capacity);

        let (mut dc_period_line, mut alpha_line) = crate::init_optional_outputs!(
            optional_outputs, &[false, false],
            dc_period_line: capacity,
            alpha_line: capacity
        );

        // init_state warms up the HD pipeline, seeds MAMA/FAMA from the first valid bar's price,
        // processes bar min_data−1 = 22 (0-indexed), and writes outputs[0] for all series.
        let state = State::init_state(
            inputs[i][0],
            fast_limit,
            slow_limit,
            &mut mama_line,
            &mut fama_line,
            &mut dc_period_line,
            &mut alpha_line,
        );

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(dc_period_line, alpha_line);
        }

        // init_state wrote index 0; the driver writes indices 1..capacity.
        // Pass slices starting at index 1 to the road_train.
        let mut output_buffer = vec![mama_line, fama_line, dc_period_line, alpha_line];
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());
        for j in 0..output_buffer.len() {
            unsafe {
                let buf = &mut output_buffer[j];
                let len = buf.len();
                // Start from index 1: init_state already wrote index 0.
                let start = if len > 0 { 1 } else { 0 };
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    buf.as_mut_ptr().add(start),
                    len.saturating_sub(start),
                ));
            }
        }

        road_train.add_asset(Asset::new(
            vec![inputs[i][0]],
            asset_outputs,
            i,
            // init_state consumed bars 0..22 (inclusive), so the driver starts at bar 23 = min_data.
            Mama::min_data(options),
            0,
            state,
            None,
        ));

        output_buffers.push(output_buffer);
    }

    let mut driver = MamaDriver {
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
