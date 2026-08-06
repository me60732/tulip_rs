use crate::common::validate_inputs;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::indicators::{
    max::State as MaxState,
    min::{Min, State as MinState},
};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 3;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

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

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    periods: ((usize, usize), (usize, usize), (usize, usize)),
    high: Vec<f64>,
    low: Vec<f64>,
    state: State<Warm>,
}
impl IndicatorState {
    pub fn new(
        high: &[f64],
        low: &[f64],
        periods: ((usize, usize), (usize, usize), (usize, usize)),
        state: State<Warm>,
    ) -> Self {
        Self {
            high: high[high.len() - periods.2 .1..].to_vec(),
            low: low[low.len() - periods.2 .1..].to_vec(),
            periods,
            state,
        }
    }
}
impl TIndicatorState<INPUTS> for IndicatorState {
    #[inline(always)]
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
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
        cycle(
            (&self.high, &self.low),
            self.periods,
            &mut self.state,
            (
                &mut conversion_line,
                &mut base_line,
                &mut span_a_line,
                &mut span_b_line,
            ),
        );
    
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
#[serde(bound="")]
pub struct State<S = Cold> {
    pub short_min_state: MinState<S>,
    pub short_max_state: MaxState<S>,
    pub medium_min_state: MinState<S>,
    pub medium_max_state: MaxState<S>,
    pub long_min_state: MinState<S>,
    pub long_max_state: MaxState<S>,
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
        (high, low): (&[f64], &[f64]),
        (short_periods, long_periods, ultra_periods): ((usize, usize), (usize, usize), (usize, usize)),
        out_vecs: (&mut [f64], &mut [f64], &mut [f64]),
    ) -> State<Warm> {
        let mut short_min_state = MinState::init_state(low, short_periods.1);
        let mut short_max_state = MaxState::init_state(high, short_periods.1);
        let mut medium_min_state = MinState::init_state(low, long_periods.1);
        let mut medium_max_state = MaxState::init_state(high, long_periods.1);
        let long_min_state = MinState::init_state(low, ultra_periods.1);
        let long_max_state = MaxState::init_state(high, ultra_periods.1);

        let (conversion_line, base_line, span_a_line) = out_vecs;

        let (mut base, mut span_a) = (0.0, 0.0);
        let len = high.len();
        for i in short_periods.1..ultra_periods.1 {
            let short_min = short_min_state.calc((low, i, short_periods)).0;
            let short_max = short_max_state.calc((high, i, short_periods)).0;
            let conversion = 0.5 * (short_min + short_max);

            if i >= long_periods.1 {
                let medium_min = medium_min_state.calc((low, i, long_periods)).0;
                let medium_max = medium_max_state.calc((high, i, long_periods)).0;
                base = 0.5 * (medium_min + medium_max);
                span_a = 0.5 * (conversion + base);
            }
            crate::init_store_optional_outputs!(i, len,
                conversion_line => conversion,
                base_line => base,
                span_a_line => span_a

            );
        }
        State {
            short_min_state,
            short_max_state,
            medium_min_state,
            medium_max_state,
            long_min_state,
            long_max_state,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (&'a[f64], &'a[f64], usize, ((usize, usize), (usize, usize), (usize, usize)));
    type Outputs = (f64, f64, f64, f64);

    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (high, low, i, periods): Self::Inputs<'a>,
    ) -> Self::Outputs {

        let long_min = self.long_min_state.calc((low, i, periods.2)).0;
        let medium_min = self.medium_min_state.calc((low, i, periods.1)).0;
        let short_min = self.short_min_state.calc((low, i, periods.0)).0;

        let long_max = self.long_max_state.calc((high, i, periods.2)).0;
        let medium_max = self.medium_max_state.calc((high, i, periods.1)).0;
        let short_max = self.short_max_state.calc((high, i, periods.0)).0;

        let conversion = 0.5 * (short_min + short_max);
        let base = 0.5 * (medium_min + medium_max);
        let span_a = 0.5 * (conversion + base);
        let span_b = 0.5 * (long_min + long_max);

        (conversion, base, span_a, span_b)
    }
    #[inline(always)]
    unsafe fn calc_unchecked(
        &mut self,
        inputs: Self::Inputs<'_>,
    ) -> Self::Outputs {
        self.calc_chuncked_unchecked::<1, 4, 4>(inputs)
    }
}
impl State<Warm> {
    #[inline(always)]
    pub unsafe fn calc_chuncked_unchecked<const CS: usize, const CM: usize, const CL: usize>(
        &mut self,
        (high, low, i, periods): (&[f64], &[f64], usize, ((usize, usize), (usize, usize), (usize, usize))),
    ) -> (f64, f64, f64, f64) {
        let long_min = self
            .long_min_state
            .calc_chuncked_unchecked::<CL>((low, i, periods.2))
            .0;
        let long_max = self
            .long_max_state
            .calc_chuncked_unchecked::<CL>((high, i, periods.2))
            .0;

        let medium_min = self
            .medium_min_state
            .calc_chuncked_unchecked::<CM>((low, i, periods.1))
            .0;
        let medium_max = self
            .medium_max_state
            .calc_chuncked_unchecked::<CM>((high, i, periods.1))
            .0;

        let short_min = self
            .short_min_state
            .calc_chuncked_unchecked::<CS>((low, i, periods.0))
            .0;
        let short_max = self
            .short_max_state
            .calc_chuncked_unchecked::<CS>((high, i, periods.0))
            .0;

        let conversion = 0.5 * (short_min + short_max);
        let base = 0.5 * (medium_min + medium_max);
        let span_a = 0.5 * (conversion + base);
        let span_b = 0.5 * (long_min + long_max);

        (conversion, base, span_a, span_b)
    }
}
pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
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

fn cycle(
    (high, low): (&[f64], &[f64]),
    periods: ((usize, usize), (usize, usize), (usize, usize)),
    state: &mut State<Warm>,
    (conversion_line, base_line, span_a_line, span_b_line): (&mut [f64], &mut [f64], &mut [f64], &mut [f64]),
) {

    for (j, i) in (periods.2 .1..high.len()).enumerate() {
        unsafe {
            (
                *conversion_line.get_unchecked_mut(j),
                *base_line.get_unchecked_mut(j),
                *span_a_line.get_unchecked_mut(j),
                *span_b_line.get_unchecked_mut(j),
            ) = state.calc_unchecked((high, low, i, periods))
        }
    }
}

pub struct Ichimoku;

impl Indicator<INPUTS, OPTIONS> for Ichimoku {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
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

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        Min::min_data(&[options[1] * 2.0]) + options[1] as usize
    }

    fn slot_lengths(data_len: usize, options: &[f64; OPTIONS]) -> Vec<usize> {
        let ultra_long = options[1] as usize * 2;

        vec![
            Min::output_length(data_len, &[options[0]]),
            Min::output_length(data_len, &[options[1]]),
            Min::output_length(data_len, &[ultra_long as f64]),
            Min::output_length(data_len, &[ultra_long as f64]),
            data_len,
        ]
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;

        validate_inputs(inputs, Self::min_data(options))?;

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
            let caps = Self::slot_lengths(high.len(), options);
            (
                crate::uninit_vec!(f64, caps[0]),
                crate::uninit_vec!(f64, caps[1]),
                crate::uninit_vec!(f64, caps[2]),
                crate::uninit_vec!(f64, caps[3]),
                lagging_output(close, optional_outputs),
            )
        };
        let mut state = State::init_state(
            (high, low),
            periods,
            (&mut conversion_line, &mut base_line, &mut span_a_line),
        );
        let outputs = {
            let offset = crate::slice_outputs_start!(
                span_b_line.len(),
                conversion_line,
                base_line,
                span_a_line
            );
            (
                &mut conversion_line[offset.0..],
                &mut base_line[offset.1..],
                &mut span_a_line[offset.2..],
                span_b_line.as_mut_slice(),
            )
        };
        cycle((high, low), periods, &mut state, outputs);
       
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
}
