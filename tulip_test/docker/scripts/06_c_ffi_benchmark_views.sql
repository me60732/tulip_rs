-- =============================================================================
-- 06_c_ffi_benchmark_views.sql
-- Adds C/FFI-comparison views to the existing indicator_benchmark database.
-- Does NOT recreate the database or touch existing tables/views.
--
-- Applied automatically by Docker on first init.
-- Comment out the volume mount in docker-compose.yaml to skip these views.
--
-- Run manually:
--   psql -U postgres -h localhost -d indicator_benchmark \
--        -f scripts/06_c_ffi_benchmark_views.sql
--
-- Implementation types written by the C benchmark harness
-- (tulip_rs_ffi/bench):
--   'tulip_rs_ffi_c'                    — tulip-rs called via the hand-rolled extern "C" FFI wrapper
--   'tulip_rs_ffi_c_simd_by_assets'     — SIMD: one option set across 4 assets in a single call
--   'tulip_rs_ffi_c_simd_by_options'    — SIMD: 4 option sets on one asset in a single call
--   'C_tulip'                           — Tulip Indicators (C), tulip_rs_ffi/bench/tulip_indicators submodule
--   'talib'                             — TA-Lib, tulip_rs_ffi/bench/ta_lib_src submodule
--
-- (Superseded 'tulip_rs_diplomat_c' / Diplomat-generated FFI harness — the
-- Diplomat bindings and their benchmark harness have been removed entirely.)
--
-- SIMD note: option-less indicators have no *_simd_by_options path (nothing
-- to batch options over), so they only produce *_simd_by_assets rows — same
-- as the Node harness where a SIMD variant doesn't exist.
--
-- All comparison views show tulip_rs_ffi_c results even when no reference
-- library ran the same indicator (comparison columns will be NULL in that
-- case).
-- =============================================================================

\c indicator_benchmark

\echo '>>> Creating C/FFI benchmark views...'

-- Drop in reverse-dependency order so re-running is safe
DROP VIEW IF EXISTS c_ffi_simd_asset_avg_comparison;
DROP VIEW IF EXISTS c_ffi_simd_asset_simplified_comparison;
DROP VIEW IF EXISTS c_ffi_simd_asset_performance_comparison;
DROP VIEW IF EXISTS c_ffi_simd_avg_comparison;
DROP VIEW IF EXISTS c_ffi_simd_simplified_comparison;
DROP VIEW IF EXISTS c_ffi_simd_performance_comparison;
DROP VIEW IF EXISTS c_ffi_avg_options_comparison;
DROP VIEW IF EXISTS c_ffi_performance_comparison;
-- Drop the superseded Diplomat-era views if they still exist from a previous run.
DROP VIEW IF EXISTS c_diplomat_avg_options_comparison;
DROP VIEW IF EXISTS c_diplomat_performance_comparison;

-- ---------------------------------------------------------------------------
-- c_ffi_performance_comparison
-- One row per (run, indicator, stock, option-set).
-- Pivots tulip_rs_ffi_c, C_tulip, and talib side by side and computes
-- x-faster ratios relative to tulip_rs_ffi_c.
-- Rows are included whenever tulip_rs_ffi_c has a result; reference
-- columns are NULL when no matching reference run exists for that
-- combination.
-- ---------------------------------------------------------------------------
CREATE VIEW c_ffi_performance_comparison AS
SELECT
    runs.id                            AS run_id,
    runs.run_timestamp                 AS benchmark_date,
    (runs.system_info ->> 'hostname')  AS hostname,
    ind.name                           AS indicator_name,
    res.stock_symbol,
    res.data_source,
    res.input_size,
    res.options,

    -- tulip_rs_ffi_c
    max(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
             THEN res.mean_time_ns END)                              AS tulip_rs_ffi_c_mean_ns,
    max(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
             THEN res.std_dev_ns END)                                AS tulip_rs_ffi_c_stddev_ns,

    -- C_tulip (Tulip Indicators C library)
    max(CASE WHEN res.implementation_type = 'C_tulip'
             THEN res.mean_time_ns END)                              AS c_tulip_mean_ns,
    max(CASE WHEN res.implementation_type = 'C_tulip'
             THEN res.std_dev_ns END)                                AS c_tulip_stddev_ns,

    -- talib
    max(CASE WHEN res.implementation_type = 'talib'
             THEN res.mean_time_ns END)                              AS talib_mean_ns,
    max(CASE WHEN res.implementation_type = 'talib'
             THEN res.std_dev_ns END)                                AS talib_stddev_ns,

    -- C_tulip / tulip_rs_ffi_c  (> 1 means tulip_rs_ffi_c is faster)
    round(
        (max(CASE WHEN res.implementation_type = 'C_tulip'
                  THEN res.mean_time_ns END))::numeric
        / NULLIF(
            (max(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                      THEN res.mean_time_ns END))::numeric,
          0),
    2)                                                               AS c_tulip_to_ffi_ratio,

    -- talib / tulip_rs_ffi_c  (> 1 means tulip_rs_ffi_c is faster)
    round(
        (max(CASE WHEN res.implementation_type = 'talib'
                  THEN res.mean_time_ns END))::numeric
        / NULLIF(
            (max(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                      THEN res.mean_time_ns END))::numeric,
          0),
    2)                                                               AS talib_to_ffi_ratio,

    -- % time saved vs C_tulip (NULL when C_tulip has no result)
    round(
        (
          (max(CASE WHEN res.implementation_type = 'C_tulip'
                    THEN res.mean_time_ns END)
           - max(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                      THEN res.mean_time_ns END))::numeric
          / NULLIF(
              max(CASE WHEN res.implementation_type = 'C_tulip'
                        THEN res.mean_time_ns END)::numeric,
            0)
        ) * 100,
    2)                                                               AS ffi_speedup_pct_vs_c_tulip,

    -- % time saved vs talib (NULL when talib has no result)
    round(
        (
          (max(CASE WHEN res.implementation_type = 'talib'
                    THEN res.mean_time_ns END)
           - max(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                      THEN res.mean_time_ns END))::numeric
          / NULLIF(
              max(CASE WHEN res.implementation_type = 'talib'
                        THEN res.mean_time_ns END)::numeric,
            0)
        ) * 100,
    2)                                                               AS ffi_speedup_pct_vs_talib

FROM benchmark_runs runs
JOIN benchmark_results res ON runs.id = res.run_id
JOIN indicators ind        ON res.indicator_id = ind.id
WHERE res.implementation_type IN ('tulip_rs_ffi_c', 'C_tulip', 'talib')
GROUP BY
    runs.id, runs.run_timestamp, runs.system_info,
    ind.name, res.stock_symbol, res.data_source, res.input_size, res.options
-- Require tulip_rs_ffi_c to be present; reference libraries are optional.
HAVING max(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c' THEN 1 END) = 1
ORDER BY runs.run_timestamp DESC, ind.name, res.stock_symbol;

-- ---------------------------------------------------------------------------
-- c_ffi_avg_options_comparison
-- One row per (run, indicator) — averaged across all option sets and stocks.
-- Includes all indicators that have a tulip_rs_ffi_c result; reference
-- columns are NULL when no matching reference run exists for that indicator.
-- ---------------------------------------------------------------------------
CREATE VIEW c_ffi_avg_options_comparison AS
SELECT
    runs.id                            AS run_id,
    runs.run_timestamp                 AS benchmark_date,
    (runs.system_info ->> 'hostname')  AS hostname,
    ind.name                           AS indicator_name,

    -- tulip_rs_ffi_c
    round(avg(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                   THEN res.mean_time_ns END))                       AS tulip_rs_ffi_c_avg_ns,
    count(DISTINCT CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                        THEN res.options END)                        AS ffi_options_count,

    -- C_tulip
    round(avg(CASE WHEN res.implementation_type = 'C_tulip'
                   THEN res.mean_time_ns END))                       AS c_tulip_avg_ns,
    count(DISTINCT CASE WHEN res.implementation_type = 'C_tulip'
                        THEN res.options END)                        AS c_tulip_options_count,

    -- talib
    round(avg(CASE WHEN res.implementation_type = 'talib'
                   THEN res.mean_time_ns END))                       AS talib_avg_ns,
    count(DISTINCT CASE WHEN res.implementation_type = 'talib'
                        THEN res.options END)                        AS talib_options_count,

    -- C_tulip / tulip_rs_ffi_c
    round(
        avg(CASE WHEN res.implementation_type = 'C_tulip'
                 THEN res.mean_time_ns END)
        / NULLIF(
            avg(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                     THEN res.mean_time_ns END),
          0),
    2)                                                               AS c_tulip_to_ffi_ratio,

    -- talib / tulip_rs_ffi_c
    round(
        avg(CASE WHEN res.implementation_type = 'talib'
                 THEN res.mean_time_ns END)
        / NULLIF(
            avg(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                     THEN res.mean_time_ns END),
          0),
    2)                                                               AS talib_to_ffi_ratio,

    -- % time saved vs C_tulip
    round(
        (
          avg(CASE WHEN res.implementation_type = 'C_tulip'
                   THEN res.mean_time_ns END)
          - avg(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                     THEN res.mean_time_ns END)
        )
        / NULLIF(
            avg(CASE WHEN res.implementation_type = 'C_tulip'
                     THEN res.mean_time_ns END),
          0) * 100,
    2)                                                               AS ffi_speedup_pct_vs_c_tulip,

    -- % time saved vs talib
    round(
        (
          avg(CASE WHEN res.implementation_type = 'talib'
                   THEN res.mean_time_ns END)
          - avg(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                     THEN res.mean_time_ns END)
        )
        / NULLIF(
            avg(CASE WHEN res.implementation_type = 'talib'
                     THEN res.mean_time_ns END),
          0) * 100,
    2)                                                               AS ffi_speedup_pct_vs_talib

FROM benchmark_runs runs
JOIN benchmark_results res ON runs.id = res.run_id
JOIN indicators ind        ON res.indicator_id = ind.id
WHERE res.implementation_type IN ('tulip_rs_ffi_c', 'C_tulip', 'talib')
GROUP BY runs.id, runs.run_timestamp, runs.system_info, ind.name
-- Require tulip_rs_ffi_c to be present; reference libraries are optional.
HAVING max(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c' THEN 1 END) = 1
ORDER BY runs.run_timestamp DESC, ind.name;

-- ---------------------------------------------------------------------------
-- c_ffi_simd_performance_comparison
-- SIMD-by-options: one SIMD call (4 option sets on one asset) compared
-- against the summed serial cost of running tulip_rs_ffi_c once per option
-- set for that same stock. Mirrors node_simd_performance_comparison.
-- ---------------------------------------------------------------------------
CREATE VIEW c_ffi_simd_performance_comparison AS
SELECT
    runs.id                            AS run_id,
    runs.run_timestamp                 AS benchmark_date,
    (runs.system_info ->> 'hostname')  AS hostname,
    ind.name                           AS indicator_name,
    res.stock_symbol,
    res.data_source,
    res.input_size,

    sum(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
             THEN res.mean_time_ns END)                          AS tulip_total_mean_time_ns,
    count(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
               THEN 1 END)                                       AS tulip_options_count,
    max(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c_simd_by_options'
             THEN res.mean_time_ns END)                          AS simd_mean_time_ns,
    max(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c_simd_by_options'
             THEN res.sample_count END)                          AS simd_sample_count,

    round(
        (max(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c_simd_by_options'
                  THEN res.mean_time_ns END))::numeric
        / NULLIF(
            sum(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                     THEN res.mean_time_ns END),
          0),
    4)                                                           AS simd_to_tulip_ratio,

    round(
        (sum(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                 THEN res.mean_time_ns END))::numeric
        / NULLIF(
            (max(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c_simd_by_options'
                      THEN res.mean_time_ns END))::numeric,
          0) * 100,
    2)                                                           AS simd_vs_tulip_improvement_pct,

    round(
        (sum(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                 THEN res.mean_time_ns END))::numeric
        / NULLIF(
            (max(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c_simd_by_options'
                      THEN res.mean_time_ns END))::numeric,
          0),
    2)                                                           AS simd_speedup_factor

FROM benchmark_runs runs
JOIN benchmark_results res ON runs.id = res.run_id
JOIN indicators ind        ON res.indicator_id = ind.id
WHERE res.implementation_type IN ('tulip_rs_ffi_c', 'tulip_rs_ffi_c_simd_by_options')
GROUP BY
    runs.id, runs.run_timestamp, runs.system_info,
    ind.name, res.stock_symbol, res.data_source, res.input_size
HAVING
    sum(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c' THEN 1 ELSE 0 END) > 0
    AND sum(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c_simd_by_options' THEN 1 ELSE 0 END) > 0
ORDER BY runs.run_timestamp DESC, ind.name, res.stock_symbol;

CREATE VIEW c_ffi_simd_simplified_comparison AS
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
FROM c_ffi_simd_performance_comparison
GROUP BY run_id, benchmark_date, hostname, indicator_name, input_size
ORDER BY benchmark_date DESC, indicator_name;

CREATE VIEW c_ffi_simd_avg_comparison AS
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
FROM c_ffi_simd_simplified_comparison
GROUP BY run_id, benchmark_date, hostname, indicator_name
ORDER BY benchmark_date DESC, indicator_name;

-- ---------------------------------------------------------------------------
-- c_ffi_simd_asset_performance_comparison
-- SIMD-by-assets: one option set, every loaded stock processed together in a
-- single call. Compared against the summed serial cost of running
-- tulip_rs_ffi_c / C_tulip / talib once per stock for that same option set.
-- Mirrors node_simd_asset_performance_comparison.
-- ---------------------------------------------------------------------------
CREATE VIEW c_ffi_simd_asset_performance_comparison AS
SELECT
    runs.id                            AS run_id,
    runs.run_timestamp                 AS benchmark_date,
    (runs.system_info ->> 'hostname')  AS hostname,
    ind.name                           AS indicator_name,
    res.options,
    res.data_source,

    sum(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
             THEN res.mean_time_ns ELSE 0 END)                  AS tulip_total_mean_time_ns,
    sum(CASE WHEN res.implementation_type = 'C_tulip'
             THEN res.mean_time_ns ELSE 0 END)                  AS c_tulip_total_mean_time_ns,
    sum(CASE WHEN res.implementation_type = 'talib'
             THEN res.mean_time_ns ELSE 0 END)                  AS talib_total_mean_time_ns,
    avg(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c_simd_by_assets'
             THEN res.mean_time_ns END)                         AS simd_asset_mean_time_ns,

    round(
        avg(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c_simd_by_assets'
                 THEN res.mean_time_ns END)
        / NULLIF(
            sum(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                     THEN res.mean_time_ns ELSE 0 END),
          0),
    4)                                                           AS simd_asset_to_tulip_ratio,

    round(
        (NULLIF(
            sum(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c'
                     THEN res.mean_time_ns ELSE 0 END),
          0)
        / avg(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c_simd_by_assets'
                   THEN res.mean_time_ns END)) * 100,
    2)                                                           AS simd_asset_vs_tulip_improvement_pct,

    round(
        (NULLIF(
            sum(CASE WHEN res.implementation_type = 'C_tulip'
                     THEN res.mean_time_ns ELSE 0 END),
          0)
        / avg(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c_simd_by_assets'
                   THEN res.mean_time_ns END)) * 100,
    2)                                                           AS simd_asset_vs_c_tulip_improvement_pct,

    round(
        (NULLIF(
            sum(CASE WHEN res.implementation_type = 'talib'
                     THEN res.mean_time_ns ELSE 0 END),
          0)
        / avg(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c_simd_by_assets'
                   THEN res.mean_time_ns END)) * 100,
    2)                                                           AS simd_asset_vs_talib_improvement_pct

FROM benchmark_runs runs
JOIN benchmark_results res ON runs.id = res.run_id
JOIN indicators ind        ON res.indicator_id = ind.id
WHERE res.implementation_type IN ('tulip_rs_ffi_c', 'C_tulip', 'talib', 'tulip_rs_ffi_c_simd_by_assets')
GROUP BY
    runs.id, runs.run_timestamp, runs.system_info,
    ind.name, res.options, res.data_source
HAVING
    avg(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c_simd_by_assets'
             THEN res.mean_time_ns END) IS NOT NULL
    AND (
        sum(CASE WHEN res.implementation_type = 'tulip_rs_ffi_c' THEN 1 ELSE 0 END) > 0
        OR sum(CASE WHEN res.implementation_type = 'C_tulip' THEN 1 ELSE 0 END) > 0
        OR sum(CASE WHEN res.implementation_type = 'talib' THEN 1 ELSE 0 END) > 0
    )
ORDER BY runs.run_timestamp DESC, ind.name, res.options;

CREATE VIEW c_ffi_simd_asset_simplified_comparison AS
SELECT run_id, benchmark_date, hostname, indicator_name, data_source,
    round(avg(tulip_total_mean_time_ns))                        AS tulip_avg_total_time_ns,
    round(avg(c_tulip_total_mean_time_ns))                      AS c_tulip_avg_total_time_ns,
    round(avg(talib_total_mean_time_ns))                        AS talib_avg_total_time_ns,
    round(avg(simd_asset_mean_time_ns))                         AS simd_asset_avg_time_ns,
    round(avg(simd_asset_to_tulip_ratio), 4)                    AS avg_simd_asset_to_tulip_ratio,
    round(avg(simd_asset_vs_tulip_improvement_pct), 2)          AS avg_simd_asset_vs_tulip_improvement_pct,
    round(avg(simd_asset_vs_c_tulip_improvement_pct), 2)        AS avg_simd_asset_vs_c_tulip_improvement_pct,
    round(avg(simd_asset_vs_talib_improvement_pct), 2)          AS avg_simd_asset_vs_talib_improvement_pct
FROM c_ffi_simd_asset_performance_comparison
GROUP BY run_id, benchmark_date, hostname, indicator_name, data_source
ORDER BY benchmark_date DESC, indicator_name;

CREATE VIEW c_ffi_simd_asset_avg_comparison AS
SELECT run_id, benchmark_date, hostname, indicator_name,
    round(avg(tulip_avg_total_time_ns))                         AS tulip_overall_avg_time_ns,
    round(avg(c_tulip_avg_total_time_ns))                       AS c_tulip_overall_avg_time_ns,
    round(avg(talib_avg_total_time_ns))                         AS talib_overall_avg_time_ns,
    round(avg(simd_asset_avg_time_ns))                          AS simd_asset_overall_avg_time_ns,
    round(avg(avg_simd_asset_to_tulip_ratio), 4)                AS overall_simd_asset_to_tulip_ratio,
    round(avg(avg_simd_asset_vs_tulip_improvement_pct), 2)      AS overall_simd_asset_vs_tulip_improvement_pct,
    round(avg(avg_simd_asset_vs_c_tulip_improvement_pct), 2)    AS overall_simd_asset_vs_c_tulip_improvement_pct,
    round(avg(avg_simd_asset_vs_talib_improvement_pct), 2)      AS overall_simd_asset_vs_talib_improvement_pct
FROM c_ffi_simd_asset_simplified_comparison
GROUP BY run_id, benchmark_date, hostname, indicator_name
ORDER BY benchmark_date DESC, indicator_name;

\echo '>>> C/FFI benchmark views ready.'
