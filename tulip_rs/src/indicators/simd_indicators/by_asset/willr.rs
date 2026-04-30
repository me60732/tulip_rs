//use crate::common::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::willr_simd::{assets::Calc, SimdState};
use crate::indicators::willr::{
    min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;
struct WillrDriver {
    period: usize,
}

impl Driver<State> for WillrDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        //collect outputs
        let willr_line_ptr = crate::extract_output_ptrs!(outputs, N, vhf_line_ptr);
        let inputs = crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);
        let mut state = SimdState::new(&mut states);
        //let look_back = self.period - 1;

        match self.period {
            1..=14 => {
                cycle::<N, 1>(inputs, self.period, &mut state, willr_line_ptr, len);
            }
            _ => {
                cycle::<N, 8>(inputs, self.period, &mut state, willr_line_ptr, len);
            }
        }
        // Update states efficiently
        state.write_states(&mut states);
    }
}
fn cycle<const N: usize, const CHUNK_SIZE: usize>(
    inputs: ([*const f64; N], [*const f64; N], [*const f64; N]),
    period: usize,
    state: &mut SimdState<N>,
    willr_line_ptr: [*mut f64; N],
    len: usize,
) {
    let look_back = period - 1;
    let (high_ptrs, low_ptrs, close_ptrs) = inputs;
    for (j, i) in (period..len).enumerate() {
        let close = crate::extract_simd_inputs_at_index!(i, N, close @ close_ptrs);

        let willr = unsafe {
            state.calc_unchecked_simd::<CHUNK_SIZE>(high_ptrs, low_ptrs, close, i, look_back)
        };

        // Store results using pre-computed pointers
        crate::write_simd_at_indices!(N, j,
            willr_line_ptr => willr
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
            inputs[i][0], // high
            inputs[i][1], // low
            inputs[i][2], // close
        ];

        let willr_line = {
            let len = inputs[i][0].len();
            let capacity = output_length(len, options);
            crate::uninit_vec!(f64, capacity)
        };
        let state = State::init_state(inputs[i][0], inputs[i][1], period);

        let mut output_buffer = vec![willr_line];

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
            period,
            period,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = WillrDriver { period };
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            state,
            inputs[i][0],
            inputs[i][1],
            period,
        ));
    }
    Ok((output_buffers, states))
}
