//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::dpo::{Dpo, Indicator, IndicatorState, State, INPUTS, OPTIONS};
use crate::indicators::simd_indicators::dpo_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver for the Detrended Price Oscillator (DPO) indicator, processing `N` option-set lanes per scheduling epoch.
struct DpoDriver {
    want_sma: bool,
}

impl Driver<State<Warm>, (usize, usize)> for DpoDriver {
    /// Processes one epoch of output bars for `N` option-set lanes simultaneously using SIMD. Reads the shared input, applies each lane's options, writes outputs, and updates per-lane states.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        options: Vec<Option<&(usize, usize)>>,
    ) {
        let len = outputs[0][0].len();

        let want_sma = self.want_sma;
        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::<N>::from_states(&mut states);
        let mut i = [0usize; N];
        let mut dpo_idx = [0usize; N];

        for (lane, option) in options.iter().enumerate() {
            if let Some(&(period, dpo_period)) = option {
                dpo_idx[lane] = period - dpo_period;
                i[lane] = period;
            }
        }

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs: [*const f64; N] =
            std::array::from_fn(|j| unsafe { inputs.get_unchecked(j).get_unchecked(0).as_ptr() });

        let (dpo_line_ptrs, sma_line_ptrs) =
            crate::extract_output_ptrs!(outputs, N, dpo_ptrs, sma_ptrs);
        // Optimization 3: Simplified main loop with pre-computed offsets
        for j in 0..len {
            let (new_vals, dpo_vals) = crate::extract_simd_at_indices_array!(N, input_ptrs,
                current @ i,
                dpo @ dpo_idx
            );
            let old_vals = crate::extract_simd_inputs_at_index!(j, N,
                old @ input_ptrs
            );

            let (dpo, sma) = state.calc((new_vals, old_vals, dpo_vals));

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                dpo_line_ptrs => dpo
            );
            crate::store_simd_optional_outputs!(j, N,
                want_sma, sma_line_ptrs => sma
            );

            for (i, dpo_idx) in i.iter_mut().zip(dpo_idx.iter_mut()) {
                *i += 1;
                *dpo_idx += 1;
            }
        }

        state.write_states(&mut states);
    }
}

/// Calculates the Detrended Price Oscillator (DPO) on a single asset with `N` different option sets
/// simultaneously using SIMD parallelism.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS]`), containing
///   `[real]`.
/// * `options` - An array of `N` option sets, one per SIMD lane: `[period]`.
/// * `optional_outputs` - Optional output flags: `[want_sma]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[dpo, sma?]`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS>(inputs, options, Dpo::min_data)?;
    validate_options(options, None)?;
    let params: [(usize, usize); N] =
        std::array::from_fn(|i| (options[i][0] as usize, options[i][0] as usize / 2 + 1));

    let mut road_train = PrimeMover::<N, State<Warm>, (usize, usize)>::new();

    let mut want_sma = false;
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![inputs[0]];
        let (dpo_line, sma_line) = {
            let len = inputs[0].len();
            let capacity = Dpo::output_length(len, options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    sma_line: capacity
                ),
            )
        };
        let period = options[i][0] as usize;
        let state = State::init_state(inputs[0], period);

        if i == 0 {
            (_, want_sma) = crate::calc_want_flags!(sma_line);
        }
        let mut output_buffer = vec![dpo_line, sma_line];

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
    let mut driver = DpoDriver { want_sma };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        let (period, dpo_period) = params[i];
        states.push(IndicatorState::new(
            unsafe { inputs.get_unchecked(0) },
            state,
            period,
            dpo_period,
        ));
    }
    Ok((output_buffers, states))
}
