//! Performance statistics system for candlestick pattern matching
//!
//! This module provides a zero-overhead performance tracking system that can be
//! enabled with the `perf-stats` feature flag. When disabled, all instrumentation
//! compiles to nothing, ensuring zero runtime cost.
//!
//! ## Usage
//!
//! Enable the feature in Cargo.toml:
//! ```toml
//! [features]
//! perf-stats = []
//! ```
//!
//! Then compile with: `cargo build --features perf-stats`
//!
//! ## Example
//!
//! ```ignore
//! use tulip_rs::candle_indicators::perf_stats::PERF_COUNTERS;
//!
//! // Run your pattern matching code...
//!
//! // Print statistics
//! let snapshot = PERF_COUNTERS.snapshot();
//! snapshot.print_summary();
//! ```

use std::sync::atomic::{AtomicUsize, Ordering};

/// Global performance counters for pattern matching statistics
///
/// All methods are no-ops when the `perf-stats` feature is disabled,
/// resulting in zero runtime overhead.
pub struct PerfCounters {
    /// Number of times CandleBits::matches() was called
    pub bit_matches_calls: AtomicUsize,
    /// Number of successful CandleBits::matches() calls (returned true)
    pub bit_matches_success: AtomicUsize,

    /// Number of times PatternDefinition::matches_bars() was called
    pub matches_bars_calls: AtomicUsize,
    /// Number of successful matches_bars calls (returned true)
    pub matches_bars_success: AtomicUsize,

    /// Number of times pattern.calc() was called
    pub calc_calls: AtomicUsize,
    /// Number of successful calc() calls (returned true)
    pub calc_success: AtomicUsize,

    /// Total number of patterns checked (iterations in pattern loops)
    pub total_patterns_checked: AtomicUsize,
    /// Number of early exits in matches_bars (returned false before checking all bars)
    pub early_exits: AtomicUsize,
}

impl PerfCounters {
    /// Create a new PerfCounters instance with all counters at zero
    pub const fn new() -> Self {
        Self {
            bit_matches_calls: AtomicUsize::new(0),
            bit_matches_success: AtomicUsize::new(0),
            matches_bars_calls: AtomicUsize::new(0),
            matches_bars_success: AtomicUsize::new(0),
            calc_calls: AtomicUsize::new(0),
            calc_success: AtomicUsize::new(0),
            total_patterns_checked: AtomicUsize::new(0),
            early_exits: AtomicUsize::new(0),
        }
    }

    /// Record a call to CandleBits::matches()
    #[cfg(feature = "perf-stats")]
    #[inline(always)]
    pub fn record_bit_match_call(&self) {
        self.bit_matches_calls.fetch_add(1, Ordering::Relaxed);
    }

    #[cfg(not(feature = "perf-stats"))]
    #[inline(always)]
    pub fn record_bit_match_call(&self) {}

    /// Record a successful CandleBits::matches() (returned true)
    #[cfg(feature = "perf-stats")]
    #[inline(always)]
    pub fn record_bit_match_success(&self) {
        self.bit_matches_success.fetch_add(1, Ordering::Relaxed);
    }

    #[cfg(not(feature = "perf-stats"))]
    #[inline(always)]
    pub fn record_bit_match_success(&self) {}

    /// Record a call to PatternDefinition::matches_bars()
    #[cfg(feature = "perf-stats")]
    #[inline(always)]
    pub fn record_matches_bars_call(&self) {
        self.matches_bars_calls.fetch_add(1, Ordering::Relaxed);
    }

    #[cfg(not(feature = "perf-stats"))]
    #[inline(always)]
    pub fn record_matches_bars_call(&self) {}

    /// Record a successful matches_bars() (returned true)
    #[cfg(feature = "perf-stats")]
    #[inline(always)]
    pub fn record_matches_bars_success(&self) {
        self.matches_bars_success.fetch_add(1, Ordering::Relaxed);
    }

    #[cfg(not(feature = "perf-stats"))]
    #[inline(always)]
    pub fn record_matches_bars_success(&self) {}

    /// Record an early exit in matches_bars() (returned false before checking all bars)
    #[cfg(feature = "perf-stats")]
    #[inline(always)]
    pub fn record_early_exit(&self) {
        self.early_exits.fetch_add(1, Ordering::Relaxed);
    }

    #[cfg(not(feature = "perf-stats"))]
    #[inline(always)]
    pub fn record_early_exit(&self) {}

    /// Record a call to pattern.calc()
    #[cfg(feature = "perf-stats")]
    #[inline(always)]
    pub fn record_calc_call(&self) {
        self.calc_calls.fetch_add(1, Ordering::Relaxed);
    }

    #[cfg(not(feature = "perf-stats"))]
    #[inline(always)]
    pub fn record_calc_call(&self) {}

    /// Record a successful calc() (returned true)
    #[cfg(feature = "perf-stats")]
    #[inline(always)]
    pub fn record_calc_success(&self) {
        self.calc_success.fetch_add(1, Ordering::Relaxed);
    }

    #[cfg(not(feature = "perf-stats"))]
    #[inline(always)]
    pub fn record_calc_success(&self) {}

    /// Record that a pattern was checked (increment pattern loop counter)
    #[cfg(feature = "perf-stats")]
    #[inline(always)]
    pub fn record_pattern_checked(&self) {
        self.total_patterns_checked.fetch_add(1, Ordering::Relaxed);
    }

    #[cfg(not(feature = "perf-stats"))]
    #[inline(always)]
    pub fn record_pattern_checked(&self) {}

    /// Take a snapshot of current counter values
    pub fn snapshot(&self) -> PerfSnapshot {
        PerfSnapshot {
            bit_matches_calls: self.bit_matches_calls.load(Ordering::Relaxed),
            bit_matches_success: self.bit_matches_success.load(Ordering::Relaxed),
            matches_bars_calls: self.matches_bars_calls.load(Ordering::Relaxed),
            matches_bars_success: self.matches_bars_success.load(Ordering::Relaxed),
            calc_calls: self.calc_calls.load(Ordering::Relaxed),
            calc_success: self.calc_success.load(Ordering::Relaxed),
            total_patterns_checked: self.total_patterns_checked.load(Ordering::Relaxed),
            early_exits: self.early_exits.load(Ordering::Relaxed),
        }
    }

    /// Reset all counters to zero
    #[cfg(feature = "perf-stats")]
    pub fn reset(&self) {
        self.bit_matches_calls.store(0, Ordering::Relaxed);
        self.bit_matches_success.store(0, Ordering::Relaxed);
        self.matches_bars_calls.store(0, Ordering::Relaxed);
        self.matches_bars_success.store(0, Ordering::Relaxed);
        self.calc_calls.store(0, Ordering::Relaxed);
        self.calc_success.store(0, Ordering::Relaxed);
        self.total_patterns_checked.store(0, Ordering::Relaxed);
        self.early_exits.store(0, Ordering::Relaxed);
    }

    #[cfg(not(feature = "perf-stats"))]
    pub fn reset(&self) {}
}

/// Global static instance of performance counters
pub static PERF_COUNTERS: PerfCounters = PerfCounters::new();

/// A snapshot of performance counter values at a point in time
#[derive(Debug, Clone, Copy)]
pub struct PerfSnapshot {
    pub bit_matches_calls: usize,
    pub bit_matches_success: usize,
    pub matches_bars_calls: usize,
    pub matches_bars_success: usize,
    pub calc_calls: usize,
    pub calc_success: usize,
    pub total_patterns_checked: usize,
    pub early_exits: usize,
}

impl PerfSnapshot {
    /// Print a detailed summary of the performance statistics with funnel analysis
    pub fn print_summary(&self) {
        println!("\n========================================");
        println!("  Pattern Matching Performance Stats");
        println!("========================================\n");

        println!("Pattern Funnel Analysis:");
        println!("------------------------");
        println!(
            "  Total patterns checked:    {:>10}  (100.0%)",
            self.total_patterns_checked
        );

        if self.total_patterns_checked > 0 {
            let bit_match_pct =
                (self.matches_bars_calls as f64 / self.total_patterns_checked as f64) * 100.0;
            println!(
                "  → Passed bit matching:     {:>10}  ({:>5.1}%)",
                self.matches_bars_calls, bit_match_pct
            );

            if self.matches_bars_calls > 0 {
                let calc_pct = (self.calc_calls as f64 / self.matches_bars_calls as f64) * 100.0;
                println!(
                    "  → → Reached calc():        {:>10}  ({:>5.1}%)",
                    self.calc_calls, calc_pct
                );

                if self.calc_calls > 0 {
                    let success_pct = (self.calc_success as f64 / self.calc_calls as f64) * 100.0;
                    println!(
                        "  → → → Calc succeeded:      {:>10}  ({:>5.1}%)",
                        self.calc_success, success_pct
                    );
                }
            }
        }

        println!("\nDetailed Metrics:");
        println!("-----------------");
        println!("Bit Matching (PatternMask::matches):");
        println!("  Total calls:        {:>10}", self.bit_matches_calls);
        println!("  Successful matches: {:>10}", self.bit_matches_success);
        if self.bit_matches_calls > 0 {
            let success_rate =
                (self.bit_matches_success as f64 / self.bit_matches_calls as f64) * 100.0;
            println!("  Success rate:       {:>9.2}%", success_rate);
        }

        println!("\nBar Matching (PatternDefinition::matches_bars):");
        println!("  Total calls:        {:>10}", self.matches_bars_calls);
        println!("  Successful matches: {:>10}", self.matches_bars_success);
        println!("  Early exits:        {:>10}", self.early_exits);
        if self.matches_bars_calls > 0 {
            let success_rate =
                (self.matches_bars_success as f64 / self.matches_bars_calls as f64) * 100.0;
            let early_exit_rate =
                (self.early_exits as f64 / self.matches_bars_calls as f64) * 100.0;
            println!("  Success rate:       {:>9.2}%", success_rate);
            println!("  Early exit rate:    {:>9.2}%", early_exit_rate);
        }

        println!("\nPattern Calculation (pattern.calc):");
        println!("  Total calls:        {:>10}", self.calc_calls);
        println!("  Successful calcs:   {:>10}", self.calc_success);
        if self.calc_calls > 0 {
            let success_rate = (self.calc_success as f64 / self.calc_calls as f64) * 100.0;
            println!("  Success rate:       {:>9.2}%", success_rate);
        }

        println!("\n========================================\n");
    }

    /// Calculate overall efficiency: what percentage of checked patterns result in a match
    pub fn overall_efficiency(&self) -> f64 {
        if self.total_patterns_checked == 0 {
            0.0
        } else {
            (self.calc_success as f64 / self.total_patterns_checked as f64) * 100.0
        }
    }

    /// Calculate bit matching effectiveness: percentage of bit matches that pass to calc
    pub fn bit_match_effectiveness(&self) -> f64 {
        if self.matches_bars_calls == 0 {
            0.0
        } else {
            (self.calc_calls as f64 / self.matches_bars_calls as f64) * 100.0
        }
    }

    /// Calculate calc success rate
    pub fn calc_success_rate(&self) -> f64 {
        if self.calc_calls == 0 {
            0.0
        } else {
            (self.calc_success as f64 / self.calc_calls as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perf_counters_new() {
        let counters = PerfCounters::new();
        let snapshot = counters.snapshot();
        assert_eq!(snapshot.bit_matches_calls, 0);
        assert_eq!(snapshot.bit_matches_success, 0);
        assert_eq!(snapshot.matches_bars_calls, 0);
        assert_eq!(snapshot.matches_bars_success, 0);
        assert_eq!(snapshot.calc_calls, 0);
        assert_eq!(snapshot.calc_success, 0);
        assert_eq!(snapshot.total_patterns_checked, 0);
        assert_eq!(snapshot.early_exits, 0);
    }

    #[test]
    #[cfg(feature = "perf-stats")]
    fn test_perf_counters_recording() {
        let counters = PerfCounters::new();

        counters.record_bit_match_call();
        counters.record_bit_match_success();
        counters.record_matches_bars_call();
        counters.record_matches_bars_success();
        counters.record_calc_call();
        counters.record_calc_success();
        counters.record_pattern_checked();
        counters.record_early_exit();

        let snapshot = counters.snapshot();
        assert_eq!(snapshot.bit_matches_calls, 1);
        assert_eq!(snapshot.bit_matches_success, 1);
        assert_eq!(snapshot.matches_bars_calls, 1);
        assert_eq!(snapshot.matches_bars_success, 1);
        assert_eq!(snapshot.calc_calls, 1);
        assert_eq!(snapshot.calc_success, 1);
        assert_eq!(snapshot.total_patterns_checked, 1);
        assert_eq!(snapshot.early_exits, 1);
    }

    #[test]
    fn test_snapshot_efficiency_calculations() {
        let snapshot = PerfSnapshot {
            bit_matches_calls: 1000,
            bit_matches_success: 800,
            matches_bars_calls: 500,
            matches_bars_success: 300,
            calc_calls: 200,
            calc_success: 50,
            total_patterns_checked: 1000,
            early_exits: 200,
        };

        assert_eq!(snapshot.overall_efficiency(), 5.0);
        assert_eq!(snapshot.bit_match_effectiveness(), 40.0);
        assert_eq!(snapshot.calc_success_rate(), 25.0);
    }
}
