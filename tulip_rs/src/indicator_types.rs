use crate::types::IndicatorError;
use crate::types::Info;
use serde::{de::DeserializeOwned, Serialize};

pub trait TIndicatorState<const I: usize> {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; I],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError>;
}

pub type IndicatorResult<S> = Result<(Vec<Vec<f64>>, S), IndicatorError>;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub type SimdIndicatorResult<S> = Result<(Vec<Vec<Vec<f64>>>, S), IndicatorError>;

pub trait Indicator<const I: usize, const OP: usize> {
    type IndicatorState: TIndicatorState<I> + Serialize + DeserializeOwned;
    const INFO: Info;

    fn slot_lengths(data_len: usize, options: &[f64; OP]) -> Vec<usize> {
        vec![Self::output_length(data_len, options)]
    }

    fn min_data(options: &[f64; OP]) -> usize {
        options[OP - 1] as usize + 1
    }
    fn output_length(data_len: usize, options: &[f64; OP]) -> usize {
        data_len - Self::min_data(options) + 1
    }
    fn indicator(
        inputs: &[&[f64]; I],
        options: &[f64; OP],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState>;

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; I]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OP],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>>;
}
#[cfg(feature = "simd_options")]
pub trait IndicatorByOptions<const I: usize, const OP: usize>: Indicator<I, OP> {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; I], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OP]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>>;
}

pub trait TState {
    type Inputs<'a>: Copy;
    type Outputs;
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs;
    unsafe fn calc_unchecked<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        self.calc(inputs)
    }
}

pub trait TSimdState {
    type ScalarState;
    fn from_states(states: &mut [&mut Self::ScalarState]) -> Self;
    fn write_states(&self, states: &mut [&mut Self::ScalarState]);
}
