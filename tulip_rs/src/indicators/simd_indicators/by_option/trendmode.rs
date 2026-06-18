use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::cybercycle::multiplier;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::trendmode_simd::options::SimdState;
use crate::indicators::trendmode::{
    min_data, output_length, validate_options as tm_validate_options, IndicatorState, State,
    INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver that advances the TrendMode across `N` option-set lanes per epoch.
///
/// All N lanes share the same price input; each lane uses a different α coefficient.
/// The scalar HD runs once per bar (shared DC period); CC runs in SIMD with
/// per-lane multipliers assembled from the `options` provided by PrimeMover.
struct TrendModeOptionDriver {
    /// Whether any optional output was requested.
    has_optional: bool,
    /// Whether the cycle optional output was requested.
    want_cycle: bool,
    /// Whether the peak optional output was requested.
    want_peak: bool,
}

impl Driver<State, (f64, f64, f64)> for TrendModeOptionDriver {
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

        // All N lanes share the same input; read the scalar price from lane 0.
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let (trendmode_ptrs, cycle_ptrs, peak_ptrs) =
            crate::extract_output_ptrs!(outputs, N, trendmode_ptrs, cycle_ptrs, peak_ptrs);

        let mut simd_state = SimdState::new(&mut states);

        for i in 0..len {
            // Safety: all HD and CC ring buffers are full — guaranteed by
            // State::init_state called for every lane before PrimeMover dispatches.
            let price = unsafe { *real_ptrs[0].add(i) };
            let trendmode = unsafe { simd_state.calc_simd_unchecked(price, mults) };
            crate::write_simd_at_indices!(N, i, trendmode_ptrs => trendmode);
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

/// Calculates the Ehlers TrendMode on a single asset with `N` different α values
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
/// * `optional_outputs` — index `0` = `cycle`, index `1` = `peak`.
///
/// # Returns
///
/// `Ok((outputs, states))` where `outputs[i][0]` = trendmode, `outputs[i][1]` = cycle
/// (empty unless requested), `outputs[i][2]` = peak (empty unless requested), and
/// `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(NotEnoughData)` if the input is shorter than 56 bars, or
/// `Err(InvalidOptions)` if any α is not in `(0, 1)`.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    // Delegate alpha validation to trendmode's custom validator (rejects < 1.0 check).
    validate_options(options, Some(tm_validate_options))?;
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;

    let params: [(f64, f64, f64); N] = std::array::from_fn(|i| multiplier(options[i][0]));
    let want_cycle = optional_outputs
        .and_then(|f| f.first().copied())
        .unwrap_or(false);
    let want_peak = optional_outputs
        .and_then(|f| f.get(1).copied())
        .unwrap_or(false);
    let has_optional = want_cycle || want_peak;

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State, (f64, f64, f64)>::new();

    for i in 0..N {
        let capacity = output_length(inputs[0].len(), options[i]);

        let mut trendmode_line = crate::uninit_vec!(f64, capacity);
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
            params[i],
            &mut trendmode_line,
            &mut cycle_line,
            &mut peak_line,
        );

        // Slice outputs so the driver writes indices 1..capacity.
        let mut output_buffer = vec![trendmode_line, cycle_line, peak_line];
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
            min_data(options[i]),
            0,
            state,
            Some(&params[i]),
        ));

        output_buffers.push(output_buffer);
    }

    let mut driver = TrendModeOptionDriver {
        has_optional,
        want_cycle,
        want_peak,
    };
    let final_states = road_train.drive(&mut driver);

    let states = final_states
        .into_iter()
        .enumerate()
        .map(|(i, s)| IndicatorState::new(s, params[i]))
        .collect();
    Ok((output_buffers, states))
}
