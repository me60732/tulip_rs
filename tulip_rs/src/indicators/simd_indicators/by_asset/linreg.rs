//use crate::common::validate_inputs;
use crate::indicators::linreg::{
    Linreg, Indicator, IndicatorState, State, INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::linreg_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Linear Regression (LINREG) across `N` asset lanes
/// per scheduling epoch.
struct LinregDriver {
    want_optional_outputs: (bool, bool, bool),
    period: usize,
}

impl Driver<State<Warm>, ((f64, f64, f64), (f64, f64))> for LinregDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][0]` (real), writes the LINREG to `outputs[asset][0]`,
    /// optional slope to `outputs[asset][1]`, optional intercept to `outputs[asset][2]`,
    /// and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&((f64, f64, f64), (f64, f64))>>,
    ) {
        let mut state = SimdState::<N>::from_states(&mut states);
        let len = inputs[0][0].len();

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
            let f_params = {
                let mut sum_x = [0.0; N];
                let mut per = [0.0; N];
                let mut inv_n = [0.0; N];
                for (lane, option) in options.iter().enumerate() {
                    if let Some(&(f_params, _)) = option {
                        sum_x[lane] = f_params.0;
                        per[lane] = f_params.1;
                        inv_n[lane] = f_params.2;
                    }
                }
                (
                    Simd::from_array(sum_x),
                    Simd::from_array(per),
                    Simd::from_array(inv_n)
                )
            };
            // Optimization 3: Simplified main loop with pre-computed offsets
            for (j, i) in (self.period..len).enumerate() {
                // Get inputs arrays for stocks
                let (prev_val, value) = crate::extract_simd_at_indices!(N, real_ptrs,
                    prev_real @ j+1,//i + 1 - self.period
                    real @ i
                );
    
                let (linreg, slope, intercept) = state.calc((prev_val, value, f_params));
    
                // Store results using pre-computed pointers
                crate::write_simd_at_indices!(N, j,
                    linreg_line_ptr => linreg
                );
                crate::store_simd_optional_outputs!(j, N,
                    want_slope, slope_line_ptr => slope,
                    want_intercept, intercept_line_ptr => intercept
                );
            }
        } else {
            let p_params = {
                let mut xy_coef = [0.0; N];
                let mut y_coef = [0.0; N];
                for (lane, option) in options.iter().enumerate() {
                    if let Some(&(_, p_params)) = option {
                        xy_coef[lane] = p_params.0;
                        y_coef[lane] = p_params.1;
                    }
                }
                (
                    Simd::from_array(xy_coef),
                    Simd::from_array(y_coef)
                )
            };
            // Optimization 3: Simplified main loop with pre-computed offsets
            for (j, i) in (self.period..len).enumerate() {
                // Get inputs arrays for stocks
                let (prev_val, value) = crate::extract_simd_at_indices!(N, real_ptrs,
                    prev_real @ j+1,//i + 1 - self.period
                    real @ i
                );
    
                let linreg = state.partial_calc((prev_val, value, p_params));
    
                // Store results using pre-computed pointers
                crate::write_simd_at_indices!(N, j,
                    linreg_line_ptr => linreg
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Linear Regression (LINREG) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS]`
///   containing `[real]` for asset `i`.
/// * `options` - Shared options slice; `options[0]` is the period.
/// * `optional_outputs` - Optional slice selecting extra outputs: index `0` = `linregslope`,
///   index `1` = `linregintercept`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the LINREG line for asset `i`,
/// `outputs[i][1]` is the optional slope, `outputs[i][2]` is the optional intercept,
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub(crate) fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS>(inputs, Linreg::min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let mut params: [((f64, f64, f64), (f64, f64)); N] = std::array::from_fn(|_| ((0.0, 0.0, 0.0), (0.0, 0.0)));
    let mut road_train = PrimeMover::<N, State<Warm>, ((f64, f64, f64), (f64, f64))>::new();
    let mut want_optional_outputs = (false, false, false);
    let mut output_buffers = Vec::with_capacity(N);
    let mut states: Vec<State<Warm>> = Vec::with_capacity(N);
    for i in 0..N {
        let (state, f_params, p_params) = State::init_state(&inputs[i][0][1..period], period);
        params[i].0 = f_params;
        params[i].1 = p_params;
        states.push(state);
    }
    for (i, state) in states.into_iter().enumerate() {
        let asset_inputs = vec![
            inputs[i][0], // real
        ];

        let (linreg_line, slope_line, intercept_line);
        {
            let capacity = Linreg::output_length(inputs[i][0].len(), options);
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
            period,
            period,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = LinregDriver {
        period,
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            state,
            unsafe { inputs.get_unchecked(i).get_unchecked(0) },
            period,
            params[i].0,
            params[i].1
        ));
    }
    Ok((output_buffers, states))
}
