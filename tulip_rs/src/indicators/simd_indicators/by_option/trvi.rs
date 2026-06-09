//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::trvi_simd::options::SimdState;
use crate::indicators::{
    ema::output_length as ema_output_length,
    tr::output_length as tr_output_length,
    trvi::{
        min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
    },
};

use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver for the True Range Volatility Indicator (TRVI), processing `N` option-set lanes
/// per scheduling epoch.
struct TrviDriver {
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State, (f64, f64)> for TrviDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD.
    /// Reads the shared input (high, low, close), applies each lane's options, writes TRVI
    /// outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(f64, f64)>>,
    ) {
        let mut state = SimdState::new(&mut states);
        let len = outputs[0][0].len();

        let multipliers_simd = {
            let mut multipliers = ([0.0; N], [0.0; N]);
            for (lane, option) in options.iter().enumerate() {
                if let Some(&multiplier) = option {
                    //println!("{:?}", outputs[lane][0].len());
                    multipliers.0[lane] = multiplier.0;
                    multipliers.1[lane] = multiplier.1;
                }
            }
            (
                Simd::from_array(multipliers.0),
                Simd::from_array(multipliers.1),
            )
        };

        //collect outputs
        let (trvi_line_ptr, tr_line_ptr, ema_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, trvi, tr, ema);

        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high, low, close);
        let (has_optional, want_tr, want_ema) = self.want_optional_outputs;

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let (high, low, close) = unsafe {
                (
                    *high_ptrs[0].add(i),
                    *low_ptrs[0].add(i),
                    *close_ptrs[0].add(i),
                )
            };

            let (trvi, tr, ema) =
                unsafe { state.calc_unchecked_simd(high, low, close, multipliers_simd) };

            crate::write_simd_at_indices!(N, i,
                trvi_line_ptr => trvi
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_tr, tr_line_ptr => tr,
                    want_ema, ema_line_ptr => ema
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the True Range Volatility Indicator (TRVI) on a single asset with `N` different
/// option sets simultaneously using SIMD parallelism.
///
/// TRVI is structurally identical to the Chaikin Volatility Indicator (CVI) but uses True Range
/// instead of the simple high-low spread, making it more sensitive to overnight gaps and
/// unusually large bars.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[high, low, close]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Pass `Some(&[true, true])` to also emit the `tr` and `ema`
///   intermediate series for every option lane; `None` disables all optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the TRVI series for option set `i`,
/// `outputs[i][1]` is `tr` (optional), `outputs[i][2]` is the EMA of TR (optional),
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [(f64, f64); N] = std::array::from_fn(|i| multiplier(options[i][0] as usize));
    let mut road_train = PrimeMover::<N, State, (f64, f64)>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = (false, false, false);
    for i in 0..N {
        let period = options[i][0] as usize;
        let [high, low, close] = *inputs;
        let asset_inputs = vec![high, low, close];

        let (trvi_line, (mut tr_line, mut ema_line)) = {
            let capacity = output_length(high.len(), options[i]);
            let tr_capacity = tr_output_length(high.len(), options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    tr_line: tr_capacity,
                    ema_line: ema_output_length(tr_capacity, options[i])
                ),
            )
        };

        let state = State::init_state(inputs, period, &mut tr_line, &mut ema_line);
        let mut starts = [0; 3];
        (starts[1], starts[2]) = crate::slice_outputs_start!(trvi_line.len(), tr_line, ema_line);

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(tr_line, ema_line);
        }
        let mut output_buffer = vec![trvi_line, tr_line, ema_line];

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
            period * 2 - 1,
            0,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = TrviDriver {
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, multipliers) in states_vec.into_iter().zip(params.into_iter()) {
        states.push(IndicatorState::new(state, multipliers));
    }
    Ok((output_buffers, states))
}
