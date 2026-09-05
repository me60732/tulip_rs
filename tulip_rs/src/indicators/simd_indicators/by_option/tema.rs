use crate::indicator_types::TSimdState;
//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
//use std::simd::cmp::SimdPartialOrd;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::dema::Dema;
use crate::indicators::ema::Ema;
use crate::indicators::simd_indicators::tema_simd::{SimdState, TState};
use crate::indicators::tema::{Indicator, IndicatorState, State, Tema, INPUTS, OPTIONS};
use std::simd::Simd;

/// SIMD driver for the Triple Exponential Moving Average (TEMA) indicator, processing `N` option-set lanes per scheduling epoch.
struct TemaDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State<Warm>> for TemaDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = outputs[0][0].len();

        let mut state = SimdState::<N>::from_states(&mut states);

        let (has_optional, want_dema, want_ema) = self.want_optional_outputs;
        // Pre-compute pointers for maximum efficiency
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let (tema_line_ptr, dema_line_ptr, ema_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, tema_line_ptr, dema_line_ptr, ema_line_ptr);

        // Optimized main loop with minimal overhead
        for i in 0..len {
            let values = crate::extract_simd_inputs_at_index_splat!(i, N, values @ input_ptrs);

            let (tema, dema, ema) = state.calc(values);

            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, i,
                tema_line_ptr => tema
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_dema, dema_line_ptr => dema,
                    want_ema, ema_line_ptr => ema
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Triple Exponential Moving Average (TEMA) for one shared asset across `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch option sets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - Shared input data: `inputs[0]` is `&[f64]` containing `real` (price series).
/// * `options` - An array of `N` option sets; `options[i]` is `&[f64; OPTIONS]` containing
///   `[period]` for option set `i`.
/// * `optional_outputs` - Optional slice controlling extra output series;
///   index 0 enables `dema`, index 1 enables `ema`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `tema`, `outputs[i][1]` is `dema`
/// (empty unless requested), and `outputs[i][2]` is `ema` (empty unless requested) for option set `i`,
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or any option set is invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Tema::min_data)?;
    validate_options(options, None)?;
    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut want_optional_outputs = (false, false, false);

    for i in 0..N {
        let len = inputs[0].len();
        let tema_capacity = Tema::output_length(len, options[i]);
        let tema_line = crate::uninit_vec!(f64, tema_capacity);

        let (mut ema_line, mut dema_line) = {
            let ema_capacity = Ema::output_length(len, options[i]);
            //println!("Len: {:?}, option: {:?}", len, period);
            let dema_capacity = Dema::output_length(len, options[i]);
            crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                ema_line: ema_capacity,
                dema_line: dema_capacity
            )
        };

        let period = options[i][0] as usize;
        let state = State::init_state(inputs[0], period, (&mut dema_line, &mut ema_line));
        let asset_inputs = vec![inputs[0]];

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
                    output_buffer.len() - starts[j],           // slice to
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
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
