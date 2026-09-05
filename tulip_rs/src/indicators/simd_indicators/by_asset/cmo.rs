//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::cmo::{
    Cmo, Indicator, IndicatorState, State, INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::cmo_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver that advances the Chande Momentum Oscillator (CMO) across `N` asset lanes per
/// scheduling epoch.
struct CmoDriver {
    /// The look-back period used to compute up/down momentum sums.
    period: usize,
}

impl Driver<State<Warm>> for CmoDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][0]` (real prices), writes CMO values to
    /// `outputs[asset][0]`, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        //let output_len = len - self.period;

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::from_states(&mut states);

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let cmo_line_ptr = crate::extract_output_ptrs!(outputs, N, cmo_line_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.period..len).enumerate() {
            // Get new and old values using pre-computed pointers
            let inputs = crate::extract_simd_at_indices!(N, input_ptrs,
                current @ i,
                prev_period @ j
            );

            let cmo = state.calc(inputs);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                cmo_line_ptr => cmo
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Chande Momentum Oscillator (CMO) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// CMO measures the sum of recent up-moves minus recent down-moves, normalised to a ±100 scale.
/// All assets share the same `options`. Uses the [`PrimeMover`] scheduler to batch assets into
/// SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing the real price series for asset `i`.
/// * `options` - Shared options applied to all `N` assets: `[period]`.
/// * `_optional_outputs` - Unused; CMO has no optional output lines.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the CMO series for asset `i`
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Cmo::min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // real
        ];

        let cmo_line = crate::uninit_vec!(f64, Cmo::output_length(inputs[i][0].len(), options));

        let state = State::init_state(inputs[i][0], period);

        let mut output_buffer = vec![cmo_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr(), //slice from
                    output_buffer.len(),        // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period+1,
            period,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = CmoDriver {
        period
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            unsafe { inputs.get_unchecked(i).get_unchecked(0) },
            state,
            period,
        ));
    }
    Ok((output_buffers, states))
}