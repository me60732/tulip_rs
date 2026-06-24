# Standard Performance

Single asset, averaged across 4 option sets. Ratios show how many times slower the competitor is relative to Rust — higher means Rust wins by more.

=== "vs C"

    Competitors: **Tulip Indicators (C)** and **TA-Lib (C)**

    | Indicator | Rust (ns) | C Tulip (ns) | TA-Lib (ns) | C / Rust | TA-Lib / Rust |
    |-----------|----------:|-------------:|------------:|---------:|--------------:|
    | ad | 4,746 | 5,086 | 5,076 | 1.07 | 1.07 |
    | adosc | 6,668 | 9,418 | 8,851 | 1.41 | 1.33 |
    | adx | 10,746 | 13,494 | 38,473 | 1.26 | 3.58 |
    | adxr | 14,041 | 22,981 | 38,823 | 1.64 | 2.77 |
    | ao | 5,643 | 11,723 | N/A | 2.08 | — |
    | apo | 4,709 | 8,727 | 11,231 | 1.85 | 2.39 |
    | aroon | 18,187 | 38,601 | 74,113 | 2.12 | 4.08 |
    | aroonosc | 21,287 | 37,003 | 70,349 | 1.74 | 3.30 |
    | atr | 4,714 | 10,825 | 28,077 | 2.30 | 5.96 |
    | avgprice | 1,408 | 1,994 | 3,467 | 1.42 | 2.46 |
    | bbands | 7,532 | 13,517 | 23,121 | 1.79 | 3.07 |
    | bop | 2,363 | 2,794 | 5,027 | 1.18 | 2.13 |
    | cci | 57,429 | 74,740 | 122,246 | 1.30 | 2.13 |
    | chaikinmf | 7,073 | N/A | N/A | — | — |
    | chandelierexit | 18,979 | N/A | N/A | — | — |
    | cmo | 6,155 | 22,382 | N/A | 3.64 | — |
    | cvi | 6,098 | 14,333 | N/A | 2.35 | — |
    | cybercycle | 16,376 | N/A | N/A | — | — |
    | dema | 6,157 | 6,483 | 22,935 | 1.05 | 3.73 |
    | di | 14,071 | 10,300 | 56,209 | 0.73 | 3.99 |
    | dm | 9,233 | 6,753 | N/A | 0.73 | — |
    | donchianchannel | 12,954 | 33,080 | N/A | 2.55 | — |
    | dpo | 2,607 | 2,718 | N/A | 1.04 | — |
    | dx | 9,184 | 6,567 | N/A | 0.72 | — |
    | ef | 4,715 | N/A | N/A | — | — |
    | elderray | 6,281 | 15,121 | N/A | 2.41 | — |
    | ema | 4,690 | 10,859 | 10,866 | 2.32 | 2.32 |
    | emv | 2,392 | 5,034 | N/A | 2.10 | — |
    | fisher | 50,358 | 81,210 | N/A | 1.61 | — |
    | fosc | 7,790 | 9,487 | N/A | 1.22 | — |
    | highpass | 4,679 | N/A | N/A | — | — |
    | hilberttransform | 16,848 | N/A | 201,032 | — | 11.93 |
    | hma | 8,312 | 9,266 | N/A | 1.11 | — |
    | homodynediscriminator | 218,285 | N/A | 200,951 | — | 0.92 |
    | ichimoku | 66,476 | N/A | N/A | — | — |
    | instantaneoustrendline | 224,992 | N/A | 234,784 | — | 1.04 |
    | kama | 7,041 | 8,568 | 10,847 | 1.22 | 1.54 |
    | keltnerchannel | 6,513 | N/A | N/A | — | — |
    | kvo | 9,320 | 10,773 | N/A | 1.16 | — |
    | linreg | 6,379 | 8,466 | N/A | 1.33 | — |
    | macd | 6,303 | 10,572 | 36,780 | 1.68 | 5.84 |
    | mama | 224,720 | N/A | 215,222 | — | 0.96 |
    | marketfi | 2,369 | 2,713 | N/A | 1.15 | — |
    | mass | 5,421 | 12,007 | N/A | 2.21 | — |
    | max | 5,175 | 14,218 | 18,110 | 2.75 | 3.50 |
    | md | 12,578 | 14,796 | N/A | 1.18 | — |
    | medprice | 1,073 | 1,676 | 2,279 | 1.56 | 2.12 |
    | mfi | 7,610 | 16,570 | 19,462 | 2.18 | 2.56 |
    | min | 7,717 | 27,341 | 31,539 | 3.54 | 4.09 |
    | mom | 770 | 1,351 | 1,898 | 1.75 | 2.46 |
    | msw | 121,197 | 548,438 | N/A | 4.53 | — |
    | natr | 4,875 | 10,814 | 28,327 | 2.22 | 5.81 |
    | nvi | 2,433 | 3,479 | N/A | 1.43 | — |
    | obv | 3,445 | 3,342 | 3,239 | 0.97 | 0.94 |
    | ppo | 4,963 | 9,457 | 14,398 | 1.91 | 2.90 |
    | psar | 10,378 | 12,247 | 7,898 | 1.18 | 0.76 |
    | pvi | 2,456 | 3,448 | N/A | 1.40 | — |
    | qstick | 2,809 | 3,055 | N/A | 1.09 | — |
    | roc | 2,365 | 2,653 | 4,940 | 1.12 | 2.09 |
    | rocr | 2,370 | 2,704 | 4,970 | 1.14 | 2.10 |
    | roofingfilter | 10,548 | N/A | N/A | — | — |
    | rsi | 4,859 | 9,434 | 25,737 | 1.94 | 5.30 |
    | sma | 2,475 | 2,656 | 4,895 | 1.07 | 1.98 |
    | smaenvelope | 7,287 | N/A | N/A | — | — |
    | stddev | 3,645 | 10,530 | N/A | 2.89 | — |
    | stoch | 20,573 | 48,855 | 51,371 | 2.37 | 2.50 |
    | stochrsi | 19,548 | 43,887 | N/A | 2.25 | — |
    | supersmoother | 10,352 | N/A | N/A | — | — |
    | supertrend | 14,633 | N/A | N/A | — | — |
    | tema | 6,911 | 6,867 | 32,521 | 0.99 | 4.71 |
    | tr | 1,521 | 2,005 | 4,078 | 1.32 | 2.68 |
    | trendmode | 220,885 | N/A | N/A | — | — |
    | trima | 5,493 | 7,209 | 7,183 | 1.31 | 1.31 |
    | trix | 6,634 | 10,664 | N/A | 1.61 | — |
    | trvi | 6,100 | N/A | N/A | — | — |
    | tsf | 6,345 | 8,408 | N/A | 1.33 | — |
    | typprice | 1,036 | 1,802 | N/A | 1.74 | — |
    | ultosc | 16,273 | 17,670 | N/A | 1.09 | — |
    | vhf | 16,235 | 39,222 | N/A | 2.42 | — |
    | vidya | 11,976 | 18,644 | N/A | 1.56 | — |
    | volatility | 8,823 | 17,466 | N/A | 1.98 | — |
    | vortex | 8,142 | N/A | N/A | — | — |
    | vosc | 3,866 | 4,986 | N/A | 1.29 | — |
    | vwap | 3,451 | N/A | N/A | — | — |
    | vwma | 3,384 | 4,957 | N/A | 1.46 | — |
    | wad | 3,796 | 4,989 | N/A | 1.31 | — |
    | wcprice | 1,062 | 1,786 | N/A | 1.68 | — |
    | wilders | 4,669 | 10,660 | N/A | 2.28 | — |
    | willr | 15,760 | 37,673 | 39,831 | 2.39 | 2.53 |
    | wma | 6,251 | 8,506 | 4,968 | 1.36 | 0.79 |
    | zlema | 5,790 | 8,319 | N/A | 1.44 | — |

    ??? success "Notable results"

        Rust beats C Tulip on **all but 4 indicators**: `di` (0.73×), `dm` (0.73×), `dx` (0.72×), `obv` (0.97×). `msw` is now **4.53×** faster than C Tulip (previously 0.85× — Rust was slower before the SDFT implementation).

        | Category | Indicator | C / Rust | TA-Lib / Rust |
        |----------|-----------|:--------:|:-------------:|
        | **Largest wins vs C Tulip** | `msw` | **4.53×** | — |
        | | `cmo` | **3.64×** | — |
        | | `min` | **3.54×** | 4.09× |
        | | `stddev` | **2.89×** | — |
        | | `max` | **2.75×** | 3.50× |
        | **Largest wins vs TA-Lib** | `hilberttransform` | — | **11.93×** |
        | | `atr` | 2.30× | **5.96×** |
        | | `macd` | 1.68× | **5.84×** |
        | | `natr` | 2.22× | **5.81×** |
        | | `rsi` | 1.94× | **5.30×** |
        | **Rust slower than C Tulip** | `dx` | **0.72×** | — |
        | | `di` | **0.73×** | 3.99× |
        | | `dm` | **0.73×** | — |
        | | `obv` | **0.97×** | 0.94× |
        | **Rust slower than TA-Lib** | `psar` | 1.18× | **0.76×** |
        | | `wma` | 1.36× | **0.79×** |
        | | `homodynediscriminator` | — | **0.92×** |
        | | `obv` | 0.97× | **0.94×** |
        | | `mama` | — | **0.96×** |

=== "vs Rust"

    Competitor: **RustTa** — 20 indicators benchmarked.

    | Indicator | Rust (ns) | RustTa (ns) | RustTa / Rust |
    |-----------|----------:|------------:|--------------:|
    | atr | 4,714 | 9,591 | 2.03 |
    | bbands | 7,532 | 8,312 | 1.10 |
    | cci | 57,429 | 67,261 | 1.17 |
    | chandelierexit | 18,979 | 43,002 | 2.27 |
    | ef | 4,715 | 23,624 | 5.01 |
    | ema | 4,690 | 8,135 | 1.73 |
    | keltnerchannel | 6,513 | 13,026 | 2.00 |
    | macd | 6,303 | 8,131 | 1.29 |
    | max | 5,175 | 16,567 | 3.20 |
    | md | 12,578 | 27,524 | 2.19 |
    | mfi | 7,610 | 24,943 | 3.28 |
    | min | 7,717 | 23,701 | 3.07 |
    | obv | 3,445 | 5,411 | 1.57 |
    | ppo | 4,963 | 8,154 | 1.64 |
    | roc | 2,365 | 2,959 | 1.25 |
    | rsi | 4,859 | 8,158 | 1.68 |
    | sma | 2,475 | 4,609 | 1.86 |
    | stddev | 3,645 | 8,459 | 2.32 |
    | stoch | 20,573 | 47,860 | 2.33 |
    | tr | 1,521 | 7,708 | 5.07 |

    ??? success "Notable results"

        Rust beats RustTa on **all 20 compared indicators**.

        | Category | Indicator | RustTa / Rust |
        |----------|-----------|:-------------:|
        | **Largest wins** | `tr` | **5.07×** |
        | | `ef` | **5.01×** |
        | | `mfi` | **3.28×** |
        | | `max` | **3.20×** |
        | | `min` | **3.07×** |
        | **Closest** | `bbands` | 1.10× |
        | | `cci` | 1.17× |
        | | `roc` | 1.25× |

=== "vs Kand"

    Competitor: **kand** (v0.2) — pure Rust, TA-Lib–inspired. Default features: 64-bit precision + basic `check` validation. Same validation level as tulip_rs.

    Note: kand uses NaN-padded full-length outputs (processing all n bars), while tulip_rs outputs only the valid computed bars. Both perform comparable per-call validation.

    | Indicator | Rust (ns) | Kand (ns) | Kand / Rust |
    |-----------|----------:|----------:|------------:|
    | ad | 4,746 | 5,093 | 1.07 |
    | adosc | 6,668 | 24,503 | 3.67 |
    | adx | 10,746 | 73,406 | 6.83 |
    | adxr | 14,041 | 75,707 | 5.39 |
    | aroon | 18,187 | 369,326 | 20.31 |
    | aroonosc | 21,287 | 370,515 | 17.41 |
    | atr | 4,714 | 22,382 | 4.75 |
    | bbands | 7,532 | 22,266 | 2.96 |
    | bop | 2,363 | 2,779 | 1.18 |
    | cci | 57,429 | 85,887 | 1.50 |
    | dema | 6,157 | 23,001 | 3.74 |
    | di | 14,071 | 24,367 | 1.73 |
    | dm | 9,233 | 22,344 | 2.42 |
    | dx | 9,184 | 51,084 | 5.56 |
    | ema | 4,690 | 8,423 | 1.80 |
    | macd | 6,303 | 28,800 | 4.57 |
    | medprice | 1,073 | 1,619 | 1.51 |
    | mfi | 7,610 | 327,569 | 43.04 |
    | mom | 770 | 1,388 | 1.80 |
    | natr | 4,875 | 25,498 | 5.23 |
    | obv | 3,445 | 3,679 | 1.07 |
    | roc | 2,365 | 2,701 | 1.14 |
    | rocr | 2,370 | 2,697 | 1.14 |
    | rsi | 4,859 | 23,546 | 4.85 |
    | sma | 2,475 | 4,894 | 1.98 |
    | stoch | 20,573 | 699,807 | 34.02 |
    | supertrend | 14,633 | 47,267 | 3.23 |
    | tema | 6,911 | 27,135 | 3.93 |
    | tr | 1,521 | 2,191 | 1.44 |
    | trima | 5,493 | 10,111 | 1.84 |
    | trix | 6,634 | 28,462 | 4.29 |
    | typprice | 1,036 | 2,667 | 2.57 |
    | vwap | 3,451 | 9,608 | 2.78 |
    | wcprice | 1,062 | 1,649 | 1.55 |
    | willr | 15,760 | 355,999 | 22.59 |
    | wma | 6,251 | 42,627 | 6.82 |

    ??? success "Notable results"

        tulip_rs beats kand on **all 36 compared indicators**.

        **Sliding-window indicators** show the largest gaps — kand uses O(n×period) scanning while tulip_rs uses O(n) amortised algorithms:

        | Indicator | Kand / Rust | Notes |
        |-----------|:-----------:|-------|
        | `mfi` | **43×** | |
        | `stoch` | **34×** | Ratio scales with period (k=28: 16×, k=100: 70×) |
        | `willr` | **23×** | |
        | `aroon` | **20×** | |
        | `aroonosc` | **17×** | |

        **Recursive-formula indicators** (EMA, SMA, MOM etc.) show smaller but real gaps (~1.8–2×), likely due to kand's per-bar NaN-branch for warmup output and missing `mul_add` FMA.

        **Closest results**: `ad` (1.07×), `obv` (1.07×), `bop` (1.18×), `roc` (1.14×)

=== "Python Binding"

    Competitor: **ta** (bukosabino/ta, pandas-based), called via the **`tulip_rs_python`** PyO3 binding.
    Rust native times are shown for reference — they reflect the underlying computation cost before PyO3 overhead is added.
    See [Python Binding](python.md) for setup and how to run.

    | Indicator | Rust native (ns) | tulip_rs_python (ns) | ta (ns) | ta / Python |
    |-----------|----------------:|---------------------:|--------:|------------:|
    | `ad` | 4,746 | 10,957 | 142,420 | 13× |
    | `adx` | 10,746 | 70,807 | 20,992,998 | 296× |
    | `ao` | 5,643 | 6,179 | 163,708 | 26× |
    | `aroon` | 18,187 | 21,181 | 9,554,491 | 451× |
    | `atr` | 4,714 | 10,854 | 11,182,703 | 1,030× |
    | `bbands` | 7,532 | 42,489 | 209,385 | 5× |
    | `cci` | 57,429 | 57,754 | 24,544,724 | 425× |
    | `chaikinmf` | 7,073 | 10,035 | 250,266 | 25× |
    | `dema` | 6,157 | 31,458 | 120,997 | 4× |
    | `donchianchannel` | 12,954 | 16,094 | 246,303 | 15× |
    | `dpo` | 2,607 | 2,697 | 111,249 | 41× |
    | `ema` | 4,690 | 10,476 | 51,133 | 5× |
    | `emv` | 2,392 | 3,005 | 145,097 | 48× |
    | `hma` | 8,312 | 9,690 | 9,515,258 | 982× |
    | `kama` | 7,041 | 25,185 | 4,189,509 | 166× |
    | `keltnerchannel` | 6,513 | 22,281 | 368,636 | 17× |
    | `macd` | 6,303 | 32,308 | 176,798 | 5× |
    | `mass` | 5,421 | 23,336 | 195,055 | 8× |
    | `mfi` | 7,610 | 17,889 | 29,322,188 | 1,639× |
    | `mom` | 770 | 1,156 | 13,631 | 12× |
    | `nvi` | 2,433 | 3,028 | 62,984,210 | 20,802× |
    | `obv` | 3,445 | 4,475 | 111,026 | 25× |
    | `ppo` | 4,963 | 22,245 | 213,941 | 10× |
    | `psar` | 10,378 | 25,239 | 172,831,901 | 6,848× |
    | `roc` | 2,365 | 2,642 | 84,236 | 32× |
    | `rsi` | 4,859 | 23,091 | 425,801 | 18× |
    | `sma` | 2,475 | 2,718 | 60,603 | 22× |
    | `stoch` | 20,573 | 21,957 | 261,123 | 12× |
    | `stochrsi` | 19,548 | 48,370 | 814,170 | 17× |
    | `tema` | 6,911 | 63,917 | 195,147 | 3× |
    | `trix` | 6,634 | 62,343 | 280,398 | 4× |
    | `ultosc` | 16,273 | 16,923 | 1,513,116 | 89× |
    | `vortex` | 8,142 | 9,232 | 829,711 | 90× |
    | `willr` | 15,760 | 18,076 | 256,528 | 14× |
    | `wma` | 6,251 | 4,928 | 3,094,723 | 628× |

    ??? success "Notable results"

        `tulip_rs_python` beats `ta` on **all 35 compared indicators**. Median speedup: **~22×**.

        The size of the win depends entirely on how `ta` implements each indicator:

        **`ta` falls back to pure-Python loops** — `tulip_rs_python` wins by the largest margin:

        | Indicator | ta / Python | ta implementation |
        |-----------|:-----------:|-------------------|
        | `nvi` | **20,802×** | Pure-Python loop |
        | `psar` | **6,848×** | Pure-Python loop |
        | `mfi` | **1,639×** | Pure-Python loop |
        | `atr` | **1,030×** | Pure-Python loop |
        | `hma` | **982×** | Pure-Python loop |
        | `wma` | **628×** | Pure-Python loop |
        | `aroon` | **451×** | Pure-Python loop |
        | `cci` | **425×** | Pure-Python loop |
        | `adx` | **296×** | Pure-Python loop |
        | `kama` | **166×** | Pure-Python loop |

        **`ta` uses pandas/numpy C paths** — the gap narrows because both sides use compiled code; PyO3 call overhead (~5–25 µs) is visible here:

        | Indicator | Rust native (ns) | tulip_rs_python (ns) | ta (ns) | ta / Python |
        |-----------|----------------:|---------------------:|--------:|:-----------:|
        | `tema` | 6,911 | 63,917 | 195,147 | **3×** |
        | `dema` | 6,157 | 31,458 | 120,997 | **4×** |
        | `trix` | 6,634 | 62,343 | 280,398 | **4×** |
        | `ema` | 4,690 | 10,476 | 51,133 | **5×** |
        | `bbands` | 7,532 | 42,489 | 209,385 | **5×** |
        | `macd` | 6,303 | 32,308 | 176,798 | **5×** |

=== "Node Binding"

    Competitors: **technicalindicators** (anandanand84) and **indicatorts** (Onur Cinar), called via the **`tulip_rs_node`** napi-rs binding.
    Rust native times are shown for reference — they reflect the underlying computation cost before nAPI overhead is added.
    See [Node Binding](node.md) for setup and how to run.

    Only indicators where at least one reference library ran are shown; the remaining 40 indicators have `tulip_rs_node` timings in the database but no JS competitor to compare against.

    | Indicator | Rust native (ns) | tulip_rs_node (ns) | technicalindicators (ns) | indicatorts (ns) | TI / Node | indicatorts / Node |
    |-----------|----------------:|------------------:|------------------------:|-----------------:|----------:|------------------:|
    | `ad` | 4,779 | 15,289 | 362,618 | 56,345 | 23.72 | 3.69 |
    | `adx` | 9,999 | 72,195 | 1,208,832 | — | 16.74 | — |
    | `ao` | 5,393 | 9,476 | 1,179,923 | 51,252 | 124.51 | 5.41 |
    | `apo` | 4,595 | 24,242 | — | 47,272 | — | 1.95 |
    | `aroon` | 17,988 | 29,235 | — | 755,916 | — | 25.86 |
    | `atr` | 4,559 | 12,759 | 487,010 | 412,299 | 38.17 | 32.32 |
    | `bbands` | 7,114 | 48,775 | 5,712,514 | 287,762 | 117.12 | 5.90 |
    | `bop` | 2,312 | 6,102 | — | 20,898 | — | 3.43 |
    | `cci` | 55,726 | 61,800 | 4,545,921 | 143,000 | 73.56 | 2.31 |
    | `chaikinmf` | 6,833 | 12,364 | — | 69,273 | — | 5.60 |
    | `chandelierexit` | 17,252 | 65,897 | — | 1,198,695 | — | 18.19 |
    | `dema` | 5,993 | 33,911 | — | 57,471 | — | 1.69 |
    | `donchianchannel` | 15,182 | 21,793 | — | 822,234 | — | 37.73 |
    | `ema` | 4,547 | 12,942 | 144,942 | 19,981 | 11.20 | 1.54 |
    | `emv` | 2,295 | 6,368 | — | 72,371 | — | 11.37 |
    | `keltnerchannel` | 6,182 | 27,767 | — | 514,102 | — | 18.52 |
    | `macd` | 7,045 | 37,723 | 601,426 | 66,440 | 15.94 | 1.76 |
    | `max` | 5,360 | 9,523 | — | 541,056 | — | 56.82 |
    | `mfi` | 7,513 | 18,663 | 1,727,668 | 440,229 | 92.57 | 23.59 |
    | `min` | 7,808 | 15,168 | — | 489,133 | — | 32.25 |
    | `nvi` | 2,395 | 6,126 | — | 29,651 | — | 4.84 |
    | `obv` | 3,325 | 6,646 | 313,177 | 20,825 | 47.12 | 3.13 |
    | `ppo` | 4,823 | 24,468 | — | 91,518 | — | 3.74 |
    | `psar` | 10,055 | 27,320 | 290,166 | 48,245 | 10.62 | 1.77 |
    | `qstick` | 2,731 | 6,459 | — | 19,149 | — | 2.96 |
    | `roc` | 2,294 | 6,158 | 568,223 | 17,056 | 92.27 | 2.77 |
    | `rsi` | 4,719 | 25,181 | 791,156 | 112,723 | 31.42 | 4.48 |
    | `sma` | 2,451 | 6,456 | 534,399 | 17,495 | 82.77 | 2.71 |
    | `stoch` | 20,391 | 28,822 | 1,329,714 | 580,434 | 46.14 | 20.14 |
    | `stochrsi` | 19,106 | 51,162 | 3,466,890 | — | 67.76 | — |
    | `tema` | 6,734 | 63,641 | — | 95,084 | — | 1.49 |
    | `tr` | 1,545 | 6,313 | — | 347,741 | — | 55.09 |
    | `trima` | 5,408 | 7,520 | — | 26,088 | — | 3.47 |
    | `trix` | 6,457 | 65,811 | 636,172 | 83,927 | 9.67 | 1.28 |
    | `typprice` | 1,128 | 6,234 | — | 20,974 | — | 3.36 |
    | `vortex` | 7,504 | 14,169 | — | 534,445 | — | 37.72 |
    | `vwma` | 3,310 | 7,549 | — | 39,233 | — | 5.20 |
    | `wilders` | 4,536 | 13,540 | 157,851 | 42,339 | 11.66 | 3.13 |
    | `willr` | 17,218 | 21,420 | 1,400,240 | 829,212 | 65.37 | 38.71 |
    | `wma` | 6,155 | 8,296 | 3,344,025 | — | 403.07 | — |
    | `candlestick` | 515,546 | 2,914,485 | 107,339,622 | — | 36.83 | — |

    ??? success "Notable results"

        `tulip_rs_node` beats every JS competitor on **all compared indicators**.
        Median speedup: **~46× vs technicalindicators**, **~4.5× vs indicatorts**.

        **Largest wins vs technicalindicators** (pure JS, loop-heavy implementations):

        | Indicator | TI / Node | Notes |
        |-----------|----------:|-------|
        | `wma` | **403×** | technicalindicators iterates per-bar in JS |
        | `ao` | **125×** | |
        | `bbands` | **117×** | |
        | `roc` | **92×** | |
        | `mfi` | **93×** | |
        | `sma` | **83×** | |
        | `cci` | **74×** | |
        | `stochrsi` | **68×** | |
        | `willr` | **65×** | |
        | `obv` | **47×** | |

        **Largest wins vs indicatorts** (pure TypeScript):

        | Indicator | indicatorts / Node |
        |-----------|------------------:|
        | `max` | **57×** |
        | `tr` | **55×** |
        | `willr` | **39×** |
        | `donchianchannel` | **38×** |
        | `vortex` | **38×** |
        | `min` | **32×** |
        | `atr` | **32×** |
        | `aroon` | **26×** |
        | `mfi` | **24×** |
        | `stoch` | **20×** |

        **nAPI boundary overhead** — the gap between Rust native and `tulip_rs_node` columns reflects the fixed per-call cost of the nAPI boundary (argument marshalling, `Float64Array` handoff), roughly **4–25 µs** depending on the number of input/output series:

        | Indicator | Rust native (ns) | tulip_rs_node (ns) | Overhead |
        |-----------|----------------:|------------------:|---------:|
        | `obv` | 3,325 | 6,646 | ~2× |
        | `sma` | 2,451 | 6,456 | ~2.6× |
        | `ema` | 4,547 | 12,942 | ~2.8× |
        | `rsi` | 4,719 | 25,181 | ~5.3× |
        | `bbands` | 7,114 | 48,775 | ~6.9× |
        | `trix` | 6,457 | 65,811 | ~10× |

        **Candlestick scanner** — scans 81 patterns in a single pass:

        | | Time |
        |--|-----:|
        | Rust native | 515,546 ns |
        | `tulip_rs_node` | 2,914,485 ns |
        | `technicalindicators` | 107,339,622 ns |
        | TI / Node ratio | **36.83×** |
