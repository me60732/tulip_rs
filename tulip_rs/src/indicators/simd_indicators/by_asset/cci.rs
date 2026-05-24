//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

use crate::indicators::simd_indicators::cci_simd::asset::SimdState;
use crate::indicators::{
    cci::{
        min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
    },
    md::output_length as md_output_length,
};

/// SIMD driver that advances the Commodity Channel Index (CCI) across `N` asset lanes per
/// scheduling epoch.
struct CciDriver {
    /// Pre-computed `1.0 / (0.015 * period)` CCI scaling factor.
    multiplier: f64,
    /// Optional output flags: `(has_optional, want_sma, want_md, want_typprice)`.
    want_optional_outputs: (bool, bool, bool, bool),
}

impl Driver<State> for CciDriver {
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
        let mut state = SimdState::<N>::new(&mut states);
        let len = inputs[0][0].len();

        let multiplier = Simd::splat(self.multiplier);
        let (has_optional, want_sma, want_md, want_typprice) = self.want_optional_outputs;

        //collect outputs
        let (cci_line_ptr, sma_line_ptr, md_line_ptr, typprice_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            cci_line_ptr,
            sma_line_ptr,
            md_line_ptr,
            typprice_line_ptr
        );

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

            let (cci, sma, md, typprice) =
                unsafe { state.calc_unchecked_simd(high, low, close, multiplier) };
            //unsafe { calc_simd(&mut state, high, low, close, multiplier) };
            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                cci_line_ptr => cci
            );
            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_sma, sma_line_ptr => sma,
                    want_md, md_line_ptr => md,
                    want_typprice, typprice_line_ptr => typprice
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Commodity Channel Index (CCI) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// CCI measures the deviation of the typical price from its simple moving average, normalised
/// by the mean deviation. All assets share the same `options`. Uses the [`PrimeMover`] scheduler
/// to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close]` for asset `i`.
/// * `options` - Shared options applied to all `N` assets: `[period]`.
/// * `optional_outputs` - Optional output flags: `[want_sma, want_md, want_typprice]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[cci, sma?, md?, typprice?]`
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
    let multiplier = multiplier(period);

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = (false, false, false, false);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // high
            inputs[i][1], // low
            inputs[i][2], // close
        ];

        let (cci_line, mut typprice_line, mut sma_line, mut md_line);
        {
            let len = inputs[i][0].len();
            let capacity = output_length(len, options);
            let md_capacity = md_output_length(len, options);
            cci_line = crate::uninit_vec!(f64, capacity);
            (sma_line, md_line, typprice_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                sma_line: md_capacity,
                md_line: md_capacity,
                typprice_line: len
            );
        };

        let state = State::init_state(
            inputs[i][0], // high
            inputs[i][1], // low
            inputs[i][2], // close
            period,
            (&mut sma_line, &mut md_line, &mut typprice_line),
        );

        if i == 0 {
            want_optional_outputs = crate::calc_want_flags!(sma_line, md_line, typprice_line);
        }
        let mut starts = [0; 4];
        (starts[1], starts[2], starts[3]) =
            crate::slice_outputs_start!(cci_line.len(), sma_line, md_line, typprice_line);

        let mut output_buffer = vec![cci_line, sma_line, md_line, typprice_line];

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
            period * 2 - 2,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = CciDriver {
        multiplier,
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state, multiplier, period));
    }
    Ok((output_buffers, states))
}
