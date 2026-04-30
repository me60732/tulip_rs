//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::bbands::{
    min_data, output_length, validate_options, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::{bbands_simd::calc_simd, stddev_simd::SimdState};
use crate::types::IndicatorError;
use std::simd::Simd;
/*pub use crate::indicators::simd::{
    bbands_simd::calc_simd,
    stddev_simd::{calc_simd as calc_stddev_simd, SimdState},
};*/

struct BbandsDriver {
    multiplier: f64,
    period: usize,
    std_dev: f64,
}

impl Driver<State> for BbandsDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        let std_dev = Simd::splat(self.std_dev);
        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::new(&states);

        let multiplier_simd = Simd::splat(self.multiplier);

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let (lower_band_ptr, middle_band_ptr, upper_band_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            lower_band_ptr,
            middle_band_ptr,
            upper_band_ptr
        );

        // Optimization 3: Simplified main loop with pre-computed offsets
        for (j, i) in (self.period..len).enumerate() {
            // Get new and old values using pre-computed pointers
            let (old_vals, new_vals) = crate::extract_simd_at_indices!(N, input_ptrs,
                old_vals @ j,
                new_vals @ i
            );

            let (lower_band, middle_band, upper_band) =
                calc_simd(&mut state, std_dev, new_vals, old_vals, multiplier_simd);

            crate::write_simd_at_indices!(N, j,
                lower_band_ptr => lower_band,
                middle_band_ptr => middle_band,
                upper_band_ptr => upper_band
            );
        }

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
    let std_dev = options[1];

    let real: [&[f64]; N] = std::array::from_fn(|i| inputs[i][0]);
    //init ema, sliced inputs and multipliers
    let (simd_state, multiplier) = SimdState::init_state(&real, period);
    let states = simd_state.to_states();

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);
    for (i, state) in states.into_iter().enumerate() {
        let asset_inputs = vec![inputs[i][0]];

        let (middle_band, upper_band, lower_band) = {
            let capacity = output_length(inputs[i][0].len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let mut output_buffer = vec![lower_band, middle_band, upper_band];

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
            period,
            period,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = BbandsDriver {
        multiplier,
        period,
        std_dev,
    };
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        states.push(IndicatorState::new(
            unsafe { inputs.get_unchecked(i).get_unchecked(0) },
            state,
            period,
            multiplier,
            std_dev,
        ));
    }
    Ok((output_buffers, states))
}
