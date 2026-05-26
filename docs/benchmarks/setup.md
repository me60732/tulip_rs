# Running the Benchmarks

### Prerequisites

| Requirement | Notes |
|---|---|
| **Rust nightly** | Required — the crate uses `portable_simd`. The repo's `rust-toolchain.toml` selects the right toolchain automatically. |
| **Docker + Docker Compose v2** | For the benchmark database. |
| **TA-Lib 0.4.0** *(optional)* | Only needed for the `--features talib` comparison benchmarks. |

### 1. Start the benchmark database

The benchmarks read real market data from a PostgreSQL database. A Docker Compose
file in `tulip_test/docker/` sets up both databases and loads all seed data
automatically on first boot.

```sh
cd tulip_test/docker
docker compose up -d
# First start takes ~30 s while the SQL seed data loads.
# The `pg_data` Docker volume keeps the data across restarts.
```

The compose file creates two databases:

| Database | Purpose |
|---|---|
| `stocks` | Read-only OHLCV market data (8 symbols, ~6 700 bars each) |
| `indicator_benchmark` | Benchmark run metadata and result storage |

Both are owned by the application user `tulip` (password `tulip`).

### 2. Configure the environment

The benchmarks read connection strings and optional settings from
`tulip_test/.env`. A working default is already in place after cloning:

```sh
# tulip_test/.env — defaults shown, edit only what you need to change
DATABASE_URL=postgres://tulip:tulip@localhost:5432/stocks
BENCHMARK_DATABASE_URL=postgres://tulip:tulip@localhost:5432/indicator_benchmark

# Set to 1 to write timing results into indicator_benchmark on every run.
# This is the preferred way — no shell export needed.
BENCHMARK_LOG_TO_DB=0

# TA-Lib library directory (build.rs reads this when --features talib is set)
TALIB_LIB_DIR=/usr/local/lib
```

!!! tip
    `.env` is loaded at benchmark startup, so any variable you set there is
    picked up automatically — you do **not** need to export anything in your
    shell. A shell export will still override the `.env` value if you need a
    one-off change.

If PostgreSQL is already running on port 5432, set `POSTGRES_PORT` before
starting the container:

```sh
POSTGRES_PORT=5433 docker compose up -d
# then update DATABASE_URL / BENCHMARK_DATABASE_URL in .env to use port 5433
```

### 3. Run the benchmarks

```sh
# All benchmarks (Criterion HTML reports → target/criterion/)
cargo bench --package tulip_test

# Single indicator
cargo bench --package tulip_test --bench benchmark_sma

# With TA-Lib comparison (requires TA-Lib installed — see below)
cargo bench --package tulip_test --features talib

# Log results to indicator_benchmark (set BENCHMARK_LOG_TO_DB=1 in .env, or inline):
# Option A — persistent: edit BENCHMARK_LOG_TO_DB=1 in tulip_test/.env
cargo bench --package tulip_test

# Option B — one-off shell override:
BENCHMARK_LOG_TO_DB=1 cargo bench --package tulip_test
```

!!! tip
    `RUSTFLAGS="-C target-cpu=native"` is set in `.cargo/config.toml` and
    applies automatically to every build — no need to specify it on the command
    line. This enables all native CPU instruction sets (AVX2, AVX-512, etc.)
    so the SIMD paths are exercised and the numbers reflect real-world
    performance.

Criterion writes an HTML report for each benchmark group under
`tulip_test/target/criterion/`. Open any `index.html` to view timing
distributions and history.

### 4. TA-Lib comparison benchmarks *(optional)*

TA-Lib benchmarks are disabled by default so the suite builds without any
extra system dependencies. To enable them:

1. Install TA-Lib 0.4.0 on your system. On most Linux distributions:

    ```sh
    # from the tulip_test/ directory
    bash setup_talib.sh
    # or manually:
    wget https://sourceforge.net/projects/ta-lib/files/ta-lib/0.4.0/ta-lib-0.4.0-src.tar.gz
    tar -xzf ta-lib-0.4.0-src.tar.gz
    cd ta-lib && ./configure --prefix=/usr/local && make -j$(nproc) && sudo make install
    sudo ldconfig
    ```

2. Set `TALIB_LIB_DIR` in `tulip_test/.env` if TA-Lib was installed somewhere
   other than `/usr/local/lib`.

3. Run with the feature flag:

    ```sh
    cargo bench --package tulip_test --features talib
    ```

### 5. Reset the database

```sh
# Stop the container without losing data
docker compose stop

# Full reset — removes the pg_data volume and re-runs init scripts on next start
docker compose down -v
docker compose up -d
```
