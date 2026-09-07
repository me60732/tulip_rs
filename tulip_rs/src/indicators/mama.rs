//! # MESA Adaptive Moving Average (MAMA / FAMA)
//!
//! **Source:** John Ehlers, *Rocket Science for Traders* (2001), Chapter 8.
//! Originally published as "MESA Adaptive Moving Averages",
//! *Technical Analysis of Stocks & Commodities*, February 2001.
//!
//! An exponential moving average whose smoothing factor α adapts bar-by-bar
//! based on how fast the instantaneous phase of the dominant market cycle is
//! changing. Slow phase change → large α (responsive); fast phase change →
//! small α (more smoothing). FAMA (Following Adaptive Moving Average) uses
//! half the α of MAMA to create a lagging counterpart for signal generation.
//!
//! ## Formula
//!
//! The indicator builds on the [`homodynediscriminator`] (HD) pipeline.
//! Stages 0–3 of the HD produce I1 and Q1, from which:
//!
//! ```text
//! Phase      = atan(Q1 / I1) × (180 / π)        degrees; 0 if I1 = 0
//! DeltaPhase = max(Phase[1] − Phase, 1.0)        1° floor prevents ÷0
//! α          = clamp(FastLimit / DeltaPhase, SlowLimit, FastLimit)
//!
//! MAMA = α · Price + (1 − α) · MAMA[1]
//! FAMA = 0.5α · MAMA + (1 − 0.5α) · FAMA[1]
//! ```
//!
//! Phase is expressed in degrees to match Ehlers' EasyLanguage `Atan` convention
//! and TA-Lib's implementation (both use degrees), ensuring that
//! `FastLimit / DeltaPhase` yields the expected alpha range for Ehlers' default
//! parameters (FastLimit = 0.5, SlowLimit = 0.05).
//!
//! ## TA-Lib `MAMA`
//!
//! TA-Lib's `MAMA` implements the same Ehlers 2001 formula. Outputs are close
//! but not bit-identical due to the different HD kernel implementation (TA-Lib
//! uses alternating even/odd buffers; we use a 7-slot ring buffer). The
//! benchmark comparison is valid as a throughput measurement.

use crate::common::validate_inputs;
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::indicators::homodynediscriminator;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
/// `[fast_limit, slow_limit]`
pub const OPTIONS: usize = 2;

/// Per-bar state for the Ehlers MESA Adaptive Moving Average (MAMA) and
/// Following Adaptive Moving Average (FAMA).
///
/// Composes the full [`homodynediscriminator::State`] pipeline (4-bar Hann smooth →
/// Detrender → I1/Q1 → jI/jQ → homodyne discriminator) and extends it with the
/// MAMA-specific stage:
///
/// ```text
/// Phase      = atan(Q1 / I1) × (180 / π)          // degrees — matches Ehlers & TA-Lib
/// DeltaPhase = max(Phase[1] − Phase, 1.0)           // 1° floor prevents ÷0
/// α          = clamp(FastLimit / DeltaPhase, SlowLimit, FastLimit)
/// MAMA       = α · Price + (1 − α) · MAMA[1]
/// FAMA       = ½α · MAMA  + (1 − ½α) · FAMA[1]
/// ```
///
/// Phase is expressed in degrees so that `fast_limit / delta_phase` produces the
/// expected alpha range with Ehlers' defaults (FastLimit = 0.5, SlowLimit = 0.05),
/// matching both the EasyLanguage original and TA-Lib's `TA_MAMA`.
///
/// **Warmup:** The first valid output is emitted at bar `min_data - 1 = 22` (0-indexed),
/// which is the same warmup as the Homodyne Discriminator. On that bar `mama = fama = price`
/// exactly (seeded from the first-output-bar price so that α·p + (1−α)·p = p). Subsequent
/// bars evolve from this seed.
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    /// Full Homodyne Discriminator pipeline (stages 0–3 and IIR discriminator).
    pub hd: homodynediscriminator::State<S>,
    pub prev_phase: f64,
    pub mama: f64,
    pub fama: f64,
    pub alpha: f64,
    pub fast_limit: f64,
    pub slow_limit: f64,
}

impl State {
    /// Creates a new, zeroed state ready for the first bar.
    pub fn new(fast_limit: f64, slow_limit: f64) -> Self {
        Self {
            hd: homodynediscriminator::State::new(),
            prev_phase: 0.0,
            mama: 0.0,
            fama: 0.0,
            alpha: 0.0,
            fast_limit,
            slow_limit,
        }
    }

    /// Builds a warmed-up state by feeding bars one at a time until all HD ring
    /// buffers are full, then processes the first valid bar (bar `min_data − 1`)
    /// with MAMA seeded from that bar's price so that `mama[0] = fama[0] = price`.
    ///
    /// Writes the first output values to `mama_line[0]` and `fama_line[0]`, and
    /// optionally to `dc_period_line[0]` / `alpha_line[0]` if those slices are
    /// non-empty. The caller should pass empty slices for unneeded optional outputs.
    pub fn init_state(
        real: &[f64],
        fast_limit: f64,
        slow_limit: f64,
        mama_line: &mut [f64],
        fama_line: &mut [f64],
        dc_period_line: &mut [f64],
        alpha_line: &mut [f64],
    ) -> State<Warm> {
        let hd = homodynediscriminator::State::init_state(real);

        // Seed MAMA and FAMA from bar 22's price so first output is price-exact.
        let seed = real[22];

        let mut state = State::<Warm> {
            hd,
            prev_phase: 0.0,
            mama: seed,
            fama: seed,
            alpha: 0.0,
            fast_limit,
            slow_limit,
        };

        // Process bar 22 — first valid output
        let (mama, fama) = state.calc(real[22]);
        mama_line[0] = mama;
        fama_line[0] = fama;

        let (_, want_dc, want_alpha) = crate::calc_want_flags!(dc_period_line, alpha_line);
        crate::store_optional_outputs!(0,
            want_dc,    dc_period_line => state.hd.smooth_period,
            want_alpha, alpha_line     => state.alpha
        );

        state
    }
}
impl<S> State<S> {
    /// Applies the MAMA-specific computation: phase delta → adaptive alpha → EMA updates.
    ///
    /// Shared by [`calc`](Self::calc) and [`calc_unchecked`](Self::calc_unchecked).
    /// Updates `prev_phase`, `alpha`, `mama`, and `fama` in place.
    ///
    /// Phase is converted to degrees (`× 180/π`) to match Ehlers' EasyLanguage `Atan`
    /// convention and TA-Lib's implementation, ensuring that `fast_limit / delta_phase`
    /// produces the expected alpha range with Ehlers' default parameters.
    #[inline(always)]
    fn apply_mama(&mut self, real: f64, i1: f64, q1: f64) {
        const RAD_TO_DEG: f64 = 180.0 / std::f64::consts::PI;

        // Instantaneous phase in degrees. Guard against I1 = 0 (undefined atan).
        let phase = if i1 != 0.0 {
            (q1 / i1).atan() * RAD_TO_DEG
        } else {
            0.0
        };

        // Phase decreases (advances) as cycles progress, so DeltaPhase = prev − current.
        // Floor at 1° to prevent division by zero or absurdly large alpha.
        let delta_phase = (self.prev_phase - phase).max(1.0);
        self.prev_phase = phase;

        // Adaptive alpha: larger when phase barely moved (slow market), capped at FastLimit.
        self.alpha = (self.fast_limit / delta_phase).clamp(self.slow_limit, self.fast_limit);

        // MAMA — standard EMA with adaptive alpha.
        self.mama = self.alpha.mul_add(real, (1.0 - self.alpha) * self.mama);

        // FAMA — slower EMA at half the alpha, tracking MAMA.
        let half_alpha = 0.5 * self.alpha;
        self.fama = half_alpha.mul_add(self.mama, (1.0 - half_alpha) * self.fama);
    }
}
impl TState for State<Cold> {
    type Inputs<'a> = f64;
    type Outputs = (f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
        let (_, i1, q1) = self.hd.calc_with_iq(real);
        if !self.hd.all_buffers_full() {
            return (0.0, 0.0);
        }
        self.apply_mama(real, i1, q1);
        (self.mama, self.fama)
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = (f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
        let (_, i1, q1) = self.hd.calc_with_iq(real);
        self.apply_mama(real, i1, q1);
        (self.mama, self.fama)
    }
}
/*impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}*/

pub type IndicatorState = State<Warm>;

impl TIndicatorState<INPUTS> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let len = inputs[0].len();

        let (mut mama_line, mut fama_line, (mut dc_period_line, mut alpha_line)) = (
            crate::uninit_vec!(f64, len),
            crate::uninit_vec!(f64, len),
            crate::init_optional_outputs!(
                optional_outputs, &[false, false],
                dc_period_line: len,
                alpha_line: len
            ),
        );

        cycle(
            inputs[0],
            self,
            &mut mama_line,
            &mut fama_line,
            (&mut dc_period_line, &mut alpha_line),
        );

        Ok(vec![mama_line, fama_line, dc_period_line, alpha_line])
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Indicator trait
// ─────────────────────────────────────────────────────────────────────────────

/// Unit struct that implements [`Indicator`] for MAMA / FAMA.
pub struct Mama;

impl Indicator<INPUTS, OPTIONS> for Mama {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "mama",
        indicator_type: IndicatorType::Trend,
        full_name: "MESA Adaptive Moving Average",
        inputs: &["real"],
        options: &["fast_limit", "slow_limit"],
        outputs: &["mama", "fama"],
        optional_outputs: &["dc_period", "alpha"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "mama",
                label: "MAMA / FAMA",
                display_type: DisplayType::Overlay,
                outputs: &["mama", "fama"],
            },
            DisplayGroup {
                offset: None,
                id: "mama_dc_period",
                label: "MAMA Dominant Cycle Period",
                display_type: DisplayType::Indicator,
                outputs: &["dc_period"],
            },
            DisplayGroup {
                offset: None,
                id: "mama_alpha",
                label: "MAMA Alpha",
                display_type: DisplayType::Indicator,
                outputs: &["alpha"],
            },
        ],
    };

    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        23
    }

    fn output_length(data_len: usize, _options: &[f64; OPTIONS]) -> usize {
        data_len.saturating_sub(22)
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let fast_limit = options[0];
        let slow_limit = options[1];

        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];
        let capacity = Self::output_length(real.len(), options);

        let (mut mama_line, mut fama_line, (mut dc_period_line, mut alpha_line)) = (
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
            crate::init_optional_outputs!(
                optional_outputs, &[false, false],
                dc_period_line: capacity,
                alpha_line: capacity
            ),
        );

        // init_state fills HD buffers, seeds MAMA/FAMA from first-output-bar price,
        // processes bar (min_data − 1) = 22, and writes to output[0].
        let mut state = State::init_state(
            real,
            fast_limit,
            slow_limit,
            &mut mama_line,
            &mut fama_line,
            &mut dc_period_line,
            &mut alpha_line,
        );

        // cycle processes bars min_data..len-1 and writes to output[1..].
        let real_tail = &real[Self::min_data(options)..];
        let (_, want_dc, want_alpha) = crate::calc_want_flags!(dc_period_line, alpha_line);
        let dc_tail = if want_dc {
            &mut dc_period_line[1..]
        } else {
            &mut dc_period_line[..]
        };
        let alpha_tail = if want_alpha {
            &mut alpha_line[1..]
        } else {
            &mut alpha_line[..]
        };

        cycle(
            real_tail,
            &mut state,
            &mut mama_line[1..],
            &mut fama_line[1..],
            (dc_tail, alpha_tail),
        );

        Ok((
            vec![mama_line, fama_line, dc_period_line, alpha_line],
            state,
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::mama_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Mama {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS],
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::mama_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

/// Validates MAMA options.
///
/// # Errors
///
/// Returns [`IndicatorError::InvalidOptions`] if:
/// - `fast_limit` is not in `(0.0, 1.0]`
/// - `slow_limit` is not in `(0.0, fast_limit)`
pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    let fast = options[0];
    let slow = options[1];
    if fast <= 0.0 || fast > 1.0 || slow <= 0.0 || slow >= fast {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

/// Core calculation loop for MAMA / FAMA.
///
/// All HD ring buffers must be full on entry (guaranteed after `init_state`).
/// Writes `mama` and `fama` to the corresponding output slices, and optionally
/// `dc_period` / `alpha` when those slices are non-empty.
fn cycle(
    real: &[f64],
    state: &mut State<Warm>,
    mama_line: &mut [f64],
    fama_line: &mut [f64],
    optional_outputs: (&mut [f64], &mut [f64]),
) {
    let (dc_period_line, alpha_line) = optional_outputs;
    let (has_optional, want_dc, want_alpha) = crate::calc_want_flags!(dc_period_line, alpha_line);

    for i in 0..real.len() {
        let (mama, fama) = state.calc(unsafe { *real.get_unchecked(i) });

        unsafe {
            *mama_line.get_unchecked_mut(i) = mama;
            *fama_line.get_unchecked_mut(i) = fama;
        }

        if has_optional {
            crate::store_optional_outputs!(i,
                want_dc,    dc_period_line => state.hd.smooth_period,
                want_alpha, alpha_line     => state.alpha
            );
        }
    }
}
