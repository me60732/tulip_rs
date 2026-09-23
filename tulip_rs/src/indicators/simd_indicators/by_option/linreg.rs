//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::linreg::{Indicator, IndicatorState, Linreg, State, INPUTS, OPTIONS};
use crate::indicators::simd_indicators::linreg_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver for the Linear Regression (LINREG) indicator, processing `N` option-set lanes per scheduling epoch.
struct LinregDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State<Warm>, (usize, (f64, f64, f64), (f64, f64))> for LinregDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&(usize, (f64, f64, f64), (f64, f64))>>,
    ) {
        let mut state = SimdState::<N>::from_states(&mut states);
        let len = outputs[0][0].len();
        // Optimization 1: Direct array construction instead of collect+try_into

        //collect outputs
        let (linreg_line_ptr, slope_line_ptr, intercept_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            linreg_line_ptr,
            slope_line_ptr,
            intercept_line_ptr
        );

        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        let (has_optional, want_slope, want_intercept) = self.want_optional_outputs;
        if has_optional {
            let mut i = [0usize; N];
            let f_params = {
                let mut sum_x = [0.0; N];
                let mut per = [0.0; N];
                let mut inv_n = [0.0; N];
                for (lane, option) in options.iter().enumerate() {
                    if let Some(&(period, f_params, _)) = option {
                        sum_x[lane] = f_params.0;
                        per[lane] = f_params.1;
                        inv_n[lane] = f_params.2;
                        i[lane] = period;
                    }
                }
                (
                    Simd::from_array(sum_x),
                    Simd::from_array(per),
                    Simd::from_array(inv_n),
                )
            };
            // Optimization 3: Simplified main loop with pre-computed offsets
            for j in 0..len {
                // Get inputs arrays for stocks
                let real = crate::extract_simd_inputs_at_index_array!(i, N,
                    new @ real_ptrs
                );
                let prev_real = crate::extract_simd_inputs_at_index!(j+1, N, real @ real_ptrs);

                let (linreg, slope, intercept) = state.calc((prev_real, real, f_params));

                // Store results using pre-computed pointers
                crate::write_simd_at_indices!(N, j,
                    linreg_line_ptr => linreg
                );
                crate::store_simd_optional_outputs!(j, N,
                    want_slope, slope_line_ptr => slope,
                    want_intercept, intercept_line_ptr => intercept
                );

                for i in i.iter_mut() {
                    *i += 1;
                }
            }
        } else {
            let mut i = [0usize; N];
            let p_params = {
                let mut xy_coef = [0.0; N];
                let mut y_coef = [0.0; N];
                for (lane, option) in options.iter().enumerate() {
                    if let Some(&(period, _, p_params)) = option {
                        xy_coef[lane] = p_params.0;
                        y_coef[lane] = p_params.1;
                        i[lane] = period;
                    }
                }
                (Simd::from_array(xy_coef), Simd::from_array(y_coef))
            };
            for j in 0..len {
                // Get inputs arrays for stocks
                let real = crate::extract_simd_inputs_at_index_array!(i, N,
                    new @ real_ptrs
                );
                let prev_real = crate::extract_simd_inputs_at_index!(j+1, N, real @ real_ptrs);

                let linreg = state.partial_calc((prev_real, real, p_params));

                // Store results using pre-computed pointers
                crate::write_simd_at_indices!(N, j,
                    linreg_line_ptr => linreg
                );

                for i in i.iter_mut() {
                    *i += 1;
                }
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Linear Regression (LINREG) on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[real]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Optional output flags: `[want_linregslope, want_linregintercept]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[linreg, linregslope?, linregintercept?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Linreg::min_data)?;
    validate_options(options, None)?;

    let mut params: [(usize, (f64, f64, f64), (f64, f64)); N] =
        std::array::from_fn(|i| (options[i][0] as usize, (0.0, 0.0, 0.0), (0.0, 0.0)));
    let mut road_train = PrimeMover::<N, State<Warm>, (usize, (f64, f64, f64), (f64, f64))>::new();
    let mut want_optional_outputs = (false, false, false);
    let mut output_buffers = Vec::with_capacity(N);

    let mut states: Vec<State<Warm>> = Vec::with_capacity(N);
    for i in 0..N {
        let period = params[i].0;
        let (state, f_params, p_params) = State::init_state(&inputs[0][1..period], period);
        params[i].1 = f_params;
        params[i].2 = p_params;
        states.push(state);
    }

    for (i, state) in states.into_iter().enumerate() {
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let (linreg_line, slope_line, intercept_line);
        {
            let capacity = Linreg::output_length(inputs[0].len(), options[i]);
            (slope_line, intercept_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                slope_line: capacity,
                intercept_line: capacity
            );
            linreg_line = crate::uninit_vec!(f64, capacity);
        }

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(slope_line, intercept_line);
        }

        let mut output_buffer = vec![linreg_line, slope_line, intercept_line];

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
            params[i].0,
            params[i].0,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = LinregDriver {
        want_optional_outputs,
    };

    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, (period, f_params, p_params)) in states_vec.into_iter().zip(params.into_iter()) {
        states.push(IndicatorState::new(
            state,
            unsafe { inputs.get_unchecked(0) },
            period,
            f_params,
            p_params,
        ));
    }
    Ok((output_buffers, states))
}
