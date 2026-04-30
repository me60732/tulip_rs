//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::wad_simd::SimdState;
use crate::indicators::wad::{
    min_data, output_length, IndicatorState as State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::types::IndicatorError;
use std::simd::Simd;
struct WadDriver;

impl Driver<State> for WadDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let mut state = SimdState::new(&states);
        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs);

        let output_ptrs = crate::extract_output_ptrs!(outputs, N, output_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            let (high, low, close) = crate::extract_simd_inputs_at_index!(i, N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs
            );

            let wad = state.calc_simd(high, low, close);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                output_ptrs => wad
            );
        }

        state.write_states(&mut states);
    }
}

pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<State>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(_options))?;
    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = output_length(inputs[i][0].len(), _options);
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    for i in 0..N {
        let asset_inputs = vec![inputs[i][0], inputs[i][1], inputs[i][2]];
        let state = State::new(inputs[i][2][0], 0.0);
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
                1,
                0,
                state,
                None,
            ));
        }
    }
    let mut driver = WadDriver;
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}

/*pub fn indicator_by_assets_from_state<const N: usize>(
    inputs: &[ &[ &[f64]; INPUTS_WIDTH]; N],
    states: &mut [IndicatorState; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<[Vec<Vec<f64>>; N], IndicatorError>
{
    let len = inputs[0][0].len();

    // Validate all inputs have same length
    for i in 0..N {
        if inputs[i][0].len() != len {
            return Err(IndicatorError::InvalidInputs);
        }
    }

    // Extract EMAs and multipliers from states
    let mut emas = Simd::from_array(std::array::from_fn(|i| states[i].get_ema()));
    let multipliers = states[0].get_multipliers();
    let multipliers_simd = (Simd::splat(multipliers.0), Simd::splat(multipliers.1));

    // Create output arrays and process directly
    let mut ema_lines: [Vec<Vec<f64>>; N] = std::array::from_fn(|_| {
        vec![crate::uninit_vec!(f64, len)]
    });

    for i in 0..len {
        //let values: [f64; N] = (0..N).map(|j| inputs[j][0][i]).collect::<Vec<_>>().try_into().unwrap();
        let values: [f64; N] = std::array::from_fn(|j| inputs[j][0][i]);

        let vals = Simd::from_array(values);
        emas = calc_simd(vals, emas, multipliers_simd);
        let outputs = emas.to_array();
        for j in 0..N {
            unsafe { *ema_lines[j].get_unchecked_mut(0).get_unchecked_mut(i) = outputs[j] }
        }
    }

    // Update states with final EMA values
    let final_emas = emas.to_array();
    for i in 0..N {
        states[i].set_ema(final_emas[i]);
    }

    Ok(ema_lines)
}*/
