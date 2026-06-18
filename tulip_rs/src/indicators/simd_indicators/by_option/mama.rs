use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::mama::{
    min_data, output_length, validate_options as vo, IndicatorState, State, INPUTS_WIDTH,
    OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::mama_simd::SimdState;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver for MAMA / FAMA, processing `N` option-set lanes per scheduling epoch.
struct MamaDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State, (f64, f64)> for MamaDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    ///
    /// Gathers per-lane `fast_limit` and `slow_limit` from the options into SIMD vectors,
    /// then runs the full HD + MAMA pipeline for every bar.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(f64, f64)>>,
    ) {
        let len = outputs[0][0].len();
        let mut simd_state = SimdState::new(&mut states);

        let (fast_limits, slow_limits) = {
            let mut fast = [0.0_f64; N];
            let mut slow = [0.0_f64; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&(fl, sl)) = option {
                    fast[lane] = fl;
                    slow[lane] = sl;
                }
            }
            (Simd::from_array(fast), Simd::from_array(slow))
        };
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
            let real = crate::extract_simd_inputs_at_index_splat!(i, N, real @ real_ptrs);
            // Safety: all ring buffers are full — guaranteed by State::init_state
            // called during indicator_by_options setup, before PrimeMover dispatches.
            let (mama, fama) =
                unsafe { simd_state.calc_simd_unchecked(real, fast_limits, slow_limits) };
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

/// Calculates MAMA and FAMA on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// Each lane's state is independently warmed up via [`State::init_state`] (consuming
/// the first 23 bars, writing bar 22's output to index 0), then all `N` lanes are batched
/// by the [`PrimeMover`] scheduler and advanced together through the SIMD pipeline.
///
/// # Arguments
/// * `inputs` — The single asset's price series (`[&[f64]; INPUTS_WIDTH]`).
/// * `options` — An array of `N` option sets, one per SIMD lane: `[fast_limit, slow_limit]`.
/// * `optional_outputs` — Optional flags: index `0` = `dc_period`, index `1` = `alpha`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains
/// `[mama_line, fama_line, dc_period_line?, alpha_line?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or any option set is invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, Some(vo))?;

    let params: [(f64, f64); N] = std::array::from_fn(|i| (options[i][0], options[i][1]));

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State, (f64, f64)>::new();
    let mut want_optional_outputs = (false, false, false);

    for i in 0..N {
        let (fast_limit, slow_limit) = params[i];
        let len = inputs[0].len();
        let capacity = output_length(len, options[i]);

        let mut mama_line = crate::uninit_vec!(f64, capacity);
        let mut fama_line = crate::uninit_vec!(f64, capacity);

        let (mut dc_period_line, mut alpha_line) = crate::init_optional_outputs!(
            optional_outputs, &[false, false],
            dc_period_line: capacity,
            alpha_line: capacity
        );

        // init_state warms up the HD pipeline, seeds MAMA/FAMA, processes bar 22, and
        // writes outputs[0] for all series.
        let state = State::init_state(
            inputs[0],
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
        let mut output_buffer = vec![mama_line, fama_line, dc_period_line, alpha_line];
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());
        for j in 0..output_buffer.len() {
            unsafe {
                let buf = &mut output_buffer[j];
                let blen = buf.len();
                let start = if blen > 0 { 1 } else { 0 };
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    buf.as_mut_ptr().add(start),
                    blen.saturating_sub(start),
                ));
            }
        }

        road_train.add_asset(Asset::new(
            vec![inputs[0]],
            asset_outputs,
            i,
            // init_state consumed bars 0..22 (inclusive); driver starts at bar 23 = min_data.
            min_data(options[i]),
            0,
            state,
            Some(&params[i]),
        ));

        output_buffers.push(output_buffer);
    }

    let mut driver = MamaDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let states = states_vec
        .into_iter()
        .zip(params.iter())
        .map(|(s, &(fl, sl))| IndicatorState::new(s, fl, sl))
        .collect();
    Ok((output_buffers, states))
}
