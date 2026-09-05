use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::cybercycle::{
    validate_options as cc_validate_options, Cybercycle, Indicator, IndicatorState, State, INPUTS,
    OPTIONS,
};
use crate::indicators::simd_indicators::cybercycle_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver that advances the CyberCycle across `N` option-set lanes per epoch.
///
/// All N lanes share the same price input; each lane uses a different α coefficient.
/// Per-lane coefficients are embedded in each scalar [`State`] (via [`State::seed_warmup`])
/// and gathered into the [`SimdState`] by [`TSimdState::from_states`] — no external
/// multiplier vectors are needed here.
struct CycleOptionDriver {
    want_trigger: bool,
}

impl Driver<State<Warm>> for CycleOptionDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = outputs[0][0].len();

        // All N lanes share the same input; splat one value to all lanes each bar.
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let (cycle_ptrs, trigger_ptrs) =
            crate::extract_output_ptrs!(outputs, N, cycle_ptrs, trigger_ptrs);

        // Coefficients are gathered from each lane's scalar State — no external mults needed.
        let mut simd_state = SimdState::<N>::from_states(&mut states);

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index_splat!(i, N, real @ real_ptrs);
            // Safety: all ring buffers are full — guaranteed by State::seed_warmup
            // called for every lane before PrimeMover dispatches.
            let cycle = simd_state.calc(real);

            crate::write_simd_at_indices!(N, i, cycle_ptrs => cycle);

            crate::store_simd_optional_outputs!(i, N,
                self.want_trigger, trigger_ptrs => simd_state.cycle_prev2
            );
        }

        simd_state.write_states(&mut states);
    }
}

/// Calculates the Ehlers CyberCycle on a single asset with `N` different α values
/// simultaneously using SIMD parallelism.
///
/// All N lanes process the same `inputs[0]` price series; each lane applies its own
/// `alpha` coefficient. This is ideal for scanning multiple parameter values in a
/// single SIMD pass (e.g., hyper-parameter search or ensemble signals).
///
/// # Arguments
///
/// * `inputs`           — `[&[f64]; 1]` containing `[real]` (shared across all N lanes).
/// * `options`          — Array of N option sets `[alpha; 1]`, one per SIMD lane.
/// * `optional_outputs` — index `0` = `trigger`.
///
/// # Returns
///
/// `Ok((outputs, states))` where `outputs[i][0]` = cybercycle and `outputs[i][1]` =
/// trigger (empty unless requested) for option set `i`, and `states[i]` is the final
/// [`IndicatorState`] for lane `i`. Returns `Err(NotEnoughData)` if the input is
/// shorter than 7 bars, or `Err(InvalidOptions)` if any α is not in `(0, 1)`.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    // Use our custom alpha validator (common one rejects < 1.0).
    validate_options(options, Some(cc_validate_options))?;
    validate_inputs::<OPTIONS>(inputs, options, Cybercycle::min_data)?;

    let want_trigger = optional_outputs
        .and_then(|f| f.first().copied())
        .unwrap_or(false);

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State<Warm>>::new();

    for i in 0..N {
        let alpha = options[i][0];
        let capacity = Cybercycle::output_length(inputs[0].len(), options[i]);

        let cycle_line = crate::uninit_vec!(f64, capacity);
        let trigger_line: Vec<f64> = if want_trigger {
            crate::uninit_vec!(f64, capacity)
        } else {
            Vec::new()
        };

        // seed_warmup seeds bars 0–5 and embeds coef/d1/d2 from alpha into the State.
        // The driver processes real[6..n] and writes output[0..capacity].
        let state = State::seed_warmup(inputs[0], alpha);

        let mut output_buffer = vec![cycle_line, trigger_line];
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());
        for j in 0..output_buffer.len() {
            unsafe {
                let buf = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.len()));
            }
        }

        road_train.add_asset(Asset::new(
            vec![inputs[0]],
            asset_outputs,
            i,
            // seed_warmup covered bars 0..5; driver starts at bar 6 = min_data - 1.
            Cybercycle::min_data(options[i]) - 1,
            0,
            state,
            None,
        ));

        output_buffers.push(output_buffer);
    }

    let mut driver = CycleOptionDriver { want_trigger };
    let final_states = road_train.drive(&mut driver);

    Ok((output_buffers, final_states))
}
