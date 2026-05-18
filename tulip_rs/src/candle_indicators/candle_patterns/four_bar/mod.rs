//! Four-bar candlestick patterns
//!
//! This module contains pattern implementations that require four consecutive bars
//! for pattern recognition (e.g., Three Inside Up/Down with confirmation, etc.).
//!
//! Each pattern module should use the `#[pattern_template]` attribute macro
//! to automatically register with the global pattern registry.
#[allow(dead_code)]
pub(super) const BAR_COUNT: usize = 4;
#[allow(dead_code)]
pub(super) const PREV: usize = 0;
pub(super) const FIRST: usize = 1;
pub(super) const SECOND: usize = 2;
pub(super) const THIRD: usize = 3;
pub(super) const FOURTH: usize = 4;

pub mod bearishthreelinestrike;
pub mod bullishthreelinestrike;
pub mod concealingbabyswallow;
