use crate::types::IndicatorError;

use crate::indicators::wcprice::{indicator, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH};

/// Calculates the Weighted Close Price (WCPRICE) for `N` assets by calling the scalar
/// [`indicator`] function for each asset independently.
///
/// No SIMD parallelism is used; each asset is processed sequentially.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close]` for asset `i`.
/// * `_options` - Unused; WCPRICE has no configurable options.
/// * `_optional_outputs` - Unused; WCPRICE has no optional output lines.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the weighted-close-price series for asset `i`
/// and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input is invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N],
    _options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    let mut all_outputs = Vec::with_capacity(N);
    let mut all_states = Vec::with_capacity(N);

    // Just call the scalar indicator N times, no simd
    for input in inputs.iter() {
        let (outputs, state) = indicator(input, _options, _optional_outputs)?;
        all_outputs.push(outputs);
        all_states.push(state);
    }

    Ok((all_outputs, all_states))
}
