use crate::types::IndicatorError;

use crate::indicators::mom::{indicator, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH};

/// Calculates the Momentum (MOM) indicator for one asset with `N` different option sets.
///
/// This implementation calls the scalar [`indicator`] function `N` times — one per option set —
/// rather than using SIMD lanes. Use the `by_asset` variant for true SIMD parallelism across
/// multiple assets with shared options.
///
/// # Arguments
/// * `inputs` - Shared input: `inputs[0]` is the `real` price series.
/// * `options` - An array of `N` option sets; `options[i][0]` is the `period` for option set `i`.
/// * `_optional_outputs` - Unused; MOM has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the `mom` series for option set `i`
/// and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[&[f64; OPTIONS_WIDTH]; N],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    let mut all_outputs = Vec::with_capacity(N);
    let mut all_states = Vec::with_capacity(N);

    // Just call the scalar indicator N times, no simd
    for option in options.iter() {
        let (outputs, state) = indicator(inputs, option, _optional_outputs)?;
        all_outputs.push(outputs);
        all_states.push(state);
    }

    Ok((all_outputs, all_states))
}
