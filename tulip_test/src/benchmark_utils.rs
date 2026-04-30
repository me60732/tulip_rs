use crate::benchmark_logger::should_log_to_db;
use criterion::{measurement::WallTime, BenchmarkGroup, Criterion};

/// Sample size for Criterion benchmarks when not logging to database
/// This controls how many iterations Criterion runs for each benchmark
pub const SAMPLE_SIZE: usize = 300000;

/// Create a configured benchmark group for non-database mode
pub fn create_benchmark_group<'a>(
    c: &'a mut Criterion,
    group_name: &str,
) -> BenchmarkGroup<'a, WallTime> {
    let mut group = c.benchmark_group(group_name);
    if !should_log_to_db() {
        group.sample_size(SAMPLE_SIZE);
    }
    group
}

/// Helper function to determine if we should run Criterion benchmarks
/// (i.e., when database logging is disabled)
pub fn should_run_criterion_benchmarks() -> bool {
    !should_log_to_db()
}
