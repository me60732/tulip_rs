//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::marketfi::{min_data, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH};
use crate::indicators::simd_indicators::marketfi_simd::calc_simd;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;
struct MarketfiDriver;

impl Driver<()> for MarketfiDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        _: Vec<&mut ()>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs, volume_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, volume_ptrs);

        let output_ptrs: [*mut f64; N] = std::array::from_fn(|j| unsafe {
            outputs
                .get_unchecked_mut(j)
                .get_unchecked_mut(0)
                .as_mut_ptr()
        });

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get new and old values using pre-computed pointers
            let (high, low, volume) = crate::extract_simd_inputs_at_index!(i, N,
                high @ high_ptrs,
                low @ low_ptrs,
                volume @ volume_ptrs
            );

            let marketfi = calc_simd(high, low, volume);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                output_ptrs => marketfi
            );
        }
    }
}

pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(_options))?;
    let mut road_train = PrimeMover::<N, ()>::new();
    let mut output_buffers: Vec<Vec<Vec<f64>>> = (0..N)
        .map(|i| {
            vec![{
                let capacity = inputs[i][0].len();
                crate::uninit_vec!(f64, capacity)
            }]
        })
        .collect();

    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // high
            inputs[i][1], // low
            inputs[i][2], // volume
        ];
        unsafe {
            // Get a mutable reference to the output buffer for this asset
            let output_buffer = &mut output_buffers[i][0];
            let asset_outputs = vec![std::slice::from_raw_parts_mut(
                output_buffer.as_mut_ptr(),
                output_buffer.len(),
            )];

            road_train.add_asset(Asset::new(asset_inputs, asset_outputs, i, 0, 0, (), None));
        }
    }
    let mut driver = MarketfiDriver {};
    road_train.drive(&mut driver);

    Ok((output_buffers, vec![IndicatorState; N]))
}
