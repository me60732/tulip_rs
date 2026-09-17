# Standard Performance

Single asset, averaged across 4 option sets. Ratios show how many times slower the competitor is relative to Rust — higher means Rust wins by more.

---

## Rust

Full comparison tables (vs C / TA-Lib, RustTa, Kand): **[Standard Performance →](standard/rust.md)**

??? success "Notable results"

    Rust beats C Tulip on **64 of 69 indicators (93%)**.
    Rust beats TA-Lib on **35 of 37 indicators (95%)**.

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

    Rust beats RustTa on **18 of 18 compared indicators**.
    Rust beats Kand on **35 of 35 compared indicators**.

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
    | `wma` | **370.83x** |
    | `bbands` | **318.96x** |
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
    | `stoch` | **17.05x** |

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

    `tulip_rs_ffi_c` beats `C_tulip` on **45 of 72 compared indicators** (of 91 indicators benchmarked via the FFI).
    Median speedup vs C_tulip: **1.73x**.

    **Largest wins vs C_tulip:**

    | Indicator | Speedup vs C | Speedup vs TA-Lib |
    |-----------|-------------:|------------------:|
    | `willr` | **5.69x** | 2.86x |
    | `msw` | **5.14x** | --- |
    | `aroon` | **4.68x** | 2.25x |
    | `stoch` | **4.22x** | 2.46x |
    | `min` | **4.18x** | 2.23x |
    | `aroonosc` | **4.10x** | 1.80x |
    | `donchianchannel` | **3.74x** | --- |
    | `vhf` | **3.71x** | --- |
    | `max` | **3.39x** | 1.78x |
    | `cvi` | **2.90x** | --- |

    Smallest margins (C_tulip's vectorised or state-light paths):

    | Indicator | Rust native (ns) | tulip_rs_ffi_c (ns) | C Tulip (ns) | Speedup vs C |
    |----------:|-----------------:|----------------------:|-------------:|-------------:|
    | `tr` | 1,393 | 1,395 | 1,439 | **1.03×** |
    | `md` | 13,911 | 15,293 | 15,884 | **1.04×** |
    | `typprice` | 1,086 | 1,185 | 1,247 | **1.05×** |
    | `wcprice` | 1,075 | 1,197 | 1,252 | **1.05×** |
    | `hma` | 9,349 | 8,626 | 9,416 | **1.09×** |
    | `dema` | 6,336 | 5,807 | 6,416 | **1.10×** |

    **FFI boundary overhead** — `tulip_rs_ffi_c` tracks native Rust closely: for the fastest-running indicators the call overhead is 0.9–1.1× Rust native (argument marshalling only, no runtime):

    | Indicator | Rust native (ns) | tulip_rs_ffi_c (ns) | Overhead |
    |-----------|----------------:|----------------------:|---------:|
    | `mom` | 897 | 796 | ~0.9× |
    | `avgprice` | 1,433 | 1,416 | ~1.0× |
    | `tr` | 1,393 | 1,395 | ~1.0× |
    | `medprice` | 872 | 894 | ~1.0× |
    | `typprice` | 1,086 | 1,185 | ~1.1× |
    | `wcprice` | 1,075 | 1,197 | ~1.1× |

---

## Go

Full comparison table (tulip_rs_go vs cinar/indicator/v2): **[Standard Performance →](standard/go.md)**

??? success "Notable results"

    `tulip_rs_go` beats `cinar` on **45 of 45 compared indicators** (cinar is wired where it is param-compatible; the remaining indicators have no pure-Go competitor run).
    Median speedup: **2,283×** — cinar v2's goroutine/channel stream API pays a per-bar scheduling cost the cgo path does not.

    **Largest wins vs cinar:**

    | Indicator | Speedup vs cinar |
    |-----------|-----------------:|
    | `emv` | **7,491×** |
    | `nvi` | **6,601×** |
    | `dpo` | **6,133×** |
    | `typprice` | **5,910×** |
    | `tr` | **5,126×** |
    | `qstick` | **4,833×** |
    | `wcprice` | **4,762×** |
    | `kama` | **3,828×** |
    | `rsi` | **3,714×** |
    | `ppo` | **3,695×** |

    Smallest margins (where cinar's implementation is closest, or computes fewer output rows):

    | Indicator | Rust native (ns) | tulip_rs_go (ns) | cinar (ns) | Speedup vs cinar |
    |----------:|-----------------:|-----------------:|-----------:|-----------------:|
    | `wilders` | 4,777 | 5,413 | 1,919,589 | **355×** |
    | `cci` | 78,669 | 79,522 | 33,112,153 | **416×** |
    | `fisher` | 48,681 | 64,177 | 33,741,271 | **526×** |
    | `ema` | 4,784 | 5,434 | 3,270,234 | **602×** |
    | `aroon` | 18,771 | 17,936 | 13,797,216 | **769×** |
    | `stoch` | 20,383 | 22,865 | 18,320,716 | **801×** |

    **cgo boundary overhead** — the gap between Rust native and `tulip_rs_go` columns reflects the fixed per-call cost of the cgo boundary (argument marshalling, pointer handoff), 1.2–1.6× for the fastest-running indicators:

    | Indicator | Rust native (ns) | tulip_rs_go (ns) | Overhead |
    |-----------|----------------:|-----------------:|---------:|
    | `roc` | 2,437 | 3,022 | ~1.2× |
    | `emv` | 2,449 | 3,134 | ~1.3× |
    | `bop` | 2,440 | 3,137 | ~1.3× |
    | `tr` | 1,393 | 2,096 | ~1.5× |
    | `wcprice` | 1,075 | 1,697 | ~1.6× |
    | `typprice` | 1,086 | 1,721 | ~1.6× |
