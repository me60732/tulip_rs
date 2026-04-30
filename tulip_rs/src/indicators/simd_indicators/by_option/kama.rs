use crate::common_simd::options::{validate_inputs, validate_options};
use crate::indicators::kama::{
    min_data, multiplier, output_length, IndicatorState, State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::kama_simd::{calc_simd, SimdState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

struct KamaDriver {}

impl Driver<State, (usize, (f64, f64))> for KamaDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        options: Vec<Option<&(usize, (f64, f64))>>,
    ) {
        let len = outputs[0][0].len();

        let input_ptrs = crate::extract_input_ptrs!(inputs, N, input_ptrs);
        let output_ptrs = crate::extract_output_ptrs!(outputs, N, output_ptrs);

        let (mut i, mut prev, mut old, multipliers_simd) = {
            let mut multipliers = ([0.0; N], [0.0; N]);
            let mut i = [0usize; N];
            let mut periods = [0usize; N];
            for (lane, option) in options.iter().enumerate() {
                if let Some(&(period, multiplier)) = option {
                    i[lane] = period + 1;
                    periods[lane] = period;
                    multipliers.0[lane] = multiplier.0;
                    multipliers.1[lane] = multiplier.1;
                }
            }
            (
                i,
                crate::extract_simd_inputs_at_index_array!(Simd::from_array(periods), N,
                    new @ input_ptrs
                ),
                crate::extract_simd_inputs_at_index!(0, N, real @ input_ptrs),
                (
                    Simd::from_array(multipliers.0),
                    Simd::from_array(multipliers.1),
                ),
            )
        };
        // Direct array construction
        let mut simd_state = SimdState::new(&states);

        // Optimized main loop with minimal overhead
        for j in 0..len {
            let value = crate::extract_simd_inputs_at_index_array!(i, N,
                new @ input_ptrs
            );
            let last = crate::extract_simd_inputs_at_index!(j+1, N, real @ input_ptrs);
            let kama = calc_simd(&mut simd_state, (value, prev, last, old), multipliers_simd);
            (prev, old) = (value, last);
            // Direct SIMD store if possible, otherwise individual stores
            crate::write_simd_at_indices!(N, j,
                output_ptrs => kama
            );
            for i in i.iter_mut() {
                *i += 1;
            }
        }

        simd_state.write_states(&mut states);
    }
}

pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<OPTIONS_WIDTH>(inputs, options, min_data)?;
    validate_options(options, None)?;
    let params: [(usize, (f64, f64)); N] =
        std::array::from_fn(|i| (options[i][0] as usize, multiplier()));
    // Create output buffers OUTSIDE the assets - these will be owned by this function
    let mut output_buffers = Vec::with_capacity(N);

    let mut road_train = PrimeMover::<N, State, (usize, (f64, f64))>::new();

    for i in 0..N {
        let len = inputs[0].len();
        let capacity = output_length(len, options[i]);
        let mut kama_line = crate::uninit_vec!(f64, capacity);
        let period = options[i][0] as usize;
        let state = State::init_state(inputs[0], period, &mut kama_line);
        let asset_inputs = vec![inputs[0]];

        let mut output_buffer = vec![kama_line];
        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        unsafe {
            //let slice_len = output_buffer.len() - starts[j];
            // Get a mutable reference to the output buffer for this asset
            let output_buffer = &mut output_buffer[0];
            asset_outputs.push(std::slice::from_raw_parts_mut(
                output_buffer.as_mut_ptr().add(1), //slice from
                output_buffer.len(),               // slice to
            ));
        }
        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period + 1,
            period + 1,
            state,
            Some(&params[i]),
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = KamaDriver {};
    let final_states = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for (i, state) in final_states.into_iter().enumerate() {
        let (period, multipliers) = params[i];
        states.push(IndicatorState::new(inputs[0], period, multipliers, state));
    }
    Ok((output_buffers, states))
}
