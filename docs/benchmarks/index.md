# Benchmark Results

`tulip_rs` is benchmarked against three established technical-analysis libraries across four benchmark suites.

## Competitors

| Library | Language | Description |
|---------|----------|-------------|
| **[Tulip Indicators (C)](https://tulipindicators.org/)** | C | The C library that inspired `tulip_rs`; primary scalar baseline |
| **[RustTa](https://crates.io/crates/rust-ta)** | Rust | Rust implementation with streaming support; 20 indicators benchmarked |
| **[TA-Lib](https://ta-lib.org/)** | C | Industry-standard C technical-analysis library |

All timings are **nanoseconds (ns) — lower is better**. Ratios > 1.00 mean Rust is faster than the competitor.

## Benchmark Suites

| Suite | Description |
|-------|-------------|
| [Standard Performance](standard.md) | Single-asset throughput averaged across 4 option sets, vs C Tulip / RustTa / TA-Lib |
| [SIMD](simd.md) | 4-asset parallel (`by_assets`) and 4-option-set parallel (`by_options`) vectorised performance |
| [Optional Outputs](optional-outputs.md) | Single-pass computation advantage for indicators with sub-indicator outputs |
| [Streaming](streaming.md) | Per-bar stateful update performance vs full batch recomputation |
| [Python Binding](python.md) | `tulip_rs_python` (PyO3) vs `ta` (pandas-based) across 20 indicators |

---

## Methodology

| Item | Detail |
|------|--------|
| **Data source** | `indicator_benchmark` PostgreSQL database — latest run per indicator |
| **Input data** | Real OHLCV market data — **6,705 bars** per asset (single ticker) |
| **Options** | Each indicator is run across **4 option sets** (see reference below); reported time is the average across all sets |
| **Build flags** | `-C opt-level=3 -C lto=fat -C target-cpu=native` |
| **Benchmark harness** | [Criterion.rs](https://github.com/bheisler/criterion.rs) |
| **SIMD lane count** | N = 4 (256-bit AVX2 `f64x4`) |
| **Comparison method** | C libraries called through native FFI; no wrappers inside the timed region |
| **Warm-up** | Criterion warm-up phase runs before measurement to stabilise CPU frequency and cache state |

!!! info "N/A entries"
    "N/A" in a competitor column means that library does not implement the indicator. "—" in a ratio column means the ratio is not applicable (library absent or Rust is the only implementation).

!!! info "RustTa streaming"
    RustTa processes data one bar at a time (streaming) so its timings reflect full 6,705-bar throughput across sequential single-bar calls. See [Streaming](streaming.md) for `tulip_rs` streaming performance.

??? note "Benchmark options reference — all indicators"

    Times are averaged across the following 4 option sets per indicator.
    Indicators with no options run a single configuration.

    | Indicator | Option sets used (averaged) |
    |-----------|-----------------------------|
    | `ad` | *(no options)* |
    | `adosc` | (short=2, long=5) · (short=6, long=20) · (short=5, long=15) · (short=10, long=30) |
    | `adx` | period: 5 · 14 · 24 · 30 |
    | `adxr` | period: 5 · 14 · 24 · 30 |
    | `ao` | *(no options)* |
    | `apo` | (short=2, long=5) · (short=11, long=21) · (short=5, long=11) · (short=14, long=30) |
    | `aroon` | period: 25 · 35 · 50 · 100 |
    | `aroonosc` | period: 25 · 35 · 50 · 100 |
    | `atr` | period: 5 · 14 · 25 · 30 |
    | `avgprice` | *(no options)* |
    | `bbands` | (period=5, mult=2) · (period=14, mult=2) · (period=20, mult=2) · (period=50, mult=2) |
    | `bop` | *(no options)* |
    | `cci` | period: 20 · 25 · 30 · 50 |
    | `chaikinmf` | period: 5 · 14 · 20 · 30 |
    | `chandelierexit` | (period=5, step=2) · (period=14, step=2) · (period=20, step=2) · (period=30, step=2) |
    | `cmo` | period: 5 · 14 · 20 · 30 |
    | `cvi` | period: 5 · 14 · 20 · 30 |
    | `dema` | period: 5 · 14 · 20 · 50 |
    | `di` | period: 5 · 14 · 20 · 30 |
    | `dm` | period: 24 · 14 · 5 · 30 |
    | `donchianchannel` | period: 25 · 35 · 50 · 100 |
    | `dpo` | period: 5 · 14 · 20 · 30 |
    | `dx` | period: 24 · 14 · 5 · 30 |
    | `ef` | period: 5 · 14 · 20 · 30 |
    | `elderray` | period: 5 · 14 · 20 · 30 |
    | `ema` | period: 14 · 20 · 26 · 50 |
    | `emv` | *(no options)* |
    | `fisher` | period: 25 · 35 · 50 · 100 |
    | `fosc` | period: 5 · 14 · 20 · 25 |
    | `hma` | period: 5 · 14 · 20 · 50 |
    | `kama` | period: 5 · 10 · 14 · 20 |
    | `keltnerchannel` | (period=5, step=2) · (period=14, step=2) · (period=20, step=2) · (period=30, step=2) |
    | `kvo` | (short=2, long=5) · (short=9, long=26) · (short=14, long=30) · (short=20, long=50) |
    | `linreg` | period: 5 · 14 · 20 · 25 |
    | `macd` | (fast=5, slow=13, signal=8) · (fast=19, slow=39, signal=9) · (fast=10, slow=30, signal=10) · (fast=6, slow=20, signal=9) |
    | `marketfi` | *(no options)* |
    | `mass` | period: 25 · 30 · 50 · 100 |
    | `max` | period: 25 · 35 · 50 · 100 |
    | `md` | period: 5 · 10 · 14 · 25 |
    | `medprice` | *(no options)* |
    | `mfi` | period: 14 · 20 · 25 · 30 |
    | `min` | period: 25 · 35 · 50 · 100 |
    | `mom` | period: 25 · 30 · 50 · 100 |
    | `msw` | period: 20 · 25 · 30 · 50 |
    | `natr` | period: 14 · 20 · 25 · 30 |
    | `nvi` | *(no options)* |
    | `obv` | *(no options)* |
    | `ppo` | (short=2, long=5) · (short=12, long=26) · (short=9, long=20) · (short=8, long=18) |
    | `psar` | (step=0.02, max=0.2) · (step=0.2, max=2.0) · (step=0.04, max=0.4) · (step=0.4, max=4.0) |
    | `pvi` | *(no options)* |
    | `qstick` | period: 5 · 2 · 8 · 14 |
    | `roc` | period: 25 · 30 · 50 · 100 |
    | `rocr` | period: 25 · 30 · 50 · 100 |
    | `rsi` | period: 14 · 20 · 25 · 30 |
    | `sma` | period: 50 · 100 · 200 · 300 |
    | `smaenvelope` | (period=50, pct=2) · (period=100, pct=2) · (period=200, pct=2) · (period=300, pct=2) |
    | `stddev` | period: 20 · 50 · 100 · 200 |
    | `stoch` | (k=28, ks=16, d=12) · (k=35, ks=21, d=14) · (k=50, ks=30, d=21) · (k=100, ks=50, d=30) |
    | `stochrsi` | period: 14 · 20 · 25 · 35 |
    | `tema` | period: 5 · 14 · 20 · 50 |
    | `tr` | *(no options)* |
    | `trima` | period: 14 · 20 · 25 · 30 |
    | `trvi` | period: 5 · 14 · 20 · 30 |
    | `trix` | period: 14 · 15 · 20 · 30 |
    | `tsf` | period: 5 · 14 · 20 · 25 |
    | `typprice` | *(no options)* |
    | `ultosc` | (s=2, m=3, l=5) · (s=10, m=14, l=20) · (s=14, m=20, l=50) · (s=20, m=50, l=100) |
    | `vhf` | period: 25 · 35 · 50 · 100 |
    | `vidya` | (short=9, long=12, α=0.2) · (short=12, long=26, α=0.2) · (short=14, long=30, α=0.2) · (short=14, long=30, α=0.4) |
    | `volatility` | period: 14 · 20 · 25 · 30 |
    | `vortex` | period: 5 · 14 · 20 · 30 |
    | `vosc` | (short=2, long=5) · (short=5, long=20) · (short=10, long=25) · (short=14, long=28) |
    | `vwma` | period: 14 · 20 · 25 · 30 |
    | `wad` | *(no options)* |
    | `wcprice` | *(no options)* |
    | `wilders` | period: 20 · 25 · 30 · 50 |
    | `willr` | period: 25 · 35 · 50 · 100 |
    | `wma` | period: 14 · 20 · 25 · 30 |
    | `zlema` | period: 5 · 10 · 14 · 20 |

---

## Summary and Key Findings

### Standard (single asset)

- Rust beats C Tulip for **all but 2 indicators**: `dx` (0.75×), `msw` (0.85×)
- Rust beats RustTa for **all 20 compared indicators**
- Rust beats TA-Lib for **all but 3 indicators**: `obv` (0.94×), `psar` (0.76×), `wma` (0.78×)
- **Median C / Rust ratio: ~1.47×** (Rust is ~46% faster on average)
- Largest wins vs C Tulip: `cmo` (**3.61×**), `min` (**3.37×**), `stddev` (**2.88×**), `vhf` (**2.60×**)
- Largest wins vs TA-Lib: `atr` (**6.01×**), `natr` (**5.80×**), `di` (**5.62×**), `rsi` (**5.28×**)

### SIMD by_assets

- **69% of indicators** (55 / 79) show a SIMD speedup over 4× sequential Rust
- Top performers: `tema` (**3.33×**), `cci` (**3.26×**), `dema` (**2.87×**), `trix` (**2.63×**), `zlema` (**2.62×**)
- Indicators that don't benefit are mostly trivial or have inherently sequential dependencies

### SIMD by_options

- **76% of indicators** (50 / 65) show a SIMD speedup over 4× sequential Rust
- Top performers: `tema` (**3.45×**), `dema` (**2.94×**), `zlema` (**2.70×**), `adx` (**2.65×**), `trix` (**2.62×**)
- Same non-benefiting indicators as by_assets — the bottleneck is algorithmic, not data layout

### Optional Outputs Single-Pass

- **35 indicators** offer optional outputs computed in a single pass
- C Tulip would require **2–6× more compute time** to obtain the same information via separate calls
- TA-Lib multipliers are often higher still due to its slower base times
- Standouts: `natr` (**10.68×** vs TA-Lib), `tema` (**8.35×** vs TA-Lib), `di` (**7.81×** vs TA-Lib), `ppo` (**7.16×** vs TA-Lib)

### Combined Advantage: a Worked Example

Consider computing `tema` with all its sub-indicators (`dema`, `ema`) across **4 assets simultaneously with SIMD**:

| Implementation | Time |
|----------------|-----:|
| `tulip_rs` SIMD 4-asset (all outputs, one pass) | **8,099 ns** |
| C Tulip — 4 × (tema + dema + ema) sequential | 94,408 ns |
| TA-Lib — 4 × (tema + dema + ema) sequential | 258,992 ns |

!!! success "Combined speedup"
    **~11.7× faster than C Tulip** and **~32.0× faster than TA-Lib** for the same result.

### Streaming

- **All 79 indicators** implement the stateful streaming path
- **Median streaming time: ~27 ns** per bar update
- **Fastest** — `ao`, `ema`, `typprice`, `wilders` at **14 ns/bar**
- **Slowest** — `msw` at **114 ns/bar**
- The streaming path is **100–5,489× faster** than full batch recompute depending on the indicator

### Python Binding

- **20 indicators** benchmarked: `tulip_rs_python` vs `ta` (pandas-based)
- `ta` falls back to **pure-Python loops** for many indicators — in these cases `tulip_rs_python` is **160–1,812× faster**
- `ta` uses **pandas/numpy C paths** for a few indicators (EMA, BBands, MACD) — `tulip_rs_python` is **3–5× faster** there, with PyO3 call overhead narrowing the gap
- **Median speedup: ~22×** across all 20 indicators
- The Python binding adds **~5–25 µs** of fixed per-call overhead (GIL + PyO3 marshalling) on top of the native Rust computation
