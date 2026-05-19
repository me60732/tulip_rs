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
//! ### `mandatory: u32`  (always computed at bar creation, 25 bits used)
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
//!   Bits 25–31   7 spare
//! ```
//!
//! ### `lazy_value / lazy_computed: u16`  (computed on demand, 5 bits used)
//!
//! ```text
//!   Bit  0   BODY_HEIGHT       (LONG=1, SHORT=0)
//!   Bit  1   BODY_GAP_PRESENT
//!   Bit  2   BODY_GAP_DIRECTION (DOWN=1, UP=0)
//!   Bit  3   WICK_GAP_PRESENT
//!   Bit  4   WICK_GAP_DIRECTION (DOWN=1, UP=0)
//!   Bits 5–15  11 spare for future lazy attributes
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

// ============================================================================
// LAZY BIT POSITION CONSTANTS  (shift amounts into `lazy_value / lazy_computed: u16`)
// ============================================================================

pub const BODY_HEIGHT_BIT: u32 = 0;
pub const BODY_GAP_PRESENT_BIT: u32 = 1;
pub const BODY_GAP_DIRECTION_BIT: u32 = 2;
pub const WICK_GAP_PRESENT_BIT: u32 = 3;
pub const WICK_GAP_DIRECTION_BIT: u32 = 4;

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
    | (1u32 << LINE_HEIGHT_BIT);

// ============================================================================
// LAZY BITMASK CONSTANTS  (u16)
// ============================================================================

/// Mask covering all currently-defined lazy bits
pub const LAZY_MASK: u16 = (1u16 << BODY_HEIGHT_BIT)
    | (1u16 << BODY_GAP_PRESENT_BIT)
    | (1u16 << BODY_GAP_DIRECTION_BIT)
    | (1u16 << WICK_GAP_PRESENT_BIT)
    | (1u16 << WICK_GAP_DIRECTION_BIT);

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

// ============================================================================
// LAZY BIT VALUE CONSTANTS  (u16)
// ============================================================================

// === Body Height (lazy bit 0) ===
pub const BODY_HEIGHT_LONG: u16 = 1u16 << BODY_HEIGHT_BIT;
pub const BODY_HEIGHT_SHORT: u16 = 0;

// === Body Gap (lazy bits 1–2) ===
pub const BODY_GAP_PRESENT: u16 = 1u16 << BODY_GAP_PRESENT_BIT;
pub const BODY_GAP_UP: u16 = BODY_GAP_PRESENT;
pub const BODY_GAP_DOWN: u16 = BODY_GAP_PRESENT | (1u16 << BODY_GAP_DIRECTION_BIT);

// === Wick Gap (lazy bits 3–4) ===
pub const WICK_GAP_PRESENT: u16 = 1u16 << WICK_GAP_PRESENT_BIT;
pub const WICK_GAP_UP: u16 = WICK_GAP_PRESENT;
pub const WICK_GAP_DOWN: u16 = WICK_GAP_PRESENT | (1u16 << WICK_GAP_DIRECTION_BIT);
