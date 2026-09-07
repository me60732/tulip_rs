use crate::types::IndicatorError;

use crate::indicators::pivotpoint::{Indicator, IndicatorState, PivotPoint, INPUTS, OPTIONS};

/// Calculates the Pivot Point indicator for one asset with `N` different option sets.
///
/// This implementation calls the scalar [`indicator`] function `N` times — one per option set —
/// rather than using SIMD lanes.
///
/// # Arguments
/// * `inputs` - Shared inputs: `inputs[0]` = `high`, `inputs[1]` = `low`, `inputs[2]` = `close`.
/// * `options` - An array of `N` option sets; `options[i][0]` is `period` for option set `i`.
/// * `optional_outputs` - Unused; Pivot Point has no optional outputs.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is `[s3, s2, s1, pp, r1, r2, r3]`
/// for option set `i` and `states[i]` is the final [`IndicatorState`] for option set `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub(crate) fn indicator_by_options<const N: usize>(
    inputs: &[&[f64]; INPUTS],
    options: &[&[f64; OPTIONS]; N],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    let mut all_outputs = Vec::with_capacity(N);
    let mut all_states = Vec::with_capacity(N);

    // Just call the scalar indicator N times, no simd
    for option in options.iter() {
        let (outputs, state) = PivotPoint::indicator(inputs, option, optional_outputs)?;
        all_outputs.push(outputs);
        all_states.push(state);
    }

    Ok((all_outputs, all_states))
}
