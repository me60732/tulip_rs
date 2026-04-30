//use crate::common::validate_inputs;
use crate::indicators::hma::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::hma_simd::assets::{calc_unchecked_simd, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

struct HmaDriver {
    multipliers: (f64, f64, (f64, f64, f64), (f64, f64, f64)),
    period: usize,
    period2: usize,
}

impl Driver<State> for HmaDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::new(&mut states);
        let len = inputs[0][0].len();
        //multipliers: (f64, f64, (f64, f64, f64), (f64, f64, f64)),
        let multipliers = (
            Simd::splat(self.multipliers.0),
            Simd::splat(self.multipliers.1),
            (
                Simd::splat(self.multipliers.2 .0),
                Simd::splat(self.multipliers.2 .1),
                Simd::splat(self.multipliers.2 .2),
            ),
            (
                Simd::splat(self.multipliers.3 .0),
                Simd::splat(self.multipliers.3 .1),
                Simd::splat(self.multipliers.3 .2),
            ),
        );

        //collect outputs
        let hma_line_ptr = crate::extract_output_ptrs!(outputs, N, hma_line_ptr);

        // Optimization 2: Pre-compute all input and output pointers
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.period..len).enumerate() {
            // Get inputs arrays for stocks
            let (real, prev_real, prev_real2) = crate::extract_simd_at_indices!(N, real_ptrs,
                real @ i,
                prev_real @ j,// i - self.period,
                prev_real2 @ i- self.period2
            );

            let hma = unsafe {
                calc_unchecked_simd(&mut state, real, prev_real, prev_real2, multipliers)
            };
            //unsafe { calc_simd(&mut state, high, low, close, multiplier) };
            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                hma_line_ptr => hma
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;
    let period2 = period / 2;
    let multipliers = multiplier(period);

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // real
        ];

        let hma_line = {
            let capacity = output_length(inputs[i][0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        let (start, state) = State::init_state(inputs[i][0], period);

        let mut output_buffer = vec![hma_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(0), //slice from
                    output_buffer.len(),               // slice to
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

    let mut driver = HmaDriver {
        multipliers,
        period,
        period2,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            inputs[i][0],
            state,
            period,
            period2,
            multipliers,
        ));
    }
    Ok((output_buffers, states))
}
