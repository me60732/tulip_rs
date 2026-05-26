# TulipRS

**High-performance technical analysis in Rust.**

TulipRS implements 100+ technical indicators and 77+ candlestick patterns with
first-class SIMD acceleration. Process multiple assets or multiple parameter
sets in a single CPU pass, stream live bars into stateful indicators without
reprocessing history, and call everything from Rust or Python with the same
universal API.

📖 **[Full documentation](https://me60732.github.io/tulip_rs)**

---

## Why TulipRS?

| | TulipRS | Tulip C / TA-Lib |
|---|---|---|
| **SIMD — multiple assets** | ✅ N assets in one pass | ❌ one asset at a time |
| **SIMD — multiple options** | ✅ N parameter sets in one pass | ❌ one parameter set at a time |
| **Stateful streaming** | ✅ resume from `IndicatorState` | ❌ full recompute each tick |
| **Optional outputs** | ✅ free in the same pass | ❌ separate call + full scan |
| **Language bindings** | Rust + Python (more planned) | C, various wrappers |

When optional intermediate outputs are needed (sub-EMAs, TR, AD line, etc.)
TulipRS is **1.3× – 8.7× faster** than running the equivalent TA-Lib calls.

---

## Installation

### Rust

Add TulipRS to your `Cargo.toml`. The crate is distributed via Git:

```toml
[dependencies]
tulip_rs = { git = "https://github.com/me60732/tulip_rs" }
```

> **Nightly required.** TulipRS uses `portable_simd`. The correct nightly
> version is pinned automatically by `rust-toolchain.toml` — no manual
> toolchain management needed.

To disable the SIMD multi-asset / multi-option variants (reduces compile time):

```toml
tulip_rs = { git = "https://github.com/me60732/tulip_rs", default-features = false }
```

### Python

```bash
pip install tulip-rs
```

Build from source with native CPU optimisations:

```bash
git clone https://github.com/me60732/tulip_rs_python
cd tulip_rs_python
RUSTFLAGS="-C target-cpu=native" maturin develop --release
```

---

## Quick Start

Every indicator follows the same signature — learn it once, use it everywhere.

### Rust

```rust
use tulip_rs::indicators::ema::indicator;

let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                 83.15, 82.84, 83.99, 84.55, 84.36_f64];

// inputs: &[&[f64]]  |  options: &[f64]  |  optional_outputs: Option<&[bool]>
let (outputs, state) = indicator(&[close.as_slice()], &[5.0], None).unwrap();

println!("{:?}", outputs[0]); // EMA(5) values

// Streaming: feed new bars without reprocessing history
let new_bar = [85.10_f64];
let (next_outputs, next_state) = state.batch_indicator(&[&new_bar], None).unwrap();
```

### Python

```python
import numpy as np
import tulip_rs

close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                  83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

outputs, state = tulip_rs.indicators.ema.indicator([close], [5.0])
print(outputs[0])  # EMA(5) values

# Streaming
next_outputs, next_state = state.batch_indicator([np.array([85.10])], None)
```

### SIMD — same indicator, N assets at once (Rust)

```rust
use tulip_rs::indicators::ema::indicator_by_assets;

// 4 assets processed simultaneously in one CPU pass
let inputs = [asset1.as_slice(), asset2.as_slice(),
              asset3.as_slice(), asset4.as_slice()];

let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
```

---

## Benchmarks

Benchmarks compare TulipRS (Rust scalar, Rust SIMD) against the reference
C implementation (Tulip Indicators) and TA-Lib across 8 real market symbols.

→ **[Benchmark results](https://me60732.github.io/tulip_rs/benchmarks/results/)**
→ **[How to run the benchmarks](https://me60732.github.io/tulip_rs/benchmarks/setup/)**

---

## Documentation

| Page | Description |
|---|---|
| [Getting Started](https://me60732.github.io/tulip_rs/getting_started/) | Installation, feature flags, calling convention, first examples |
| [Indicators — Overview](https://me60732.github.io/tulip_rs/indicators/) | Full indicator index with inputs, options, and output counts |
| [Indicator API](https://me60732.github.io/tulip_rs/indicators/indicator_api/) | `info()`, optional outputs, `min_data`, `min_data_accuracy` |
| [SIMD](https://me60732.github.io/tulip_rs/simd/) | By-assets and by-options modes, lane counts, when to use each |
| [State Management](https://me60732.github.io/tulip_rs/state_management/) | Streaming computation, chunked processing, JSON serialisation |
| [Candlestick Patterns](https://me60732.github.io/tulip_rs/candlestick_patterns/) | 60+ patterns with bullish/bearish forecasting |
| [Language Bindings](https://me60732.github.io/tulip_rs/language_bindings/) | Python (PyO3/maturin) details and planned bindings |

---

## Language Support

| Language | Status | Package |
|---|---|---|
| **Rust** | ✅ Native | `tulip_rs` (this repo) |
| **Python** | ✅ Supported | [`tulip_rs_python`](https://github.com/me60732/tulip_rs_python) · `pip install tulip-rs` |
| Node.js / WASM | 🔜 Planned | — |
| R | 🔜 Planned | — |
| Julia | 🔜 Planned | — |

---

## License

[MIT](LICENSE)
