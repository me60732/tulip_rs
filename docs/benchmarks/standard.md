# Standard Performance

Single asset, averaged across 4 option sets. Ratios show how many times slower the competitor is relative to Rust — higher means Rust wins by more.

---

## Rust

Full comparison tables (vs C / TA-Lib, RustTa, Kand): **[Standard Performance →](standard/rust.md)**

??? success "Notable results"

    Rust beats C Tulip on **65 of 70 indicators (93%)**.
    Rust beats TA-Lib on **36 of 38 indicators (95%)**.

    | Category | Indicator | Speedup vs C | Speedup vs TA-Lib |
    |----------|-----------|:------------:|:-------------------:|
    | **Largest wins vs C Tulip** | `msw` | **11.38x** | --- |
    | **Largest wins vs C Tulip** | `min` | **8.07x** | --- |
    | **Largest wins vs C Tulip** | `max` | **5.87x** | --- |
    | **Largest wins vs C Tulip** | `vhf` | **5.68x** | --- |
    | **Largest wins vs C Tulip** | `willr` | **4.99x** | --- |
    | **Largest wins vs TA-Lib** | `hilberttransform` | --- | **14.25x** |
    | **Largest wins vs TA-Lib** | `macd` | --- | **5.67x** |
    | **Largest wins vs TA-Lib** | `atr` | --- | **5.65x** |
    | **Largest wins vs TA-Lib** | `natr` | --- | **5.50x** |
    | **Largest wins vs TA-Lib** | `rsi` | --- | **5.25x** |
    | **Rust slower than C Tulip** | `di` | **0.62x** | --- |
    | **Rust slower than C Tulip** | `dm` | **0.70x** | --- |
    | **Rust slower than C Tulip** | `dx` | **0.75x** | --- |
    | **Rust slower than C Tulip** | `ao` | **0.90x** | --- |
    | **Rust slower than C Tulip** | `obv` | **0.99x** | --- |
    | **Rust slower than TA-Lib** | `homodynediscriminator` | --- | **0.94x** |
    | **Rust slower than TA-Lib** | `psar` | --- | **0.95x** |


??? success "Notable results"

    Rust beats RustTa on **19 of 19 compared indicators**.
    Rust beats Kand on **36 of 36 compared indicators**.

    | Category | Indicator | Speedup vs RustTa | Speedup vs Kand |
    |----------|-----------|:------------------:|:----------------:|
    | **Largest wins vs RustTa** | `ef` | **6.78x** | --- |
    | **Largest wins vs RustTa** | `tr` | **5.65x** | 1.60x |
    | **Largest wins vs RustTa** | `min` | **3.62x** | --- |
    | **Largest wins vs RustTa** | `mfi` | **2.98x** | 36.10x |
    | **Largest wins vs RustTa** | `max` | **2.73x** | --- |
    | **Largest wins vs Kand** | `mfi` | 2.98x | **36.10x** |
    | **Largest wins vs Kand** | `willr` | --- | **20.79x** |
    | **Largest wins vs Kand** | `aroon` | --- | **19.97x** |
    | **Largest wins vs Kand** | `stoch` | 2.41x | **19.96x** |
    | **Largest wins vs Kand** | `aroonosc` | --- | **19.24x** |

---

## Python

Full comparison table (tulip_rs_python vs ta): **[Standard Performance →](standard/python.md)**

??? success "Notable results"

    `tulip_rs_python` beats `ta` on **35 of 35 compared indicators**.
    Median speedup: **~27×**.

    | Indicator | ta / Python |
    |-----------|:-----------:|
    | `nvi` | **7072×** |
    | `psar` | **6818×** |
    | `mfi` | **2803×** |
    | `adx` | **956×** |
    | `atr` | **808×** |
    | `wma` | **554×** |
    | `hma` | **486×** |
    | `aroon` | **293×** |
    | `cci` | **279×** |
    | `kama` | **200×** |

    The following indicators show the smallest gap, likely because `ta` uses a compiled numpy/pandas path rather than a pure-Python loop:

    | Indicator | Rust native (ns) | tulip_rs_python (ns) | ta (ns) | ta / Python |
    |-----------|----------------:|---------------------:|--------:|:-----------:|
    | `macd` | 6,331 | 24,354 | 197,360 | **8×** |
    | `ema` | 4,784 | 6,102 | 57,546 | **9×** |
    | `stoch` | 20,383 | 31,356 | 299,433 | **10×** |
    | `roc` | 2,437 | 7,616 | 105,768 | **14×** |
    | `willr` | 17,282 | 21,444 | 298,910 | **14×** |
    | `mom` | 897 | 1,340 | 20,291 | **15×** |

    `tulip_rs_python` beats `pandas_ta` on **72 of 72 compared indicators**.
    Median speedup: **~42×**.

    | Indicator | pandas_ta / Python |
    |-----------|:-----------------:|
    | `vidya` | **3912×** |
    | `supertrend` | **1488×** |
    | `tsf` | **1026×** |
    | `linreg` | **723×** |
    | `md` | **550×** |
    | `kama` | **542×** |
    | `aroonosc` | **384×** |
    | `aroon` | **359×** |
    | `cci` | **237×** |
    | `psar` | **169×** |

---

## Node

Full comparison table (tulip_rs_node vs technicalindicators / indicatorts): **[Standard Performance →](standard/node.md)**

??? success "Notable results"

    `tulip_rs_node` beats every JS competitor on **23 compared indicators** against technicalindicators and **36 compared indicators** against indicatorts.
    Median speedup: **56.71x vs technicalindicators**
    Median speedup: **6.62x vs indicatorts**

    **Largest wins vs technicalindicators** (pure JS, loop-heavy implementations):

    | Indicator | Speedup vs technicalindicators |
    |-----------|--------------------------------:|
    | `bbands` | **405.37x** |
    | `wma` | **370.83x** |
    | `mfi` | **154.82x** |
    | `stochrsi` | **154.66x** |
    | `ao` | **135.94x** |
    | `rsi` | **120.13x** |
    | `roc` | **100.39x** |
    | `adx` | **96.04x** |
    | `willr` | **82.45x** |
    | `trix` | **73.85x** |

    **Largest wins vs indicatorts** (pure TypeScript):

    | Indicator | Speedup vs indicatorts |
    |-----------|------------------------:|
    | `max` | **52.04x** |
    | `willr` | **42.76x** |
    | `tr` | **40.08x** |
    | `donchianchannel` | **39.34x** |
    | `chandelierexit` | **38.45x** |
    | `min` | **33.46x** |
    | `atr` | **30.45x** |
    | `aroon` | **26.11x** |
    | `mfi` | **19.90x** |
    | `bbands` | **19.00x** |

    **nAPI boundary overhead** — the gap between Rust native and `tulip_rs_node` columns reflects the fixed per-call cost of the nAPI boundary (argument marshalling, `Float64Array` handoff), roughly **4–6 µs** for the fastest-running indicators:

    | Indicator | Rust native (ns) | tulip_rs_node (ns) | Overhead |
    |-----------|----------------:|------------------:|---------:|
    | `tr` | 1,393 | 5,899 | ~4.2× |
    | `avgprice` | 1,433 | 7,315 | ~5.1× |
    | `mom` | 897 | 4,933 | ~5.5× |
    | `typprice` | 1,086 | 6,193 | ~5.7× |
    | `wcprice` | 1,075 | 6,609 | ~6.1× |
    | `medprice` | 872 | 5,559 | ~6.4× |

---

## C

Full comparison table (tulip_rs_ffi_c vs Tulip Indicators C / TA-Lib): **[Standard Performance →](standard/c.md)**

??? success "Notable results"

    `tulip_rs_ffi_c` beats `C_tulip` on **46 of 72 compared indicators** (of 91 indicators benchmarked via the FFI).
    Median speedup vs C_tulip: **1.62x**.

    **Largest wins vs C_tulip:**

    | Indicator | Speedup vs C | Speedup vs TA-Lib |
    |-----------|-------------:|------------------:|
    | `cmo` | **4.95x** | 3.77x |
    | `msw` | **4.54x** | --- |
    | `mfi` | **3.55x** | 2.75x |
    | `rsi` | **3.40x** | 4.75x |
    | `willr` | **2.91x** | 1.67x |
    | `cvi` | **2.85x** | --- |
    | `aroon` | **2.76x** | 1.38x |
    | `stoch` | **2.70x** | 1.73x |
    | `aroonosc` | **2.48x** | 1.19x |
    | `ema` | **2.19x** | 2.18x |

    Smallest margins (C_tulip's vectorised or state-light paths):

    | Indicator | Rust native (ns) | tulip_rs_ffi_c (ns) | C Tulip (ns) | Speedup vs C |
    |----------:|-----------------:|----------------------:|-------------:|-------------:|
    | `qstick` | 2,571 | 2,693 | 2,729 | **1.01×** |
    | `md` | 13,911 | 15,523 | 15,890 | **1.02×** |
    | `hma` | 9,349 | 8,876 | 9,404 | **1.06×** |
    | `dema` | 6,336 | 5,874 | 6,432 | **1.09×** |
    | `fisher` | 48,681 | 77,711 | 85,314 | **1.10×** |
    | `wcprice` | 1,075 | 1,187 | 1,304 | **1.10×** |

    **FFI boundary overhead** — `tulip_rs_ffi_c` tracks native Rust closely: for the fastest-running indicators the call overhead is 1.1–1.8× Rust native (argument marshalling only, no runtime):

    | Indicator | Rust native (ns) | tulip_rs_ffi_c (ns) | Overhead |
    |-----------|----------------:|----------------------:|---------:|
    | `mom` | 897 | 967 | ~1.1× |
    | `wcprice` | 1,075 | 1,187 | ~1.1× |
    | `typprice` | 1,086 | 1,376 | ~1.3× |
    | `tr` | 1,393 | 1,900 | ~1.4× |
    | `medprice` | 872 | 1,237 | ~1.4× |
    | `avgprice` | 1,433 | 2,567 | ~1.8× |

---

## Go

Full comparison table (tulip_rs_go vs cinar/indicator/v2): **[Standard Performance →](standard/go.md)**

??? success "Notable results"

    `tulip_rs_go` beats `cinar` on **40 of 40 compared indicators** (cinar is wired where it is param-compatible; the remaining indicators have no pure-Go competitor run).
    Median speedup: **1,112×** — cinar v2's goroutine/channel stream API pays a per-bar scheduling cost the cgo path does not.

    **Largest wins vs cinar:**

    | Indicator | Speedup vs cinar |
    |-----------|-----------------:|
    | `dpo` | **3,001×** |
    | `ao` | **2,548×** |
    | `typprice` | **2,476×** |
    | `tr` | **2,430×** |
    | `emv` | **2,357×** |
    | `nvi` | **2,214×** |
    | `kama` | **1,741×** |
    | `adosc` | **1,736×** |
    | `tema` | **1,660×** |
    | `mfi` | **1,439×** |

    Smallest margins (where cinar's implementation is closest, or computes fewer output rows):

    | Indicator | Rust native (ns) | tulip_rs_go (ns) | cinar (ns) | Speedup vs cinar |
    |----------:|-----------------:|-----------------:|-----------:|-----------------:|
    | `ema` | 4,784 | 11,431 | 2,522,108 | **221×** |
    | `cci` | 78,669 | 97,465 | 31,323,732 | **321×** |
    | `stddev` | 3,795 | 15,844 | 5,261,197 | **332×** |
    | `fisher` | 48,681 | 72,406 | 24,455,999 | **338×** |
    | `aroon` | 18,771 | 35,676 | 13,911,132 | **390×** |
    | `apo` | 4,872 | 8,481 | 3,473,150 | **410×** |

    **cgo boundary overhead** — the gap between Rust native and `tulip_rs_go` columns reflects the fixed per-call cost of the cgo boundary (argument marshalling, pointer handoff), 1.8–3.8× for the fastest-running indicators:

    | Indicator | Rust native (ns) | tulip_rs_go (ns) | Overhead |
    |-----------|----------------:|-----------------:|---------:|
    | `roc` | 2,437 | 4,450 | ~1.8× |
    | `tr` | 1,393 | 2,729 | ~2.0× |
    | `sma` | 2,558 | 6,001 | ~2.3× |
    | `typprice` | 1,086 | 2,913 | ~2.7× |
    | `bop` | 2,440 | 8,234 | ~3.4× |
    | `emv` | 2,449 | 9,278 | ~3.8× |
