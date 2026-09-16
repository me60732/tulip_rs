# SIMD Performance

`tulip_rs` uses 256-bit AVX2 `f64x4` SIMD (N = 4 lanes) in two modes:

- **[by_assets](simd/by-assets.md)** - process 4 different assets with the same options in a single pass
- **[by_options](simd/by-options.md)** - process 1 asset with 4 different option configurations in a single pass

All times are nanoseconds (ns). Lower is better.

---

## by_assets: 4 Assets Simultaneously

Full comparison tables (vs C / TA-Lib, Python Binding, Node Binding, C Binding, Go Binding, and each binding's reference libraries): **[SIMD by_assets →](simd/by-assets.md)**

??? success "Notable results - by_assets"

    **67 of 93 indicators (72%) show a SIMD speedup over 4x sequential Rust.**
    Median speedup for benefiting indicators: **~1.80x**.

    | Category | Indicator | SIMD Speedup vs 4x Sequential Rust |
    |----------|-----------|:-----------------------------------:|
    | **Top performers** | `roofingfilter` | **3.63x** |
    | **Top performers** | `supersmoother` | **3.53x** |
    | **Top performers** | `trendmode` | **3.49x** |
    | **Top performers** | `mama` | **3.43x** |
    | **Top performers** | `ccfisher` | **3.31x** |
    | **Top performers** | `cci` | **3.30x** |
    | **Top performers** | `linreg` | **3.05x** |
    | **Top performers** | `di` | **2.97x** |
    | **Top performers** | `homodynediscriminator` | **2.91x** |
    | **Top performers** | `dema` | **2.69x** |
    | **Notable improvement** | `msw` | 1.73x (SDFT optimisation) |
    | **SIMD slower than sequential** | `donchianchannel` | 0.45x |
    | **SIMD slower than sequential** | `psar` | 0.47x |
    | **SIMD slower than sequential** | `stochrsi` | 0.57x |
    | **SIMD slower than sequential** | `chandelierexit` | 0.66x |
    | **SIMD slower than sequential** | `ichimoku` | 0.69x |

    Indicators where SIMD is slower typically involve highly sequential computation or irregular memory access patterns where SIMD setup overhead dominates.

---

## by_options: 4 Option Sets Simultaneously

Full comparison tables (vs Sequential Rust, Python Binding, Node Binding, C Binding, Go Binding): **[SIMD by_options →](simd/by-options.md)**

??? success "Notable results - by_options"

    **53 of 75 indicators (71%) show a SIMD speedup over 4x sequential Rust.**

    | Category | Indicator | Speedup |
    |----------|-----------|:-------:|
    | **Top performers** | `trendmode` | **3.85x** |
    | **Top performers** | `msw` | **3.54x** |
    | **Top performers** | `roofingfilter` | **3.52x** |
    | **Top performers** | `mama` | **3.50x** |
    | **Top performers** | `di` | **3.48x** |
    | **Top performers** | `ccfisher` | **3.26x** |
    | **Top performers** | `cybercycle` | **3.15x** |
    | **Top performers** | `supersmoother` | **3.12x** |
    | **Top performers** | `tema` | **2.86x** |
    | **Notable improvement** | `msw` | **3.54x** (SDFT optimisation) |
    | **SIMD slower than sequential** | `donchianchannel` | 0.44x |
    | **SIMD slower than sequential** | `psar` | 0.56x |
    | **SIMD slower than sequential** | `vhf` | 0.56x |
    | **SIMD slower than sequential** | `min` | 0.59x |

    Indicators that don't benefit tend to involve complex branching or irregular memory access patterns that prevent effective vectorisation.
