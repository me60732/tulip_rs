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
/// use crate::candle_indicators::common::{BODY_GAP_UP, BODY_GAP_DOWN, WICK_GAP_UP, WICK_GAP_DOWN};
/// PatternMask::new()
///     .with_colour(GREEN)
///     .with_body_gap(BODY_GAP_UP)
///     .with_wick_gap(WICK_GAP_DOWN)
/// ```
///
/// Gap types:
/// - **Body Gap**: Current candle's body doesn't touch previous close
/// - **Wick Gap**: No overlap at all between current and previous candle (complete gap)
use crate::candle_indicators::candle_patterns::CandlePattern;
use crate::candle_indicators::candle_types::{CDLBasic, CDLDoji, CDLMarubozu, CDLSpinningTop};
use crate::candle_indicators::common::{
    cdl_gap, BODY_GAP_DOWN, BODY_GAP_UP, NO_GAP, WICK_GAP_DOWN, WICK_GAP_UP,
};
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
/// Split into two fields (8 bytes total: mandatory: u32 + lazy_value: u16 + lazy_computed: u16):
///
/// **`mandatory: u32`** — always computed at bar creation (25 bits used, tight-packed):
/// ```text
///   Bits  0– 5   Basic variants      (6 types, 1-hot)
///   Bits  6–10   Doji variants       (5 types, 1-hot)
///   Bits 11–16   Marubozu variants   (6 types, 1-hot)
///   Bits 17–19   SpinningTop variants(3 types, 1-hot)
///   Bit  20      OTHER
///   Bit  21      COLOUR   (GREEN=1, RED=0)
///   Bit  22      FILL     (HOLLOW=1, FILLED=0)
///   Bit  23      TREND    (UP=1, DOWN=0)
///   Bit  24      LINE_HEIGHT (LONG=1, SHORT=0)
///   Bits 25–31   7 spare
/// ```
///
/// **`lazy_value / lazy_computed: u16`** — computed on demand (5 bits used):
/// ```text
///   Bit  0   BODY_HEIGHT       (LONG=1, SHORT=0)
///   Bit  1   BODY_GAP_PRESENT
///   Bit  2   BODY_GAP_DIRECTION (DOWN=1, UP=0)
///   Bit  3   WICK_GAP_PRESENT
///   Bit  4   WICK_GAP_DIRECTION (DOWN=1, UP=0)
///   Bits 5–15  11 spare for future lazy attributes
/// ```
///
/// ## Compulsory vs Lazy Bits
///
/// **Compulsory bits** (always computed at bar creation, stored in `mandatory`):
/// - Candle type (bits 0–20)
/// - Colour (bit 21)
/// - Fill (bit 22)
/// - Trend (bit 23)
/// - Line height (bit 24)
///
/// **Lazy bits** (computed on-demand, stored in `lazy_value` / tracked in `lazy_computed`):
/// - Body height (bit 0)
/// - Open position relative to prev body (bits 1–2)
/// - Close position relative to prev body (bits 3–4)
/// - High position relative to prev body/line (bits 5–7)
/// - Low position relative to prev body/line (bits 8–10)
/// - Engulf bits (bits 11–13)
/// - Wick 2× bits (bits 14–15)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CandleBits {
    pub mandatory: u32,     // Compulsory bits (always computed at bar creation)
    pub lazy_value: u16,    // Lazy bit values (computed on demand)
    pub lazy_computed: u16, // Which lazy bits have been computed
}

impl CandleBits {
    // Re-export bit position constants from tulip_rs_shared for single source of truth
    // Mandatory bit positions (shift amounts into `mandatory: u32`)
    pub const BASIC_OFFSET: u32 = tulip_rs_shared::BASIC_OFFSET;
    pub const DOJI_OFFSET: u32 = tulip_rs_shared::DOJI_OFFSET;
    pub const MARUBOZU_OFFSET: u32 = tulip_rs_shared::MARUBOZU_OFFSET;
    pub const SPINNING_TOP_OFFSET: u32 = tulip_rs_shared::SPINNING_TOP_OFFSET;
    pub const OTHER_BIT: u32 = tulip_rs_shared::OTHER_BIT;
    pub const COLOUR_BIT: u32 = tulip_rs_shared::COLOUR_BIT;
    pub const FILL_BIT: u32 = tulip_rs_shared::FILL_BIT;
    pub const TREND_BIT: u32 = tulip_rs_shared::TREND_BIT;
    pub const LINE_HEIGHT_BIT: u32 = tulip_rs_shared::LINE_HEIGHT_BIT;
    // Mandatory wick vs body bit positions (bits 25–26)
    pub const LOWER_WICK_LT_BODY_BIT: u32 = tulip_rs_shared::LOWER_WICK_LT_BODY_BIT;
    pub const UPPER_WICK_LT_BODY_BIT: u32 = tulip_rs_shared::UPPER_WICK_LT_BODY_BIT;
    // Mandatory body height bit position (bit 27)
    pub const BODY_HEIGHT_BIT: u32 = tulip_rs_shared::BODY_HEIGHT_BIT;
    // Lazy bit positions (shift amounts into `lazy_value / lazy_computed: u16`)
    pub const OPEN_ABOVE_PREV_BODY_MID_BIT: u32 = tulip_rs_shared::OPEN_ABOVE_PREV_BODY_MID_BIT;
    pub const OPEN_IN_PREV_BODY_BIT: u32 = tulip_rs_shared::OPEN_IN_PREV_BODY_BIT;
    pub const CLOSE_ABOVE_PREV_BODY_MID_BIT: u32 = tulip_rs_shared::CLOSE_ABOVE_PREV_BODY_MID_BIT;
    pub const CLOSE_IN_PREV_BODY_BIT: u32 = tulip_rs_shared::CLOSE_IN_PREV_BODY_BIT;
    pub const HIGH_ABOVE_PREV_BODY_MID_BIT: u32 = tulip_rs_shared::HIGH_ABOVE_PREV_BODY_MID_BIT;
    pub const HIGH_IN_PREV_BODY_BIT: u32 = tulip_rs_shared::HIGH_IN_PREV_BODY_BIT;
    pub const HIGH_IN_PREV_LINE_BIT: u32 = tulip_rs_shared::HIGH_IN_PREV_LINE_BIT;
    pub const LOW_ABOVE_PREV_BODY_MID_BIT: u32 = tulip_rs_shared::LOW_ABOVE_PREV_BODY_MID_BIT;
    pub const LOW_IN_PREV_BODY_BIT: u32 = tulip_rs_shared::LOW_IN_PREV_BODY_BIT;
    pub const LOW_IN_PREV_LINE_BIT: u32 = tulip_rs_shared::LOW_IN_PREV_LINE_BIT;
    pub const I_ENGULF_PREV_BODY_BIT: u32 = tulip_rs_shared::I_ENGULF_PREV_BODY_BIT;
    pub const PREV_HIGH_IN_MY_BODY_BIT: u32 = tulip_rs_shared::PREV_HIGH_IN_MY_BODY_BIT;
    pub const PREV_LOW_IN_MY_BODY_BIT: u32 = tulip_rs_shared::PREV_LOW_IN_MY_BODY_BIT;
    pub const LOWER_WICK_LONG_2X_BIT: u32 = tulip_rs_shared::LOWER_WICK_LONG_2X_BIT;
    pub const UPPER_WICK_LONG_2X_BIT: u32 = tulip_rs_shared::UPPER_WICK_LONG_2X_BIT;
    pub const BODY_GT_PREV_BODY_BIT: u32 = tulip_rs_shared::BODY_GT_PREV_BODY_BIT;

    // Re-export masks from tulip_rs_shared
    pub const BASIC_MASK: u32 = tulip_rs_shared::BASIC_MASK;
    pub const DOJI_MASK: u32 = tulip_rs_shared::DOJI_MASK;
    pub const MARUBOZU_MASK: u32 = tulip_rs_shared::MARUBOZU_MASK;
    pub const SPINNING_TOP_MASK: u32 = tulip_rs_shared::SPINNING_TOP_MASK;

    // Re-export masks for compulsory and lazy bits
    pub const CANDLE_TYPE_MASK: u32 = tulip_rs_shared::CANDLE_TYPE_MASK;
    pub const COMPULSORY_MASK: u32 = tulip_rs_shared::COMPULSORY_MASK;
    pub const LAZY_MASK: u16 = tulip_rs_shared::LAZY_MASK;

    // ========================================================================
    // PUBLIC BIT CONSTANTS FOR FAST PATTERN MATCHING
    // ========================================================================
    // These constants allow direct bit operations in pattern calc() functions
    // for maximum performance. Use these instead of get_candle_type() in hot paths.
    //
    // Example usage:
    //     if bars[i].mandatory & CandleBits::HIGH_WAVE != 0 { ... }
    //     if bars[i].mandatory & CandleBits::COLOUR_GREEN != 0 { ... }

    // === Basic Candle Types (bits 0–5) ===
    pub const SHORT_WHITE_CANDLE: u32 = tulip_rs_shared::SHORT_WHITE_CANDLE;
    pub const WHITE_CANDLE: u32 = tulip_rs_shared::WHITE_CANDLE;
    pub const LONG_WHITE_CANDLE: u32 = tulip_rs_shared::LONG_WHITE_CANDLE;
    pub const SHORT_BLACK_CANDLE: u32 = tulip_rs_shared::SHORT_BLACK_CANDLE;
    pub const BLACK_CANDLE: u32 = tulip_rs_shared::BLACK_CANDLE;
    pub const LONG_BLACK_CANDLE: u32 = tulip_rs_shared::LONG_BLACK_CANDLE;

    // === Doji Types (bits 6–10) ===
    pub const DOJI: u32 = tulip_rs_shared::DOJI;
    pub const LONG_LEGGED_DOJI: u32 = tulip_rs_shared::LONG_LEGGED_DOJI;
    pub const DRAGONFLY_DOJI: u32 = tulip_rs_shared::DRAGONFLY_DOJI;
    pub const GRAVESTONE_DOJI: u32 = tulip_rs_shared::GRAVESTONE_DOJI;
    pub const FOUR_PRICE_DOJI: u32 = tulip_rs_shared::FOUR_PRICE_DOJI;

    // === Marubozu Types (bits 11–16) ===
    pub const WHITE_MARUBOZU: u32 = tulip_rs_shared::WHITE_MARUBOZU;
    pub const OPENING_WHITE_MARUBOZU: u32 = tulip_rs_shared::OPENING_WHITE_MARUBOZU;
    pub const CLOSING_WHITE_MARUBOZU: u32 = tulip_rs_shared::CLOSING_WHITE_MARUBOZU;
    pub const BLACK_MARUBOZU: u32 = tulip_rs_shared::BLACK_MARUBOZU;
    pub const OPENING_BLACK_MARUBOZU: u32 = tulip_rs_shared::OPENING_BLACK_MARUBOZU;
    pub const CLOSING_BLACK_MARUBOZU: u32 = tulip_rs_shared::CLOSING_BLACK_MARUBOZU;

    // === SpinningTop Types (bits 17–19) ===
    pub const WHITE_SPINNING_TOP: u32 = tulip_rs_shared::WHITE_SPINNING_TOP;
    pub const BLACK_SPINNING_TOP: u32 = tulip_rs_shared::BLACK_SPINNING_TOP;
    pub const HIGH_WAVE: u32 = tulip_rs_shared::HIGH_WAVE;

    // === Other Type (bit 20) ===
    pub const OTHER: u32 = tulip_rs_shared::OTHER;

    // === Colour (bit 21) ===
    pub const COLOUR_GREEN: u32 = tulip_rs_shared::COLOUR_GREEN;
    pub const COLOUR_RED: u32 = tulip_rs_shared::COLOUR_RED;

    // === Fill (bit 22) ===
    pub const FILL_HALLOW: u32 = tulip_rs_shared::FILL_HOLLOW; // Note: shared uses HOLLOW spelling
    pub const FILL_FILLED: u32 = tulip_rs_shared::FILL_FILLED;

    // === Trend (bit 23) ===
    pub const TREND_UP: u32 = tulip_rs_shared::TREND_UP;
    pub const TREND_DOWN: u32 = tulip_rs_shared::TREND_DOWN;

    // === Body Height (mandatory bit 27) ===
    pub const BODY_HEIGHT_LONG: u32 = tulip_rs_shared::BODY_HEIGHT_LONG;
    pub const BODY_HEIGHT_SHORT: u32 = tulip_rs_shared::BODY_HEIGHT_SHORT;

    // === Line Height (mandatory bit 24) ===
    pub const LINE_HEIGHT_LONG: u32 = tulip_rs_shared::LINE_HEIGHT_LONG;
    pub const LINE_HEIGHT_SHORT: u32 = tulip_rs_shared::LINE_HEIGHT_SHORT;

    // === Wick vs Body (mandatory bits 25–26) ===
    pub const LOWER_WICK_LT_BODY: u32 = tulip_rs_shared::LOWER_WICK_LT_BODY;
    pub const UPPER_WICK_LT_BODY: u32 = tulip_rs_shared::UPPER_WICK_LT_BODY;

    // === Open position (lazy bits 0–1) ===
    pub const OPEN_ABOVE_PREV_BODY_MID: u16 = tulip_rs_shared::OPEN_ABOVE_PREV_BODY_MID;
    pub const OPEN_IN_PREV_BODY: u16 = tulip_rs_shared::OPEN_IN_PREV_BODY;

    // === Close position (lazy bits 2–3) ===
    pub const CLOSE_ABOVE_PREV_BODY_MID: u16 = tulip_rs_shared::CLOSE_ABOVE_PREV_BODY_MID;
    pub const CLOSE_IN_PREV_BODY: u16 = tulip_rs_shared::CLOSE_IN_PREV_BODY;

    // === High position (lazy bits 4–6) ===
    pub const HIGH_ABOVE_PREV_BODY_MID: u16 = tulip_rs_shared::HIGH_ABOVE_PREV_BODY_MID;
    pub const HIGH_IN_PREV_BODY: u16 = tulip_rs_shared::HIGH_IN_PREV_BODY;
    pub const HIGH_IN_PREV_LINE: u16 = tulip_rs_shared::HIGH_IN_PREV_LINE;

    // === Low position (lazy bits 7–9) ===
    pub const LOW_ABOVE_PREV_BODY_MID: u16 = tulip_rs_shared::LOW_ABOVE_PREV_BODY_MID;
    pub const LOW_IN_PREV_BODY: u16 = tulip_rs_shared::LOW_IN_PREV_BODY;
    pub const LOW_IN_PREV_LINE: u16 = tulip_rs_shared::LOW_IN_PREV_LINE;

    // === Engulf bits (lazy bits 10–12) ===
    pub const I_ENGULF_PREV_BODY: u16 = tulip_rs_shared::I_ENGULF_PREV_BODY;
    pub const PREV_HIGH_IN_MY_BODY: u16 = tulip_rs_shared::PREV_HIGH_IN_MY_BODY;
    pub const PREV_LOW_IN_MY_BODY: u16 = tulip_rs_shared::PREV_LOW_IN_MY_BODY;

    // === Wick 2× bits (lazy bits 13–14) ===
    pub const LOWER_WICK_LONG_2X: u16 = tulip_rs_shared::LOWER_WICK_LONG_2X;
    pub const UPPER_WICK_LONG_2X: u16 = tulip_rs_shared::UPPER_WICK_LONG_2X;

    // === Body vs previous body size (lazy bit 15) ===
    pub const BODY_GT_PREV_BODY: u16 = tulip_rs_shared::BODY_GT_PREV_BODY;

    /// Sets all compulsory bits immediately. Lazy position/engulf/wick-2x
    /// attributes are left unset and computed on-demand via the `set_*` methods
    /// when a pattern actually requires them.
    ///
    /// Compulsory bits (stored in `mandatory: u32`):
    /// - Candle type (bits 0–20)
    /// - Colour (bit 21)
    /// - Fill (bit 22)
    /// - Trend (bit 23)
    /// - Line height (bit 24)
    /// - Lower wick < body (bit 25)
    /// - Upper wick < body (bit 26)
    /// - Body height (bit 27)
    #[inline(always)]
    pub fn new(
        candle_type: &CandleTypes,
        colour: bool,
        fill: bool,
        trend: bool,
        line_height: bool,
        lower_wick_lt_body: bool,
        upper_wick_lt_body: bool,
        body_height: bool,
    ) -> Self {
        let mut mandatory: u32 = 0;

        // Set candle type bits using tulip_rs_shared encoding functions
        // Use discriminant() to get the variant index (0, 1, 2, ...) which the encoding functions expect
        match candle_type {
            CandleTypes::Basic(variant) => {
                mandatory |= tulip_rs_shared::encode_basic_variant(variant.discriminant() as u32);
            }
            CandleTypes::Doji(variant) => {
                mandatory |= tulip_rs_shared::encode_doji_variant(variant.discriminant() as u32);
            }
            CandleTypes::Marubozu(variant) => {
                mandatory |=
                    tulip_rs_shared::encode_marubozu_variant(variant.discriminant() as u32);
            }
            CandleTypes::SpinningTop(variant) => {
                mandatory |=
                    tulip_rs_shared::encode_spinning_top_variant(variant.discriminant() as u32);
            }
            CandleTypes::Other => {
                mandatory |= 1u32 << Self::OTHER_BIT;
            }
        }

        // Set colour bit (GREEN=1, RED=0)
        if colour {
            mandatory |= 1u32 << Self::COLOUR_BIT;
        }

        // Set fill bit (HALLOW=1, FILL=0)
        if fill {
            mandatory |= 1u32 << Self::FILL_BIT;
        }

        // Set trend bit (UP_TREND=1, DOWN_TREND=0)
        if trend {
            mandatory |= 1u32 << Self::TREND_BIT;
        }

        // Set line height bit (LONG=1, SHORT=0)
        if line_height {
            mandatory |= 1u32 << Self::LINE_HEIGHT_BIT;
        }

        // Set lower wick < body bit (mandatory bit 25)
        if lower_wick_lt_body {
            mandatory |= 1u32 << Self::LOWER_WICK_LT_BODY_BIT;
        }

        // Set upper wick < body bit (mandatory bit 26)
        if upper_wick_lt_body {
            mandatory |= 1u32 << Self::UPPER_WICK_LT_BODY_BIT;
        }

        // Set body height bit (mandatory bit 27)
        if body_height {
            mandatory |= 1u32 << Self::BODY_HEIGHT_BIT;
        }

        // If a wick is already known to be shorter than the body it cannot be ≥ 2× the body.
        // Pre-mark those lazy bits as computed=true, value=false so the 2× calculation is
        // never needed and pattern mask checks against them short-circuit immediately.
        let mut lazy_computed: u16 = 0;
        if lower_wick_lt_body {
            lazy_computed |= 1u16 << Self::LOWER_WICK_LONG_2X_BIT;
        }
        if upper_wick_lt_body {
            lazy_computed |= 1u16 << Self::UPPER_WICK_LONG_2X_BIT;
        }

        CandleBits {
            mandatory,
            lazy_value: 0,
            lazy_computed,
        }
    }
    #[inline(always)]
    pub fn apply_gap(&mut self, prev: (f64, f64, f64, f64), current: (f64, f64, f64, f64)) -> i8 {
        let (prev_open, prev_high, prev_low, prev_close) = prev;
        let (cur_open, cur_high, cur_low, cur_close) = current;
        let gap = cdl_gap(prev, current);
        self.set_gap(gap);
        // Always mark high/low in-line bits — correct for all gap types
        if gap != WICK_GAP_UP && gap != WICK_GAP_DOWN {
            self.set_high_in_line(cur_high >= prev_low && cur_high <= prev_high);
            self.set_low_in_line(cur_low >= prev_low && cur_low <= prev_high);

            if gap == NO_GAP {
                // Bodies overlap: open/close body-position bits need first principles too
                let prev_body_bot = prev_open.min(prev_close);
                let prev_body_top = prev_open.max(prev_close);
                let prev_mid = (prev_body_bot + prev_body_top) / 2.0;
                self.set_open_above_mid(cur_open > prev_mid);
                self.set_open_in_body(cur_open >= prev_body_bot && cur_open <= prev_body_top);
                self.set_close_above_mid(cur_close > prev_mid);
                self.set_close_in_body(cur_close >= prev_body_bot && cur_close <= prev_body_top);
            }
        }
        gap
    }

    /// Compute all relative-position lazy bits (1–13) from raw OHLC data.
    ///
    /// Unlike `apply_gap` which infers bits from the gap type, this method has
    /// full OHLC data for both bars and sets every position bit exactly — no
    /// inference needed.  Call this from `compute_bits()` for patterns that
    /// need engulf or position bits so they are populated before the pattern
    /// evaluator runs.
    ///
    /// **Bits set:** 1–2 (open), 3–4 (close), 5–7 (high), 8–10 (low), 11–13 (engulf)
    ///
    /// # Arguments
    /// * `prev`    — `(open, high, low, close)` of the previous bar
    /// * `current` — `(open, high, low, close)` of the current bar
    #[inline(always)]
    pub fn apply_engulfing(&mut self, prev: (f64, f64, f64, f64), current: (f64, f64, f64, f64)) {
        let (prev_open, prev_high, prev_low, prev_close) = prev;
        let (cur_open, cur_high, cur_low, cur_close) = current;

        let prev_body_top = prev_open.max(prev_close);
        let prev_body_bot = prev_open.min(prev_close);
        let prev_mid = (prev_body_top + prev_body_bot) / 2.0;
        let cur_body_top = cur_open.max(cur_close);
        let cur_body_bot = cur_open.min(cur_close);

        // === Bits 1–2: open vs prev body ===
        self.set_open_above_mid(cur_open > prev_mid);
        self.set_open_in_body(cur_open >= prev_body_bot && cur_open <= prev_body_top);

        // === Bits 3–4: close vs prev body ===
        self.set_close_above_mid(cur_close > prev_mid);
        self.set_close_in_body(cur_close >= prev_body_bot && cur_close <= prev_body_top);

        // === Bits 5–7: high vs prev body / line ===
        self.set_high_above_mid(cur_high > prev_mid);
        self.set_high_in_body(cur_high >= prev_body_bot && cur_high <= prev_body_top);
        self.set_high_in_line(cur_high >= prev_low && cur_high <= prev_high);

        // === Bits 8–10: low vs prev body / line ===
        self.set_low_above_mid(cur_low > prev_mid);
        self.set_low_in_body(cur_low >= prev_body_bot && cur_low <= prev_body_top);
        self.set_low_in_line(cur_low >= prev_low && cur_low <= prev_high);

        // === Bit 11: I engulf prev body ===
        // My body must fully span prev body AND be strictly wider on at least one side.
        // One side may be flush (cur_top == prev_top OR cur_bot == prev_bot), but both
        // sides flush simultaneously means same-size — that is not an engulf.
        let body_engulf = cur_body_top >= prev_body_top
            && cur_body_bot <= prev_body_bot
            && (cur_body_top > prev_body_top || cur_body_bot < prev_body_bot);
        self.set_engulfs_prev(body_engulf);

        // === Bit 15: body_gt_prev_body ===
        // Engulfing my body contains the prev body and extends at least one side,
        // so my body is definitively larger — stamp TRUE for free while the
        // geometry is already computed. Non-engulf is ambiguous; leave uncomputed.
        if body_engulf {
            self.set_body_gt_prev_body(true);
        }

        // === Bits 12–13: prev high/low within my body ===
        self.set_prev_high_in_my_body(prev_high <= cur_body_top && prev_high >= cur_body_bot);
        self.set_prev_low_in_my_body(prev_low <= cur_body_top && prev_low >= cur_body_bot);
    }

    #[inline(always)]
    fn set_gap(&mut self, gap: i8) {
        match gap {
            WICK_GAP_DOWN => {
                self.set_high_above_mid(false);
                self.set_low_above_mid(false);
                self.set_high_in_body(false);
                self.set_high_in_line(false);
                self.set_low_in_body(false);
                self.set_low_in_line(false);

                self.set_open_above_mid(false);
                self.set_open_in_body(false);
                self.set_close_above_mid(false);
                self.set_close_in_body(false);
            }
            BODY_GAP_DOWN => {
                self.set_low_above_mid(false);
                self.set_low_in_body(false);
                self.set_open_above_mid(false);
                self.set_open_in_body(false);
                self.set_close_above_mid(false);
                self.set_close_in_body(false);
            }
            WICK_GAP_UP => {
                self.set_high_above_mid(true);
                self.set_low_above_mid(true);
                self.set_high_in_body(false);
                self.set_high_in_line(false);
                self.set_low_in_body(false);
                self.set_low_in_line(false);

                self.set_open_above_mid(true);
                self.set_open_in_body(false);
                self.set_close_above_mid(true);
                self.set_close_in_body(false);
            }
            BODY_GAP_UP => {
                self.set_high_above_mid(true);
                self.set_high_in_body(false);

                self.set_open_above_mid(true);
                self.set_open_in_body(false);
                self.set_close_above_mid(true);
                self.set_close_in_body(false);
            }

            _ => {}
        }
    }
    /// Set the body height attribute (mandatory bit 27).
    ///
    /// # Arguments
    /// * `is_long` - true for LONG body, false for SHORT body
    #[inline(always)]
    pub fn set_body_height(&mut self, is_long: bool) {
        if is_long {
            self.mandatory |= 1u32 << Self::BODY_HEIGHT_BIT;
        } else {
            self.mandatory &= !(1u32 << Self::BODY_HEIGHT_BIT);
        }
    }

    // --- Granular open position setters ---
    // Each sets exactly one bit and marks only that bit computed.
    // Use these when you know only a subset of the open-position bits.
    // Use set_open_position() when both are known.

    /// Set whether the current bar's open is above the prev bar's body midpoint.
    #[inline(always)]
    pub fn set_open_above_mid(&mut self, above_mid: bool) {
        if above_mid {
            self.lazy_value |= 1u16 << Self::OPEN_ABOVE_PREV_BODY_MID_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::OPEN_ABOVE_PREV_BODY_MID_BIT);
        }
        self.lazy_computed |= 1u16 << Self::OPEN_ABOVE_PREV_BODY_MID_BIT;
    }

    /// Set whether the current bar's open is within the prev bar's body [BOT, TOP].
    #[inline(always)]
    pub fn set_open_in_body(&mut self, in_body: bool) {
        if in_body {
            self.lazy_value |= 1u16 << Self::OPEN_IN_PREV_BODY_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::OPEN_IN_PREV_BODY_BIT);
        }
        self.lazy_computed |= 1u16 << Self::OPEN_IN_PREV_BODY_BIT;
    }

    /// Set both open position bits at once. Use when both values are known.
    /// Composes the two granular setters above.
    ///
    /// # Arguments
    /// * `above_mid` - true if open is above prev body midpoint
    /// * `in_body`   - true if open is within prev body [BOT, TOP]
    #[inline(always)]
    pub fn set_open_position(&mut self, above_mid: bool, in_body: bool) {
        self.set_open_above_mid(above_mid);
        self.set_open_in_body(in_body);
    }

    // --- Granular close position setters ---
    // Each sets exactly one bit and marks only that bit computed.
    // Use these when you know only a subset of the close-position bits.
    // Use set_close_position() when both are known.

    /// Set whether the current bar's close is above the prev bar's body midpoint.
    #[inline(always)]
    pub fn set_close_above_mid(&mut self, above_mid: bool) {
        if above_mid {
            self.lazy_value |= 1u16 << Self::CLOSE_ABOVE_PREV_BODY_MID_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::CLOSE_ABOVE_PREV_BODY_MID_BIT);
        }
        self.lazy_computed |= 1u16 << Self::CLOSE_ABOVE_PREV_BODY_MID_BIT;
    }

    /// Set whether the current bar's close is within the prev bar's body [BOT, TOP].
    #[inline(always)]
    pub fn set_close_in_body(&mut self, in_body: bool) {
        if in_body {
            self.lazy_value |= 1u16 << Self::CLOSE_IN_PREV_BODY_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::CLOSE_IN_PREV_BODY_BIT);
        }
        self.lazy_computed |= 1u16 << Self::CLOSE_IN_PREV_BODY_BIT;
    }

    /// Set both close position bits at once. Use when both values are known.
    /// Composes the two granular setters above.
    ///
    /// # Arguments
    /// * `above_mid` - true if close is above prev body midpoint
    /// * `in_body`   - true if close is within prev body [BOT, TOP]
    #[inline(always)]
    pub fn set_close_position(&mut self, above_mid: bool, in_body: bool) {
        self.set_close_above_mid(above_mid);
        self.set_close_in_body(in_body);
    }

    // --- Granular high position setters ---
    // Each sets exactly one bit and marks only that bit computed.
    // Use these when you know only a subset of the high-position bits.
    // Use set_high_position() when all three are known.

    /// Set whether the current bar's high is above the prev bar's body midpoint.
    #[inline(always)]
    pub fn set_high_above_mid(&mut self, above_mid: bool) {
        if above_mid {
            self.lazy_value |= 1u16 << Self::HIGH_ABOVE_PREV_BODY_MID_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::HIGH_ABOVE_PREV_BODY_MID_BIT);
        }
        self.lazy_computed |= 1u16 << Self::HIGH_ABOVE_PREV_BODY_MID_BIT;
    }

    /// Set whether the current bar's high is within the prev bar's body [BOT, TOP].
    #[inline(always)]
    pub fn set_high_in_body(&mut self, in_body: bool) {
        if in_body {
            self.lazy_value |= 1u16 << Self::HIGH_IN_PREV_BODY_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::HIGH_IN_PREV_BODY_BIT);
        }
        self.lazy_computed |= 1u16 << Self::HIGH_IN_PREV_BODY_BIT;
    }

    /// Set whether the current bar's high is within the prev bar's full line [LOW, HIGH].
    #[inline(always)]
    pub fn set_high_in_line(&mut self, in_line: bool) {
        if in_line {
            self.lazy_value |= 1u16 << Self::HIGH_IN_PREV_LINE_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::HIGH_IN_PREV_LINE_BIT);
        }
        self.lazy_computed |= 1u16 << Self::HIGH_IN_PREV_LINE_BIT;
    }

    // --- Granular low position setters ---

    /// Set whether the current bar's low is above the prev bar's body midpoint.
    #[inline(always)]
    pub fn set_low_above_mid(&mut self, above_mid: bool) {
        if above_mid {
            self.lazy_value |= 1u16 << Self::LOW_ABOVE_PREV_BODY_MID_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::LOW_ABOVE_PREV_BODY_MID_BIT);
        }
        self.lazy_computed |= 1u16 << Self::LOW_ABOVE_PREV_BODY_MID_BIT;
    }

    /// Set whether the current bar's low is within the prev bar's body [BOT, TOP].
    #[inline(always)]
    pub fn set_low_in_body(&mut self, in_body: bool) {
        if in_body {
            self.lazy_value |= 1u16 << Self::LOW_IN_PREV_BODY_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::LOW_IN_PREV_BODY_BIT);
        }
        self.lazy_computed |= 1u16 << Self::LOW_IN_PREV_BODY_BIT;
    }

    /// Set whether the current bar's low is within the prev bar's full line [LOW, HIGH].
    #[inline(always)]
    pub fn set_low_in_line(&mut self, in_line: bool) {
        if in_line {
            self.lazy_value |= 1u16 << Self::LOW_IN_PREV_LINE_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::LOW_IN_PREV_LINE_BIT);
        }
        self.lazy_computed |= 1u16 << Self::LOW_IN_PREV_LINE_BIT;
    }

    /// Set all three high position bits at once. Use when all values are known
    /// (e.g. body_gap patterns that compute prev body bounds anyway).
    /// Composes the three granular setters above.
    #[inline(always)]
    pub fn set_high_position(&mut self, above_mid: bool, in_body: bool, in_line: bool) {
        self.set_high_above_mid(above_mid);
        self.set_high_in_body(in_body);
        self.set_high_in_line(in_line);
    }

    /// Set all three low position bits at once. Use when all values are known.
    /// Composes the three granular setters above.
    #[inline(always)]
    pub fn set_low_position(&mut self, above_mid: bool, in_body: bool, in_line: bool) {
        self.set_low_above_mid(above_mid);
        self.set_low_in_body(in_body);
        self.set_low_in_line(in_line);
    }

    /// Set whether my body engulfs prev bar's body (lazy evaluation)
    ///
    /// True means prev open AND prev close are both within my body range.
    #[inline(always)]
    pub fn set_engulfs_prev(&mut self, does_engulf: bool) {
        if does_engulf {
            self.lazy_value |= 1u16 << Self::I_ENGULF_PREV_BODY_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::I_ENGULF_PREV_BODY_BIT);
        }
        self.lazy_computed |= 1u16 << Self::I_ENGULF_PREV_BODY_BIT;
    }

    /// Set whether prev bar's high is within my body (lazy evaluation)
    #[inline(always)]
    pub fn set_prev_high_in_my_body(&mut self, is_in: bool) {
        if is_in {
            self.lazy_value |= 1u16 << Self::PREV_HIGH_IN_MY_BODY_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::PREV_HIGH_IN_MY_BODY_BIT);
        }
        self.lazy_computed |= 1u16 << Self::PREV_HIGH_IN_MY_BODY_BIT;
    }

    /// Set whether prev bar's low is within my body (lazy evaluation)
    #[inline(always)]
    pub fn set_prev_low_in_my_body(&mut self, is_in: bool) {
        if is_in {
            self.lazy_value |= 1u16 << Self::PREV_LOW_IN_MY_BODY_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::PREV_LOW_IN_MY_BODY_BIT);
        }
        self.lazy_computed |= 1u16 << Self::PREV_LOW_IN_MY_BODY_BIT;
    }

    /// Set the lower wick ≥ 2× body height bit (lazy evaluation)
    #[inline(always)]
    pub fn set_lower_wick_2x(&mut self, is_2x: bool) {
        if is_2x {
            self.lazy_value |= 1u16 << Self::LOWER_WICK_LONG_2X_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::LOWER_WICK_LONG_2X_BIT);
        }
        self.lazy_computed |= 1u16 << Self::LOWER_WICK_LONG_2X_BIT;
    }

    /// Set the upper wick ≥ 2× body height bit (lazy evaluation)
    #[inline(always)]
    pub fn set_upper_wick_2x(&mut self, is_2x: bool) {
        if is_2x {
            self.lazy_value |= 1u16 << Self::UPPER_WICK_LONG_2X_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::UPPER_WICK_LONG_2X_BIT);
        }
        self.lazy_computed |= 1u16 << Self::UPPER_WICK_LONG_2X_BIT;
    }

    /// Set whether this bar's body is strictly greater than the previous bar's body (lazy bit 15).
    ///
    /// `TRUE` = |close - open| > |prev_close - prev_open|
    /// `FALSE` = less than OR equal (ties count as FALSE)
    #[inline(always)]
    pub fn set_body_gt_prev_body(&mut self, is_gt: bool) {
        if is_gt {
            self.lazy_value |= 1u16 << Self::BODY_GT_PREV_BODY_BIT;
        } else {
            self.lazy_value &= !(1u16 << Self::BODY_GT_PREV_BODY_BIT);
        }
        self.lazy_computed |= 1u16 << Self::BODY_GT_PREV_BODY_BIT;
    }

    /// Set the body height bit from raw OHLC values.
    /// Prefer passing `body_height` to `CandleBits::new()`; use this only
    /// when the bit needs to be recomputed after construction.
    #[inline(always)]
    pub fn ensure_body_height(&mut self, open: f64, close: f64, ema_body: f64) {
        let body = (open - close).abs();
        self.set_body_height(body >= ema_body);
    }

    /// Ensure BODY_GT_PREV_BODY_BIT (lazy bit 15): current body > previous body.
    ///
    /// Computes abs(close - open) vs abs(prev_close - prev_open) and stamps the bit.
    /// Equal sizes are treated as NOT greater (FALSE).
    #[inline(always)]
    pub fn ensure_body_gt_prev_body(
        &mut self,
        open: f64,
        close: f64,
        prev_open: f64,
        prev_close: f64,
    ) {
        if (self.lazy_computed & (1u16 << Self::BODY_GT_PREV_BODY_BIT)) != 0 {
            return;
        }
        let body = (open - close).abs();
        let prev_body = (prev_open - prev_close).abs();
        self.set_body_gt_prev_body(body > prev_body);
    }

    /// Ensure OPEN_IN_PREV_BODY + OPEN_ABOVE_PREV_BODY_MID — single geometry pass.
    #[inline(always)]
    pub fn ensure_open_position(&mut self, open: f64, prev_open: f64, prev_close: f64) {
        let open_in_mask = 1u16 << Self::OPEN_IN_PREV_BODY_BIT;
        let open_above_mask = 1u16 << Self::OPEN_ABOVE_PREV_BODY_MID_BIT;
        let needs_in = (self.lazy_computed & open_in_mask) == 0;
        let needs_above = (self.lazy_computed & open_above_mask) == 0;
        if !needs_in && !needs_above {
            return;
        }
        let body_top = prev_open.max(prev_close);
        let body_bot = prev_open.min(prev_close);
        if needs_in {
            self.set_open_in_body(open >= body_bot && open <= body_top);
        }
        if needs_above {
            let body_mid = (body_top + body_bot) / 2.0;
            self.set_open_above_mid(open > body_mid);
        }
    }

    /// Ensure CLOSE_IN_PREV_BODY + CLOSE_ABOVE_PREV_BODY_MID — single geometry pass.
    #[inline(always)]
    pub fn ensure_close_position(&mut self, close: f64, prev_open: f64, prev_close: f64) {
        let close_in_mask = 1u16 << Self::CLOSE_IN_PREV_BODY_BIT;
        let close_above_mask = 1u16 << Self::CLOSE_ABOVE_PREV_BODY_MID_BIT;
        let needs_in = (self.lazy_computed & close_in_mask) == 0;
        let needs_above = (self.lazy_computed & close_above_mask) == 0;
        if !needs_in && !needs_above {
            return;
        }
        let body_top = prev_open.max(prev_close);
        let body_bot = prev_open.min(prev_close);
        if needs_in {
            self.set_close_in_body(close >= body_bot && close <= body_top);
        }
        if needs_above {
            let body_mid = (body_top + body_bot) / 2.0;
            self.set_close_above_mid(close > body_mid);
        }
    }

    /// Ensure all four open+close position bits — body geometry computed once.
    #[inline(always)]
    pub fn ensure_open_close_position(
        &mut self,
        open: f64,
        close: f64,
        prev_open: f64,
        prev_close: f64,
    ) {
        let open_in_mask = 1u16 << Self::OPEN_IN_PREV_BODY_BIT;
        let open_above_mask = 1u16 << Self::OPEN_ABOVE_PREV_BODY_MID_BIT;
        let close_in_mask = 1u16 << Self::CLOSE_IN_PREV_BODY_BIT;
        let close_above_mask = 1u16 << Self::CLOSE_ABOVE_PREV_BODY_MID_BIT;
        let needs_open_in = (self.lazy_computed & open_in_mask) == 0;
        let needs_open_above = (self.lazy_computed & open_above_mask) == 0;
        let needs_close_in = (self.lazy_computed & close_in_mask) == 0;
        let needs_close_above = (self.lazy_computed & close_above_mask) == 0;
        if !needs_open_in && !needs_open_above && !needs_close_in && !needs_close_above {
            return;
        }
        let body_top = prev_open.max(prev_close);
        let body_bot = prev_open.min(prev_close);
        if needs_open_in {
            self.set_open_in_body(open >= body_bot && open <= body_top);
        }
        if needs_close_in {
            self.set_close_in_body(close >= body_bot && close <= body_top);
        }
        if needs_open_above || needs_close_above {
            let body_mid = (body_top + body_bot) / 2.0;
            if needs_open_above {
                self.set_open_above_mid(open > body_mid);
            }
            if needs_close_above {
                self.set_close_above_mid(close > body_mid);
            }
        }
    }

    /// Ensure LOWER_WICK_LONG_2X + UPPER_WICK_LONG_2X — body computed once.
    #[inline(always)]
    pub fn ensure_wick_2x(&mut self, open: f64, close: f64, high: f64, low: f64) {
        let lower_mask = 1u16 << Self::LOWER_WICK_LONG_2X_BIT;
        let upper_mask = 1u16 << Self::UPPER_WICK_LONG_2X_BIT;
        let needs_lower = (self.lazy_computed & lower_mask) == 0;
        let needs_upper = (self.lazy_computed & upper_mask) == 0;
        if !needs_lower && !needs_upper {
            return;
        }
        let body = (open - close).abs();
        let body_top = open.max(close);
        let body_bot = open.min(close);
        if needs_lower {
            let lower_wick = body_bot - low;
            self.set_lower_wick_2x(lower_wick >= 2.0 * body);
        }
        if needs_upper {
            let upper_wick = high - body_top;
            self.set_upper_wick_2x(upper_wick >= 2.0 * body);
        }
    }

    /// Ensure LOW_IN_PREV_LINE_BIT: low is within [prev_low, prev_high].
    #[inline(always)]
    pub fn ensure_low_in_prev_line(&mut self, low: f64, prev_low: f64, prev_high: f64) {
        if (self.lazy_computed & (1u16 << Self::LOW_IN_PREV_LINE_BIT)) != 0 {
            return;
        }
        self.set_low_in_line(low >= prev_low && low <= prev_high);
    }

    /// Ensure HIGH_IN_PREV_LINE_BIT: high is within [prev_low, prev_high].
    #[inline(always)]
    pub fn ensure_high_in_prev_line(&mut self, high: f64, prev_low: f64, prev_high: f64) {
        if (self.lazy_computed & (1u16 << Self::HIGH_IN_PREV_LINE_BIT)) != 0 {
            return;
        }
        self.set_high_in_line(high >= prev_low && high <= prev_high);
    }

    /// Create a wildcard (all bits set to match any value)
    pub const fn wildcard() -> Self {
        CandleBits {
            mandatory: 0,
            lazy_value: 0,
            lazy_computed: 0,
        }
    }

    /// Reconstruct the CandleTypes enum from the bitmask
    ///
    /// Extracts the candle type bits from the bitmask and returns the appropriate
    /// CandleTypes enum variant. Checks categories in priority order:
    /// DOJI -> BASIC -> MARUBOZU -> SPINNING_TOP -> OTHER
    pub fn get_candle_type(&self) -> CandleTypes {
        // Check DOJI first (bits 6–10)
        let doji_bits = ((self.mandatory & Self::DOJI_MASK) >> Self::DOJI_OFFSET) as u8;
        if let Some(doji) = CDLDoji::from_bit(doji_bits) {
            return CandleTypes::Doji(doji);
        }

        // Check BASIC (bits 0–5)
        let basic_bits = ((self.mandatory & Self::BASIC_MASK) >> Self::BASIC_OFFSET) as u8;
        if let Some(basic) = CDLBasic::from_bit(basic_bits) {
            return CandleTypes::Basic(basic);
        }

        // Check MARUBOZU (bits 11–16)
        let marubozu_bits = ((self.mandatory & Self::MARUBOZU_MASK) >> Self::MARUBOZU_OFFSET) as u8;
        if let Some(marubozu) = CDLMarubozu::from_bit(marubozu_bits) {
            return CandleTypes::Marubozu(marubozu);
        }

        // Check SPINNING_TOP (bits 17–19)
        let spinning_top_bits =
            ((self.mandatory & Self::SPINNING_TOP_MASK) >> Self::SPINNING_TOP_OFFSET) as u8;
        if let Some(spinning_top) = CDLSpinningTop::from_bit(spinning_top_bits) {
            return CandleTypes::SpinningTop(spinning_top);
        }

        // Check OTHER (bit 20)
        if self.mandatory & (1u32 << Self::OTHER_BIT) != 0 {
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
        if self.mandatory & Self::COLOUR_GREEN != 0 {
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
        if self.mandatory & Self::FILL_HALLOW != 0 {
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
        if self.mandatory & Self::TREND_UP != 0 {
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
        if self.mandatory & Self::BODY_HEIGHT_LONG != 0 {
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
        if self.mandatory & Self::LINE_HEIGHT_LONG != 0 {
            LONG
        } else {
            SHORT
        }
    }

    /// Returns true if the lower wick height is less than the body height (mandatory bit 25).
    #[inline(always)]
    pub fn get_lower_wick_lt_body(&self) -> bool {
        self.mandatory & Self::LOWER_WICK_LT_BODY != 0
    }

    /// Returns true if the upper wick height is less than the body height (mandatory bit 26).
    #[inline(always)]
    pub fn get_upper_wick_lt_body(&self) -> bool {
        self.mandatory & Self::UPPER_WICK_LT_BODY != 0
    }

    /// Check if this matches the given mask pattern
    /// Only checks bits that have been computed (are set in self.computed)
    #[inline(always)]
    pub fn matches_compulsory_only(&self, pattern: &PatternMask) -> bool {
        let candle_type_part = pattern.mandatory_mask & CandleBits::CANDLE_TYPE_MASK;

        // For candle types: match if ANY bit matches (OR logic)
        let candle_match = if candle_type_part != 0 {
            (self.mandatory & candle_type_part & pattern.mandatory_value) != 0
        } else {
            true // No candle type requirement
        };

        // For other compulsory fields: exact match (AND logic)
        let other_part = pattern.mandatory_mask & !CandleBits::CANDLE_TYPE_MASK;
        let mandatory_match =
            (self.mandatory & other_part) == (pattern.mandatory_value & other_part);
        candle_match && mandatory_match
    }

    pub fn matches(&self, pattern: &PatternMask) -> bool {
        PERF_COUNTERS.record_bit_match_call();

        let candle_type_part = pattern.mandatory_mask & CandleBits::CANDLE_TYPE_MASK;

        // For candle types: match if ANY bit matches (OR logic)
        let candle_match = if candle_type_part != 0 {
            (self.mandatory & candle_type_part & pattern.mandatory_value) != 0
        } else {
            true // No candle type requirement
        };

        // For other mandatory fields (colour, fill, trend, line_height): exact match (AND logic)
        let other_mandatory_part = pattern.mandatory_mask & !CandleBits::CANDLE_TYPE_MASK;
        let mandatory_match = (self.mandatory & other_mandatory_part)
            == (pattern.mandatory_value & other_mandatory_part);

        // Lazy: only check bits that have been computed
        // This allows lazy evaluation - uncomputed bits are ignored in matching
        let lazy_part = pattern.lazy_mask & self.lazy_computed;
        let lazy_match = (self.lazy_value & lazy_part) == (pattern.lazy_value & lazy_part);

        let result = candle_match && mandatory_match && lazy_match;
        if result {
            PERF_COUNTERS.record_bit_match_success();
        }
        result
    }

    /// Returns the candle-type group index for dispatch:
    /// 0=Basic, 1=Doji, 2=Marubozu, 3=SpinningTop, None=Other (no patterns)
    #[inline(always)]
    pub fn candle_group(&self) -> Option<usize> {
        let v = self.mandatory;
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
    pub mandatory_mask: u32,  // Which mandatory bits to check
    pub mandatory_value: u32, // Expected mandatory bit values
    pub lazy_mask: u16,       // Which lazy bits to check
    pub lazy_value: u16,      // Expected lazy bit values
    pub has_engulf: bool,     // true when template declares engulf_prev or inside_prev
    pub has_gap: bool,        // true when template declares body_gap or wick_gap
}

impl PatternMask {
    /// Create a new pattern mask
    pub const fn new(mandatory_mask: u32, mandatory_value: u32) -> Self {
        PatternMask {
            mandatory_mask,
            mandatory_value,
            lazy_mask: 0,
            lazy_value: 0,
            has_engulf: false,
            has_gap: false,
        }
    }

    /// Create a wildcard pattern that matches anything
    pub const fn wildcard() -> Self {
        PatternMask {
            mandatory_mask: 0,
            mandatory_value: 0,
            lazy_mask: 0,
            lazy_value: 0,
            has_engulf: false,
            has_gap: false,
        }
    }

    /// Builder: Set colour requirement
    pub const fn with_colour(mut self, colour: bool) -> Self {
        self.mandatory_mask |= 1u32 << CandleBits::COLOUR_BIT;
        if colour {
            self.mandatory_value |= 1u32 << CandleBits::COLOUR_BIT;
        }
        self
    }

    /// Builder: Set fill requirement
    pub const fn with_fill(mut self, fill: bool) -> Self {
        self.mandatory_mask |= 1u32 << CandleBits::FILL_BIT;
        if fill {
            self.mandatory_value |= 1u32 << CandleBits::FILL_BIT;
        }
        self
    }

    /// Builder: Set candle type requirement
    pub const fn with_candle_type(mut self, candle_type_pattern: CandleTypePattern) -> Self {
        match candle_type_pattern {
            CandleTypePattern::Basic(variant_mask) => {
                self.mandatory_mask |= CandleBits::BASIC_MASK;
                self.mandatory_value |= (variant_mask as u32) << CandleBits::BASIC_OFFSET;
            }
            CandleTypePattern::Doji(variant_mask) => {
                self.mandatory_mask |= CandleBits::DOJI_MASK;
                self.mandatory_value |= (variant_mask as u32) << CandleBits::DOJI_OFFSET;
            }
            CandleTypePattern::Marubozu(variant_mask) => {
                self.mandatory_mask |= CandleBits::MARUBOZU_MASK;
                self.mandatory_value |= (variant_mask as u32) << CandleBits::MARUBOZU_OFFSET;
            }
            CandleTypePattern::SpinningTop(variant_mask) => {
                self.mandatory_mask |= CandleBits::SPINNING_TOP_MASK;
                self.mandatory_value |= (variant_mask as u32) << CandleBits::SPINNING_TOP_OFFSET;
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
                    self.mandatory_mask |= CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                    self.mandatory_value |= CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                } else {
                    let inverted_mask = ALL_BASIC_VARIANTS & !variant_mask;
                    self.mandatory_mask |= CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                    self.mandatory_value |= CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                    self.mandatory_mask |= CandleBits::BASIC_MASK;
                    self.mandatory_value |= (inverted_mask as u32) << CandleBits::BASIC_OFFSET;
                }
            }
            CandleTypePattern::Doji(variant_mask) => {
                if variant_mask == ALL_DOJI_VARIANTS {
                    self.mandatory_mask |= CandleBits::BASIC_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                    self.mandatory_value |= CandleBits::BASIC_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                } else {
                    let inverted_mask = ALL_DOJI_VARIANTS & !variant_mask;
                    self.mandatory_mask |= CandleBits::BASIC_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                    self.mandatory_value |= CandleBits::BASIC_MASK
                        | CandleBits::MARUBOZU_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                    self.mandatory_mask |= CandleBits::DOJI_MASK;
                    self.mandatory_value |= (inverted_mask as u32) << CandleBits::DOJI_OFFSET;
                }
            }
            CandleTypePattern::Marubozu(variant_mask) => {
                if variant_mask == ALL_MARUBOZU_VARIANTS {
                    self.mandatory_mask |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                    self.mandatory_value |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                } else {
                    let inverted_mask = ALL_MARUBOZU_VARIANTS & !variant_mask;
                    self.mandatory_mask |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                    self.mandatory_value |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::SPINNING_TOP_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                    self.mandatory_mask |= CandleBits::MARUBOZU_MASK;
                    self.mandatory_value |= (inverted_mask as u32) << CandleBits::MARUBOZU_OFFSET;
                }
            }
            CandleTypePattern::SpinningTop(variant_mask) => {
                if variant_mask == ALL_SPINNING_TOP_VARIANTS {
                    self.mandatory_mask |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                    self.mandatory_value |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                } else {
                    let inverted_mask = ALL_SPINNING_TOP_VARIANTS & !variant_mask;
                    self.mandatory_mask |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                    self.mandatory_value |= CandleBits::BASIC_MASK
                        | CandleBits::DOJI_MASK
                        | CandleBits::MARUBOZU_MASK
                        | (1u32 << CandleBits::OTHER_BIT);
                    self.mandatory_mask |= CandleBits::SPINNING_TOP_MASK;
                    self.mandatory_value |=
                        (inverted_mask as u32) << CandleBits::SPINNING_TOP_OFFSET;
                }
            }
            CandleTypePattern::Any => {
                // Negating "Any" doesn't make sense, but we'll treat it as matching nothing
                // Set mask but no value bits - will never match
                self.mandatory_mask |= CandleBits::BASIC_MASK
                    | CandleBits::DOJI_MASK
                    | CandleBits::MARUBOZU_MASK
                    | CandleBits::SPINNING_TOP_MASK
                    | (1u32 << CandleBits::OTHER_BIT);
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
            self.mandatory_mask |= CandleBits::BASIC_MASK;
            self.mandatory_value |= (allowed_basic as u32) << CandleBits::BASIC_OFFSET;
        }
        if allowed_doji != 0 {
            self.mandatory_mask |= CandleBits::DOJI_MASK;
            self.mandatory_value |= (allowed_doji as u32) << CandleBits::DOJI_OFFSET;
        }
        if allowed_marubozu != 0 {
            self.mandatory_mask |= CandleBits::MARUBOZU_MASK;
            self.mandatory_value |= (allowed_marubozu as u32) << CandleBits::MARUBOZU_OFFSET;
        }
        if allowed_spinning != 0 {
            self.mandatory_mask |= CandleBits::SPINNING_TOP_MASK;
            self.mandatory_value |= (allowed_spinning as u32) << CandleBits::SPINNING_TOP_OFFSET;
        }
        if allowed_other {
            self.mandatory_mask |= 1u32 << CandleBits::OTHER_BIT;
            self.mandatory_value |= 1u32 << CandleBits::OTHER_BIT;
        }

        self
    }

    /// Builder: Set trend requirement
    pub const fn with_trend(mut self, trend: bool) -> Self {
        self.mandatory_mask |= 1u32 << CandleBits::TREND_BIT;
        if trend {
            self.mandatory_value |= 1u32 << CandleBits::TREND_BIT;
        }
        self
    }

    /// Builder: Set body height requirement (LONG=true, SHORT=false)
    pub const fn with_body_height(mut self, is_long: bool) -> Self {
        self.mandatory_mask |= 1u32 << CandleBits::BODY_HEIGHT_BIT;
        if is_long {
            self.mandatory_value |= 1u32 << CandleBits::BODY_HEIGHT_BIT;
        }
        self
    }

    /// Builder: Set line height requirement (LONG=true, SHORT=false)
    pub const fn with_line_height(mut self, is_long: bool) -> Self {
        self.mandatory_mask |= 1u32 << CandleBits::LINE_HEIGHT_BIT;
        if is_long {
            self.mandatory_value |= 1u32 << CandleBits::LINE_HEIGHT_BIT;
        }
        self
    }

    /// Builder: Require lower wick < body height (mandatory bit 25)
    pub const fn with_lower_wick_lt_body(mut self, is_lt: bool) -> Self {
        self.mandatory_mask |= 1u32 << CandleBits::LOWER_WICK_LT_BODY_BIT;
        if is_lt {
            self.mandatory_value |= 1u32 << CandleBits::LOWER_WICK_LT_BODY_BIT;
        }
        self
    }

    /// Builder: Require upper wick < body height (mandatory bit 26)
    pub const fn with_upper_wick_lt_body(mut self, is_lt: bool) -> Self {
        self.mandatory_mask |= 1u32 << CandleBits::UPPER_WICK_LT_BODY_BIT;
        if is_lt {
            self.mandatory_value |= 1u32 << CandleBits::UPPER_WICK_LT_BODY_BIT;
        }
        self
    }

    /// Builder: Set body gap requirement using position bits.
    ///
    /// `gap` must be one of:
    /// - `BODY_GAP_UP`  (1)  — open and close are both above prev body
    ///   (OPEN/CLOSE not in prev body, OPEN/CLOSE above prev body midpoint)
    /// - `BODY_GAP_DOWN` (-1) — open and close are both below prev body
    ///   (OPEN/CLOSE not in prev body, OPEN/CLOSE below prev body midpoint)
    pub const fn with_body_gap(mut self, gap: i8) -> Self {
        self.has_gap = true;
        // Require OPEN_IN_PREV_BODY = 0 and CLOSE_IN_PREV_BODY = 0 (gap means not in body)
        // Require OPEN_ABOVE and CLOSE_ABOVE to match the direction
        self.lazy_mask |= (1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT)
            | (1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT)
            | (1u16 << CandleBits::OPEN_ABOVE_PREV_BODY_MID_BIT)
            | (1u16 << CandleBits::CLOSE_ABOVE_PREV_BODY_MID_BIT);
        // IN_PREV_BODY bits must be 0 (anti-mask) — clear them in value
        self.lazy_value &= !((1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT)
            | (1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT));
        if gap > 0 {
            // BODY_GAP_UP: open and close are above prev body
            self.lazy_value |= (1u16 << CandleBits::OPEN_ABOVE_PREV_BODY_MID_BIT)
                | (1u16 << CandleBits::CLOSE_ABOVE_PREV_BODY_MID_BIT);
        } else {
            // BODY_GAP_DOWN: open and close are below prev body (anti-mask)
            self.lazy_value &= !((1u16 << CandleBits::OPEN_ABOVE_PREV_BODY_MID_BIT)
                | (1u16 << CandleBits::CLOSE_ABOVE_PREV_BODY_MID_BIT));
        }
        self
    }

    /// Builder: Set wick gap requirement using position bits.
    ///
    /// `gap` must be one of:
    /// - `WICK_GAP_UP`   (2)  — entire candle is above prev full range (my low > prev high)
    /// - `WICK_GAP_DOWN` (-2) — entire candle is below prev full range (my high < prev low)
    pub const fn with_wick_gap(mut self, gap: i8) -> Self {
        self.has_gap = true;
        // HIGH_IN_PREV_LINE and LOW_IN_PREV_LINE must be 0 (no overlap with prev line)
        // HIGH_ABOVE and LOW_ABOVE indicate direction
        self.lazy_mask |= (1u16 << CandleBits::HIGH_IN_PREV_LINE_BIT)
            | (1u16 << CandleBits::LOW_IN_PREV_LINE_BIT)
            | (1u16 << CandleBits::HIGH_ABOVE_PREV_BODY_MID_BIT)
            | (1u16 << CandleBits::LOW_ABOVE_PREV_BODY_MID_BIT);
        // IN_PREV_LINE bits must be 0 (anti-mask)
        self.lazy_value &= !((1u16 << CandleBits::HIGH_IN_PREV_LINE_BIT)
            | (1u16 << CandleBits::LOW_IN_PREV_LINE_BIT));
        if gap > 0 {
            // WICK_GAP_UP: high and low are above prev range
            self.lazy_value |= (1u16 << CandleBits::HIGH_ABOVE_PREV_BODY_MID_BIT)
                | (1u16 << CandleBits::LOW_ABOVE_PREV_BODY_MID_BIT);
        } else {
            // WICK_GAP_DOWN: high and low are below prev range (anti-mask)
            self.lazy_value &= !((1u16 << CandleBits::HIGH_ABOVE_PREV_BODY_MID_BIT)
                | (1u16 << CandleBits::LOW_ABOVE_PREV_BODY_MID_BIT));
        }
        self
    }

    /// Builder: Require open above prev body midpoint (lazy bit 1)
    pub const fn with_open_above_prev_mid(mut self, is_above: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::OPEN_ABOVE_PREV_BODY_MID_BIT;
        if is_above {
            self.lazy_value |= 1u16 << CandleBits::OPEN_ABOVE_PREV_BODY_MID_BIT;
        }
        self
    }

    /// Builder: Require open within prev body (lazy bit 2)
    pub const fn with_open_in_prev_body(mut self, is_in: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT;
        if is_in {
            self.lazy_value |= 1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT;
        }
        self
    }

    /// Builder: Require close above prev body midpoint (lazy bit 3)
    pub const fn with_close_above_prev_mid(mut self, is_above: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::CLOSE_ABOVE_PREV_BODY_MID_BIT;
        if is_above {
            self.lazy_value |= 1u16 << CandleBits::CLOSE_ABOVE_PREV_BODY_MID_BIT;
        }
        self
    }

    /// Builder: Require close within prev body (lazy bit 4)
    pub const fn with_close_in_prev_body(mut self, is_in: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT;
        if is_in {
            self.lazy_value |= 1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT;
        }
        self
    }

    /// Builder: Require high above prev body midpoint (lazy bit 5)
    pub const fn with_high_above_prev_mid(mut self, is_above: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::HIGH_ABOVE_PREV_BODY_MID_BIT;
        if is_above {
            self.lazy_value |= 1u16 << CandleBits::HIGH_ABOVE_PREV_BODY_MID_BIT;
        }
        self
    }

    /// Builder: Require high within prev body (lazy bit 6)
    pub const fn with_high_in_prev_body(mut self, is_in: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::HIGH_IN_PREV_BODY_BIT;
        if is_in {
            self.lazy_value |= 1u16 << CandleBits::HIGH_IN_PREV_BODY_BIT;
        }
        self
    }

    /// Builder: Require high within prev full line [prev LOW, prev HIGH] (lazy bit 7)
    pub const fn with_high_in_prev_line(mut self, is_in: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::HIGH_IN_PREV_LINE_BIT;
        if is_in {
            self.lazy_value |= 1u16 << CandleBits::HIGH_IN_PREV_LINE_BIT;
        }
        self
    }

    /// Builder: Require low above prev body midpoint (lazy bit 8)
    pub const fn with_low_above_prev_mid(mut self, is_above: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::LOW_ABOVE_PREV_BODY_MID_BIT;
        if is_above {
            self.lazy_value |= 1u16 << CandleBits::LOW_ABOVE_PREV_BODY_MID_BIT;
        }
        self
    }

    /// Builder: Require low within prev body (lazy bit 9)
    pub const fn with_low_in_prev_body(mut self, is_in: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::LOW_IN_PREV_BODY_BIT;
        if is_in {
            self.lazy_value |= 1u16 << CandleBits::LOW_IN_PREV_BODY_BIT;
        }
        self
    }

    /// Builder: Require low within prev full line [prev LOW, prev HIGH] (lazy bit 10)
    pub const fn with_low_in_prev_line(mut self, is_in: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::LOW_IN_PREV_LINE_BIT;
        if is_in {
            self.lazy_value |= 1u16 << CandleBits::LOW_IN_PREV_LINE_BIT;
        }
        self
    }

    /// Builder: Require my body engulfs prev body — both prev open and prev close
    /// are within my body range (lazy bit 11)
    pub const fn with_engulfs_prev(mut self, does_engulf: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::I_ENGULF_PREV_BODY_BIT;
        if does_engulf {
            self.lazy_value |= 1u16 << CandleBits::I_ENGULF_PREV_BODY_BIT;
        }
        self
    }

    /// Builder: Require prev bar's high AND low are both within my body —
    /// my body fully contains the previous bar's full range (lazy bits 12–13)
    pub const fn with_prev_in_my_body(mut self, engulfs: bool) -> Self {
        self.lazy_mask |= (1u16 << CandleBits::PREV_HIGH_IN_MY_BODY_BIT)
            | (1u16 << CandleBits::PREV_LOW_IN_MY_BODY_BIT);
        if engulfs {
            self.lazy_value |= (1u16 << CandleBits::PREV_HIGH_IN_MY_BODY_BIT)
                | (1u16 << CandleBits::PREV_LOW_IN_MY_BODY_BIT);
        }
        self
    }

    /// Builder: Require prev bar's high is within my body (lazy bit 12)
    pub const fn with_prev_high_in_my_body(mut self, is_in: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::PREV_HIGH_IN_MY_BODY_BIT;
        if is_in {
            self.lazy_value |= 1u16 << CandleBits::PREV_HIGH_IN_MY_BODY_BIT;
        }
        self
    }

    /// Builder: Require prev bar's low is within my body (lazy bit 13)
    pub const fn with_prev_low_in_my_body(mut self, is_in: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::PREV_LOW_IN_MY_BODY_BIT;
        if is_in {
            self.lazy_value |= 1u16 << CandleBits::PREV_LOW_IN_MY_BODY_BIT;
        }
        self
    }

    /// Shorthand: the current bar engulfs the previous bar.
    ///
    /// `kind` must be one of:
    /// - `ENGULF_BODY` (1) — my body strictly spans prev body
    ///   → sets `I_ENGULF_PREV_BODY` (bit 11)
    /// - `ENGULF_LINE` (2) — my body spans prev body **and** wicks
    ///   → sets `PREV_HIGH_IN_MY_BODY + PREV_LOW_IN_MY_BODY` (bits 12–13)
    pub const fn with_engulf_prev(mut self, kind: i8) -> Self {
        self.has_engulf = true;
        if kind >= 2 {
            // LINE: my body contains the entire prev bar line (wicks included)
            self.lazy_mask |= (1u16 << CandleBits::PREV_HIGH_IN_MY_BODY_BIT)
                | (1u16 << CandleBits::PREV_LOW_IN_MY_BODY_BIT);
            self.lazy_value |= (1u16 << CandleBits::PREV_HIGH_IN_MY_BODY_BIT)
                | (1u16 << CandleBits::PREV_LOW_IN_MY_BODY_BIT);
        } else {
            // BODY: my body strictly spans prev body
            self.lazy_mask |= 1u16 << CandleBits::I_ENGULF_PREV_BODY_BIT;
            self.lazy_value |= 1u16 << CandleBits::I_ENGULF_PREV_BODY_BIT;
        }
        self
    }

    /// Shorthand: the current bar sits inside the previous bar.
    ///
    /// `kind` must be one of:
    /// - `ENGULF_BODY` (1) — my body is inside prev body
    ///   → sets `OPEN_IN_PREV_BODY + CLOSE_IN_PREV_BODY` (bits 2 + 4)
    /// - `ENGULF_LINE` (2) — my entire line is inside prev line
    ///   → sets `HIGH_IN_PREV_LINE + LOW_IN_PREV_LINE` (bits 7 + 10)
    pub const fn with_inside_prev(mut self, kind: i8) -> Self {
        self.has_engulf = true;
        if kind >= 2 {
            // LINE: my entire candle (including wicks) sits within prev candle's range
            self.lazy_mask |= (1u16 << CandleBits::HIGH_IN_PREV_LINE_BIT)
                | (1u16 << CandleBits::LOW_IN_PREV_LINE_BIT);
            self.lazy_value |= (1u16 << CandleBits::HIGH_IN_PREV_LINE_BIT)
                | (1u16 << CandleBits::LOW_IN_PREV_LINE_BIT);
        } else {
            // BODY: my body sits within prev body
            self.lazy_mask |= (1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT)
                | (1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT);
            self.lazy_value |= (1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT)
                | (1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT);
        }
        self
    }

    /// Builder: Require lower wick ≥ 2× body height (lazy bit 14)
    pub const fn with_lower_wick_2x(mut self, is_2x: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::LOWER_WICK_LONG_2X_BIT;
        if is_2x {
            self.lazy_value |= 1u16 << CandleBits::LOWER_WICK_LONG_2X_BIT;
        }
        self
    }

    /// Builder: Require upper wick ≥ 2× body height (lazy bit 15)
    pub const fn with_upper_wick_2x(mut self, is_2x: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::UPPER_WICK_LONG_2X_BIT;
        if is_2x {
            self.lazy_value |= 1u16 << CandleBits::UPPER_WICK_LONG_2X_BIT;
        }
        self
    }

    /// Builder: Require this bar's body is (strictly) greater than the previous bar's body (lazy bit 15).
    ///
    /// `is_gt = true`  → body must be strictly larger than prev body
    /// `is_gt = false` → body must be smaller than or equal to prev body
    pub const fn with_body_gt_prev_body(mut self, is_gt: bool) -> Self {
        self.lazy_mask |= 1u16 << CandleBits::BODY_GT_PREV_BODY_BIT;
        if is_gt {
            self.lazy_value |= 1u16 << CandleBits::BODY_GT_PREV_BODY_BIT;
        }
        self
    }

    /// Builder: Mark that this bar's pattern declares engulf_prev or inside_prev.
    /// Signals `ensure_lazy_bits` to call `apply_engulfing` for this bar.
    pub const fn with_has_engulf(mut self) -> Self {
        self.has_engulf = true;
        self
    }

    /// Builder: Mark that this bar's pattern declares body_gap or wick_gap.
    /// Signals `ensure_lazy_bits` to call `apply_gap` for this bar.
    pub const fn with_has_gap(mut self) -> Self {
        self.has_gap = true;
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
    pub lazy_bits_mask: u16,    // Which lazy bits this pattern needs (0 = none)
}

impl<const N: usize> PatternDefinition<N> {
    /// Create a new pattern definition
    pub const fn new(
        pattern: CandlePattern,
        forecast: ForcastType,
        bars: [PatternMask; N],
        check_prev_bar: bool,
        lazy_bits_mask: u16,
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
}

/// Compute all lazy bits required by the pattern's masks that are not yet stamped.
/// Called after mandatory bits pass, before the lazy mask check.
/// Uses `apply_engulfing` / `apply_gap` when the template declared those attributes
/// (signalled by `PatternMask::has_engulf` / `has_gap`), otherwise uses individual
/// `ensure_*` helpers.
#[inline(always)]
pub fn ensure_lazy_bits(
    masks: &[PatternMask],
    bars: &mut [CandleBits],
    ohlc: (&[f64], &[f64], &[f64], &[f64]),
    state: &EmaState,
) {
    let (open, high, low, close) = ohlc;
    let _ = state; // body_height is now mandatory — state no longer needed here
    for i in 0..bars.len() {
        let missing = masks[i].lazy_mask & !bars[i].lazy_computed;
        if missing == 0 {
            continue;
        }

        // apply_engulfing: only when template declared engulf_prev or inside_prev
        if masks[i].has_engulf && i > 0 {
            if bars[i].lazy_computed & (1u16 << CandleBits::I_ENGULF_PREV_BODY_BIT) == 0 {
                bars[i].apply_engulfing(
                    (open[i - 1], high[i - 1], low[i - 1], close[i - 1]),
                    (open[i], high[i], low[i], close[i]),
                );
                continue; // apply_engulfing stamps all position bits 1–13
            }
        }

        // apply_gap: only when template declared body_gap or wick_gap
        if masks[i].has_gap && i > 0 {
            if bars[i].lazy_computed & (1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT) == 0 {
                bars[i].apply_gap(
                    (open[i - 1], high[i - 1], low[i - 1], close[i - 1]),
                    (open[i], high[i], low[i], close[i]),
                );
            }
        }

        // Re-read missing after bulk helpers may have stamped bits
        let missing = masks[i].lazy_mask & !bars[i].lazy_computed;
        if missing == 0 {
            continue;
        }

        let wick_2x_mask = (1u16 << CandleBits::LOWER_WICK_LONG_2X_BIT)
            | (1u16 << CandleBits::UPPER_WICK_LONG_2X_BIT);
        if missing & wick_2x_mask != 0 {
            bars[i].ensure_wick_2x(open[i], close[i], high[i], low[i]);
        }

        // Bit 15: body > previous body
        if i > 0 && missing & (1u16 << CandleBits::BODY_GT_PREV_BODY_BIT) != 0 {
            bars[i].ensure_body_gt_prev_body(open[i], close[i], open[i - 1], close[i - 1]);
        }

        // Individual position bits — only reached if apply_engulfing/gap didn't run
        if i > 0 {
            let needs_open = missing
                & ((1u16 << CandleBits::OPEN_IN_PREV_BODY_BIT)
                    | (1u16 << CandleBits::OPEN_ABOVE_PREV_BODY_MID_BIT))
                != 0;
            let needs_close = missing
                & ((1u16 << CandleBits::CLOSE_IN_PREV_BODY_BIT)
                    | (1u16 << CandleBits::CLOSE_ABOVE_PREV_BODY_MID_BIT))
                != 0;
            match (needs_open, needs_close) {
                (true, true) => {
                    bars[i].ensure_open_close_position(open[i], close[i], open[i - 1], close[i - 1])
                }
                (true, false) => bars[i].ensure_open_position(open[i], open[i - 1], close[i - 1]),
                (false, true) => bars[i].ensure_close_position(close[i], open[i - 1], close[i - 1]),
                (false, false) => {}
            }

            if missing & (1u16 << CandleBits::LOW_IN_PREV_LINE_BIT) != 0 {
                bars[i].ensure_low_in_prev_line(low[i], low[i - 1], high[i - 1]);
            }
            if missing & (1u16 << CandleBits::HIGH_IN_PREV_LINE_BIT) != 0 {
                bars[i].ensure_high_in_prev_line(high[i], low[i - 1], high[i - 1]);
            }
        }
    }
}

/// Emit the hot-path funnel (compulsory bits → lazy compute → full bits → calc)
/// for one bar-count group.
///
/// Trend filtering is guaranteed by the group×trend dispatch — every pattern in
/// the dispatched slice is pre-filtered to the correct trend bucket — so no
/// per-pattern trend check is needed here.
///
/// For patterns with no lazy bits (`lazy_bits_mask == 0`) a single full-bit pass
/// is used, skipping the redundant compulsory-only pre-check.
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
            for pattern_def in $patterns {
                PERF_COUNTERS.record_pattern_checked();
                if pattern_def.lazy_bits_mask != 0 {
                    // Two-pass: reject on compulsory bits first (cheap), then
                    // compute lazy bits and do the full match.
                    if !pattern_def.matches_bars_compulsory_only(window) {
                        continue;
                    }
                    let ohlc_start = $i + 1 - $window_size;
                    let sliced_ohlc = (
                        &$inputs.0[ohlc_start..=$i],
                        &$inputs.1[ohlc_start..=$i],
                        &$inputs.2[ohlc_start..=$i],
                        &$inputs.3[ohlc_start..=$i],
                    );
                    ensure_lazy_bits(&pattern_def.bars, window, sliced_ohlc, $state);
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

        // Group, colour, and fill are all properties of the final (current) bar —
        // constant across all bar-count windows.
        let current_bar = bars.last()?;
        let group = current_bar.candle_group()?;

        // colour: 1=GREEN, 0=RED
        let colour = (current_bar.mandatory & CandleBits::COLOUR_GREEN != 0) as usize;
        // fill: 1=HALLOW, 0=FILL
        let fill = (current_bar.mandatory & CandleBits::FILL_HALLOW != 0) as usize;
        // Pre-combine colour+fill into the low 2 bits of the key offset
        let cf = colour * 2 + fill;

        // key = group*8 + trend*4 + colour*2 + fill

        // 1-bar patterns: window = [prev_bar, bar1]
        if bars.len() >= 2 {
            let is_uptrend = (bars[bars.len() - 2].mandatory & CandleBits::TREND_UP) != 0;
            let key = group * 8 + (is_uptrend as usize) * 4 + cf;
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
            let is_uptrend = (bars[bars.len() - 3].mandatory & CandleBits::TREND_UP) != 0;
            let key = group * 8 + (is_uptrend as usize) * 4 + cf;
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
            let is_uptrend = (bars[bars.len() - 4].mandatory & CandleBits::TREND_UP) != 0;
            let key = group * 8 + (is_uptrend as usize) * 4 + cf;
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
            let is_uptrend = (bars[bars.len() - 5].mandatory & CandleBits::TREND_UP) != 0;
            let key = group * 8 + (is_uptrend as usize) * 4 + cf;
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
            let is_uptrend = (bars[bars.len() - 6].mandatory & CandleBits::TREND_UP) != 0;
            let key = group * 8 + (is_uptrend as usize) * 4 + cf;
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

/// Dispatch table keyed by group × trend × colour × fill.
///   group  0=Basic, 1=Doji, 2=Marubozu, 3=SpinningTop
///   trend  0=DOWN,  1=UP
///   colour 0=RED,   1=GREEN
///   fill   0=FILL,  1=HALLOW
///   key = group*8 + trend*4 + colour*2 + fill  (32 keys total)
#[derive(Debug, Clone)]
pub struct GroupTrendDispatch {
    pub one_bar: [(usize, usize); 32],
    pub two_bar: [(usize, usize); 32],
    pub three_bar: [(usize, usize); 32],
    pub four_bar: [(usize, usize); 32],
    pub five_bar: [(usize, usize); 32],
}

impl GroupTrendDispatch {
    pub const fn new(
        one_bar: [(usize, usize); 32],
        two_bar: [(usize, usize); 32],
        three_bar: [(usize, usize); 32],
        four_bar: [(usize, usize); 32],
        five_bar: [(usize, usize); 32],
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
