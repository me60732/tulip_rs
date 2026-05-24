use crate::types::IndicatorError;

use crate::indicators::typprice::{indicator, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH};

/// Calculates the Typical Price (TYPPRICE) for `N` assets by calling the scalar
/// [`indicator`] function for each asset independently.
///
/// No SIMD parallelism is used; each asset is processed sequentially.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close]` for asset `i`.
/// * `options` - Forwarded to the scalar `indicator`; TYPPRICE has no configurable options.
/// * `optional_outputs` - Forwarded to the scalar `indicator`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the typical-price series for asset `i`
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is invalid.
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
