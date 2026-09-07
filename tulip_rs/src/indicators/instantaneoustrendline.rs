//! # Ehlers Instantaneous Trendline
//!
//! **Source:** John Ehlers, *Rocket Science for Traders* (2001), Chapter 8.
//!
//! A fully adaptive 2-pole IIR low-pass filter whose corner frequency tracks
//! the dominant market cycle period bar-by-bar. By measuring and removing the
//! dominant cycle via the embedded Homodyne Discriminator, it reveals a clean,
//! lag-reduced trendline without a user-supplied period parameter.
//!
//! ## Formula
//!
//! ```text
//! DC   = SmoothPeriod from embedded Homodyne Discriminator
//! α    = 2 / (DC + 1)
//!
//! Seeding (IT's bar_count < 6 — absorbed in init_state warmup):
//!   IT = (Price + 2·Price[1] + Price[2]) / 4
//!
//! Steady state (bar_count ≥ 6):
//!   c₀ = α − α²/4,   c₁ = α²/2,   c₂ = −(α − 3α²/4)
//!   d₁ = 2·(1 − α),  d₂ = −(1 − α)²
//!   IT = c₀·Price + c₁·Price[1] + c₂·Price[2]
//!      + d₁·IT[1] + d₂·IT[2]
//!
//! Trigger = 2·IT − IT[1]   (optional extrapolation, leads by ~1 bar)
//! ```
//!
//! **Unit gain at DC (z = 1):** numerator = α², denominator = α² → gain = 1 ✓
//!
//! ## TA-Lib `HT_TRENDLINE`
//!
//! TA-Lib's `HT_TRENDLINE` does **not** implement Ehlers' 2-pole IIR. Instead it
//! computes a variable-length SMA of length `DCPeriodInt` over raw price, followed
//! by a 4-bar WMA of that SMA, and applies an extra 63-bar lookback for
//! TradeStation compatibility. This implementation follows Ehlers' EasyLanguage
//! original. The TA-Lib benchmark is a **throughput-only** comparison between two
//! fundamentally different algorithms that share the same name.

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
/// Zero — the IT is fully adaptive via the embedded Homodyne Discriminator.
pub const OPTIONS: usize = 0;

/// Per-bar state for the Ehlers Instantaneous Trendline.
///
/// Composes the full [`homodynediscriminator::State`] pipeline (4-bar Hann smooth →
/// Detrender → I1/Q1 → jI/jQ → homodyne discriminator) and extends it with the
/// 2-pole IIR trendline stage:
///
/// ```text
/// DC  = SmoothPeriod from embedded HD
/// α   = 2 / (DC + 1)
/// IT  = c₀·P + c₁·P[1] + c₂·P[2] + d₁·IT[1] + d₂·IT[2]
/// ```
///
/// **Warmup:** `init_state` runs the HD for 22 bars, seeds the IIR from the seeding
/// formula for bars 20 and 21, then processes bar 22 (the first valid output bar)
/// using the full formula. After `init_state`, the seeding branch is permanently
/// bypassed — the hot path is unconditionally the IIR.
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    /// Embedded Homodyne Discriminator pipeline — provides `SmoothPeriod` (DC) per bar.
    /// Its `price_buf[0..2]` holds the 3 most-recent raw prices used by the IIR.
    pub hd: homodynediscriminator::State<S>,

    /// IT[1] — previous trendline value (IIR feedback state `d₁`).
    pub it_prev: f64,

    /// IT[2] — two-bar-ago trendline value (IIR feedback state `d₂`).
    pub it_prev2: f64,

    /// Last computed adaptive α = 2/(DC+1), stored for optional output.
    pub alpha: f64,
}

impl State<Cold> {
    /// Creates a new, zeroed state ready for the first bar.
    pub fn new() -> State<Cold> {
        Self {
            hd: homodynediscriminator::State::new(),
            it_prev: 0.0,
            it_prev2: 0.0,
            alpha: 0.0,
        }
    }

    /// Builds a warmed-up state by running the HD for 22 bars, seeding the IIR
    /// from the 3-bar weighted average on bars 20 and 21, then processing bar 22
    /// (the first output bar) with the full 2-pole formula.
    ///
    /// Writes the first output values to the respective output slices at index 0.
    /// Pass empty slices (`&mut []`) for any optional output that is not needed.
    pub fn init_state(
        real: &[f64],
        trendline_line: &mut [f64],
        trigger_line: &mut [f64],
        dc_period_line: &mut [f64],
        alpha_line: &mut [f64],
    ) -> State<Warm> {
        let hd = homodynediscriminator::State::init_state(real);

        // Seed IIR from the HD's price buffer — same values as the manual loop left behind:
        // price_buf[0]=real[21], [1]=real[20], [2]=real[19], [3]=real[18]
        let it_prev2 = (hd.price_buf[1] + 2.0 * hd.price_buf[2] + hd.price_buf[3]) / 4.0;
        let it_prev = (hd.price_buf[0] + 2.0 * hd.price_buf[1] + hd.price_buf[2]) / 4.0;

        let mut state = State::<Warm> {
            hd,
            it_prev,
            it_prev2,
            alpha: 0.0,
        };

        // Bar 22 — first valid output
        let it = state.calc(real[22]);
        trendline_line[0] = it;

        let (_, want_trigger, want_dc, want_alpha) =
            crate::calc_want_flags!(trigger_line, dc_period_line, alpha_line);
        crate::store_optional_outputs!(0,
            want_trigger, trigger_line   => 2.0 * state.it_prev - state.it_prev2,
            want_dc,      dc_period_line => state.hd.smooth_period,
            want_alpha,   alpha_line     => state.alpha
        );

        state
    }
}
impl TState for State<Cold> {
    type Inputs<'a> = f64;
    type Outputs = f64;

    #[inline(always)]
    fn calc<'a>(&mut self, price: Self::Inputs<'a>) -> Self::Outputs {
        self.hd.calc(price);
        if !self.hd.all_buffers_full() {
            return 0.0;
        }
        let dc = self.hd.smooth_period;
        let alpha = 2.0 / (dc + 1.0);
        self.alpha = alpha;
        let a2 = alpha * alpha;
        let beta = 1.0 - alpha;
        let it = (2.0 * beta).mul_add(
            self.it_prev,
            (-(beta * beta)).mul_add(
                self.it_prev2,
                (alpha - a2 * 0.25).mul_add(
                    self.hd.price_buf[0],
                    (a2 * 0.5).mul_add(
                        self.hd.price_buf[1],
                        -(alpha - a2 * 0.75) * self.hd.price_buf[2],
                    ),
                ),
            ),
        );
        self.it_prev2 = self.it_prev;
        self.it_prev = it;
        it
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = f64;
    #[inline(always)]
    fn calc<'a>(&mut self, price: Self::Inputs<'a>) -> Self::Outputs {
        let dc = self.hd.calc(price);

        let alpha = 2.0 / (dc + 1.0);
        self.alpha = alpha;
        let a2 = alpha * alpha;
        let beta = 1.0 - alpha;

        // 2-pole IIR from Ehlers §1.3
        // 4 FMAs: accumulate from innermost term outward.
        // Each mul_add(b, c) = self*b + c — one hardware FMA instruction.
        let it = (2.0 * beta).mul_add(
            // d₁·IT[1] + (
            self.it_prev,
            (-(beta * beta)).mul_add(
                //   d₂·IT[2] + (
                self.it_prev2,
                (alpha - a2 * 0.25).mul_add(
                    //     c₀·Price + (
                    self.hd.price_buf[0],
                    (a2 * 0.5).mul_add(
                        //       c₁·Price[1] +
                        self.hd.price_buf[1],
                        -(alpha - a2 * 0.75) * self.hd.price_buf[2], // c₂·Price[2]
                    ),
                ),
            ),
        );

        self.it_prev2 = self.it_prev;
        self.it_prev = it;
        it
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
pub type IndicatorState = State<Warm>;

impl TIndicatorState<INPUTS> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let len = inputs[0].len();

        let mut trendline_line = crate::uninit_vec!(f64, len);
        let (mut trigger_line, mut dc_period_line, mut alpha_line) = crate::init_optional_outputs!(
            optional_outputs, &[false, false, false],
            trigger_line: len,
            dc_period_line: len,
            alpha_line: len
        );

        cycle(
            inputs[0],
            self,
            &mut trendline_line,
            &mut trigger_line,
            &mut dc_period_line,
            &mut alpha_line,
        );

        Ok(vec![
            trendline_line,
            trigger_line,
            dc_period_line,
            alpha_line,
        ])
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Indicator trait
// ─────────────────────────────────────────────────────────────────────────────

/// Unit struct that implements [`Indicator`] for the Ehlers Instantaneous Trendline.
pub struct InstantaneousTrendline;

impl Indicator<INPUTS, OPTIONS> for InstantaneousTrendline {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "instantaneoustrendline",
        indicator_type: IndicatorType::Cycle,
        full_name: "Ehlers Instantaneous Trendline",
        inputs: &["real"],
        options: &[],
        outputs: &["trendline"],
        optional_outputs: &["trigger", "dc_period", "alpha"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "instantaneoustrendline",
                label: "Ehlers Instantaneous Trendline",
                display_type: DisplayType::Overlay,
                outputs: &["trendline", "trigger"],
            },
            DisplayGroup {
                offset: None,
                id: "instantaneoustrendline_dc_period",
                label: "IT Dominant Cycle Period",
                display_type: DisplayType::Indicator,
                outputs: &["dc_period"],
            },
            DisplayGroup {
                offset: None,
                id: "instantaneoustrendline_alpha",
                label: "IT Adaptive Alpha",
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
        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];
        let capacity = Self::output_length(real.len(), options);

        let mut trendline_line = crate::uninit_vec!(f64, capacity);
        let (mut trigger_line, mut dc_period_line, mut alpha_line) = crate::init_optional_outputs!(
            optional_outputs, &[false, false, false],
            trigger_line: capacity,
            dc_period_line: capacity,
            alpha_line: capacity
        );

        let mut state = State::init_state(
            real,
            &mut trendline_line,
            &mut trigger_line,
            &mut dc_period_line,
            &mut alpha_line,
        );

        // cycle processes bars min_data..len and writes to output[1..].
        let real_tail = &real[Self::min_data(options)..];
        let (_, want_trigger, want_dc, want_alpha) =
            crate::calc_want_flags!(trigger_line, dc_period_line, alpha_line);

        let trigger_tail = if want_trigger {
            &mut trigger_line[1..]
        } else {
            &mut trigger_line[..]
        };
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
            &mut trendline_line[1..],
            trigger_tail,
            dc_tail,
            alpha_tail,
        );

        Ok((
            vec![trendline_line, trigger_line, dc_period_line, alpha_line],
            state,
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::instantaneoustrendline_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

/// Core calculation loop for the Instantaneous Trendline.
///
/// All HD ring buffers must be full on entry (guaranteed after `init_state`).
/// Writes `trendline` for every bar, and optionally `trigger`, `dc_period`, `alpha`.
fn cycle(
    real: &[f64],
    state: &mut State<Warm>,
    trendline_line: &mut [f64],
    trigger_line: &mut [f64],
    dc_period_line: &mut [f64],
    alpha_line: &mut [f64],
) {
    let (has_optional, want_trigger, want_dc, want_alpha) =
        crate::calc_want_flags!(trigger_line, dc_period_line, alpha_line);

    for i in 0..real.len() {
        let it = state.calc(unsafe { *real.get_unchecked(i) });

        unsafe {
            *trendline_line.get_unchecked_mut(i) = it;
        }

        if has_optional {
            // After calc_unchecked: it_prev = IT, it_prev2 = IT[1].
            // trigger = 2·IT − IT[1] = 2·it_prev − it_prev2.
            crate::store_optional_outputs!(i,
                want_trigger, trigger_line    => 2.0 * state.it_prev - state.it_prev2,
                want_dc,      dc_period_line  => state.hd.smooth_period,
                want_alpha,   alpha_line      => state.alpha
            );
        }
    }
}
