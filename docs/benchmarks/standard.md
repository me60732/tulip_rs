# Standard Performance

Single asset, averaged across 4 option sets. Ratios show how many times slower the competitor is relative to Rust — higher means Rust wins by more.

=== "vs C"

    Competitors: **Tulip Indicators (C)** and **TA-Lib (C)**

    | Indicator | Rust (ns) | C Tulip (ns) | TA-Lib (ns) | C / Rust | TA-Lib / Rust |
    |-----------|----------:|-------------:|------------:|---------:|--------------:|
    | ad | 4,779 | 4,975 | 4,994 | 1.04 | 1.04 |
    | adosc | 6,556 | 9,076 | 8,556 | 1.38 | 1.31 |
    | adx | 9,999 | 13,249 | 36,735 | 1.32 | 3.67 |
    | adxr | 11,039 | 22,607 | 37,836 | 2.05 | 3.43 |
    | ao | 5,393 | 11,548 | N/A | 2.14 | — |
    | apo | 4,595 | 8,573 | 11,053 | 1.87 | 2.41 |
    | aroon | 17,988 | 37,121 | 72,090 | 2.06 | 4.01 |
    | aroonosc | 19,873 | 35,588 | 68,045 | 1.79 | 3.42 |
    | atr | 4,559 | 10,439 | 27,396 | 2.29 | 6.01 |
    | avgprice | 1,362 | 1,951 | 3,479 | 1.43 | 2.55 |
    | bbands | 7,114 | 12,000 | 21,282 | 1.69 | 2.99 |
    | bop | 2,312 | 2,708 | 4,854 | 1.17 | 2.10 |
    | cci | 55,726 | 72,586 | 118,567 | 1.30 | 2.13 |
    | chaikinmf | 6,833 | N/A | N/A | — | — |
    | chandelierexit | 17,252 | N/A | N/A | — | — |
    | cmo | 5,959 | 21,534 | N/A | 3.61 | — |
    | cvi | 5,926 | 13,919 | N/A | 2.35 | — |
    | dema | 5,993 | 6,326 | 22,264 | 1.06 | 3.71 |
    | di | 9,713 | 10,067 | 54,622 | 1.04 | 5.62 |
    | dm | 6,611 | 6,840 | N/A | 1.03 | — |
    | donchianchannel | 15,182 | 31,697 | N/A | 2.09 | — |
    | dpo | 2,528 | 2,652 | N/A | 1.05 | — |
    | dx | 8,441 | 6,367 | N/A | 0.75 | — |
    | ef | 4,574 | N/A | N/A | — | — |
    | elderray | 6,197 | 14,688 | N/A | 2.37 | — |
    | ema | 4,547 | 10,501 | 10,477 | 2.31 | 2.30 |
    | emv | 2,295 | 4,843 | N/A | 2.11 | — |
    | fisher | 48,523 | 77,634 | N/A | 1.60 | — |
    | fosc | 7,513 | 9,175 | N/A | 1.22 | — |
    | hma | 8,114 | 8,955 | N/A | 1.10 | — |
    | kama | 6,892 | 8,320 | 10,529 | 1.21 | 1.53 |
    | keltnerchannel | 6,182 | N/A | N/A | — | — |
    | kvo | 9,068 | 11,273 | N/A | 1.24 | — |
    | linreg | 6,228 | 8,219 | N/A | 1.32 | — |
    | macd | 7,045 | 10,707 | 35,862 | 1.52 | 5.09 |
    | marketfi | 2,309 | 2,662 | N/A | 1.15 | — |
    | mass | 5,568 | 11,698 | N/A | 2.10 | — |
    | max | 5,360 | 13,735 | 17,476 | 2.56 | 3.26 |
    | md | 12,120 | 14,292 | N/A | 1.18 | — |
    | medprice | 1,009 | 1,593 | 2,217 | 1.58 | 2.20 |
    | mfi | 7,513 | 17,149 | 18,809 | 2.28 | 2.50 |
    | min | 7,808 | 26,312 | 30,331 | 3.37 | 3.88 |
    | mom | 838 | 1,374 | 1,813 | 1.64 | 2.16 |
    | msw | 625,817 | 530,147 | N/A | 0.85 | — |
    | natr | 4,712 | 10,511 | 27,336 | 2.23 | 5.80 |
    | nvi | 2,395 | 3,509 | N/A | 1.47 | — |
    | obv | 3,325 | 3,330 | 3,112 | 1.00 | 0.94 |
    | ppo | 4,823 | 9,175 | 13,961 | 1.90 | 2.89 |
    | psar | 10,055 | 11,231 | 7,631 | 1.12 | 0.76 |
    | pvi | 2,400 | 3,329 | N/A | 1.39 | — |
    | qstick | 2,731 | 2,919 | N/A | 1.07 | — |
    | roc | 2,294 | 2,596 | 4,796 | 1.13 | 2.09 |
    | rocr | 2,289 | 2,592 | 4,804 | 1.13 | 2.10 |
    | rsi | 4,719 | 9,144 | 24,939 | 1.94 | 5.28 |
    | sma | 2,451 | 2,572 | 4,775 | 1.05 | 1.95 |
    | smaenvelope | 7,266 | N/A | N/A | — | — |
    | stddev | 3,601 | 10,383 | N/A | 2.88 | — |
    | stoch | 20,391 | 47,916 | 50,051 | 2.35 | 2.45 |
    | stochrsi | 19,106 | 43,358 | N/A | 2.27 | — |
    | tema | 6,734 | 6,775 | 32,007 | 1.01 | 4.75 |
    | tr | 1,545 | 2,000 | 3,738 | 1.29 | 2.42 |
    | trima | 5,408 | 7,095 | 7,083 | 1.31 | 1.31 |
    | trix | 6,457 | 10,519 | N/A | 1.63 | — |
    | trvi | 6,010 | N/A | N/A | — | — |
    | tsf | 6,270 | 8,222 | N/A | 1.31 | — |
    | typprice | 1,128 | 1,805 | N/A | 1.60 | — |
    | ultosc | 16,033 | 17,425 | N/A | 1.09 | — |
    | vhf | 14,842 | 38,604 | N/A | 2.60 | — |
    | vidya | 11,607 | 18,316 | N/A | 1.58 | — |
    | volatility | 8,689 | 17,238 | N/A | 1.98 | — |
    | vortex | 7,504 | N/A | N/A | — | — |
    | vosc | 3,791 | 4,850 | N/A | 1.28 | — |
    | vwma | 3,310 | 4,843 | N/A | 1.46 | — |
    | wad | 3,751 | 4,847 | N/A | 1.29 | — |
    | wcprice | 1,058 | 1,846 | N/A | 1.74 | — |
    | wilders | 4,536 | 10,448 | N/A | 2.30 | — |
    | willr | 17,218 | 37,209 | 39,136 | 2.16 | 2.27 |
    | wma | 6,155 | 8,299 | 4,827 | 1.35 | 0.78 |
    | zlema | 5,716 | 8,255 | N/A | 1.44 | — |

    ??? success "Notable results"

        Rust beats C Tulip on **70 of 72 comparable indicators**. Median C / Rust ratio: **~1.47×**.

        | Category | Indicator | C / Rust | TA-Lib / Rust |
        |----------|-----------|:--------:|:-------------:|
        | **Largest wins vs C Tulip** | `cmo` | **3.61×** | — |
        | | `min` | **3.37×** | 3.88× |
        | | `stddev` | **2.88×** | — |
        | | `vhf` | **2.60×** | — |
        | | `max` | **2.56×** | 3.26× |
        | **Largest wins vs TA-Lib** | `atr` | 2.29× | **6.01×** |
        | | `natr` | 2.23× | **5.80×** |
        | | `di` | 1.04× | **5.62×** |
        | | `rsi` | 1.94× | **5.28×** |
        | **Rust slower than C Tulip** | `dx` | **0.75×** | — |
        | | `msw` | **0.85×** | — |
        | **Rust slower than TA-Lib** | `obv` | 1.00× | **0.94×** |
        | | `psar` | 1.12× | **0.76×** |
        | | `wma` | 1.35× | **0.78×** |

=== "vs Rust"

    Competitor: **RustTa** — 20 indicators benchmarked.

    | Indicator | Rust (ns) | RustTa (ns) | RustTa / Rust |
    |-----------|----------:|------------:|--------------:|
    | atr | 4,559 | 9,342 | 2.05 |
    | bbands | 7,114 | 8,093 | 1.14 |
    | cci | 55,726 | 65,098 | 1.17 |
    | chandelierexit | 17,252 | 42,341 | 2.45 |
    | ef | 4,574 | 26,694 | 5.84 |
    | ema | 4,547 | 7,919 | 1.74 |
    | keltnerchannel | 6,182 | 12,597 | 2.04 |
    | macd | 7,045 | 7,891 | 1.12 |
    | max | 5,360 | 17,229 | 3.21 |
    | md | 12,120 | 26,952 | 2.22 |
    | mfi | 7,513 | 23,766 | 3.16 |
    | min | 7,808 | 22,910 | 2.93 |
    | obv | 3,325 | 5,216 | 1.57 |
    | ppo | 4,823 | 7,900 | 1.64 |
    | roc | 2,294 | 2,860 | 1.25 |
    | rsi | 4,719 | 7,909 | 1.68 |
    | sma | 2,451 | 4,533 | 1.85 |
    | stddev | 3,601 | 8,346 | 2.32 |
    | stoch | 20,391 | 46,721 | 2.29 |
    | tr | 1,545 | 7,502 | 4.86 |

    ??? success "Notable results"

        Rust beats RustTa on **all 20 compared indicators**.

        | Category | Indicator | RustTa / Rust |
        |----------|-----------|:-------------:|
        | **Largest wins** | `ef` | **5.84×** |
        | | `tr` | **4.86×** |
        | | `max` | **3.21×** |
        | | `mfi` | **3.16×** |
        | | `min` | **2.93×** |
        | **Closest** | `macd` | 1.12× |
        | | `bbands` | 1.14× |
        | | `cci` | 1.17× |

=== "Python Binding"

    Competitor: **ta** (bukosabino/ta, pandas-based), called via the **`tulip_rs_python`** PyO3 binding.
    Rust native times are shown for reference — they reflect the underlying computation cost before PyO3 overhead is added.
    See [Python Binding](python.md) for setup and how to run.

    | Indicator | Rust native (ns) | tulip_rs_python (ns) | ta (ns) | ta / Python |
    |-----------|----------------:|---------------------:|--------:|------------:|
    | `ad` | 4,779 | 12,244 | 152,134 | 12× |
    | `adx` | 9,999 | 73,271 | 21,609,413 | 295× |
    | `ao` | 5,393 | 6,731 | 171,799 | 26× |
    | `aroon` | 17,988 | 24,274 | 9,945,796 | 410× |
    | `atr` | 4,559 | 12,450 | 11,952,345 | 960× |
    | `bbands` | 7,114 | 45,436 | 222,692 | 5× |
    | `cci` | 55,726 | 60,716 | 25,316,041 | 417× |
    | `chaikinmf` | 6,833 | 10,203 | 266,671 | 26× |
    | `dema` | 5,993 | 31,406 | 127,379 | 4× |
    | `donchianchannel` | 15,182 | 21,220 | 259,165 | 12× |
    | `dpo` | 2,528 | 3,470 | 119,586 | 34× |
    | `ema` | 4,547 | 11,427 | 53,971 | 5× |
    | `emv` | 2,295 | 3,232 | 153,166 | 47× |
    | `hma` | 8,114 | 12,682 | 9,786,766 | 772× |
    | `kama` | 6,892 | 28,224 | 4,335,510 | 154× |
    | `keltnerchannel` | 6,182 | 24,853 | 393,355 | 16× |
    | `macd` | 7,045 | 35,954 | 187,289 | 5× |
    | `mass` | 5,568 | 24,558 | 201,046 | 8× |
    | `mfi` | 7,513 | 17,490 | 29,447,695 | 1,684× |
    | `mom` | 838 | 1,820 | 18,970 | 10× |
    | `nvi` | 2,395 | 4,613 | 63,459,538 | 13,757× |
    | `obv` | 3,325 | 4,994 | 116,664 | 23× |
    | `ppo` | 4,823 | 22,515 | 218,643 | 10× |
    | `psar` | 10,055 | 26,031 | 180,119,421 | 6,919× |
    | `roc` | 2,294 | 5,015 | 93,173 | 19× |
    | `rsi` | 4,719 | 24,063 | 460,166 | 19× |
    | `sma` | 2,451 | 3,609 | 66,403 | 18× |
    | `stoch` | 20,391 | 28,012 | 287,919 | 10× |
    | `stochrsi` | 19,106 | 53,442 | 856,806 | 16× |
    | `tema` | 6,734 | 63,883 | 204,996 | 3× |
    | `trix` | 6,457 | 64,496 | 293,063 | 5× |
    | `ultosc` | 16,033 | 20,992 | 1,637,169 | 78× |
    | `vortex` | 7,504 | 14,512 | 911,423 | 63× |
    | `willr` | 17,218 | 23,020 | 278,524 | 12× |
    | `wma` | 6,155 | 7,493 | 3,262,180 | 435× |

    ??? success "Notable results"

        `tulip_rs_python` beats `ta` on **all 35 compared indicators**. Median speedup: **~19×**.

        The size of the win depends entirely on how `ta` implements each indicator:

        **`ta` falls back to pure-Python loops** — `tulip_rs_python` wins by the largest margin:

        | Indicator | ta / Python | ta implementation |
        |-----------|:-----------:|-------------------|
        | `nvi` | **13,757×** | Pure-Python loop |
        | `psar` | **6,919×** | Pure-Python loop |
        | `mfi` | **1,684×** | Pure-Python loop |
        | `atr` | **960×** | Pure-Python loop |
        | `hma` | **772×** | Pure-Python loop |
        | `wma` | **435×** | Pure-Python loop |
        | `cci` | **417×** | Pure-Python loop |
        | `aroon` | **410×** | Pure-Python loop |
        | `adx` | **295×** | Pure-Python loop |
        | `kama` | **154×** | Pure-Python loop |

        **`ta` uses pandas/numpy C paths** — the gap narrows because both sides use compiled code; PyO3 call overhead (~5–25 µs) is visible here:

        | Indicator | Rust native (ns) | tulip_rs_python (ns) | ta (ns) | ta / Python |
        |-----------|----------------:|---------------------:|--------:|:-----------:|
        | `tema` | 6,734 | 63,883 | 204,996 | **3×** |
        | `dema` | 5,993 | 31,406 | 127,379 | **4×** |
        | `trix` | 6,457 | 64,496 | 293,063 | **5×** |
        | `ema` | 4,547 | 11,427 | 53,971 | **5×** |
        | `bbands` | 7,114 | 45,436 | 222,692 | **5×** |
        | `macd` | 7,045 | 35,954 | 187,289 | **5×** |
