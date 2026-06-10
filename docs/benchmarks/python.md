# Python Binding — Running the Benchmarks

The `tulip_rs_python` vs `ta` benchmark results are part of the
[Standard Performance](standard.md) page — select the **Python Binding** tab there to see the full comparison table.

This page covers how to install the bench package and run it locally.

---

## Setup

### Prerequisites

| Requirement | Notes |
|---|---|
| **Python 3.9+** | With `tulip_rs_python` installed (`maturin develop --release` or from PyPI) |
| **Docker + Docker Compose v2** | For the benchmark databases |
| `uv` or `pip` | To install the `tulip-rs-bench` package |

### 1. Start the databases

The Python benchmarks use the same PostgreSQL instance as the Rust benchmarks.
The Python benchmark views (`python_performance_comparison`, `python_avg_options_comparison`)
are included in the main compose file and applied automatically on first boot.

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

### 2. Install the bench package

```sh
cd tulip_rs_python/bench
pip install -e .
# or with uv:
uv pip install -e .
```

This installs the `tulip-rs-bench` console script.

### 3. Configure the environment

Create or edit `tulip_rs_python/bench/.env`:

```sh
# Connection to the stocks database (read-only market data)
DATABASE_URL=postgres://tulip:tulip@localhost:5432/stocks

# Connection to the benchmark results database
BENCHMARK_DATABASE_URL=postgres://tulip:tulip@localhost:5432/indicator_benchmark

# Set to 1 to write timing rows into indicator_benchmark after each run.
BENCHMARK_LOG_TO_DB=0
```

!!! tip
    `python-dotenv` walks upward from the working directory to find `.env`, so
    running `tulip-rs-bench` from `tulip_rs_python/bench/` picks it up
    automatically. No shell exports needed.

---

## Running

```sh
cd tulip_rs_python/bench

# All 20 indicators — stdout only
tulip-rs-bench

# Single indicator
tulip-rs-bench ema

# Multiple indicators
tulip-rs-bench ema rsi macd adx

# Write results to the benchmark database
BENCHMARK_LOG_TO_DB=1 tulip-rs-bench

# Equivalent alternative (module entry point)
python -m tulip_rs_bench.run_all
```

### Example output

```
================================================================
  tulip-rs Python Benchmark Suite
================================================================

[1/3] Loading stock data ...
  loaded 6,705 bars  BHP/ASX
  loaded 6,705 bars  CBA/ASX
  loaded 6,705 bars  AAPL/NYSE
  loaded 6,705 bars  MSFT/NYSE

[2/3] DB logging disabled (BENCHMARK_LOG_TO_DB=0) -- stdout only

[3/3] Running benchmarks ...

----------------------------------------------------------------
  EMA
----------------------------------------------------------------
    tulip_rs_python      BHP_ASX        [14]      11,432 ns +/- 254
    ta                   BHP_ASX        [14]      53,928 ns +/- 2,389  x4.7 slower than tulip_rs
    tulip_rs_python      BHP_ASX        [20]      11,434 ns +/- 233
    ta                   BHP_ASX        [20]      53,717 ns +/- 1,985  x4.7 slower than tulip_rs
    ...

================================================================
  Finished 20 indicator(s) in 142.3s
================================================================
```

---

## Methodology

| Item | Detail |
|---|---|
| **Reference library** | `ta` (bukosabino/ta) — pandas-based; calculations use `pd.Series` internally |
| **Input data** | Real OHLCV market data — **6,705 bars** per asset |
| **Stocks** | BHP/ASX, CBA/ASX, AAPL/NYSE, MSFT/NYSE |
| **Option sets** | 4 per indicator (same sets as the Rust benchmarks); results averaged across all 4 |
| **Timing harness** | Python `timeit` — 10 back-to-back calls per sample, 30 independent samples |
| **Pre-conversion** | `pd.Series` (`ta`) and contiguous `np.ndarray` (`tulip_rs_python`) are built **before** the timed region — data-conversion cost is excluded |
| **Indicator count** | 35 indicators — the subset that `ta` implements |

!!! info "PyO3 call overhead"
    Each call from Python into Rust carries a fixed overhead of roughly **5–25 µs** for GIL
    acquisition, argument marshalling, and numpy array allocation for the outputs. This cost
    is independent of indicator complexity and dominates timings for trivially fast indicators
    (SMA, MOM, ROC) where the actual Rust computation is sub-microsecond.

    The Rust native times in the results table show the underlying computation cost before
    this overhead is added — the gap between "Rust native" and "tulip_rs_python" columns
    is the pure PyO3 boundary cost.

---

## Reset the Database

```sh
# Stop without losing data
docker compose stop

# Full reset — drops the volume and re-runs init scripts on next start
docker compose down -v
docker compose up -d
```
