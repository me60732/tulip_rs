//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::chandelierexit_simd::{options::Calc, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::{
    chandelierexit::{
        min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
    },
    tr::output_length as tr_output_length,
};
use crate::types::IndicatorError;
use std::simd::Simd;
/// SIMD driver for the Chandelier Exit indicator, processing `N` option-set lanes per scheduling epoch.
struct ChandelierExitDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State, (usize, usize, (f64, (f64, f64)))> for ChandelierExitDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(usize, usize, (f64, (f64, f64)))>>,
    ) {
        let len = outputs[0][0].len();

        let (look_back, multipliers, mut i_simd) = {
            let mut look_back = [0; N];
            let mut i_array = [0; N];
            let mut multipliers = ([0.0; N], ([0.0; N], [0.0; N]));
            for (lane, option) in options.iter().enumerate() {
                if let Some(&(p, l, m)) = option {
                    look_back[lane] = l;
                    i_array[lane] = p;
                    multipliers.0[lane] = m.0;
                    multipliers.1 .0[lane] = m.1 .0;
                    multipliers.1 .1[lane] = m.1 .1;
                }
            }
            (
                Simd::from_array(look_back),
                (
                    Simd::from_array(multipliers.0),
                    (
                        Simd::from_array(multipliers.1 .0),
                        Simd::from_array(multipliers.1 .1),
                    ),
                ),
                Simd::from_array(i_array),
            )
        };

        //collect outputs
        let (long_line_ptr, short_line_ptr, atr_line_ptr, tr_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            long_line_ptrs,
            short_line_ptrs,
            atr_line_ptrs,
            tr_line_ptrs
        );

        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);

        let mut state = SimdState::new(&mut states);
        let one_splat = Simd::splat(1);
        let (has_optional, want_atr, want_tr) = self.want_optional_outputs;
        //println!("start: {:?}, N: {:?}, LEN: {:?}", start, N, real.len());
        for j in 0..len {
            let close = crate::extract_simd_inputs_at_index_array!(i_simd.as_array(), N,
                close @ close_ptrs
            );
            let (long, short, atr, tr) = unsafe {
                state.calc_unchecked_simd(
                    high_ptrs,
                    low_ptrs,
                    close,
                    i_simd,
                    look_back,
                    multipliers,
                )
            };

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                long_line_ptr => long,
                short_line_ptr => short
            );
            if has_optional {
                crate::store_simd_optional_outputs!(j, N,
                    want_atr, atr_line_ptr => atr,
                    want_tr, tr_line_ptr => tr
                );
            }

            i_simd += one_splat;
        }
        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Chandelier Exit indicator on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[high, low, close]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period, multiplier]`.
/// * `optional_outputs` - Optional flags `[want_atr, want_tr]` to enable extra outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[long, short, atr?, tr?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [(usize, usize, (f64, (f64, f64))); N] = std::array::from_fn(|i| {
        let period = options[i][0] as usize;
        (period, period - 1, (options[i][1], multiplier(period)))
    });
    let mut road_train = PrimeMover::<N, State, (usize, usize, (f64, (f64, f64)))>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = (false, false, false);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
            inputs[2], // close
        ];

        let (long_line, short_line, (atr_line, mut tr_line)) = {
            let len = inputs[0].len();
            let capacity = output_length(len, options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    atr_line: capacity,
                    tr_line: tr_output_length(len, options[i])
                ),
            )
        };
        let state = State::new(
            inputs[0],
            inputs[1],
            inputs[2],
            params[i].0,
            params[i].1,
            &mut tr_line,
        );
        let mut starts = [0; 4];
        starts[3] = crate::slice_outputs_start!(long_line.len(), tr_line);
        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(atr_line, tr_line);
        }
        let mut output_buffer = vec![long_line, short_line, atr_line, tr_line];

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
            params[i].1,
            params[i].1,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = ChandelierExitDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (state, &param) in states_vec.into_iter().zip(params.iter()) {
        let periods = (param.0, param.1);
        let multipliers = param.2;
        states.push(IndicatorState::new(
            inputs[0],
            inputs[1],
            state,
            periods,
            multipliers,
        ));
    }
    Ok((output_buffers, states))
}
