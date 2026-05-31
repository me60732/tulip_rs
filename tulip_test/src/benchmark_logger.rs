use crate::criterion_logger::TimingMeasurements;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

// Global state for centralized logging
static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static LOGGER: OnceLock<Option<BenchmarkLogger>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub indicator_name: String,
    pub implementation_type: String, // "C", "Rust", "Rust_FromState"
    pub stock_symbol: Option<String>,
    pub data_source: String, // "synthetic" or "database"
    pub options: Vec<f64>,
    pub mean_time_ns: u64,
    pub std_dev_ns: u64,
    pub min_time_ns: u64,
    pub max_time_ns: u64,
    pub sample_count: u32,
    pub input_size: usize,
}

pub struct BenchmarkLogger {
    pool: PgPool,
    current_run_id: Option<i32>,
    indicator_cache: HashMap<String, i32>, // Cache indicator name -> id mapping
}

impl BenchmarkLogger {
    pub async fn new() -> Result<Self, sqlx::Error> {
        let database_url =
            std::env::var("BENCHMARK_DATABASE_URL").expect("BENCHMARK_DATABASE_URL must be set");
        let pool = PgPool::connect(&database_url).await?;

        // Load indicator cache
        let mut indicator_cache = HashMap::new();
        let rows = sqlx::query("SELECT id, name FROM indicators")
            .fetch_all(&pool)
            .await?;

        for row in rows {
            let id: i32 = row.get("id");
            let name: String = row.get("name");
            indicator_cache.insert(name, id);
        }

        println!("Loaded {} indicators into cache", indicator_cache.len());

        Ok(Self {
            pool,
            current_run_id: None,
            indicator_cache,
        })
    }

    pub async fn start_benchmark_run(&mut self, notes: Option<&str>) -> Result<i32, sqlx::Error> {
        let rust_version = get_rust_version();
        let system_info = get_system_info().await;

        let row = sqlx::query(
            "INSERT INTO benchmark_runs (rust_version, system_info, notes)
             VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(rust_version)
        .bind(serde_json::to_value(system_info).unwrap())
        .bind(notes)
        .fetch_one(&self.pool)
        .await?;

        let run_id = row.get::<i32, _>("id");
        self.current_run_id = Some(run_id);
        println!("Started benchmark run with ID: {}", run_id);
        Ok(run_id)
    }

    pub async fn log_benchmark_result(&self, result: BenchmarkResult) -> Result<(), sqlx::Error> {
        let run_id = self.current_run_id.expect("No active benchmark run");

        // Get indicator ID from cache
        let indicator_id = self
            .indicator_cache
            .get(&result.indicator_name)
            .copied()
            .unwrap_or_else(|| {
                eprintln!(
                    "Warning: Indicator '{}' not found in cache",
                    result.indicator_name
                );
                -1
            });

        if indicator_id == -1 {
            return Ok(()); // Skip unknown indicators
        }

        // Serialize options as JSON
        let options_json =
            serde_json::to_string(&result.options).map_err(|e| sqlx::Error::TypeNotFound {
                type_name: format!("JSON serialization error: {}", e),
            })?;

        sqlx::query(
            "INSERT INTO benchmark_results
             (run_id, indicator_id, implementation_type, stock_symbol, data_source,
              options, mean_time_ns, std_dev_ns, min_time_ns, max_time_ns,
              sample_count, input_size)
             VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $8, $9, $10, $11, $12)",
        )
        .bind(run_id)
        .bind(indicator_id)
        .bind(&result.implementation_type)
        .bind(&result.stock_symbol)
        .bind(&result.data_source)
        .bind(&options_json)
        .bind(result.mean_time_ns as i64)
        .bind(result.std_dev_ns as i64)
        .bind(result.min_time_ns as i64)
        .bind(result.max_time_ns as i64)
        .bind(result.sample_count as i32)
        .bind(result.input_size as i32)
        .execute(&self.pool)
        .await?;

        println!(
            "Logged: {} {} {:?} - {}ns",
            result.indicator_name, result.implementation_type, result.options, result.mean_time_ns
        );

        Ok(())
    }

    pub async fn calculate_performance_ratios(&self) -> Result<(), sqlx::Error> {
        let run_id = self.current_run_id.expect("No active benchmark run");

        // Calculate performance ratios for Rust implementation vs C
        let rust_rows = sqlx::query(
            "UPDATE benchmark_results
             SET performance_ratio = rust.mean_time_ns::float / c.mean_time_ns::float
             FROM benchmark_results rust
             JOIN benchmark_results c ON (
                 rust.run_id = c.run_id AND
                 rust.indicator_id = c.indicator_id AND
                 rust.stock_symbol = c.stock_symbol AND
                 rust.options = c.options AND
                 rust.implementation_type = 'Rust' AND
                 c.implementation_type = 'C_tulip'
             )
             WHERE benchmark_results.run_id = $1
             AND benchmark_results.implementation_type = 'Rust'",
        )
        .bind(run_id)
        .execute(&self.pool)
        .await?;

        // Calculate performance ratios for Rust_FromState implementation vs C
        let rust_state_rows = sqlx::query(
            "UPDATE benchmark_results
             SET performance_ratio = rust_state.mean_time_ns::float / c.mean_time_ns::float
             FROM benchmark_results rust_state
             JOIN benchmark_results c ON (
                 rust_state.run_id = c.run_id AND
                 rust_state.indicator_id = c.indicator_id AND
                 rust_state.stock_symbol = c.stock_symbol AND
                 rust_state.options = c.options AND
                 rust_state.implementation_type = 'Rust_FromState' AND
                 c.implementation_type = 'C_tulip'
             )
             WHERE benchmark_results.run_id = $1
             AND benchmark_results.implementation_type = 'Rust_FromState'",
        )
        .bind(run_id)
        .execute(&self.pool)
        .await?;

        println!(
            "Calculated performance ratios for {} Rust and {} Rust_FromState results",
            rust_rows.rows_affected(),
            rust_state_rows.rows_affected()
        );

        Ok(())
    }

    pub fn get_indicator_id(&self, name: &str) -> Option<i32> {
        self.indicator_cache.get(name).copied()
    }
}

// Helper functions
fn get_rust_version() -> String {
    std::process::Command::new("rustc")
        .args(["--version"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string()
}

async fn get_system_info() -> serde_json::Value {
    serde_json::json!({
        "cpu_cores": num_cpus::get(),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "hostname": gethostname::gethostname().to_string_lossy().to_string()
    })
}

// Convenience function to create a global logger instance
static mut GLOBAL_LOGGER: Option<BenchmarkLogger> = None;
static INIT: std::sync::Once = std::sync::Once::new();

pub async fn get_global_logger() -> &'static mut BenchmarkLogger {
    unsafe {
        INIT.call_once(|| {
            // This will be set by init_global_logger
        });
        #[allow(static_mut_refs)]
        GLOBAL_LOGGER
            .as_mut()
            .expect("Global logger not initialized")
    }
}

pub async fn init_global_logger(_database_url: &str) -> Result<(), sqlx::Error> {
    let logger = BenchmarkLogger::new().await?;
    unsafe {
        GLOBAL_LOGGER = Some(logger);
    }
    Ok(())
}

// ===== CENTRALIZED LOGGING INITIALIZATION =====

/// Ensure the .env file is loaded exactly once, regardless of CWD.
///
/// `dotenv::dotenv()` walks *up* from CWD and never reaches `tulip_test/.env`
/// when `cargo bench` is invoked from the workspace root.
/// Cargo sets `CARGO_MANIFEST_DIR` at runtime for bench/test executables to
/// the package root (`tulip_test/`), giving us a stable anchor.
static DOTENV_LOADED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

fn load_dotenv() {
    DOTENV_LOADED.get_or_init(|| {
        // 1. Explicit package-root path via CARGO_MANIFEST_DIR (most reliable).
        if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let path = std::path::Path::new(&dir).join(".env");
            if dotenv::from_path(&path).is_ok() {
                return;
            }
        }
        // 2. Fall back to dotenv's default upward search from CWD.
        dotenv::dotenv().ok();
    });
}

/// Returns true if DB logging is requested.
///
/// Loads `.env` on the first call so that `BENCHMARK_LOG_TO_DB=1` set inside
/// the file is visible before any bench function checks this flag.
pub fn should_log_to_db() -> bool {
    load_dotenv();
    std::env::var("BENCHMARK_LOG_TO_DB").as_deref() == Ok("1")
}

/// Initialize logging infrastructure (thread-safe, can be called multiple times).
/// Only call this when `should_log_to_db()` is true.
pub fn init_logging(benchmark_name: &str) {
    // .env is already loaded by should_log_to_db() before this is called,
    // but call load_dotenv() here too so init_logging remains safe to call
    // standalone (e.g. from tests).
    load_dotenv();

    // Only create runtime if not already set
    if RUNTIME.get().is_none() {
        let rt = Runtime::new().unwrap();
        if let Err(_) = RUNTIME.set(rt) {
            // Already set by another thread, this is fine
        }
    }

    // Only create logger if not already set
    if LOGGER.get().is_none() {
        let runtime = RUNTIME.get().unwrap();
        let logger = runtime.block_on(async {
            match BenchmarkLogger::new().await {
                Ok(mut logger) => {
                    if let Err(e) = logger
                        .start_benchmark_run(Some(&format!(
                            "Criterion {} Benchmarks",
                            benchmark_name
                        )))
                        .await
                    {
                        eprintln!("Failed to start benchmark run: {}", e);
                        return None;
                    }
                    Some(logger)
                }
                Err(e) => {
                    eprintln!("Failed to create benchmark logger: {}", e);
                    None
                }
            }
        });

        if let Err(_) = LOGGER.set(logger) {
            // Already set by another thread, this is fine
        }
    }
}

/// Log timing results if logging is enabled
pub fn log_timing_result(
    indicator: &str,
    implementation: &str,
    options: &[f64],
    data_size: usize,
    timing: &TimingMeasurements,
    stock_symbol: Option<&str>,
) {
    if let Some(Some(logger)) = LOGGER.get() {
        if let Some(result) = timing.to_benchmark_result(
            indicator,
            implementation,
            stock_symbol,
            "real_data",
            options,
            data_size,
        ) {
            if let Some(runtime) = RUNTIME.get() {
                runtime.block_on(async {
                    if let Err(e) = logger.log_benchmark_result(result).await {
                        eprintln!("Failed to log benchmark result: {}", e);
                    }
                });
            }
        }
    }
}

/// Finalize logging after all benchmarks
pub fn finalize_logging() {
    if let Some(Some(logger)) = LOGGER.get() {
        if let Some(runtime) = RUNTIME.get() {
            runtime.block_on(async {
                if let Err(e) = logger.calculate_performance_ratios().await {
                    eprintln!("Failed to calculate performance ratios: {}", e);
                } else {
                    println!("Benchmark results logged to database!");
                }
            });
        }
    }
}
