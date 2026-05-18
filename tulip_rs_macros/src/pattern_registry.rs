//! Declarative macros for auto-generating CandlePattern match arms
//!
//! **Note**: This file contains declarative macros (`macro_rules!`) that cannot be
//! exported from a proc-macro crate. To use these macros:
//!
//! 1. Copy this file to your main crate (e.g., `tulip_rs/src/candle_indicators/`)
//! 2. Include it in your module tree with `#[macro_use] mod pattern_registry;`
//! 3. Or use `#[macro_export]` and import with `use crate::impl_pattern_dispatch;`
//!
//! This module provides macros that simplify the implementation of pattern dispatch
//! logic by auto-generating match arms for the `calc()` and `get_info()` methods.

/// Generate match arms for CandlePattern impl block
///
/// This macro takes a list of pattern variants and their corresponding module paths,
/// and generates both `calc()` and `get_info()` implementations with appropriate
/// match arms.
///
/// # Usage
///
/// ```rust,ignore
/// use tulip_rs::impl_pattern_dispatch;
///
/// impl_pattern_dispatch! {
///     AdvanceBlock => three_bar::advanceblock,
///     Hammer => one_bar::hammer,
///     Doji => one_bar::doji,
///     ShootingStar => one_bar::shooting_star,
/// }
/// ```
///
/// # Expansion
///
/// The macro expands to an `impl CandlePattern` block with two methods:
///
/// ```rust,ignore
/// impl CandlePattern {
///     pub fn calc(
///         &self,
///         open: &[f64],
///         high: &[f64],
///         low: &[f64],
///         close: &[f64],
///         i: usize,
///         state: &EmaState
///     ) -> bool {
///         match self {
///             CandlePattern::AdvanceBlock => three_bar::advanceblock::calc(open, high, low, close, i, state),
///             CandlePattern::Hammer => one_bar::hammer::calc(open, high, low, close, i, state),
///             CandlePattern::Doji => one_bar::doji::calc(open, high, low, close, i, state),
///             CandlePattern::ShootingStar => one_bar::shooting_star::calc(open, high, low, close, i, state),
///             _ => true,
///         }
///     }
///
///     pub fn get_info(&self) -> CandleInfo {
///         match self {
///             CandlePattern::AdvanceBlock => three_bar::advanceblock::info(),
///             CandlePattern::Hammer => one_bar::hammer::info(),
///             CandlePattern::Doji => one_bar::doji::info(),
///             CandlePattern::ShootingStar => one_bar::shooting_star::info(),
///             _ => panic!("Pattern {:?} info not implemented", self),
///         }
///     }
/// }
/// ```
///
/// # Benefits
///
/// - **DRY Principle**: Avoids duplicating the pattern-to-module mapping across methods
/// - **Consistency**: Ensures both `calc()` and `get_info()` use identical mappings
/// - **Maintainability**: Adding a new pattern requires only one line of code
/// - **Type Safety**: Compile-time verification that enum variants and modules exist
/// - **Reduced Errors**: Eliminates the risk of mismatched implementations
///
/// # Pattern Module Requirements
///
/// Each pattern module must provide two functions:
///
/// - `calc(open: &[f64], high: &[f64], low: &[f64], close: &[f64], i: usize, state: &EmaState) -> bool`
/// - `info() -> CandleInfo`
///
/// # Default Behavior
///
/// - Patterns without an implementation will return `true` for `calc()` (treated as always present)
/// - Patterns without an implementation will panic in `get_info()` with a descriptive message
///
/// # Example
///
/// ```rust,ignore
/// // In your candle_patterns.rs or similar file:
/// use crate::candle_indicators::pattern_registry::impl_pattern_dispatch;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// pub enum CandlePattern {
///     AdvanceBlock,
///     Hammer,
///     Doji,
///     // ... more patterns
/// }
///
/// impl_pattern_dispatch! {
///     AdvanceBlock => three_bar::advanceblock,
///     Hammer => one_bar::hammer,
///     Doji => one_bar::doji,
/// }
/// ```
#[macro_export]
macro_rules! impl_pattern_dispatch {
    ($($variant:ident => $module:path),* $(,)?) => {
        impl CandlePattern {
            /// Calculate whether the pattern is present at the given index
            ///
            /// This method dispatches to the appropriate pattern implementation's
            /// `calc()` function based on the enum variant.
            ///
            /// # Arguments
            ///
            /// * `open` - Array of opening prices
            /// * `high` - Array of high prices
            /// * `low` - Array of low prices
            /// * `close` - Array of closing prices
            /// * `i` - Current index to evaluate (must have sufficient lookback)
            /// * `state` - EMA state for trend detection
            ///
            /// # Returns
            ///
            /// `true` if the pattern is detected at index `i`, `false` otherwise.
            /// Unimplemented patterns return `true` by default.
            ///
            /// # Example
            ///
            /// ```rust,ignore
            /// let pattern = CandlePattern::Hammer;
            /// let state = EmaState::new(/* ... */);
            /// if pattern.calc(&open, &high, &low, &close, 10, &state) {
            ///     println!("Hammer pattern detected!");
            /// }
            /// ```
            pub fn calc(&self, open: &[f64], high: &[f64], low: &[f64], close: &[f64], i: usize, state: &EmaState) -> bool {
                match self {
                    $(CandlePattern::$variant => $module::calc(open, high, low, close, i, state),)*
                    // Default for patterns without calc implementation
                    _ => true,
                }
            }

            /// Get metadata information about the pattern
            ///
            /// This method dispatches to the appropriate pattern implementation's
            /// `info()` function based on the enum variant.
            ///
            /// # Returns
            ///
            /// A `CandleInfo` struct containing:
            /// - Pattern name
            /// - Forecast direction (Bullish/Bearish/Neutral)
            /// - Prior trend requirements
            /// - Number of bars needed for the pattern
            ///
            /// # Panics
            ///
            /// Panics with a descriptive message if the pattern's info is not implemented.
            /// This is intentional to catch missing implementations at runtime during testing.
            ///
            /// # Example
            ///
            /// ```rust,ignore
            /// let pattern = CandlePattern::Doji;
            /// let info = pattern.get_info();
            /// println!("Pattern: {}", info.name);
            /// println!("Forecast: {:?}", info.forecast);
            /// println!("Bars needed: {}", info.bar_count);
            /// ```
            pub fn get_info(&self) -> CandleInfo {
                match self {
                    $(CandlePattern::$variant => $module::info(),)*
                    // Default stub for unimplemented patterns
                    _ => panic!("Pattern {:?} info not implemented", self),
                }
            }
        }
    };
}

// Additional helper macros can be added here as needed

/// Generate only the calc() match arms (without the full impl block)
///
/// This is useful if you need more control over the impl block structure
/// or want to add additional methods alongside the generated ones.
///
/// # Usage
///
/// ```rust,ignore
/// impl CandlePattern {
///     pub fn calc(&self, open: &[f64], high: &[f64], low: &[f64], close: &[f64], i: usize, state: &EmaState) -> bool {
///         match self {
///             impl_pattern_calc_arms! {
///                 AdvanceBlock => three_bar::advanceblock,
///                 Hammer => one_bar::hammer,
///             }
///             _ => true,
///         }
///     }
///
///     // Your custom methods here
/// }
/// ```
#[macro_export]
macro_rules! impl_pattern_calc_arms {
    ($($variant:ident => $module:path),* $(,)?) => {
        $(CandlePattern::$variant => $module::calc(open, high, low, close, i, state),)*
    };
}

/// Generate only the get_info() match arms (without the full impl block)
///
/// This is useful if you need more control over the impl block structure
/// or want to add additional methods alongside the generated ones.
///
/// # Usage
///
/// ```rust,ignore
/// impl CandlePattern {
///     pub fn get_info(&self) -> CandleInfo {
///         match self {
///             impl_pattern_info_arms! {
///                 AdvanceBlock => three_bar::advanceblock,
///                 Hammer => one_bar::hammer,
///             }
///             _ => panic!("Pattern {:?} info not implemented", self),
///         }
///     }
///
///     // Your custom methods here
/// }
/// ```
#[macro_export]
macro_rules! impl_pattern_info_arms {
    ($($variant:ident => $module:path),* $(,)?) => {
        $(CandlePattern::$variant => $module::info(),)*
    };
}
