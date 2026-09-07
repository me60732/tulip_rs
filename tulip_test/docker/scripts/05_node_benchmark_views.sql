-- =============================================================================
-- 05_node_benchmark_views.sql
-- Adds Node.js-comparison views to the existing indicator_benchmark database.
-- Does NOT recreate the database or touch existing tables/views.
--
-- Applied automatically by Docker on first init.
-- Comment out the volume mount in docker-compose.yaml to skip these views.
--
-- Run manually:
--   psql -U postgres -h localhost -d indicator_benchmark \
--        -f scripts/05_node_benchmark_views.sql
--
-- Implementation types written by the Node.js benchmarks:
--   'tulip_rs_node'         — tulip-rs called via napi-rs Node.js binding
--   'technicalindicators'   — anandanand84/technicalindicators (pure JS/TS)
--   'indicatorts'           — Onur Cinar/indicatorts (pure TS)
--
-- Both comparison views show tulip_rs_node results even when no reference
-- library ran the same indicator (comparison columns will be NULL in that case).
-- =============================================================================

\c indicator_benchmark

\echo '>>> Creating Node.js benchmark views...'

-- Drop in reverse-dependency order so re-running is safe
DROP VIEW IF EXISTS node_simd_asset_avg_comparison;
DROP VIEW IF EXISTS node_simd_asset_simplified_comparison;
DROP VIEW IF EXISTS node_simd_asset_performance_comparison;
DROP VIEW IF EXISTS node_simd_avg_comparison;
DROP VIEW IF EXISTS node_simd_simplified_comparison;
DROP VIEW IF EXISTS node_simd_performance_comparison;
DROP VIEW IF EXISTS node_avg_options_comparison;
DROP VIEW IF EXISTS node_performance_comparison;

-- ---------------------------------------------------------------------------
-- node_performance_comparison
-- One row per (run, indicator, stock, option-set).
-- Pivots tulip_rs_node, technicalindicators, and indicatorts side by side
-- and computes x-faster ratios relative to tulip_rs_node.
-- Rows are included whenever tulip_rs_node has a result; reference columns
-- are NULL when no matching reference run exists for that combination.
-- ---------------------------------------------------------------------------
CREATE VIEW node_performance_comparison AS
SELECT
    runs.id                            AS run_id,
    runs.run_timestamp                 AS benchmark_date,
    (runs.system_info ->> 'hostname')  AS hostname,
    ind.name                           AS indicator_name,
    res.stock_symbol,
    res.data_source,
    res.input_size,
    res.options,

    -- tulip_rs_node
    max(CASE WHEN res.implementation_type = 'tulip_rs_node'
             THEN res.mean_time_ns END)                              AS tulip_rs_node_mean_ns,
    max(CASE WHEN res.implementation_type = 'tulip_rs_node'
             THEN res.std_dev_ns END)                                AS tulip_rs_node_stddev_ns,

    -- technicalindicators
    max(CASE WHEN res.implementation_type = 'technicalindicators'
             THEN res.mean_time_ns END)                              AS ti_mean_ns,
    max(CASE WHEN res.implementation_type = 'technicalindicators'
             THEN res.std_dev_ns END)                                AS ti_stddev_ns,

    -- indicatorts
    max(CASE WHEN res.implementation_type = 'indicatorts'
             THEN res.mean_time_ns END)                              AS indicatorts_mean_ns,
    max(CASE WHEN res.implementation_type = 'indicatorts'
             THEN res.std_dev_ns END)                                AS indicatorts_stddev_ns,

    -- technicalindicators / tulip_rs_node  (> 1 means tulip is faster)
    round(
        (max(CASE WHEN res.implementation_type = 'technicalindicators'
                  THEN res.mean_time_ns END))::numeric
        / NULLIF(
            (max(CASE WHEN res.implementation_type = 'tulip_rs_node'
                      THEN res.mean_time_ns END))::numeric,
          0),
    2)                                                               AS ti_to_tulip_ratio,

    -- indicatorts / tulip_rs_node  (> 1 means tulip is faster)
    round(
        (max(CASE WHEN res.implementation_type = 'indicatorts'
                  THEN res.mean_time_ns END))::numeric
        / NULLIF(
            (max(CASE WHEN res.implementation_type = 'tulip_rs_node'
                      THEN res.mean_time_ns END))::numeric,
          0),
    2)                                                               AS indicatorts_to_tulip_ratio,

    -- % time saved vs technicalindicators (NULL when ti has no result)
    round(
        (
          (max(CASE WHEN res.implementation_type = 'technicalindicators'
                    THEN res.mean_time_ns END)
           - max(CASE WHEN res.implementation_type = 'tulip_rs_node'
                      THEN res.mean_time_ns END))::numeric
          / NULLIF(
              max(CASE WHEN res.implementation_type = 'technicalindicators'
                        THEN res.mean_time_ns END)::numeric,
            0)
        ) * 100,
    2)                                                               AS tulip_speedup_pct_vs_ti,

    -- % time saved vs indicatorts (NULL when indicatorts has no result)
    round(
        (
          (max(CASE WHEN res.implementation_type = 'indicatorts'
                    THEN res.mean_time_ns END)
           - max(CASE WHEN res.implementation_type = 'tulip_rs_node'
                      THEN res.mean_time_ns END))::numeric
          / NULLIF(
              max(CASE WHEN res.implementation_type = 'indicatorts'
                        THEN res.mean_time_ns END)::numeric,
            0)
        ) * 100,
    2)                                                               AS tulip_speedup_pct_vs_indicatorts

FROM benchmark_runs runs
JOIN benchmark_results res ON runs.id = res.run_id
JOIN indicators ind        ON res.indicator_id = ind.id
WHERE res.implementation_type IN ('tulip_rs_node', 'technicalindicators', 'indicatorts')
GROUP BY
    runs.id, runs.run_timestamp, runs.system_info,
    ind.name, res.stock_symbol, res.data_source, res.input_size, res.options
-- Require tulip_rs_node to be present; reference libraries are optional.
HAVING max(CASE WHEN res.implementation_type = 'tulip_rs_node' THEN 1 END) = 1
ORDER BY runs.run_timestamp DESC, ind.name, res.stock_symbol;

-- ---------------------------------------------------------------------------
-- node_avg_options_comparison
-- One row per (run, indicator) — averaged across all option sets and stocks.
-- Includes all indicators that have a tulip_rs_node result; reference columns
-- are NULL when no matching reference run exists for that indicator.
-- ---------------------------------------------------------------------------
CREATE VIEW node_avg_options_comparison AS
SELECT
    runs.id                            AS run_id,
    runs.run_timestamp                 AS benchmark_date,
    (runs.system_info ->> 'hostname')  AS hostname,
    ind.name                           AS indicator_name,

    -- tulip_rs_node
    round(avg(CASE WHEN res.implementation_type = 'tulip_rs_node'
                   THEN res.mean_time_ns END))                       AS tulip_rs_node_avg_ns,
    count(DISTINCT CASE WHEN res.implementation_type = 'tulip_rs_node'
                        THEN res.options END)                        AS tulip_options_count,

    -- technicalindicators
    round(avg(CASE WHEN res.implementation_type = 'technicalindicators'
                   THEN res.mean_time_ns END))                       AS ti_avg_ns,
    count(DISTINCT CASE WHEN res.implementation_type = 'technicalindicators'
                        THEN res.options END)                        AS ti_options_count,

    -- indicatorts
    round(avg(CASE WHEN res.implementation_type = 'indicatorts'
                   THEN res.mean_time_ns END))                       AS indicatorts_avg_ns,
    count(DISTINCT CASE WHEN res.implementation_type = 'indicatorts'
                        THEN res.options END)                        AS indicatorts_options_count,

    -- technicalindicators / tulip_rs_node
    round(
        avg(CASE WHEN res.implementation_type = 'technicalindicators'
                 THEN res.mean_time_ns END)
        / NULLIF(
            avg(CASE WHEN res.implementation_type = 'tulip_rs_node'
                     THEN res.mean_time_ns END),
          0),
    2)                                                               AS ti_to_tulip_ratio,

    -- indicatorts / tulip_rs_node
    round(
        avg(CASE WHEN res.implementation_type = 'indicatorts'
                 THEN res.mean_time_ns END)
        / NULLIF(
            avg(CASE WHEN res.implementation_type = 'tulip_rs_node'
                     THEN res.mean_time_ns END),
          0),
    2)                                                               AS indicatorts_to_tulip_ratio,

    -- % time saved vs technicalindicators
    round(
        (
          avg(CASE WHEN res.implementation_type = 'technicalindicators'
                   THEN res.mean_time_ns END)
          - avg(CASE WHEN res.implementation_type = 'tulip_rs_node'
                     THEN res.mean_time_ns END)
        )
        / NULLIF(
            avg(CASE WHEN res.implementation_type = 'technicalindicators'
                     THEN res.mean_time_ns END),
          0) * 100,
    2)                                                               AS tulip_speedup_pct_vs_ti,

    -- % time saved vs indicatorts
    round(
        (
          avg(CASE WHEN res.implementation_type = 'indicatorts'
                   THEN res.mean_time_ns END)
          - avg(CASE WHEN res.implementation_type = 'tulip_rs_node'
                     THEN res.mean_time_ns END)
        )
        / NULLIF(
            avg(CASE WHEN res.implementation_type = 'indicatorts'
                     THEN res.mean_time_ns END),
          0) * 100,
    2)                                                               AS tulip_speedup_pct_vs_indicatorts

FROM benchmark_runs runs
JOIN benchmark_results res ON runs.id = res.run_id
JOIN indicators ind        ON res.indicator_id = ind.id
WHERE res.implementation_type IN ('tulip_rs_node', 'technicalindicators', 'indicatorts')
GROUP BY runs.id, runs.run_timestamp, runs.system_info, ind.name
-- Require tulip_rs_node to be present; reference libraries are optional.
HAVING max(CASE WHEN res.implementation_type = 'tulip_rs_node' THEN 1 END) = 1
ORDER BY runs.run_timestamp DESC, ind.name;

-- =============================================================================
-- SIMD comparison views
-- Compares the tulip_rs_node SIMD-batched code paths (written by the bench
-- runner as 'tulip_rs_node_simd_by_options' / '..._simd_by_assets') against
-- the plain 'tulip_rs_node' baseline (and, for the by-assets case, also
-- against 'technicalindicators' / 'indicatorts'). Mirrors the Rust
-- rust_simd_* view chain:
--   *_performance_comparison  (one row per run/indicator/stock or option-set)
--   *_simplified_comparison   (averaged per run/indicator/input_size or data_source)
--   *_avg_comparison          (averaged per run/indicator - the top-level view)
-- =============================================================================

-- ---------------------------------------------------------------------------
-- node_simd_performance_comparison
-- SIMD-by-options: one stock, every option set processed together in a
-- single call. Compared against the summed serial cost of running
-- tulip_rs_node once per option set for that same stock.
-- ---------------------------------------------------------------------------
CREATE VIEW node_simd_performance_comparison AS
SELECT
    runs.id                            AS run_id,
    runs.run_timestamp                 AS benchmark_date,
    (runs.system_info ->> 'hostname')  AS hostname,
    ind.name                           AS indicator_name,
    res.stock_symbol,
    res.data_source,
    res.input_size,

    sum(CASE WHEN res.implementation_type = 'tulip_rs_node'
             THEN res.mean_time_ns END)                          AS tulip_total_mean_time_ns,
    count(CASE WHEN res.implementation_type = 'tulip_rs_node'
               THEN 1 END)                                       AS tulip_options_count,
    max(CASE WHEN res.implementation_type = 'tulip_rs_node_simd_by_options'
             THEN res.mean_time_ns END)                          AS simd_mean_time_ns,
    max(CASE WHEN res.implementation_type = 'tulip_rs_node_simd_by_options'
             THEN res.sample_count END)                          AS simd_sample_count,

    round(
        (max(CASE WHEN res.implementation_type = 'tulip_rs_node_simd_by_options'
                  THEN res.mean_time_ns END))::numeric
        / NULLIF(
            sum(CASE WHEN res.implementation_type = 'tulip_rs_node'
                     THEN res.mean_time_ns END),
          0),
    4)                                                           AS simd_to_tulip_ratio,

    round(
        (sum(CASE WHEN res.implementation_type = 'tulip_rs_node'
                 THEN res.mean_time_ns END))::numeric
        / NULLIF(
            (max(CASE WHEN res.implementation_type = 'tulip_rs_node_simd_by_options'
                      THEN res.mean_time_ns END))::numeric,
          0) * 100,
    2)                                                           AS simd_vs_tulip_improvement_pct,

    round(
        (sum(CASE WHEN res.implementation_type = 'tulip_rs_node'
                 THEN res.mean_time_ns END))::numeric
        / NULLIF(
            (max(CASE WHEN res.implementation_type = 'tulip_rs_node_simd_by_options'
                      THEN res.mean_time_ns END))::numeric,
          0),
    2)                                                           AS simd_speedup_factor

FROM benchmark_runs runs
JOIN benchmark_results res ON runs.id = res.run_id
JOIN indicators ind        ON res.indicator_id = ind.id
WHERE res.implementation_type IN ('tulip_rs_node', 'tulip_rs_node_simd_by_options')
GROUP BY
    runs.id, runs.run_timestamp, runs.system_info,
    ind.name, res.stock_symbol, res.data_source, res.input_size
HAVING
    sum(CASE WHEN res.implementation_type = 'tulip_rs_node' THEN 1 ELSE 0 END) > 0
    AND sum(CASE WHEN res.implementation_type = 'tulip_rs_node_simd_by_options' THEN 1 ELSE 0 END) > 0
ORDER BY runs.run_timestamp DESC, ind.name, res.stock_symbol;

CREATE VIEW node_simd_simplified_comparison AS
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
FROM node_simd_performance_comparison
GROUP BY run_id, benchmark_date, hostname, indicator_name, input_size
ORDER BY benchmark_date DESC, indicator_name;

CREATE VIEW node_simd_avg_comparison AS
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
FROM node_simd_simplified_comparison
GROUP BY run_id, benchmark_date, hostname, indicator_name
ORDER BY benchmark_date DESC, indicator_name;

-- ---------------------------------------------------------------------------
-- node_simd_asset_performance_comparison
-- SIMD-by-assets: one option set, every loaded stock processed together in a
-- single call. Compared against the summed serial cost of running
-- tulip_rs_node / technicalindicators / indicatorts once per stock for that
-- same option set.
-- ---------------------------------------------------------------------------
CREATE VIEW node_simd_asset_performance_comparison AS
SELECT
    runs.id                            AS run_id,
    runs.run_timestamp                 AS benchmark_date,
    (runs.system_info ->> 'hostname')  AS hostname,
    ind.name                           AS indicator_name,
    res.options,
    res.data_source,

    sum(CASE WHEN res.implementation_type = 'tulip_rs_node'
             THEN res.mean_time_ns ELSE 0 END)                  AS tulip_total_mean_time_ns,
    sum(CASE WHEN res.implementation_type = 'technicalindicators'
             THEN res.mean_time_ns ELSE 0 END)                  AS ti_total_mean_time_ns,
    sum(CASE WHEN res.implementation_type = 'indicatorts'
             THEN res.mean_time_ns ELSE 0 END)                  AS indicatorts_total_mean_time_ns,
    avg(CASE WHEN res.implementation_type = 'tulip_rs_node_simd_by_assets'
             THEN res.mean_time_ns END)                         AS simd_asset_mean_time_ns,

    round(
        avg(CASE WHEN res.implementation_type = 'tulip_rs_node_simd_by_assets'
                 THEN res.mean_time_ns END)
        / NULLIF(
            sum(CASE WHEN res.implementation_type = 'tulip_rs_node'
                     THEN res.mean_time_ns ELSE 0 END),
          0),
    4)                                                           AS simd_asset_to_tulip_ratio,

    round(
        (NULLIF(
            sum(CASE WHEN res.implementation_type = 'tulip_rs_node'
                     THEN res.mean_time_ns ELSE 0 END),
          0)
        / avg(CASE WHEN res.implementation_type = 'tulip_rs_node_simd_by_assets'
                   THEN res.mean_time_ns END)) * 100,
    2)                                                           AS simd_asset_vs_tulip_improvement_pct,

    round(
        (NULLIF(
            sum(CASE WHEN res.implementation_type = 'technicalindicators'
                     THEN res.mean_time_ns ELSE 0 END),
          0)
        / avg(CASE WHEN res.implementation_type = 'tulip_rs_node_simd_by_assets'
                   THEN res.mean_time_ns END)) * 100,
    2)                                                           AS simd_asset_vs_ti_improvement_pct,

    round(
        (NULLIF(
            sum(CASE WHEN res.implementation_type = 'indicatorts'
                     THEN res.mean_time_ns ELSE 0 END),
          0)
        / avg(CASE WHEN res.implementation_type = 'tulip_rs_node_simd_by_assets'
                   THEN res.mean_time_ns END)) * 100,
    2)                                                           AS simd_asset_vs_indicatorts_improvement_pct

FROM benchmark_runs runs
JOIN benchmark_results res ON runs.id = res.run_id
JOIN indicators ind        ON res.indicator_id = ind.id
WHERE res.implementation_type IN ('tulip_rs_node', 'technicalindicators', 'indicatorts', 'tulip_rs_node_simd_by_assets')
GROUP BY
    runs.id, runs.run_timestamp, runs.system_info,
    ind.name, res.options, res.data_source
HAVING
    avg(CASE WHEN res.implementation_type = 'tulip_rs_node_simd_by_assets'
             THEN res.mean_time_ns END) IS NOT NULL
    AND (
        sum(CASE WHEN res.implementation_type = 'tulip_rs_node' THEN 1 ELSE 0 END) > 0
        OR sum(CASE WHEN res.implementation_type = 'technicalindicators' THEN 1 ELSE 0 END) > 0
        OR sum(CASE WHEN res.implementation_type = 'indicatorts' THEN 1 ELSE 0 END) > 0
    )
ORDER BY runs.run_timestamp DESC, ind.name, res.options;

CREATE VIEW node_simd_asset_simplified_comparison AS
SELECT run_id, benchmark_date, hostname, indicator_name, data_source,
    round(avg(tulip_total_mean_time_ns))                        AS tulip_avg_total_time_ns,
    round(avg(ti_total_mean_time_ns))                           AS ti_avg_total_time_ns,
    round(avg(indicatorts_total_mean_time_ns))                  AS indicatorts_avg_total_time_ns,
    round(avg(simd_asset_mean_time_ns))                         AS simd_asset_avg_time_ns,
    round(avg(simd_asset_to_tulip_ratio), 4)                    AS avg_simd_asset_to_tulip_ratio,
    round(avg(simd_asset_vs_tulip_improvement_pct), 2)          AS avg_simd_asset_vs_tulip_improvement_pct,
    round(avg(simd_asset_vs_ti_improvement_pct), 2)             AS avg_simd_asset_vs_ti_improvement_pct,
    round(avg(simd_asset_vs_indicatorts_improvement_pct), 2)    AS avg_simd_asset_vs_indicatorts_improvement_pct
FROM node_simd_asset_performance_comparison
GROUP BY run_id, benchmark_date, hostname, indicator_name, data_source
ORDER BY benchmark_date DESC, indicator_name;

CREATE VIEW node_simd_asset_avg_comparison AS
SELECT run_id, benchmark_date, hostname, indicator_name,
    round(avg(tulip_avg_total_time_ns))                         AS tulip_overall_avg_time_ns,
    round(avg(ti_avg_total_time_ns))                            AS ti_overall_avg_time_ns,
    round(avg(indicatorts_avg_total_time_ns))                   AS indicatorts_overall_avg_time_ns,
    round(avg(simd_asset_avg_time_ns))                          AS simd_asset_overall_avg_time_ns,
    round(avg(avg_simd_asset_to_tulip_ratio), 4)                AS overall_simd_asset_to_tulip_ratio,
    round(avg(avg_simd_asset_vs_tulip_improvement_pct), 2)      AS overall_simd_asset_vs_tulip_improvement_pct,
    round(avg(avg_simd_asset_vs_ti_improvement_pct), 2)         AS overall_simd_asset_vs_ti_improvement_pct,
    round(avg(avg_simd_asset_vs_indicatorts_improvement_pct), 2) AS overall_simd_asset_vs_indicatorts_improvement_pct
FROM node_simd_asset_simplified_comparison
GROUP BY run_id, benchmark_date, hostname, indicator_name
ORDER BY benchmark_date DESC, indicator_name;

\echo '>>> Node.js benchmark views ready.'
