use crate::types::IndicatorError;
use crate::common::validate_options as val_options;
pub(crate) mod assets {
    use crate::types::IndicatorError;

    pub(crate) fn validate_inputs<const INPUTS_WIDTH: usize>( inputs: &[&[&[f64]; INPUTS_WIDTH]], min_data_length: usize) -> Result<(), IndicatorError> {
        for input in inputs.iter() {
            let len = input[0].len();
            if len < min_data_length {
                return Err(IndicatorError::NotEnoughData);
            }
            for &field in input.iter().skip(1) {
                if field.len() != len {
                    return Err(IndicatorError::InvalidInputs);
                }
            }
        }
        Ok(())
    }
}

pub(crate) mod options {
    use super::*;
    use crate::common::validate_inputs as vi;
    pub(crate) fn validate_inputs<const OPTIONS_WIDTH: usize>(
        inputs: &[&[f64]],
        options: &[&[f64; OPTIONS_WIDTH]],
        min_data: fn(&[f64]) -> usize
    ) -> Result<(), IndicatorError> {
        
        let mut min_len = 0;
        for &option in options.iter() {
            let len = min_data(option);
            min_len = if len > min_len { len } else { min_len };
        }
        vi(inputs, min_len)
    } 
    pub(crate) fn validate_options<const OPTIONS_WIDTH: usize>(
        options: &[&[f64; OPTIONS_WIDTH]],
        vo: Option<fn(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError>>
    ) ->  Result<(), IndicatorError> {
        for &option in options.iter() {
            if let Some(vo) = vo {
                vo(option)?;
            } else {
                val_options(option)?;
            }
        }
        Ok(())
    }
}