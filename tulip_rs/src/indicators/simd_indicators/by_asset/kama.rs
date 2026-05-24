use crate::indicators::kama::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::kama_simd::{calc_simd, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Kaufman's Adaptive Moving Average (KAMA) across `N` asset
/// lanes per scheduling epoch.
struct KamaDriver {
    multipliers: (f64, f64),
    period: usize,
}

impl Driver<State> for KamaDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][0]` (real), writes the KAMA to `outputs[asset][0]`,
    /// and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Direct array construction
        let mut simd_state = SimdState::new(&states);

        let multipliers_simd = (
            Simd::splat(self.multipliers.0),
            Simd::splat(self.multipliers.1),
        );

        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let output_ptrs = crate::extract_output_ptrs!(outputs, N, output_ptrs);

        let (mut prev, mut old) = crate::extract_simd_at_indices!(N, input_ptrs,
            prev_value @ self.period,
            old_value @ 0
        );

        // Optimized main loop with minimal overhead
        for (j, i) in (self.period + 1..len).enumerate() {
            let (value, last) = crate::extract_simd_at_indices!(N, input_ptrs,
                value @ i,
                last_value @ j+1
            );

            let kama = calc_simd(&mut simd_state, (value, prev, last, old), multipliers_simd);
            old = last;
            prev = value;
            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, j,
                output_ptrs => kama
            );
        }

        simd_state.write_states(&mut states);
    }
}

/// Calculates the Kaufman's Adaptive Moving Average (KAMA) for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[real]` for asset `i`.
/// * `options` - Shared options slice; `options[0]` is the period.
/// * `_optional_outputs` - Unused; KAMA has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the KAMA line for asset `i`
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    // Create output buffers OUTSIDE the assets - these will be owned by this function
    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, State>::new();

    for i in 0..N {
        let len = inputs[i][0].len();
        let capacity = output_length(len, options);
        let mut kama_line = crate::uninit_vec!(f64, capacity);

        let state = State::init_state(inputs[i][0], period, &mut kama_line);
        let asset_inputs = vec![inputs[i][0]];

        let mut output_buffer = vec![kama_line];
        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        unsafe {
            //let slice_len = output_buffer.len() - starts[j];
            // Get a mutable reference to the output buffer for this asset
            let output_buffer = &mut output_buffer[0];
            asset_outputs.push(std::slice::from_raw_parts_mut(
                output_buffer.as_mut_ptr().add(1), //slice from
                output_buffer.len(),               // slice to
            ));
        }
        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period + 1,
            period + 1,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = KamaDriver {
        period,
        multipliers: multiplier(),
    };
    let final_states = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in final_states.into_iter().enumerate() {
        states.push(IndicatorState::new(
            inputs[i][0],
            period,
            driver.multipliers,
            state,
        ));
    }
    Ok((output_buffers, states))
}
