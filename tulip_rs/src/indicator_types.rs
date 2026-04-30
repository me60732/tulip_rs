use crate::types::IndicatorError;

pub trait TIndicatorState<const I: usize> {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; I],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError>;
}