use crate::common::validate_inputs;
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::{
    max::State as MaxState,
    min::{min_data as min_min_data, output_length as min_outpout_length, State as MinState},
};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS_WIDTH: usize = 2;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::ichimoku_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::ichimoku_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::ichimoku_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::ichimoku_simd::indicator_by_options as indicator;
}

/// Returns metadata for the Ichimoku Cloud (Ichimoku Kinkō Hyō) indicator.
///
/// # Returns
///
/// An `Info` struct containing metadata about the Ichimoku indicator, including
/// its inputs (`high`, `low`, `close`), configurable periods (`short_period`,
/// `long_period`), and output lines (`conversion`, `base`, `leading_span_a`,
/// `leading_span_b`, with optional `lagging_span`).
pub const INFO: Info = Info {
    name: "ichimoku",
    full_name: "Ichimoku",
    indicator_type: IndicatorType::Trend,
    inputs: &["high", "low", "close"],
    options: &["short_period", "long_period"],
    outputs: &["conversion", "base", "leading_span_a", "leading_span_b"],
    optional_outputs: &["lagging_span"],
    display_groups: &[
        DisplayGroup {
            offset: None,
            id: "Conversion_Base",
            label: "Tenkan-sel & Kijun-sen",
            display_type: DisplayType::Overlay,
            outputs: &["conversion", "base"],
        },
        DisplayGroup {
            offset: Some("+long_period"),
            id: "leading",
            label: "Senkou Span A & Senkou Span B",
            display_type: DisplayType::Overlay,
            outputs: &["leading_span_a", "leading_span_b"],
        },
        DisplayGroup {
            offset: Some("-long_period"),
            id: "close",
            label: "Chikou Span",
            display_type: DisplayType::Price,
            outputs: &["lagging_span"],
        },
    ],
};
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    periods: ((usize, usize), (usize, usize), (usize, usize)),
    high: Vec<f64>,
    low: Vec<f64>,
    state: State,
}
impl IndicatorState {
    pub fn new(
        high: &[f64],
        low: &[f64],
        periods: ((usize, usize), (usize, usize), (usize, usize)),
        state: State,
    ) -> Self {
        Self {
            high: high[high.len() - periods.2 .1..].to_vec(),
            low: low[low.len() - periods.2 .1..].to_vec(),
            periods,
            state,
        }
    }
}
impl TIndicatorState<INPUTS_WIDTH> for IndicatorState {
    #[inline(always)]
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let [high, low, close] = *inputs;
        self.high.extend_from_slice(high);
        self.low.extend_from_slice(low);

        let (mut conversion_line, mut base_line, mut span_a_line, mut span_b_line, lagging_span) = {
            let len = high.len();
            (
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                lagging_output(close, optional_outputs),
            )
        };
        match self.periods.2 .1 {
            1..40 => cycle::<1, 4, 4>(
                (&self.high, &self.low),
                self.periods,
                &mut self.state,
                (
                    &mut conversion_line,
                    &mut base_line,
                    &mut span_a_line,
                    &mut span_b_line,
                ),
            ),
            _ => cycle::<1, 4, 8>(
                (&self.high, &self.low),
                self.periods,
                &mut self.state,
                (
                    &mut conversion_line,
                    &mut base_line,
                    &mut span_a_line,
                    &mut span_b_line,
                ),
            ),
        }

        self.high.drain(..self.high.len() - self.periods.2 .1);
        self.low.drain(..self.low.len() - self.periods.2 .1);

        Ok(vec![
            conversion_line,
            base_line,
            span_a_line,
            span_b_line,
            lagging_span,
        ])
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub short_min_state: MinState,
    pub short_max_state: MaxState,
    pub medium_min_state: MinState,
    pub medium_max_state: MaxState,
    pub long_min_state: MinState,
    pub long_max_state: MaxState,
}
impl State {
    pub fn new(
        high: &[f64],
        low: &[f64],
        periods: ((usize, usize), (usize, usize), (usize, usize)),
    ) -> Self {
        Self {
            short_min_state: MinState::new(low[0], periods.0 .1),
            short_max_state: MaxState::new(high[0], periods.0 .1),
            medium_min_state: MinState::new(low[0], periods.1 .1),
            medium_max_state: MaxState::new(high[0], periods.1 .1),
            long_min_state: MinState::new(low[0], periods.2 .1),
            long_max_state: MaxState::new(high[0], periods.2 .1),
        }
    }
    pub fn init_state(
        inputs: (&[f64], &[f64]),
        periods: ((usize, usize), (usize, usize), (usize, usize)),
        out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
    ) -> Self {
        let (high, low) = inputs;
        let mut state = Self::new(high, low, periods);
        let (short_periods, long_periods, ultra_periods) = periods;
        let (conversion_line, base_line, span_a_line) = out_vecs;

        let (mut base, mut span_a) = (0.0, 0.0);
        let len = high.len();
        for i in short_periods.1..ultra_periods.1 {
            let short_min = state.short_min_state.calc(low, i, short_periods).0;
            let short_max = state.short_max_state.calc(high, i, short_periods).0;
            let conversion = 0.5 * (short_min + short_max);

            if i >= long_periods.1 {
                let medium_min = state.medium_min_state.calc(low, i, long_periods).0;
                let medium_max = state.medium_max_state.calc(high, i, long_periods).0;
                base = 0.5 * (medium_min + medium_max);
                span_a = 0.5 * (conversion + base);
            }
            crate::init_store_optional_outputs!(i, len,
                conversion_line => conversion,
                base_line => base,
                span_a_line => span_a

            );
        }
        state
    }
    #[inline(always)]
    pub fn calc(
        &mut self,
        inputs: (&[f64], &[f64]),
        periods: ((usize, usize), (usize, usize), (usize, usize)),
        i: usize,
    ) -> (f64, f64, f64, f64) {
        let (high, low) = inputs;

        let long_min = self.long_min_state.calc(low, i, periods.2).0;
        let medium_min = self.medium_min_state.calc(low, i, periods.1).0;
        let short_min = self.short_min_state.calc(low, i, periods.0).0;

        let long_max = self.long_max_state.calc(high, i, periods.2).0;
        let medium_max = self.medium_max_state.calc(high, i, periods.1).0;
        let short_max = self.short_max_state.calc(high, i, periods.0).0;

        let conversion = 0.5 * (short_min + short_max);
        let base = 0.5 * (medium_min + medium_max);
        let span_a = 0.5 * (conversion + base);
        let span_b = 0.5 * (long_min + long_max);

        (conversion, base, span_a, span_b)
    }
    #[inline(always)]
    pub unsafe fn calc_unchecked<const CS: usize, const CM: usize, const CL: usize>(
        &mut self,
        inputs: (&[f64], &[f64]),
        periods: ((usize, usize), (usize, usize), (usize, usize)),
        i: usize,
    ) -> (f64, f64, f64, f64) {
        let (high, low) = inputs;

        let long_min = self
            .long_min_state
            .calc_unchecked::<CL>(low, i, periods.2)
            .0;
        let long_max = self
            .long_max_state
            .calc_unchecked::<CL>(high, i, periods.2)
            .0;

        let medium_min = self
            .medium_min_state
            .calc_unchecked::<CM>(low, i, periods.1)
            .0;
        let medium_max = self
            .medium_max_state
            .calc_unchecked::<CM>(high, i, periods.1)
            .0;

        let short_min = self
            .short_min_state
            .calc_unchecked::<CS>(low, i, periods.0)
            .0;
        let short_max = self
            .short_max_state
            .calc_unchecked::<CS>(high, i, periods.0)
            .0;

        let conversion = 0.5 * (short_min + short_max);
        let base = 0.5 * (medium_min + medium_max);
        let span_a = 0.5 * (conversion + base);
        let span_b = 0.5 * (long_min + long_max);

        (conversion, base, span_a, span_b)
    }
}
/*pub fn output_length(data_len: usize, options: &[f64]) -> (usize, usize, usize, usize, usize) {
    let long_period = options[1] as usize;

    let conversion_capacity = don_outpout_length(data_len, &[options[0]]);
    let base_capacity = don_outpout_length(data_len, &[options[1]]);
    let leading_span_a_capacity = (data_len - base_capacity) + long_period + 1;
    let leading_span_b_capacity = data_len - min_data(options) + 1;
    (conversion_capacity, base_capacity, leading_span_a_capacity, leading_span_b_capacity, data_len)
}*/
pub fn output_length(
    data_len: usize,
    options: &[f64],
) -> (usize, usize, usize, usize, usize) {
    let ultra_long = options[1] as usize * 2;

    let conversion_capacity = min_outpout_length(data_len, &[options[0]]);
    let base_capacity = min_outpout_length(data_len, &[options[1]]);
    let span_a_capacity = min_outpout_length(data_len, &[ultra_long as f64]);
    let span_b_capacity = min_outpout_length(data_len, &[ultra_long as f64]);
    (
        conversion_capacity,
        base_capacity,
        span_a_capacity,
        span_b_capacity,
        data_len,
    )
}
#[inline]
pub fn min_data(options: &[f64]) -> usize {
    min_min_data(&[options[1] * 2.0]) + options[1] as usize
}
pub(crate) fn validate_options(options: &[f64; OPTIONS_WIDTH]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}
fn lagging_output(close: &[f64], optional_outputs: Option<&[bool]>) -> Vec<f64> {
    if let Some(oo) = optional_outputs {
        if oo.len() > 0 && oo[0] {
            close.to_vec()
        } else {
            Vec::<f64>::with_capacity(0)
        }
    } else {
        Vec::<f64>::with_capacity(0)
    }
}
/// Calculates the Ichimoku Cloud (Ichimoku Kinkō Hyō) indicator over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — high prices
/// * `inputs[1]` — low prices
/// * `inputs[2]` — close prices
///
/// # Options
///
/// * `options[0]` — short_period (Tenkan-sen / conversion line period)
/// * `options[1]` — long_period (Kijun-sen / base line period)
///
/// # Arguments
///
/// * `inputs` - Array of input price slices (see Inputs above).
/// * `options` - Array of indicator options (see Options above).
/// * `optional_outputs` - Pass `Some(&[true])` to include `lagging_span`
///   (Chikou Span — the close series shifted back by `long_period`);
///   `None` or `Some(&[false])` disables it.
///
/// # Returns
///
/// `Ok((outputs, state))` where:
/// - `outputs[0]` — `conversion` (Tenkan-sen)
/// - `outputs[1]` — `base` (Kijun-sen)
/// - `outputs[2]` — `leading_span_a` (Senkou Span A, displayed +long_period bars ahead)
/// - `outputs[3]` — `leading_span_b` (Senkou Span B, displayed +long_period bars ahead)
/// - `outputs[4]` — `lagging_span` (Chikou Span — full close vec, empty if not requested)
///
/// `state` can be passed to `IndicatorState::batch_indicator` for streaming.
/// Returns `Err(IndicatorError)` if inputs are too short or options are invalid.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_options(options)?;

    validate_inputs(inputs, min_data(options))?;

    let [high, low, close] = *inputs;

    let periods = {
        let (short_period, long_period) = (options[0] as usize, options[1] as usize);
        let ultra_long = long_period * 2;
        (
            (short_period, short_period - 1),
            (long_period, long_period - 1),
            (ultra_long, ultra_long - 1),
        )
    };
    let (mut conversion_line, mut base_line, mut span_a_line, mut span_b_line, lagging_span) = {
        let (conversion_cap, base_cap, span_a_cap, span_b_cap, _) =
            output_length(high.len(), options);
        (
            crate::uninit_vec!(f64, conversion_cap),
            crate::uninit_vec!(f64, base_cap),
            crate::uninit_vec!(f64, span_a_cap),
            crate::uninit_vec!(f64, span_b_cap),
            lagging_output(close, optional_outputs),
        )
    };
    let mut state = State::init_state(
        (high, low),
        periods,
        (&mut conversion_line, &mut base_line, &mut span_a_line),
    );
    let outputs = {
        let offset =
            crate::slice_outputs_start!(span_b_line.len(), conversion_line, base_line, span_a_line);
        (
            &mut conversion_line[offset.0..],
            &mut base_line[offset.1..],
            &mut span_a_line[offset.2..],
            span_b_line.as_mut_slice(),
        )
    };

    match periods.2 .1 {
        1..40 => cycle::<1, 4, 4>((high, low), periods, &mut state, outputs),
        _ => cycle::<1, 4, 8>((high, low), periods, &mut state, outputs),
    }

    Ok((
        vec![
            conversion_line,
            base_line,
            span_a_line,
            span_b_line,
            lagging_span,
        ],
        IndicatorState::new(high, low, periods, state),
    ))
}

fn cycle<const CS: usize, const CM: usize, const CL: usize>(
    inputs: (&[f64], &[f64]),
    periods: ((usize, usize), (usize, usize), (usize, usize)),
    state: &mut State,
    outputs: (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
) {
    let (conversion_line, base_line, span_a_line, span_b_line) = outputs;
    for (j, i) in (periods.2 .1..inputs.0.len()).enumerate() {
        unsafe {
            (
                *conversion_line.get_unchecked_mut(j),
                *base_line.get_unchecked_mut(j),
                *span_a_line.get_unchecked_mut(j),
                *span_b_line.get_unchecked_mut(j),
            ) = state.calc_unchecked::<CS, CM, CL>(inputs, periods, i)
        }
    }
}

/// Calculates the Ichimoku output values for a single bar.
///
/// # Arguments
///
/// * `state` - A mutable reference to the current `State` holding rolling min/max values.
/// * `inputs` - A tuple of `(high, low)` price slices.
/// * `periods` - A tuple of three `(period, look_back)` pairs for short, medium (long), and ultra-long windows.
/// * `i` - The current bar index into the input slices.
///
/// # Returns
///
/// A tuple of `(conversion, base, leading_span_a, leading_span_b)` for bar `i`.
pub fn calc(
    state: &mut State,
    inputs: (&[f64], &[f64]),
    periods: ((usize, usize), (usize, usize), (usize, usize)),
    i: usize,
) -> (f64, f64, f64, f64) {
    state.calc(inputs, periods, i)
}
