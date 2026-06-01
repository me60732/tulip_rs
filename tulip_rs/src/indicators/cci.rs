use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::TIndicatorState;
pub use crate::indicators::md::multiplier;
use crate::indicators::md::{calc_md, calc_md_simd, output_length as md_output_length};
use crate::indicators::sma::calc as calc_sma;
use crate::indicators::typprice::calc as typprice_calc;
use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, RingBuffer};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::cci_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::cci_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::cci_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::cci_simd::indicator_by_options as indicator;
}

/// Returns information about the Commodity Channel Index (CCI) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the CCI indicator.
pub const INFO: Info = Info {
    name: "cci",
    indicator_type: IndicatorType::Momentum,
    full_name: "Commodity Channel Index",
    inputs: &["high", "low", "close"],
    options: &["period"],
    outputs: &["cci"],
    optional_outputs: &["sma", "md", "typprice"],
    display_groups: &[
        DisplayGroup {
            id: "cci",
            label: "CCI",
            display_type: DisplayType::Indicator,
            outputs: &["cci"],
        },
        DisplayGroup {
            id: "sma_typprice",
            label: "Typical Price",
            display_type: DisplayType::Overlay,
            outputs: &["sma", "typprice"],
        },
        DisplayGroup {
            id: "md",
            label: "Mean Deviation",
            display_type: DisplayType::Indicator,
            outputs: &["md"],
        }
    ],
};
#[derive(Serialize, Deserialize)]
pub struct State {
    pub buffer: Buffer,
    pub sum: f64,
}
impl State {
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
    ) -> State {
        let (sma_line, md_line, typprice_line) = out_vecs;
        let mut state = Self {
            buffer: Buffer::new(period),
            sum: 0.0,
        };
        let (mut sma, mut md) = (0.0, 0.0);
        let mut typprice;
        for (i, ((high_val, low_val), close_val)) in high
            .iter()
            .zip(low.iter())
            .zip(close.iter())
            .enumerate()
            .take(period * 2 - 2)
        {
            if i < period {
                typprice = typprice_calc(high_val, low_val, close_val);
                state.buffer.push(typprice);
                state.sum += typprice;
            } else {
                (_, sma, md, typprice) =
                    calc(&mut state, high_val, low_val, close_val, multiplier(period));
            }

            crate::init_store_optional_outputs!(i, high.len(),
                sma_line => sma,
                md_line => md,
                typprice_line => typprice
            );
        }
        state
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
    multiplier: f64,
    period: usize,
}
impl IndicatorState {
    pub fn new(state: State, multiplier: f64, period: usize) -> Self {
        Self {
            state,
            multiplier,
            period,
        }
    }
}
impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut cci_line, mut typprice_line, mut sma_line, mut md_line);
        {
            let capacity = inputs[0].len();
            (typprice_line, sma_line, md_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false, false],
                typprice_line: capacity,
                sma_line: capacity,
                md_line: capacity
            );
            cci_line = crate::uninit_vec!(f64, capacity);
        };

        match self.period {
            1..=50 => cycle::<1>(
                (inputs[0], inputs[1], inputs[2]),
                self.multiplier,
                &mut self.state,
                &mut cci_line,
                (&mut sma_line, &mut md_line, &mut typprice_line),
            ),
            _ => cycle::<8>(
                (inputs[0], inputs[1], inputs[2]),
                self.multiplier,
                &mut self.state,
                &mut cci_line,
                (&mut sma_line, &mut md_line, &mut typprice_line),
            ),
        }

        Ok(vec![cci_line, sma_line, md_line, typprice_line])
    }
}

/// Returns the minimum number of input bars required to produce accurate results.
///
/// For this indicator accuracy does not depend on decimal precision, so
/// this always returns the same value as [`min_data`].
///
/// # Arguments
///
/// * `options` - A slice containing the indicator options.
/// * `_decimals` - Unused. Accuracy is independent of decimal precision for this indicator.
///
/// # Returns
///
/// The minimum number of input bars required, identical to [`min_data`].
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the CCI indicator.
///
/// # Arguments
///
/// * `options` - A slice containing the options for the CCI calculation.
///
/// # Returns
///
/// The minimum amount of data required.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize * 2 - 1
}

/// Returns the number of output values given an input data length and options.
///
/// # Arguments
///
/// * `data_len` - The length of the input data.
/// * `options` - A slice containing the options for the CCI calculation.
///
/// # Returns
///
/// The number of output values (`data_len - min_data(options) + 1`).
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

/// Calculates the Commodity Channel Index (CCI) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
///
/// # Options
///
/// * `options[0]` — period (number of bars in the SMA / mean-deviation window)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true, false, false])` to enable `sma`,
///   `Some(&[false, true, false])` for `md`, `Some(&[false, false, true])` for
///   `typprice`, or any combination; `None` disables all optional outputs.
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `cci`, `outputs[1]` is `sma`,
/// `outputs[2]` is `md`, and `outputs[3]` is `typprice` (empty unless requested).
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;
    let period = options[0] as usize;
    let multiplier = multiplier(period);

    validate_inputs(inputs, min_data(options))?;
    let high = inputs[0];
    let low = inputs[1];
    let close = inputs[2];

    let (mut cci_line, mut typprice_line, mut sma_line, mut md_line);
    {
        let capacity = output_length(high.len(), options);
        let md_capacity = md_output_length(high.len(), options);
        cci_line = crate::uninit_vec!(f64, capacity);
        (sma_line, md_line, typprice_line) = crate::init_optional_outputs_eff!(
            optional_outputs, &[false, false, false],
            sma_line: md_capacity,
            md_line: md_capacity,
            typprice_line: high.len()
        );
    };

    let mut state = State::init_state(
        high,
        low,
        close,
        period,
        (&mut sma_line, &mut md_line, &mut typprice_line),
    );
    let optional_outputs = {
        let offset = crate::slice_outputs_start!(cci_line.len(), sma_line, md_line, typprice_line);
        (
            &mut sma_line[offset.0..],
            &mut md_line[offset.1..],
            &mut typprice_line[offset.2..],
        )
    };
    let inputs = {
        let from = period * 2 - 2;
        (&high[from..], &low[from..], &close[from..])
    };
    match period {
        1..=50 => cycle::<1>(
            inputs,
            multiplier,
            &mut state,
            &mut cci_line,
            optional_outputs,
        ),
        _ => cycle::<8>(
            inputs,
            multiplier,
            &mut state,
            &mut cci_line,
            optional_outputs,
        ),
    }

    Ok((
        vec![cci_line, sma_line, md_line, typprice_line],
        IndicatorState::new(state, multiplier, period),
    ))
}

/// Performs the main calculation loop for the CCI indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of `(high, low, close)` price slices.
/// * `multiplier` - The CCI multiplier derived from the period (`1.0 / period`).
/// * `buffer` - Mutable reference to the indicator state (ring buffer and running sum).
/// * `cci_line` - Mutable slice to write the CCI output values into.
/// * `out_vecs` - A tuple of `(sma_line, md_line, typprice_line)` for optional outputs.
fn cycle<const N: usize>(
    inputs: (&[f64], &[f64], &[f64]),
    multiplier: f64,
    state: &mut State,
    cci_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
) {
    let (high, low, close) = inputs;
    let (sma_line, md_line, typprice_line) = out_vecs;
    let (has_optional, want_typ, want_sma, want_md) =
        crate::calc_want_flags!(typprice_line, sma_line, md_line);

    //high.iter().zip(low.iter()).zip(close.iter()).skip(start).enumerate().for_each(|(i, ((h, l), c))| {
    for i in 0..high.len() {
        let (h, l, c) = unsafe {
            (
                high.get_unchecked(i),
                low.get_unchecked(i),
                close.get_unchecked(i),
            )
        };
        let (cci, sma, md, typprice) = unsafe { calc_unchecked::<N>(state, h, l, c, multiplier) };

        unsafe { *cci_line.get_unchecked_mut(i) = cci };
        if has_optional {
            crate::store_optional_outputs!(i,
                want_sma, sma_line => sma,
                want_md, md_line => md,
                want_typ, typprice_line => typprice
            );
        }
    }
}
/// Calculates the current Commodity Channel Index (CCI) value.
///
/// # Arguments
///
/// * `state` - Mutable reference to the CCI state (ring buffer and running sum).
/// * `high` - The current high price.
/// * `low` - The current low price.
/// * `close` - The current close price.
/// * `multiplier` - The CCI multiplier derived from the period (`1.0 / period`).
///
/// # Returns
///
/// A tuple `(cci, sma, md, typprice)` representing the CCI value, the SMA,
/// the mean deviation, and the typical price.
#[inline(always)]
pub fn calc(
    state: &mut State,
    high: &f64,
    low: &f64,
    close: &f64,
    multiplier: f64,
) -> (f64, f64, f64, f64) {
    let typprice = typprice_calc(high, low, close);
    //let (mut mean_deviation, mut sma, mut cci) = (0.0, 0.0, 0.0);

    if let Some(old) = state.buffer.push_with_info(typprice) {
        let sma = calc_sma(&mut state.sum, &typprice, &old, &multiplier);
        let md = calc_md(state.buffer.get_slice(), sma, multiplier);

        let cci = (typprice - sma) / (0.015 * md);
        if md == 0.0 {
            return (0.0, sma, md, typprice);
        }
        return (cci, sma, md, typprice);
    }

    state.sum += typprice;
    (0.0, 0.0, 0.0, typprice)
}
/// Calculates the CCI value using SIMD-accelerated mean deviation.
///
/// # Safety
///
/// The ring buffer in `state` must be full before calling this function.
///
/// # Arguments
///
/// * `state` - Mutable reference to the CCI state (ring buffer and running sum).
/// * `high` - The current high price.
/// * `low` - The current low price.
/// * `close` - The current close price.
/// * `multiplier` - The CCI multiplier derived from the period (`1.0 / period`).
///
/// # Returns
///
/// A tuple `(cci, sma, md, typprice)`.
#[inline(always)]
pub(crate) unsafe fn calc_unchecked<const N: usize>(
    state: &mut State,
    high: &f64,
    low: &f64,
    close: &f64,
    multiplier: f64,
) -> (f64, f64, f64, f64) {
    let typprice = typprice_calc(high, low, close);
    //let (mut mean_deviation, mut sma, mut cci) = (0.0, 0.0, 0.0);
    let old = state.buffer.push_with_info_unchecked(typprice);
    let sma = calc_sma(&mut state.sum, &typprice, &old, &multiplier);

    let md = if N == 1 {
        calc_md(state.buffer.get_slice(), sma, multiplier)
    } else {
        calc_md_simd::<N>(state.buffer.get_slice(), sma, multiplier)
    };
    if md == 0.0 {
        return (0.0, sma, md, typprice);
    }
    let cci = (typprice - sma) / (0.015 * md);
    (cci, sma, md, typprice)
}
