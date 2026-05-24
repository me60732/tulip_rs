//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
//use std::simd::cmp::SimdPartialOrd;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::trima_simd::SimdState;
use crate::indicators::trima::{
    initialize_counters, min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH,
    OPTIONS_WIDTH,
};
use std::simd::Simd;

struct Params {
    counters: (usize, usize),
    multiplier: f64,
    period: usize,
}
/// SIMD driver for the Triangular Moving Average (TRIMA) indicator, processing `N` option-set lanes per scheduling epoch.
struct TrimaDriver;

impl Driver<State, Params> for TrimaDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&Params>>,
    ) {
        let len = outputs[0][0].len();
        let mut state = SimdState::new(&states);

        let (mut i, mut lsi, mut tsi1, multiplier_simd) = {
            let mut multipliers = [0.0; N];
            let mut i = [0usize; N];
            let mut lsi = [0usize; N];
            let mut tsi1 = [0usize; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(param) = option {
                    i[lane] = param.period - 1;
                    multipliers[lane] = param.multiplier;
                    lsi[lane] = param.counters.0;
                    tsi1[lane] = param.counters.1;
                }
            }
            //(Simd::from_array(i), Simd::from_array(lsi), Simd::from_array(tsi1), Simd::from_array(tsi2), Simd::from_array(multipliers))
            (i, lsi, tsi1, Simd::from_array(multipliers))
        };

        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let trima_line_ptr = crate::extract_output_ptrs!(outputs, N, trima_line_ptr);

        // Optimized main loop with minimal overhead
        for j in 0..len {
            let (real, lsi_value, tsi1_value) = crate::extract_simd_at_indices_array!(N, input_ptrs,
                current @ i,
                lsi_value @ lsi,
                tsi1_value @ tsi1
            );
            let tsi2_value = crate::extract_simd_inputs_at_index!(j, N,
                tsi2_value @ input_ptrs
            );
            let trima = state.calc_simd(real, lsi_value, tsi1_value, tsi2_value, multiplier_simd);

            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, j,
                trima_line_ptr => trima
            );

            for lane in 0..N {
                i[lane] += 1;
                lsi[lane] += 1;
                tsi1[lane] += 1;
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Triangular Moving Average (TRIMA) for one shared asset across `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch option sets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - Shared input data: `inputs[0]` is `&[f64]` containing `real` (price series).
/// * `options` - An array of `N` option sets; `options[i]` is `&[f64; OPTIONS_WIDTH]` containing
///   `[period]` for option set `i`.
/// * `optional_outputs` - Unused; TRIMA has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `trima` for option set `i`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or any option set is invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [Params; N] = std::array::from_fn(|i| Params {
        period: options[i][0] as usize,
        multiplier: multiplier(options[i][0] as usize),
        counters: initialize_counters(options[i][0] as usize),
    });

    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = output_length(inputs[0].len(), options[i]);
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    let mut road_train = PrimeMover::<N, State, Params>::new();
    for i in 0..N {
        let period = options[i][0] as usize;
        let state = State::init_state(inputs[0], period);
        let asset_inputs = vec![inputs[0]];

        unsafe {
            // Get a mutable reference to the output buffer for this asset
            let output_buffer = &mut output_buffers[i][0];
            let asset_outputs = vec![std::slice::from_raw_parts_mut(
                output_buffer.as_mut_ptr(),
                output_buffer.len(),
            )];
            road_train.add_asset(Asset::new(
                asset_inputs,
                asset_outputs,
                i,
                period - 1,
                period - 1,
                state,
                Some(&params[i]),
            ));
        }
    }
    let mut driver = TrimaDriver {};
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, params) in states_vec.into_iter().zip(params.into_iter()) {
        states.push(IndicatorState::new(
            inputs[0],
            state,
            params.multiplier,
            params.period,
        ));
    }
    Ok((output_buffers, states))
}
