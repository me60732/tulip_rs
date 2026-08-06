use crate::common::validate_inputs;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::indicators::{
    atr::{multiplier as atr_multiplier, State as AtrState},
    ema::{multiplier as ema_multiplier, State as EmaState},
    tr::Tr,
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
pub use crate::indicators::simd_indicators::keltnerchannel_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::keltnerchannel_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::keltnerchannel_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::keltnerchannel_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
#[serde(bound="")]
pub struct State<S = Cold> {
    pub atr_state: AtrState<S>,
    pub ema_state: EmaState<S>,
    pub step: f64,
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64, f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, (high, low, close): Self::Inputs<'a>) -> (f64, f64, f64, f64, f64) {
        let (atr, tr) = self.atr_state.calc((high, low, close));
        let ema = self.ema_state.calc(close);

        let per = atr * self.step;
        let upper = ema + per;
        let lower = ema - per;

        (lower, ema, upper, atr, tr)
    }
}
impl State<Cold> {
    /// Initialises the Keltner Channel state from the first `period` bars.
    ///
    /// Seeds the ATR with the simple-average true range over `[0, period)` and
    /// seeds the EMA with the exponentially-smoothed close over the same window.
    /// If `tr_line` is non-empty the raw true-range values for bars `[1, period)` are
    /// written into it (index 0 = bar 1).
    ///
    /// # Arguments
    ///
    /// * `high` - High prices; must contain at least `period` elements.
    /// * `low` - Low prices; must contain at least `period` elements.
    /// * `close` - Close prices; must contain at least `period` elements.
    /// * `period` - Lookback period for ATR and EMA initialisation.
    /// * `multipliers` - Smoothing constants `((atr_alpha, atr_1m_alpha), (ema_alpha, ema_1m_alpha))`.
    /// * `tr_line` - Optional output buffer for raw true-range values written during warm-up.
    pub fn init_state(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        step: f64,
        tr_line: &mut [f64],
    ) -> State<Warm> {
        let atr_state = AtrState::init_state(high, low, close, period, tr_line, false);
        let ema_state = EmaState::init_state(close, period);

        State {
            atr_state,
            ema_state,
            step,
        }
    }
}
pub type IndicatorState = State<Warm>;

impl TIndicatorState<3> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let [high, low, close] = inputs;
        let (mut middle_band, mut upper_band, mut lower_band, (mut atr_line, mut tr_line)) = {
            let len = high.len();
            (
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    atr_line: len,
                    tr_line: len
                ),
            )
        };

        cycle(
            (high, low, close),
            (&mut lower_band, &mut middle_band, &mut upper_band),
            self,
            (&mut atr_line, &mut tr_line),
        );

        Ok(vec![lower_band, middle_band, upper_band, atr_line, tr_line])
    }
}

/// Validates Keltner Channel options.
///
/// # Errors
///
/// Returns `Err(IndicatorError::InvalidOptions)` if `period < 1` or `step ≤ 0`.
pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= 0.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

/// Performs the main calculation loop for the Keltner Channel indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of `(high, low, close)` price slices (starting at the first output bar).
/// * `step` - The ATR multiplier controlling channel width.
/// * `multipliers` - Smoothing constants `((atr_alpha, atr_1m_alpha), (ema_alpha, ema_1m_alpha))`.
/// * `outputs` - A tuple of mutable slices for storing the `(lower, middle, upper)` channel bands.
/// * `state` - A mutable reference to the current indicator state.
/// * `optional_outputs` - A tuple of mutable slices for optional `(atr, tr)` outputs.
fn cycle(
    (high, low, close): (&[f64], &[f64], &[f64]),
    (lower_band, middle_band, upper_band): (&mut [f64], &mut [f64], &mut [f64]),
    state: &mut State<Warm>,
    (atr_line, tr_line): (&mut [f64], &mut [f64]),
) {
    let (has_optional, want_atr, want_tr) = crate::calc_want_flags!(atr_line, tr_line);
    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
            )
        };
        let (lower, middle, upper, atr, tr) = state.calc(inputs);

        unsafe {
            *middle_band.get_unchecked_mut(i) = middle;
            *upper_band.get_unchecked_mut(i) = upper;
            *lower_band.get_unchecked_mut(i) = lower;
        }
        if has_optional {
            crate::store_optional_outputs!(i,
                want_atr, atr_line => atr,
                want_tr, tr_line => tr
            );
        }
    }
}

/// Returns the precomputed smoothing constants for the Keltner Channel.
///
/// # Returns
///
/// A tuple `((atr_alpha, atr_1m_alpha), (ema_alpha, ema_1m_alpha))` where the first pair
/// contains Wilder ATR smoothing constants and the second pair contains EMA smoothing constants.
pub fn multiplier(period: usize) -> ((f64, f64), (f64, f64)) {
    (atr_multiplier(period), ema_multiplier(period))
}

pub struct KeltnerChannel;

impl Indicator<INPUTS, OPTIONS> for KeltnerChannel {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "keltnerchannel",
        full_name: "Keltner Channel",
        indicator_type: IndicatorType::Volatility,
        inputs: &["high", "low", "close"],
        options: &["period", "step"],
        outputs: &["lower", "middle", "upper"],
        optional_outputs: &["atr", "tr"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "keltnerchannel",
                label: "Keltner Channel",
                display_type: DisplayType::Overlay,
                outputs: &["lower", "middle", "upper"],
            },
            DisplayGroup {
                offset: None,
                id: "atr_tr",
                label: "True Range",
                display_type: DisplayType::Indicator,
                outputs: &["atr", "tr"],
            },
        ],
    };
    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize + 1
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
        let step = options[1];
        let [high, low, close] = inputs;

        validate_inputs(inputs, Self::min_data(options))?;

        let (mut middle_band, mut upper_band, mut lower_band, (mut atr_line, mut tr_line)) = {
            let len = high.len();
            let capacity = Self::output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    atr_line: capacity,
                    tr_line: Tr::output_length(len, &[])
                ),
            )
        };

        let mut state = State::init_state(high, low, close, period, step, &mut tr_line);
        let (inputs, tr) = {
            let tr_offset = crate::slice_outputs_start!(middle_band.len(), tr_line);
            (
                (&high[period..], &low[period..], &close[period..]),
                &mut tr_line[tr_offset..],
            )
        };
        cycle(
            inputs,
            (&mut lower_band, &mut middle_band, &mut upper_band),
            &mut state,
            (&mut atr_line, tr),
        );

        Ok((
            vec![lower_band, middle_band, upper_band, atr_line, tr_line],
            state,
        ))
    }
}
