//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::chandelierexit_simd::{assets::Calc, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::{
    chandelierexit::{
        min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
    },
    tr::output_length as tr_output_length,
};
use crate::types::IndicatorError;
use std::simd::Simd;
/// SIMD driver that advances the Aroon Indicator (AROON) across `N` asset lanes per scheduling
/// epoch.
struct ChandelierExitDriver {
    periods: (usize, usize),
    multipliers: (f64, (f64, f64)),
    want_optional_outputs: (bool, bool, bool),
}

impl Driver<State> for ChandelierExitDriver {
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
        let data_len = inputs[0][0].len();

        //collect outputs
        let outputs =
            crate::extract_output_ptrs!(outputs, N, long_ptr, short_ptr, atr_line_ptr, tr_line_ptr);
        let inputs = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);
        let mut state = SimdState::new(&mut states);

        match self.periods.0 {
            1..=14 => {
                self.cycle::<N, 1>(inputs, outputs, data_len, &mut state);
            }
            _ => {
                self.cycle::<N, 8>(inputs, outputs, data_len, &mut state);
            }
        }
        // Update states efficiently
        state.write_states(&mut states);
    }
}
impl ChandelierExitDriver {
    fn cycle<const N: usize, const CHUNK_SIZE: usize>(
        &self,
        inputs: ([*const f64; N], [*const f64; N], [*const f64; N]),
        outputs: ([*mut f64; N], [*mut f64; N], [*mut f64; N], [*mut f64; N]),
        data_len: usize,
        state: &mut SimdState<N>,
    ) {
        let (high_ptrs, low_ptrs, close_ptrs) = inputs;
        let (long_line_ptr, short_line_ptr, atr_line_ptr, tr_line_ptr) = outputs;
        let (period, look_back) = self.periods;
        let multipliers = (
            Simd::splat(self.multipliers.0),
            (
                Simd::splat(self.multipliers.1 .0),
                Simd::splat(self.multipliers.1 .1),
            ),
        );
        let (has_optional, want_atr, want_tr) = self.want_optional_outputs;
        for (j, i) in (period..data_len).enumerate() {
            let close = crate::extract_simd_inputs_at_index!(i, N,
                close @ close_ptrs
            );
            let (long, short, atr, tr) = unsafe {
                state.calc_unchecked_simd::<N>(
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
                long_line_ptr => long,
                short_line_ptr => short
            );
            if has_optional {
                crate::store_simd_optional_outputs!(j, N,
                    want_atr, atr_line_ptr => atr,
                    want_tr, tr_line_ptr => tr
                );
            }
        }
    }
}

pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let multipliers = multiplier(period);
    let step = options[1];
    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = (false, false, false);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // high
            inputs[i][1], // low
            inputs[i][2], // close
        ];

        let (long_line, short_line, (atr_line, mut tr_line)) = {
            let len = inputs[i][0].len();
            let capacity = output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    atr_line: capacity,
                    tr_line: tr_output_length(len, options)
                ),
            )
        };

        let state = State::new(
            inputs[i][0],
            inputs[i][1],
            inputs[i][2],
            period,
            period - 1,
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
            period,
            period,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = ChandelierExitDriver {
        periods: (period, period - 1),
        multipliers: (step, multipliers),
        want_optional_outputs,
    };
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            inputs[i][0],
            inputs[i][1],
            state,
            driver.periods,
            driver.multipliers,
        ));
    }
    Ok((output_buffers, states))
}
