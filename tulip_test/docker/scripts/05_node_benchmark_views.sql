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
-- =============================================================================

\c indicator_benchmark

\echo '>>> Creating Node.js benchmark views...'

-- Drop in reverse-dependency order so re-running is safe
DROP VIEW IF EXISTS node_avg_options_comparison;
DROP VIEW IF EXISTS node_performance_comparison;

-- ---------------------------------------------------------------------------
-- node_performance_comparison
-- One row per (run, indicator, stock, option-set).
-- Pivots tulip_rs_node and technicalindicators side by side and computes ratio.
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

    max(CASE WHEN res.implementation_type = 'tulip_rs_node'
             THEN res.mean_time_ns END)                          AS tulip_rs_node_mean_ns,
    max(CASE WHEN res.implementation_type = 'tulip_rs_node'
             THEN res.std_dev_ns END)                            AS tulip_rs_node_stddev_ns,
    max(CASE WHEN res.implementation_type = 'technicalindicators'
             THEN res.mean_time_ns END)                          AS ti_mean_ns,
    max(CASE WHEN res.implementation_type = 'technicalindicators'
             THEN res.std_dev_ns END)                            AS ti_stddev_ns,

    -- How many times slower is technicalindicators vs tulip_rs_node?
    round(
        (max(CASE WHEN res.implementation_type = 'technicalindicators'
                  THEN res.mean_time_ns END))::numeric
        / NULLIF(
            (max(CASE WHEN res.implementation_type = 'tulip_rs_node'
                      THEN res.mean_time_ns END))::numeric,
          0),
    2)                                                           AS ti_to_tulip_ratio,

    -- Percentage of time saved by using tulip_rs_node instead of technicalindicators
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
    2)                                                           AS tulip_speedup_pct

FROM benchmark_runs runs
JOIN benchmark_results res ON runs.id = res.run_id
JOIN indicators ind        ON res.indicator_id = ind.id
WHERE res.implementation_type IN ('tulip_rs_node', 'technicalindicators')
GROUP BY
    runs.id, runs.run_timestamp, runs.system_info,
    ind.name, res.stock_symbol, res.data_source, res.input_size, res.options
HAVING count(DISTINCT res.implementation_type) >= 2
ORDER BY runs.run_timestamp DESC, ind.name, res.stock_symbol;

-- ---------------------------------------------------------------------------
-- node_avg_options_comparison
-- One row per (run, indicator) — averaged across all option sets and stocks.
-- Mirrors the style of avg_options_comparison for the Rust benchmarks.
-- ---------------------------------------------------------------------------
CREATE VIEW node_avg_options_comparison AS
SELECT
    runs.id                            AS run_id,
    runs.run_timestamp                 AS benchmark_date,
    (runs.system_info ->> 'hostname')  AS hostname,
    ind.name                           AS indicator_name,

    round(avg(CASE WHEN res.implementation_type = 'tulip_rs_node'
                   THEN res.mean_time_ns END))                   AS tulip_rs_node_avg_ns,
    round(avg(CASE WHEN res.implementation_type = 'technicalindicators'
                   THEN res.mean_time_ns END))                   AS ti_avg_ns,

    count(DISTINCT CASE WHEN res.implementation_type = 'tulip_rs_node'
                        THEN res.options END)                    AS tulip_options_count,
    count(DISTINCT CASE WHEN res.implementation_type = 'technicalindicators'
                        THEN res.options END)                    AS ti_options_count,

    round(
        avg(CASE WHEN res.implementation_type = 'technicalindicators'
                 THEN res.mean_time_ns END)
        / NULLIF(
            avg(CASE WHEN res.implementation_type = 'tulip_rs_node'
                     THEN res.mean_time_ns END),
          0),
    2)                                                           AS ti_to_tulip_ratio,

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
    2)                                                           AS tulip_speedup_pct

FROM benchmark_runs runs
JOIN benchmark_results res ON runs.id = res.run_id
JOIN indicators ind        ON res.indicator_id = ind.id
WHERE res.implementation_type IN ('tulip_rs_node', 'technicalindicators')
GROUP BY runs.id, runs.run_timestamp, runs.system_info, ind.name
HAVING count(DISTINCT res.implementation_type) >= 2
ORDER BY runs.run_timestamp DESC, ind.name;

\echo '>>> Node.js benchmark views ready.'
