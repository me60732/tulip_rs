///TODO create new pattern test bitmask following simular PatternMask defination line 89 and candlebits
/// create advancedblock pattern using new bitmask pattern
/// create new proc macros to register patterns
///
/// ## Using Body Height and Line Height in Patterns
///
/// Height attributes allow filtering patterns based on candle size:
///
/// ```ignore
/// // In pattern_template macro:
/// bar(
///     colour = "GREEN",
///     fill = "HALLOW",
///     body_height = "LONG",     // Body must be LONG relative to EMA body
///     line_height = "SHORT"     // Total candle range must be SHORT relative to EMA line
/// )
/// ```
///
/// Or in manual PatternMask construction:
/// ```ignore
/// PatternMask::new()
///     .with_colour(GREEN)
///     .with_body_height(LONG)   // Requires long body
///     .with_line_height(SHORT)  // Requires short overall line
/// ```
///
/// Heights are automatically calculated in pattern_test.rs using:
/// - `body_height`: cdl_height((open, close), ema_body) - compares body range to average
/// - `line_height`: cdl_height((high, low), ema_line) - compares total range to average
///
/// ## Using Gap Detection in Patterns
///
/// Gap attributes allow filtering patterns based on gaps from the previous candle:
///
/// ```ignore
/// // In pattern_template macro:
/// bar(
///     colour = "GREEN",
///     body_gap = "GAP_UP",      // Gap up from previous close
///     wick_gap = "GAP_DOWN"     // Complete gap down with no overlap
/// )
/// ```
///
/// Or in manual PatternMask construction:
/// ```ignore
/// PatternMask::new()
///     .with_colour(GREEN)
///     .with_body_gap(false)    // false = gap up, true = gap down
///     .with_wick_gap(true)     // true = gap down
/// ```
///
/// Gap types:
/// - **Body Gap**: Current candle's body doesn't touch previous close
/// - **Wick Gap**: No overlap at all between current and previous candle (complete gap)
use crate::candle_indicators::candle_patterns::CandlePattern;
use crate::candle_indicators::candle_types::{CDLBasic, CDLDoji, CDLMarubozu, CDLSpinningTop};
use crate::candle_indicators::pattern_test::EmaState;
use crate::candle_indicators::perf_stats::PERF_COUNTERS;
use crate::candle_indicators::types::{CandleStick, CandleTypePattern, CandleTypes, ForcastType};
use serde::{Deserialize, Serialize};

/// Bitmask representation of a single bar's properties with lazy evaluation
///
/// **IMPORTANT**: The trend stored in each bar represents the trend AT that bar.
/// - When pattern matching, trend is extracted from `bars[0]` (prev_bar)
/// - This represents the trend going INTO the pattern
/// - Patterns requiring specific entry trend should use `prev_bar(trend = "UP")` or `prev_bar(trend = "DOWN")`
///
/// Bit layout (u64):
/// - Bits 0-7:   Basic candle variant bits (CDLBasic)
/// - Bits 8-15:  Doji candle variant bits (CDLDoji)
/// - Bits 16-23: Marubozu candle variant bits (CDLMarubozu)
/// - Bits 24-27: SpinningTop candle variant bits (CDLSpinningTop)
/// - Bit 28:     Other candle type
/// - Bit 29:     Colour (GREEN=1, RED=0)
/// - Bit 30:     Fill (HALLOW=1, FILL=0)
/// - Bit 31:     Trend (UP_TREND=1, DOWN_TREND=0)
/// - Bit 32:     Body Height (LONG=1, SHORT=0) - LAZY: calculated from EMA body on demand
/// - Bit 33:     Line Height (LONG=1, SHORT=0) - COMPULSORY: calculated at bar creation
/// - Bits 34-35: Body Gap (present + direction) - LAZY: calculated on demand
/// - Bits 36-37: Wick Gap (present + direction) - LAZY: calculated on demand
/// - Bits 38-63: Reserved for future pattern attributes
///
/// ## Compulsory vs Lazy Bits
///
/// **Compulsory bits** (always computed at bar creation):
/// - Candle type (bits 0-28)
/// - Colour (bit 29)
/// - Fill (bit 30)
/// - Trend (bit 31)
/// - Line height (bit 33)
///
/// **Lazy bits** (computed on-demand):
/// - Body height (bit 32)
/// - Body gap (bits 34-35)
/// - Wick gap (bits 36-37)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CandleBits {
    pub value: u64,    // The actual bit values
    pub computed: u64, // Which bits have been computed (1 = computed, 0 = not computed yet)
}

impl CandleBits {
    // Re-export bit position constants from tulip_rs_shared for single source of truth
    pub const BASIC_OFFSET: u64 = tulip_rs_shared::BASIC_OFFSET;
    pub const DOJI_OFFSET: u64 = tulip_rs_shared::DOJI_OFFSET;
    pub const MARUBOZU_OFFSET: u64 = tulip_rs_shared::MARUBOZU_OFFSET;
    pub const SPINNING_TOP_OFFSET: u64 = tulip_rs_shared::SPINNING_TOP_OFFSET;
    pub const OTHER_BIT: u64 = tulip_rs_shared::OTHER_BIT;
    pub const COLOUR_BIT: u64 = tulip_rs_shared::COLOUR_BIT;
    pub const FILL_BIT: u64 = tulip_rs_shared::FILL_BIT;
    pub const TREND_BIT: u64 = tulip_rs_shared::TREND_BIT;
    pub const BODY_HEIGHT_BIT: u64 = tulip_rs_shared::BODY_HEIGHT_BIT;
    pub const LINE_HEIGHT_BIT: u64 = tulip_rs_shared::LINE_HEIGHT_BIT;
    pub const BODY_GAP_PRESENT_BIT: u64 = tulip_rs_shared::BODY_GAP_PRESENT_BIT;
    pub const BODY_GAP_DIRECTION_BIT: u64 = tulip_rs_shared::BODY_GAP_DIRECTION_BIT;
    pub const WICK_GAP_PRESENT_BIT: u64 = tulip_rs_shared::WICK_GAP_PRESENT_BIT;
    pub const WICK_GAP_DIRECTION_BIT: u64 = tulip_rs_shared::WICK_GAP_DIRECTION_BIT;

    // Re-export masks from tulip_rs_shared
    pub const BASIC_MASK: u64 = tulip_rs_shared::BASIC_MASK;
    pub const DOJI_MASK: u64 = tulip_rs_shared::DOJI_MASK;
    pub const MARUBOZU_MASK: u64 = tulip_rs_shared::MARUBOZU_MASK;
    pub const SPINNING_TOP_MASK: u64 = tulip_rs_shared::SPINNING_TOP_MASK;

    // Re-export masks for compulsory and lazy bits
    pub const COMPULSORY_MASK: u64 = tulip_rs_shared::COMPULSORY_MASK;
    pub const LAZY_MASK: u64 = tulip_rs_shared::LAZY_MASK;

    // ========================================================================
    // PUBLIC BIT CONSTANTS FOR FAST PATTERN MATCHING
    // ========================================================================
    // These constants allow direct bit operations in pattern calc() functions
    // for maximum performance. Use these instead of get_candle_type() in hot paths.
    //
    // Example usage:
    //     if bars[i].value & CandleBits::HIGH_WAVE != 0 { ... }
    //     if bars[i].value & CandleBits::GREEN != 0 { ... }

    // === Basic Candle Types (bits 0-7) ===
    pub const SHORT_WHITE_CANDLE: u64 = tulip_rs_shared::SHORT_WHITE_CANDLE;
    pub const WHITE_CANDLE: u64 = tulip_rs_shared::WHITE_CANDLE;
    pub const LONG_WHITE_CANDLE: u64 = tulip_rs_shared::LONG_WHITE_CANDLE;
    pub const SHORT_BLACK_CANDLE: u64 = tulip_rs_shared::SHORT_BLACK_CANDLE;
    pub const BLACK_CANDLE: u64 = tulip_rs_shared::BLACK_CANDLE;
    pub const LONG_BLACK_CANDLE: u64 = tulip_rs_shared::LONG_BLACK_CANDLE;

    // === Doji Types (bits 8-15) ===
    pub const DOJI: u64 = tulip_rs_shared::DOJI;
    pub const LONG_LEGGED_DOJI: u64 = tulip_rs_shared::LONG_LEGGED_DOJI;
    pub const DRAGONFLY_DOJI: u64 = tulip_rs_shared::DRAGONFLY_DOJI;
    pub const GRAVESTONE_DOJI: u64 = tulip_rs_shared::GRAVESTONE_DOJI;
    pub const FOUR_PRICE_DOJI: u64 = tulip_rs_shared::FOUR_PRICE_DOJI;

    // === Marubozu Types (bits 16-23) ===
    pub const WHITE_MARUBOZU: u64 = tulip_rs_shared::WHITE_MARUBOZU;
    pub const OPENING_WHITE_MARUBOZU: u64 = tulip_rs_shared::OPENING_WHITE_MARUBOZU;
    pub const CLOSING_WHITE_MARUBOZU: u64 = tulip_rs_shared::CLOSING_WHITE_MARUBOZU;
    pub const BLACK_MARUBOZU: u64 = tulip_rs_shared::BLACK_MARUBOZU;
    pub const OPENING_BLACK_MARUBOZU: u64 = tulip_rs_shared::OPENING_BLACK_MARUBOZU;
    pub const CLOSING_BLACK_MARUBOZU: u64 = tulip_rs_shared::CLOSING_BLACK_MARUBOZU;

    // === SpinningTop Types (bits 24-27) ===
    pub const WHITE_SPINNING_TOP: u64 = tulip_rs_shared::WHITE_SPINNING_TOP;
    pub const BLACK_SPINNING_TOP: u64 = tulip_rs_shared::BLACK_SPINNING_TOP;
    pub const HIGH_WAVE: u64 = tulip_rs_shared::HIGH_WAVE;

    // === Other Type (bit 28) ===
    pub const OTHER: u64 = tulip_rs_shared::OTHER;

    // === Colour (bit 29) ===
    pub const COLOUR_GREEN: u64 = tulip_rs_shared::COLOUR_GREEN;
    pub const COLOUR_RED: u64 = tulip_rs_shared::COLOUR_RED;

    // === Fill (bit 30) ===
    pub const FILL_HALLOW: u64 = tulip_rs_shared::FILL_HOLLOW; // Note: shared uses HOLLOW spelling
    pub const FILL_FILLED: u64 = tulip_rs_shared::FILL_FILLED;

    // === Trend (bit 31) ===
    pub const TREND_UP: u64 = tulip_rs_shared::TREND_UP;
    pub const TREND_DOWN: u64 = tulip_rs_shared::TREND_DOWN;

    // === Body Height (bit 32) ===
    pub const BODY_HEIGHT_LONG: u64 = tulip_rs_shared::BODY_HEIGHT_LONG;
    pub const BODY_HEIGHT_SHORT: u64 = tulip_rs_shared::BODY_HEIGHT_SHORT;

    // === Line Height (bit 33) ===
    pub const LINE_HEIGHT_LONG: u64 = tulip_rs_shared::LINE_HEIGHT_LONG;
    pub const LINE_HEIGHT_SHORT: u64 = tulip_rs_shared::LINE_HEIGHT_SHORT;

    // === Body Gap (bits 34-35) ===
    pub const BODY_GAP_PRESENT: u64 = tulip_rs_shared::BODY_GAP_PRESENT;
    pub const BODY_GAP_UP: u64 = tulip_rs_shared::BODY_GAP_UP;
    pub const BODY_GAP_DOWN: u64 = tulip_rs_shared::BODY_GAP_DOWN;

    // === Wick Gap (bits 36-37) ===
    pub const WICK_GAP_PRESENT: u64 = tulip_rs_shared::WICK_GAP_PRESENT;
    pub const WICK_GAP_UP: u64 = tulip_rs_shared::WICK_GAP_UP;
    pub const WICK_GAP_DOWN: u64 = tulip_rs_shared::WICK_GAP_DOWN;

    /// Create a new CandleBits from candle type, colour, fill, trend, heights, and gaps
    ///
    /// This is the "full" constructor that computes all attributes immediately.
    /// For lazy evaluation, use `new_minimal()` instead.
    #[inline(always)]
    pub fn new(
        candle_type: &CandleTypes,
        colour: bool,
        fill: bool,
        trend: bool,
        body_height: bool,
        line_height: bool,
        body_gap: Option<bool>, // None = no gap, Some(false) = gap up, Some(true) = gap down
        wick_gap: Option<bool>, // None = no gap, Some(false) = gap up, Some(true) = gap down
    ) -> Self {
        let mut bits = 0u64;
        let mut computed_bits = 0u64;

        // Set candle type bits using tulip_rs_shared encoding functions
        // Use discriminant() to get the variant index (0, 1, 2, ...) which the encoding functions expect
        match candle_type {
            CandleTypes::Basic(variant) => {
                bits |= tulip_rs_shared::encode_basic_variant(variant.discriminant() as u64);
            }
            CandleTypes::Doji(variant) => {
                bits |= tulip_rs_shared::encode_doji_variant(variant.discriminant() as u64);
            }
            CandleTypes::Marubozu(variant) => {
                bits |= tulip_rs_shared::encode_marubozu_variant(variant.discriminant() as u64);
            }
            CandleTypes::SpinningTop(variant) => {
                bits |= tulip_rs_shared::encode_spinning_top_variant(variant.discriminant() as u64);
            }
            CandleTypes::Other => {
                bits |= 1 << Self::OTHER_BIT;
            }
        }
        computed_bits |= tulip_rs_shared::CANDLE_TYPE_MASKS;

        // Set colour bit (GREEN=1, RED=0)
        if colour {
            bits |= 1 << Self::COLOUR_BIT;
        }
        computed_bits |= 1 << Self::COLOUR_BIT;

        // Set fill bit (HALLOW=1, FILL=0)
        if fill {
            bits |= 1 << Self::FILL_BIT;
        }
        computed_bits |= 1 << Self::FILL_BIT;

        // Set trend bit (UP_TREND=1, DOWN_TREND=0)
        if trend {
            bits |= 1 << Self::TREND_BIT;
        }
        computed_bits |= 1 << Self::TREND_BIT;

        // Set body height bit (LONG=1, SHORT=0)
        if body_height {
            bits |= 1 << Self::BODY_HEIGHT_BIT;
        }
        computed_bits |= 1 << Self::BODY_HEIGHT_BIT;

        // Set line height bit (LONG=1, SHORT=0)
        if line_height {
            bits |= 1 << Self::LINE_HEIGHT_BIT;
        }
        computed_bits |= 1 << Self::LINE_HEIGHT_BIT;

        // Set body gap bits if present (2 bits: present + direction)
        if let Some(gap_down) = body_gap {
            bits |= 1 << Self::BODY_GAP_PRESENT_BIT; // Gap exists
            if gap_down {
                bits |= 1 << Self::BODY_GAP_DIRECTION_BIT; // Gap down
            }
            computed_bits |=
                (1 << Self::BODY_GAP_PRESENT_BIT) | (1 << Self::BODY_GAP_DIRECTION_BIT);
        }

        // Set wick gap bits if present (2 bits: present + direction)
        if let Some(gap_down) = wick_gap {
            bits |= 1 << Self::WICK_GAP_PRESENT_BIT; // Gap exists
            if gap_down {
                bits |= 1 << Self::WICK_GAP_DIRECTION_BIT; // Gap down
            }
            computed_bits |=
                (1 << Self::WICK_GAP_PRESENT_BIT) | (1 << Self::WICK_GAP_DIRECTION_BIT);
        }

        CandleBits {
            value: bits,
            computed: computed_bits,
        }
    }

    /// Create a new CandleBits with only compulsory attributes computed
    ///
    /// This constructor is for lazy evaluation - it sets only the bits that are
    /// always needed (candle_type, colour, fill, trend, line_height) and marks
    /// them as computed. Optional attributes (body_height, body_gap, wick_gap)
    /// can be set later using the set_* methods.
    ///
    /// Compulsory bits:
    /// - Candle type (bits 0-28)
    /// - Colour (bit 29)
    /// - Fill (bit 30)
    /// - Trend (bit 31)
    /// - Line height (bit 33)
    #[inline(always)]
    pub fn new_minimal(
        candle_type: &CandleTypes,
        colour: bool,
        fill: bool,
        trend: bool,
        line_height: bool,
    ) -> Self {
        let mut bits = 0u64;

        // Set candle type bits using tulip_rs_shared encoding functions
        // Use discriminant() to get the variant index (0, 1, 2, ...) which the encoding functions expect
        match candle_type {
            CandleTypes::Basic(variant) => {
                bits |= tulip_rs_shared::encode_basic_variant(variant.discriminant() as u64);
            }
            CandleTypes::Doji(variant) => {
                bits |= tulip_rs_shared::encode_doji_variant(variant.discriminant() as u64);
            }
            CandleTypes::Marubozu(variant) => {
                bits |= tulip_rs_shared::encode_marubozu_variant(variant.discriminant() as u64);
            }
            CandleTypes::SpinningTop(variant) => {
                bits |= tulip_rs_shared::encode_spinning_top_variant(variant.discriminant() as u64);
            }
            CandleTypes::Other => {
                bits |= 1 << Self::OTHER_BIT;
            }
        }

        // Set colour bit (GREEN=1, RED=0)
        if colour {
            bits |= 1 << Self::COLOUR_BIT;
        }

        // Set fill bit (HALLOW=1, FILL=0)
        if fill {
            bits |= 1 << Self::FILL_BIT;
        }

        // Set trend bit (UP_TREND=1, DOWN_TREND=0)
        if trend {
            bits |= 1 << Self::TREND_BIT;
        }

        // Set line height bit (LONG=1, SHORT=0)
        if line_height {
            bits |= 1 << Self::LINE_HEIGHT_BIT;
        }

        CandleBits {
            value: bits,
            computed: Self::COMPULSORY_MASK,
        }
    }

    /// Set the body height attribute (lazy evaluation)
    ///
    /// This marks the body height bit as computed. Call this method when you
    /// have calculated the body height relative to the EMA.
    ///
    /// # Arguments
    /// * `is_long` - true for LONG body, false for SHORT body
    #[inline(always)]
    pub fn set_body_height(&mut self, is_long: bool) {
        if is_long {
            self.value |= 1 << Self::BODY_HEIGHT_BIT;
        } else {
            self.value &= !(1 << Self::BODY_HEIGHT_BIT);
        }
        self.computed |= 1 << Self::BODY_HEIGHT_BIT;
    }

    /// Set the body gap attribute (lazy evaluation)
    ///
    /// This marks the body gap bits as computed. Call this method when you
    /// have calculated whether there's a gap between the current body and
    /// the previous close.
    ///
    /// # Arguments
    /// * `gap` - None for no gap, Some(false) for gap up, Some(true) for gap down
    #[inline(always)]
    pub fn set_body_gap(&mut self, gap: Option<bool>) {
        // Clear both gap bits first
        self.value &= !((1 << Self::BODY_GAP_PRESENT_BIT) | (1 << Self::BODY_GAP_DIRECTION_BIT));

        if let Some(gap_up) = gap {
            self.value |= 1 << Self::BODY_GAP_PRESENT_BIT; // Gap exists
                                                           // Note: gap_up = true means GAP_UP, gap_up = false means GAP_DOWN
                                                           // BODY_GAP_DIRECTION_BIT set means DOWN, unset means UP
            if !gap_up {
                // If gap is DOWN (false), set the direction bit
                self.value |= 1 << Self::BODY_GAP_DIRECTION_BIT; // Gap down
            }
            // If gap_up is true (GAP_UP), direction bit stays unset
        }

        // Mark both bits as computed
        self.computed |= (1 << Self::BODY_GAP_PRESENT_BIT) | (1 << Self::BODY_GAP_DIRECTION_BIT);
    }

    /// Set the wick gap attribute (lazy evaluation)
    ///
    /// This marks the wick gap bits as computed. Call this method when you
    /// have calculated whether there's a complete gap between the current and
    /// previous candle wicks (no overlap at all).
    ///
    /// # Arguments
    /// * `gap` - None for no gap, Some(false) for gap up, Some(true) for gap down
    #[inline(always)]
    pub fn set_wick_gap(&mut self, gap: Option<bool>) {
        // Clear both gap bits first
        self.value &= !((1 << Self::WICK_GAP_PRESENT_BIT) | (1 << Self::WICK_GAP_DIRECTION_BIT));

        if let Some(gap_up) = gap {
            self.value |= 1 << Self::WICK_GAP_PRESENT_BIT; // Gap exists
                                                           // Note: gap_up = true means GAP_UP, gap_up = false means GAP_DOWN
                                                           // WICK_GAP_DIRECTION_BIT set means DOWN, unset means UP
            if !gap_up {
                // If gap is DOWN (false), set the direction bit
                self.value |= 1 << Self::WICK_GAP_DIRECTION_BIT; // Gap down
            }
            // If gap_up is true (GAP_UP), direction bit stays unset
        }

        // Mark both bits as computed
        self.computed |= (1 << Self::WICK_GAP_PRESENT_BIT) | (1 << Self::WICK_GAP_DIRECTION_BIT);
    }

    /// Create a wildcard (all bits set to match any value)
    pub const fn wildcard() -> Self {
        CandleBits {
            value: 0,
            computed: 0,
        }
    }

    /// Reconstruct the CandleTypes enum from the bitmask
    ///
    /// Extracts the candle type bits from the bitmask and returns the appropriate
    /// CandleTypes enum variant. Checks categories in priority order:
    /// DOJI -> BASIC -> MARUBOZU -> SPINNING_TOP -> OTHER
    pub fn get_candle_type(&self) -> CandleTypes {
        // Check DOJI first (bits 8-15)
        let doji_bits = ((self.value & Self::DOJI_MASK) >> Self::DOJI_OFFSET) as u8;
        if let Some(doji) = CDLDoji::from_bit(doji_bits) {
            return CandleTypes::Doji(doji);
        }

        // Check BASIC (bits 0-7)
        let basic_bits = ((self.value & Self::BASIC_MASK) >> Self::BASIC_OFFSET) as u8;
        if let Some(basic) = CDLBasic::from_bit(basic_bits) {
            return CandleTypes::Basic(basic);
        }

        // Check MARUBOZU (bits 16-23)
        let marubozu_bits = ((self.value & Self::MARUBOZU_MASK) >> Self::MARUBOZU_OFFSET) as u8;
        if let Some(marubozu) = CDLMarubozu::from_bit(marubozu_bits) {
            return CandleTypes::Marubozu(marubozu);
        }

        // Check SPINNING_TOP (bits 24-27)
        let spinning_top_bits =
            ((self.value & Self::SPINNING_TOP_MASK) >> Self::SPINNING_TOP_OFFSET) as u8;
        if let Some(spinning_top) = CDLSpinningTop::from_bit(spinning_top_bits) {
            return CandleTypes::SpinningTop(spinning_top);
        }

        // Check OTHER (bit 28)
        if self.value & (1 << Self::OTHER_BIT) != 0 {
            return CandleTypes::Other;
        }

        // Default to Other if no bits are set
        CandleTypes::Other
    }

    /// Get the colour of the candle
    ///
    /// Returns `common::GREEN` (true) if the candle is green, `common::RED` (false) otherwise.
    /// This represents the candle's momentum/trend color relative to previous close.
    #[inline(always)]
    pub fn get_colour(&self) -> bool {
        use crate::candle_indicators::common::{GREEN, RED};
        if self.value & Self::COLOUR_GREEN != 0 {
            GREEN
        } else {
            RED
        }
    }

    /// Get the fill of the candle
    ///
    /// Returns `common::HALLOW` (true) if the candle is hollow (close > open),
    /// or `common::FILL` (false) if the candle is filled (close < open).
    #[inline(always)]
    pub fn get_fill(&self) -> bool {
        use crate::candle_indicators::common::{FILL, HALLOW};
        if self.value & Self::FILL_HALLOW != 0 {
            HALLOW
        } else {
            FILL
        }
    }

    /// Get the trend direction
    ///
    /// Returns `common::UP_TREND` (true) for uptrend, `common::DOWN_TREND` (false) for downtrend.
    #[inline(always)]
    pub fn get_trend(&self) -> bool {
        use crate::candle_indicators::common::{DOWN_TREND, UP_TREND};
        if self.value & Self::TREND_UP != 0 {
            UP_TREND
        } else {
            DOWN_TREND
        }
    }

    /// Get the body height classification
    ///
    /// Returns `common::LONG` (true) if the body is long relative to average,
    /// or `common::SHORT` (false) if the body is short.
    #[inline(always)]
    pub fn get_body_height(&self) -> bool {
        use crate::candle_indicators::common::{LONG, SHORT};
        if self.value & Self::BODY_HEIGHT_LONG != 0 {
            LONG
        } else {
            SHORT
        }
    }

    /// Get the line height classification
    ///
    /// Returns `common::LONG` (true) if the total line (high to low) is long relative to average,
    /// or `common::SHORT` (false) if the line is short.
    #[inline(always)]
    pub fn get_line_height(&self) -> bool {
        use crate::candle_indicators::common::{LONG, SHORT};
        if self.value & Self::LINE_HEIGHT_LONG != 0 {
            LONG
        } else {
            SHORT
        }
    }

    /// Get the body gap information
    ///
    /// Returns:
    /// - `None` if there is no body gap with the previous candle
    /// - `Some(common::GAP_UP)` (false) if there is a gap up
    /// - `Some(common::GAP_DOWN)` (true) if there is a gap down
    #[inline(always)]
    pub fn get_body_gap(&self) -> Option<bool> {
        use crate::candle_indicators::common::{GAP_DOWN, GAP_UP};
        let gap_present = self.value & Self::BODY_GAP_PRESENT != 0;
        if gap_present {
            // Check the direction bit (bit 35)
            let gap_down = self.value & (1 << Self::BODY_GAP_DIRECTION_BIT) != 0;
            Some(if gap_down { GAP_DOWN } else { GAP_UP })
        } else {
            None
        }
    }

    /// Get the wick gap information
    ///
    /// Returns:
    /// - `None` if there is no wick gap with the previous candle
    /// - `Some(common::GAP_UP)` (false) if there is a gap up between wicks
    /// - `Some(common::GAP_DOWN)` (true) if there is a gap down between wicks
    #[inline(always)]
    pub fn get_wick_gap(&self) -> Option<bool> {
        use crate::candle_indicators::common::{GAP_DOWN, GAP_UP};
        let gap_present = self.value & Self::WICK_GAP_PRESENT != 0;
        if gap_present {
            // Check the direction bit (bit 37)
            let gap_down = self.value & (1 << Self::WICK_GAP_DIRECTION_BIT) != 0;
            Some(if gap_down { GAP_DOWN } else { GAP_UP })
        } else {
            None
        }
    }

    /// Check if this matches the given mask pattern
    /// Only checks bits that have been computed (are set in self.computed)
    #[inline(always)]
    pub fn matches_compulsory_only(&self, pattern: &PatternMask) -> bool {
        // For candle type fields (BASIC, DOJI, MARUBOZU, SPINNING_TOP, OTHER):
        // Check if ANY of the specified type bits match
        const CANDLE_TYPE_MASKS: u64 = CandleBits::BASIC_MASK
            | CandleBits::DOJI_MASK
            | CandleBits::MARUBOZU_MASK
            | CandleBits::SPINNING_TOP_MASK
            | CandleBits::OTHER;

        let candle_type_part = pattern.mask.value & CANDLE_TYPE_MASKS;
        let candle_value_part = pattern.value.value & CANDLE_TYPE_MASKS;

        // For other fields: only check compulsory bits (ignore lazy bits)
        let other_part = pattern.mask.value & !CANDLE_TYPE_MASKS & Self::COMPULSORY_MASK;

        // For candle types: match if ANY bit matches (OR logic)
        let candle_match = if candle_type_part != 0 {
            (self.value & candle_type_part & candle_value_part) != 0
        } else {
            true // No candle type requirement
        };

        // For other compulsory fields: exact match (AND logic)
        let other_match = (self.value & other_part) == (pattern.value.value & other_part);
        candle_match && other_match
    }

    pub fn matches(&self, pattern: &PatternMask) -> bool {
        PERF_COUNTERS.record_bit_match_call();

        // For candle type fields (BASIC, DOJI, MARUBOZU, SPINNING_TOP, OTHER):
        // Check if ANY of the specified type bits match
        const CANDLE_TYPE_MASKS: u64 = CandleBits::BASIC_MASK
            | CandleBits::DOJI_MASK
            | CandleBits::MARUBOZU_MASK
            | CandleBits::SPINNING_TOP_MASK
            | CandleBits::OTHER;

        let candle_type_part = pattern.mask.value & CANDLE_TYPE_MASKS;
        let candle_value_part = pattern.value.value & CANDLE_TYPE_MASKS;

        // For other fields: only check bits that have been computed
        // This allows lazy evaluation - uncomputed bits are ignored in matching
        let other_part = pattern.mask.value & !CANDLE_TYPE_MASKS & self.computed;

        // For candle types: match if ANY bit matches (OR logic)
        // IMPORTANT: Only check candle type bits, not other bits like body_gap!
        let candle_match = if candle_type_part != 0 {
            (self.value & candle_type_part & candle_value_part) != 0
        } else {
            true // No candle type requirement
        };

        // For other fields (colour, fill, trend, heights, gaps): exact match (AND logic)
        // Only checks bits that are both required (in pattern.mask) and computed (in self.computed)
        let other_match = (self.value & other_part) == (pattern.value.value & other_part);
        let result = candle_match && other_match;

        if result {
            PERF_COUNTERS.record_bit_match_success();
        }

        result
    }

    /// Returns the candle-type group index for dispatch:
    /// 0=Basic, 1=Doji, 2=Marubozu, 3=SpinningTop, None=Other (no patterns)
    #[inline(always)]
    pub fn candle_group(&self) -> Option<usize> {
        let v = self.value;
        if (v & Self::BASIC_MASK) != 0 {
            Some(0)
        } else if (v & Self::DOJI_MASK) != 0 {
            Some(1)
        } else if (v & Self::MARUBOZU_MASK) != 0 {
            Some(2)
        } else if (v & Self::SPINNING_TOP_MASK) != 0 {
            Some(3)
        } else {
            None
        }
    }
}

/// Pattern mask for matching bars
///
/// The mask defines which bits we care about (1 = must match, 0 = don't care)
/// The value defines the expected bit values for the bits we care about
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternMask {
    pub mask: CandleBits,  // Which bits to check
    pub value: CandleBits, // Expected values for checked bits
}

impl PatternMask {
    /// Create a new pattern mask
    pub const fn new(mask: u64, value: u64) -> Self {
        PatternMask {
            mask: CandleBits {
                value: mask,
                computed: 0,
            },
            value: CandleBits { value, computed: 0 },
        }
    }

    /// Create a wildcard pattern that matches anything
    pub const fn wildcard() -> Self {
        PatternMask {
            mask: CandleBits {
                value: 0,
                computed: 0,
            },
            value: CandleBits {
                value: 0,
                computed: 0,
            },
        }
    }

    /// Builder: Set colour requirement
    pub const fn with_colour(mut self, colour: bool) -> Self {
        self.mask.value |= 1 << CandleBits::COLOUR_BIT;
        if colour {
            self.value.value |= 1 << CandleBits::COLOUR_BIT;
        }
        self
    }

    /// Builder: Set fill requirement
    pub const fn with_fill(mut self, fill: bool) -> Self {
        self.mask.value |= 1 << CandleBits::FILL_BIT;
        if fill {
            self.value.value |= 1 << CandleBits::FILL_BIT;
        }
        self
    }

    /// Builder: Set candle type requirement
    pub const fn with_candle_type(mut self, candle_type_pattern: CandleTypePattern) -> Self {
        match candle_type_pattern {
            CandleTypePattern::Basic(variant_mask) => {
                self.mask.value |= CandleBits::BASIC_MASK;
                self.value.value |= (variant_mask as u64) << CandleBits::BASIC_OFFSET;
            }
            CandleTypePattern::Doji(variant_mask) => {
                self.mask.value |= CandleBits::DOJI_MASK;
                self.value.value |= (variant_mask as u64) << CandleBits::DOJI_OFFSET;
            }
            CandleTypePattern::Marubozu(variant_mask) => {
                self.mask.value |= CandleBits::MARUBOZU_MASK;
                self.value.value |= (variant_mask as u64) << CandleBits::MARUBOZU_OFFSET;
            }
            CandleTypePattern::SpinningTop(variant_mask) => {
                self.mask.value |= CandleBits::SPINNING_TOP_MASK;
                self.value.value |= (variant_mask as u64) << CandleBits::SPINNING_TOP_OFFSET;
            }
            CandleTypePattern::Any => {
                // Don't set any mask bits - matches anything
            }
        }
        self
    }

    /// Builder: Set negated candle type requirement
    ///
    /// Two behaviors:
    /// 1. If variant_mask is 0xFF (all variants), match anything EXCEPT that entire category
    ///    Example: !Doji(all) means match Basic OR Marubozu OR SpinningTop OR Other
    /// 2. If variant_mask specifies specific variants, match that category but EXCLUDE those variants
    ///    Example: !Doji(FourPriceDoji) means match Doji::Doji | Doji::LongLeggedDoji | Doji::DragonflyDoji | Doji::GravestoneDoji
    pub const fn with_negated_candle_type(
        mut self,
        candle_type_pattern: CandleTypePattern,
    ) -> Self {
        // Maximum variant masks for each category (all possible variants)
        const ALL_BASIC_VARIANTS: u8 = 0x3F; // Basic has 6 variants (bits 0-5)
        const ALL_DOJI_VARIANTS: u8 = 0x1F; // Doji has 5 variants (bits 0-4)
        const ALL_MARUBOZU_VARIANTS: u8 = 0x3F; // Marubozu has 6 variants (bits 0-5)
        const ALL_SPINNING_TOP_VARIANTS: u8 = 0x07; // SpinningTop has 3 variants (bits 0-2)

        match candle_type_pattern {
            CandleTypePattern::Basic(variant_mask) => {
                if variant_mask == ALL_BASIC_VARIANTS {
                    // Reject entire Basic category - match other categories
                    self.mask.value |= CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1 << CandleBits::OTHER_BIT);
                    self.value.value |= CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1 << CandleBits::OTHER_BIT);
                } else {
                    // Match Basic variants EXCEPT the specified ones, OR any other category
                    let inverted_mask = ALL_BASIC_VARIANTS & !variant_mask;
                    // Set all other categories
                    self.mask.value |= CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1 << CandleBits::OTHER_BIT);
                    self.value.value |= CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1 << CandleBits::OTHER_BIT);
                    // Also set Basic with inverted mask
                    self.mask.value |= CandleBits::BASIC_MASK;
                    self.value.value |= (inverted_mask as u64) << CandleBits::BASIC_OFFSET;
                }
            }
            CandleTypePattern::Doji(variant_mask) => {
                if variant_mask == ALL_DOJI_VARIANTS {
                    // Reject entire Doji category - match other categories
                    self.mask.value |= CandleBits::BASIC_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1 << CandleBits::OTHER_BIT);
                    self.value.value |= CandleBits::BASIC_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1 << CandleBits::OTHER_BIT);
                } else {
                    // Match Doji variants EXCEPT the specified ones, OR any other category
                    let inverted_mask = ALL_DOJI_VARIANTS & !variant_mask;
                    // Set all other categories
                    self.mask.value |= CandleBits::BASIC_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1 << CandleBits::OTHER_BIT);
                    self.value.value |= CandleBits::BASIC_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1 << CandleBits::OTHER_BIT);
                    // Also set Doji with inverted mask
                    self.mask.value |= CandleBits::DOJI_MASK;
                    self.value.value |= (inverted_mask as u64) << CandleBits::DOJI_OFFSET;
                }
            }
            CandleTypePattern::Marubozu(variant_mask) => {
                if variant_mask == ALL_MARUBOZU_VARIANTS {
                    // Reject entire Marubozu category - match other categories
                    self.mask.value |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1 << CandleBits::OTHER_BIT);
                    self.value.value |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1 << CandleBits::OTHER_BIT);
                } else {
                    // Match Marubozu variants EXCEPT the specified ones, OR any other category
                    let inverted_mask = ALL_MARUBOZU_VARIANTS & !variant_mask;
                    // Set all other categories
                    self.mask.value |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1 << CandleBits::OTHER_BIT);
                    self.value.value |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1 << CandleBits::OTHER_BIT);
                    // Also set Marubozu with inverted mask
                    self.mask.value |= CandleBits::MARUBOZU_MASK;
                    self.value.value |= (inverted_mask as u64) << CandleBits::MARUBOZU_OFFSET;
                }
            }
            CandleTypePattern::SpinningTop(variant_mask) => {
                if variant_mask == ALL_SPINNING_TOP_VARIANTS {
                    // Reject entire SpinningTop category - match other categories
                    self.mask.value |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | (1 << CandleBits::OTHER_BIT);
                    self.value.value |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | (1 << CandleBits::OTHER_BIT);
                } else {
                    // Match SpinningTop variants EXCEPT the specified ones, OR any other category
                    let inverted_mask = ALL_SPINNING_TOP_VARIANTS & !variant_mask;
                    // Set all other categories
                    self.mask.value |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | (1 << CandleBits::OTHER_BIT);
                    self.value.value |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | (1 << CandleBits::OTHER_BIT);
                    // Also set SpinningTop with inverted mask
                    self.mask.value |= CandleBits::SPINNING_TOP_MASK;
                    self.value.value |= (inverted_mask as u64) << CandleBits::SPINNING_TOP_OFFSET;
                }
            }
            CandleTypePattern::Any => {
                // Negating "Any" doesn't make sense, but we'll treat it as matching nothing
                // Set mask but no value bits - will never match
                self.mask.value |= CandleBits::BASIC_MASK
                    | CandleBits::DOJI_MASK
                    | CandleBits::MARUBOZU_MASK
                    | CandleBits::SPINNING_TOP_MASK
                    | (1 << CandleBits::OTHER_BIT);
            }
        }
        self
    }

    /// Builder: Set multiple negated candle type requirements (combines with AND logic)
    ///
    /// Handles patterns like: !Doji(FourPriceDoji) !Marubozu(BlackMarubozu | ClosingBlackMarubozu)
    /// This means: NOT FourPriceDoji AND NOT (BlackMarubozu or ClosingBlackMarubozu)
    ///
    /// Algorithm:
    /// 1. Start with all candle types allowed (all bits set)
    /// 2. For each negation, remove those variants from the allowed set (AND logic)
    /// 3. Build final bitmask from remaining allowed types
    ///
    /// This is used when you have multiple negations in a single candle_type attribute.
    pub const fn with_multiple_negated_candle_types(
        mut self,
        patterns: &[CandleTypePattern], // array of patterns to negate
    ) -> Self {
        // Maximum variant masks for each category (all possible variants)
        const ALL_BASIC_VARIANTS: u8 = 0x3F; // Basic has 6 variants (bits 0-5)
        const ALL_DOJI_VARIANTS: u8 = 0x1F; // Doji has 5 variants (bits 0-4)
        const ALL_MARUBOZU_VARIANTS: u8 = 0x3F; // Marubozu has 6 variants (bits 0-5)
        const ALL_SPINNING_TOP_VARIANTS: u8 = 0x07; // SpinningTop has 3 variants (bits 0-2)

        // Start with all variants of all categories allowed
        let mut allowed_basic: u8 = ALL_BASIC_VARIANTS;
        let mut allowed_doji: u8 = ALL_DOJI_VARIANTS;
        let mut allowed_marubozu: u8 = ALL_MARUBOZU_VARIANTS;
        let mut allowed_spinning: u8 = ALL_SPINNING_TOP_VARIANTS;
        let mut allowed_other: bool = true;

        // Process each negation - remove from allowed set
        let mut i = 0;
        while i < patterns.len() {
            match patterns[i] {
                CandleTypePattern::Basic(mask) => {
                    allowed_basic &= !mask;
                }
                CandleTypePattern::Doji(mask) => {
                    allowed_doji &= !mask;
                }
                CandleTypePattern::Marubozu(mask) => {
                    allowed_marubozu &= !mask;
                }
                CandleTypePattern::SpinningTop(mask) => {
                    allowed_spinning &= !mask;
                }
                CandleTypePattern::Any => {
                    // Negating "Any" means match nothing
                    allowed_basic = 0;
                    allowed_doji = 0;
                    allowed_marubozu = 0;
                    allowed_spinning = 0;
                    allowed_other = false;
                }
            }
            i += 1;
        }

        // Build the mask from the final allowed sets
        if allowed_basic != 0 {
            self.mask.value |= CandleBits::BASIC_MASK;
            self.value.value |= (allowed_basic as u64) << CandleBits::BASIC_OFFSET;
        }
        if allowed_doji != 0 {
            self.mask.value |= CandleBits::DOJI_MASK;
            self.value.value |= (allowed_doji as u64) << CandleBits::DOJI_OFFSET;
        }
        if allowed_marubozu != 0 {
            self.mask.value |= CandleBits::MARUBOZU_MASK;
            self.value.value |= (allowed_marubozu as u64) << CandleBits::MARUBOZU_OFFSET;
        }
        if allowed_spinning != 0 {
            self.mask.value |= CandleBits::SPINNING_TOP_MASK;
            self.value.value |= (allowed_spinning as u64) << CandleBits::SPINNING_TOP_OFFSET;
        }
        if allowed_other {
            self.mask.value |= 1 << CandleBits::OTHER_BIT;
            self.value.value |= 1 << CandleBits::OTHER_BIT;
        }

        self
    }

    /// Builder: Set trend requirement
    pub const fn with_trend(mut self, trend: bool) -> Self {
        self.mask.value |= 1 << CandleBits::TREND_BIT;
        if trend {
            self.value.value |= 1 << CandleBits::TREND_BIT;
        }
        self
    }

    /// Builder: Set body height requirement (LONG=true, SHORT=false)
    pub const fn with_body_height(mut self, is_long: bool) -> Self {
        self.mask.value |= 1 << CandleBits::BODY_HEIGHT_BIT;
        if is_long {
            self.value.value |= 1 << CandleBits::BODY_HEIGHT_BIT;
        }
        self
    }

    /// Builder: Set line height requirement (LONG=true, SHORT=false)
    pub const fn with_line_height(mut self, is_long: bool) -> Self {
        self.mask.value |= 1 << CandleBits::LINE_HEIGHT_BIT;
        if is_long {
            self.value.value |= 1 << CandleBits::LINE_HEIGHT_BIT;
        }
        self
    }

    /// Builder: Set body gap requirement (gap_up=false, gap_down=true)
    /// Body gap means current candle's body doesn't touch previous close
    pub const fn with_body_gap(mut self, gap_down: bool) -> Self {
        // Set both bits in mask (we care about presence AND direction)
        self.mask.value |= 1 << CandleBits::BODY_GAP_PRESENT_BIT;
        self.mask.value |= 1 << CandleBits::BODY_GAP_DIRECTION_BIT;
        // Set present bit in value (we require a gap to exist)
        self.value.value |= 1 << CandleBits::BODY_GAP_PRESENT_BIT;
        // Set direction bit if gap down
        if gap_down {
            self.value.value |= 1 << CandleBits::BODY_GAP_DIRECTION_BIT;
        }
        self
    }

    /// Builder: Set wick gap requirement (gap_up=false, gap_down=true)
    /// Wick gap means no overlap at all between current and previous candle
    pub const fn with_wick_gap(mut self, gap_down: bool) -> Self {
        // Set both bits in mask (we care about presence AND direction)
        self.mask.value |= 1 << CandleBits::WICK_GAP_PRESENT_BIT;
        self.mask.value |= 1 << CandleBits::WICK_GAP_DIRECTION_BIT;
        // Set present bit in value (we require a gap to exist)
        self.value.value |= 1 << CandleBits::WICK_GAP_PRESENT_BIT;
        // Set direction bit if gap down
        if gap_down {
            self.value.value |= 1 << CandleBits::WICK_GAP_DIRECTION_BIT;
        }
        self
    }

    /// Check if a bar matches this pattern
    #[inline(always)]
    pub fn matches(&self, bar: &CandleBits) -> bool {
        bar.matches(self)
    }
}

/// A pattern definition with its bar requirements
#[derive(Debug, Clone, Copy)]
pub struct PatternDefinition<const N: usize> {
    pub pattern: CandlePattern,
    pub forecast: ForcastType,
    pub bars: [PatternMask; N], // One mask per bar (oldest to newest)
    pub check_prev_bar: bool,   // If true, bars[0] contains prev_bar constraint
    pub lazy_bits_mask: u64,    // Which lazy bits this pattern needs (0 = none)
}

impl<const N: usize> PatternDefinition<N> {
    /// Create a new pattern definition
    pub const fn new(
        pattern: CandlePattern,
        forecast: ForcastType,
        bars: [PatternMask; N],
        check_prev_bar: bool,
        lazy_bits_mask: u64,
    ) -> Self {
        PatternDefinition {
            pattern,
            forecast,
            bars,
            check_prev_bar,
            lazy_bits_mask,
        }
    }

    /// Get the number of bars this pattern requires
    pub const fn bar_count(&self) -> usize {
        N
    }

    /// Check if this pattern has lazy bits that haven't been computed yet
    ///
    /// Returns true if:
    /// - The pattern uses lazy bits (lazy_bits_mask != 0), AND
    /// - Any bar in the slice is missing at least one lazy bit the pattern needs
    ///
    /// This method is used to determine when to call compute_bits() before matching.
    ///
    /// # Example
    /// ```ignore
    /// // Before matching, check if we need to compute lazy bits
    /// if pattern_def.has_uncomputed_lazy_bits(&bars) {
    ///     // Compute missing lazy bits for all bars
    ///     for bar in bars.iter_mut() {
    ///         bar.compute_bits(open, high, low, close, i, state);
    ///     }
    /// }
    /// // Now safe to match the pattern
    /// let matches = pattern_def.matches(&bars, is_uptrend);
    /// ```
    #[deprecated(
        note = "Do not use in hot paths. Use lazy_bits_mask != 0 check + unconditional compute_bits() instead."
    )]
    pub fn has_uncomputed_lazy_bits(&self, bars: &[CandleBits]) -> bool {
        if self.lazy_bits_mask == 0 {
            return false; // Pattern doesn't use lazy bits at all
        }

        // Check if any bar is missing lazy bits that the pattern needs
        bars.iter().any(|bar| {
            // Which lazy bits does the pattern need?
            let needed = self.lazy_bits_mask;
            // Which of those needed bits are NOT computed?
            let missing = needed & !bar.computed;
            missing != 0
        })
    }

    /// Check if this pattern matches the given bars (oldest to newest)
    /// Does NOT check trend - caller must verify trend separately
    #[inline(always)]
    pub fn matches_bars_compulsory_only(&self, bars: &[CandleBits]) -> bool {
        if bars.len() < self.bars.len() {
            return false;
        }

        let start = if self.check_prev_bar { 0 } else { 1 };
        for i in (start..self.bars.len()).rev() {
            if !bars[i].matches_compulsory_only(&self.bars[i]) {
                return false;
            }
        }

        true
    }

    pub fn matches_bars(&self, bars: &[CandleBits]) -> bool {
        PERF_COUNTERS.record_matches_bars_call();

        // When check_prev_bar is false, bars[0] is wildcard (skip mask[0] and bars[0])
        // When check_prev_bar is true, bars[0] must match mask[0]
        if bars.len() < self.bars.len() {
            PERF_COUNTERS.record_early_exit();
            return false;
        }

        let start = if self.check_prev_bar { 0 } else { 1 };
        for i in (start..self.bars.len()).rev() {
            if !self.bars[i].matches(&bars[i]) {
                PERF_COUNTERS.record_early_exit();
                return false;
            }
        }

        PERF_COUNTERS.record_matches_bars_success();
        true
    }

    /// Check if this pattern's trend requirement matches
    pub fn matches_trend(&self, is_uptrend: bool) -> bool {
        self.forecast.matches_trend(is_uptrend)
    }

    /// Full check: bars + trend
    #[inline(always)]
    pub fn matches(&self, bars: &mut [CandleBits], is_uptrend: bool) -> bool {
        self.matches_trend(is_uptrend) && self.matches_bars(bars)
    }
}

/// Emit the hot-path funnel (trend → compulsory bits → lazy compute → full bits → calc)
/// for one bar-count group.
///
/// Parameters:
/// - `patterns`    — a slice expression yielding `&[PatternDefinition<N>]`
/// - `min_bars`    — minimum number of bars required (`bars.len() >= min_bars`)
/// - `window_size` — how many bars to take from the tail of `bars`
/// - `bars`        — the `&mut [CandleBits]` buffer
/// - `inputs`      — the OHLC tuple
/// - `i`           — current bar index
/// - `state`       — `&EmaState`
/// - `matched`     — `Option<Vec<CandlePattern>>` accumulator (mutated in-place)
macro_rules! check_bar_group {
    (
        patterns = $patterns:expr,
        min_bars = $min_bars:expr,
        window_size = $window_size:expr,
        bars = $bars:expr,
        inputs = $inputs:expr,
        i = $i:expr,
        state = $state:expr,
        matched = $matched:expr
    ) => {{
        if $bars.len() >= $min_bars {
            let window_start = $bars.len() - $window_size;
            let window = &mut $bars[window_start..];
            let is_uptrend = (window[0].value & (1 << CandleBits::TREND_BIT)) != 0;
            for pattern_def in $patterns {
                PERF_COUNTERS.record_pattern_checked();
                if !pattern_def.matches_trend(is_uptrend) {
                    continue;
                }
                if !pattern_def.matches_bars_compulsory_only(window) {
                    continue;
                }
                if pattern_def.lazy_bits_mask != 0 {
                    pattern_def
                        .pattern
                        .compute_bits($inputs, $i, $state, window);
                }
                if !pattern_def.matches_bars(window) {
                    continue;
                }
                PERF_COUNTERS.record_calc_call();
                if pattern_def.pattern.calc($inputs, $i, $state, window) {
                    PERF_COUNTERS.record_calc_success();
                    $matched
                        .get_or_insert_with(|| Vec::with_capacity(1))
                        .push(pattern_def.pattern);
                    
                    //break;
                }
            }
        }
    }};
}

/// Pattern definition registry organized by bar count for efficient lookup
/// Holds owned PatternDefinition values
/// Note: All patterns now include a prev_bar slot, so 1-bar patterns have N=2, 2-bar patterns have N=3, etc.
#[derive(Debug)]
pub struct PatternDefinitionRegister<
    const N1: usize,
    const N2: usize,
    const N3: usize,
    const N4: usize,
    const N5: usize,
> {
    pub one_bar: [PatternDefinition<2>; N1], // 1 pattern bar + 1 prev_bar slot
    pub two_bar: [PatternDefinition<3>; N2], // 2 pattern bars + 1 prev_bar slot
    pub three_bar: [PatternDefinition<4>; N3], // 3 pattern bars + 1 prev_bar slot
    pub four_bar: [PatternDefinition<5>; N4], // 4 pattern bars + 1 prev_bar slot
    pub five_bar: [PatternDefinition<6>; N5], // 5 pattern bars + 1 prev_bar slot
}

/// Pattern definition registry with references (for forecast filtering)
/// Holds references to PatternDefinition values owned elsewhere
impl<const N1: usize, const N2: usize, const N3: usize, const N4: usize, const N5: usize>
    PatternDefinitionRegister<N1, N2, N3, N4, N5>
{
    /// Create a new registry from fixed-size arrays
    pub const fn new(
        one_bar: [PatternDefinition<2>; N1], // 1 pattern bar + 1 prev_bar slot
        two_bar: [PatternDefinition<3>; N2], // 2 pattern bars + 1 prev_bar slot
        three_bar: [PatternDefinition<4>; N3], // 3 pattern bars + 1 prev_bar slot
        four_bar: [PatternDefinition<5>; N4], // 4 pattern bars + 1 prev_bar slot
        five_bar: [PatternDefinition<6>; N5], // 5 pattern bars + 1 prev_bar slot
    ) -> Self {
        PatternDefinitionRegister {
            one_bar,
            two_bar,
            three_bar,
            four_bar,
            five_bar,
        }
    }

    /// Get validated patterns using group×trend dispatch.
    ///
    /// Selects the contiguous slice for (final-bar candle group, prev-bar trend)
    /// from each bar-count array, then runs the full match pipeline.
    pub fn get_validated_patterns_with_group_trend_dispatch(
        &self,
        bars: &mut [CandleBits],
        inputs: (&[f64], &[f64], &[f64], &[f64]),
        i: usize,
        state: &EmaState,
        gd: &GroupTrendDispatch,
    ) -> Option<Vec<CandlePattern>> {
        let mut matched_patterns: Option<Vec<CandlePattern>> = None;

        // Group is determined from the final (current) bar — constant across all bar counts.
        let group = match bars.last().and_then(|b| b.candle_group()) {
            Some(g) => g,
            None => return None, // Other type — no patterns defined for it
        };

        // 1-bar patterns: window = [prev_bar, bar1]
        if bars.len() >= 2 {
            let is_uptrend = (bars[bars.len() - 2].value & CandleBits::TREND_UP) != 0;
            let key = group * 2 + (is_uptrend as usize);
            let (start, end) = gd.one_bar[key];
            check_bar_group!(
                patterns = &self.one_bar[start..end],
                min_bars = 2,
                window_size = 2,
                bars = bars,
                inputs = inputs,
                i = i,
                state = state,
                matched = matched_patterns
            );
        }

        // 2-bar patterns: window = [prev_bar, bar1, bar2]
        if bars.len() >= 3 {
            let is_uptrend = (bars[bars.len() - 3].value & CandleBits::TREND_UP) != 0;
            let key = group * 2 + (is_uptrend as usize);
            let (start, end) = gd.two_bar[key];
            check_bar_group!(
                patterns = &self.two_bar[start..end],
                min_bars = 3,
                window_size = 3,
                bars = bars,
                inputs = inputs,
                i = i,
                state = state,
                matched = matched_patterns
            );
        }

        // 3-bar patterns
        if bars.len() >= 4 {
            let is_uptrend = (bars[bars.len() - 4].value & CandleBits::TREND_UP) != 0;
            let key = group * 2 + (is_uptrend as usize);
            let (start, end) = gd.three_bar[key];
            check_bar_group!(
                patterns = &self.three_bar[start..end],
                min_bars = 4,
                window_size = 4,
                bars = bars,
                inputs = inputs,
                i = i,
                state = state,
                matched = matched_patterns
            );
        }

        // 4-bar patterns
        if bars.len() >= 5 {
            let is_uptrend = (bars[bars.len() - 5].value & CandleBits::TREND_UP) != 0;
            let key = group * 2 + (is_uptrend as usize);
            let (start, end) = gd.four_bar[key];
            check_bar_group!(
                patterns = &self.four_bar[start..end],
                min_bars = 5,
                window_size = 5,
                bars = bars,
                inputs = inputs,
                i = i,
                state = state,
                matched = matched_patterns
            );
        }

        // 5-bar patterns
        if bars.len() >= 6 {
            let is_uptrend = (bars[bars.len() - 6].value & CandleBits::TREND_UP) != 0;
            let key = group * 2 + (is_uptrend as usize);
            let (start, end) = gd.five_bar[key];
            check_bar_group!(
                patterns = &self.five_bar[start..end],
                min_bars = 6,
                window_size = 6,
                bars = bars,
                inputs = inputs,
                i = i,
                state = state,
                matched = matched_patterns
            );
        }

        matched_patterns
    }
}

/// Group-trend dispatch table.
///
/// Patterns in each bar-count array are sorted by (group, trend, forecast_order, name).
/// Each of the 8 (group × trend) combinations maps to a contiguous slice [start..end].
///
/// Key = group * 2 + (is_uptrend as usize), range 0..8:
///   group  0=Basic, 1=Doji, 2=Marubozu, 3=SpinningTop
///   trend  0=DOWN,  1=UP
#[derive(Debug, Clone)]
pub struct GroupTrendDispatch {
    pub one_bar: [(usize, usize); 8],
    pub two_bar: [(usize, usize); 8],
    pub three_bar: [(usize, usize); 8],
    pub four_bar: [(usize, usize); 8],
    pub five_bar: [(usize, usize); 8],
}

impl GroupTrendDispatch {
    pub const fn new(
        one_bar: [(usize, usize); 8],
        two_bar: [(usize, usize); 8],
        three_bar: [(usize, usize); 8],
        four_bar: [(usize, usize); 8],
        five_bar: [(usize, usize); 8],
    ) -> Self {
        GroupTrendDispatch {
            one_bar,
            two_bar,
            three_bar,
            four_bar,
            five_bar,
        }
    }
}

/// Main pattern registry with group×trend dispatch.
///
/// Two lookup paths:
/// 1. Global (no forecast): group×trend dispatch over all patterns.
/// 2. Forecast-specific: group×trend dispatch over the pattern sub-range for that forecast.
pub struct PatternRegister<
    const N1: usize,
    const N2: usize,
    const N3: usize,
    const N4: usize,
    const N5: usize,
> {
    pub pattern_definitions: PatternDefinitionRegister<N1, N2, N3, N4, N5>,
    /// Group-trend dispatches indexed by `ForcastType as usize` (6 entries).
    pub forecast_dispatches: [GroupTrendDispatch; 6],
    /// Group-trend dispatch covering all patterns (no forecast filter).
    pub global_dispatch: GroupTrendDispatch,
}

impl<const N1: usize, const N2: usize, const N3: usize, const N4: usize, const N5: usize>
    PatternRegister<N1, N2, N3, N4, N5>
{
    pub const fn new(
        pattern_definitions: PatternDefinitionRegister<N1, N2, N3, N4, N5>,
        forecast_dispatches: [GroupTrendDispatch; 6],
        global_dispatch: GroupTrendDispatch,
    ) -> Self {
        PatternRegister {
            pattern_definitions,
            forecast_dispatches,
            global_dispatch,
        }
    }

    /// Get patterns matching registry filters + calc() validation.
    /// Returns at most one pattern per bar length (breaks after first match).
    pub fn get_validated_patterns(
        &self,
        bars: &mut [CandleBits],
        inputs: (&[f64], &[f64], &[f64], &[f64]),
        i: usize,
        state: &EmaState,
        forecast: Option<ForcastType>,
    ) -> Option<Vec<CandlePattern>> {
        let gd = match forecast {
            Some(fc) => {
                let idx = fc as usize;
                if idx < self.forecast_dispatches.len() {
                    &self.forecast_dispatches[idx]
                } else {
                    return None;
                }
            }
            None => &self.global_dispatch,
        };
        self.pattern_definitions
            .get_validated_patterns_with_group_trend_dispatch(bars, inputs, i, state, gd)
    }
}

// Note: PATTERN_REGISTRY and get_global_registry() are generated by build.rs
// and included via candle_patterns.rs module. They are not defined here to avoid
// symbol conflicts and debugger confusion.
