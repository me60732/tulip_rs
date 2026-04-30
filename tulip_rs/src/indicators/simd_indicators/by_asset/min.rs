//use crate::common::validate_inputs;
use crate::indicators::min::{
    min_data, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::min_simd::{assets::Calc, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};

struct MinDriver {
    periods: (usize, usize),
}

impl Driver<State> for MinDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        //collect outputs
        let min_line_ptr = crate::extract_output_ptrs!(outputs, N, min_line_ptr);
        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let mut state = SimdState::new(&states);
        let (period, look_back) = self.periods;
        //let current: Vec<Simd<f64, N>> = crate::create_simd_vec_from_inputs!(real_ptrs, N, len);
        match period {
            1..=14 => {
                for (j, i) in (self.periods.1..len).enumerate() {
                    let (min, _) =
                        unsafe { state.calc_unchecked_simd::<1>(real_ptrs, i, look_back) };

                    // Store results using pre-computed pointers
                    crate::write_simd_at_indices!(N, j,
                        min_line_ptr => min
                    );
                }
            }
            /*15..=24 => {
                for (j, i) in (self.periods.1..len).enumerate() {
                    let (min, _) =
                        unsafe { state.calc_unchecked_simd::<4>(real_ptrs, i, self.periods) };

                    // Store results using pre-computed pointers
                    crate::write_simd_at_indices!(N, j,
                        min_line_ptr => min
                    );
                }
            }*/
            _ => {
                for (j, i) in (self.periods.1..len).enumerate() {
                    let (min, _) =
                        unsafe { state.calc_unchecked_simd::<8>(real_ptrs, i, look_back) };

                    // Store results using pre-computed pointers
                    crate::write_simd_at_indices!(N, j,
                        min_line_ptr => min
                    );
                }
            }
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
    let periods = (options[0] as usize, options[0] as usize - 1);

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // real
        ];

        let min_line = {
            let len = inputs[i][0].len();
            let capacity = output_length(len, options);
            crate::uninit_vec!(f64, capacity)
        };

        let state = State::new(
            inputs[i][0][0], // real
            periods.1,
        );

        let mut output_buffer = vec![min_line];

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
            periods.1,
            periods.1,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = MinDriver { periods };
    let states_vec = road_train.drive(&mut driver);
    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(inputs[i][0], state, periods));
    }
    Ok((output_buffers, states))
}
