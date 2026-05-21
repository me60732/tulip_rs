//! # tulip_rs_shared
//!
//! Shared constants and helper functions for candlestick bitmask encoding.
//!
//! This crate provides the single source of truth for bit positions and encoding
//! used by build.rs, proc macros, and runtime code.
//!
//! ## CandleBits layout
//!
//! CandleBits is split into two separate fields:
//!
//! ### `mandatory: u32`  (always computed at bar creation, 27 bits used)
//!
//! Tight-packed — no byte-boundary waste:
//!
//! ```text
//!   Bits  0– 5   Basic variants      (6 types, 1-hot)
//!   Bits  6–10   Doji variants       (5 types, 1-hot)
//!   Bits 11–16   Marubozu variants   (6 types, 1-hot)
//!   Bits 17–19   SpinningTop variants(3 types, 1-hot)
//!   Bit  20      OTHER
//!   Bit  21      COLOUR   (GREEN=1, RED=0)
//!   Bit  22      FILL     (HOLLOW=1, FILLED=0)
//!   Bit  23      TREND    (UP=1, DOWN=0)
//!   Bit  24      LINE_HEIGHT (LONG=1, SHORT=0)
//!   Bit  25      LOWER_WICK_LT_BODY  (lower wick < body height)
//!   Bit  26      UPPER_WICK_LT_BODY  (upper wick < body height)
//!   Bits 27–31  5 spare
//! ```
//!
//! ### `lazy_value / lazy_computed: u16`  (computed on demand, all 16 bits used)
//!
//! ```text
//!   Bit  0   BODY_HEIGHT           (LONG=1, SHORT=0)
//!   Bit  1   OPEN_ABOVE_PREV_BODY_MID  (my open > prev body midpoint)
//!   Bit  2   OPEN_IN_PREV_BODY     (my open ∈ prev body)
//!   Bit  3   CLOSE_ABOVE_PREV_BODY_MID (my close > prev body midpoint)
//!   Bit  4   CLOSE_IN_PREV_BODY    (my close ∈ prev body)
//!   Bit  5   HIGH_ABOVE_PREV_BODY_MID  (my high > prev body midpoint)
//!   Bit  6   HIGH_IN_PREV_BODY     (my high ∈ prev body)
//!   Bit  7   HIGH_IN_PREV_LINE     (my high ∈ [prev LOW, prev HIGH])
//!   Bit  8   LOW_ABOVE_PREV_BODY_MID   (my low > prev body midpoint)
//!   Bit  9   LOW_IN_PREV_BODY      (my low ∈ prev body)
//!   Bit 10   LOW_IN_PREV_LINE      (my low ∈ [prev LOW, prev HIGH])
//!   Bit 11   I_ENGULF_PREV_BODY    (prev open AND prev close both ∈ my body)
//!   Bit 12   PREV_HIGH_IN_MY_BODY  (prev bar's high ∈ my body)
//!   Bit 13   PREV_LOW_IN_MY_BODY   (prev bar's low ∈ my body)
//!   Bit 14   LOWER_WICK_LONG_2X    (lower wick ≥ 2× body height)
//!   Bit 15   UPPER_WICK_LONG_2X    (upper wick ≥ 2× body height)
//! ```

// ============================================================================
// MANDATORY BIT POSITION CONSTANTS  (shift amounts into `mandatory: u32`)
// ============================================================================

pub const BASIC_OFFSET: u32 = 0;
pub const DOJI_OFFSET: u32 = 6;
pub const MARUBOZU_OFFSET: u32 = 11;
pub const SPINNING_TOP_OFFSET: u32 = 17;
pub const OTHER_BIT: u32 = 20;
pub const COLOUR_BIT: u32 = 21;
pub const FILL_BIT: u32 = 22;
pub const TREND_BIT: u32 = 23;
pub const LINE_HEIGHT_BIT: u32 = 24;
pub const LOWER_WICK_LT_BODY_BIT: u32 = 25;
pub const UPPER_WICK_LT_BODY_BIT: u32 = 26;

// ============================================================================
// LAZY BIT POSITION CONSTANTS  (shift amounts into `lazy_value / lazy_computed: u16`)
// ============================================================================

pub const BODY_HEIGHT_BIT: u32 = 0;
pub const OPEN_ABOVE_PREV_BODY_MID_BIT: u32 = 1;
pub const OPEN_IN_PREV_BODY_BIT: u32 = 2;
pub const CLOSE_ABOVE_PREV_BODY_MID_BIT: u32 = 3;
pub const CLOSE_IN_PREV_BODY_BIT: u32 = 4;
pub const HIGH_ABOVE_PREV_BODY_MID_BIT: u32 = 5;
pub const HIGH_IN_PREV_BODY_BIT: u32 = 6;
pub const HIGH_IN_PREV_LINE_BIT: u32 = 7;
pub const LOW_ABOVE_PREV_BODY_MID_BIT: u32 = 8;
pub const LOW_IN_PREV_BODY_BIT: u32 = 9;
pub const LOW_IN_PREV_LINE_BIT: u32 = 10;
pub const I_ENGULF_PREV_BODY_BIT: u32 = 11;
pub const PREV_HIGH_IN_MY_BODY_BIT: u32 = 12;
pub const PREV_LOW_IN_MY_BODY_BIT: u32 = 13;
pub const LOWER_WICK_LONG_2X_BIT: u32 = 14;
pub const UPPER_WICK_LONG_2X_BIT: u32 = 15;

// ============================================================================
// MANDATORY BITMASK CONSTANTS  (u32)
// ============================================================================

pub const BASIC_MASK: u32 = 0x3F; // bits 0–5
pub const DOJI_MASK: u32 = 0x1F << DOJI_OFFSET; // bits 6–10
pub const MARUBOZU_MASK: u32 = 0x3F << MARUBOZU_OFFSET; // bits 11–16
pub const SPINNING_TOP_MASK: u32 = 0x07 << SPINNING_TOP_OFFSET; // bits 17–19

pub const CANDLE_TYPE_MASK: u32 =
    BASIC_MASK | DOJI_MASK | MARUBOZU_MASK | SPINNING_TOP_MASK | (1u32 << OTHER_BIT);

/// Mask covering all compulsory mandatory bits
pub const COMPULSORY_MASK: u32 = CANDLE_TYPE_MASK
    | (1u32 << COLOUR_BIT)
    | (1u32 << FILL_BIT)
    | (1u32 << TREND_BIT)
    | (1u32 << LINE_HEIGHT_BIT)
    | (1u32 << LOWER_WICK_LT_BODY_BIT)
    | (1u32 << UPPER_WICK_LT_BODY_BIT);

// ============================================================================
// LAZY BITMASK CONSTANTS  (u16)
// ============================================================================

/// Mask covering all currently-defined lazy bits
pub const LAZY_MASK: u16 = 0xFFFF; // All 16 bits used

// ============================================================================
// HELPER FUNCTIONS — Variant Encoding (produce u32 mandatory field bits)
// ============================================================================

/// Convert a variant discriminant to a 1-hot bit value
#[inline]
pub const fn variant_to_bit(discriminant: u32) -> u32 {
    1u32 << discriminant
}

#[inline]
pub const fn encode_basic_variant(discriminant: u32) -> u32 {
    variant_to_bit(discriminant) << BASIC_OFFSET
}

#[inline]
pub const fn encode_doji_variant(discriminant: u32) -> u32 {
    variant_to_bit(discriminant) << DOJI_OFFSET
}

#[inline]
pub const fn encode_marubozu_variant(discriminant: u32) -> u32 {
    variant_to_bit(discriminant) << MARUBOZU_OFFSET
}

#[inline]
pub const fn encode_spinning_top_variant(discriminant: u32) -> u32 {
    variant_to_bit(discriminant) << SPINNING_TOP_OFFSET
}

// ============================================================================
// MANDATORY BIT VALUE CONSTANTS  (u32)
// ============================================================================

// === Basic Candle Types (bits 0–5) ===
pub const SHORT_WHITE_CANDLE: u32 = encode_basic_variant(0);
pub const WHITE_CANDLE: u32 = encode_basic_variant(1);
pub const LONG_WHITE_CANDLE: u32 = encode_basic_variant(2);
pub const SHORT_BLACK_CANDLE: u32 = encode_basic_variant(3);
pub const BLACK_CANDLE: u32 = encode_basic_variant(4);
pub const LONG_BLACK_CANDLE: u32 = encode_basic_variant(5);

// === Doji Types (bits 6–10) ===
pub const DOJI: u32 = encode_doji_variant(0);
pub const LONG_LEGGED_DOJI: u32 = encode_doji_variant(1);
pub const DRAGONFLY_DOJI: u32 = encode_doji_variant(2);
pub const GRAVESTONE_DOJI: u32 = encode_doji_variant(3);
pub const FOUR_PRICE_DOJI: u32 = encode_doji_variant(4);

// === Marubozu Types (bits 11–16) ===
pub const WHITE_MARUBOZU: u32 = encode_marubozu_variant(0);
pub const OPENING_WHITE_MARUBOZU: u32 = encode_marubozu_variant(1);
pub const CLOSING_WHITE_MARUBOZU: u32 = encode_marubozu_variant(2);
pub const BLACK_MARUBOZU: u32 = encode_marubozu_variant(3);
pub const OPENING_BLACK_MARUBOZU: u32 = encode_marubozu_variant(4);
pub const CLOSING_BLACK_MARUBOZU: u32 = encode_marubozu_variant(5);

// === SpinningTop Types (bits 17–19) ===
pub const WHITE_SPINNING_TOP: u32 = encode_spinning_top_variant(0);
pub const BLACK_SPINNING_TOP: u32 = encode_spinning_top_variant(1);
pub const HIGH_WAVE: u32 = encode_spinning_top_variant(2);

// === Other (bit 20) ===
pub const OTHER: u32 = 1u32 << OTHER_BIT;

// === Colour (bit 21) ===
pub const COLOUR_GREEN: u32 = 1u32 << COLOUR_BIT;
pub const COLOUR_RED: u32 = 0;

// === Fill (bit 22) ===
pub const FILL_HOLLOW: u32 = 1u32 << FILL_BIT;
pub const FILL_FILLED: u32 = 0;

// === Trend (bit 23) ===
pub const TREND_UP: u32 = 1u32 << TREND_BIT;
pub const TREND_DOWN: u32 = 0;

// === Line Height (bit 24) ===
pub const LINE_HEIGHT_LONG: u32 = 1u32 << LINE_HEIGHT_BIT;
pub const LINE_HEIGHT_SHORT: u32 = 0;

// === Lower/Upper Wick vs Body (bits 25–26) ===
pub const LOWER_WICK_LT_BODY: u32 = 1u32 << LOWER_WICK_LT_BODY_BIT;
pub const UPPER_WICK_LT_BODY: u32 = 1u32 << UPPER_WICK_LT_BODY_BIT;

// ============================================================================
// LAZY BIT VALUE CONSTANTS  (u16)
// ============================================================================

// === Body Height (lazy bit 0) ===
pub const BODY_HEIGHT_LONG: u16 = 1u16 << BODY_HEIGHT_BIT;
pub const BODY_HEIGHT_SHORT: u16 = 0;

// === Open vs Prev Body (lazy bits 1–2) ===
pub const OPEN_ABOVE_PREV_BODY_MID: u16 = 1u16 << OPEN_ABOVE_PREV_BODY_MID_BIT;
pub const OPEN_IN_PREV_BODY: u16 = 1u16 << OPEN_IN_PREV_BODY_BIT;

// === Close vs Prev Body (lazy bits 3–4) ===
pub const CLOSE_ABOVE_PREV_BODY_MID: u16 = 1u16 << CLOSE_ABOVE_PREV_BODY_MID_BIT;
pub const CLOSE_IN_PREV_BODY: u16 = 1u16 << CLOSE_IN_PREV_BODY_BIT;

// === High vs Prev Body/Line (lazy bits 5–7) ===
pub const HIGH_ABOVE_PREV_BODY_MID: u16 = 1u16 << HIGH_ABOVE_PREV_BODY_MID_BIT;
pub const HIGH_IN_PREV_BODY: u16 = 1u16 << HIGH_IN_PREV_BODY_BIT;
pub const HIGH_IN_PREV_LINE: u16 = 1u16 << HIGH_IN_PREV_LINE_BIT;

// === Low vs Prev Body/Line (lazy bits 8–10) ===
pub const LOW_ABOVE_PREV_BODY_MID: u16 = 1u16 << LOW_ABOVE_PREV_BODY_MID_BIT;
pub const LOW_IN_PREV_BODY: u16 = 1u16 << LOW_IN_PREV_BODY_BIT;
pub const LOW_IN_PREV_LINE: u16 = 1u16 << LOW_IN_PREV_LINE_BIT;

// === Engulfment relationships (lazy bits 11–13) ===
pub const I_ENGULF_PREV_BODY: u16 = 1u16 << I_ENGULF_PREV_BODY_BIT;
pub const PREV_HIGH_IN_MY_BODY: u16 = 1u16 << PREV_HIGH_IN_MY_BODY_BIT;
pub const PREV_LOW_IN_MY_BODY: u16 = 1u16 << PREV_LOW_IN_MY_BODY_BIT;

// === Wick length vs body (lazy bits 14–15) ===
pub const LOWER_WICK_LONG_2X: u16 = 1u16 << LOWER_WICK_LONG_2X_BIT;
pub const UPPER_WICK_LONG_2X: u16 = 1u16 << UPPER_WICK_LONG_2X_BIT;

// ============================================================================
// CDL_GAP RETURN CODE CONSTANTS  (i8)
// ============================================================================
//
// Returned by `cdl_gap(prev, current)` to describe the gap relationship
// between two consecutive candles. Used by:
//   - Runtime pattern calc functions to interpret cdl_gap results.
//   - PatternMask::with_body_gap / with_wick_gap setters.
//   - The pattern_template proc macro (via tulip_rs_macros).

/// No gap — bodies overlap.
pub const NO_GAP: i8 = 0;
/// Current body is entirely above prev body; wicks may still overlap.
pub const BODY_GAP_UP: i8 = 1;
/// Current body is entirely below prev body; wicks may still overlap.
pub const BODY_GAP_DOWN: i8 = -1;
/// Entire current candle (including wicks) is above prev candle.
pub const WICK_GAP_UP: i8 = 2;
/// Entire current candle (including wicks) is below prev candle.
pub const WICK_GAP_DOWN: i8 = -2;

// ============================================================================
// ENGULF KIND CONSTANTS  (i8)
// ============================================================================
//
// Used by `PatternMask::with_engulf_prev` / `with_inside_prev` and the
// `pattern_template` proc macro to specify the engulf type.

/// Body-only engulf — current/previous body spans the target body.
pub const ENGULF_BODY: i8 = 1;
/// Full-line engulf — current/previous body spans the target body AND wicks.
pub const ENGULF_LINE: i8 = 2;
