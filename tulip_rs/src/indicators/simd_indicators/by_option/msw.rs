//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::msw::{
    min_data, multiplier, output_length, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::msw_simd::options::calc_simd;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;

/// SIMD driver for the Mesa Sine Wave (MSW) indicator, processing `N` option-set lanes per scheduling epoch.
struct MswDriver;

impl Driver<(), (usize, f64)> for MswDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut _states: Vec<&mut ()>,
        options: Vec<Option<&(usize, f64)>>,
    ) {
        let len = outputs[0][0].len();

        let (mut i, periods, multipliers) = {
            let mut i = [0usize; N];
            let mut periods = [0usize; N];
            let mut multiplier = [0.0; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&(period, multi)) = option {
                    i[lane] = period;
                    periods[lane] = period;
                    multiplier[lane] = multi;
                }
            }
            (i, periods, multiplier)
        };

        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        let (sine_line_ptr, lead_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, sine_line_ptr, lead_line_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for j in 0..len {
            let (sine, lead) = calc_simd(real_ptrs, periods, multipliers, i);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                sine_line_ptr => sine,
                lead_line_ptr => lead
            );

            for i in i.iter_mut() {
                *i += 1;
            }
        }
    }
}

/// Calculates the Mesa Sine Wave (MSW) indicator for one asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// Applies each of the `N` period configurations to the same shared input series, computing
/// sine and lead wave lines for all option sets in a single SIMD-accelerated pass via
/// [`PrimeMover`].
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
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [(usize, f64); N] = std::array::from_fn(|i| {
        let period = options[i][0] as usize;
        (period, multiplier(period))
    });

    let mut road_train = PrimeMover::<N, (), (usize, f64)>::new();

    let mut output_buffers = Vec::with_capacity(N);

    for (i, &(period, _)) in params.iter().enumerate() {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let (sine_line, lead_line) = {
            let capacity = output_length(inputs[0].len(), options[i]);
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
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = MswDriver;
    road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (&input, (period, multiplier)) in inputs.into_iter().zip(params.into_iter()) {
        states.push(IndicatorState::new(input, period, multiplier));
    }
    Ok((output_buffers, states))
}
