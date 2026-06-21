use crate::types::IndicatorError;


/// Validates the inputs against the Info struct.
///
/// # Arguments
///
/// * `inputs` - A slice of vectors containing the input data.
/// * `info` - The `Info` struct containing the expected inputs.
/// * `min_data_length` - The minimum data length required.
///
/// # Returns
///
/// `true` if the inputs are valid, `false` otherwise.
//#[inline(always)]
pub(crate) fn validate_inputs(
    inputs: &[&[f64]],
    min_data_length: usize,
) -> Result<(), IndicatorError> {
    let first_len = inputs[0].len();
    if first_len < min_data_length {
        return Err(IndicatorError::NotEnoughData);
    }

    for input in inputs.iter().skip(1) {
        if input.len() != first_len {
            return Err(IndicatorError::InvalidInputs);
        }
    }
    Ok(())
}
/// Validates the options against the Info struct.
///
/// # Arguments
///
/// * `options` - A slice of f64 containing the options.
/// * `info` - The `Info` struct containing the expected options.
///
/// # Returns
///
/// `true` if the options are valid, `false` otherwise.
pub(crate) fn validate_options(options: &[f64]) -> Result<(), IndicatorError> {
    for &option in options.iter() {
        if option < 1.0 {
            return Err(IndicatorError::InvalidOptions);
        }
    }
    Ok(())
}
