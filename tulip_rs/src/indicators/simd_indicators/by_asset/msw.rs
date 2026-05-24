//use crate::common::validate_inputs;
use crate::indicators::msw::{
    min_data, multiplier, output_length, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::msw_simd::assets::calc_simd;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Mesa Sine Wave (MSW) across `N` asset lanes per scheduling epoch.
struct MswDriver {
    period: usize,
    multiplier: f64,
}

impl Driver<()> for MswDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut _states: Vec<&mut ()>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let multiplier_simd = Simd::splat(self.multiplier as f64);
        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);

        let real_simd: Vec<Simd<f64, N>> = crate::create_simd_vec_from_inputs!(input_ptrs, N, len);

        let (sine_line_ptr, lead_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, sine_line_ptr, lead_line_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.period..len).enumerate() {
            // Get new and old values using pre-computed pointers

            let (sine, lead) = calc_simd(
                unsafe { real_simd.get_unchecked(j + 1..=i) },
                multiplier_simd,
            );

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                sine_line_ptr => sine,
                lead_line_ptr => lead
            );
        }
    }
}

/// Calculates the Mesa Sine Wave (MSW) for `N` assets simultaneously using SIMD parallelism.
///
/// MSW decomposes a real input series into sine and lead components using a
/// Goertzel-style frequency analyser. Uses the [`PrimeMover`] scheduler to batch
/// assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[real]` for asset `i`.
/// * `options` - `[period]` — the look-back window length for the sine-wave fit.
/// * `_optional_outputs` - Unused; MSW produces no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the `msw_sine` line and
/// `outputs[i][1]` is the `msw_lead` line for asset `i`, and `states[i]` is the
/// final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let multiplier = multiplier(period);
    let mut road_train = PrimeMover::<N, ()>::new();

    let mut output_buffers = Vec::with_capacity(N);

    for (i, &input) in inputs.into_iter().enumerate() {
        let asset_inputs = vec![input[0]];
        let (sine_line, lead_line) = {
            let capacity = output_length(input[0].len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let mut output_buffer = vec![sine_line, lead_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(0), //slice from
                    output_buffer.len(),               // slice to
                ));
            }
        }
        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period,
            period,
            (),
            None,
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = MswDriver { period, multiplier };
    road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for &input in inputs.into_iter() {
        states.push(IndicatorState::new(
            unsafe { input.get_unchecked(0) },
            period,
            multiplier,
        ));
    }
    Ok((output_buffers, states))
}
