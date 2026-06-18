use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::cybercycle::{
    min_data, multiplier, output_length, validate_options as cc_validate_options, IndicatorState,
    State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::cybercycle_simd::SimdState;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver that advances the CyberCycle across `N` option-set lanes per epoch.
///
/// All N lanes share the same price input; each lane uses a different α coefficient.
/// Per-lane multipliers are assembled into SIMD vectors once per epoch from the
/// per-lane `options` provided by the `PrimeMover`.
struct CycleOptionDriver {
    want_trigger: bool,
}

impl Driver<State, (f64, f64, f64)> for CycleOptionDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(f64, f64, f64)>>,
    ) {
        let len = outputs[0][0].len();

        // Pack per-lane (coeff, d1, d2) into three SIMD vectors.
        let mults = {
            let mut m0 = [0.0_f64; N];
            let mut m1 = [0.0_f64; N];
            let mut m2 = [0.0_f64; N];
            for (lane, opt) in options.iter().enumerate() {
                if let Some(&(c, d, e)) = opt {
                    m0[lane] = c;
                    m1[lane] = d;
                    m2[lane] = e;
                }
            }
            (
                Simd::from_array(m0),
                Simd::from_array(m1),
                Simd::from_array(m2),
            )
        };

        // All N lanes share the same input; splat one value to all lanes each bar.
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let (cycle_ptrs, trigger_ptrs) =
            crate::extract_output_ptrs!(outputs, N, cycle_ptrs, trigger_ptrs);

        let mut simd_state = SimdState::new(&mut states);

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index_splat!(i, N, real @ real_ptrs);
            // Safety: all ring buffers are full — guaranteed by State::seed_warmup
            // called for every lane before PrimeMover dispatches.
            let cycle = unsafe { simd_state.calc_simd_unchecked(real, mults) };
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
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    // Use our custom alpha validator (common one rejects < 1.0).
    validate_options(options, Some(cc_validate_options))?;
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;

    let params: [(f64, f64, f64); N] = std::array::from_fn(|i| multiplier(options[i][0]));
    let want_trigger = optional_outputs
        .and_then(|f| f.first().copied())
        .unwrap_or(false);

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State, (f64, f64, f64)>::new();

    for i in 0..N {
        let capacity = output_length(inputs[0].len(), options[i]);

        let cycle_line = crate::uninit_vec!(f64, capacity);
        let trigger_line: Vec<f64> = if want_trigger {
            crate::uninit_vec!(f64, capacity)
        } else {
            Vec::new()
        };

        // Seed bars 0–5 (seeding formula, no bar-6 output).
        // The driver processes real[6..n] and writes output[0..capacity].
        let state = State::seed_warmup(inputs[0]);

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
            min_data(options[i]) - 1,
            0,
            state,
            Some(&params[i]),
        ));

        output_buffers.push(output_buffer);
    }

    let mut driver = CycleOptionDriver { want_trigger };
    let final_states = road_train.drive(&mut driver);

    let states = final_states
        .into_iter()
        .enumerate()
        .map(|(i, s)| IndicatorState::new(s, params[i]))
        .collect();
    Ok((output_buffers, states))
}
