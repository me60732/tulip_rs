use crate::types::IndicatorError;

use crate::indicators::keltnerchannel::{indicator, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH};

// Change these lines in by_asset/keltnerchannel.rs:

/// Calculates the Keltner Channel for `N` assets by calling the scalar
/// [`indicator`] function for each asset independently.
///
/// No SIMD parallelism is used; each asset is processed sequentially.
/// The SIMD by-assets path was measured ~29% slower than sequential scalar
/// for this indicator due to 3 inputs × 3 outputs causing cache pressure that
/// outweighs the SIMD compute savings.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close]` for asset `i`.
/// * `options` - Shared parameter array: `options[0]` = period, `options[1]` = step multiplier.
/// * `optional_outputs` - Forwarded to the scalar `indicator`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains the Keltner Channel bands for asset `i`
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is too short or options are invalid.

pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    let mut all_outputs = Vec::with_capacity(N);
    let mut all_states = Vec::with_capacity(N);

    // Just call the scalar indicator N times, no roadtrain
    for input in inputs.iter() {
        let (outputs, state) = indicator(input, options, optional_outputs)?;
        all_outputs.push(outputs);
        all_states.push(state);
    }

    Ok((all_outputs, all_states))
}
