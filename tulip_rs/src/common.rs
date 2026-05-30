use crate::types::{IndicatorError, IndicatorInfoOrInteger};

pub(crate) fn min_process(
    options: &[f64],
    recent_only: Option<(usize, usize)>,
    alpha: &[f64],
    indicator_info: IndicatorInfoOrInteger,
    min_data: fn(&[f64]) -> usize,
) -> usize {
    if let Some((acc, _)) = recent_only {
        let integers = match indicator_info {
            IndicatorInfoOrInteger::Info(info) => {
                // Automatically assign based on expected inputs.
                let inputs = info.inputs;
                let mut ints = 3;
                for &input in inputs.iter() {
                    if input == "volume" {
                        ints = 8;
                        break;
                    }
                }
                ints
            }
            IndicatorInfoOrInteger::Integer(val) => val,
        };

        if acc > 0 {
            let mut data_len = 0;
            for (i, &al) in alpha.iter().enumerate() {
                if i == 0 {
                    data_len += min_data_accuracy(options, acc, al, Some(min_data), integers);
                } else {
                    data_len += min_data_accuracy(options, acc, al, None, integers);
                }
            }
            return data_len;
        }
    }
    min_data(options)
}
/// Returns the minimum number of records required for an indicator (like EMA or others)
/// to converge within a given decimal place tolerance.
///
/// This function uses the smoothing factor `alpha`, retrieved via the module’s
/// `multiplier(period)` function (typically, for EMA, alpha = 2 / (period + 1)).
/// The error in the computed value decays roughly by a factor of (1 - alpha) per update.
/// For a desired accuracy of `decimal_places`, the tolerance is defined as half of the unit at that
/// precision (e.g. for 2 decimal places, tolerance = 0.005). The additional iterations needed,
/// beyond the minimum seeding data, can be approximated by:
///
///    /* additional = ceil( ln(tolerance) / ln(1 - alpha) ) */
///
/// The total minimum records required is then given by the initial seed (returned by `min_data(options)`)
/// plus these additional iterations.
///
/// # Arguments
///
/// * `options` - A slice of f64 where the first element is the period for the calculation.
/// * `decimal_places` - The desired number of decimal places of accuracy.
/// * 'alpha' - The smoothing factor for the calculation. comes from the modules multiplier function.
/// * `min_data` - A modules function that returns the minimum number of records required for the calculation.
///
/// # Returns
///
/// The total minimum number of records that should be processed.
pub(crate) fn min_data_accuracy(
    options: &[f64],
    decimal_places: usize,
    alpha: f64,
    min_data: Option<fn(&[f64]) -> usize>,
    integer_digits: usize,
) -> usize {
    // Use 0 as default if not provided.
    // New tolerance: note the addition rather than subtraction.
    let tolerance = 0.5 * 10_f64.powi(-(decimal_places as i32 + integer_digits as i32));
    // Since ln(1 - alpha) is negative (for 0 < alpha < 1), the division yields a positive number.
    let additional = (tolerance.ln() / (1.0 - alpha).ln()).ceil() as usize;
    if let Some(f) = min_data {
        f(options) + additional
    } else {
        additional
    }
}

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
