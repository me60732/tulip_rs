//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{Cold, IndicatorError, Warm};
//use std::simd::cmp::SimdPartialOrd;
use crate::indicators::simd_indicators::trima_simd::{SimdState, TSimdState, TState};
use crate::indicators::trima::{
    initialize_counters, Indicator, IndicatorState, State, Trima, INPUTS, OPTIONS,
};
use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;
/// SIMD driver that advances the Triangular Moving Average (TRIMA) across `N` asset lanes per scheduling epoch.
struct TrimaDriver {
    counters: (usize, usize),
    period: usize,
}

impl Driver<State<Warm>> for TrimaDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let mut state = SimdState::from_states(&mut states);

        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let trima_line_ptr = crate::extract_output_ptrs!(outputs, N, trima_line_ptr);
        let p = self.period - 1;
        let periods = [p - self.counters.0, p - self.counters.1, p];
        let mut buffer = {
            let mut buf = Buffer::<Cold, Simd<f64, N>>::new(p);
            for i in 0..p {
                let real = crate::extract_simd_at_indices!(N, input_ptrs,
                    new_vals @ i
                );
                buf.push(real);
            }
            buf.into_full() // → SimdBuffer<N> = Buffer<Warm, Simd<f64,N>>
        };
        // Optimized main loop with minimal overhead
        for (j, i) in (p..len).enumerate() {
            let real = crate::extract_simd_at_indices!(N, input_ptrs,
                real @ i
            );
            let [lsi_val, tsi1_val, tsi2_val] = buffer.push_with_info_periods(real, periods);

            let trima = state.calc((real, lsi_val, tsi1_val, tsi2_val));

            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, j,
                trima_line_ptr => trima
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Triangular Moving Average (TRIMA) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// TRIMA produces no optional outputs. Uses the [`PrimeMover`] scheduler to batch assets into
/// SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[real]` for asset `i`.
/// * `options` - `options[0]` is the `period`.
/// * `_optional_outputs` - Unused; TRIMA has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the TRIMA line for asset `i` and
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Trima::min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = Trima::output_length(inputs[i][0].len(), options);
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    for i in 0..N {
        let state = State::init_state(inputs[i][0], period);
        let asset_inputs = vec![inputs[i][0]];

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
                None,
            ));
        }
    }
    let mut driver = TrimaDriver {
        period,
        counters: initialize_counters(period),
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(inputs[i][0], state, period));
    }
    Ok((output_buffers, states))
}
