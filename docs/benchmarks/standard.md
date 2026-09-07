# Standard Performance

Single asset, averaged across 4 option sets. Ratios show how many times slower the competitor is relative to Rust — higher means Rust wins by more.

---

## Rust

Full comparison tables (vs C / TA-Lib, RustTa, Kand): **[Standard Performance →](standard/rust.md)**

??? success "Notable results"

    Rust beats C Tulip on **65 of 71 indicators (92%)**.
    Rust beats TA-Lib on **36 of 39 indicators (92%)**.

    | Category | Indicator | Speedup vs C | Speedup vs TA-Lib |
    |----------|-----------|:------------:|:-------------------:|
    | **Largest wins vs C Tulip** | `msw` | **11.27x** | --- |
    | **Largest wins vs C Tulip** | `min` | **8.07x** | --- |
    | **Largest wins vs C Tulip** | `max` | **5.87x** | --- |
    | **Largest wins vs C Tulip** | `pvi` | **4.49x** | --- |
    | **Largest wins vs C Tulip** | `donchianchannel` | **4.38x** | --- |
    | **Largest wins vs TA-Lib** | `hilberttransform` | --- | **14.25x** |
    | **Largest wins vs TA-Lib** | `macd` | --- | **5.67x** |
    | **Largest wins vs TA-Lib** | `atr` | --- | **5.65x** |
    | **Largest wins vs TA-Lib** | `natr` | --- | **5.50x** |
    | **Largest wins vs TA-Lib** | `tema` | --- | **4.99x** |
    | **Rust slower than C Tulip** | `di` | **0.62x** | --- |
    | **Rust slower than C Tulip** | `dm` | **0.70x** | --- |
    | **Rust slower than C Tulip** | `dx` | **0.75x** | --- |
    | **Rust slower than C Tulip** | `ao` | **0.90x** | --- |
    | **Rust slower than C Tulip** | `cci` | **0.95x** | --- |
    | **Rust slower than C Tulip** | `obv` | **0.99x** | --- |
    | **Rust slower than TA-Lib** | `wma` | --- | **0.79x** |
    | **Rust slower than TA-Lib** | `homodynediscriminator` | --- | **0.94x** |
    | **Rust slower than TA-Lib** | `psar` | --- | **0.95x** |


??? success "Notable results"

    Rust beats RustTa on **18 of 19 compared indicators**.
    Rust beats Kand on **32 of 33 compared indicators**.

    | Category | Indicator | Speedup vs RustTa | Speedup vs Kand |
    |----------|-----------|:------------------:|:----------------:|
    | **Largest wins vs RustTa** | `ef` | **6.78x** | --- |
    | **Largest wins vs RustTa** | `tr` | **5.22x** | --- |
    | **Largest wins vs RustTa** | `min` | **3.62x** | --- |
    | **Largest wins vs RustTa** | `mfi` | **2.98x** | 36.10x |
    | **Largest wins vs RustTa** | `max` | **2.73x** | --- |
    | **Largest wins vs Kand** | `mfi` | 2.98x | **36.10x** |
    | **Largest wins vs Kand** | `stoch` | 2.25x | **33.22x** |
    | **Largest wins vs Kand** | `willr` | --- | **21.24x** |
    | **Largest wins vs Kand** | `aroon` | --- | **19.97x** |
    | **Largest wins vs Kand** | `aroonosc` | --- | **19.24x** |
    | **RustTa faster** | `cci` | **0.95x** | 0.91x |
    | **Kand faster** | `cci` | 0.95x | **0.91x** |

---

## Python

Full comparison table (tulip_rs_python vs ta): **[Standard Performance →](standard/python.md)**

??? success "Notable results"

    `tulip_rs_python` beats `ta` on **35 of 35 compared indicators**.
    Median speedup: **~28×**.

    | Indicator | ta / Python |
    |-----------|:-----------:|
    | `psar` | **8154×** |
    | `nvi` | **7913×** |
    | `mfi` | **2435×** |
    | `atr` | **1599×** |
    | `hma` | **744×** |
    | `adx` | **689×** |
    | `wma` | **466×** |
    | `kama` | **390×** |
    | `aroon` | **335×** |
    | `cci` | **306×** |

    The following indicators show the smallest gap, likely because `ta` uses a compiled numpy/pandas path rather than a pure-Python loop:

    | Indicator | Rust native (ns) | tulip_rs_python (ns) | ta (ns) | ta / Python |
    |-----------|----------------:|---------------------:|--------:|:-----------:|
    | `ema` | 4,790 | 6,709 | 57,441 | **9×** |
    | `stoch` | 21,788 | 27,252 | 298,106 | **11×** |
    | `mom` | 897 | 1,924 | 21,808 | **11×** |
    | `sma` | 2,445 | 6,155 | 79,650 | **13×** |
    | `willr` | 16,465 | 19,971 | 295,277 | **15×** |
    | `trix` | 6,197 | 17,067 | 303,990 | **18×** |

    `tulip_rs_python` beats `pandas_ta` on **72 of 72 compared indicators**.
    Median speedup: **~45×**.

    | Indicator | pandas_ta / Python |
    |-----------|:-----------------:|
    | `vidya` | **4737×** |
    | `linreg` | **1864×** |
    | `supertrend` | **1658×** |
    | `md` | **1110×** |
    | `kama` | **1083×** |
    | `tsf` | **1039×** |
    | `aroonosc` | **530×** |
    | `aroon` | **442×** |
    | `cci` | **262×** |
    | `psar` | **210×** |

---

## Node

Full comparison table (tulip_rs_node vs technicalindicators / indicatorts): **[Standard Performance →](standard/node.md)**

??? success "Notable results"

    `tulip_rs_node` beats every JS competitor on **23 compared indicators** against technicalindicators and **36 compared indicators** against indicatorts.
    Median speedup: **57.33x vs technicalindicators**
    Median speedup: **7.00x vs indicatorts**

    **Largest wins vs technicalindicators** (pure JS, loop-heavy implementations):

    | Indicator | Speedup vs technicalindicators |
    |-----------|--------------------------------:|
    | `bbands` | **411.52x** |
    | `wma` | **380.84x** |
    | `stochrsi` | **164.30x** |
    | `mfi` | **152.41x** |
    | `ao` | **124.52x** |
    | `rsi` | **119.51x** |
    | `roc` | **89.40x** |
    | `adx` | **85.21x** |
    | `willr` | **75.73x** |
    | `trix` | **73.87x** |

    **Largest wins vs indicatorts** (pure TypeScript):

    | Indicator | Speedup vs indicatorts |
    |-----------|------------------------:|
    | `max` | **48.78x** |
    | `willr` | **43.02x** |
    | `chandelierexit` | **39.41x** |
    | `donchianchannel` | **38.45x** |
    | `min` | **35.59x** |
    | `tr` | **35.01x** |
    | `atr` | **28.53x** |
    | `aroon` | **24.86x** |
    | `mfi` | **20.79x** |
    | `bbands` | **20.49x** |

    **nAPI boundary overhead** — the gap between Rust native and `tulip_rs_node` columns reflects the fixed per-call cost of the nAPI boundary (argument marshalling, `Float64Array` handoff), roughly **4–6 µs** for the fastest-running indicators:

    | Indicator | Rust native (ns) | tulip_rs_node (ns) | Overhead |
    |-----------|----------------:|------------------:|---------:|
    | `avgprice` | 1,433 | 5,511 | ~3.8× |
    | `tr` | 1,501 | 7,093 | ~4.7× |
    | `wcprice` | 1,062 | 5,446 | ~5.1× |
    | `typprice` | 1,036 | 5,915 | ~5.7× |
    | `mom` | 897 | 5,173 | ~5.8× |
    | `medprice` | 872 | 5,538 | ~6.4× |
