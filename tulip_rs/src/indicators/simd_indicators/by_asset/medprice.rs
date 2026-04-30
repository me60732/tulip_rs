use crate::types::IndicatorError;

use crate::indicators::medprice::{indicator, IndicatorState, INPUTS_WIDTH, OPTIONS_WIDTH};

pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    let mut all_outputs = Vec::with_capacity(N);
    let mut all_states = Vec::with_capacity(N);

    // Just call the scalar indicator N times, no roadtrain
    for i in 0..N {
        let inputs_single = [inputs[i][0], inputs[i][1]];
        let (outputs, state) = indicator(&inputs_single, options, optional_outputs)?;
        all_outputs.push(outputs);
        all_states.push(state);
    }

    Ok((all_outputs, all_states))
}
