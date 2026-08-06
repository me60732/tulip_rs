//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::fosc::{Fosc, Indicator, IndicatorState, State, INPUTS, OPTIONS};
use crate::indicators::simd_indicators::fosc_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::tsf::Tsf;
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;
/// SIMD driver for the Forecast Oscillator (FOSC) indicator, processing `N` option-set lanes per scheduling epoch.
struct FoscDriver {
    want_optional_outputs: (bool, bool, bool, bool, bool),
}

impl Driver<State<Warm>, usize> for FoscDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&usize>>,
    ) {
        let mut state = SimdState::<N>::from_states(&mut states);
        let len = outputs[0][0].len();
        let mut i = [0usize; N];
        for (lane, option) in options.iter().enumerate() {
            if let Some(&period) = option {
                i[lane] = period;
            }
        }

        let (has_optional, want_tsf, want_linreg, want_slope, want_intercept) =
            self.want_optional_outputs;
        // Optimization 1: Direct array construction instead of collect+try_into
        //collect outputs
        let (fosc_line_ptr, tsf_line_ptr, linreg_line_ptr, slope_line_ptr, intercept_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            fosc_line_ptr,
            tsf_line_ptr,
            linreg_line_ptr,
            slope_line_ptr,
            intercept_line_ptr
        );

        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for j in 0..len {
            // Get inputs arrays for stocks
            let real = crate::extract_simd_inputs_at_index_array!(i, N,
                new @ real_ptrs
            );
            let prev_real = crate::extract_simd_inputs_at_index!(j+1, N, real @ real_ptrs);
            let (fosc, tsf, linreg, slope, intercept) = state.calc((prev_real, real));

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                fosc_line_ptr => fosc
            );

            if has_optional {
                crate::store_simd_optional_outputs!(j, N,
                    want_tsf, tsf_line_ptr => tsf,
                    want_linreg, linreg_line_ptr => linreg,
                    want_slope, slope_line_ptr => slope,
                    want_intercept, intercept_line_ptr => intercept
                );
            }
            for i in i.iter_mut() {
                *i += 1;
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Forecast Oscillator (FOSC) on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[real]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Optional output flags:
///   `[want_tsf, want_linreg, want_linregslope, want_linregintercept]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains
/// `[fosc, tsf?, linreg?, linregslope?, linregintercept?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Fosc::min_data)?;
    validate_options(options, None)?;
    let params: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);

    let mut road_train = PrimeMover::<N, State<Warm>, usize>::new();
    let mut want_optional_outputs = (false, false, false, false, false);
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let capacity = Fosc::output_length(inputs[0].len(), options[i]);
        let fosc_line = crate::uninit_vec!(f64, capacity);
        let (mut tsf_line, mut linreg_line, mut slope_line, mut intercept_line) = {
            let tsf_capacity = Tsf::output_length(inputs[0].len(), options[i]);

            crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false, false],
                tsf_line: tsf_capacity,
                linreg_line: tsf_capacity,
                slope_line: tsf_capacity,
                intercept_line: tsf_capacity
            )
        };
        let period = options[i][0] as usize;
        let state = State::init_state(
            &inputs[0],
            period,
            (
                &mut tsf_line,
                &mut linreg_line,
                &mut slope_line,
                &mut intercept_line,
            ),
        );

        if i == 0 {
            want_optional_outputs =
                crate::calc_want_flags!(tsf_line, linreg_line, slope_line, intercept_line);
        }

        let mut starts = [0; 5];
        (starts[1], starts[2], starts[3], starts[4]) = crate::slice_outputs_start!(
            capacity,
            tsf_line,
            linreg_line,
            slope_line,
            intercept_line
        );

        let mut output_buffer = vec![fosc_line, tsf_line, linreg_line, slope_line, intercept_line];
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
            period + 1,
            period,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = FoscDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            state,
            unsafe { inputs.get_unchecked(0) },
            params[i],
        ));
    }
    Ok((output_buffers, states))
}
