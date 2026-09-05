//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::ultosc_simd::options::{SimdState, TState};
use crate::indicators::{
    tr::Tr,
    ultosc::{
        Ultosc, Indicator, validate_options as vo, IndicatorState, State, INPUTS,
        OPTIONS,
    },
};
use crate::types::{IndicatorError, Warm};

/// SIMD driver for the Ultimate Oscillator (ULTOSC) indicator, processing `N` option-set lanes per scheduling epoch.
struct UltoscDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State<Warm>, (usize, usize, usize)> for UltoscDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&(usize, usize, usize)>>,
    ) {
        let len = outputs[0][0].len();
        let mut state = {
            let mut short_periods = [0usize; N];
            let mut medium_periods = [0usize; N];
            let mut long_periods = [0usize; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&(short_period, medium_period, long_period)) = option {
                    short_periods[lane] = short_period;
                    medium_periods[lane] = medium_period;
                    long_periods[lane] = long_period;
                }
            }
            SimdState::<N>::from_states(&mut states, (short_periods, medium_periods, long_periods))
        };
        //collect outputs
        let (ultosc_line_ptr, tr_line_ptr, bp_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, ultosc, tr, bp);

        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);

        let (has_optional, want_tr, want_bp) = self.want_optional_outputs;

        for i in 0..len {
            // Get inputs arrays for stocks
            let inputs = unsafe {
                (
                    *high_ptrs[0].add(i),
                    *low_ptrs[0].add(i),
                    *close_ptrs[0].add(i),
                )
            };

            let (ultosc, tr, bp) = state.calc(inputs);

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

/// Calculates the Ultimate Oscillator (ULTOSC) for one shared asset across `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch option sets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - Shared input data: `inputs[0]` is `&[f64]` for `high`, `inputs[1]` for `low`,
///   `inputs[2]` for `close`.
/// * `options` - An array of `N` option sets; `options[i]` is `&[f64; OPTIONS]` containing
///   `[short_period, medium_period, long_period]` for option set `i`.
/// * `optional_outputs` - Unused; ULTOSC has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `ultosc` for option set `i`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or any option set is invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Ultosc::min_data)?;
    validate_options(options, Some(vo))?;
    let periods: [(usize, usize, usize); N] = std::array::from_fn(|i| {
        (
            options[i][0] as usize,
            options[i][1] as usize,
            options[i][2] as usize,
        )
    });
    let mut road_train = PrimeMover::<N, State<Warm>, (usize, usize, usize)>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = (false, false, false);
    let [high, low, close] = *inputs;
    for i in 0..N {
        let asset_inputs = vec![high, low, close];

        let (mut ultosc_line, (mut tr_line, mut bp_line)) = {
            let capacity = Ultosc::output_length(high.len(), options[i]);
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
            periods[i],
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
            periods[i].2 + 1,
            0,
            state,
            Some(&periods[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = UltoscDriver {
        want_optional_outputs,
    };

    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, periods) in states_vec.into_iter().zip(periods.into_iter()) {
        states.push(IndicatorState::new(state, (periods.0, periods.1)));
    }
    Ok((output_buffers, states))
}
