#![allow(dead_code)]
//! Five-bar candlestick patterns
//!
//! This module contains pattern implementations that require five consecutive bars
//! for pattern recognition (e.g., Five White Soldiers, complex reversal patterns, etc.).
//!
//! Each pattern module should use the `#[pattern_template]` attribute macro
//! to automatically register with the global pattern registry.

pub(super) const BAR_COUNT: usize = 5;
pub(super) const PREV_BAR: usize = 0;
pub(super) const FIRST_BAR: usize = 1;
pub(super) const SECOND_BAR: usize = 2;
pub(super) const THIRD_BAR: usize = 3;
pub(super) const FOURTH_BAR: usize = 4;
pub(super) const FIFTH_BAR: usize = 5;
