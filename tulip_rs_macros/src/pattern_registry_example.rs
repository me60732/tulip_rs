//! Example usage of pattern_registry macros
//!
//! This file demonstrates how to use the declarative macros from `pattern_registry.rs`
//! after copying them to your main crate.
//!
//! **Note**: This is an example file and won't compile in this proc-macro crate.
//! Copy `pattern_registry.rs` to your main crate first!

// ============================================================================
// Step 1: Define your CandlePattern enum
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CandlePattern {
    // One-bar patterns
    Doji,
    Hammer,
    HangingMan,
    InvertedHammer,
    ShootingStar,

    // Two-bar patterns
    Engulfing,
    Harami,
    Piercing,
    DarkCloudCover,

    // Three-bar patterns
    AdvanceBlock,
    MorningStar,
    EveningStar,
    ThreeWhiteSoldiers,
    ThreeBlackCrows,

    // Four-bar patterns
    ThreeLineStrike,
    // Add more patterns as needed...
}

// ============================================================================
// Step 2: Import the macro (after copying pattern_registry.rs to main crate)
// ============================================================================

// In your main crate's candle_indicators/mod.rs:
// #[macro_use]
// pub mod pattern_registry;

// Or with macro_export in pattern_registry.rs:
// use crate::impl_pattern_dispatch;

// ============================================================================
// Step 3: Use the macro to generate implementations
// ============================================================================

// Full implementation with both calc() and get_info():
impl_pattern_dispatch! {
    // One-bar patterns
    Doji => one_bar::doji,
    Hammer => one_bar::hammer,
    HangingMan => one_bar::hanging_man,
    InvertedHammer => one_bar::inverted_hammer,
    ShootingStar => one_bar::shooting_star,

    // Two-bar patterns
    Engulfing => two_bar::engulfing,
    Harami => two_bar::harami,
    Piercing => two_bar::piercing,
    DarkCloudCover => two_bar::dark_cloud_cover,

    // Three-bar patterns
    AdvanceBlock => three_bar::advanceblock,
    MorningStar => three_bar::morning_star,
    EveningStar => three_bar::evening_star,
    ThreeWhiteSoldiers => three_bar::three_white_soldiers,
    ThreeBlackCrows => three_bar::three_black_crows,

    // Four-bar patterns
    ThreeLineStrike => four_bar::three_line_strike,
}

// ============================================================================
// Alternative: Use component macros for more control
// ============================================================================

/*
impl CandlePattern {
    /// Calculate pattern presence at given index
    pub fn calc(
        &self,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        i: usize,
        state: &EmaState
    ) -> bool {
        match self {
            impl_pattern_calc_arms! {
                Doji => one_bar::doji,
                Hammer => one_bar::hammer,
                AdvanceBlock => three_bar::advanceblock,
            }
            _ => true,
        }
    }

    /// Get pattern metadata
    pub fn get_info(&self) -> CandleInfo {
        match self {
            impl_pattern_info_arms! {
                Doji => one_bar::doji,
                Hammer => one_bar::hammer,
                AdvanceBlock => three_bar::advanceblock,
            }
            _ => panic!("Pattern {:?} info not implemented", self),
        }
    }

    // Add custom methods alongside generated ones
    pub fn name(&self) -> &'static str {
        self.get_info().name
    }

    pub fn is_bullish(&self) -> bool {
        matches!(self.get_info().forecast, ForcastType::Bullish)
    }

    pub fn is_bearish(&self) -> bool {
        matches!(self.get_info().forecast, ForcastType::Bearish)
    }

    pub fn bar_count(&self) -> usize {
        self.get_info().bar_count
    }
}
*/

// ============================================================================
// Step 4: Implement the pattern modules
// ============================================================================

// Example pattern module structure:
mod one_bar {
    pub mod doji {
        use crate::candle_indicators::common::NO_TREND;
        use crate::candle_indicators::types::{CandleInfo, ForcastType};
        use crate::indicators::ema::State as EmaState;

        pub fn calc(
            open: &[f64],
            high: &[f64],
            low: &[f64],
            close: &[f64],
            i: usize,
            _state: &EmaState,
        ) -> bool {
            if i >= open.len() {
                return false;
            }

            let body = (close[i] - open[i]).abs();
            let range = high[i] - low[i];

            // Doji has very small body compared to range
            range > 0.0 && body / range < 0.1
        }

        pub fn info() -> CandleInfo {
            CandleInfo {
                name: "Doji",
                forecast: ForcastType::Neutral,
                prior_trend: None,
                bar_count: 1,
            }
        }
    }

    pub mod hammer {
        use crate::candle_indicators::common::DOWN_TREND;
        use crate::candle_indicators::types::{CandleInfo, ForcastType};
        use crate::indicators::ema::State as EmaState;

        pub fn calc(
            open: &[f64],
            high: &[f64],
            low: &[f64],
            close: &[f64],
            i: usize,
            state: &EmaState,
        ) -> bool {
            if i >= open.len() {
                return false;
            }

            // Check for downtrend
            if !state.is_downtrend() {
                return false;
            }

            let body = (close[i] - open[i]).abs();
            let lower_shadow = open[i].min(close[i]) - low[i];
            let upper_shadow = high[i] - open[i].max(close[i]);

            // Hammer: long lower shadow, small body, small upper shadow
            lower_shadow > body * 2.0 && upper_shadow < body * 0.3
        }

        pub fn info() -> CandleInfo {
            CandleInfo {
                name: "Hammer",
                forecast: ForcastType::Bullish,
                prior_trend: Some(DOWN_TREND),
                bar_count: 1,
            }
        }
    }
}

mod three_bar {
    pub mod advanceblock {
        use crate::candle_indicators::common::UP_TREND;
        use crate::candle_indicators::types::{CandleInfo, ForcastType};
        use crate::indicators::ema::State as EmaState;

        pub fn calc(
            open: &[f64],
            high: &[f64],
            low: &[f64],
            close: &[f64],
            i: usize,
            state: &EmaState,
        ) -> bool {
            if i < 2 || i >= open.len() {
                return false;
            }

            // Check for uptrend
            if !state.is_uptrend() {
                return false;
            }

            // Three white candles with diminishing bodies
            let body1 = close[i - 2] - open[i - 2];
            let body2 = close[i - 1] - open[i - 1];
            let body3 = close[i] - open[i];

            body1 > 0.0 && body2 > 0.0 && body3 > 0.0 && body2 < body1 && body3 < body2
        }

        pub fn info() -> CandleInfo {
            CandleInfo {
                name: "Advance Block",
                forecast: ForcastType::Bearish,
                prior_trend: Some(UP_TREND),
                bar_count: 3,
            }
        }
    }
}

// ============================================================================
// Step 5: Use the patterns
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_dispatch() {
        let open = vec![10.0, 11.0, 12.0, 13.0];
        let high = vec![11.0, 12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0, 12.0];
        let close = vec![10.5, 11.5, 12.5, 13.5];
        let state = EmaState::new(/* ... */);

        // Test calc dispatch
        let doji = CandlePattern::Doji;
        let result = doji.calc(&open, &high, &low, &close, 2, &state);
        println!("Doji detected: {}", result);

        // Test info dispatch
        let info = doji.get_info();
        assert_eq!(info.name, "Doji");
        assert_eq!(info.bar_count, 1);
    }

    #[test]
    fn test_all_patterns_info() {
        // Verify all patterns have info implementations
        let patterns = vec![
            CandlePattern::Doji,
            CandlePattern::Hammer,
            CandlePattern::AdvanceBlock,
            // ... add all your patterns
        ];

        for pattern in patterns {
            let info = pattern.get_info();
            println!(
                "{}: {:?}, bars: {}",
                info.name, info.forecast, info.bar_count
            );
        }
    }
}

// ============================================================================
// Benefits of using impl_pattern_dispatch!
// ============================================================================

// ✅ DRY: Single source of truth for pattern-to-module mapping
// ✅ Consistency: calc() and get_info() always stay in sync
// ✅ Maintainability: Adding a new pattern = one line of code
// ✅ Type Safety: Compile-time verification of variants and modules
// ✅ Less Error-Prone: No risk of mismatched implementations
// ✅ Clean Code: Eliminates repetitive boilerplate

// ============================================================================
// Migration Guide
// ============================================================================

// Before (manual implementation):
/*
impl CandlePattern {
    pub fn calc(...) -> bool {
        match self {
            CandlePattern::Doji => one_bar::doji::calc(...),
            CandlePattern::Hammer => one_bar::hammer::calc(...),
            // ... 50+ more patterns
        }
    }

    pub fn get_info(&self) -> CandleInfo {
        match self {
            CandlePattern::Doji => one_bar::doji::info(),
            CandlePattern::Hammer => one_bar::hammer::info(),
            // ... 50+ more patterns (must match above!)
        }
    }
}
*/

// After (with macro):
/*
impl_pattern_dispatch! {
    Doji => one_bar::doji,
    Hammer => one_bar::hammer,
    // ... 50+ more patterns (single source!)
}
*/
