# Node.js Binding — Running the Benchmarks

The `tulip_rs_node` vs `technicalindicators` / `indicatorts` benchmark results are part of the
[Standard Performance](standard.md) page — select the **Node Binding** tab there to see the full comparison table.

This page covers how to install the bench package and run it locally.

---

## Setup

### Prerequisites

| Requirement | Notes |
|---|---|
| **Node.js 18+** | LTS recommended |
| **tulip-rs-node built** | Run `npm run build` in `tulip_rs_node/` first |
| **Docker + Docker Compose v2** | For the benchmark databases |

### 1. Build tulip-rs-node

The bench package references `tulip-rs-node` as a local file dependency.
Build it first if you have not already:

```sh
cd tulip_rs_node
npm run build
```

### 2. Start the databases

The Node benchmarks use the same PostgreSQL instance as the Rust and Python benchmarks.
The Node benchmark views (`node_performance_comparison`, `node_avg_options_comparison`)
are applied automatically via `05_node_benchmark_views.sql` on first boot.

```sh
cd tulip_rs/tulip_test/docker
docker compose up -d
# First start takes ~30 s while the SQL seed data loads.
# The pg_data volume keeps data across restarts.
```

| Database | Purpose |
|---|---|
| `stocks` | Read-only OHLCV market data (4 symbols, ~6,705 bars each) |
| `indicator_benchmark` | Benchmark run metadata and result storage |

### 3. Install the bench package

```sh
cd tulip_rs_node/bench
npm install
```

### 4. Configure the environment

Create or edit `tulip_rs_node/bench/.env`:

```sh
# Connection to the stocks database (read-only market data)
DATABASE_URL=postgres://tulip:tulip@localhost:5432/stocks

# Connection to the benchmark results database
BENCHMARK_DATABASE_URL=postgres://tulip:tulip@localhost:5432/indicator_benchmark

# Set to 1 to write timing rows into indicator_benchmark after each run.
BENCHMARK_LOG_TO_DB=0
```

!!! tip
    `dotenv` walks upward from the working directory to find `.env`, so running
    `npm run bench` from `tulip_rs_node/bench/` picks it up automatically.
    No shell exports needed.

---

## Running

```sh
cd tulip_rs_node/bench

# All 81 indicators — stdout only
npm run bench

# All 81 indicators — write results to benchmark DB
npm run bench:db

# Specific indicators by name — stdout only
node src/run_all.js sma
node src/run_all.js ema rsi macd adx

# Specific indicators — write to DB
BENCHMARK_LOG_TO_DB=1 node src/run_all.js ema rsi macd
```

### Example output

```
================================================================
  tulip-rs Node.js Benchmark Suite
================================================================

[1/3] Loading stock data ...
  loaded 6,705 bars  BHP/ASX
  loaded 6,705 bars  CBA/ASX
  loaded 6,705 bars  AAPL/NYSE
  loaded 6,705 bars  MSFT/NYSE

[2/3] DB logging disabled (BENCHMARK_LOG_TO_DB=0) — stdout only

[3/3] Running benchmarks ...

----------------------------------------------------------------
  EMA
----------------------------------------------------------------
  tulip_rs_node        BHP_ASX  [14]      12,942 ns ± 312
  technicalindicators  BHP_ASX  [14]     144,942 ns ± 4,210  × 11.2 slower
  indicatorts          BHP_ASX  [14]      19,981 ns ±   589  ×  1.5 slower
  tulip_rs_node        BHP_ASX  [20]      12,983 ns ± 287
  ...

================================================================
  Finished 81 indicator(s) in 284.7s
================================================================
```

---

## Methodology

| Item | Detail |
|---|---|
| **Reference libraries** | `technicalindicators` (anandanand84) and `indicatorts` (Onur Cinar) — pure JS/TS |
| **Input data** | Real OHLCV market data — **6,705 bars** per asset |
| **Stocks** | BHP/ASX, CBA/ASX, AAPL/NYSE, MSFT/NYSE |
| **Option sets** | 4 per indicator (same sets as the Rust benchmarks); results averaged across all 4 |
| **Timing harness** | `performance.now()` — 10 back-to-back calls per sample, 30 independent samples |
| **Inputs** | `Float64Array` passed to `tulip_rs_node`; reference libraries receive `number[]` (their native format) |
| **Indicator count** | 81 indicators total; reference library coverage is a subset |

!!! info "nAPI boundary cost"
    Each call from JavaScript into the native `tulip_rs_node` binding carries a fixed overhead
    for argument marshalling and `Float64Array` ownership handoff. This cost is roughly
    **4–25 µs** per call depending on the number of input and output series, and is independent
    of indicator complexity.

    The Rust native times shown in the [Standard Performance](standard.md) Node Binding tab
    reflect the underlying computation before this overhead — the gap between "Rust native" and
    "tulip_rs_node" columns is the pure nAPI boundary cost.

!!! info "Reference library coverage"
    Not every indicator has a `technicalindicators` or `indicatorts` equivalent.
    Where neither library implements an indicator, only the `tulip_rs_node` time is recorded.
    The results table on the Standard Performance page shows only the 41 indicators where
    at least one reference library ran.

---

## Reset the Database

```sh
# Stop without losing data
docker compose stop

# Full reset — drops the volume and re-runs init scripts on next start
docker compose down -v
docker compose up -d
```
