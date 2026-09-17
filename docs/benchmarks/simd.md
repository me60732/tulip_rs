# SIMD Performance

`tulip_rs` uses 256-bit AVX2 `f64x4` SIMD (N = 4 lanes) in two modes:

- **[by_assets](simd/by-assets.md)** - process 4 different assets with the same options in a single pass
- **[by_options](simd/by-options.md)** - process 1 asset with 4 different option configurations in a single pass

All times are nanoseconds (ns). Lower is better.

---

## by_assets: 4 Assets Simultaneously

Full comparison tables (vs Rust, vs C / TA-Lib, Python Binding, Node Binding, C Binding, Go Binding, and each binding's reference libraries): **[SIMD by_assets →](simd/by-assets.md)**

??? success "Notable results - by_assets"

    === "Rust"

        **66 of 93 indicators (71%) show a SIMD speedup over 4x sequential calls.**
        Median speedup for benefiting indicators: **~1.83x**.

        | Category | Indicator | SIMD Speedup |
        |----------|-----------|:------------:|
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
        | **SIMD slower than sequential** | `donchianchannel` | 0.45x |
        | **SIMD slower than sequential** | `psar` | 0.47x |
        | **SIMD slower than sequential** | `stochrsi` | 0.57x |
        | **SIMD slower than sequential** | `chandelierexit` | 0.66x |
        | **SIMD slower than sequential** | `ichimoku` | 0.69x |

        Indicators where SIMD is slower typically involve highly sequential computation or irregular memory access patterns where SIMD setup overhead dominates the batched competitor's own per-call cost.

    === "Python"

        **75 of 93 indicators (81%) show a SIMD speedup over 4x sequential calls.**
        Median speedup for benefiting indicators: **~1.89x**.

        | Category | Indicator | SIMD Speedup |
        |----------|-----------|:------------:|
        | **Top performers** | `cci` | **4.99x** |
        | **Top performers** | `tsf` | **4.70x** |
        | **Top performers** | `di` | **3.98x** |
        | **Top performers** | `dx` | **3.57x** |
        | **Top performers** | `trendmode` | **3.47x** |
        | **Top performers** | `mama` | **3.44x** |
        | **Top performers** | `linreg` | **3.30x** |
        | **Top performers** | `supertrend` | **3.27x** |
        | **Top performers** | `kama` | **3.18x** |
        | **Top performers** | `md` | **3.16x** |
        | **SIMD slower than sequential** | `sma` | 0.53x |
        | **SIMD slower than sequential** | `marketfi` | 0.63x |
        | **SIMD slower than sequential** | `vhf` | 0.64x |
        | **SIMD slower than sequential** | `donchianchannel` | 0.65x |
        | **SIMD slower than sequential** | `obv` | 0.65x |

        Indicators where SIMD is slower typically involve highly sequential computation or irregular memory access patterns where SIMD setup overhead dominates the batched competitor's own per-call cost.

    === "Node"

        **59 of 93 indicators (63%) show a SIMD speedup over 4x sequential calls.**
        Median speedup for benefiting indicators: **~1.29x**.

        | Category | Indicator | SIMD Speedup |
        |----------|-----------|:------------:|
        | **Top performers** | `mama` | **3.31x** |
        | **Top performers** | `trendmode` | **3.29x** |
        | **Top performers** | `cci` | **3.19x** |
        | **Top performers** | `homodynediscriminator` | **2.80x** |
        | **Top performers** | `instantaneoustrendline` | **2.39x** |
        | **Top performers** | `msw` | **2.36x** |
        | **Top performers** | `ccfisher` | **2.27x** |
        | **Top performers** | `supersmoother` | **2.11x** |
        | **Top performers** | `adxr` | **1.80x** |
        | **Top performers** | `ultosc` | **1.67x** |
        | **SIMD slower than sequential** | `vhf` | 0.40x |
        | **SIMD slower than sequential** | `ad` | 0.50x |
        | **SIMD slower than sequential** | `stochrsi` | 0.62x |
        | **SIMD slower than sequential** | `donchianchannel` | 0.63x |
        | **SIMD slower than sequential** | `psar` | 0.63x |

        Indicators where SIMD is slower typically involve highly sequential computation or irregular memory access patterns where SIMD setup overhead dominates the batched competitor's own per-call cost.

    === "C"

        **64 of 91 indicators (70%) show a SIMD speedup over 4x sequential calls.**
        Median speedup for benefiting indicators: **~2.05x**.

        | Category | Indicator | SIMD Speedup |
        |----------|-----------|:------------:|
        | **Top performers** | `ao` | **6.79x** |
        | **Top performers** | `trendmode` | **5.30x** |
        | **Top performers** | `adaptivemsw` | **4.86x** |
        | **Top performers** | `ccfisher` | **4.77x** |
        | **Top performers** | `ad` | **4.77x** |
        | **Top performers** | `cybercycle` | **4.76x** |
        | **Top performers** | `vwap` | **4.76x** |
        | **Top performers** | `cci` | **4.38x** |
        | **Top performers** | `wad` | **4.29x** |
        | **Top performers** | `wcprice` | **4.01x** |
        | **SIMD slower than sequential** | `donchianchannel` | 0.31x |
        | **SIMD slower than sequential** | `macd` | 0.34x |
        | **SIMD slower than sequential** | `keltnerchannel` | 0.35x |
        | **SIMD slower than sequential** | `ichimoku` | 0.55x |
        | **SIMD slower than sequential** | `stochrsi` | 0.57x |

        Indicators where SIMD is slower typically involve highly sequential computation or irregular memory access patterns where SIMD setup overhead dominates the batched competitor's own per-call cost.

    === "Go"

        **62 of 94 indicators (66%) show a SIMD speedup over 4x sequential calls.**
        Median speedup for benefiting indicators: **~1.60x**.

        | Category | Indicator | SIMD Speedup |
        |----------|-----------|:------------:|
        | **Top performers** | `cci` | **4.05x** |
        | **Top performers** | `trendmode` | **3.51x** |
        | **Top performers** | `mama` | **3.07x** |
        | **Top performers** | `roofingfilter` | **3.01x** |
        | **Top performers** | `ccfisher` | **3.00x** |
        | **Top performers** | `supersmoother` | **2.90x** |
        | **Top performers** | `homodynediscriminator` | **2.88x** |
        | **Top performers** | `instantaneoustrendline` | **2.42x** |
        | **Top performers** | `msw` | **2.24x** |
        | **Top performers** | `tema` | **2.12x** |
        | **SIMD slower than sequential** | `donchianchannel` | 0.35x |
        | **SIMD slower than sequential** | `macd` | 0.38x |
        | **SIMD slower than sequential** | `keltnerchannel` | 0.39x |
        | **SIMD slower than sequential** | `smaenvelope` | 0.41x |
        | **SIMD slower than sequential** | `bbands` | 0.46x |

        Indicators where SIMD is slower typically involve highly sequential computation or irregular memory access patterns where SIMD setup overhead dominates the batched competitor's own per-call cost.

---

## by_options: 4 Option Sets Simultaneously

Full comparison tables (vs Rust, vs Sequential Rust / Binding scalar, C / TA-Lib, Python Binding, Node Binding, C Binding, Go Binding): **[SIMD by_options →](simd/by-options.md)**

??? success "Notable results - by_options"

    === "Rust"

        **52 of 75 indicators (69%) show a SIMD speedup over 4x sequential calls.**
        Median speedup for benefiting indicators: **~2.02x**.

        | Category | Indicator | SIMD Speedup |
        |----------|-----------|:------------:|
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

    === "Python"

        **67 of 75 indicators (89%) show a SIMD speedup over 4x sequential calls.**
        Median speedup for benefiting indicators: **~2.00x**.

        | Category | Indicator | SIMD Speedup |
        |----------|-----------|:------------:|
        | **Top performers** | `natr` | **5.29x** |
        | **Top performers** | `dx` | **5.28x** |
        | **Top performers** | `di` | **5.24x** |
        | **Top performers** | `trix` | **5.14x** |
        | **Top performers** | `kama` | **4.73x** |
        | **Top performers** | `supertrend` | **4.60x** |
        | **Top performers** | `linreg` | **4.22x** |
        | **Top performers** | `trendmode` | **3.87x** |
        | **Top performers** | `kvo` | **3.86x** |
        | **Notable improvement** | `msw` | **3.61x** (SDFT optimisation) |
        | **SIMD slower than sequential** | `vhf` | 0.63x |
        | **SIMD slower than sequential** | `donchianchannel` | 0.65x |
        | **SIMD slower than sequential** | `dpo` | 0.66x |
        | **SIMD slower than sequential** | `trvi` | 0.79x |

        Indicators that don't benefit tend to involve complex branching or irregular memory access patterns that prevent effective vectorisation.

    === "Node"

        **54 of 75 indicators (72%) show a SIMD speedup over 4x sequential calls.**
        Median speedup for benefiting indicators: **~1.37x**.

        | Category | Indicator | SIMD Speedup |
        |----------|-----------|:------------:|
        | **Top performers** | `trendmode` | **3.60x** |
        | **Top performers** | `msw` | **3.39x** |
        | **Top performers** | `mama` | **3.34x** |
        | **Top performers** | `ccfisher` | **3.14x** |
        | **Top performers** | `cybercycle` | **2.27x** |
        | **Top performers** | `cci` | **2.25x** |
        | **Top performers** | `di` | **2.13x** |
        | **Top performers** | `supersmoother` | **1.92x** |
        | **Top performers** | `adxr` | **1.78x** |
        | **Notable improvement** | `msw` | **3.39x** (SDFT optimisation) |
        | **SIMD slower than sequential** | `vhf` | 0.41x |
        | **SIMD slower than sequential** | `donchianchannel` | 0.62x |
        | **SIMD slower than sequential** | `psar` | 0.68x |
        | **SIMD slower than sequential** | `willr` | 0.68x |

        Indicators that don't benefit tend to involve complex branching or irregular memory access patterns that prevent effective vectorisation.

    === "C"

        **51 of 75 indicators (68%) show a SIMD speedup over 4x sequential calls.**
        Median speedup for benefiting indicators: **~2.09x**.

        | Category | Indicator | SIMD Speedup |
        |----------|-----------|:------------:|
        | **Top performers** | `trendmode` | **3.89x** |
        | **Top performers** | `msw` | **3.63x** |
        | **Top performers** | `roofingfilter` | **3.52x** |
        | **Top performers** | `di` | **3.49x** |
        | **Top performers** | `mama` | **3.43x** |
        | **Top performers** | `cybercycle` | **3.29x** |
        | **Top performers** | `ccfisher` | **3.27x** |
        | **Top performers** | `supersmoother` | **3.27x** |
        | **Top performers** | `tema` | **2.91x** |
        | **Notable improvement** | `msw` | **3.63x** (SDFT optimisation) |
        | **SIMD slower than sequential** | `donchianchannel` | 0.31x |
        | **SIMD slower than sequential** | `macd` | 0.35x |
        | **SIMD slower than sequential** | `keltnerchannel` | 0.36x |
        | **SIMD slower than sequential** | `vhf` | 0.51x |

        Indicators that don't benefit tend to involve complex branching or irregular memory access patterns that prevent effective vectorisation.

    === "Go"

        **53 of 76 indicators (70%) show a SIMD speedup over 4x sequential calls.**
        Median speedup for benefiting indicators: **~1.73x**.

        | Category | Indicator | SIMD Speedup |
        |----------|-----------|:------------:|
        | **Top performers** | `trendmode` | **3.87x** |
        | **Top performers** | `msw` | **3.57x** |
        | **Top performers** | `roofingfilter` | **3.11x** |
        | **Top performers** | `mama` | **3.08x** |
        | **Top performers** | `ccfisher` | **2.99x** |
        | **Top performers** | `supersmoother` | **2.85x** |
        | **Top performers** | `cybercycle` | **2.66x** |
        | **Top performers** | `cci` | **2.56x** |
        | **Top performers** | `adx` | **2.46x** |
        | **Notable improvement** | `msw` | **3.57x** (SDFT optimisation) |
        | **SIMD slower than sequential** | `donchianchannel` | 0.34x |
        | **SIMD slower than sequential** | `keltnerchannel` | 0.39x |
        | **SIMD slower than sequential** | `macd` | 0.39x |
        | **SIMD slower than sequential** | `smaenvelope` | 0.43x |

        Indicators that don't benefit tend to involve complex branching or irregular memory access patterns that prevent effective vectorisation.
