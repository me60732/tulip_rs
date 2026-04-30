//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::vhf_simd::{assets::Calc, SimdState};
use crate::indicators::vhf::{
    init_state, min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;
struct VhfDriver {
    period: usize,
}

impl Driver<State> for VhfDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        //collect outputs
        let vhf_line_ptr = crate::extract_output_ptrs!(outputs, N, vhf_line_ptr);
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let mut state = SimdState::new(&mut states);
        
        match self.period {
            1..=14 => {
                cycle::<N, 1>(real_ptrs, len, self.period, &mut state, vhf_line_ptr);
            }
            /*26..=40 => {
                cycle::<N, 4>(real_ptrs, len, self.period, &mut state, vhf_line_ptr);
            }*/
            _ => {
                cycle::<N, 8>(real_ptrs, len, self.period, &mut state, vhf_line_ptr);
            }
        }
        // Update states efficiently
        state.write_states(&mut states);
    }
}
fn cycle<const N: usize, const CHUNK_SIZE: usize>(real_ptrs: [*const f64; N], len: usize, period: usize, state: &mut SimdState<N>, vhf_line_ptr: [*mut f64; N]) {
    let look_back = period -1;
    for (j, i) in (period+1..len).enumerate() {
        let values = crate::extract_simd_at_indices!(N, real_ptrs,
            cur_vals @ i,
            prev_vals @ i-1,
            old_vals @ j+1,
            drop_vals @ j
        );

        let vhf = unsafe {
            state.calc_unchecked_simd::<CHUNK_SIZE>(values, real_ptrs, look_back, i)
        };

        // Store results using pre-computed pointers
        crate::write_simd_at_indices!(N, j,
            vhf_line_ptr => vhf
        );
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
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // real
        ];

        let mut vhf_line = {
            let len = inputs[i][0].len();
            let capacity = output_length(len, options);
            crate::uninit_vec!(f64, capacity)
        };
        let state = init_state(inputs[i][0], period, &mut vhf_line);

        let mut output_buffer = vec![vhf_line];

        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(1), //slice from
                    output_buffer.len(),               // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period + 1,
            period + 1,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = VhfDriver { period };
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(state, inputs[i][0], period));
    }
    Ok((output_buffers, states))
}
