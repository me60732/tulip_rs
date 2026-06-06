//use crate::common::validate_inputs;
use crate::common::validate_options;
use crate::common_simd::assets::validate_inputs;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use std::simd::Simd;

use crate::indicators::elderray::{
    min_data, output_length, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH,
};
//use crate::indicators::ad::output_length;
use crate::indicators::simd_indicators::{by_asset::ema::init_state, elderray_simd::calc_simd};

/// SIMD driver that advances Elder-ray across `N` asset lanes per scheduling epoch.
struct ElderRayDriver {
    multipliers: (f64, f64),
    want_optional_outputs: bool,
}

impl Driver<f64> for ElderRayDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// For each bar, computes `bull = high − EMA` and `bear = low − EMA` across all `N`
    /// asset lanes, writes bull and bear to their output buffers, optionally writes the
    /// updated EMA, and updates `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut f64>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Direct array construction
        let mut emas = Simd::<f64, N>::from_array(std::array::from_fn(|i| unsafe {
            **states.get_unchecked(i)
        }));

        let multipliers = (
            Simd::splat(self.multipliers.0),
            Simd::splat(self.multipliers.1),
        );

        // Pre-compute pointers for maximum efficiency
        let (high_ptrs, low_ptrs, close_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high, low, close);
        let (bull_line_ptr, bear_line_ptr, ema_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, bull, bear, ema);
        let want_ema = self.want_optional_outputs;
        // Optimized main loop with minimal overhead
        for i in 0..len {
            let (high, low, close) = crate::extract_simd_inputs_at_index!(i, N,
                h @ high_ptrs,
                l @ low_ptrs,
                c @ close_ptrs
            );
            let (bull, bear);
            (bull, bear, emas) = calc_simd(high, low, close, emas, multipliers);

            crate::write_simd_at_indices!(N, i,
                bull_line_ptr => bull,
                bear_line_ptr => bear
            );
            crate::store_simd_optional_outputs!(i, N,
                want_ema, ema_line_ptr => emas
            );
        }

        // Update states efficiently
        let final_emas = emas.to_array();
        for (i, state) in states.iter_mut().enumerate() {
            **state = final_emas[i];
        }
    }
}

/// Calculates Elder-ray for `N` assets simultaneously using SIMD parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close]` for asset `i`.
/// * `options` - Shared options slice; `options[0]` is the EMA period.
/// * `optional_outputs` - Pass `Some(&[true])` to also populate the EMA line for every asset.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` is `[bull, bear, ema]` for asset `i`
/// (the `ema` vec is empty unless `optional_outputs` enables it) and `states[i]`
/// is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;

    //init ema, sliced inputs and multipliers
    let (emas, multipliers) = {
        let close: [&[f64]; N] = std::array::from_fn(|i| inputs[i][2]);
        init_state(&close, period)
    };

    let mut road_train = PrimeMover::<N, f64>::new();
    let mut want_optional_outputs = false;
    let mut output_buffers = Vec::with_capacity(N);
    for i in 0..N {
        let [high, low, close] = *inputs[i];
        let asset_inputs = vec![high, low, close];
        let (bull_line, bear_line, ema_line) = {
            let capacity = output_length(inputs[i][0].len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    ema_line: capacity
                )
            )
        };

        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(ema_line);
        }
        let mut output_buffer = vec![bull_line, bear_line, ema_line];
        let mut asset_outputs = Vec::with_capacity(output_buffers.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(0), //slice from
                    output_buffer.len(),                       // slice to
                ));
            }
        }
        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period,
            0,
            emas[i],
            None,
        ));
        output_buffers.push(output_buffer);
    }
    let mut driver = ElderRayDriver {
        multipliers,
        want_optional_outputs,
    };
    let emas = road_train.drive(&mut driver);

    let mut states = Vec::with_capacity(N);
    for ema in emas {
        states.push(IndicatorState::new(ema, multipliers));
    }
    Ok((output_buffers, states))
}
