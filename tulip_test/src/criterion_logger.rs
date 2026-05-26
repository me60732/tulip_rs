use std::time::{Duration, Instant};
use crate::benchmark_logger::BenchmarkResult;

pub struct TimingMeasurements {
    pub times: Vec<Duration>,
}

impl TimingMeasurements {
    pub fn new() -> Self {
        Self {
            times: Vec::new(),
        }
    }
    
    pub fn measure<F>(&mut self, mut f: F, samples: usize)
    where
        F: FnMut(),
    {
        self.times.clear();
        self.times.reserve(samples);
        
        // Warm up
        for _ in 0..10 {
            f();
        }
        
        // Take measurements
        for _ in 0..samples {
            let start = Instant::now();
            f();
            let elapsed = start.elapsed();
            self.times.push(elapsed);
        }
    }
    
    pub fn to_benchmark_result(
        &self,
        indicator_name: &str,
        implementation_type: &str,
        stock_symbol: Option<&str>,
        data_source: &str,
        options: &[f64],
        input_size: usize,
    ) -> Option<BenchmarkResult> {
        if let Some(stats) = calculate_stats(&self.times) {
            Some(BenchmarkResult {
                indicator_name: indicator_name.to_string(),
                implementation_type: implementation_type.to_string(),
                stock_symbol: stock_symbol.map(|s| s.to_string()),
                data_source: data_source.to_string(),
                options: options.to_vec(),
                mean_time_ns: stats.mean_ns,
                std_dev_ns: stats.std_dev_ns,
                min_time_ns: stats.min_ns,
                max_time_ns: stats.max_ns,
                sample_count: self.times.len() as u32,
                input_size,
            })
        } else {
            None
        }
    }
}

struct TimingStats {
    mean_ns: u64,
    std_dev_ns: u64,
    min_ns: u64,
    max_ns: u64,
}

fn calculate_stats(measurements: &[Duration]) -> Option<TimingStats> {
    if measurements.is_empty() {
        return None;
    }
    
    let times_ns: Vec<u64> = measurements.iter().map(|d| d.as_nanos() as u64).collect();
    
    let sum: u64 = times_ns.iter().sum();
    let mean_ns = sum / times_ns.len() as u64;
    
    let variance = times_ns.iter()
        .map(|&time| {
            let diff = time as i64 - mean_ns as i64;
            (diff * diff) as u64
        })
        .sum::<u64>() / times_ns.len() as u64;
    
    let std_dev_ns = (variance as f64).sqrt() as u64;
    let min_ns = *times_ns.iter().min().unwrap();
    let max_ns = *times_ns.iter().max().unwrap();
    
    Some(TimingStats {
        mean_ns,
        std_dev_ns,
        min_ns,
        max_ns,
    })
}
