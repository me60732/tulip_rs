use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{TIndicatorState, Indicator, TState, IndicatorResult};
use crate::indicators::ef::State as EfState;
use crate::indicators::ema::multiplier as ema_multiplier;
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold
};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

/// SIMD-parallel variant that processes `N` assets with identical options simultaneously.
/// Requires the `simd_assets` Cargo feature. See [`by_assets`] for the module form.
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::kama_simd::indicator_by_assets;

/// SIMD-parallel variant that processes a single asset with `N` different option
/// sets simultaneously. Requires the `simd_options` Cargo feature. See [`by_options`].
#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::kama_simd::indicator_by_options;

/// Convenience module that re-exports [`indicator_by_assets`] as `indicator`,
/// allowing SIMD multi-asset computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_assets` Cargo feature.
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    /// See the parent module's [`super::indicator_by_assets`] for full documentation.
    pub use crate::indicators::simd_indicators::kama_simd::indicator_by_assets as indicator;
}

/// Convenience module that re-exports [`indicator_by_options`] as `indicator`,
/// allowing SIMD multi-option computation to be used as a drop-in replacement
/// for the standard single-asset [`indicator`] function.
/// Requires the `simd_options` Cargo feature.
#[cfg(feature = "simd_options")]
pub mod by_options {
    /// Processes a single asset with `N` different option sets in parallel.
    /// See the parent module's [`super::indicator_by_options`] for full documentation.
    pub use crate::indicators::simd_indicators::kama_simd::indicator_by_options as indicator;
}

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    period: usize,
    state: State<Warm>,
}
impl IndicatorState {
    pub fn new(real: &[f64], period: usize, state: State<Warm>) -> Self {
        Self {
            period,
            state,
            real: real[real.len() - period..].to_vec(),
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        self.real.extend_from_slice(inputs[0]);

        let (mut kama_line, mut ef_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs!(
                    optional_outputs, &[false],
                    ef_line: capacity
                ),
            )
        };

        cycle_kama(
            &self.real,
            &mut self.state,
            self.period,
            &mut kama_line,
            &mut ef_line,
        );
        self.real.drain(..self.real.len() - self.period);

        Ok(vec![kama_line, ef_line])
    }
}

#[derive(Serialize, Deserialize)]
#[serde(bound="")]
pub struct State<S = Cold>  {
    pub ef_state: EfState<S>,
    pub fast_ema: f64,
    pub slow_ema: f64,
    pub kama: f64,
}
impl State<Cold> {
    pub fn new(kama: f64, ef_state: EfState) -> Self {
        let (fast_ema, slow_ema) = multiplier();
        Self { kama, ef_state, fast_ema, slow_ema }
    }
    pub(crate) fn into_warm(self) -> State<Warm> {
        State {
            ef_state: self.ef_state.into_warm(),
            fast_ema: self.fast_ema,
            slow_ema: self.slow_ema,
            kama: self.kama,
        }
    }
    pub fn init_state(
        real: &[f64],
        period: usize,
        kama_line: &mut [f64],
        ef_line: &mut [f64],
    ) -> State<Warm> {

        let sum = (1..=period).map(|i| (real[i] - real[i - 1]).abs()).sum();

        let (value, last_value) = (real[period], real[0]);
        
        let ef = if sum != 0.0 {
            (value - last_value).abs() / sum
        } else {
            0.0
        };
        let mut state = State::new(real[period-1], EfState::new(sum, real[period], real[0]));
        
        let kama = state.calc_kama(value, ef);
        kama_line[0] = kama;
        let (_, want_ef) = crate::calc_want_flags!(ef_line);
        crate::store_optional_outputs!(0,
            want_ef, ef_line => ef
        );

        state.into_warm()
    }
}
impl<S> State<S> {
    #[inline(always)]
    fn calc_kama(&mut self, value: f64, ef: f64) -> f64 {

        //let smoothing_constant = (efficiency_ratio * (fast_ema - slow_ema) + slow_ema).powi(2);
        let smoothing_constant = (self.fast_ema - self.slow_ema)
            .mul_add(ef, self.slow_ema)
            .powi(2);

        // Optimized calculation using C-style EMA pattern
        let per1 = 1.0 - smoothing_constant;
        //self.kama = self.kama * per1 + value * smoothing_constant;
        self.kama = self.kama.mul_add(per1, value * smoothing_constant);
        self.kama
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = (f64, f64);
    #[inline(always)]
    fn calc<'a>(
        &mut self,
        (value, last_value): Self::Inputs<'a>
    ) -> Self::Outputs {
        let ef = self.ef_state.calc((value, last_value));
        let kama = self.calc_kama(value, ef);
        (kama, ef)
    }
}

/// Performs the main calculation loop for the KAMA indicator.
///
/// # Arguments
///
/// * `real` - A slice of input data.
/// * `state` - A mutable reference to the indicator state.
/// * `period` - The period for the KAMA calculation.
/// * `multipliers` - A tuple of `(fast_ema, slow_ema)` smoothing constants.
/// * `kama_line` - A mutable slice for storing the KAMA output values.
fn cycle_kama(
    real: &[f64],
    state: &mut State<Warm>,
    period: usize,
    kama_line: &mut [f64],
    ef_line: &mut [f64],
) {
    let (_, want_ef) = crate::calc_want_flags!(ef_line);
    for (j, i) in (period..real.len()).enumerate() {
        let values = unsafe {
            (
                *real.get_unchecked(i),
                *real.get_unchecked(j),
            )
        };
        let (kama, efficiency_ratio) = state.calc(values);

        unsafe { *kama_line.get_unchecked_mut(j) = kama };

        crate::store_optional_outputs!(j,
            want_ef, ef_line => efficiency_ratio
        );
    }
}

#[inline(always)]
pub fn multiplier() -> (f64, f64) {
    (ema_multiplier(2).0, ema_multiplier(30).0)
}

pub struct Kama;

impl Indicator<INPUTS, OPTIONS> for Kama {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "kama",
        indicator_type: IndicatorType::Trend,
        full_name: "Kaufman's Adaptive Moving Average",
        inputs: &["real"],
        options: &["period"],
        outputs: &["kama"],
        optional_outputs: &["ef"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "kama",
                label: "KAMA",
                display_type: DisplayType::Overlay,
                outputs: &["kama"],
            },
            DisplayGroup {
                offset: None,
                id: "ef",
                label: "Efficiency Ratio",
                display_type: DisplayType::Indicator,
                outputs: &["ef"],
            },
        ],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
    
        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];
    
        let (mut kama_line, mut ef_line) = {
            let capacity = Self::output_length(real.len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs!(
                    optional_outputs, &[false],
                    ef_line: capacity
                ),
            )
        };
        let mut state = State::init_state(real, period, &mut kama_line, &mut ef_line);
        let ef = {
            let (_, want_ef) = crate::calc_want_flags!(ef_line);
    
            if want_ef {
                &mut ef_line[1..]
            } else {
                &mut ef_line
            }
        };
        // Perform the main KAMA calculation
        cycle_kama(
            &real[1..],
            &mut state,
            period,
            &mut kama_line[1..],
            ef,
        );
    
        Ok((
            vec![kama_line, ef_line],
            IndicatorState::new(real, period, state),
        ))
    }
}