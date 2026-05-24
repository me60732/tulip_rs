//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::atr::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::atr_simd::SimdState;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::tr::output_length as tr_output_length;
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver that advances the Average True Range (ATR) across `N` asset lanes per scheduling
/// epoch.
struct AtrDriver {
    /// Pre-computed Wilder smoothing multiplier for the given period.
    multiplier: f64,
    /// Whether to also emit the raw True Range (TR) output.
    want_optional_outputs: bool,
}

impl Driver<State> for AtrDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][field]` (high, low, close), writes to
    /// `outputs[asset][output]`, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::new(&states);
        let len = inputs[0][0].len();
        let multipliers = Simd::splat(self.multiplier);

        //collect outputs
        let (atr_line_ptr, tr_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, atr_line_ptr, tr_line_ptr);

        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let (high, low, close) = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs
            );

            let (atr, tr) = state.calc_simd(high, low, close, multipliers);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                atr_line_ptr => atr
            );

            crate::store_simd_optional_outputs!(i, N,
                self.want_optional_outputs, tr_line_ptr => tr
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Average True Range (ATR) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// ATR smooths the True Range over a rolling period using Wilder's smoothing method.
/// All assets share the same `options`. Uses the [`PrimeMover`] scheduler to batch assets
/// into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close]` for asset `i`.
/// * `options` - Shared options applied to all `N` assets: `[period]`.
/// * `optional_outputs` - Optional output flags: `[want_tr]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[atr, tr?]`
/// for asset `i` and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;

    let (multiplier, _) = multiplier(period);

    let mut road_train = PrimeMover::<N, State>::new();
    let mut want_optional_outputs = false;
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // high
            inputs[i][1], // low
            inputs[i][2], // close
        ];

        let atr_capacity = output_length(inputs[i][0].len(), options);
        let atr_line = crate::uninit_vec!(f64, atr_capacity);

        let mut tr_line = crate::init_optional_outputs_eff!(
            optional_outputs, &[false],
            tr_line: tr_output_length(inputs[i][0].len(), options)
        );

        let state = State::init_state(
            inputs[i][0],
            inputs[i][1],
            inputs[i][2],
            period,
            &mut tr_line,
            false,
        );

        let mut starts = [0; 2];
        starts[1] = crate::slice_outputs_start!(atr_capacity, tr_line);
        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(tr_line);
        }

        let mut output_buffer = vec![atr_line, tr_line];

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
            period,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = AtrDriver {
        multiplier,
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state));
    }
    Ok((output_buffers, states))
}
