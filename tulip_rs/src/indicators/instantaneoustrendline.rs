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
pub use crate::indicator_types::TIndicatorState;
use crate::indicators::homodynediscriminator;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::instantaneoustrendline_simd::indicator_by_assets;

#[cfg(feature = "simd_assets")]
pub mod by_assets {
    /// Processes `N` assets in parallel with shared options.
    pub use crate::indicators::simd_indicators::instantaneoustrendline_simd::indicator_by_assets as indicator;
}

/// Number of input price series required by this indicator.
pub const INPUTS_WIDTH: usize = 1;

/// Number of option parameters required by this indicator.
/// Zero — the IT is fully adaptive via the embedded Homodyne Discriminator.
pub const OPTIONS_WIDTH: usize = 0;

/// Metadata describing the Instantaneous Trendline indicator.
pub const INFO: Info = Info {
    name: "instantaneoustrendline",
    indicator_type: IndicatorType::Trend,
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
            outputs: &["trendline"],
        },
        DisplayGroup {
            offset: None,
            id: "instantaneoustrendline_trigger",
            label: "IT Trigger",
            display_type: DisplayType::Overlay,
            outputs: &["trigger"],
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
pub struct State {
    /// Embedded Homodyne Discriminator pipeline — provides `SmoothPeriod` (DC) per bar.
    /// Its `price_buf[0..2]` holds the 3 most-recent raw prices used by the IIR.
    pub hd: homodynediscriminator::State,

    /// IT[1] — previous trendline value (IIR feedback state `d₁`).
    pub it_prev: f64,

    /// IT[2] — two-bar-ago trendline value (IIR feedback state `d₂`).
    pub it_prev2: f64,

    /// Last computed adaptive α = 2/(DC+1), stored for optional output.
    pub alpha: f64,
}

impl State {
    /// Creates a new, zeroed state ready for the first bar.
    pub fn new() -> Self {
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
    ) -> Self {
        let mut state = Self::new();
        let mut i = 0;

        // Feed warmup bars through the HD pipeline.
        // The HD's `calc` returns 0.0 (and keeps smooth_period = 0) until all five
        // ring buffers are full — which takes exactly 22 bars (bars 0..21).
        while !state.hd.all_buffers_full() {
            state.hd.calc(real[i]);
            i += 1;
        }
        // After the loop: i = 22, all HD buffers full.
        // hd.price_buf = [real[21], real[20], real[19], real[18]]

        // Seed the IIR feedback using the seeding formula on bars 20 and 21.
        // seeding(bar k) = (real[k] + 2·real[k-1] + real[k-2]) / 4
        //
        // it_prev2 = seeding(20) = (real[20] + 2·real[19] + real[18]) / 4
        // it_prev  = seeding(21) = (real[21] + 2·real[20] + real[19]) / 4
        //
        // With i = 22: hd.price_buf[0]=real[21], [1]=real[20], [2]=real[19], [3]=real[18].
        state.it_prev2 =
            (state.hd.price_buf[1] + 2.0 * state.hd.price_buf[2] + state.hd.price_buf[3]) / 4.0;
        state.it_prev =
            (state.hd.price_buf[0] + 2.0 * state.hd.price_buf[1] + state.hd.price_buf[2]) / 4.0;

        // Process bar 22 (first valid bar) — HD buffers full, IIR seeded.
        let it = unsafe { state.calc_unchecked(real[i]) };
        trendline_line[0] = it;

        // After calc_unchecked: state.it_prev = IT[22], state.it_prev2 = seeding(21).
        // trigger = 2·IT[22] − IT[21] = 2·it_prev − it_prev2.
        let (_, want_trigger, want_dc, want_alpha) =
            crate::calc_want_flags!(trigger_line, dc_period_line, alpha_line);
        crate::store_optional_outputs!(0,
            want_trigger, trigger_line    => 2.0 * state.it_prev - state.it_prev2,
            want_dc,      dc_period_line  => state.hd.smooth_period,
            want_alpha,   alpha_line      => state.alpha
        );

        state
    }

    /// One-bar update (safe). Returns the trendline value.
    ///
    /// Returns `0.0` while any HD ring buffer is still filling.
    /// After [`init_state`] all buffers are guaranteed full.
    #[inline(always)]
    pub fn calc(&mut self, price: f64) -> f64 {
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

    /// Unsafe one-bar update — skips all ring-buffer fullness guards.
    ///
    /// After the call:
    /// - `state.it_prev`  = IT (current bar)
    /// - `state.it_prev2` = IT[1] (previous bar)
    /// - `state.alpha`    = α used this bar
    /// - trigger = `2·it_prev − it_prev2`
    ///
    /// # Safety
    ///
    /// All HD ring buffers must be full on entry. Guaranteed after [`init_state`].
    #[inline(always)]
    pub unsafe fn calc_unchecked(&mut self, price: f64) -> f64 {
        let dc = self.hd.calc_unchecked(price);

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

/// Streaming indicator state, wrapping [`State`] for use with [`batch_indicator`].
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    state: State,
}

impl IndicatorState {
    pub fn new(state: State) -> Self {
        Self { state }
    }
}

impl TIndicatorState<INPUTS_WIDTH> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
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
            &mut self.state,
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

/// Returns the minimum number of input bars required for the Instantaneous Trendline.
///
/// Fixed at 23 — identical to the Homodyne Discriminator warmup. The IT's 6-bar
/// seeding threshold is fully absorbed within the HD's 23-bar warmup in `init_state`.
pub fn min_data(_options: &[f64]) -> usize {
    23
}

/// Returns the minimum number of input bars required to produce results accurate
/// to `decimals` decimal places. Delegates to [`min_data`] (IIR warmup is
/// independent of decimal precision).
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}

/// Returns the number of output values produced for a given input length.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len.saturating_sub(min_data(options) - 1)
}

/// Calculates the Ehlers Instantaneous Trendline over the full input dataset.
///
/// # Inputs
///
/// * `inputs[0]` — `real` price series (typically close, or `(high + low) / 2`)
///
/// # Options
///
/// None — `options` must be `&[]` (zero width).
///
/// # Optional outputs
///
/// * index 0 — `trigger`:   `2·IT − IT[1]` — leads trendline by ~1 bar
/// * index 1 — `dc_period`: dominant cycle period from the embedded HD
/// * index 2 — `alpha`:     adaptive α per bar = `2/(DC+1)`
///
/// # Returns
///
/// `Ok((outputs, state))` where `outputs[0]` is `trendline`, and optional slices
/// `outputs[1..3]` are populated only when requested. `state` can be passed to
/// `IndicatorState::batch_indicator` for streaming updates.
///
/// Returns `Err(IndicatorError::NotEnoughData)` if fewer than 23 bars are provided.
pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    validate_inputs(inputs, min_data(options))?;
    let real = inputs[0];
    let capacity = output_length(real.len(), options);

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
    let real_tail = &real[min_data(options)..];
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
        IndicatorState::new(state),
    ))
}

/// Core calculation loop for the Instantaneous Trendline.
///
/// All HD ring buffers must be full on entry (guaranteed after `init_state`).
/// Writes `trendline` for every bar, and optionally `trigger`, `dc_period`, `alpha`.
fn cycle(
    real: &[f64],
    state: &mut State,
    trendline_line: &mut [f64],
    trigger_line: &mut [f64],
    dc_period_line: &mut [f64],
    alpha_line: &mut [f64],
) {
    let (has_optional, want_trigger, want_dc, want_alpha) =
        crate::calc_want_flags!(trigger_line, dc_period_line, alpha_line);

    for i in 0..real.len() {
        let it = unsafe { state.calc_unchecked(*real.get_unchecked(i)) };

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
