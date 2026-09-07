# Standard Performance: Node

Competitors: **technicalindicators** (anandanand84) and **indicatorts** (Onur Cinar), called via the **`tulip_rs_node`** napi-rs binding.

Only indicators where at least one reference library ran are shown; the remaining indicators have `tulip_rs_node` timings in the database but no JS competitor to compare against.

| Indicator | tulip_rs_node (ns) | technicalindicators (ns) | indicatorts (ns) | Speedup vs technicalindicators | Speedup vs indicatorts |
|-----------|------------------:|------------------------:|-----------------:|--------------------------------:|------------------------:|
| `Rust_Candlestick` | 2,427,665 | 102,505,127 | N/A | 42.22x | --- |
| `ad` | 9,984 | 205,547 | 42,784 | 20.59x | 4.29x |
| `adaptivemsw` | 537,794 | N/A | N/A | --- | --- |
| `adosc` | 8,949 | N/A | N/A | --- | --- |
| `adx` | 14,263 | 1,215,328 | N/A | 85.21x | --- |
| `adxr` | 20,018 | N/A | N/A | --- | --- |
| `ao` | 8,688 | 1,081,898 | 29,900 | 124.52x | 3.44x |
| `apo` | 7,063 | N/A | 42,761 | --- | 6.05x |
| `aroon` | 29,211 | N/A | 726,118 | --- | 24.86x |
| `aroonosc` | 25,646 | N/A | N/A | --- | --- |
| `atr` | 8,952 | 398,175 | 255,429 | 44.48x | 28.53x |
| `avgprice` | 5,511 | N/A | N/A | --- | --- |
| `bbands` | 14,379 | 5,917,073 | 294,652 | 411.52x | 20.49x |
| `bop` | 6,125 | N/A | 17,489 | --- | 2.86x |
| `ccfisher` | 235,246 | N/A | N/A | --- | --- |
| `cci` | 84,509 | 4,845,108 | 67,713 | 57.33x | 0.80x |
| `chaikinmf` | 11,835 | N/A | 54,135 | --- | 4.57x |
| `chandelierexit` | 25,641 | N/A | 1,010,519 | --- | 39.41x |
| `cmo` | 8,793 | N/A | N/A | --- | --- |
| `cvi` | 8,286 | N/A | N/A | --- | --- |
| `cybercycle` | 18,953 | N/A | N/A | --- | --- |
| `dema` | 7,934 | N/A | 47,165 | --- | 5.94x |
| `di` | 27,118 | N/A | N/A | --- | --- |
| `dm` | 16,554 | N/A | N/A | --- | --- |
| `donchianchannel` | 21,100 | N/A | 811,376 | --- | 38.45x |
| `dpo` | 5,597 | N/A | N/A | --- | --- |
| `dx` | 11,921 | N/A | N/A | --- | --- |
| `ef` | 6,910 | N/A | N/A | --- | --- |
| `elderray` | 17,084 | N/A | N/A | --- | --- |
| `ema` | 6,948 | 148,524 | 20,371 | 21.38x | 2.93x |
| `emv` | 5,920 | N/A | 56,421 | --- | 9.53x |
| `fisher` | 73,506 | N/A | N/A | --- | --- |
| `fosc` | 38,742 | N/A | N/A | --- | --- |
| `highpass` | 6,714 | N/A | N/A | --- | --- |
| `hilberttransform` | 21,797 | N/A | N/A | --- | --- |
| `hma` | 11,065 | N/A | N/A | --- | --- |
| `homodynediscriminator` | 220,588 | N/A | N/A | --- | --- |
| `ichimoku` | 87,035 | 4,162,843 | N/A | 47.83x | --- |
| `instantaneoustrendline` | 225,232 | N/A | N/A | --- | --- |
| `kama` | 9,462 | N/A | N/A | --- | --- |
| `keltnerchannel` | 17,032 | N/A | 282,962 | --- | 16.61x |
| `kvo` | 11,855 | N/A | N/A | --- | --- |
| `linreg` | 9,435 | N/A | N/A | --- | --- |
| `macd` | 17,948 | 621,042 | 60,969 | 34.60x | 3.40x |
| `mama` | 227,367 | N/A | N/A | --- | --- |
| `marketfi` | 6,766 | N/A | N/A | --- | --- |
| `mass` | 8,845 | N/A | N/A | --- | --- |
| `max` | 11,508 | N/A | 561,391 | --- | 48.78x |
| `md` | 18,427 | N/A | N/A | --- | --- |
| `medprice` | 5,538 | N/A | N/A | --- | --- |
| `mfi` | 12,680 | 1,932,480 | 263,645 | 152.41x | 20.79x |
| `min` | 14,148 | N/A | 503,547 | --- | 35.59x |
| `mom` | 5,173 | N/A | N/A | --- | --- |
| `msw` | 137,983 | N/A | N/A | --- | --- |
| `natr` | 8,569 | N/A | N/A | --- | --- |
| `nvi` | 5,651 | N/A | 26,940 | --- | 4.77x |
| `obv` | 6,801 | 189,363 | 19,665 | 27.84x | 2.89x |
| `pivotpoint` | 2,110 | N/A | N/A | --- | --- |
| `ppo` | 8,008 | N/A | 79,927 | --- | 9.98x |
| `psar` | 12,955 | 225,375 | 45,925 | 17.40x | 3.54x |
| `pvi` | 5,945 | N/A | N/A | --- | --- |
| `qstick` | 6,110 | N/A | 16,838 | --- | 2.76x |
| `roc` | 6,611 | 591,026 | 14,099 | 89.40x | 2.13x |
| `rocr` | 5,191 | N/A | N/A | --- | --- |
| `roofingfilter` | 12,982 | N/A | N/A | --- | --- |
| `rsi` | 7,696 | 919,797 | 108,653 | 119.51x | 14.12x |
| `sma` | 6,208 | 438,395 | 13,458 | 70.61x | 2.17x |
| `smaenvelope` | 15,370 | N/A | N/A | --- | --- |
| `stddev` | 9,223 | N/A | N/A | --- | --- |
| `stoch` | 34,491 | 1,792,464 | 592,336 | 51.97x | 17.17x |
| `stochrsi` | 23,124 | 3,799,361 | N/A | 164.30x | --- |
| `supersmoother` | 12,706 | N/A | N/A | --- | --- |
| `supertrend` | 15,846 | N/A | N/A | --- | --- |
| `tema` | 9,321 | N/A | 77,890 | --- | 8.36x |
| `tr` | 7,093 | N/A | 248,333 | --- | 35.01x |
| `trendmode` | 221,588 | N/A | N/A | --- | --- |
| `trima` | 8,353 | N/A | 22,760 | --- | 2.72x |
| `trix` | 9,541 | 704,720 | 75,803 | 73.87x | 7.95x |
| `trvi` | 8,880 | N/A | N/A | --- | --- |
| `tsf` | 9,131 | N/A | N/A | --- | --- |
| `typprice` | 5,915 | N/A | 17,684 | --- | 2.99x |
| `ultosc` | 17,418 | N/A | N/A | --- | --- |
| `vhf` | 16,579 | N/A | N/A | --- | --- |
| `vidya` | 14,794 | N/A | N/A | --- | --- |
| `volatility` | 13,909 | N/A | N/A | --- | --- |
| `vortex` | 20,929 | N/A | 306,157 | --- | 14.63x |
| `vosc` | 7,345 | N/A | N/A | --- | --- |
| `vwap` | 9,973 | 252,891 | N/A | 25.36x | --- |
| `vwma` | 7,850 | N/A | 28,340 | --- | 3.61x |
| `wad` | 6,972 | N/A | N/A | --- | --- |
| `wcprice` | 5,446 | N/A | N/A | --- | --- |
| `wilders` | 7,562 | 120,444 | 40,009 | 15.93x | 5.29x |
| `willr` | 19,166 | 1,451,408 | 824,585 | 75.73x | 43.02x |
| `wma` | 9,592 | 3,653,176 | N/A | 380.84x | --- |
| `zlema` | 7,890 | N/A | N/A | --- | --- |

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
