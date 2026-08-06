use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::ultosc_simd::assets::{SimdState, TSimdState, TState};
use crate::indicators::{
    tr::Tr,
    ultosc::{
        Ultosc, Indicator, validate_options, IndicatorState, State, INPUTS,
        OPTIONS,
    },
};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver that advances the Ultimate Oscillator (ULTOSC) across `N` asset lanes per scheduling epoch.
struct UltoscDriver {
    periods: (usize, usize),
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State<Warm>> for UltoscDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::from_states(&mut states);
        let len = inputs[0][0].len();

        //collect outputs
        let (ultosc_line_ptr, tr_line_ptr, bp_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, ultosc, tr, bp);

        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);

        let (has_optional, want_tr, want_bp) = self.want_optional_outputs;
        let (short_period, medium_period) = self.periods;
        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let (high, low, close) = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs
            );

            let (ultosc, tr, bp) =
                state.calc((high, low, close, short_period, medium_period));

            crate::write_simd_at_indices!(N, i,
                ultosc_line_ptr => ultosc
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_tr, tr_line_ptr => tr,
                    want_bp, bp_line_ptr => bp
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Ultimate Oscillator (ULTOSC) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// ULTOSC produces no optional outputs. Uses the [`PrimeMover`] scheduler to batch assets into
/// SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[high, low, close]` for asset `i`.
/// * `options` - `options[0]` is `short_period`, `options[1]` is `medium_period`,
///   `options[2]` is `long_period`.
/// * `_optional_outputs` - Unused; ULTOSC has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the ULTOSC line for asset `i` and
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Ultosc::min_data(options))?;
    validate_options(options)?;
    let periods = (
        options[0] as usize,
        options[1] as usize,
        options[2] as usize,
    );

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = (false, false, false);
    for i in 0..N {
        let [high, low, close] = *inputs[i];
        let asset_inputs = vec![high, low, close];

        let (mut ultosc_line, (mut tr_line, mut bp_line)) = {
            let capacity = Ultosc::output_length(high.len(), options);
            let tr_capacity = Tr::output_length(high.len(), &[]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    tr_line: tr_capacity,
                    bp_line: tr_capacity
                ),
            )
        };

        let state = State::init_state(
            high,
            low,
            close,
            periods,
            &mut ultosc_line,
            &mut tr_line,
            &mut bp_line,
        );
        let mut starts = [1; 3];
        (starts[1], starts[2]) = {
            let (mut tr, mut bp) = crate::slice_outputs_start!(ultosc_line.len(), tr_line, bp_line);
            if tr > 0 {
                tr += 1;
            }
            if bp > 0 {
                bp += 1;
            }
            (tr, bp)
        };

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(tr_line, bp_line);
        }
        let mut output_buffer = vec![ultosc_line, tr_line, bp_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(starts[j]), //slice from
                    output_buffer.len() - starts[j],           // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            periods.2 + 1,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = UltoscDriver {
        periods: (periods.0, periods.1),
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state, (periods.0, periods.1)));
    }
    Ok((output_buffers, states))
}
