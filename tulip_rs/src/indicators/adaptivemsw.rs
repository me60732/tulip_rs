//! # Adaptive MESA Sine Wave
//!
//! **Source:** John Ehlers, *Rocket Science for Traders* (2001), Chapter 9.
//!
//! The true adaptive form of the MESA Sine Wave: the Homodyne Discriminator
//! measures the dominant cycle period bar-by-bar, and a windowed DFT of that
//! adaptive length extracts the instantaneous phase at every bar. No user-supplied
//! period is required.
//!
//! ## How it differs from the fixed-period `msw`
//!
//! | Property | `msw` (tulip / fixed) | `adaptivemsw` (Ehlers) |
//! |---|---|---|
//! | **Period source** | User-supplied constant | HD `SmoothPeriod` per bar |
//! | **DFT window** | Fixed `period` bars | `DCPeriodInt = round(SmoothPeriod)` bars |
//! | **min_data** | `period + 1` | 23 (HD warmup) |
//! | **Options** | `period` | None |
//!
//! ## Algorithm
//!
//! ```text
//! DC          = SmoothPeriod from embedded Homodyne Discriminator
//! DCPeriodInt = clamp(round(DC), 6, 50)
//!
//! Rp = Σ_{j=0}^{DCPeriodInt-1} Price[j] · cos(2π·j / DCPeriodInt)
//! Ip = Σ_{j=0}^{DCPeriodInt-1} Price[j] · sin(2π·j / DCPeriodInt)
//!   (j = 0 → oldest bar in the window)
//!
//! Phase    = atan(Ip / Rp) + π/2   (quadrant-adjusted, normalised to [0, 2π])
//! Sine     = sin(Phase)
//! LeadSine = sin(Phase + π/4)
//! ```
//!
//! The DFT computation reuses [`msw::calc_full`] — only the window length changes each bar.

use crate::common::validate_inputs;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::indicators::homodynediscriminator;
use crate::indicators::msw;
use crate::ring_buffer::fixed_single_buffer::FixedMirrorBuffer;
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold};
use serde::{Deserialize, Serialize};

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::adaptivemsw_simd::indicator_by_assets;

#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::adaptivemsw_simd::indicator_by_assets as indicator;
}

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
/// Zero — the period is fully adaptive via the embedded Homodyne Discriminator.
pub const OPTIONS: usize = 0;

/// Per-bar state for the Adaptive MESA Sine Wave.
///
/// Embeds a [`homodynediscriminator::State`] to measure the adaptive cycle period,
/// and maintains a 50-bar price history ring buffer for the DFT. The DFT window
/// length equals `DCPeriodInt = clamp(round(SmoothPeriod), 6, 50)` and changes
/// bar-by-bar as the HD tracks the market cycle.
///
/// **Warmup:** The HD needs 22 bars to fill all its ring buffers; the first valid
/// output is produced at bar `min_data − 1 = 22` (0-indexed). In the first ~28
/// output bars the price history buffer may be shorter than the full DFT window
/// (it fills from 23 bars at the first output to 50 bars after 28 more bars). The
/// first ~50 output bars should be treated as transient while both the HD IIR and
/// the DFT window converge.
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    /// Embedded Homodyne Discriminator — provides `SmoothPeriod` (DC) per bar.
    pub hd: homodynediscriminator::State<S>,

    /// Rolling price history for the DFT: `view[0]` = oldest, `view[count-1]` = newest.
    /// `get_slice_by_period(p)` returns the last `p` bars as a contiguous oldest-first
    /// slice — exactly what `msw::calc_full` expects, with zero copying.
    /// Capacity 50 = HD's guaranteed maximum `SmoothPeriod`.
    pub price_buf: FixedMirrorBuffer<f64, 50>,
}

impl State<Cold> {
    pub fn new() -> State<Cold> {
        Self {
            hd: homodynediscriminator::State::new(),
            price_buf: FixedMirrorBuffer::new(),
        }
    }

    /// Builds a warmed-up state by running the HD for 22 bars (filling all its
    /// ring buffers), then processes bar 22 as the first output.
    ///
    /// Pass empty slices (`&mut []`) for any optional output that is not needed.
    pub fn init_state(
        real: &[f64],
        sine_line: &mut [f64],
        lead_line: &mut [f64],
        dc_period_line: &mut [f64],
    ) -> State<Warm> {
        // HD warmup = min_data - 1 = 22 bars, returns State<Warm>
        let hd = homodynediscriminator::State::init_state(real);
    
        // Push those same 22 bars into price_buf
        let mut price_buf = FixedMirrorBuffer::new();
        for price in &real[..22] {
            price_buf.push(*price);
        }
    
        let mut state = State::<Warm> { hd, price_buf };
    
        // Bar 22 — first valid output
        let (sine, lead) = state.calc(real[22]);
        sine_line[0] = sine;
        lead_line[0] = lead;
    
        let (_, want_dc) = crate::calc_want_flags!(dc_period_line);
        crate::store_optional_outputs!(0, want_dc, dc_period_line => state.hd.smooth_period);
    
        state
    }

}
impl<S> State<S> {
    /// Computes the windowed DFT and phase-to-sine conversion.
    ///
    /// Window length = `clamp(round(dc), 6, price_buf.len().min(50))`.
    /// `get_slice_by_period` returns a direct pointer into `view[count-period..count]` —
    /// contiguous, oldest-first, zero copy — which is exactly what `calc_rp_ip` expects.
    #[inline(always)]
    fn apply_dft(&self, dc: f64) -> (f64, f64) {
        // Round and clamp to available history (grows from 23 to 50 over the
        // first 28 output bars, then stays at 50 for all subsequent bars).
        let period = ((dc + 0.5) as usize).clamp(6, self.price_buf.len().min(50));
        // Static twiddle tables — no runtime trig, no heap allocation.
        let (cos_tw, sin_tw) = msw::twiddles_for_period(period);
        let (rp, ip) =
            msw::dot_product_simd::<8>(self.price_buf.get_slice_by_period(period), cos_tw, sin_tw);
        msw::phase_from_rp_ip(rp, ip)
    }
}
impl TState for State<Cold> {
    type Inputs<'a> = f64;
    type Outputs = (f64, f64);
    /// One-bar update (safe). Returns `(sine, lead_sine)`.
    ///
    /// Returns `(0.0, 0.0)` while any HD ring buffer is still filling.
    #[inline(always)]
    fn calc<'a>(&mut self, price: Self::Inputs<'a>) -> Self::Outputs {
        let dc = self.hd.calc(price);
        self.price_buf.push(price);
        if !self.hd.all_buffers_full() {
            return (0.0, 0.0);
        }
        self.apply_dft(dc)
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = (f64, f64);

    #[inline(always)]
    fn calc<'a>(&mut self, price: Self::Inputs<'a>) -> Self::Outputs {
        let dc = self.hd.calc(price);
        self.price_buf.push(price);
        self.apply_dft(dc)
    }
}
impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming indicator state — `State` directly, matching the cybercycle pattern.
pub type IndicatorState = State<Warm>;

impl TIndicatorState<INPUTS> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        let len = inputs[0].len();

        let (mut sine_line, mut lead_line) =
            (crate::uninit_vec!(f64, len), crate::uninit_vec!(f64, len));
        let mut dc_period_line = crate::init_optional_outputs!(
            optional_outputs, &[false],
            dc_period_line: len
        );

        cycle(
            inputs[0],
            self,
            &mut sine_line,
            &mut lead_line,
            &mut dc_period_line,
        );

        Ok(vec![sine_line, lead_line, dc_period_line])
    }
}

/// Core calculation loop.
///
/// All HD ring buffers must be full on entry (guaranteed after `init_state`).
fn cycle(
    real: &[f64],
    state: &mut State<Warm>,
    sine_line: &mut [f64],
    lead_line: &mut [f64],
    dc_period_line: &mut [f64],
) {
    let (has_optional, want_dc) = crate::calc_want_flags!(dc_period_line);

    for i in 0..real.len() {
        let (sine, lead) = state.calc( unsafe { *real.get_unchecked(i) });
        unsafe {
            *sine_line.get_unchecked_mut(i) = sine;
            *lead_line.get_unchecked_mut(i) = lead;
        }
        if has_optional {
            crate::store_optional_outputs!(i, want_dc, dc_period_line => state.hd.smooth_period);
        }
    }
}

pub struct AdaptiveMSW;

impl Indicator<INPUTS, OPTIONS> for AdaptiveMSW {
    /// Metadata describing the Adaptive MESA Sine Wave indicator.
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "adaptivemsw",
        full_name: "Adaptive MESA Sine Wave",
        indicator_type: IndicatorType::Cycle,
        inputs: &["real"],
        options: &[],
        outputs: &["sine", "lead_sine"],
        optional_outputs: &["dc_period"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "adaptivemsw",
                label: "Adaptive MESA Sine Wave",
                display_type: DisplayType::Indicator,
                outputs: &["sine", "lead_sine"],
            },
            DisplayGroup {
                offset: None,
                id: "adaptivemsw_dc_period",
                label: "Adaptive MSW Dominant Cycle Period",
                display_type: DisplayType::Indicator,
                outputs: &["dc_period"],
            },
        ],
    };

    /// Returns the minimum number of input bars required for the Adaptive MESA Sine Wave.
    ///
    /// Fixed at 23 — the Homodyne Discriminator warmup. The DFT window may be shorter
    /// than the adaptive period for the first ~28 output bars while the price history
    /// fills; treat the first ~50 outputs as transient.
    fn min_data(_options: &[f64; OPTIONS]) -> usize {
        23
    }

    /// Returns the number of output values produced for a given input length.
    fn output_length(data_len: usize, options: &[f64; OPTIONS]) -> usize {
        data_len.saturating_sub(Self::min_data(options) - 1)
    }

    /// Calculates the Adaptive MESA Sine Wave over the full input dataset.
    ///
    /// # Inputs
    /// * `inputs[0]` — `real` price series (typically close, or `(H + L) / 2`)
    ///
    /// # Options
    /// None — `options` must be `&[]`.
    ///
    /// # Optional outputs
    /// * index 0 — `dc_period`: dominant cycle period from the embedded HD
    ///
    /// # Returns
    /// `Ok((outputs, state))` where `outputs[0]` = `sine`, `outputs[1]` = `lead_sine`,
    /// `outputs[2]` = `dc_period` (empty unless requested).
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_inputs(inputs, Self::min_data(options))?;
        let real = inputs[0];
        let capacity = Self::output_length(real.len(), options);

        let (mut sine_line, mut lead_line) = (
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
        );
        let mut dc_period_line = crate::init_optional_outputs!(
            optional_outputs, &[false],
            dc_period_line: capacity
        );

        let mut state =
            State::init_state(real, &mut sine_line, &mut lead_line, &mut dc_period_line);

        let real_tail = &real[Self::min_data(options)..];
        let (_, want_dc) = crate::calc_want_flags!(dc_period_line);
        let dc_tail = if want_dc {
            &mut dc_period_line[1..]
        } else {
            &mut dc_period_line[..]
        };

        cycle(
            real_tail,
            &mut state,
            &mut sine_line[1..],
            &mut lead_line[1..],
            dc_tail,
        );

        Ok((vec![sine_line, lead_line, dc_period_line], state))
    }
}
