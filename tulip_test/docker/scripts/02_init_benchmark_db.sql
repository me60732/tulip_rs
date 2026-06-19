-- =============================================================================
-- init_benchmark_db.sql
-- Benchmark database: schema (tables, sequences, indexes) + indicator seed data.
-- Views are in separate scripts: 03_rust_benchmark_views.sql and 04_python_benchmark_views.sql.
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
    (160, 'Rust_Candlestick',       'Single candlestick indicator that scans for all candle patterns',            4, 1, true,  'candlestick'),
    (161, 'ef',                     'Efficency Ratio',                                                            1, 1, true,  'trend'),
    (162, 'chandelierexit',         'Chandelier Exit',                                                            3, 4, true,  'trend'),
    (163, 'keltnerchannel',         'Keltner Channel',                                                            3, 2, true, 'volatility'),
    (164,  'smaenvelope',            'SMA Envelope',                                                              1, 2, true, 'trend'),
    (165,  'donchianchannel',       'Donchian Channel',                                                           2, 3, true, 'trend'),
    (166,  'elderray',              'Elder-Ray',                                                                  3, 2, true, 'trend'),
    (167,  'vortex',                'Vortex',                                                                     3, 2, true, 'trend'),
    (168,  'trvi',                  'True Range Volatility Indicator',                                            3, 1, true, 'trend'),
    (169,  'chaikinmf',             'Chaikin Money Flow',                                                         4, 1, true, 'volume'),
    (170,  'vwap',                  'Volume Weighted Average Price',                                              4, 1, false, 'trend'),
    (171,  'supertrend',            'Super Trend',                                                                3, 1, true, 'trend'),
    (172,  'ichimoku',              'Ichimoku',                                                                   3, 5, true, 'trend'),
    (173,  'supersmoother',         'Ehlers Super Smootherer',                                                    1, 1, true, 'trend'),
    (174,  'highpass',              'Ehlers High Pass Filter',                                                    1, 1, true, 'trend'),
    (175,  'roofingfilter',         'Ehlers Roofing Filter',                                                      1, 1, true, 'trend'),
    (176,  'hilberttransform',      'Ehlers Hilbert Transform',                                                   1, 2, true, 'trend'),
    (177,  'homodynediscriminator', 'homodynediscriminator',                                                      1, 1, false, 'trend'),
    (178,  'mama',                  'MESA Adaptive Moving Average',                                               1, 2, true, 'trend'),
    (179,  'instantaneoustrendline','Ehlers Instantaneous Trendline',                                             1, 1, false, 'trend'),
    (180,  'adaptivemsw',           'Adaptive Mesa Sine Wave',                                                    1, 2, false, 'cycle'),
    (181,  'trendmode',             'Ehlers TrendMode',                                                           1, 2, true, 'trend'),
    (182,   'cybercycle',           'Ehlers Cyber Cycle',                                                         1, 2, true,   'cycle'),
    (183,   'ccfisher',             'Cyber Cycle Fisher',                                                         1, 2, true, 'cycle');

-- Reset sequence to max id + 1
SELECT setval('indicators_id_seq', (SELECT MAX(id) FROM indicators));

\echo '>>> Done. indicator_benchmark database ready.'
SELECT category, COUNT(*) AS indicator_count FROM indicators GROUP BY category ORDER BY category;
