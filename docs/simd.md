# SIMD Acceleration

## What is SIMD?

SIMD stands for **Single Instruction, Multiple Data**. Modern CPUs can execute one instruction that simultaneously operates on 2, 4, 8, or 16 floating-point values packed into a single wide register. A 256-bit AVX2 register holds four `f64` values, and a 512-bit AVX-512 register holds eight. By structuring data to fill these registers, TulipRS achieves throughput that is proportionally higher than scalar (one-at-a-time) computation.

TulipRS uses Rust's **`portable_simd`** API (nightly feature), which generates the correct intrinsics for the target CPU at compile time rather than relying on hand-written platform-specific code. This means the same source compiles to SSE2, AVX2, or AVX-512 depending on what the build machine supports.

---

## Two SIMD Modes

TulipRS exposes SIMD through two complementary strategies, each suited to a different use case.

### By Assets — `indicator_by_assets::<N>`

**The same options are applied to N different assets in a single CPU pass.**

All N assets' data streams are interleaved into SIMD lanes. One call to `indicator_by_assets::<4>` is not four sequential scalar calls — it is a single vectorised sweep that produces results for all four assets at once, using only a fraction more memory bandwidth than a single scalar call.

**Typical use case:** Portfolio scanning, universe filtering, backtesting across many symbols with the same parameters.

#### Mismatched bar counts

Assets passed to `indicator_by_assets` do **not** need to have the same number of bars. A newly-listed stock with 200 days of history can be processed in the same call as an asset with 5,000 bars. This is handled internally by the `PrimeMover` scheduler (`road_train` module — inspired by the road trains of [Outback Truckers](https://www.youtube.com/channel/UCps44nJRcJFdw9N32k8pmMA), where multiple trailers hitch behind a single cab and are picked up along the route).

**Performance implication:** the closer the bar counts are across your N assets, the higher the overall throughput. When scanning a universe where bar counts vary widely it is worth grouping assets by similar history length before passing them in batches.

### By Options — `indicator_by_options::<N>`

**N different option sets are applied to the same asset in a single CPU pass.**

The same data stream feeds N parallel computations with different parameters. `indicator_by_options::<4>` computes SMA(50), SMA(100), SMA(200), and SMA(300) in one pass over the close price series.

**Typical use case:** Parameter sweeps, multi-timeframe analysis on one symbol, optimisation loops.

---

## Choosing Lane Count N

`N` must be a **power of 2**. The hardware lane count for `f64` depends on the available instruction set:

| Instruction Set | Register Width | f64 Lanes |
|---|---|---|
| SSE2 (baseline x86-64) | 128-bit | 2 |
| AVX / AVX2 | 256-bit | **4** ← most common sweet spot |
| AVX-512 | 512-bit | 8 |

!!! tip "Start with N = 4"
    On the vast majority of modern desktop and server CPUs, `N = 4` fills a 256-bit AVX2 register exactly and gives the best performance-to-portability tradeoff. Use `N = 8` only when you have confirmed AVX-512 support on the target machine.

Values of 2, 4, 8, and 16 are all supported. If you request a lane count wider than your CPU's registers, the compiler will fold multiple instructions together — you still get correctness, but not peak throughput.

---

## Feature Flags

Both SIMD modes are compiled in by default. You can disable them independently:

| Feature | Default | What it enables |
|---|---|---|
| `simd_assets` | ✅ on | `indicator_by_assets::<N>` for every indicator |
| `simd_options` | ✅ on | `indicator_by_options::<N>` for every indicator |

To turn off a mode, disable the default features and re-enable only what you need:

```toml
[dependencies]
tulip_rs = { git = "https://github.com/me60732/tulip_rs", default-features = false, features = ["simd_assets"] }
```

Both modes require a nightly toolchain. `portable_simd` is a core language feature used throughout the crate and is always active — it does not need to be enabled separately.

---

## Build Configuration

TulipRS's build script emits `-C target-cpu=native` when compiling with SIMD features enabled. This instructs LLVM to use the widest vector registers available on the machine performing the build. Distributing a binary compiled with `target-cpu=native` to a machine with a different (older) CPU may cause an illegal instruction fault — for portable distribution, either disable the SIMD features or use a lower baseline like `target-cpu=x86-64-v3`.

---

## When to Use Each Mode

| Scenario | Recommended Mode |
|---|---|
| Scanning 500 symbols for RSI crossovers with a fixed period | `by_assets` |
| Running a MACD grid search (12 fast × 8 slow × 5 signal combinations) | `by_options` |
| Evaluating the same SMA period across correlated assets | `by_assets` |
| Multi-timeframe dashboard (SMA 50/100/200/400 on one chart) | `by_options` |
| Simple one-off calculation on a single symbol | Scalar `indicator()` |

---

## Code Examples

Code examples for both SIMD modes are included on each indicator's own documentation page alongside the scalar example. See the [Moving Averages](indicators/moving_averages.md), [Oscillators](indicators/oscillators.md), and other indicator pages for concrete usage patterns.
