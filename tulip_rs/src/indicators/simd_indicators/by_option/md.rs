use crate::types::IndicatorError;

use crate::indicators::md::{indicator, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH};

/// Computes the Mean Deviation (MD) for `N` independent option sets by running the scalar
/// indicator once per option set.
///
/// # Arguments
/// * `inputs` - The single asset's price series (`[&[f64]; INPUTS_WIDTH]`), containing
///   `[real]`.
/// * `options` - An array of `N` option sets: `[period]`.
/// * `optional_outputs` - Passed through to each scalar call; flags `[want_sma]`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i]` contains the output series for option set `i`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    let mut all_outputs = Vec::with_capacity(N);
    let mut all_states = Vec::with_capacity(N);

    // Just call the scalar indicator N times, no roadtrain
    for option in options.iter() {
        let (outputs, state) = indicator(inputs, option, optional_outputs)?;
        all_outputs.push(outputs);
        all_states.push(state);
    }

    Ok((all_outputs, all_states))
}
