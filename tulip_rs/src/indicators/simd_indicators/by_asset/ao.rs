//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::ao_simd::SimdState;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::{
    ao::{
        min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, LONG_PERIOD,
        OPTIONS_WIDTH, SHORT_PERIOD,
    },
    sma::output_length as sma_output_length,
};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver that advances the Awesome Oscillator (AO) across `N` asset lanes per scheduling
/// epoch.
struct AoDriver {
    /// SMA scaling factors `(short_multiplier, long_multiplier)` for the 5- and 34-bar windows.
    multipliers: (f64, f64),
    /// Optional output flags: `(has_optional, want_short_sma, want_long_sma, want_medprice)`.
    want_optional_outputs: (bool, bool, bool, bool),
}

impl Driver<State, ()> for AoDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][field]` (high, low), writes to `outputs[asset][output]`,
    /// and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::new(&mut states);
        let len = inputs[0][0].len();
        let multipliers = (
            Simd::splat(self.multipliers.0),
            Simd::splat(self.multipliers.1),
        );
        let (has_optional, want_short_sma, want_long_sma, want_medprice) =
            self.want_optional_outputs;
        // Optimization 1: Direct array construction instead of collect+try_into

        //collect outputs
        let (ao_line_ptr, short_sma_line_ptr, long_sma_line_ptr, medprice_line_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            ao_line_ptr,
            short_sma_line_ptr,
            long_sma_line_ptr,
            medprice_line_ptr
        );

        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs) = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let (high, low) = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs
            );

            let (ao, short_sma, long_sma, medprice) =
                unsafe { state.calc_unchecked_simd(high, low, multipliers) };

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                ao_line_ptr => ao
            );

            if has_optional {
                crate::store_simd_optional_outputs!(i, N,
                    want_short_sma, short_sma_line_ptr => short_sma,
                    want_long_sma, long_sma_line_ptr => long_sma,
                    want_medprice, medprice_line_ptr => medprice
                );
            }
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Awesome Oscillator (AO) for `N` assets simultaneously using SIMD parallelism.
///
/// AO uses fixed short (5-bar) and long (34-bar) SMA windows and requires no configurable
/// options. Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low]` for asset `i`.
/// * `options` - Unused; AO uses fixed-length SMA windows.
/// * `optional_outputs` - Optional output flags:
///   `[want_short_sma, want_long_sma, want_medprice]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains `[ao, short_sma?, long_sma?, medprice?]`
/// for asset `i` and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    _options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(_options))?;
    let multipliers = multiplier((SHORT_PERIOD, LONG_PERIOD));

    let mut road_train = PrimeMover::<N, State>::new();
    let mut want_optional_outputs = (false, false, false, false);
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // high
            inputs[i][1], // low
        ];

        let ao_capacity = output_length(inputs[i][0].len(), _options);
        let ao_line = crate::uninit_vec!(f64, ao_capacity);

        let (mut short_sma_line, long_sma_line, mut medprice_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false, false],
            short_sma_line: sma_output_length(inputs[i][0].len(), &[SHORT_PERIOD as f64]),
            long_ema_line: ao_capacity,
            medprice: inputs[i][0].len()
        );

        let state = State::init_state(
            (inputs[i][0], inputs[i][1]),
            &mut medprice_line,
            &mut short_sma_line,
        );

        let mut starts = [0; 4];
        (starts[1], starts[3]) =
            crate::slice_outputs_start!(ao_capacity, short_sma_line, medprice_line);
        if i == 0 {
            want_optional_outputs =
                crate::calc_want_flags!(short_sma_line, long_sma_line, medprice_line);
        }

        let mut output_buffer = vec![ao_line, short_sma_line, long_sma_line, medprice_line];

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
            LONG_PERIOD,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = AoDriver {
        multipliers,
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for state in states_vec.into_iter() {
        states.push(IndicatorState::new(state, multipliers));
    }
    Ok((output_buffers, states))
}
