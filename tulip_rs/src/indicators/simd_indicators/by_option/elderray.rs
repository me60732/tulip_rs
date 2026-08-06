//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::elderray::{Elderray, Indicator, IndicatorState, State, INPUTS, OPTIONS};

use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::elderray_simd::{SimdState, TSimdState, TState};

/// SIMD driver for Elder-ray, processing `N` option-set lanes per scheduling epoch.
struct EmaDriver {
    want_ema: bool,
}

impl Driver<State<Warm>> for EmaDriver {
    /// Processes one epoch of bars for `N` option-set lanes simultaneously using SIMD.
    ///
    /// For each bar, computes `bull = high − EMA` and `bear = low − EMA` using each lane's
    /// multipliers, writes bull and bear to their output buffers, optionally writes the
    /// updated EMA, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = outputs[0][0].len();
        //println!("N: {:?}", N);
        //let mut period_arr = [0usize; N];

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::<N>::from_states(&mut states);

        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high, low, close);
        let (bull_line_ptr, bear_line_ptr, ema_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, bull, bear, ema);
        //let mut j = 0;
        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            let inputs = crate::extract_simd_inputs_at_index_splat!(i, N,
                h @ high_ptrs,
                l @ low_ptrs,
                c @ close_ptrs
            );

            let (bull, bear, ema) = state.calc(inputs);

            crate::write_simd_at_indices!(N, i,
                bull_line_ptr => bull,
                bear_line_ptr => bear
            );
            crate::store_simd_optional_outputs!(i, N,
                self.want_ema, ema_line_ptr => ema
            );
        }

        state.write_states(&mut states);
    }
}

/// Calculates Elder-ray on a single asset with `N` different option sets simultaneously
/// using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[high, low, close]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Pass `Some(&[true])` to also populate the EMA line for every lane.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` is `[bull, bear, ema]` for option set `i`
/// (the `ema` vec is empty unless `optional_outputs` enables it) and `states[i]` is the
/// final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Elderray::min_data)?;
    validate_options(options, None)?;
    let periods: [usize; N] = std::array::from_fn(|i| options[i][0] as usize);

    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_ema = false;
    for (i, &period) in periods.iter().enumerate() {
        let [high, low, close] = *inputs;
        let asset_inputs = vec![high, low, close];

        let (bull_line, bear_line, ema_line) = {
            let len = inputs[0].len();
            let capacity = Elderray::output_length(len, options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    ema_line: capacity
                ),
            )
        };

        let state = State::init_state(close, period);
        if i == 0 {
            (_, want_ema) = crate::calc_want_flags!(ema_line);
        }
        let mut output_buffer = vec![bull_line, bear_line, ema_line];

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
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = EmaDriver { want_ema };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
