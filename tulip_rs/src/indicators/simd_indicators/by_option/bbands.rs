//use crate::common::validate_inputs;
use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::bbands::{
    min_data, multiplier, output_length, validate_options as vo, IndicatorState, State,
    INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::indicators::simd_indicators::{bbands_simd::calc_simd, stddev_simd::SimdState};
use crate::types::IndicatorError;
use std::simd::Simd;

struct BbandsDriver {}

impl Driver<State, (f64, usize, f64)> for BbandsDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(f64, usize, f64)>>,
    ) {
        let len = outputs[0][0].len();

        let mut i = [0usize; N];
        let (stddev_simd, multiplier_simd) = {
            let mut stddevs = [0.0; N];
            let mut multipliers = [0.0; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&(multiplier, period, stddev)) = option {
                    multipliers[lane] = multiplier;
                    i[lane] = period;
                    stddevs[lane] = stddev;
                }
            }
            (Simd::from_array(stddevs), Simd::from_array(multipliers))
        };

        // Optimization 1: Direct array construction instead of collect+try_into
        let mut state = SimdState::new(&states);

        // Optimization 2: Pre-compute all input and output pointers
        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);

        let (lower_band_ptr, middle_band_ptr, upper_band_ptr) = crate::extract_output_ptrs!(
            outputs,
            N,
            lower_band_ptr,
            middle_band_ptr,
            upper_band_ptr
        );

        // Optimization 3: Simplified main loop with pre-computed offsets
        for j in 0..len {
            let old_vals = crate::extract_simd_inputs_at_index!(j, N,
                old @ input_ptrs
            );
            let new_vals = crate::extract_simd_inputs_at_index_array!(i, N,
                new @ input_ptrs
            );

            let (lower_band, middle_band, upper_band) =
                calc_simd(&mut state, stddev_simd, new_vals, old_vals, multiplier_simd);

            crate::write_simd_at_indices!(N, j,
                lower_band_ptr => lower_band,
                middle_band_ptr => middle_band,
                upper_band_ptr => upper_band
            );

            for i in i.iter_mut() {
                *i += 1;
            }
        }

        state.write_states(&mut states);
    }
}

pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH], //stock[ fields [ field [f64] ] ]
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, Some(vo))?;
    let mut road_train = PrimeMover::<N, State, (f64, usize, f64)>::new();

    let mut params = [(0.0, 0usize, 0.0); N];
    for i in 0..N {
        let period = options[i][0] as usize;
        let stddev = options[i][1];
        params[i] = (multiplier(period), period, stddev);
    }
    let mut output_buffers = Vec::with_capacity(N);

    for i in 0..N {
        let period = options[i][0] as usize;
        let asset_inputs = vec![
            inputs[0], // real
        ];

        let (middle_band, upper_band, lower_band) = {
            let capacity = output_length(inputs[0].len(), options[i]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        let state = State::init_state(inputs[0], period);

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
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = BbandsDriver {};
    let states_vec = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in states_vec.into_iter().enumerate() {
        let (multiplier, period, stddev) = params[i];
        states.push(IndicatorState::new(
            inputs[0], state, period, multiplier, stddev,
        ));
    }
    Ok((output_buffers, states))
}
