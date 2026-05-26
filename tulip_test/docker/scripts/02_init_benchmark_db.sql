-- =============================================================================
-- init_benchmark_db.sql
-- Benchmark database: full schema + all views + indicator seed data.
-- No benchmark_runs or benchmark_results data is included.
--
-- Run as:
--   psql -U postgres -h <host> -d postgres -f scripts/init_benchmark_db.sql
-- =============================================================================

\echo '>>> Dropping and recreating indicator_benchmark database...'
DROP DATABASE IF EXISTS indicator_benchmark;
CREATE DATABASE indicator_benchmark;

-- Create tulip user at cluster level if it does not already exist
DO $$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'tulip') THEN
    CREATE USER tulip WITH PASSWORD 'tulip';
  END IF;
END
$$;

\c indicator_benchmark

-- Grant privileges to tulip on indicator_benchmark
GRANT CONNECT ON DATABASE indicator_benchmark TO tulip;
GRANT USAGE ON SCHEMA public TO tulip;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO tulip;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO tulip;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO tulip;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO tulip;

-- ---------------------------------------------------------------------------
-- Sequences
-- ---------------------------------------------------------------------------

CREATE SEQUENCE benchmark_results_id_seq START 1;
CREATE SEQUENCE benchmark_runs_id_seq    START 1;
CREATE SEQUENCE indicators_id_seq        START 1;

-- ---------------------------------------------------------------------------
-- Base tables
-- ---------------------------------------------------------------------------

CREATE TABLE indicators (
    id          INTEGER   NOT NULL DEFAULT nextval('indicators_id_seq'),
    name        VARCHAR   NOT NULL,
    description TEXT,
    input_count INTEGER   NOT NULL,
    output_count INTEGER  NOT NULL,
    has_options BOOLEAN   DEFAULT true,
    category    VARCHAR,
    created_at  TIMESTAMPTZ DEFAULT now(),
    CONSTRAINT indicators_pkey PRIMARY KEY (id),
    CONSTRAINT indicators_name_key UNIQUE (name)
);

CREATE INDEX idx_indicators_category ON indicators (category);
CREATE INDEX idx_indicators_name     ON indicators (name);

CREATE TABLE benchmark_runs (
    id            INTEGER     NOT NULL DEFAULT nextval('benchmark_runs_id_seq'),
    run_timestamp TIMESTAMPTZ DEFAULT now(),
    rust_version  VARCHAR,
    system_info   JSONB,
    notes         TEXT,
    CONSTRAINT benchmark_runs_pkey PRIMARY KEY (id)
);

CREATE TABLE benchmark_results (
    id                  INTEGER     NOT NULL DEFAULT nextval('benchmark_results_id_seq'),
    run_id              INTEGER,
    indicator_id        INTEGER,
    implementation_type VARCHAR     NOT NULL,
    stock_symbol        VARCHAR,
    data_source         VARCHAR     NOT NULL,
    options             JSONB,
    mean_time_ns        BIGINT,
    std_dev_ns          BIGINT,
    min_time_ns         BIGINT,
    max_time_ns         BIGINT,
    sample_count        INTEGER,
    input_size          INTEGER,
    created_at          TIMESTAMPTZ DEFAULT now(),
    CONSTRAINT benchmark_results_pkey PRIMARY KEY (id),
    CONSTRAINT benchmark_results_run_id_fkey FOREIGN KEY (run_id)
        REFERENCES benchmark_runs (id),
    CONSTRAINT benchmark_results_indicator_id_fkey FOREIGN KEY (indicator_id)
        REFERENCES indicators (id)
);

CREATE INDEX idx_benchmark_results_run_id    ON benchmark_results (run_id);
CREATE INDEX idx_benchmark_results_indicator ON benchmark_results (indicator_id);
CREATE INDEX idx_benchmark_results_impl_type ON benchmark_results (implementation_type);
CREATE INDEX idx_benchmark_results_stock     ON benchmark_results (stock_symbol);
CREATE INDEX idx_benchmark_results_options   ON benchmark_results USING GIN (options);

-- ---------------------------------------------------------------------------
-- Views — created in dependency order
-- ---------------------------------------------------------------------------

-- Level 1: reference only base tables

CREATE VIEW avg_options_comparison AS
SELECT runs.id AS run_id,
    runs.run_timestamp AS benchmark_date,
    (runs.system_info ->> 'hostname'::text) AS hostname,
    ind.name AS indicator_name,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS rust_avg_mean_time_ns,
    count(DISTINCT CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.options ELSE NULL::jsonb END) AS rust_options_count,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS c_avg_mean_time_ns,
    count(DISTINCT CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.options ELSE NULL::jsonb END) AS c_options_count,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS talib_avg_mean_time_ns,
    count(DISTINCT CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.options ELSE NULL::jsonb END) AS talib_options_count,
    round((avg(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END) / avg(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END)), 2) AS c_to_rust_ratio,
    round((((avg(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END) - avg(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) / avg(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) * (100)::numeric), 2) AS rust_vs_c_percent_diff,
    round((avg(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.mean_time_ns ELSE NULL::bigint END) / avg(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END)), 2) AS talib_to_rust_ratio,
    round((((avg(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END) - avg(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) / avg(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) * (100)::numeric), 2) AS rust_vs_talib_percent_diff
   FROM ((benchmark_runs runs
     JOIN benchmark_results results ON ((runs.id = results.run_id)))
     JOIN indicators ind ON ((results.indicator_id = ind.id)))
  GROUP BY runs.id, runs.run_timestamp, runs.system_info, ind.name
 HAVING (count(DISTINCT CASE WHEN ((results.implementation_type)::text = ANY ((ARRAY['C_tulip'::character varying, 'Rust'::character varying, 'talib'::character varying])::text[])) THEN results.implementation_type ELSE NULL::character varying END) >= 2)
  ORDER BY runs.run_timestamp DESC, ind.name;

CREATE VIEW candlestick_simplified_comparison AS
SELECT runs.id AS run_id,
    runs.run_timestamp AS benchmark_date,
    (runs.system_info ->> 'hostname'::text) AS hostname,
    br.implementation_type AS forecast_name,
    br.input_size,
    br.options,
    round(avg(br.mean_time_ns)) AS avg_mean_time_ns,
    count(*) AS symbol_count,
    round(avg(br.std_dev_ns)) AS avg_std_dev_ns,
    min(br.min_time_ns) AS overall_min_time_ns,
    max(br.max_time_ns) AS overall_max_time_ns
   FROM ((benchmark_runs runs
     JOIN benchmark_results br ON ((runs.id = br.run_id)))
     JOIN indicators ind ON ((br.indicator_id = ind.id)))
  WHERE ((ind.name)::text = 'Rust_Candlestick'::text)
  GROUP BY runs.id, runs.run_timestamp, runs.system_info, br.implementation_type, br.input_size, br.options
  ORDER BY runs.run_timestamp DESC, br.implementation_type;

CREATE VIEW indicator_summary AS
SELECT i.name,
    i.description,
    i.category,
    i.input_count,
    i.output_count,
    i.has_options,
    count(br.id) AS benchmark_count,
    max(br.created_at) AS last_benchmarked
   FROM (indicators i
     LEFT JOIN benchmark_results br ON ((i.id = br.indicator_id)))
  GROUP BY i.id, i.name, i.description, i.category, i.input_count, i.output_count, i.has_options
  ORDER BY i.category, i.name;

CREATE VIEW performance_comparison AS
SELECT runs.id AS run_id,
    runs.run_timestamp AS benchmark_date,
    (runs.system_info ->> 'hostname'::text) AS hostname,
    ind.name AS indicator_name,
    results.stock_symbol,
    results.data_source,
    results.input_size,
    results.options,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END) AS rust_mean_time_ns,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.sample_count ELSE NULL::integer END) AS rust_sample_count,
    max(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END) AS c_mean_time_ns,
    max(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.sample_count ELSE NULL::integer END) AS c_sample_count,
    max(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.mean_time_ns ELSE NULL::bigint END) AS talib_mean_time_ns,
    max(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.sample_count ELSE NULL::integer END) AS talib_sample_count,
    round(((max(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric / (max(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric), 2) AS c_to_rust_ratio,
    round(((((max(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric - (max(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric) / (max(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric) * (100)::numeric), 2) AS rust_vs_c_percent_diff,
    round(((max(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric / (max(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric), 2) AS talib_to_rust_ratio,
    round(((((max(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric - (max(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric) / (max(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric) * (100)::numeric), 2) AS rust_vs_talib_percent_diff
   FROM ((benchmark_runs runs
     JOIN benchmark_results results ON ((runs.id = results.run_id)))
     JOIN indicators ind ON ((results.indicator_id = ind.id)))
  GROUP BY runs.id, runs.run_timestamp, runs.system_info, ind.name, results.stock_symbol, results.data_source, results.input_size, results.options
 HAVING (count(DISTINCT CASE WHEN ((results.implementation_type)::text = ANY ((ARRAY['C_tulip'::character varying, 'Rust'::character varying, 'talib'::character varying])::text[])) THEN results.implementation_type ELSE NULL::character varying END) >= 2)
  ORDER BY runs.run_timestamp DESC, ind.name, results.stock_symbol;

CREATE VIEW simplified_comparison AS
SELECT runs.id AS run_id,
    runs.run_timestamp AS benchmark_date,
    (runs.system_info ->> 'hostname'::text) AS hostname,
    ind.name AS indicator_name,
    results.input_size,
    results.options,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS rust_avg_mean_time_ns,
    count(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN 1 ELSE NULL::integer END) AS rust_symbol_count,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS c_avg_mean_time_ns,
    count(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN 1 ELSE NULL::integer END) AS c_symbol_count,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS talib_avg_mean_time_ns,
    count(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN 1 ELSE NULL::integer END) AS talib_symbol_count,
    round((avg(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END) / avg(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END)), 2) AS c_to_rust_ratio,
    round((((avg(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END) - avg(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) / avg(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) * (100)::numeric), 2) AS rust_vs_c_percent_diff,
    round((avg(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.mean_time_ns ELSE NULL::bigint END) / avg(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END)), 2) AS talib_to_rust_ratio,
    round((((avg(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END) - avg(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) / avg(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) * (100)::numeric), 2) AS rust_vs_talib_percent_diff
   FROM ((benchmark_runs runs
     JOIN benchmark_results results ON ((runs.id = results.run_id)))
     JOIN indicators ind ON ((results.indicator_id = ind.id)))
  GROUP BY runs.id, runs.run_timestamp, runs.system_info, ind.name, results.input_size, results.options
 HAVING (count(DISTINCT CASE WHEN ((results.implementation_type)::text = ANY ((ARRAY['C_tulip'::character varying, 'Rust'::character varying, 'talib'::character varying])::text[])) THEN results.implementation_type ELSE NULL::character varying END) >= 2)
  ORDER BY runs.run_timestamp DESC, ind.name;

CREATE VIEW rust_impl_avg_options_comparison AS
SELECT runs.id AS run_id,
    runs.run_timestamp AS benchmark_date,
    (runs.system_info ->> 'hostname'::text) AS hostname,
    ind.name AS indicator_name,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS rust_avg_mean_time_ns,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS rust_fromstate_avg_mean_time_ns,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'Rust_optional'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS rust_optional_avg_mean_time_ns,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState_1_Bar'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS rust_fromstate_1_bar_avg_mean_time_ns,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState_1_Bar_json'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS rust_fromstate_1_bar_json_avg_mean_time_ns
   FROM ((benchmark_runs runs
     JOIN benchmark_results results ON ((runs.id = results.run_id)))
     JOIN indicators ind ON ((results.indicator_id = ind.id)))
  WHERE ((ind.category)::text <> 'candlestick'::text)
  GROUP BY runs.id, runs.run_timestamp, runs.system_info, ind.name
 HAVING (count(DISTINCT CASE WHEN ((results.implementation_type)::text = ANY ((ARRAY['Rust'::character varying, 'Rust_FromState'::character varying, 'Rust_optional'::character varying, 'Rust_FromState_1_Bar'::character varying, 'Rust_FromState_1_Bar_json'::character varying])::text[])) THEN results.implementation_type ELSE NULL::character varying END) >= 1)
  ORDER BY runs.run_timestamp DESC, ind.name;

CREATE VIEW rust_impl_performance_comparison AS
SELECT runs.id AS run_id,
    runs.run_timestamp AS benchmark_date,
    (runs.system_info ->> 'hostname'::text) AS hostname,
    ind.name AS indicator_name,
    results.stock_symbol,
    results.data_source,
    results.input_size,
    results.options,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END) AS rust_mean_time_ns,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.sample_count ELSE NULL::integer END) AS rust_sample_count,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState'::text) THEN results.mean_time_ns ELSE NULL::bigint END) AS rust_fromstate_mean_time_ns,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState'::text) THEN results.sample_count ELSE NULL::integer END) AS rust_fromstate_sample_count,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust_optional'::text) THEN results.mean_time_ns ELSE NULL::bigint END) AS rust_optional_mean_time_ns,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust_optional'::text) THEN results.sample_count ELSE NULL::integer END) AS rust_optional_sample_count,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState_1_Bar'::text) THEN results.mean_time_ns ELSE NULL::bigint END) AS rust_fromstate_1_bar_mean_time_ns,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState_1_Bar'::text) THEN results.sample_count ELSE NULL::integer END) AS rust_fromstate_1_bar_sample_count,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState_1_Bar_json'::text) THEN results.mean_time_ns ELSE NULL::bigint END) AS rust_fromstate_1_bar_json_mean_time_ns,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState_1_Bar_json'::text) THEN results.sample_count ELSE NULL::integer END) AS rust_fromstate_1_bar_json_sample_count
   FROM ((benchmark_runs runs
     JOIN benchmark_results results ON ((runs.id = results.run_id)))
     JOIN indicators ind ON ((results.indicator_id = ind.id)))
  WHERE ((ind.category)::text <> 'candlestick'::text)
  GROUP BY runs.id, runs.run_timestamp, runs.system_info, ind.name, results.stock_symbol, results.data_source, results.input_size, results.options
 HAVING (count(DISTINCT CASE WHEN ((results.implementation_type)::text = ANY ((ARRAY['Rust'::character varying, 'Rust_FromState'::character varying, 'Rust_optional'::character varying, 'Rust_FromState_1_Bar'::character varying, 'Rust_FromState_1_Bar_json'::character varying])::text[])) THEN results.implementation_type ELSE NULL::character varying END) >= 1)
  ORDER BY runs.run_timestamp DESC, ind.name, results.stock_symbol;

CREATE VIEW rust_impl_simplified_comparison AS
SELECT runs.id AS run_id,
    runs.run_timestamp AS benchmark_date,
    (runs.system_info ->> 'hostname'::text) AS hostname,
    ind.name AS indicator_name,
    results.input_size,
    results.options,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS rust_avg_mean_time_ns,
    count(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN 1 ELSE NULL::integer END) AS rust_symbol_count,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS rust_fromstate_avg_mean_time_ns,
    count(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState'::text) THEN 1 ELSE NULL::integer END) AS rust_fromstate_symbol_count,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'Rust_optional'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS rust_optional_avg_mean_time_ns,
    count(CASE WHEN ((results.implementation_type)::text = 'Rust_optional'::text) THEN 1 ELSE NULL::integer END) AS rust_optional_symbol_count,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState_1_Bar'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS rust_fromstate_1_bar_avg_mean_time_ns,
    count(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState_1_Bar'::text) THEN 1 ELSE NULL::integer END) AS rust_fromstate_1_bar_symbol_count,
    round(avg(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState_1_Bar_json'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) AS rust_fromstate_1_bar_json_avg_mean_time_ns,
    count(CASE WHEN ((results.implementation_type)::text = 'Rust_FromState_1_Bar_json'::text) THEN 1 ELSE NULL::integer END) AS rust_fromstate_1_bar_json_symbol_count
   FROM ((benchmark_runs runs
     JOIN benchmark_results results ON ((runs.id = results.run_id)))
     JOIN indicators ind ON ((results.indicator_id = ind.id)))
  WHERE ((ind.category)::text <> 'candlestick'::text)
  GROUP BY runs.id, runs.run_timestamp, runs.system_info, ind.name, results.input_size, results.options
 HAVING (count(DISTINCT CASE WHEN ((results.implementation_type)::text = ANY ((ARRAY['Rust'::character varying, 'Rust_FromState'::character varying, 'Rust_optional'::character varying, 'Rust_FromState_1_Bar'::character varying, 'Rust_FromState_1_Bar_json'::character varying])::text[])) THEN results.implementation_type ELSE NULL::character varying END) >= 1)
  ORDER BY runs.run_timestamp DESC, ind.name;

CREATE VIEW rust_simd_performance_comparison AS
SELECT runs.id AS run_id,
    runs.run_timestamp AS benchmark_date,
    (runs.system_info ->> 'hostname'::text) AS hostname,
    ind.name AS indicator_name,
    results.stock_symbol,
    results.data_source,
    results.input_size,
    sum(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END) AS rust_total_mean_time_ns,
    count(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN 1 ELSE NULL::integer END) AS rust_options_count,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD'::text) THEN results.mean_time_ns ELSE NULL::bigint END) AS rust_simd_mean_time_ns,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD'::text) THEN results.sample_count ELSE NULL::integer END) AS rust_simd_sample_count,
    round(((max(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric / sum(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END)), 4) AS simd_to_rust_ratio,
    round(((sum(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END) / (max(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric) * (100)::numeric), 2) AS simd_vs_rust_percent_improvement,
    round((sum(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE NULL::bigint END) / (max(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric), 2) AS simd_speedup_factor
   FROM ((benchmark_runs runs
     JOIN benchmark_results results ON ((runs.id = results.run_id)))
     JOIN indicators ind ON ((results.indicator_id = ind.id)))
  WHERE ((results.implementation_type)::text = ANY ((ARRAY['Rust'::character varying, 'Rust_SIMD'::character varying])::text[]))
  GROUP BY runs.id, runs.run_timestamp, runs.system_info, ind.name, results.stock_symbol, results.data_source, results.input_size
 HAVING ((sum(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN 1 ELSE 0 END) > 0) AND (sum(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD'::text) THEN 1 ELSE 0 END) > 0))
  ORDER BY runs.run_timestamp DESC, ind.name, results.stock_symbol;

CREATE VIEW rust_simd_asset_performance_comparison AS
SELECT runs.id AS run_id,
    runs.run_timestamp AS benchmark_date,
    (runs.system_info ->> 'hostname'::text) AS hostname,
    ind.name AS indicator_name,
    results.options,
    results.data_source,
    sum(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE (0)::bigint END) AS rust_total_mean_time_ns,
    sum(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE (0)::bigint END) AS c_tulip_total_mean_time_ns,
    sum(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.mean_time_ns ELSE (0)::bigint END) AS talib_total_mean_time_ns,
    avg(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD_by_assets'::text) THEN results.mean_time_ns ELSE NULL::bigint END) AS rust_simd_asset_mean_time_ns,
    round((avg(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD_by_assets'::text) THEN results.mean_time_ns ELSE NULL::bigint END) / NULLIF(sum(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE (0)::bigint END), (0)::numeric)), 4) AS simd_asset_to_rust_ratio,
    round(((NULLIF(sum(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN results.mean_time_ns ELSE (0)::bigint END), (0)::numeric) / avg(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD_by_assets'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) * (100)::numeric), 2) AS simd_asset_vs_rust_percent_improvement,
    round(((NULLIF(sum(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE (0)::bigint END), (0)::numeric) / avg(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD_by_assets'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) * (100)::numeric), 2) AS simd_asset_vs_c_tulip_percent_improvement,
    round(((NULLIF(sum(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN results.mean_time_ns ELSE (0)::bigint END), (0)::numeric) / avg(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD_by_assets'::text) THEN results.mean_time_ns ELSE NULL::bigint END)) * (100)::numeric), 2) AS simd_asset_vs_talib_percent_improvement
   FROM ((benchmark_runs runs
     JOIN benchmark_results results ON ((runs.id = results.run_id)))
     JOIN indicators ind ON ((results.indicator_id = ind.id)))
  WHERE ((results.implementation_type)::text = ANY ((ARRAY['Rust'::character varying, 'C_tulip'::character varying, 'talib'::character varying, 'Rust_SIMD_by_assets'::character varying])::text[]))
  GROUP BY runs.id, runs.run_timestamp, runs.system_info, ind.name, results.options, results.data_source
 HAVING ((avg(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD_by_assets'::text) THEN results.mean_time_ns ELSE NULL::bigint END) IS NOT NULL) AND ((sum(CASE WHEN ((results.implementation_type)::text = 'Rust'::text) THEN 1 ELSE 0 END) > 0) OR (sum(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN 1 ELSE 0 END) > 0) OR (sum(CASE WHEN ((results.implementation_type)::text = 'talib'::text) THEN 1 ELSE 0 END) > 0)))
  ORDER BY runs.run_timestamp DESC, ind.name, results.options;

CREATE VIEW rust_simd_c_tulip_performance_comparison AS
SELECT runs.id AS run_id,
    runs.run_timestamp AS benchmark_date,
    (runs.system_info ->> 'hostname'::text) AS hostname,
    ind.name AS indicator_name,
    results.stock_symbol,
    results.data_source,
    results.input_size,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD'::text) THEN results.mean_time_ns ELSE NULL::bigint END) AS rust_simd_mean_time_ns,
    max(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD'::text) THEN results.sample_count ELSE NULL::integer END) AS rust_simd_sample_count,
    sum(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END) AS c_tulip_total_mean_time_ns,
    count(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN 1 ELSE NULL::integer END) AS c_tulip_options_count,
    round((sum(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END) / (max(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric), 4) AS c_tulip_to_simd_ratio,
    round(((sum(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END) / (max(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric) * (100)::numeric), 2) AS simd_vs_c_tulip_percent_improvement,
    round((sum(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN results.mean_time_ns ELSE NULL::bigint END) / (max(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD'::text) THEN results.mean_time_ns ELSE NULL::bigint END))::numeric), 2) AS simd_speedup_vs_c_tulip
   FROM ((benchmark_runs runs
     JOIN benchmark_results results ON ((runs.id = results.run_id)))
     JOIN indicators ind ON ((results.indicator_id = ind.id)))
  WHERE ((results.implementation_type)::text = ANY ((ARRAY['Rust_SIMD'::character varying, 'C_tulip'::character varying])::text[]))
  GROUP BY runs.id, runs.run_timestamp, runs.system_info, ind.name, results.stock_symbol, results.data_source, results.input_size
 HAVING ((sum(CASE WHEN ((results.implementation_type)::text = 'Rust_SIMD'::text) THEN 1 ELSE 0 END) > 0) AND (sum(CASE WHEN ((results.implementation_type)::text = 'C_tulip'::text) THEN 1 ELSE 0 END) > 0))
  ORDER BY runs.run_timestamp DESC, ind.name, results.stock_symbol;

-- Level 2: reference level-1 views

CREATE VIEW prev_avg_run_comparison AS
SELECT current_run.run_id,
    current_run.benchmark_date,
    current_run.hostname,
    current_run.indicator_name,
    current_run.rust_avg_mean_time_ns AS current_rust_avg_time_ns,
    current_run.c_avg_mean_time_ns AS current_c_avg_time_ns,
    current_run.talib_avg_mean_time_ns AS current_talib_avg_time_ns,
    current_run.c_to_rust_ratio AS current_c_to_rust_ratio,
    current_run.rust_options_count AS current_rust_options_count,
    prev_run.run_id AS prev_run_id,
    prev_run.benchmark_date AS prev_benchmark_date,
    prev_run.rust_avg_mean_time_ns AS prev_rust_avg_time_ns,
    prev_run.talib_avg_mean_time_ns AS prev_talib_avg_time_ns,
    prev_run.c_to_rust_ratio AS prev_c_to_rust_ratio,
    round((((current_run.rust_avg_mean_time_ns - prev_run.rust_avg_mean_time_ns) / prev_run.rust_avg_mean_time_ns) * (100)::numeric), 2) AS rust_performance_change_pct,
    round((((current_run.c_avg_mean_time_ns - prev_run.c_avg_mean_time_ns) / prev_run.c_avg_mean_time_ns) * (100)::numeric), 2) AS c_performance_change_pct,
    round((((current_run.talib_avg_mean_time_ns - prev_run.talib_avg_mean_time_ns) / prev_run.talib_avg_mean_time_ns) * (100)::numeric), 2) AS talib_performance_change_pct,
    round((((current_run.c_to_rust_ratio - prev_run.c_to_rust_ratio) / prev_run.c_to_rust_ratio) * (100)::numeric), 2) AS c_ratio_change_pct,
    round((((current_run.talib_to_rust_ratio - prev_run.talib_to_rust_ratio) / prev_run.talib_to_rust_ratio) * (100)::numeric), 2) AS talib_ratio_change_pct,
    (EXTRACT(epoch FROM (current_run.benchmark_date - prev_run.benchmark_date)) / (86400)::numeric) AS days_between_runs
   FROM (( SELECT avg_options_comparison.run_id, avg_options_comparison.benchmark_date, avg_options_comparison.hostname, avg_options_comparison.indicator_name, avg_options_comparison.rust_avg_mean_time_ns, avg_options_comparison.rust_options_count, avg_options_comparison.c_avg_mean_time_ns, avg_options_comparison.c_options_count, avg_options_comparison.talib_avg_mean_time_ns, avg_options_comparison.talib_options_count, avg_options_comparison.c_to_rust_ratio, avg_options_comparison.rust_vs_c_percent_diff, avg_options_comparison.talib_to_rust_ratio, avg_options_comparison.rust_vs_talib_percent_diff,
            lag(avg_options_comparison.run_id) OVER (PARTITION BY avg_options_comparison.hostname, avg_options_comparison.indicator_name ORDER BY avg_options_comparison.benchmark_date) AS prev_run_id_ref,
            lag(avg_options_comparison.benchmark_date) OVER (PARTITION BY avg_options_comparison.hostname, avg_options_comparison.indicator_name ORDER BY avg_options_comparison.benchmark_date) AS prev_benchmark_date_ref
           FROM avg_options_comparison) current_run
     JOIN avg_options_comparison prev_run ON (((prev_run.run_id = current_run.prev_run_id_ref) AND (prev_run.hostname = current_run.hostname) AND ((prev_run.indicator_name)::text = (current_run.indicator_name)::text))))
  WHERE (current_run.prev_run_id_ref IS NOT NULL)
  ORDER BY current_run.benchmark_date DESC, current_run.indicator_name;

CREATE VIEW prev_run_comparison AS
SELECT current_run.run_id, current_run.benchmark_date, current_run.hostname, current_run.indicator_name, current_run.input_size, current_run.options,
    current_run.rust_avg_mean_time_ns AS current_rust_avg_time_ns,
    current_run.c_avg_mean_time_ns AS current_c_avg_time_ns,
    current_run.talib_avg_mean_time_ns AS current_talib_avg_time_ns,
    current_run.c_to_rust_ratio AS current_c_to_rust_ratio,
    current_run.talib_to_rust_ratio AS current_talib_to_rust_ratio,
    prev_run.run_id AS prev_run_id, prev_run.benchmark_date AS prev_benchmark_date,
    prev_run.rust_avg_mean_time_ns AS prev_rust_avg_time_ns,
    prev_run.c_avg_mean_time_ns AS prev_c_avg_time_ns,
    prev_run.talib_avg_mean_time_ns AS prev_talib_avg_time_ns,
    prev_run.c_to_rust_ratio AS prev_c_to_rust_ratio,
    prev_run.talib_to_rust_ratio AS prev_talib_to_rust_ratio,
    round((((current_run.rust_avg_mean_time_ns - prev_run.rust_avg_mean_time_ns) / prev_run.rust_avg_mean_time_ns) * (100)::numeric), 2) AS rust_performance_change_pct,
    round((((current_run.c_avg_mean_time_ns - prev_run.c_avg_mean_time_ns) / prev_run.c_avg_mean_time_ns) * (100)::numeric), 2) AS c_performance_change_pct,
    round((((current_run.talib_avg_mean_time_ns - prev_run.talib_avg_mean_time_ns) / prev_run.talib_avg_mean_time_ns) * (100)::numeric), 2) AS talib_performance_change_pct,
    round((((current_run.c_to_rust_ratio - prev_run.c_to_rust_ratio) / prev_run.c_to_rust_ratio) * (100)::numeric), 2) AS c_ratio_change_pct,
    round((((current_run.talib_to_rust_ratio - prev_run.talib_to_rust_ratio) / prev_run.talib_to_rust_ratio) * (100)::numeric), 2) AS talib_ratio_change_pct,
    (EXTRACT(epoch FROM (current_run.benchmark_date - prev_run.benchmark_date)) / (86400)::numeric) AS days_between_runs
   FROM (( SELECT simplified_comparison.run_id, simplified_comparison.benchmark_date, simplified_comparison.hostname, simplified_comparison.indicator_name, simplified_comparison.input_size, simplified_comparison.options, simplified_comparison.rust_avg_mean_time_ns, simplified_comparison.rust_symbol_count, simplified_comparison.c_avg_mean_time_ns, simplified_comparison.c_symbol_count, simplified_comparison.talib_avg_mean_time_ns, simplified_comparison.talib_symbol_count, simplified_comparison.c_to_rust_ratio, simplified_comparison.rust_vs_c_percent_diff, simplified_comparison.talib_to_rust_ratio, simplified_comparison.rust_vs_talib_percent_diff,
            lag(simplified_comparison.run_id) OVER (PARTITION BY simplified_comparison.hostname, simplified_comparison.indicator_name, simplified_comparison.input_size, simplified_comparison.options ORDER BY simplified_comparison.benchmark_date) AS prev_run_id_ref,
            lag(simplified_comparison.benchmark_date) OVER (PARTITION BY simplified_comparison.hostname, simplified_comparison.indicator_name, simplified_comparison.input_size, simplified_comparison.options ORDER BY simplified_comparison.benchmark_date) AS prev_benchmark_date_ref
           FROM simplified_comparison) current_run
     JOIN simplified_comparison prev_run ON (((prev_run.run_id = current_run.prev_run_id_ref) AND (prev_run.hostname = current_run.hostname) AND ((prev_run.indicator_name)::text = (current_run.indicator_name)::text) AND (prev_run.input_size = current_run.input_size) AND (prev_run.options = current_run.options))))
  WHERE (current_run.prev_run_id_ref IS NOT NULL)
  ORDER BY current_run.benchmark_date DESC, current_run.indicator_name;

CREATE VIEW rust_impl_prev_avg_run_comparison AS
SELECT current_run.run_id, current_run.benchmark_date, current_run.hostname, current_run.indicator_name,
    current_run.rust_avg_mean_time_ns AS current_rust_avg_time_ns,
    current_run.rust_fromstate_avg_mean_time_ns AS current_rust_fromstate_avg_time_ns,
    current_run.rust_optional_avg_mean_time_ns AS current_rust_optional_avg_time_ns,
    current_run.rust_fromstate_1_bar_avg_mean_time_ns AS current_rust_fromstate_1_bar_avg_time_ns,
    current_run.rust_fromstate_1_bar_json_avg_mean_time_ns AS current_rust_fromstate_1_bar_json_avg_time_ns,
    prev_run.run_id AS prev_run_id, prev_run.benchmark_date AS prev_benchmark_date,
    prev_run.rust_avg_mean_time_ns AS prev_rust_avg_time_ns,
    prev_run.rust_fromstate_avg_mean_time_ns AS prev_rust_fromstate_avg_time_ns,
    prev_run.rust_optional_avg_mean_time_ns AS prev_rust_optional_avg_time_ns,
    prev_run.rust_fromstate_1_bar_avg_mean_time_ns AS prev_rust_fromstate_1_bar_avg_time_ns,
    prev_run.rust_fromstate_1_bar_json_avg_mean_time_ns AS prev_rust_fromstate_1_bar_json_avg_time_ns,
    round((((current_run.rust_avg_mean_time_ns - prev_run.rust_avg_mean_time_ns) / prev_run.rust_avg_mean_time_ns) * (100)::numeric), 2) AS rust_performance_change_pct,
    round((((current_run.rust_fromstate_avg_mean_time_ns - prev_run.rust_fromstate_avg_mean_time_ns) / prev_run.rust_fromstate_avg_mean_time_ns) * (100)::numeric), 2) AS rust_fromstate_performance_change_pct,
    round((((current_run.rust_optional_avg_mean_time_ns - prev_run.rust_optional_avg_mean_time_ns) / prev_run.rust_optional_avg_mean_time_ns) * (100)::numeric), 2) AS rust_optional_performance_change_pct,
    round((((current_run.rust_fromstate_1_bar_avg_mean_time_ns - prev_run.rust_fromstate_1_bar_avg_mean_time_ns) / prev_run.rust_fromstate_1_bar_avg_mean_time_ns) * (100)::numeric), 2) AS rust_fromstate_1_bar_performance_change_pct,
    round((((current_run.rust_fromstate_1_bar_json_avg_mean_time_ns - prev_run.rust_fromstate_1_bar_json_avg_mean_time_ns) / prev_run.rust_fromstate_1_bar_json_avg_mean_time_ns) * (100)::numeric), 2) AS rust_fromstate_1_bar_json_performance_change_pct,
    (EXTRACT(epoch FROM (current_run.benchmark_date - prev_run.benchmark_date)) / (86400)::numeric) AS days_between_runs
   FROM (( SELECT rust_impl_avg_options_comparison.run_id, rust_impl_avg_options_comparison.benchmark_date, rust_impl_avg_options_comparison.hostname, rust_impl_avg_options_comparison.indicator_name, rust_impl_avg_options_comparison.rust_avg_mean_time_ns, rust_impl_avg_options_comparison.rust_fromstate_avg_mean_time_ns, rust_impl_avg_options_comparison.rust_optional_avg_mean_time_ns, rust_impl_avg_options_comparison.rust_fromstate_1_bar_avg_mean_time_ns, rust_impl_avg_options_comparison.rust_fromstate_1_bar_json_avg_mean_time_ns,
            lag(rust_impl_avg_options_comparison.run_id) OVER (PARTITION BY rust_impl_avg_options_comparison.hostname, rust_impl_avg_options_comparison.indicator_name ORDER BY rust_impl_avg_options_comparison.benchmark_date) AS prev_run_id_ref,
            lag(rust_impl_avg_options_comparison.benchmark_date) OVER (PARTITION BY rust_impl_avg_options_comparison.hostname, rust_impl_avg_options_comparison.indicator_name ORDER BY rust_impl_avg_options_comparison.benchmark_date) AS prev_benchmark_date_ref
           FROM rust_impl_avg_options_comparison) current_run
     JOIN rust_impl_avg_options_comparison prev_run ON (((prev_run.run_id = current_run.prev_run_id_ref) AND (prev_run.hostname = current_run.hostname) AND ((prev_run.indicator_name)::text = (current_run.indicator_name)::text))))
  WHERE (current_run.prev_run_id_ref IS NOT NULL)
  ORDER BY current_run.benchmark_date DESC, current_run.indicator_name;

CREATE VIEW rust_impl_prev_run_comparison AS
SELECT current_run.run_id, current_run.benchmark_date, current_run.hostname, current_run.indicator_name, current_run.input_size, current_run.options,
    current_run.rust_avg_mean_time_ns AS current_rust_avg_time_ns,
    current_run.rust_fromstate_avg_mean_time_ns AS current_rust_fromstate_avg_time_ns,
    current_run.rust_optional_avg_mean_time_ns AS current_rust_optional_avg_time_ns,
    current_run.rust_fromstate_1_bar_avg_mean_time_ns AS current_rust_fromstate_1_bar_avg_time_ns,
    current_run.rust_fromstate_1_bar_json_avg_mean_time_ns AS current_rust_fromstate_1_bar_json_avg_time_ns,
    prev_run.run_id AS prev_run_id, prev_run.benchmark_date AS prev_benchmark_date,
    prev_run.rust_avg_mean_time_ns AS prev_rust_avg_time_ns,
    prev_run.rust_fromstate_avg_mean_time_ns AS prev_rust_fromstate_avg_time_ns,
    prev_run.rust_optional_avg_mean_time_ns AS prev_rust_optional_avg_time_ns,
    prev_run.rust_fromstate_1_bar_avg_mean_time_ns AS prev_rust_fromstate_1_bar_avg_time_ns,
    prev_run.rust_fromstate_1_bar_json_avg_mean_time_ns AS prev_rust_fromstate_1_bar_json_avg_time_ns,
    round((((current_run.rust_avg_mean_time_ns - prev_run.rust_avg_mean_time_ns) / prev_run.rust_avg_mean_time_ns) * (100)::numeric), 2) AS rust_performance_change_pct,
    round((((current_run.rust_fromstate_avg_mean_time_ns - prev_run.rust_fromstate_avg_mean_time_ns) / prev_run.rust_fromstate_avg_mean_time_ns) * (100)::numeric), 2) AS rust_fromstate_performance_change_pct,
    round((((current_run.rust_optional_avg_mean_time_ns - prev_run.rust_optional_avg_mean_time_ns) / prev_run.rust_optional_avg_mean_time_ns) * (100)::numeric), 2) AS rust_optional_performance_change_pct,
    round((((current_run.rust_fromstate_1_bar_avg_mean_time_ns - prev_run.rust_fromstate_1_bar_avg_mean_time_ns) / prev_run.rust_fromstate_1_bar_avg_mean_time_ns) * (100)::numeric), 2) AS rust_fromstate_1_bar_performance_change_pct,
    round((((current_run.rust_fromstate_1_bar_json_avg_mean_time_ns - prev_run.rust_fromstate_1_bar_json_avg_mean_time_ns) / prev_run.rust_fromstate_1_bar_json_avg_mean_time_ns) * (100)::numeric), 2) AS rust_fromstate_1_bar_json_performance_change_pct,
    (EXTRACT(epoch FROM (current_run.benchmark_date - prev_run.benchmark_date)) / (86400)::numeric) AS days_between_runs
   FROM (( SELECT rust_impl_simplified_comparison.run_id, rust_impl_simplified_comparison.benchmark_date, rust_impl_simplified_comparison.hostname, rust_impl_simplified_comparison.indicator_name, rust_impl_simplified_comparison.input_size, rust_impl_simplified_comparison.options, rust_impl_simplified_comparison.rust_avg_mean_time_ns, rust_impl_simplified_comparison.rust_symbol_count, rust_impl_simplified_comparison.rust_fromstate_avg_mean_time_ns, rust_impl_simplified_comparison.rust_fromstate_symbol_count, rust_impl_simplified_comparison.rust_optional_avg_mean_time_ns, rust_impl_simplified_comparison.rust_optional_symbol_count, rust_impl_simplified_comparison.rust_fromstate_1_bar_avg_mean_time_ns, rust_impl_simplified_comparison.rust_fromstate_1_bar_symbol_count, rust_impl_simplified_comparison.rust_fromstate_1_bar_json_avg_mean_time_ns, rust_impl_simplified_comparison.rust_fromstate_1_bar_json_symbol_count,
            lag(rust_impl_simplified_comparison.run_id) OVER (PARTITION BY rust_impl_simplified_comparison.hostname, rust_impl_simplified_comparison.indicator_name, rust_impl_simplified_comparison.input_size, rust_impl_simplified_comparison.options ORDER BY rust_impl_simplified_comparison.benchmark_date) AS prev_run_id_ref,
            lag(rust_impl_simplified_comparison.benchmark_date) OVER (PARTITION BY rust_impl_simplified_comparison.hostname, rust_impl_simplified_comparison.indicator_name, rust_impl_simplified_comparison.input_size, rust_impl_simplified_comparison.options ORDER BY rust_impl_simplified_comparison.benchmark_date) AS prev_benchmark_date_ref
           FROM rust_impl_simplified_comparison) current_run
     JOIN rust_impl_simplified_comparison prev_run ON (((prev_run.run_id = current_run.prev_run_id_ref) AND (prev_run.hostname = current_run.hostname) AND ((prev_run.indicator_name)::text = (current_run.indicator_name)::text) AND (prev_run.input_size = current_run.input_size) AND (prev_run.options = current_run.options))))
  WHERE (current_run.prev_run_id_ref IS NOT NULL)
  ORDER BY current_run.benchmark_date DESC, current_run.indicator_name;

CREATE VIEW rust_simd_simplified_comparison AS
SELECT run_id, benchmark_date, hostname, indicator_name, input_size,
    round(avg(rust_total_mean_time_ns)) AS rust_avg_total_time_ns,
    avg(rust_options_count) AS rust_avg_options_count,
    count(CASE WHEN (rust_total_mean_time_ns IS NOT NULL) THEN 1 ELSE NULL::integer END) AS rust_stock_count,
    round(avg(rust_simd_mean_time_ns)) AS rust_simd_avg_time_ns,
    count(CASE WHEN (rust_simd_mean_time_ns IS NOT NULL) THEN 1 ELSE NULL::integer END) AS rust_simd_stock_count,
    round(avg(simd_to_rust_ratio), 4) AS avg_simd_to_rust_ratio,
    round(avg(simd_vs_rust_percent_improvement), 2) AS avg_simd_improvement_pct,
    round(avg(simd_speedup_factor), 2) AS avg_simd_speedup_factor,
    round(min(simd_vs_rust_percent_improvement), 2) AS min_simd_improvement_pct,
    round(max(simd_vs_rust_percent_improvement), 2) AS max_simd_improvement_pct,
    round(min(simd_speedup_factor), 2) AS min_simd_speedup,
    round(max(simd_speedup_factor), 2) AS max_simd_speedup
   FROM rust_simd_performance_comparison pc
  GROUP BY run_id, benchmark_date, hostname, indicator_name, input_size
  ORDER BY benchmark_date DESC, indicator_name;

CREATE VIEW rust_simd_asset_simplified_comparison AS
SELECT run_id, benchmark_date, hostname, indicator_name, data_source,
    round(avg(rust_total_mean_time_ns)) AS rust_avg_total_time_ns,
    round(avg(c_tulip_total_mean_time_ns)) AS c_tulip_avg_total_time_ns,
    round(avg(talib_total_mean_time_ns)) AS talib_avg_total_time_ns,
    round(avg(rust_simd_asset_mean_time_ns)) AS rust_simd_asset_avg_time_ns,
    round(avg(simd_asset_to_rust_ratio), 4) AS avg_simd_asset_to_rust_ratio,
    round(avg(simd_asset_vs_rust_percent_improvement), 2) AS avg_simd_asset_vs_rust_improvement_pct,
    round(avg(simd_asset_vs_c_tulip_percent_improvement), 2) AS avg_simd_asset_vs_c_tulip_improvement_pct,
    round(avg(simd_asset_vs_talib_percent_improvement), 2) AS avg_simd_asset_vs_talib_improvement_pct
   FROM rust_simd_asset_performance_comparison pc
  GROUP BY run_id, benchmark_date, hostname, indicator_name, data_source
  ORDER BY benchmark_date DESC, indicator_name;

CREATE VIEW rust_simd_c_tulip_simplified_comparison AS
SELECT run_id, benchmark_date, hostname, indicator_name, input_size,
    round(avg(rust_simd_mean_time_ns)) AS rust_simd_avg_time_ns,
    count(CASE WHEN (rust_simd_mean_time_ns IS NOT NULL) THEN 1 ELSE NULL::integer END) AS rust_simd_stock_count,
    round(avg(c_tulip_total_mean_time_ns)) AS c_tulip_avg_total_time_ns,
    avg(c_tulip_options_count) AS c_tulip_avg_options_count,
    count(CASE WHEN (c_tulip_total_mean_time_ns IS NOT NULL) THEN 1 ELSE NULL::integer END) AS c_tulip_stock_count,
    round(avg(c_tulip_to_simd_ratio), 4) AS avg_c_tulip_to_simd_ratio,
    round(avg(simd_vs_c_tulip_percent_improvement), 2) AS avg_simd_vs_c_tulip_improvement_pct,
    round(avg(simd_speedup_vs_c_tulip), 2) AS avg_simd_speedup_vs_c_tulip,
    round(min(simd_vs_c_tulip_percent_improvement), 2) AS min_simd_vs_c_tulip_improvement_pct,
    round(max(simd_vs_c_tulip_percent_improvement), 2) AS max_simd_vs_c_tulip_improvement_pct,
    round(min(simd_speedup_vs_c_tulip), 2) AS min_simd_speedup_vs_c_tulip,
    round(max(simd_speedup_vs_c_tulip), 2) AS max_simd_speedup_vs_c_tulip
   FROM rust_simd_c_tulip_performance_comparison pc
  GROUP BY run_id, benchmark_date, hostname, indicator_name, input_size
  ORDER BY benchmark_date DESC, indicator_name;

-- Level 3: reference level-2 views

CREATE VIEW rust_impl_slower_indicators AS
SELECT run_id, benchmark_date, hostname, indicator_name,
    current_rust_avg_time_ns, current_rust_fromstate_avg_time_ns, current_rust_optional_avg_time_ns,
    current_rust_fromstate_1_bar_avg_time_ns, current_rust_fromstate_1_bar_json_avg_time_ns,
    rust_performance_change_pct, rust_fromstate_performance_change_pct, rust_optional_performance_change_pct,
    rust_fromstate_1_bar_performance_change_pct, rust_fromstate_1_bar_json_performance_change_pct,
    days_between_runs
   FROM ( SELECT rust_impl_prev_avg_run_comparison.*,
            row_number() OVER (PARTITION BY rust_impl_prev_avg_run_comparison.hostname, rust_impl_prev_avg_run_comparison.indicator_name ORDER BY rust_impl_prev_avg_run_comparison.benchmark_date DESC) AS rn
           FROM rust_impl_prev_avg_run_comparison) latest_runs
  WHERE (rn = 1)
  ORDER BY hostname, indicator_name;

CREATE VIEW rust_slower_indicators AS
SELECT run_id, benchmark_date, hostname, indicator_name,
    current_rust_avg_time_ns, current_c_avg_time_ns, current_talib_avg_time_ns, current_c_to_rust_ratio,
    rust_performance_change_pct, talib_performance_change_pct, talib_ratio_change_pct,
    CASE WHEN ((current_c_avg_time_ns IS NOT NULL) AND (current_rust_avg_time_ns > current_c_avg_time_ns)) THEN round((((current_rust_avg_time_ns - current_c_avg_time_ns) / current_c_avg_time_ns) * (100)::numeric), 2) ELSE NULL::numeric END AS rust_slower_than_c_by_pct,
    CASE WHEN ((current_talib_avg_time_ns IS NOT NULL) AND (current_rust_avg_time_ns > current_talib_avg_time_ns)) THEN round((((current_rust_avg_time_ns - current_talib_avg_time_ns) / current_talib_avg_time_ns) * (100)::numeric), 2) ELSE NULL::numeric END AS rust_slower_than_talib_by_pct,
    days_between_runs
   FROM ( SELECT prev_avg_run_comparison.*,
            row_number() OVER (PARTITION BY prev_avg_run_comparison.hostname, prev_avg_run_comparison.indicator_name ORDER BY prev_avg_run_comparison.benchmark_date DESC) AS rn
           FROM prev_avg_run_comparison) latest_runs
  WHERE ((rn = 1) AND (((current_c_avg_time_ns IS NOT NULL) AND (current_rust_avg_time_ns > current_c_avg_time_ns)) OR ((current_talib_avg_time_ns IS NOT NULL) AND (current_rust_avg_time_ns > current_talib_avg_time_ns))))
  ORDER BY hostname, indicator_name;

CREATE VIEW rust_simd_avg_comparison AS
SELECT run_id, benchmark_date, hostname, indicator_name,
    round(avg(rust_avg_total_time_ns)) AS rust_overall_avg_time_ns,
    round(avg(rust_avg_options_count)) AS rust_overall_avg_options,
    round(avg(rust_simd_avg_time_ns)) AS rust_simd_overall_avg_time_ns,
    round(avg(avg_simd_to_rust_ratio), 4) AS overall_simd_to_rust_ratio,
    round(avg(avg_simd_improvement_pct), 2) AS overall_simd_improvement_pct,
    round(avg(avg_simd_speedup_factor), 2) AS overall_simd_speedup_factor,
    round(min(min_simd_improvement_pct), 2) AS best_case_improvement_pct,
    round(max(max_simd_improvement_pct), 2) AS worst_case_improvement_pct,
    round(min(min_simd_speedup), 2) AS best_case_speedup,
    round(max(max_simd_speedup), 2) AS worst_case_speedup,
    sum(rust_stock_count) AS total_rust_measurements,
    sum(rust_simd_stock_count) AS total_simd_measurements
   FROM rust_simd_simplified_comparison sc
  GROUP BY run_id, benchmark_date, hostname, indicator_name
  ORDER BY benchmark_date DESC, indicator_name;

CREATE VIEW rust_simd_asset_avg_comparison AS
SELECT run_id, benchmark_date, hostname, indicator_name,
    round(avg(rust_avg_total_time_ns)) AS rust_overall_avg_time_ns,
    round(avg(c_tulip_avg_total_time_ns)) AS c_tulip_overall_avg_time_ns,
    round(avg(talib_avg_total_time_ns)) AS talib_overall_avg_time_ns,
    round(avg(rust_simd_asset_avg_time_ns)) AS rust_simd_asset_overall_avg_time_ns,
    round(avg(avg_simd_asset_to_rust_ratio), 4) AS overall_simd_asset_to_rust_ratio,
    round(avg(avg_simd_asset_vs_rust_improvement_pct), 2) AS overall_simd_asset_vs_rust_improvement_pct,
    round(avg(avg_simd_asset_vs_c_tulip_improvement_pct), 2) AS overall_simd_asset_vs_c_tulip_improvement_pct,
    round(avg(avg_simd_asset_vs_talib_improvement_pct), 2) AS overall_simd_asset_vs_talib_improvement_pct
   FROM rust_simd_asset_simplified_comparison sc
  GROUP BY run_id, benchmark_date, hostname, indicator_name
  ORDER BY benchmark_date DESC, indicator_name;

CREATE VIEW rust_simd_c_tulip_avg_comparison AS
SELECT run_id, benchmark_date, hostname, indicator_name,
    round(avg(rust_simd_avg_time_ns)) AS rust_simd_overall_avg_time_ns,
    round(avg(c_tulip_avg_total_time_ns)) AS c_tulip_overall_avg_time_ns,
    round(avg(c_tulip_avg_options_count)) AS c_tulip_overall_avg_options,
    round(avg(avg_c_tulip_to_simd_ratio), 4) AS overall_c_tulip_to_simd_ratio,
    round(avg(avg_simd_vs_c_tulip_improvement_pct), 2) AS overall_simd_vs_c_tulip_improvement_pct,
    round(avg(avg_simd_speedup_vs_c_tulip), 2) AS overall_simd_speedup_vs_c_tulip,
    round(min(min_simd_vs_c_tulip_improvement_pct), 2) AS best_case_simd_vs_c_tulip_improvement_pct,
    round(max(max_simd_vs_c_tulip_improvement_pct), 2) AS worst_case_simd_vs_c_tulip_improvement_pct,
    round(min(min_simd_speedup_vs_c_tulip), 2) AS best_case_simd_speedup_vs_c_tulip,
    round(max(max_simd_speedup_vs_c_tulip), 2) AS worst_case_simd_speedup_vs_c_tulip,
    sum(rust_simd_stock_count) AS total_simd_measurements,
    sum(c_tulip_stock_count) AS total_c_tulip_measurements
   FROM rust_simd_c_tulip_simplified_comparison sc
  GROUP BY run_id, benchmark_date, hostname, indicator_name
  ORDER BY benchmark_date DESC, indicator_name;

-- Level 4: reference level-3 views

CREATE VIEW rust_simd_prev_run_comparison AS
SELECT current_run.run_id, current_run.benchmark_date, current_run.hostname, current_run.indicator_name,
    current_run.rust_overall_avg_time_ns AS current_rust_avg_time_ns,
    current_run.rust_simd_overall_avg_time_ns AS current_simd_avg_time_ns,
    current_run.overall_simd_improvement_pct AS current_simd_improvement_pct,
    current_run.overall_simd_speedup_factor AS current_simd_speedup_factor,
    prev_run.run_id AS prev_run_id, prev_run.benchmark_date AS prev_benchmark_date,
    prev_run.rust_overall_avg_time_ns AS prev_rust_avg_time_ns,
    prev_run.rust_simd_overall_avg_time_ns AS prev_simd_avg_time_ns,
    prev_run.overall_simd_improvement_pct AS prev_simd_improvement_pct,
    prev_run.overall_simd_speedup_factor AS prev_simd_speedup_factor,
    round((((current_run.rust_overall_avg_time_ns - prev_run.rust_overall_avg_time_ns) / prev_run.rust_overall_avg_time_ns) * (100)::numeric), 2) AS rust_performance_change_pct,
    round((((current_run.rust_simd_overall_avg_time_ns - prev_run.rust_simd_overall_avg_time_ns) / prev_run.rust_simd_overall_avg_time_ns) * (100)::numeric), 2) AS simd_performance_change_pct,
    round((current_run.overall_simd_improvement_pct - prev_run.overall_simd_improvement_pct), 2) AS simd_improvement_change_pct,
    round((((current_run.overall_simd_speedup_factor - prev_run.overall_simd_speedup_factor) / prev_run.overall_simd_speedup_factor) * (100)::numeric), 2) AS simd_speedup_change_pct,
    (EXTRACT(epoch FROM (current_run.benchmark_date - prev_run.benchmark_date)) / (86400)::numeric) AS days_between_runs
   FROM (( SELECT rust_simd_avg_comparison.*,
            lag(rust_simd_avg_comparison.run_id) OVER (PARTITION BY rust_simd_avg_comparison.hostname, rust_simd_avg_comparison.indicator_name ORDER BY rust_simd_avg_comparison.benchmark_date) AS prev_run_id_ref,
            lag(rust_simd_avg_comparison.benchmark_date) OVER (PARTITION BY rust_simd_avg_comparison.hostname, rust_simd_avg_comparison.indicator_name ORDER BY rust_simd_avg_comparison.benchmark_date) AS prev_benchmark_date_ref
           FROM rust_simd_avg_comparison) current_run
     JOIN rust_simd_avg_comparison prev_run ON (((prev_run.run_id = current_run.prev_run_id_ref) AND (prev_run.hostname = current_run.hostname) AND ((prev_run.indicator_name)::text = (current_run.indicator_name)::text))))
  WHERE (current_run.prev_run_id_ref IS NOT NULL)
  ORDER BY current_run.benchmark_date DESC, current_run.indicator_name;

CREATE VIEW rust_simd_asset_prev_run_comparison AS
SELECT current_run.run_id, current_run.benchmark_date, current_run.hostname, current_run.indicator_name,
    current_run.prev_run_id_ref,
    round((((current_run.rust_simd_asset_overall_avg_time_ns - prev_run.rust_simd_asset_overall_avg_time_ns) / prev_run.rust_simd_asset_overall_avg_time_ns) * (100)::numeric), 2) AS simd_asset_performance_change_pct,
    round((current_run.overall_simd_asset_to_rust_ratio - prev_run.overall_simd_asset_to_rust_ratio), 4) AS simd_asset_to_rust_ratio_change,
    round((current_run.overall_simd_asset_vs_rust_improvement_pct - prev_run.overall_simd_asset_vs_rust_improvement_pct), 2) AS simd_asset_vs_rust_improvement_change_pct,
    round((current_run.overall_simd_asset_vs_c_tulip_improvement_pct - prev_run.overall_simd_asset_vs_c_tulip_improvement_pct), 2) AS simd_asset_vs_c_tulip_improvement_change_pct,
    round((current_run.overall_simd_asset_vs_talib_improvement_pct - prev_run.overall_simd_asset_vs_talib_improvement_pct), 2) AS simd_asset_vs_talib_improvement_change_pct,
    prev_run.benchmark_date AS prev_run_date,
    current_run.overall_simd_asset_to_rust_ratio,
    current_run.overall_simd_asset_vs_rust_improvement_pct,
    current_run.overall_simd_asset_vs_c_tulip_improvement_pct,
    current_run.overall_simd_asset_vs_talib_improvement_pct
   FROM (( SELECT rust_simd_asset_avg_comparison.*,
            lag(rust_simd_asset_avg_comparison.run_id) OVER (PARTITION BY rust_simd_asset_avg_comparison.hostname, rust_simd_asset_avg_comparison.indicator_name ORDER BY rust_simd_asset_avg_comparison.benchmark_date) AS prev_run_id_ref
           FROM rust_simd_asset_avg_comparison) current_run
     JOIN rust_simd_asset_avg_comparison prev_run ON (((prev_run.run_id = current_run.prev_run_id_ref) AND (prev_run.hostname = current_run.hostname) AND ((prev_run.indicator_name)::text = (current_run.indicator_name)::text))))
  WHERE (current_run.prev_run_id_ref IS NOT NULL)
  ORDER BY current_run.benchmark_date DESC, current_run.indicator_name;

CREATE VIEW rust_simd_c_tulip_prev_run_comparison AS
SELECT current_run.run_id, current_run.benchmark_date, current_run.hostname, current_run.indicator_name,
    current_run.rust_simd_overall_avg_time_ns AS current_simd_avg_time_ns,
    current_run.c_tulip_overall_avg_time_ns AS current_c_tulip_avg_time_ns,
    current_run.overall_simd_vs_c_tulip_improvement_pct AS current_simd_vs_c_tulip_improvement_pct,
    current_run.overall_simd_speedup_vs_c_tulip AS current_simd_speedup_vs_c_tulip,
    prev_run.run_id AS prev_run_id, prev_run.benchmark_date AS prev_benchmark_date,
    prev_run.rust_simd_overall_avg_time_ns AS prev_simd_avg_time_ns,
    prev_run.c_tulip_overall_avg_time_ns AS prev_c_tulip_avg_time_ns,
    prev_run.overall_simd_vs_c_tulip_improvement_pct AS prev_simd_vs_c_tulip_improvement_pct,
    prev_run.overall_simd_speedup_vs_c_tulip AS prev_simd_speedup_vs_c_tulip,
    round((((current_run.rust_simd_overall_avg_time_ns - prev_run.rust_simd_overall_avg_time_ns) / prev_run.rust_simd_overall_avg_time_ns) * (100)::numeric), 2) AS simd_performance_change_pct,
    round((((current_run.c_tulip_overall_avg_time_ns - prev_run.c_tulip_overall_avg_time_ns) / prev_run.c_tulip_overall_avg_time_ns) * (100)::numeric), 2) AS c_tulip_performance_change_pct,
    round((current_run.overall_simd_vs_c_tulip_improvement_pct - prev_run.overall_simd_vs_c_tulip_improvement_pct), 2) AS simd_vs_c_tulip_improvement_change_pct,
    round((((current_run.overall_simd_speedup_vs_c_tulip - prev_run.overall_simd_speedup_vs_c_tulip) / prev_run.overall_simd_speedup_vs_c_tulip) * (100)::numeric), 2) AS simd_speedup_vs_c_tulip_change_pct,
    (EXTRACT(epoch FROM (current_run.benchmark_date - prev_run.benchmark_date)) / (86400)::numeric) AS days_between_runs
   FROM (( SELECT rust_simd_c_tulip_avg_comparison.*,
            lag(rust_simd_c_tulip_avg_comparison.run_id) OVER (PARTITION BY rust_simd_c_tulip_avg_comparison.hostname, rust_simd_c_tulip_avg_comparison.indicator_name ORDER BY rust_simd_c_tulip_avg_comparison.benchmark_date) AS prev_run_id_ref,
            lag(rust_simd_c_tulip_avg_comparison.benchmark_date) OVER (PARTITION BY rust_simd_c_tulip_avg_comparison.hostname, rust_simd_c_tulip_avg_comparison.indicator_name ORDER BY rust_simd_c_tulip_avg_comparison.benchmark_date) AS prev_benchmark_date_ref
           FROM rust_simd_c_tulip_avg_comparison) current_run
     JOIN rust_simd_c_tulip_avg_comparison prev_run ON (((prev_run.run_id = current_run.prev_run_id_ref) AND (prev_run.hostname = current_run.hostname) AND ((prev_run.indicator_name)::text = (current_run.indicator_name)::text))))
  WHERE (current_run.prev_run_id_ref IS NOT NULL)
  ORDER BY current_run.benchmark_date DESC, current_run.indicator_name;

-- ---------------------------------------------------------------------------
-- Seed: indicators (exact IDs preserved via OVERRIDING SYSTEM VALUE)
-- ---------------------------------------------------------------------------

INSERT INTO indicators (id, name, description, input_count, output_count, has_options, category)
OVERRIDING SYSTEM VALUE VALUES
    (1,   'sma',                    'Simple Moving Average',                                                      1, 1, true,  'trend'),
    (2,   'ema',                    'Exponential Moving Average',                                                 1, 1, true,  'trend'),
    (3,   'wma',                    'Weighted Moving Average',                                                    1, 1, true,  'trend'),
    (4,   'dema',                   'Double Exponential Moving Average',                                          1, 1, true,  'trend'),
    (5,   'tema',                   'Triple Exponential Moving Average',                                          1, 1, true,  'trend'),
    (6,   'trima',                  'Triangular Moving Average',                                                  1, 1, true,  'trend'),
    (7,   'hma',                    'Hull Moving Average',                                                        1, 1, true,  'trend'),
    (8,   'kama',                   'Kaufman Adaptive Moving Average',                                            1, 1, true,  'trend'),
    (9,   'vwma',                   'Volume Weighted Moving Average',                                             2, 1, true,  'trend'),
    (10,  'zlema',                  'Zero Lag Exponential Moving Average',                                        1, 1, true,  'trend'),
    (11,  'rema',                   'Regularized Exponential Moving Average',                                     1, 1, true,  'trend'),
    (12,  'macd',                   'Moving Average Convergence Divergence',                                      1, 3, true,  'momentum'),
    (13,  'rsi',                    'Relative Strength Index',                                                    1, 1, true,  'momentum'),
    (14,  'stoch',                  'Stochastic Oscillator',                                                      3, 2, true,  'momentum'),
    (15,  'stochrsi',               'Stochastic RSI',                                                             1, 2, true,  'momentum'),
    (16,  'cmo',                    'Chande Momentum Oscillator',                                                 1, 1, true,  'momentum'),
    (17,  'mom',                    'Momentum',                                                                   1, 1, true,  'momentum'),
    (18,  'roc',                    'Rate of Change',                                                             1, 1, true,  'momentum'),
    (19,  'rocr',                   'Rate of Change Ratio',                                                       1, 1, true,  'momentum'),
    (20,  'apo',                    'Absolute Price Oscillator',                                                  1, 1, true,  'momentum'),
    (21,  'ppo',                    'Percentage Price Oscillator',                                                1, 1, true,  'momentum'),
    (22,  'ao',                     'Awesome Oscillator',                                                         2, 1, false, 'momentum'),
    (23,  'fosc',                   'Forecast Oscillator',                                                        1, 1, true,  'momentum'),
    (24,  'qstick',                 'Qstick',                                                                     2, 1, true,  'momentum'),
    (25,  'ultosc',                 'Ultimate Oscillator',                                                        3, 1, true,  'momentum'),
    (26,  'willr',                  'Williams %R',                                                                3, 1, true,  'momentum'),
    (27,  'aroon',                  'Aroon',                                                                      2, 2, true,  'momentum'),
    (28,  'aroonosc',               'Aroon Oscillator',                                                           2, 1, true,  'momentum'),
    (29,  'atr',                    'Average True Range',                                                         3, 1, true,  'volatility'),
    (30,  'natr',                   'Normalized Average True Range',                                              3, 1, true,  'volatility'),
    (31,  'tr',                     'True Range',                                                                 3, 1, false, 'volatility'),
    (32,  'volatility',             'Volatility',                                                                 1, 1, true,  'volatility'),
    (33,  'stddev',                 'Standard Deviation',                                                         1, 1, true,  'volatility'),
    (34,  'bbands',                 'Bollinger Bands',                                                            1, 3, true,  'volatility'),
    (35,  'ad',                     'Accumulation/Distribution Line',                                             4, 1, false, 'volume'),
    (36,  'adosc',                  'Accumulation/Distribution Oscillator',                                       4, 1, true,  'volume'),
    (37,  'obv',                    'On Balance Volume',                                                          2, 1, false, 'volume'),
    (38,  'pvi',                    'Positive Volume Index',                                                      2, 1, false, 'volume'),
    (39,  'nvi',                    'Negative Volume Index',                                                      2, 1, false, 'volume'),
    (40,  'kvo',                    'Klinger Volume Oscillator',                                                  5, 1, true,  'volume'),
    (41,  'marketfi',               'Market Facilitation Index',                                                  3, 1, false, 'volume'),
    (42,  'mfi',                    'Money Flow Index',                                                           4, 1, true,  'volume'),
    (43,  'emv',                    'Ease of Movement',                                                           3, 1, true,  'volume'),
    (44,  'vhf',                    'Vertical Horizontal Filter',                                                 1, 1, true,  'volume'),
    (45,  'vosc',                   'Volume Oscillator',                                                          1, 1, true,  'volume'),
    (46,  'wad',                    'Williams Accumulation/Distribution',                                         3, 1, false, 'volume'),
    (47,  'adx',                    'Average Directional Index',                                                  3, 1, true,  'trend'),
    (48,  'adxr',                   'Average Directional Index Rating',                                           3, 1, true,  'trend'),
    (49,  'dm',                     'Directional Movement',                                                       2, 2, true,  'trend'),
    (50,  'di',                     'Directional Indicator',                                                      3, 2, true,  'trend'),
    (51,  'dx',                     'Directional Movement Index',                                                 3, 1, true,  'trend'),
    (52,  'cci',                    'Commodity Channel Index',                                                    3, 1, true,  'trend'),
    (53,  'dpo',                    'Detrended Price Oscillator',                                                 1, 1, true,  'trend'),
    (54,  'linreg',                 'Linear Regression',                                                          1, 1, true,  'trend'),
    (55,  'tsf',                    'Time Series Forecast',                                                       1, 1, true,  'trend'),
    (56,  'psar',                   'Parabolic SAR',                                                              3, 1, true,  'trend'),
    (57,  'trix',                   'TRIX',                                                                       1, 1, true,  'trend'),
    (58,  'mass',                   'Mass Index',                                                                 2, 1, true,  'trend'),
    (59,  'cvi',                    'Chaikins Volatility',                                                        2, 1, true,  'trend'),
    (60,  'msw',                    'Mesa Sine Wave',                                                             1, 2, true,  'trend'),
    (61,  'vidya',                  'Variable Index Dynamic Average',                                             1, 1, true,  'trend'),
    (62,  'avgprice',               'Average Price',                                                              4, 1, false, 'price'),
    (63,  'medprice',               'Median Price',                                                               2, 1, false, 'price'),
    (64,  'typprice',               'Typical Price',                                                              3, 1, false, 'price'),
    (65,  'wcprice',                'Weighted Close Price',                                                       3, 1, false, 'price'),
    (66,  'max',                    'Maximum',                                                                    1, 1, true,  'math'),
    (67,  'min',                    'Minimum',                                                                    1, 1, true,  'math'),
    (68,  'md',                     'Mean Deviation',                                                             1, 1, true,  'math'),
    (69,  'range',                  'Range',                                                                      2, 1, true,  'math'),
    (70,  'wilders',                'Wilders Smoothing',                                                          1, 1, true,  'overlap'),
    (71,  'bop',                    'Balance of Power',                                                           4, 1, false, 'overlap'),
    (72,  'pivotpoint',             'Pivot Point',                                                                3, 7, false, 'support_resistance'),
    (158, 'ao_medprice',            'Awesome Oscillator With Medprice Input',                                     1, 1, false, 'momentum'),
    (159, 'fisher',                 'Fisher Transform',                                                           2, 2, true,  'momentum'),
    (160, 'Rust_Candlestick',       'Single candlestick indicator that scans for all candle patterns',            4, 1, true,  'candlestick');

-- Reset sequence to max id + 1
SELECT setval('indicators_id_seq', (SELECT MAX(id) FROM indicators));

\echo '>>> Done. indicator_benchmark database ready.'
SELECT category, COUNT(*) AS indicator_count FROM indicators GROUP BY category ORDER BY category;
