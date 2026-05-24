//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::stoch_simd::assets::SimdState;
use crate::indicators::stoch::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;
/// SIMD driver that advances the Stochastic Oscillator (STOCH) across `N` asset lanes per scheduling epoch.
struct StochDriver {
    k_period: usize,
    multipliers: (f64, f64),
}

impl Driver<State> for StochDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        //collect outputs
        let (k_line_ptr, d_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, k_line_ptr, d_line_ptr);
        let inputs = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);
        let mut state = SimdState::new(&mut states);

        match self.k_period {
            1..=14 => {
                cycle::<N, 1>(
                    inputs,
                    self.k_period,
                    &mut state,
                    k_line_ptr,
                    d_line_ptr,
                    len,
                    self.multipliers,
                );
            }
            _ => {
                cycle::<N, 8>(
                    inputs,
                    self.k_period,
                    &mut state,
                    k_line_ptr,
                    d_line_ptr,
                    len,
                    self.multipliers,
                );
            }
        }
        // Update states efficiently
        state.write_states(&mut states);
    }
}
fn cycle<const N: usize, const CHUNK_SIZE: usize>(
    inputs: ([*const f64; N], [*const f64; N], [*const f64; N]),
    k_period: usize,
    state: &mut SimdState<N>,
    k_line_ptr: [*mut f64; N],
    d_line_ptr: [*mut f64; N],
    len: usize,
    multipliers: (f64, f64),
) {
    let multipliers = (Simd::splat(multipliers.0), Simd::splat(multipliers.1));
    let look_back = k_period - 1;
    let (high_ptrs, low_ptrs, close_ptrs) = inputs;
    for (j, i) in (k_period..len).enumerate() {
        let close = crate::extract_simd_inputs_at_index!(i, N, close @ close_ptrs);

        let (k, d) = unsafe {
            state.calc_unchecked_simd::<CHUNK_SIZE>(
                high_ptrs,
                low_ptrs,
                close,
                i,
                look_back,
                multipliers,
            )
        };

        // Store results using pre-computed pointers
        crate::write_simd_at_indices!(N, j,
            k_line_ptr => k,
            d_line_ptr => d
        );
    }
}
/// Calculates the Stochastic Oscillator (STOCH) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// STOCH computes the fast %K and smoothed %D stochastic lines, expressing the closing
/// price relative to the high/low range over the look-back period.
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close]` for asset `i`.
/// * `options` - `[k_period, k_slow, d_period]` — the fast %K look-back period,
///   the slow %K smoothing period, and the %D smoothing period.
/// * `_optional_outputs` - Unused; STOCH produces no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the `stoch_k` line and
/// `outputs[i][1]` is the `stoch_d` line for asset `i`, and
/// `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let k_period = options[0] as usize;
    let multipliers = multiplier(options[1] as usize, options[2] as usize);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // high
            inputs[i][1], // low
            inputs[i][2], // close
        ];
        let mut starts = [0; 2];
        let (mut k_line, d_line, state, start);
        {
            let (k_capacity, d_capacity) = output_length(inputs[i][0].len(), options);
            k_line = crate::uninit_vec!(f64, k_capacity);
            d_line = crate::uninit_vec!(f64, d_capacity);

            let k_slow = options[1] as usize;
            let d_period = options[2] as usize;
            (state, starts[0], start) = State::init_state(
                (inputs[i][0], inputs[i][1], inputs[i][2]),
                k_period,
                k_slow,
                d_period,
                &mut k_line,
            );
        }

        let mut output_buffer = vec![k_line, d_line];

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
            start,
            period,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = StochDriver {
        k_period,
        multipliers,
    };
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            state,
            inputs[i][0],
            inputs[i][1],
            multipliers,
            k_period,
        ));
    }
    Ok((output_buffers, states))
}
