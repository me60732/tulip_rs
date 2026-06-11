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

\echo '>>> Node.js benchmark views ready.'
