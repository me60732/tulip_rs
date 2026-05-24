//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::trix_simd::{calc_simd, SimdState};
use crate::indicators::trix::{
    init_state, min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH,
    OPTIONS_WIDTH,
};
use crate::indicators::{
    dema::output_length as dema_output_length, ema::output_length as ema_output_length,
    tema::output_length as tema_output_length,
};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver for the Triple Exponential Oscillator (TRIX) indicator, processing `N` option-set lanes per scheduling epoch.
struct TrixDriver {
    want_optional_outputs: (bool, bool, bool, bool),
}

impl Driver<State, (f64, f64)> for TrixDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(f64, f64)>>,
    ) {
        let len = outputs[0][0].len();

        let mut state = SimdState::new(&states);

        let multipliers_simd = {
            let mut multipliers = ([0.0; N], [0.0; N]);
            for (lane, option) in options.iter().enumerate() {
                let &multiplier =
                    option.expect("Missing multiplier for lane - bug in add_asset call");
                multipliers.0[lane] = multiplier.0;
                multipliers.1[lane] = multiplier.1;
            }
            (
                Simd::from_array(multipliers.0),
                Simd::from_array(multipliers.1),
            )
        };

        let (has_optional, want_tema, want_dema, want_ema) = self.want_optional_outputs;
        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (trix_line_ptr, tema_line_ptr, dema_line_ptr, ema_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            trix_line_ptr,
            tema_line_ptr,
            dema_line_ptr,
            ema_line_ptr
        );

        // Optimized main loop with minimal overhead
        for i in 0..len {
            let values = crate::extract_simd_inputs_at_index_splat!(i, N, values @ input_ptrs);

            let (trix, tema, dema, ema) = calc_simd(&mut state, values, multipliers_simd);

            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, i,
                trix_line_ptr => trix
            );

            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_tema, tema_line_ptr => tema,
                    want_dema, dema_line_ptr => dema,
                    want_ema, ema_line_ptr => ema
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Triple Exponential Oscillator (TRIX) for one shared asset across `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch option sets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - Shared input data: `inputs[0]` is `&[f64]` containing `real` (price series).
/// * `options` - An array of `N` option sets; `options[i]` is `&[f64; OPTIONS_WIDTH]` containing
///   `[period]` for option set `i`.
/// * `optional_outputs` - Optional slice controlling extra output series;
///   index 0 enables `tema`, index 1 enables `dema`, index 2 enables `ema`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `trix`, `outputs[i][1]` is `tema`
/// (empty unless requested), `outputs[i][2]` is `dema` (empty unless requested), and
/// `outputs[i][3]` is `ema` (empty unless requested) for option set `i`,
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or any option set is invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let multipliers: [(f64, f64); N] = std::array::from_fn(|i| multiplier(options[i][0] as usize));

    let mut road_train = PrimeMover::<N, State, (f64, f64)>::new();
    let mut output_buffers = Vec::with_capacity(N);

    let mut want_optional_outputs = (false, false, false, false);
    for i in 0..N {
        let len = inputs[0].len();
        let trix_capacity = output_length(len, options[i]);
        let trix_line = crate::uninit_vec!(f64, trix_capacity);

        let (mut tema_line, mut dema_line, mut ema_line) = {
            let tema_cap = tema_output_length(len, options[i]);
            let dema_cap = dema_output_length(len, options[i]);
            let ema_cap = ema_output_length(len, options[i]);

            crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                tema_line: tema_cap,
                dema_line: dema_cap,
                ema_line: ema_cap
            )
        };
        let period = options[i][0] as usize;
        let state = init_state(
            inputs[0],
            period,
            trix_capacity,
            (&mut tema_line, &mut dema_line, &mut ema_line),
        );
        let asset_inputs = vec![inputs[0]];
        let mut starts = [0; 4];
        (starts[1], starts[2], starts[3]) =
            crate::slice_outputs_start!(trix_capacity, tema_line, dema_line, ema_line);

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(tema_line, dema_line, ema_line);
        }
        let mut output_buffer = vec![trix_line, tema_line, dema_line, ema_line];
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
            len - trix_capacity,
            0,
            state,
            Some(&multipliers[i]),
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = TrixDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, &multiplier) in states_vec.into_iter().zip(multipliers.iter()) {
        states.push(IndicatorState::new(state, multiplier));
    }
    Ok((output_buffers, states))
}
