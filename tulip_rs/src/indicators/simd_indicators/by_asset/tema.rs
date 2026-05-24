//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;

use crate::indicators::dema::output_length as dema_output_length;
use crate::indicators::ema::output_length as ema_output_length;
use crate::indicators::simd_indicators::tema_simd::{calc_simd, SimdState};
use crate::indicators::tema::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Triple Exponential Moving Average (TEMA) across `N` asset lanes per scheduling epoch.
struct TemaDriver {
    multiplier: f64,
    inv_multiplier: f64,
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State> for TemaDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        let mut state = SimdState::new(&states);

        let multipliers_simd = (
            Simd::splat(self.multiplier),
            Simd::splat(self.inv_multiplier),
        );
        let (has_optional, want_dema, want_ema) = self.want_optional_outputs;
        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (tema_line_ptr, dema_line_ptr, ema_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, tema_line_ptr, dema_line_ptr, ema_line_ptr);

        // Optimized main loop with minimal overhead
        for i in 0..len {
            let values = crate::extract_simd_inputs_at_index!(i, N, values @ input_ptrs);

            let (temas, demas, emas) = calc_simd(&mut state, values, multipliers_simd);

            // Direct SIMD store if possible, otherwise individual stores
            unsafe {
                let results = temas.to_array();
                for j in 0..N {
                    *tema_line_ptr[j].add(i) = results[j];
                }
            }
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_dema, dema_line_ptr => demas,
                    want_ema, ema_line_ptr => emas
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Triple Exponential Moving Average (TEMA) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[real]` for asset `i`.
/// * `options` - `options[0]` is the `period`.
/// * `optional_outputs` - `optional_outputs[0] = true` enables `dema`,
///   `optional_outputs[1] = true` enables `ema`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the TEMA line for asset `i`,
/// `outputs[i][1]` is `dema` (empty unless requested),
/// `outputs[i][2]` is `ema` (empty unless requested), and
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;

    let period = options[0] as usize;
    let (multiplier, inv_multiplier) = multiplier(period);
    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, State>::new();
    let mut want_optional_outputs = (false, false, false);
    for i in 0..N {
        let len = inputs[i][0].len();
        let tema_capacity = output_length(len, options);
        let tema_line = crate::uninit_vec!(f64, tema_capacity);

        let (mut ema_line, mut dema_line) = {
            let ema_capacity = ema_output_length(len, options);
            //println!("Len: {:?}, option: {:?}", len, period);
            let dema_capacity = dema_output_length(len, options);
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                ema_line: ema_capacity,
                dema_line: dema_capacity
            )
        };

        let state = State::init_state(
            inputs[i][0],
            period,
            tema_capacity,
            (&mut dema_line, &mut ema_line),
        );
        let asset_inputs = vec![inputs[i][0]];
        let mut starts = [0; 3];
        (starts[1], starts[2]) = crate::slice_outputs_start!(tema_capacity, dema_line, ema_line);

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(dema_line, ema_line);
        }
        let mut output_buffer = vec![tema_line, dema_line, ema_line];
        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(starts[j]), //slice from
                    output_buffer.len(),                       // slice to
                ));
            }
        }
        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            len - tema_capacity,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = TemaDriver {
        multiplier,
        inv_multiplier,
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec {
        states.push(IndicatorState::new(state, (multiplier, inv_multiplier)));
    }
    Ok((output_buffers, states))
}
