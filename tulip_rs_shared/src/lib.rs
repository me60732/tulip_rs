//! # tulip_rs_shared
//!
//! Shared constants and helper functions for candlestick bitmask encoding.
//!
//! This crate provides the single source of truth for bit positions and encoding
//! used by build.rs, proc macros, and runtime code.

// ============================================================================
// BIT POSITION CONSTANTS
// ============================================================================

pub const BASIC_OFFSET: u64 = 0;
pub const DOJI_OFFSET: u64 = 8;
pub const MARUBOZU_OFFSET: u64 = 16;
pub const SPINNING_TOP_OFFSET: u64 = 24;
pub const OTHER_BIT: u64 = 28;
pub const COLOUR_BIT: u64 = 29;
pub const FILL_BIT: u64 = 30;
pub const TREND_BIT: u64 = 31;
pub const BODY_HEIGHT_BIT: u64 = 32;
pub const LINE_HEIGHT_BIT: u64 = 33;
pub const BODY_GAP_PRESENT_BIT: u64 = 34;
pub const BODY_GAP_DIRECTION_BIT: u64 = 35;
pub const WICK_GAP_PRESENT_BIT: u64 = 36;
pub const WICK_GAP_DIRECTION_BIT: u64 = 37;

// ============================================================================
// BITMASK CONSTANTS
// ============================================================================

pub const BASIC_MASK: u64 = 0xFF << BASIC_OFFSET;
pub const DOJI_MASK: u64 = 0xFF << DOJI_OFFSET;
pub const MARUBOZU_MASK: u64 = 0xFF << MARUBOZU_OFFSET;
pub const SPINNING_TOP_MASK: u64 = 0x0F << SPINNING_TOP_OFFSET;

pub const CANDLE_TYPE_MASKS: u64 =
    BASIC_MASK | DOJI_MASK | MARUBOZU_MASK | SPINNING_TOP_MASK | (1 << OTHER_BIT);

/// Mask for all compulsory bits (always computed at bar creation)
pub const COMPULSORY_MASK: u64 = CANDLE_TYPE_MASKS
    | (1 << COLOUR_BIT)
    | (1 << FILL_BIT)
    | (1 << TREND_BIT)
    | (1 << LINE_HEIGHT_BIT);

/// Mask for lazy bits (computed on-demand)
pub const LAZY_MASK: u64 = (1 << BODY_HEIGHT_BIT)
    | (1 << BODY_GAP_PRESENT_BIT)
    | (1 << BODY_GAP_DIRECTION_BIT)
    | (1 << WICK_GAP_PRESENT_BIT)
    | (1 << WICK_GAP_DIRECTION_BIT);

// ============================================================================
// HELPER FUNCTIONS - Variant Encoding
// ============================================================================

/// Convert a variant discriminant to a bit value using (1 << discriminant)
#[inline]
pub const fn variant_to_bit(discriminant: u64) -> u64 {
    1 << discriminant
}

#[inline]
pub const fn encode_basic_variant(discriminant: u64) -> u64 {
    variant_to_bit(discriminant) << BASIC_OFFSET
}

#[inline]
pub const fn encode_doji_variant(discriminant: u64) -> u64 {
    variant_to_bit(discriminant) << DOJI_OFFSET
}

#[inline]
pub const fn encode_marubozu_variant(discriminant: u64) -> u64 {
    variant_to_bit(discriminant) << MARUBOZU_OFFSET
}

#[inline]
pub const fn encode_spinning_top_variant(discriminant: u64) -> u64 {
    variant_to_bit(discriminant) << SPINNING_TOP_OFFSET
}

// ============================================================================
// BIT CONSTANTS
// ============================================================================

pub const SHORT_WHITE_CANDLE: u64 = encode_basic_variant(0);
pub const WHITE_CANDLE: u64 = encode_basic_variant(1);
pub const LONG_WHITE_CANDLE: u64 = encode_basic_variant(2);
pub const SHORT_BLACK_CANDLE: u64 = encode_basic_variant(3);
pub const BLACK_CANDLE: u64 = encode_basic_variant(4);
pub const LONG_BLACK_CANDLE: u64 = encode_basic_variant(5);

pub const DOJI: u64 = encode_doji_variant(0);
pub const LONG_LEGGED_DOJI: u64 = encode_doji_variant(1);
pub const DRAGONFLY_DOJI: u64 = encode_doji_variant(2);
pub const GRAVESTONE_DOJI: u64 = encode_doji_variant(3);
pub const FOUR_PRICE_DOJI: u64 = encode_doji_variant(4);

pub const WHITE_MARUBOZU: u64 = encode_marubozu_variant(0);
pub const OPENING_WHITE_MARUBOZU: u64 = encode_marubozu_variant(1);
pub const CLOSING_WHITE_MARUBOZU: u64 = encode_marubozu_variant(2);
pub const BLACK_MARUBOZU: u64 = encode_marubozu_variant(3);
pub const OPENING_BLACK_MARUBOZU: u64 = encode_marubozu_variant(4);
pub const CLOSING_BLACK_MARUBOZU: u64 = encode_marubozu_variant(5);

pub const WHITE_SPINNING_TOP: u64 = encode_spinning_top_variant(0);
pub const BLACK_SPINNING_TOP: u64 = encode_spinning_top_variant(1);
pub const HIGH_WAVE: u64 = encode_spinning_top_variant(2);

pub const OTHER: u64 = 1 << OTHER_BIT;
pub const COLOUR_GREEN: u64 = 1 << COLOUR_BIT;
pub const COLOUR_RED: u64 = 0;
pub const FILL_HOLLOW: u64 = 1 << FILL_BIT;
pub const FILL_FILLED: u64 = 0;
pub const TREND_UP: u64 = 1 << TREND_BIT;
pub const TREND_DOWN: u64 = 0;
pub const BODY_HEIGHT_LONG: u64 = 1 << BODY_HEIGHT_BIT;
pub const BODY_HEIGHT_SHORT: u64 = 0;
pub const LINE_HEIGHT_LONG: u64 = 1 << LINE_HEIGHT_BIT;
pub const LINE_HEIGHT_SHORT: u64 = 0;
pub const BODY_GAP_PRESENT: u64 = 1 << BODY_GAP_PRESENT_BIT;
pub const BODY_GAP_UP: u64 = BODY_GAP_PRESENT;
pub const BODY_GAP_DOWN: u64 = BODY_GAP_PRESENT | (1 << BODY_GAP_DIRECTION_BIT);
pub const WICK_GAP_PRESENT: u64 = 1 << WICK_GAP_PRESENT_BIT;
pub const WICK_GAP_UP: u64 = WICK_GAP_PRESENT;
pub const WICK_GAP_DOWN: u64 = WICK_GAP_PRESENT | (1 << WICK_GAP_DIRECTION_BIT);
