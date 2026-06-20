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
    count(DISTINCT res.implementation_type) >= 2
    AND count(DISTINCT CASE WHEN res.implementation_type = 'tulip_rs_python'
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
    count(DISTINCT res.implementation_type) >= 2
    AND count(DISTINCT CASE WHEN res.implementation_type = 'tulip_rs_python'
                            THEN res.implementation_type END) = 1
ORDER BY runs.run_timestamp DESC, ind.name;
