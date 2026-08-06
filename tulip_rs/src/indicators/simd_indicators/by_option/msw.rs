use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicator_types::Indicator;
use crate::indicator_types::TSimdState;
use crate::indicators::msw::{
    dot_product_simd, multiplier, precompute_twiddles, IndicatorState, Msw, State, INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::msw_simd::options::calc_sdft;
use crate::indicators::simd_indicators::msw_simd::SimdState;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver for the Mesa Sine Wave (MSW) indicator, processing `N` option-set lanes per
/// scheduling epoch using a vectorized Sliding DFT (SDFT) recurrence — O(1) per bar.
struct MswDriver;

impl Driver<State, (usize, f64)> for MswDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(usize, f64)>>,
    ) {
        let len = outputs[0][0].len();

        // Gather per-lane SDFT accumulators and rotation phasors into a single SimdState.
        let mut simd_state = SimdState::from_states(&mut states);

        // i[lane] = period[lane]; increments by 1 each bar (mirrors SMA driver).
        let mut i = {
            let mut arr = [0usize; N];
            for (lane, opt) in options.iter().enumerate() {
                if let Some(&(period, _)) = opt {
                    arr[lane] = period;
                }
            }
            arr
        };

        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        let (sine_line_ptr, lead_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, sine_line_ptr, lead_line_ptr);

        for j in 0..len {
            // old_sample: bar leaving the window — uniform index j across all lanes.
            let old_sample = crate::extract_simd_inputs_at_index!(j, N, old @ real_ptrs);
            // new_sample: bar entering the window — per-lane index i[lane].
            let new_sample = crate::extract_simd_inputs_at_index_array!(i, N, new @ real_ptrs);

            // Vectorized SDFT update — O(1) per bar, no per-bar trig.
            let (sine, lead) = calc_sdft(&mut simd_state, new_sample, old_sample);

            crate::write_simd_at_indices!(N, j,
                sine_line_ptr => sine,
                lead_line_ptr => lead
            );

            for idx in i.iter_mut() {
                *idx += 1;
            }
        }

        // Scatter updated accumulators back to per-lane states.
        simd_state.write_states(&mut states);
    }
}

/// Calculates the Mesa Sine Wave (MSW) indicator for one asset with `N` different option sets
/// simultaneously using a vectorized Sliding DFT (SDFT) — O(1) per bar after seeding.
///
/// Applies each of the `N` period configurations to the same shared input series, computing
/// sine and lead wave lines for all option sets in a single SIMD-accelerated pass via
/// [`PrimeMover`]. Each lane's SDFT is seeded once upfront with a full DFT over the first
/// `period` bars (O(period) trig, one-time cost), then the hot loop runs at O(1) per bar.
///
/// # Arguments
/// * `inputs` - Shared input: `inputs[0]` is the `real` price series.
/// * `options` - An array of `N` option sets; `options[i][0]` is the `period` for lane `i`.
/// * `_optional_outputs` - Unused; MSW has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the `msw_sine` series and
/// `outputs[i][1]` is the `msw_lead` series for option set `i`, and `states[i]` is
/// the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Msw::min_data)?;
    validate_options(options, None)?;
    let real = inputs[0];

    let params: [(usize, f64); N] = std::array::from_fn(|i| {
        let period = options[i][0] as usize;
        (period, multiplier(period))
    });

    let mut road_train = PrimeMover::<N, State, (usize, f64)>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for (i, &(period, mult)) in params.iter().enumerate() {
        // Seed the SDFT once with a full DFT over the first `period` bars.
        let (cos_tw, sin_tw) = precompute_twiddles(period, mult);
        let (rp, ip) = dot_product_simd::<8>(&real[..period], &cos_tw, &sin_tw);
        let state = State {
            rp,
            ip,
            ..State::new(mult)
        };

        let capacity = Msw::output_length(real.len(), options[i]);
        let sine_line = crate::uninit_vec!(f64, capacity);
        let lead_line = crate::uninit_vec!(f64, capacity);
        let mut output_buffer = vec![sine_line, lead_line];

        let mut asset_outputs = Vec::with_capacity(output_buffer.len());
        for j in 0..output_buffer.len() {
            unsafe {
                let buf = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.len()));
            }
        }

        road_train.add_asset(Asset::new(
            vec![real],
            asset_outputs,
            i,
            period, // inputs_idx — road_train starts reading from here
            period, // start_offset — lookback bars prepended to each slice
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = MswDriver;
    road_train.drive(&mut driver);

    let states = params
        .iter()
        .map(|&(period, mult)| IndicatorState::new(real, period, mult))
        .collect();

    Ok((output_buffers, states))
}
