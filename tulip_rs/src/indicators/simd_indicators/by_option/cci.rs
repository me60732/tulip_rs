//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};

use crate::indicators::simd_indicators::cci_simd::{options::SimdState, TSimdState, TState};
use crate::indicators::{
    cci::{Cci, Indicator, IndicatorState, State, INPUTS, OPTIONS},
    md::Md,
};

/// SIMD driver for the Commodity Channel Index (CCI) indicator, processing `N` option-set lanes per scheduling epoch.
struct CciDriver {
    want_optional_outputs: (bool, bool, bool, bool),
}

impl Driver<State<Warm>> for CciDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = outputs[0][0].len();

        let mut state = SimdState::<N>::from_states(&mut states);
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
            let inputs = unsafe {
                (
                    *high_ptrs[0].add(i),
                    *low_ptrs[0].add(i),
                    *close_ptrs[0].add(i),
                )
            };

            let (cci, sma, md, typprice) = state.calc(inputs);
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

/// Calculates the Commodity Channel Index (CCI) on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[high, low, close]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Optional output flags: `[want_sma, want_md, want_typprice]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[cci, sma?, md?, typprice?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Cci::min_data)?;
    validate_options(options, None)?;
    let mut road_train = PrimeMover::<N, State<Warm>>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = (false, false, false, false);
    for i in 0..N {
        let period = options[i][0] as usize;
        let asset_inputs = vec![
            inputs[0], // high
            inputs[1], // low
            inputs[2], // close
        ];

        let (cci_line, mut typprice_line, mut sma_line, mut md_line);
        {
            let len = inputs[0].len();
            let capacity = Cci::output_length(len, options[i]);
            let md_capacity = Md::output_length(len, options[i]);
            cci_line = crate::uninit_vec!(f64, capacity);
            (sma_line, md_line, typprice_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                sma_line: md_capacity,
                md_line: md_capacity,
                typprice_line: len
            );
        };

        let state = State::init_state(
            inputs[0], // high
            inputs[1], // low
            inputs[2], // close
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
                    output_buffer.len() - starts[j],           // slice to
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
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (state, period) in states_vec.into_iter().zip(options.iter()) {
        states.push(IndicatorState::new(state, period[0] as usize));
    }
    Ok((output_buffers, states))
}
