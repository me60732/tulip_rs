//use crate::common::validate_inputs;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::ad::{min_data, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH};
use crate::indicators::simd_indicators::ad_simd::calc_simd;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

/// SIMD driver that advances the Accumulation/Distribution (AD) line across `N` asset lanes
/// per scheduling epoch.
struct AdDriver;

impl Driver<f64, ()> for AdDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][field]` (high, low, close, volume), writes the running AD value
    /// to `outputs[asset][0]`, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut f64>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();
        // Optimization 1: Direct array construction instead of collect+try_into
        let mut ads = Simd::<f64, N>::from_array(std::array::from_fn(|i| unsafe {
            **states.get_unchecked(i)
        }));

        // Optimization 2: Pre-compute all input and output pointers
        let (high_ptrs, low_ptrs, close_ptrs, volume_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs, volume_ptrs);

        let ad_line_ptr = crate::extract_output_ptrs!(outputs, N, ad_line_ptr);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get new and old values using pre-computed pointers
            let (high, low, close, volume) = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs,
                volume @ volume_ptrs
            );

            ads = calc_simd(ads, high, low, close, volume);

            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                ad_line_ptr => ads
            );
        }

        // Update states efficiently
        let final_ads = ads.to_array();
        for (i, state) in states.iter_mut().enumerate().take(N) {
            **state = final_ads[i];
        }
    }
}

/// Calculates the Accumulation/Distribution (AD) line for `N` assets simultaneously using SIMD
/// parallelism.
///
/// AD requires no configurable options. Uses the [`PrimeMover`] scheduler to batch assets into
/// SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close, volume]` for asset `i`.
/// * `options` - Unused; AD has no configurable options.
/// * `optional_outputs` - Unused; AD produces only the single AD line output.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the AD line for asset `i`
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(_options))?;
    let ads = [0.0; N];
    let mut road_train = PrimeMover::<N, f64>::new();
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
            inputs[i][2], // close
            inputs[i][3], // volume
        ];
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
                0,
                0,
                ads[i],
                None,
            ));
        }
    }
    let mut driver = AdDriver {};
    let ads = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for &ad in ads.iter() {
        states.push(IndicatorState::new(ad));
    }
    Ok((output_buffers, states))
}

//const min: Simd<f64, 4> = Simd::from_array([f64::EPSILON;4]);
/*#[inline(always)]
pub fn calc_simd<const N: usize>(
    ad: Simd<f64, N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
    volume: Simd<f64, N>
) -> Simd<f64, N>
where
    LaneCount<N>:
{
    let range = high - low;
    let valid_mask = range.simd_ge(AdF64Constants::<N>::EPSILON);

    // Optimized: use const TWO instead of close + close
    let clv_factor = (close * <()>::TWO::<N> - low - high) * volume;
    let calculated_ad = ad + clv_factor / range;

    valid_mask.select(calculated_ad, ad)
}*/

/*#[inline(always)]
pub fn calc_simd<const N: usize>(
    ad: Simd<f64, N>,
    high: Simd<f64, N>,
    low: Simd<f64, N>,
    close: Simd<f64, N>,
    volume: Simd<f64, N>
) -> Simd<f64, N>
where
    LaneCount<N>:
{
    let range = high - low;
    let valid_mask = range.simd_ge(AdF64Constants::<N>::EPSILON);

    // Optimized: fewer SIMD operations, better instruction pipelining
    //let hl_sum = low - high;                          // 1 SIMD add
    let clv_factor = (close * AdF64Constants::<N>::TWO - low - high) * volume;  // 1 SIMD mul, 1 SIMD sub, 1 SIMD mul
    let calculated_ad = ad + clv_factor / range;      // 1 SIMD div, 1 SIMD add

    valid_mask.select(calculated_ad, ad)
}*/
