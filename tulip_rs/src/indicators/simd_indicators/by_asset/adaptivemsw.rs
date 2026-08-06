use crate::common_simd::assets::validate_inputs;
use crate::indicators::adaptivemsw::{
    AdaptiveMSW, Indicator, IndicatorState, State, INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::adaptivemsw_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver that advances the Adaptive MESA Sine Wave across `N` lanes per epoch.
struct AdaptiveMswDriver {
    /// `(has_optional, want_dc_period)`
    want_optional_outputs: (bool, bool),
}

impl Driver<State<Warm>> for AdaptiveMswDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let mut simd_state = SimdState::from_states(&mut states);

        let (has_optional, want_dc) = self.want_optional_outputs;

        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let (sine_ptrs, lead_ptrs, dc_period_ptrs) =
            crate::extract_output_ptrs!(outputs, N, sine_ptrs, lead_ptrs, dc_period_ptrs);

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index!(i, N, real @ real_ptrs);
            let (sine, lead) = simd_state.calc(real);
            crate::write_simd_at_indices!(N, i,
                sine_ptrs => sine,
                lead_ptrs => lead
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_dc, dc_period_ptrs => simd_state.hd.smooth_period
                );
            }
        }

        simd_state.write_states(&mut states);
    }
}

/// Calculates the Adaptive MESA Sine Wave for `N` assets simultaneously.
///
/// Each asset is warmed up via [`State::init_state`] (23 bars, writing index 0),
/// then all assets are batched by [`PrimeMover`] through the SIMD HD + per-lane
/// DFT pipeline from bar 23 onwards.
///
/// # Arguments
/// * `inputs`   — `N` asset input sets; `inputs[i][0]` = real price series for asset `i`.
/// * `options`  — Unused; pass `&[]`.
/// * `optional_outputs` — index `0` = `dc_period`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` = sine, `[1]` = lead_sine,
/// `[2]` = dc_period (empty unless requested).
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N],
    options: &[f64; OPTIONS],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, AdaptiveMSW::min_data(options))?;

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut want_optional_outputs = (false, false);

    for i in 0..N {
        let len = inputs[i][0].len();
        let capacity = AdaptiveMSW::output_length(len, options);

        let (mut sine_line, mut lead_line) = (
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
        );
        let mut dc_period_line = crate::init_optional_outputs!(
            optional_outputs, &[false],
            dc_period_line: capacity
        );

        let state = State::init_state(
            inputs[i][0],
            &mut sine_line,
            &mut lead_line,
            &mut dc_period_line,
        );

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(dc_period_line);
        }

        let mut output_buffer = vec![sine_line, lead_line, dc_period_line];
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
            AdaptiveMSW::min_data(options),
            0,
            state,
            None,
        ));

        output_buffers.push(output_buffer);
    }

    let mut driver = AdaptiveMswDriver {
        want_optional_outputs,
    };
    let final_states = road_train.drive(&mut driver);

    let states: Vec<IndicatorState> = final_states;
    Ok((output_buffers, states))
}
