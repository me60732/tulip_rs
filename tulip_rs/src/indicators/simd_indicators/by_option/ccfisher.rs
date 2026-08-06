use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicator_types::Indicator;
use crate::indicators::ccfisher::{
    validate_options as cf_validate_options, CcFisher, IndicatorState, State, INPUTS, OPTIONS,
};

use crate::indicators::simd_indicators::ccfisher_simd::{options::SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};

/// SIMD driver that advances the CyberCycle Fisher across `N` option-set lanes per epoch.
///
/// All N lanes share the same price input; each lane uses a different α coefficient.
/// The scalar HD runs once per bar (shared DC period); CC runs in SIMD with
/// per-lane multipliers assembled from the `options` provided by PrimeMover.
struct CCFisherOptionDriver {
    /// Whether any of cycle or peak optional outputs was requested.
    has_optional: bool,
    /// Whether the trendmode optional output was requested.
    want_trendmode: bool,
    /// Whether the cycle optional output was requested.
    want_cycle: bool,
    /// Whether the peak optional output was requested.
    want_peak: bool,
}

impl Driver<State<Warm>, f64> for CCFisherOptionDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&f64>>,
    ) {
        use std::simd::Simd;

        let len = outputs[0][0].len();

        // All N lanes share the same input; read the scalar price from lane 0.
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let (fisher_ptrs, signal_ptrs, trendmode_ptrs, cycle_ptrs, peak_ptrs) = crate::extract_output_ptrs!(
            outputs,
            N,
            fisher_ptrs,
            signal_ptrs,
            trendmode_ptrs,
            cycle_ptrs,
            peak_ptrs
        );

        let mut simd_state: SimdState<N> = SimdState::from_states(&mut states);

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index_splat!(i, N, real @ real_ptrs);
            let (fisher, signal) = simd_state.calc(real);
            crate::write_simd_at_indices!(N, i, fisher_ptrs => fisher, signal_ptrs => signal);

            if self.want_trendmode {
                let cycle_arr = simd_state.cc.cycle_prev.to_array();
                let mut trendmode_arr = [0.0_f64; N];
                for j in 0..N {
                    trendmode_arr[j] =
                        if simd_state.pk[j] > 0.0 && cycle_arr[j].abs() < 0.2 * simd_state.pk[j] {
                            1.0
                        } else {
                            0.0
                        };
                }
                crate::write_simd_at_indices!(N, i, trendmode_ptrs => trendmode_arr);
            }
            if self.has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    self.want_cycle, cycle_ptrs => simd_state.cc.cycle_prev,
                    self.want_peak,  peak_ptrs  => simd_state.pk
                );
            }
        }

        simd_state.write_states(&mut states);
    }
}

/// Calculates the Ehlers CyberCycle Fisher on a single asset with `N` different α values
/// simultaneously using SIMD parallelism.
///
/// All N lanes process the same `inputs[0]` price series; each lane applies its own
/// `alpha` coefficient. Each lane is independently warmed up via [`State::init_state`]
/// (consuming bars 0–55, writing output index 0), then the SIMD driver processes
/// bars 56..n writing output indices 1..capacity.
///
/// # Arguments
///
/// * `inputs`           — `[&[f64]; 1]` containing `[real]` (shared across all N lanes).
/// * `options`          — Array of N option sets `[alpha; 1]`, one per SIMD lane.
/// * `optional_outputs` — index `0` = `trendmode`, index `1` = `cycle`, index `2` = `peak`.
///
/// # Returns
///
/// `Ok((outputs, states))` where:
/// - `outputs[i][0]` = fisher (always present)
/// - `outputs[i][1]` = signal (always present)
/// - `outputs[i][2]` = trendmode (empty unless requested)
/// - `outputs[i][3]` = cycle (empty unless requested)
/// - `outputs[i][4]` = peak (empty unless requested)
///
/// Returns `Err(NotEnoughData)` if the input is shorter than 56 bars, or
/// `Err(InvalidOptions)` if any α is not in `(0, 1)`.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    // Delegate alpha validation to ccfisher's custom validator.
    validate_options(options, Some(cf_validate_options))?;
    validate_inputs::<OPTIONS>(inputs, options, CcFisher::min_data)?;

    let alphas: [f64; N] = std::array::from_fn(|i| options[i][0]);
    let want_trendmode = optional_outputs
        .and_then(|f| f.first().copied())
        .unwrap_or(false);
    let want_cycle = optional_outputs
        .and_then(|f| f.get(1).copied())
        .unwrap_or(false);
    let want_peak = optional_outputs
        .and_then(|f| f.get(2).copied())
        .unwrap_or(false);
    let has_optional = want_cycle || want_peak;

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State<Warm>, f64>::new();

    for i in 0..N {
        let capacity = CcFisher::output_length(inputs[0].len(), options[i]);

        let mut fisher_line = crate::uninit_vec!(f64, capacity);
        let mut signal_line = crate::uninit_vec!(f64, capacity);
        let mut trendmode_line: Vec<f64> = if want_trendmode {
            crate::uninit_vec!(f64, capacity)
        } else {
            Vec::new()
        };
        let mut cycle_line: Vec<f64> = if want_cycle {
            crate::uninit_vec!(f64, capacity)
        } else {
            Vec::new()
        };
        let mut peak_line: Vec<f64> = if want_peak {
            crate::uninit_vec!(f64, capacity)
        } else {
            Vec::new()
        };

        // Each lane runs its full warmup independently (bars 0–54 + bar 55 output).
        let state = State::init_state(
            inputs[0],
            options[i][0],
            &mut fisher_line,
            &mut signal_line,
            &mut trendmode_line,
            &mut cycle_line,
            &mut peak_line,
        );

        // Slice outputs so the driver writes indices 1..capacity.
        let mut output_buffer = vec![
            fisher_line,
            signal_line,
            trendmode_line,
            cycle_line,
            peak_line,
        ];
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
            vec![inputs[0]],
            asset_outputs,
            i,
            // init_state consumed bars 0..55 inclusive; driver starts at bar 56 = min_data.
            CcFisher::min_data(options[i]),
            0,
            state,
            Some(&alphas[i]),
        ));

        output_buffers.push(output_buffer);
    }

    let mut driver = CCFisherOptionDriver {
        has_optional,
        want_trendmode,
        want_cycle,
        want_peak,
    };
    let final_states = road_train.drive(&mut driver);

    let states: Vec<IndicatorState> = final_states;
    Ok((output_buffers, states))
}
