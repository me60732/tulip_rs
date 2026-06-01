use bigdecimal::BigDecimal;
use sqlx::{PgPool, Row};
use std::env;
const LIMIT: i64 = 6705; //7500;
#[derive(Debug, Clone)]
pub struct EodData {
    pub code: String,
    pub ts: chrono::NaiveDate,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

pub async fn get_database_pool() -> Result<PgPool, sqlx::Error> {
    dotenv::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    PgPool::connect(&database_url).await
}

pub async fn fetch_eod_data(
    pool: &PgPool,
    code: &str,
    exchange: &str,
    limit: i64,
) -> Result<Vec<EodData>, sqlx::Error> {
    let query = r#"
        SELECT
          l.code,
          e.ts,
          e.open,
          e.high,
          e.low,
          e.close,
          e.volume
        FROM listing l
        INNER JOIN adj_eod e ON l.listing_id = e.listing_id
        WHERE l.code = $1
          AND l.exchange_code = $2
          AND e.volume > 0
        ORDER BY e.ts DESC
        LIMIT $3
    "#;

    let rows = sqlx::query(query)
        .bind(code)
        .bind(exchange)
        .bind(limit)
        .fetch_all(pool)
        .await?;

    let mut data = Vec::new();
    for row in rows {
        let open_bd: BigDecimal = row.get("open");
        let high_bd: BigDecimal = row.get("high");
        let low_bd: BigDecimal = row.get("low");
        let close_bd: BigDecimal = row.get("close");
        let volume_bd: BigDecimal = row.get("volume");

        data.push(EodData {
            code: row.get("code"),
            ts: row.get("ts"),
            open: open_bd.to_string().parse().unwrap_or(0.0),
            high: high_bd.to_string().parse().unwrap_or(0.0),
            low: low_bd.to_string().parse().unwrap_or(0.0),
            close: close_bd.to_string().parse().unwrap_or(0.0),
            volume: volume_bd.to_string().parse().unwrap_or(0.0),
        });
    }

    // Reverse the data so it's in chronological order (oldest to newest) for indicators
    data.reverse();

    Ok(data)
}

// Function to get multiple stocks data
pub async fn fetch_multiple_stocks_data(
) -> Result<Vec<(String, Vec<EodData>)>, Box<dyn std::error::Error>> {
    let pool = get_database_pool().await?;

    let stocks = vec![
        ("BHP", "ASX"),
        ("CBA", "ASX"),
        ("AAPL", "NYSE"),
        ("MSFT", "NYSE"),
        /*("MYR", "ASX"),
        ("NAB", "ASX"),
        ("NVDA", "NYSE"),
        ("BA", "NYSE"),*/
    ];

    let mut all_data = Vec::new();

    for (code, exchange) in stocks {
        match fetch_eod_data(&pool, code, exchange, LIMIT).await {
            Ok(data) => {
                if !data.is_empty() {
                    let record_count = data.len();
                    all_data.push((format!("{}_{}", code, exchange), data));
                    println!(
                        "Fetched {} records for {} on {}",
                        record_count, code, exchange
                    );
                } else {
                    println!("No data found for {} on {}", code, exchange);
                }
            }
            Err(e) => {
                println!("Error fetching data for {} on {}: {}", code, exchange, e);
            }
        }
    }

    Ok(all_data)
}

// Convert EodData to the format expected by indicators
pub fn eod_data_to_arrays(data: &[EodData]) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = data.iter().map(|d| d.low).collect();
    let close: Vec<f64> = data.iter().map(|d| d.close).collect();
    let volume: Vec<f64> = data.iter().map(|d| d.volume).collect();

    (high, low, close, volume)
}

// Convert EodData to arrays with open data included
pub fn eod_data_to_arrays_with_open(
    data: &[EodData],
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let open: Vec<f64> = data.iter().map(|d| d.open).collect();
    let high: Vec<f64> = data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = data.iter().map(|d| d.low).collect();
    let close: Vec<f64> = data.iter().map(|d| d.close).collect();
    let volume: Vec<f64> = data.iter().map(|d| d.volume).collect();

    (open, high, low, close, volume)
}

use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

// Global storage for database data - using OnceLock for thread safety
static STOCK_DATA: OnceLock<Vec<(String, Vec<EodData>)>> = OnceLock::new();

/// Initialize database data once.
///
/// When `BENCHMARK_LOG_TO_DB` is not `"1"` (e.g. in CI), the real database is
/// skipped and synthetic data is returned instead — the standard 15-point test
/// arrays repeated 100 times (1 500 rows) under the symbol `"SYNTHETIC"`.
/// This keeps all `_database` tests running and exercising real code paths
/// without needing a live PostgreSQL connection.
pub fn init_database_data() {
    STOCK_DATA.get_or_init(|| {
        if !crate::benchmark_logger::should_log_to_db() {
            // Build synthetic EodData from the canonical hardcoded test arrays.
            const OPEN: [f64; 15] = [
                81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45,
                86.18, 88.00, 87.60,
            ];
            const HIGH: [f64; 15] = [
                82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58,
                86.98, 88.00, 87.87,
            ];
            const LOW: [f64; 15] = [
                81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39,
                85.76, 87.17, 87.01,
            ];
            const CLOSE: [f64; 15] = [
                81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54,
                86.89, 87.77, 87.29,
            ];
            const VOLUME: [f64; 15] = [
                5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0,
                4732000.0, 4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0,
                4807900.0,
            ];

            let base_date = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();

            // Mirror the 4 symbols the real DB query returns so that tests
            // which index stock_data[0..3] always find all 4 entries.
            let symbols = ["BHP_ASX", "CBA_ASX", "AAPL_NYSE", "MSFT_NYSE"];
            let mut all_stocks: Vec<(String, Vec<EodData>)> = Vec::with_capacity(4);

            for symbol in symbols {
                let mut eod_data: Vec<EodData> = Vec::with_capacity(15 * 100);
                for rep in 0..100_i64 {
                    for i in 0..15_usize {
                        eod_data.push(EodData {
                            code: symbol.to_string(),
                            ts: base_date + chrono::Duration::days(rep * 15 + i as i64),
                            open: OPEN[i],
                            high: HIGH[i],
                            low: LOW[i],
                            close: CLOSE[i],
                            volume: VOLUME[i],
                        });
                    }
                }
                all_stocks.push((symbol.to_string(), eod_data));
            }

            println!(
                "Using synthetic data: {} stocks, {} data points each",
                all_stocks.len(),
                15 * 100
            );
            return all_stocks;
        }

        let rt = Runtime::new().expect("Failed to create Tokio runtime");
        match rt.block_on(fetch_multiple_stocks_data()) {
            Ok(data) => {
                if data.is_empty() {
                    panic!("No stock data found in database!");
                }

                println!("Loaded {} stocks for benchmarking:", data.len());
                for (name, eod_data) in &data {
                    println!("  {}: {} data points", name, eod_data.len());
                }

                data
            }
            Err(e) => {
                panic!("Failed to fetch stock data: {}", e);
            }
        }
    });
}

/// Get input arrays for testing - uses database data if available, falls back to test data
/// requested_inputs: &["open", "high", "low", "close", "volume"] (specify which arrays you need)
/// Returns a HashMap with the requested arrays
pub fn get_input_arrays(requested_inputs: &[&str]) -> HashMap<String, Vec<f64>> {
    let mut result = HashMap::new();

    // Try to get data from database first
    if let Some(stock_data) = STOCK_DATA.get() {
        if let Some((name, eod_data)) = stock_data.first() {
            println!(
                "Using database data for {}: {} data points",
                name,
                eod_data.len()
            );

            for &input in requested_inputs {
                let array = match input {
                    "open" => eod_data.iter().map(|d| d.open).collect(),
                    "high" => eod_data.iter().map(|d| d.high).collect(),
                    "low" => eod_data.iter().map(|d| d.low).collect(),
                    "close" => eod_data.iter().map(|d| d.close).collect(),
                    "volume" => eod_data.iter().map(|d| d.volume).collect(),
                    _ => {
                        println!("Warning: Unknown input '{}' requested", input);
                        continue;
                    }
                };
                result.insert(input.to_string(), array);
            }
            return result;
        }
    }

    // Fallback to original test data if database data is not available
    println!("Using fallback test data (15 data points)");

    // Original test data
    let open_data = vec![
        81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18,
        88.00, 87.60,
    ];
    let high_data = vec![
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ];
    let low_data = vec![
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ];
    let close_data = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];
    let volume_data = vec![
        5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
        4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
    ];

    for &input in requested_inputs {
        let array = match input {
            "open" => open_data.clone(),
            "high" => high_data.clone(),
            "low" => low_data.clone(),
            "close" => close_data.clone(),
            "volume" => volume_data.clone(),
            _ => {
                println!("Warning: Unknown input '{}' requested", input);
                continue;
            }
        };
        result.insert(input.to_string(), array);
    }

    result
}

/// Get all stock data for multi-stock benchmarking
pub fn get_all_stock_data() -> Option<&'static Vec<(String, Vec<EodData>)>> {
    STOCK_DATA.get()
}

/*/// A streaming wrapper that provides both stack-allocated batches and heap-allocated remainder.
pub struct BatchingStream<const BATCH_SIZE: usize, const COLS: usize> {
    pool: sqlx::PgPool,
    code: String,
    exchange: String,
    limit: i64,
    columns: [String; COLS],
    consumed: bool,
}

impl<const BATCH_SIZE: usize, const COLS: usize> BatchingStream<BATCH_SIZE, COLS> {
    /// Extract full batches as stack-allocated arrays.
    pub fn stream_extract(self) -> (impl Stream<Item = [[f64; BATCH_SIZE]; COLS]>, Self) {
        use futures::StreamExt;

        let mut self_for_remainder = BatchingStream {
            pool: self.pool.clone(),
            code: self.code.clone(),
            exchange: self.exchange.clone(),
            limit: self.limit,
            columns: self.columns.clone(),
            consumed: false,
        };

        let stream = async_stream::stream! {
            // Build dynamic query based on requested columns
            let column_list = self.columns.join(", e.");
            let query = format!(
                r#"
                SELECT
                  e.{}
                FROM listing l
                INNER JOIN adj_eod e ON l.listing_id = e.listing_id
                WHERE l.code = $1
                  AND l.exchange_code = $2
                  AND e.volume > 0
                ORDER BY e.ts ASC
                LIMIT $3
                "#,
                column_list
            );

            let mut db_stream = sqlx::query(&query)
                .bind(&self.code)
                .bind(&self.exchange)
                .bind(self.limit)
                .fetch(&self.pool);

            let mut buffers = [[0.0; BATCH_SIZE]; COLS];
            let mut buffer_pos = 0;

            while let Some(row_result) = db_stream.next().await {
                match row_result {
                    Ok(row) => {
                        for (col_idx, column_name) in self.columns.iter().enumerate() {
                            let value_bd: BigDecimal = row.get(column_name.as_str());
                            let value = value_bd.to_string().parse().unwrap_or(0.0);
                            buffers[col_idx][buffer_pos] = value;
                        }
                        buffer_pos += 1;

                        // Yield full batches only
                        if buffer_pos == BATCH_SIZE {
                            yield buffers;
                            buffer_pos = 0;
                        }
                    }
                    Err(e) => {
                        println!("Database error: {}", e);
                        break;
                    }
                }
            }

            // Mark that we've consumed the stream
            self_for_remainder.consumed = true;
        };

        (stream, self_for_remainder)
    }

    /// Get any remaining data that didn't fill a complete batch.
    pub async fn stream_remainder(self) -> Result<Vec<Vec<f64>>, sqlx::Error> {
        use futures::StreamExt;

        if !self.consumed {
            // If stream_extract wasn't called, we need to consume it first
            // This is a fallback - ideally stream_extract should always be called first
        }

        // Build dynamic query based on requested columns
        let column_list = self.columns.join(", e.");
        let query = format!(
            r#"
            SELECT
              e.{}
            FROM listing l
            INNER JOIN adj_eod e ON l.listing_id = e.listing_id
            WHERE l.code = $1
              AND l.exchange_code = $2
              AND e.volume > 0
            ORDER BY e.ts ASC
            LIMIT $3
            "#,
            column_list
        );

        let mut db_stream = sqlx::query(&query)
            .bind(&self.code)
            .bind(&self.exchange)
            .bind(self.limit)
            .fetch(&self.pool);

        let mut all_data: Vec<Vec<f64>> = vec![Vec::new(); COLS];
        let mut row_count = 0;

        while let Some(row_result) = db_stream.next().await {
            match row_result {
                Ok(row) => {
                    for (col_idx, column_name) in self.columns.iter().enumerate() {
                        let value_bd: BigDecimal = row.get(column_name.as_str());
                        let value = value_bd.to_string().parse().unwrap_or(0.0);
                        all_data[col_idx].push(value);
                    }
                    row_count += 1;
                }
                Err(e) => {
                    println!("Database error: {}", e);
                    return Err(e);
                }
            }
        }

        // Calculate remainder by skipping full batches
        let full_batches = row_count / BATCH_SIZE;
        let remainder_start = full_batches * BATCH_SIZE;

        if remainder_start < row_count {
            // Extract remainder
            let mut remainder: Vec<Vec<f64>> = vec![Vec::new(); COLS];
            for col_idx in 0..COLS {
                remainder[col_idx] = all_data[col_idx][remainder_start..].to_vec();
            }
            Ok(remainder)
        } else {
            // No remainder
            Ok(vec![Vec::new(); COLS])
        }
    }
}

/// Create a BatchingStream from SQLx database query with flexible column selection.
///
/// This function returns a BatchingStream that provides both stack-allocated batches
/// and heap-allocated remainder processing.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `code` - Stock/instrument code (e.g., "AAPL")
/// * `exchange` - Exchange code (e.g., "NYSE", "ASX")
/// * `limit` - Maximum number of rows to fetch
/// * `columns` - Array of column names to fetch (e.g., ["high", "low", "close"])
///
/// # Returns
/// A BatchingStream that can yield `[[f64; BATCH_SIZE]; COLS]` for full batches and `Vec<Vec<f64>>` for remainder.
pub fn create_sqlx_batching_stream<const BATCH_SIZE: usize, const COLS: usize>(
    pool: sqlx::PgPool,
    code: String,
    exchange: String,
    limit: i64,
    columns: [String; COLS],
) -> BatchingStream<BATCH_SIZE, COLS> {
    BatchingStream {
        pool,
        code,
        exchange,
        limit,
        columns,
        consumed: false,
    }
}

/// Helper function to create HLC batching stream.
pub fn create_hlc_stream<const BATCH_SIZE: usize>(
    pool: sqlx::PgPool,
    code: &str,
    exchange: &str,
    limit: i64,
) -> BatchingStream<BATCH_SIZE, 3> {
    create_sqlx_batching_stream::<BATCH_SIZE, 3>(
        pool,
        code.to_string(),
        exchange.to_string(),
        limit,
        ["high".to_string(), "low".to_string(), "close".to_string()],
    )
}

/// Helper function to create close-only batching stream.
pub fn create_close_stream<const BATCH_SIZE: usize>(
    pool: sqlx::PgPool,
    code: &str,
    exchange: &str,
    limit: i64,
) -> BatchingStream<BATCH_SIZE, 1> {
    create_sqlx_batching_stream::<BATCH_SIZE, 1>(
        pool,
        code.to_string(),
        exchange.to_string(),
        limit,
        ["close".to_string()],
    )
}

/// Helper function to create OHLCV batching stream.
pub fn create_ohlcv_stream<const BATCH_SIZE: usize>(
    pool: sqlx::PgPool,
    code: &str,
    exchange: &str,
    limit: i64,
) -> BatchingStream<BATCH_SIZE, 5> {
    create_sqlx_batching_stream::<BATCH_SIZE, 5>(
        pool,
        code.to_string(),
        exchange.to_string(),
        limit,
        [
            "open".to_string(),
            "high".to_string(),
            "low".to_string(),
            "close".to_string(),
            "volume".to_string(),
        ],
    )
}*/
