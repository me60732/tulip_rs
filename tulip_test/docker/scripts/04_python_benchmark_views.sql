-- =============================================================================
-- 03_python_benchmark_views.sql
-- Adds Python-comparison views to the existing indicator_benchmark database.
-- Does NOT recreate the database or touch existing tables/views.
--
-- Apply to a running database:
--   psql -U postgres -h localhost -d indicator_benchmark \
--        -f scripts/03_python_benchmark_views.sql
--
-- Implementation types written by the Python benchmarks:
--   'tulip_rs_python'  — tulip-rs called via the PyO3/maturin Python binding
--   'ta'               — bukosabino/ta (pure Python + numpy/pandas reference)
--   'pandas_ta'        — twopirllc/pandas-ta extension for pandas
-- =============================================================================

-- Drop in reverse-dependency order so re-running is safe
DROP VIEW IF EXISTS python_simd_asset_avg_comparison;
DROP VIEW IF EXISTS python_simd_asset_simplified_comparison;
DROP VIEW IF EXISTS python_simd_asset_performance_comparison;
DROP VIEW IF EXISTS python_simd_avg_comparison;
DROP VIEW IF EXISTS python_simd_simplified_comparison;
DROP VIEW IF EXISTS python_simd_performance_comparison;
DROP VIEW IF EXISTS python_avg_options_comparison;
DROP VIEW IF EXISTS python_performance_comparison;

-- ---------------------------------------------------------------------------
-- python_performance_comparison
-- One row per (run, indicator, stock, option-set).
-- Pivots tulip_rs_python and ta side by side and computes the ratio.
-- ---------------------------------------------------------------------------
CREATE VIEW python_performance_comparison AS
SELECT
    runs.id                            AS run_id,
    runs.run_timestamp                 AS benchmark_date,
    (runs.system_info ->> 'hostname')  AS hostname,
    ind.name                           AS indicator_name,
    res.stock_symbol,
    res.data_source,
    res.input_size,
    res.options,

    max(CASE WHEN res.implementation_type = 'tulip_rs_python'
             THEN res.mean_time_ns END)                          AS tulip_rs_python_mean_ns,
    max(CASE WHEN res.implementation_type = 'tulip_rs_python'
             THEN res.std_dev_ns END)                            AS tulip_rs_python_stddev_ns,
    max(CASE WHEN res.implementation_type = 'ta'
             THEN res.mean_time_ns END)                          AS ta_mean_ns,
    max(CASE WHEN res.implementation_type = 'ta'
             THEN res.std_dev_ns END)                            AS ta_stddev_ns,
    max(CASE WHEN res.implementation_type = 'pandas_ta'
             THEN res.mean_time_ns END)                          AS pandas_ta_mean_ns,
    max(CASE WHEN res.implementation_type = 'pandas_ta'
             THEN res.std_dev_ns END)                            AS pandas_ta_stddev_ns,

    -- How many times slower is ta vs tulip_rs_python?
    round(
        (max(CASE WHEN res.implementation_type = 'ta'
                  THEN res.mean_time_ns END))::numeric
        / NULLIF(
            (max(CASE WHEN res.implementation_type = 'tulip_rs_python'
                      THEN res.mean_time_ns END))::numeric,
          0),
    2)                                                           AS ta_to_tulip_ratio,

    -- How many times slower is pandas_ta vs tulip_rs_python?
    round(
        (max(CASE WHEN res.implementation_type = 'pandas_ta'
                  THEN res.mean_time_ns END))::numeric
        / NULLIF(
            (max(CASE WHEN res.implementation_type = 'tulip_rs_python'
                      THEN res.mean_time_ns END))::numeric,
          0),
    2)                                                           AS pandas_ta_to_tulip_ratio,

    -- Percentage of time saved by using tulip_rs_python instead of ta
    round(
        (
          (max(CASE WHEN res.implementation_type = 'ta'
                    THEN res.mean_time_ns END)
           - max(CASE WHEN res.implementation_type = 'tulip_rs_python'
                      THEN res.mean_time_ns END))::numeric
          / NULLIF(
              max(CASE WHEN res.implementation_type = 'ta'
                        THEN res.mean_time_ns END)::numeric,
            0)
        ) * 100,
    2)                                                           AS tulip_speedup_pct

    ,round(
        (
          (max(CASE WHEN res.implementation_type = 'pandas_ta'
                    THEN res.mean_time_ns END)
           - max(CASE WHEN res.implementation_type = 'tulip_rs_python'
                      THEN res.mean_time_ns END))::numeric
          / NULLIF(
              max(CASE WHEN res.implementation_type = 'pandas_ta'
                        THEN res.mean_time_ns END)::numeric,
            0)
        ) * 100,
    2)                                                           AS tulip_speedup_vs_pandas_ta_pct

FROM benchmark_runs runs
JOIN benchmark_results res ON runs.id = res.run_id
JOIN indicators ind        ON res.indicator_id = ind.id
WHERE res.implementation_type IN ('tulip_rs_python', 'ta', 'pandas_ta')
GROUP BY
    runs.id, runs.run_timestamp, runs.system_info,
    ind.name, res.stock_symbol, res.data_source, res.input_size, res.options
HAVING
    count(DISTINCT CASE WHEN res.implementation_type = 'tulip_rs_python'
                        THEN res.implementation_type END) = 1
ORDER BY runs.run_timestamp DESC, ind.name, res.stock_symbol;

-- ---------------------------------------------------------------------------
-- python_avg_options_comparison
-- One row per (run, indicator) — averaged across all option sets and stocks.
-- Mirrors the style of avg_options_comparison for the Rust benchmarks.
-- ---------------------------------------------------------------------------
CREATE VIEW python_avg_options_comparison AS
SELECT
    runs.id                            AS run_id,
    runs.run_timestamp                 AS benchmark_date,
    (runs.system_info ->> 'hostname')  AS hostname,
    ind.name                           AS indicator_name,

    round(avg(CASE WHEN res.implementation_type = 'tulip_rs_python'
                   THEN res.mean_time_ns END))                   AS tulip_rs_python_avg_ns,
    round(avg(CASE WHEN res.implementation_type = 'ta'
                   THEN res.mean_time_ns END))                   AS ta_avg_ns,
    round(avg(CASE WHEN res.implementation_type = 'pandas_ta'
                   THEN res.mean_time_ns END))                   AS pandas_ta_avg_ns,

    count(DISTINCT CASE WHEN res.implementation_type = 'tulip_rs_python'
                        THEN res.options END)                    AS tulip_options_count,
    count(DISTINCT CASE WHEN res.implementation_type = 'ta'
                        THEN res.options END)                    AS ta_options_count,
    count(DISTINCT CASE WHEN res.implementation_type = 'pandas_ta'
                        THEN res.options END)                    AS pandas_ta_options_count,

    round(
        avg(CASE WHEN res.implementation_type = 'ta'
                 THEN res.mean_time_ns END)
        / NULLIF(
            avg(CASE WHEN res.implementation_type = 'tulip_rs_python'
                     THEN res.mean_time_ns END),
          0),
    2)                                                           AS ta_to_tulip_ratio,

    round(
        avg(CASE WHEN res.implementation_type = 'pandas_ta'
                 THEN res.mean_time_ns END)
        / NULLIF(
            avg(CASE WHEN res.implementation_type = 'tulip_rs_python'
                     THEN res.mean_time_ns END),
          0),
    2)                                                           AS pandas_ta_to_tulip_ratio,

    round(
        (
          avg(CASE WHEN res.implementation_type = 'ta'
                   THEN res.mean_time_ns END)
          - avg(CASE WHEN res.implementation_type = 'tulip_rs_python'
                     THEN res.mean_time_ns END)
        )
        / NULLIF(
            avg(CASE WHEN res.implementation_type = 'ta'
                     THEN res.mean_time_ns END),
          0) * 100,
    2)                                                           AS tulip_speedup_pct

    ,round(
        (
          avg(CASE WHEN res.implementation_type = 'pandas_ta'
                   THEN res.mean_time_ns END)
          - avg(CASE WHEN res.implementation_type = 'tulip_rs_python'
                     THEN res.mean_time_ns END)
        )
        / NULLIF(
            avg(CASE WHEN res.implementation_type = 'pandas_ta'
                     THEN res.mean_time_ns END),
          0) * 100,
    2)                                                           AS tulip_speedup_vs_pandas_ta_pct

FROM benchmark_runs runs
JOIN benchmark_results res ON runs.id = res.run_id
JOIN indicators ind        ON res.indicator_id = ind.id
WHERE res.implementation_type IN ('tulip_rs_python', 'ta', 'pandas_ta')
GROUP BY runs.id, runs.run_timestamp, runs.system_info, ind.name
HAVING
    count(DISTINCT CASE WHEN res.implementation_type = 'tulip_rs_python'
                        THEN res.implementation_type END) = 1
ORDER BY runs.run_timestamp DESC, ind.name;

-- =============================================================================
-- SIMD comparison views
-- Compares the tulip_rs_python SIMD-batched code paths (written by the
-- bench runner as 'tulip_rs_python_simd_by_options' / '..._simd_by_assets')
-- against the plain 'tulip_rs_python' baseline (and, for the by-assets case,
-- also against 'ta' / 'pandas_ta'). Mirrors the Rust rust_simd_* view chain:
--   *_performance_comparison  (one row per run/indicator/stock or option-set)
--   *_simplified_comparison   (averaged per run/indicator/input_size or data_source)
--   *_avg_comparison          (averaged per run/indicator — the top-level view)
-- =============================================================================

-- ---------------------------------------------------------------------------
-- python_simd_performance_comparison
-- SIMD-by-options: one stock, every option set processed together in a
-- single call. Compared against the summed serial cost of running
-- tulip_rs_python once per option set for that same stock.
-- ---------------------------------------------------------------------------
CREATE VIEW python_simd_performance_comparison AS
SELECT
    runs.id                            AS run_id,
    runs.run_timestamp                 AS benchmark_date,
    (runs.system_info ->> 'hostname')  AS hostname,
    ind.name                           AS indicator_name,
    res.stock_symbol,
    res.data_source,
    res.input_size,

    sum(CASE WHEN res.implementation_type = 'tulip_rs_python'
             THEN res.mean_time_ns END)                          AS tulip_total_mean_time_ns,
    count(CASE WHEN res.implementation_type = 'tulip_rs_python'
               THEN 1 END)                                       AS tulip_options_count,
    max(CASE WHEN res.implementation_type = 'tulip_rs_python_simd_by_options'
             THEN res.mean_time_ns END)                          AS simd_mean_time_ns,
    max(CASE WHEN res.implementation_type = 'tulip_rs_python_simd_by_options'
             THEN res.sample_count END)                          AS simd_sample_count,

    round(
        (max(CASE WHEN res.implementation_type = 'tulip_rs_python_simd_by_options'
                  THEN res.mean_time_ns END))::numeric
        / NULLIF(
            sum(CASE WHEN res.implementation_type = 'tulip_rs_python'
                     THEN res.mean_time_ns END),
          0),
    4)                                                           AS simd_to_tulip_ratio,

    round(
        (sum(CASE WHEN res.implementation_type = 'tulip_rs_python'
                 THEN res.mean_time_ns END))::numeric
        / NULLIF(
            (max(CASE WHEN res.implementation_type = 'tulip_rs_python_simd_by_options'
                      THEN res.mean_time_ns END))::numeric,
          0) * 100,
    2)                                                           AS simd_vs_tulip_improvement_pct,

    round(
        (sum(CASE WHEN res.implementation_type = 'tulip_rs_python'
                 THEN res.mean_time_ns END))::numeric
        / NULLIF(
            (max(CASE WHEN res.implementation_type = 'tulip_rs_python_simd_by_options'
                      THEN res.mean_time_ns END))::numeric,
          0),
    2)                                                           AS simd_speedup_factor

FROM benchmark_runs runs
JOIN benchmark_results res ON runs.id = res.run_id
JOIN indicators ind        ON res.indicator_id = ind.id
WHERE res.implementation_type IN ('tulip_rs_python', 'tulip_rs_python_simd_by_options')
GROUP BY
    runs.id, runs.run_timestamp, runs.system_info,
    ind.name, res.stock_symbol, res.data_source, res.input_size
HAVING
    sum(CASE WHEN res.implementation_type = 'tulip_rs_python' THEN 1 ELSE 0 END) > 0
    AND sum(CASE WHEN res.implementation_type = 'tulip_rs_python_simd_by_options' THEN 1 ELSE 0 END) > 0
ORDER BY runs.run_timestamp DESC, ind.name, res.stock_symbol;

CREATE VIEW python_simd_simplified_comparison AS
SELECT run_id, benchmark_date, hostname, indicator_name, input_size,
    round(avg(tulip_total_mean_time_ns))                        AS tulip_avg_total_time_ns,
    avg(tulip_options_count)                                    AS tulip_avg_options_count,
    count(CASE WHEN tulip_total_mean_time_ns IS NOT NULL THEN 1 END) AS tulip_stock_count,
    round(avg(simd_mean_time_ns))                               AS simd_avg_time_ns,
    count(CASE WHEN simd_mean_time_ns IS NOT NULL THEN 1 END)   AS simd_stock_count,
    round(avg(simd_to_tulip_ratio), 4)                          AS avg_simd_to_tulip_ratio,
    round(avg(simd_vs_tulip_improvement_pct), 2)                AS avg_simd_improvement_pct,
    round(avg(simd_speedup_factor), 2)                          AS avg_simd_speedup_factor,
    round(min(simd_vs_tulip_improvement_pct), 2)                AS min_simd_improvement_pct,
    round(max(simd_vs_tulip_improvement_pct), 2)                AS max_simd_improvement_pct,
    round(min(simd_speedup_factor), 2)                          AS min_simd_speedup,
    round(max(simd_speedup_factor), 2)                          AS max_simd_speedup
FROM python_simd_performance_comparison
GROUP BY run_id, benchmark_date, hostname, indicator_name, input_size
ORDER BY benchmark_date DESC, indicator_name;

CREATE VIEW python_simd_avg_comparison AS
SELECT run_id, benchmark_date, hostname, indicator_name,
    round(avg(tulip_avg_total_time_ns))                         AS tulip_overall_avg_time_ns,
    round(avg(tulip_avg_options_count))                         AS tulip_overall_avg_options,
    round(avg(simd_avg_time_ns))                                AS simd_overall_avg_time_ns,
    round(avg(avg_simd_to_tulip_ratio), 4)                      AS overall_simd_to_tulip_ratio,
    round(avg(avg_simd_improvement_pct), 2)                     AS overall_simd_improvement_pct,
    round(avg(avg_simd_speedup_factor), 2)                      AS overall_simd_speedup_factor,
    round(min(min_simd_improvement_pct), 2)                     AS best_case_improvement_pct,
    round(max(max_simd_improvement_pct), 2)                     AS worst_case_improvement_pct,
    round(min(min_simd_speedup), 2)                             AS best_case_speedup,
    round(max(max_simd_speedup), 2)                             AS worst_case_speedup,
    sum(tulip_stock_count)                                      AS total_tulip_measurements,
    sum(simd_stock_count)                                       AS total_simd_measurements
FROM python_simd_simplified_comparison
GROUP BY run_id, benchmark_date, hostname, indicator_name
ORDER BY benchmark_date DESC, indicator_name;

-- ---------------------------------------------------------------------------
-- python_simd_asset_performance_comparison
-- SIMD-by-assets: one option set, every loaded stock processed together in a
-- single call. Compared against the summed serial cost of running
-- tulip_rs_python / ta / pandas_ta once per stock for that same option set.
-- ---------------------------------------------------------------------------
CREATE VIEW python_simd_asset_performance_comparison AS
SELECT
    runs.id                            AS run_id,
    runs.run_timestamp                 AS benchmark_date,
    (runs.system_info ->> 'hostname')  AS hostname,
    ind.name                           AS indicator_name,
    res.options,
    res.data_source,

    sum(CASE WHEN res.implementation_type = 'tulip_rs_python'
             THEN res.mean_time_ns ELSE 0 END)                  AS tulip_total_mean_time_ns,
    sum(CASE WHEN res.implementation_type = 'ta'
             THEN res.mean_time_ns ELSE 0 END)                  AS ta_total_mean_time_ns,
    sum(CASE WHEN res.implementation_type = 'pandas_ta'
             THEN res.mean_time_ns ELSE 0 END)                  AS pandas_ta_total_mean_time_ns,
    avg(CASE WHEN res.implementation_type = 'tulip_rs_python_simd_by_assets'
             THEN res.mean_time_ns END)                         AS simd_asset_mean_time_ns,

    round(
        avg(CASE WHEN res.implementation_type = 'tulip_rs_python_simd_by_assets'
                 THEN res.mean_time_ns END)
        / NULLIF(
            sum(CASE WHEN res.implementation_type = 'tulip_rs_python'
                     THEN res.mean_time_ns ELSE 0 END),
          0),
    4)                                                           AS simd_asset_to_tulip_ratio,

    round(
        (NULLIF(
            sum(CASE WHEN res.implementation_type = 'tulip_rs_python'
                     THEN res.mean_time_ns ELSE 0 END),
          0)
        / avg(CASE WHEN res.implementation_type = 'tulip_rs_python_simd_by_assets'
                   THEN res.mean_time_ns END)) * 100,
    2)                                                           AS simd_asset_vs_tulip_improvement_pct,

    round(
        (NULLIF(
            sum(CASE WHEN res.implementation_type = 'ta'
                     THEN res.mean_time_ns ELSE 0 END),
          0)
        / avg(CASE WHEN res.implementation_type = 'tulip_rs_python_simd_by_assets'
                   THEN res.mean_time_ns END)) * 100,
    2)                                                           AS simd_asset_vs_ta_improvement_pct,

    round(
        (NULLIF(
            sum(CASE WHEN res.implementation_type = 'pandas_ta'
                     THEN res.mean_time_ns ELSE 0 END),
          0)
        / avg(CASE WHEN res.implementation_type = 'tulip_rs_python_simd_by_assets'
                   THEN res.mean_time_ns END)) * 100,
    2)                                                           AS simd_asset_vs_pandas_ta_improvement_pct

FROM benchmark_runs runs
JOIN benchmark_results res ON runs.id = res.run_id
JOIN indicators ind        ON res.indicator_id = ind.id
WHERE res.implementation_type IN ('tulip_rs_python', 'ta', 'pandas_ta', 'tulip_rs_python_simd_by_assets')
GROUP BY
    runs.id, runs.run_timestamp, runs.system_info,
    ind.name, res.options, res.data_source
HAVING
    avg(CASE WHEN res.implementation_type = 'tulip_rs_python_simd_by_assets'
             THEN res.mean_time_ns END) IS NOT NULL
    AND (
        sum(CASE WHEN res.implementation_type = 'tulip_rs_python' THEN 1 ELSE 0 END) > 0
        OR sum(CASE WHEN res.implementation_type = 'ta' THEN 1 ELSE 0 END) > 0
        OR sum(CASE WHEN res.implementation_type = 'pandas_ta' THEN 1 ELSE 0 END) > 0
    )
ORDER BY runs.run_timestamp DESC, ind.name, res.options;

CREATE VIEW python_simd_asset_simplified_comparison AS
SELECT run_id, benchmark_date, hostname, indicator_name, data_source,
    round(avg(tulip_total_mean_time_ns))                        AS tulip_avg_total_time_ns,
    round(avg(ta_total_mean_time_ns))                           AS ta_avg_total_time_ns,
    round(avg(pandas_ta_total_mean_time_ns))                    AS pandas_ta_avg_total_time_ns,
    round(avg(simd_asset_mean_time_ns))                         AS simd_asset_avg_time_ns,
    round(avg(simd_asset_to_tulip_ratio), 4)                    AS avg_simd_asset_to_tulip_ratio,
    round(avg(simd_asset_vs_tulip_improvement_pct), 2)          AS avg_simd_asset_vs_tulip_improvement_pct,
    round(avg(simd_asset_vs_ta_improvement_pct), 2)             AS avg_simd_asset_vs_ta_improvement_pct,
    round(avg(simd_asset_vs_pandas_ta_improvement_pct), 2)      AS avg_simd_asset_vs_pandas_ta_improvement_pct
FROM python_simd_asset_performance_comparison
GROUP BY run_id, benchmark_date, hostname, indicator_name, data_source
ORDER BY benchmark_date DESC, indicator_name;

CREATE VIEW python_simd_asset_avg_comparison AS
SELECT run_id, benchmark_date, hostname, indicator_name,
    round(avg(tulip_avg_total_time_ns))                         AS tulip_overall_avg_time_ns,
    round(avg(ta_avg_total_time_ns))                            AS ta_overall_avg_time_ns,
    round(avg(pandas_ta_avg_total_time_ns))                     AS pandas_ta_overall_avg_time_ns,
    round(avg(simd_asset_avg_time_ns))                          AS simd_asset_overall_avg_time_ns,
    round(avg(avg_simd_asset_to_tulip_ratio), 4)                AS overall_simd_asset_to_tulip_ratio,
    round(avg(avg_simd_asset_vs_tulip_improvement_pct), 2)      AS overall_simd_asset_vs_tulip_improvement_pct,
    round(avg(avg_simd_asset_vs_ta_improvement_pct), 2)         AS overall_simd_asset_vs_ta_improvement_pct,
    round(avg(avg_simd_asset_vs_pandas_ta_improvement_pct), 2)  AS overall_simd_asset_vs_pandas_ta_improvement_pct
FROM python_simd_asset_simplified_comparison
GROUP BY run_id, benchmark_date, hostname, indicator_name
ORDER BY benchmark_date DESC, indicator_name;
