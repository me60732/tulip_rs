use crate::common_simd::assets::validate_inputs;
use crate::indicator_types::Indicator;
use crate::indicators::ccfisher::{
    validate_options, CcFisher, IndicatorState, State, INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::ccfisher_simd::{assets::SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver that advances the CyberCycle Fisher across `N` asset lanes per epoch.
struct CCFisherDriver {
    /// Whether any of cycle or peak optional outputs was requested.
    has_optional: bool,
    /// Whether the trendmode optional output was requested.
    want_trendmode: bool,
    /// Whether the cycle optional output was requested.
    want_cycle: bool,
    /// Whether the peak optional output was requested.
    want_peak: bool,
}

impl Driver<State<Warm>> for CCFisherDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let mut simd_state = SimdState::from_states(&mut states);

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

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index!(i, N, real @ real_ptrs);
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

/// Calculates the Ehlers CyberCycle Fisher for `N` assets simultaneously using SIMD.
///
/// Each asset's state is independently warmed up via [`State::init_state`]
/// (consuming bars 0–55, writing output index 0), then all assets are batched
/// by the [`PrimeMover`] scheduler starting at bar 56.
///
/// # Arguments
///
/// * `inputs`           — `N` asset input sets; `inputs[i]` is `[&[f64]; 1]` = `[real]`.
/// * `options`          — `[alpha; 1]`; same value applied to all N assets.
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
/// Returns `Err(NotEnoughData)` if any asset has fewer than 56 bars, or
/// `Err(InvalidOptions)` if `alpha` is not in `(0, 1)`.
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N],
    options: &[f64; OPTIONS],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_options(options)?;
    validate_inputs::<INPUTS>(inputs, CcFisher::min_data(options))?;

    let alpha = options[0];
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
    let mut road_train = PrimeMover::<N, State<Warm>>::new();

    for i in 0..N {
        let len = inputs[i][0].len();
        let capacity = CcFisher::output_length(len, options);

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

        // init_state seeds bars 0–54 and processes bar 55 (output index 0).
        let state = State::init_state(
            inputs[i][0],
            alpha,
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
            vec![inputs[i][0]],
            asset_outputs,
            i,
            // init_state consumed bars 0..55 inclusive; driver starts at bar 56 = min_data.
            CcFisher::min_data(options),
            0,
            state,
            None,
        ));

        output_buffers.push(output_buffer);
    }

    let mut driver = CCFisherDriver {
        has_optional,
        want_trendmode,
        want_cycle,
        want_peak,
    };
    let final_states = road_train.drive(&mut driver);

    let states: Vec<IndicatorState> = final_states;
    Ok((output_buffers, states))
}
