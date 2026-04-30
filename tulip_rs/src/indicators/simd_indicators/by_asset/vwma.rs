//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::vwma::{
    min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::vwma_simd::SimdState;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
struct VwmaDriver {
    period: usize,
}

impl Driver<State> for VwmaDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::new(&states);

        // Optimization 2: Pre-compute all input and output pointers
        let (close_ptrs, volume_ptrs) =
            crate::extract_input_ptrs!(inputs, N, close_ptrs, volume_ptrs);

        let output_ptrs = crate::extract_output_ptrs!(outputs, N, output_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.period..len).enumerate() {
            let (prev_close, close) = crate::extract_simd_at_indices!(N, close_ptrs,
                prev_close @ j,
                close @ i
            );
            let (prev_volume, volume) = crate::extract_simd_at_indices!(N, volume_ptrs,
                prev_volume @ j,
                volume @ i
            );

            let vwma = state.calc_simd(close, volume, prev_close, prev_volume);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, j,
                output_ptrs => vwma
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
    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = output_length(inputs[i][0].len(), options);
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    for i in 0..N {
        let state = State::init_state(period, inputs[i][0], inputs[i][1]);
        let asset_inputs = vec![inputs[i][0], inputs[i][1]];
        unsafe {
            // Get a mutable reference to the output buffer for this asset
            let output_buffer = &mut output_buffers[i][0];
            let asset_outputs = vec![std::slice::from_raw_parts_mut(
                output_buffer.as_mut_ptr(),
                output_buffer.len(),
            )];

            road_train.add_asset(Asset::new(
                asset_inputs,
                asset_outputs,
                i,
                period,
                period,
                state,
                None,
            ));
        }
    }
    let mut driver = VwmaDriver { period };
    let states = road_train.drive(&mut driver);

    let mut indicator_states = Vec::with_capacity(N);

    for (i, state) in states.into_iter().enumerate() {
        indicator_states.push(IndicatorState::new(
            inputs[i][0],
            inputs[i][1],
            state,
            period,
        ));
    }
    Ok((output_buffers, indicator_states))
}
