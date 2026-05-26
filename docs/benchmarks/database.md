# Benchmark Database

> The `indicator_benchmark` PostgreSQL database stores raw timing results for every benchmark run and exposes a layered set of views that progressively aggregate and compare that data. Set `BENCHMARK_LOG_TO_DB=1` in `tulip_test/.env` before running `cargo bench` to write results into the database — see [Running Benchmarks](setup.md) for setup instructions.

## Which View Do I Need?

| Question | View |
|---|---|
| What indicators have been benchmarked? | `indicator_summary` |
| How does Rust compare to C and TA-Lib for a specific run? | `avg_options_comparison` |
| Same, but per option set? | `simplified_comparison` |
| Per option set AND per stock symbol? | `performance_comparison` |
| How do all five Rust variants compare (batch, streaming, optional)? | `rust_impl_avg_options_comparison` |
| How fast is the 1-bar streaming update? | `rust_impl_avg_options_comparison` |
| SIMD by-options: speedup over 4× sequential Rust? | `rust_simd_avg_comparison` |
| SIMD by-assets: speedup over 4× sequential (Rust + C + TA-Lib)? | `rust_simd_asset_avg_comparison` |
| Which indicators is Rust currently slower than C on? | `rust_slower_indicators` |
| What changed since the last benchmark run? | `prev_avg_run_comparison` |
| Did SIMD performance change between runs? | `rust_simd_prev_run_comparison` |
| Candlestick pattern scan performance? | `candlestick_simplified_comparison` |

## Overview

**Database**: `indicator_benchmark` (PostgreSQL)  
**Schema script**: `scripts/init_benchmark_db.sql`

### Implementation Types

| `implementation_type` | Description |
|-----------------------|-------------|
| `Rust` | Standard Rust batch computation over the full dataset |
| `C_tulip` | C tulip indicators library, batch |
| `talib` | TA-Lib, batch |
| `Rust_FromState` | Rust stateful path, processing the full dataset via saved state |
| `Rust_optional` | Rust with optional outputs (computes intermediate values in a single pass) |
| `Rust_FromState_1_Bar` | Rust streaming: cost of updating one new bar using saved state |
| `Rust_FromState_1_Bar_json` | Same as above, but state is serialized/deserialized as JSON |
| `Rust_SIMD` | Rust SIMD: one asset processed with 4 different option sets simultaneously |
| `Rust_SIMD_by_assets` | Rust SIMD: 4 different assets processed with identical options simultaneously |

!!! info "Interpreting ratios"
    - **Ratio > 1.0** — the competitor is slower than Rust by that factor (Rust wins)
    - **Ratio < 1.0** — Rust is slower than the competitor
    - **Percent diff** — positive = Rust is slower, negative = Rust is faster

## Base Tables

| Table | Purpose |
|-------|---------|
| `indicators` | Catalog of all indicators — name, category, input/output counts |
| `benchmark_runs` | Metadata per run: timestamp, Rust version, system info (JSON), notes |
| `benchmark_results` | Raw timing rows: one row per indicator × implementation × stock × options × run |

## Views

Views are created in dependency order (Level 1 → 4). Higher-level views build on lower ones.

=== "Level 1 — Raw Aggregations"

    ### `indicator_summary`

    **What it shows**: Catalogue view of every indicator with its metadata and how many benchmark results exist for it.

    !!! tip "Use this view when..."
        Checking which indicators have been benchmarked, when they were last run, and what category/input-output counts they have.

    ```sql
    SELECT * FROM indicator_summary WHERE category = 'momentum';
    SELECT * FROM indicator_summary WHERE last_benchmarked IS NULL; -- never benchmarked
    ```

    Key columns: `name`, `category`, `input_count`, `output_count`, `has_options`, `benchmark_count`, `last_benchmarked`

    ---

    ### `performance_comparison`

    **What it shows**: Side-by-side raw mean times for `Rust`, `C_tulip`, and `talib` for each combination of run × indicator × stock symbol × input_size × options. Only rows where at least two implementations are present are included.

    !!! tip "Use this view when..."
        Drilling into a specific stock symbol or option set to see exact timing numbers and ratios for a given run.

    ```sql
    SELECT * FROM performance_comparison
    WHERE indicator_name = 'ema' AND run_id = 5
    ORDER BY stock_symbol;
    ```

    Key columns: `rust_mean_time_ns`, `c_mean_time_ns`, `talib_mean_time_ns`, `c_to_rust_ratio`, `talib_to_rust_ratio`, `rust_vs_c_percent_diff`, `rust_vs_talib_percent_diff`

    ---

    ### `simplified_comparison`

    **What it shows**: Averages `Rust`, `C_tulip`, and `talib` times across all stock symbols, grouped by run × indicator × input_size × options. Ratios and percent differences are computed from those averages.

    !!! tip "Use this view when..."
        You want a cleaner per-indicator-per-options summary without the per-symbol noise. Good starting point for comparing implementations.

    ```sql
    SELECT * FROM simplified_comparison
    WHERE indicator_name = 'rsi' ORDER BY benchmark_date DESC;
    ```

    Key columns: `rust_avg_mean_time_ns`, `c_avg_mean_time_ns`, `talib_avg_mean_time_ns`, `rust_symbol_count`, `c_to_rust_ratio`, `talib_to_rust_ratio`

    ---

    ### `avg_options_comparison`

    **What it shows**: Further averages `simplified_comparison` across all option sets, giving one row per run × indicator. Shows how many distinct option sets were tested for each implementation.

    !!! tip "Use this view when..."
        You need a quick headline comparison across all indicators for a given run — the broadest aggregate of batch performance.

    ```sql
    SELECT indicator_name, rust_avg_mean_time_ns, c_avg_mean_time_ns, c_to_rust_ratio
    FROM avg_options_comparison
    WHERE run_id = (SELECT MAX(id) FROM benchmark_runs)
    ORDER BY c_to_rust_ratio DESC;
    ```

    Key columns: `rust_avg_mean_time_ns`, `c_avg_mean_time_ns`, `talib_avg_mean_time_ns`, `rust_options_count`, `c_to_rust_ratio`, `talib_to_rust_ratio`, `rust_vs_c_percent_diff`

    ---

    ### `candlestick_simplified_comparison`

    **What it shows**: Aggregates results specifically for the `Rust_Candlestick` indicator (which scans all candle patterns in one pass). Groups by implementation type, input size, and options.

    !!! tip "Use this view when..."
        Evaluating the single-scan candlestick approach — how fast it is across different datasets and how timing varies by option set.

    ```sql
    SELECT * FROM candlestick_simplified_comparison ORDER BY benchmark_date DESC;
    ```

    Key columns: `forecast_name` (implementation type), `input_size`, `avg_mean_time_ns`, `symbol_count`, `avg_std_dev_ns`

    ---

    ### `rust_impl_performance_comparison`

    **What it shows**: Side-by-side raw mean times for all five Rust implementation variants (`Rust`, `Rust_FromState`, `Rust_optional`, `Rust_FromState_1_Bar`, `Rust_FromState_1_Bar_json`) per run × indicator × stock × input_size × options. Excludes candlestick indicators.

    !!! tip "Use this view when..."
        Comparing every Rust strategy at the per-symbol level — e.g., how much the streaming 1-bar path costs vs the full batch path for a specific stock.

    ```sql
    SELECT indicator_name, stock_symbol,
           rust_mean_time_ns, rust_fromstate_1_bar_mean_time_ns
    FROM rust_impl_performance_comparison
    WHERE run_id = 5 AND indicator_name = 'macd';
    ```

    Key columns: `rust_mean_time_ns`, `rust_fromstate_mean_time_ns`, `rust_optional_mean_time_ns`, `rust_fromstate_1_bar_mean_time_ns`, `rust_fromstate_1_bar_json_mean_time_ns`

    ---

    ### `rust_impl_simplified_comparison`

    **What it shows**: Averages all five Rust implementation variants across stock symbols, grouped by run × indicator × input_size × options. Excludes candlestick indicators.

    !!! tip "Use this view when..."
        Comparing streaming vs batch vs optional output costs across option sets without per-symbol noise.

    ```sql
    SELECT indicator_name, input_size,
           rust_avg_mean_time_ns, rust_fromstate_1_bar_avg_mean_time_ns
    FROM rust_impl_simplified_comparison
    WHERE run_id = 5 ORDER BY indicator_name;
    ```

    ---

    ### `rust_impl_avg_options_comparison`

    **What it shows**: Further averages all Rust variants across option sets, giving one row per run × indicator. Excludes candlestick indicators.

    !!! tip "Use this view when..."
        You need a headline streaming performance summary per indicator — the source for the streaming benchmarks report.

    ```sql
    SELECT indicator_name,
           rust_avg_mean_time_ns           AS batch_ns,
           rust_fromstate_1_bar_avg_mean_time_ns AS streaming_1bar_ns
    FROM rust_impl_avg_options_comparison
    WHERE run_id = (SELECT MAX(id) FROM benchmark_runs)
    ORDER BY rust_fromstate_1_bar_avg_mean_time_ns;
    ```

    Key columns: `rust_avg_mean_time_ns`, `rust_fromstate_avg_mean_time_ns`, `rust_optional_avg_mean_time_ns`, `rust_fromstate_1_bar_avg_mean_time_ns`, `rust_fromstate_1_bar_json_avg_mean_time_ns`

    ---

    ### `rust_simd_performance_comparison`

    **What it shows**: Per run × indicator × stock × input_size: compares `Rust_SIMD` (4 option sets simultaneously) against the **sum** of 4 separate `Rust` runs. Computes speedup factor and percent improvement.

    A `simd_speedup_factor` of 3.5 means SIMD processes 4 option sets in the time it would take to do 3.5 sequential Rust runs.

    !!! tip "Use this view when..."
        Evaluating whether SIMD-by-options gives a real throughput benefit vs running 4 individual Rust calls sequentially.

    ```sql
    SELECT indicator_name, stock_symbol, simd_speedup_factor, simd_vs_rust_percent_improvement
    FROM rust_simd_performance_comparison
    WHERE run_id = 5 ORDER BY simd_speedup_factor DESC;
    ```

    Key columns: `rust_total_mean_time_ns` (sum of 4 Rust runs), `rust_simd_mean_time_ns`, `simd_to_rust_ratio`, `simd_vs_rust_percent_improvement`, `simd_speedup_factor`

    ---

    ### `rust_simd_asset_performance_comparison`

    **What it shows**: Per run × indicator × options × data_source: compares `Rust_SIMD_by_assets` (4 different assets simultaneously) against the **sum** of 4 separate single-asset runs for `Rust`, `C_tulip`, and `talib`. Computes percent improvement over each competitor.

    !!! tip "Use this view when..."
        Evaluating the asset-parallel SIMD strategy — does processing 4 stocks at once beat 4 sequential calls, and by how much vs C and talib?

    ```sql
    SELECT indicator_name, options, simd_asset_to_rust_ratio,
           simd_asset_vs_rust_percent_improvement,
           simd_asset_vs_c_tulip_percent_improvement
    FROM rust_simd_asset_performance_comparison
    WHERE run_id = 5;
    ```

    Key columns: `rust_total_mean_time_ns`, `c_tulip_total_mean_time_ns`, `talib_total_mean_time_ns`, `rust_simd_asset_mean_time_ns`, `simd_asset_to_rust_ratio`, `simd_asset_vs_rust_percent_improvement`, `simd_asset_vs_c_tulip_percent_improvement`, `simd_asset_vs_talib_percent_improvement`

    ---

    ### `rust_simd_c_tulip_performance_comparison`

    **What it shows**: Per run × indicator × stock × input_size: compares `Rust_SIMD` (by options) against the **sum** of 4 separate `C_tulip` runs. Computes speedup and percent improvement vs C.

    !!! tip "Use this view when..."
        You need a direct SIMD vs C comparison — does Rust SIMD beat C running 4 sequential option sets?

    ```sql
    SELECT indicator_name, simd_speedup_vs_c_tulip, simd_vs_c_tulip_percent_improvement
    FROM rust_simd_c_tulip_performance_comparison
    WHERE run_id = 5 ORDER BY simd_speedup_vs_c_tulip DESC;
    ```

    Key columns: `rust_simd_mean_time_ns`, `c_tulip_total_mean_time_ns`, `c_tulip_to_simd_ratio`, `simd_vs_c_tulip_percent_improvement`, `simd_speedup_vs_c_tulip`

=== "Level 2 — Cross-Run Aggregations"

    ### `prev_run_comparison`

    **What it shows**: For each indicator × input_size × options × hostname, compares the current run to the immediately preceding run (using window functions). Shows the percent change in avg time for Rust, C, and talib, and how the C/talib-to-Rust ratios shifted.

    !!! tip "Use this view when..."
        Spotting performance regressions or improvements between two consecutive benchmark runs, at the per-options level.

    ```sql
    SELECT indicator_name, input_size, rust_performance_change_pct, c_ratio_change_pct
    FROM prev_run_comparison
    WHERE run_id = 5 AND rust_performance_change_pct > 5  -- Rust got slower
    ORDER BY rust_performance_change_pct DESC;
    ```

    Key columns: `current_rust_avg_time_ns`, `prev_rust_avg_time_ns`, `rust_performance_change_pct`, `c_performance_change_pct`, `talib_performance_change_pct`, `c_ratio_change_pct`, `days_between_runs`

    ---

    ### `prev_avg_run_comparison`

    **What it shows**: Same as `prev_run_comparison` but uses `avg_options_comparison` as input — so results are already averaged across option sets. One row per run × indicator × hostname.

    !!! tip "Use this view when..."
        You need higher-level regression tracking across runs without worrying about which specific option set changed.

    ```sql
    SELECT indicator_name, rust_performance_change_pct, talib_ratio_change_pct, days_between_runs
    FROM prev_avg_run_comparison
    WHERE run_id = 5 ORDER BY rust_performance_change_pct DESC;
    ```

    ---

    ### `rust_impl_prev_run_comparison`

    **What it shows**: For each indicator × input_size × options × hostname, compares all five Rust implementation variants between the current and previous run.

    !!! tip "Use this view when..."
        Checking whether any particular Rust strategy (e.g., `Rust_FromState_1_Bar`) regressed between runs, independent of the C/talib comparison.

    ```sql
    SELECT indicator_name, rust_performance_change_pct,
           rust_fromstate_1_bar_performance_change_pct
    FROM rust_impl_prev_run_comparison WHERE run_id = 5;
    ```

    ---

    ### `rust_impl_prev_avg_run_comparison`

    **What it shows**: Same as `rust_impl_prev_run_comparison` but averaged across option sets (one row per run × indicator × hostname).

    !!! tip "Use this view when..."
        You need headline Rust-only regression tracking across all five variants.

    ```sql
    SELECT indicator_name,
           current_rust_avg_time_ns, prev_rust_avg_time_ns,
           rust_performance_change_pct,
           rust_fromstate_1_bar_performance_change_pct
    FROM rust_impl_prev_avg_run_comparison WHERE run_id = 5;
    ```

    ---

    ### `rust_simd_simplified_comparison`

    **What it shows**: Aggregates `rust_simd_performance_comparison` across all stock symbols, grouped by run × indicator × input_size. Produces average, min, and max speedup/improvement figures.

    !!! tip "Use this view when..."
        Getting a representative SIMD-by-options speedup number per indicator and input size, without per-symbol detail.

    ```sql
    SELECT indicator_name, input_size,
           avg_simd_speedup_factor, min_simd_speedup, max_simd_speedup
    FROM rust_simd_simplified_comparison WHERE run_id = 5;
    ```

    Key columns: `rust_avg_total_time_ns`, `rust_simd_avg_time_ns`, `avg_simd_to_rust_ratio`, `avg_simd_improvement_pct`, `avg_simd_speedup_factor`, `min_simd_improvement_pct`, `max_simd_improvement_pct`

    ---

    ### `rust_simd_asset_simplified_comparison`

    **What it shows**: Aggregates `rust_simd_asset_performance_comparison` across options, grouped by run × indicator × data_source. Gives average improvement figures vs Rust, C_tulip, and talib.

    !!! tip "Use this view when..."
        You need a summary SIMD-by-assets improvement per indicator, showing whether the asset-parallel strategy is consistently beneficial.

    ```sql
    SELECT indicator_name,
           avg_simd_asset_vs_rust_improvement_pct,
           avg_simd_asset_vs_c_tulip_improvement_pct
    FROM rust_simd_asset_simplified_comparison WHERE run_id = 5;
    ```

    ---

    ### `rust_simd_c_tulip_simplified_comparison`

    **What it shows**: Aggregates `rust_simd_c_tulip_performance_comparison` across stock symbols, grouped by run × indicator × input_size. Gives average, min, and max SIMD speedup vs C_tulip.

    !!! tip "Use this view when..."
        Summarizing whether Rust SIMD (4 option sets) consistently beats 4 sequential C_tulip calls.

    ```sql
    SELECT indicator_name, avg_simd_speedup_vs_c_tulip,
           min_simd_speedup_vs_c_tulip, max_simd_speedup_vs_c_tulip
    FROM rust_simd_c_tulip_simplified_comparison WHERE run_id = 5;
    ```

=== "Level 3 — Actionable Filters"

    ### `rust_slower_indicators`

    **What it shows**: Filters `prev_avg_run_comparison` to only the **latest** run per indicator × hostname, then keeps only rows where Rust is currently **slower** than C_tulip or talib. Shows exactly how much slower Rust is (as a percent).

    !!! tip "Use this view when..."
        Immediately spotting which indicators need optimization work — the actionable regression list.

    ```sql
    SELECT * FROM rust_slower_indicators ORDER BY rust_slower_than_c_by_pct DESC NULLS LAST;
    ```

    Key columns: `rust_slower_than_c_by_pct`, `rust_slower_than_talib_by_pct`, `current_rust_avg_time_ns`, `current_c_avg_time_ns`, `rust_performance_change_pct`

    ---

    ### `rust_impl_slower_indicators`

    **What it shows**: Filters `rust_impl_prev_avg_run_comparison` to the **latest** run per indicator × hostname. Shows performance change percentages for all five Rust variants.

    !!! tip "Use this view when..."
        Tracking whether any Rust implementation variant (streaming, optional, etc.) has regressed since the last run, even if it isn't slower than C or talib.

    ```sql
    SELECT * FROM rust_impl_slower_indicators
    WHERE rust_fromstate_1_bar_performance_change_pct > 10 -- streaming got 10%+ slower
    ORDER BY rust_fromstate_1_bar_performance_change_pct DESC;
    ```

    Key columns: `current_rust_avg_time_ns`, `current_rust_fromstate_1_bar_avg_time_ns`, `rust_performance_change_pct`, `rust_fromstate_1_bar_performance_change_pct`, etc.

    ---

    ### `rust_simd_avg_comparison`

    **What it shows**: Aggregates `rust_simd_simplified_comparison` across input sizes, giving one overall row per run × indicator × hostname. Includes best-case and worst-case speedup bounds.

    !!! tip "Use this view when..."
        You need the headline SIMD-by-options number per indicator — "does SIMD help for this indicator, and by how much on average?"

    ```sql
    SELECT indicator_name, overall_simd_improvement_pct, overall_simd_speedup_factor,
           best_case_speedup, worst_case_speedup
    FROM rust_simd_avg_comparison
    WHERE run_id = (SELECT MAX(id) FROM benchmark_runs)
    ORDER BY overall_simd_speedup_factor DESC;
    ```

    ---

    ### `rust_simd_asset_avg_comparison`

    **What it shows**: Aggregates `rust_simd_asset_simplified_comparison` further, giving one row per run × indicator × hostname. Overall improvement percentages vs Rust, C_tulip, and talib.

    !!! tip "Use this view when..."
        Comparing the SIMD-by-assets strategy against all three competitors in a single summary query.

    ```sql
    SELECT indicator_name,
           overall_simd_asset_vs_rust_improvement_pct,
           overall_simd_asset_vs_c_tulip_improvement_pct,
           overall_simd_asset_vs_talib_improvement_pct
    FROM rust_simd_asset_avg_comparison
    WHERE run_id = (SELECT MAX(id) FROM benchmark_runs);
    ```

    ---

    ### `rust_simd_c_tulip_avg_comparison`

    **What it shows**: Aggregates `rust_simd_c_tulip_simplified_comparison` further per run × indicator × hostname. Headline SIMD vs C_tulip speedup with best/worst case bounds.

    !!! tip "Use this view when..."
        Answering "is Rust SIMD (4 option sets) faster than running C_tulip 4 times sequentially, and by how much?"

    ```sql
    SELECT indicator_name, overall_simd_vs_c_tulip_improvement_pct,
           overall_simd_speedup_vs_c_tulip,
           best_case_simd_speedup_vs_c_tulip, worst_case_simd_speedup_vs_c_tulip
    FROM rust_simd_c_tulip_avg_comparison
    WHERE run_id = (SELECT MAX(id) FROM benchmark_runs);
    ```

=== "Level 4 — SIMD Regression Tracking"

    ### `rust_simd_prev_run_comparison`

    **What it shows**: Compares `rust_simd_avg_comparison` between the current and previous run. Shows how the SIMD speedup factor and improvement percentage shifted between runs.

    !!! tip "Use this view when..."
        Detecting whether SIMD performance is trending better or worse across benchmark runs.

    ```sql
    SELECT indicator_name,
           current_simd_speedup_factor, prev_simd_speedup_factor,
           simd_speedup_change_pct, simd_improvement_change_pct,
           days_between_runs
    FROM rust_simd_prev_run_comparison ORDER BY benchmark_date DESC;
    ```

    Key columns: `current_simd_avg_time_ns`, `prev_simd_avg_time_ns`, `simd_performance_change_pct`, `simd_improvement_change_pct`, `simd_speedup_change_pct`

    ---

    ### `rust_simd_asset_prev_run_comparison`

    **What it shows**: Compares `rust_simd_asset_avg_comparison` between the current and previous run. Shows changes in SIMD-by-assets improvement ratios vs Rust, C_tulip, and talib.

    !!! tip "Use this view when..."
        Tracking whether the asset-parallel strategy is getting more or less effective over time.

    ```sql
    SELECT indicator_name,
           simd_asset_performance_change_pct,
           simd_asset_vs_rust_improvement_change_pct,
           simd_asset_vs_c_tulip_improvement_change_pct
    FROM rust_simd_asset_prev_run_comparison ORDER BY benchmark_date DESC;
    ```

    ---

    ### `rust_simd_c_tulip_prev_run_comparison`

    **What it shows**: Compares `rust_simd_c_tulip_avg_comparison` between the current and previous run. Shows how the SIMD-vs-C speedup changed.

    !!! tip "Use this view when..."
        Tracking whether Rust SIMD is widening or narrowing its lead over C_tulip across runs.

    ```sql
    SELECT indicator_name,
           current_simd_vs_c_tulip_improvement_pct,
           prev_simd_vs_c_tulip_improvement_pct,
           simd_vs_c_tulip_improvement_change_pct,
           simd_speedup_vs_c_tulip_change_pct
    FROM rust_simd_c_tulip_prev_run_comparison ORDER BY benchmark_date DESC;
    ```

## Common Query Patterns

### Latest Run ID

```sql
SELECT MAX(id) AS latest_run FROM benchmark_runs;
-- or with metadata:
SELECT id, run_timestamp, rust_version, system_info->>'hostname' AS host
FROM benchmark_runs ORDER BY run_timestamp DESC LIMIT 5;
```

### Standard Benchmark Results for the Latest Run

```sql
SELECT indicator_name, rust_avg_mean_time_ns, c_avg_mean_time_ns, c_to_rust_ratio
FROM avg_options_comparison
WHERE run_id = (SELECT MAX(id) FROM benchmark_runs)
ORDER BY c_to_rust_ratio DESC;
```

### Streaming Efficiency: Batch Time vs 1-Bar Update Cost

```sql
SELECT indicator_name,
       rust_avg_mean_time_ns                    AS batch_ns,
       rust_fromstate_1_bar_avg_mean_time_ns    AS streaming_1bar_ns,
       round(rust_avg_mean_time_ns::numeric /
             NULLIF(rust_fromstate_1_bar_avg_mean_time_ns, 0), 1) AS speedup
FROM rust_impl_avg_options_comparison
WHERE run_id = (SELECT MAX(id) FROM benchmark_runs)
  AND rust_fromstate_1_bar_avg_mean_time_ns IS NOT NULL
ORDER BY speedup DESC;
```

### Find Indicators Where Rust Is Slower Than C

```sql
SELECT * FROM rust_slower_indicators ORDER BY rust_slower_than_c_by_pct DESC NULLS LAST;
```

### Regression Check: What Changed Between the Last Two Runs

```sql
SELECT indicator_name, rust_performance_change_pct, c_ratio_change_pct, days_between_runs
FROM prev_avg_run_comparison
WHERE run_id = (SELECT MAX(id) FROM benchmark_runs)
ORDER BY ABS(rust_performance_change_pct) DESC;
```

### SIMD by-Options: Top Performers

```sql
SELECT indicator_name, overall_simd_speedup_factor, best_case_speedup
FROM rust_simd_avg_comparison
WHERE run_id = (SELECT MAX(id) FROM benchmark_runs)
ORDER BY overall_simd_speedup_factor DESC;
```

### SIMD by-Assets vs All Competitors

```sql
SELECT indicator_name,
       overall_simd_asset_vs_rust_improvement_pct    AS vs_rust_pct,
       overall_simd_asset_vs_c_tulip_improvement_pct AS vs_c_pct,
       overall_simd_asset_vs_talib_improvement_pct   AS vs_talib_pct
FROM rust_simd_asset_avg_comparison
WHERE run_id = (SELECT MAX(id) FROM benchmark_runs)
ORDER BY vs_rust_pct DESC;
```

## View Dependency Map

```text
Base tables: indicators, benchmark_runs, benchmark_results
│
├── Level 1 (direct table queries)
│   ├── indicator_summary
│   ├── performance_comparison
│   ├── simplified_comparison
│   ├── avg_options_comparison
│   ├── candlestick_simplified_comparison
│   ├── rust_impl_performance_comparison
│   ├── rust_impl_simplified_comparison
│   ├── rust_impl_avg_options_comparison
│   ├── rust_simd_performance_comparison
│   ├── rust_simd_asset_performance_comparison
│   └── rust_simd_c_tulip_performance_comparison
│
├── Level 2 (build on Level 1)
│   ├── prev_run_comparison               ← simplified_comparison
│   ├── prev_avg_run_comparison           ← avg_options_comparison
│   ├── rust_impl_prev_run_comparison     ← rust_impl_simplified_comparison
│   ├── rust_impl_prev_avg_run_comparison ← rust_impl_avg_options_comparison
│   ├── rust_simd_simplified_comparison   ← rust_simd_performance_comparison
│   ├── rust_simd_asset_simplified_comparison ← rust_simd_asset_performance_comparison
│   └── rust_simd_c_tulip_simplified_comparison ← rust_simd_c_tulip_performance_comparison
│
├── Level 3 (build on Level 2)
│   ├── rust_slower_indicators            ← prev_avg_run_comparison
│   ├── rust_impl_slower_indicators       ← rust_impl_prev_avg_run_comparison
│   ├── rust_simd_avg_comparison          ← rust_simd_simplified_comparison
│   ├── rust_simd_asset_avg_comparison    ← rust_simd_asset_simplified_comparison
│   └── rust_simd_c_tulip_avg_comparison  ← rust_simd_c_tulip_simplified_comparison
│
└── Level 4 (build on Level 3)
    ├── rust_simd_prev_run_comparison          ← rust_simd_avg_comparison
    ├── rust_simd_asset_prev_run_comparison    ← rust_simd_asset_avg_comparison
    └── rust_simd_c_tulip_prev_run_comparison  ← rust_simd_c_tulip_avg_comparison
```
