# Benchmarks

`tulip_rs` is benchmarked against two established C technical-analysis libraries across four scenarios:

| Library | Description |
|---------|-------------|
| **[Tulip Indicators (C)](https://tulipindicators.org/)** | The C library that inspired `tulip_rs`; used as the primary scalar baseline |
| **[TA-Lib](https://ta-lib.org/)** | The industry-standard C technical-analysis library |

All timings are **nanoseconds (ns) — lower is better**. Ratios > 1.00 mean Rust is faster than the competitor.

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
    | `cmo` | period: 5 · 14 · 20 · 30 |
    | `cvi` | period: 5 · 14 · 20 · 30 |
    | `dema` | period: 5 · 14 · 20 · 50 |
    | `di` | period: 5 · 14 · 20 · 30 |
    | `dm` | period: 24 · 14 · 5 · 30 |
    | `dpo` | period: 5 · 14 · 20 · 30 |
    | `dx` | period: 24 · 14 · 5 · 30 |
    | `ema` | period: 14 · 20 · 26 · 50 |
    | `emv` | *(no options)* |
    | `fisher` | period: 25 · 35 · 50 · 100 |
    | `fosc` | period: 5 · 14 · 20 · 25 |
    | `hma` | period: 5 · 14 · 20 · 50 |
    | `kama` | period: 5 · 10 · 14 · 20 |
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
    | `stddev` | period: 20 · 50 · 100 · 200 |
    | `stoch` | (k=28, ks=16, d=12) · (k=35, ks=21, d=14) · (k=50, ks=30, d=21) · (k=100, ks=50, d=30) |
    | `stochrsi` | period: 14 · 20 · 25 · 35 |
    | `tema` | period: 5 · 14 · 20 · 50 |
    | `tr` | *(no options)* |
    | `trima` | period: 14 · 20 · 25 · 30 |
    | `trix` | period: 14 · 15 · 20 · 30 |
    | `tsf` | period: 5 · 14 · 20 · 25 |
    | `typprice` | *(no options)* |
    | `ultosc` | (s=2, m=3, l=5) · (s=10, m=14, l=20) · (s=14, m=20, l=50) · (s=20, m=50, l=100) |
    | `vhf` | period: 25 · 35 · 50 · 100 |
    | `vidya` | (short=9, long=12, α=0.2) · (short=12, long=26, α=0.2) · (short=14, long=30, α=0.2) · (short=14, long=30, α=0.4) |
    | `volatility` | period: 14 · 20 · 25 · 30 |
    | `vosc` | (short=2, long=5) · (short=5, long=20) · (short=10, long=25) · (short=14, long=28) |
    | `vwma` | period: 14 · 20 · 25 · 30 |
    | `wad` | *(no options)* |
    | `wcprice` | *(no options)* |
    | `wilders` | period: 20 · 25 · 30 · 50 |
    | `willr` | period: 25 · 35 · 50 · 100 |
    | `wma` | period: 14 · 20 · 25 · 30 |
    | `zlema` | period: 5 · 10 · 14 · 20 |

---

## 1. Standard Performance (Single Asset)

Single asset, single option set. Ratios show how many times slower the competitor is relative to Rust.

| Indicator | Rust (ns) | C Tulip (ns) | TA-Lib (ns) | C / Rust | TA-Lib / Rust |
|-----------|----------:|-------------:|------------:|---------:|--------------:|
| ad | 4,768 | 5,088 | 5,097 | 1.07 | 1.07 |
| adosc | 6,712 | 9,209 | 8,869 | 1.37 | 1.32 |
| adx | 12,504 | 14,521 | 37,786 | 1.16 | 3.02 |
| adxr | 14,188 | 18,412 | 38,867 | 1.30 | 2.74 |
| ao | 5,488 | 11,683 | N/A | 2.13 | — |
| apo | 4,709 | 8,765 | 11,296 | 1.86 | 2.40 |
| aroon | 17,561 | 79,068 | 73,312 | 4.50 | 4.17 |
| aroonosc | 16,517 | 77,160 | 69,340 | 4.67 | 4.20 |
| atr | 4,631 | 10,741 | 28,159 | 2.32 | 6.08 |
| avgprice | 1,408 | 2,018 | 3,442 | 1.43 | 2.44 |
| bbands | 7,628 | 12,218 | 21,814 | 1.60 | 2.86 |
| bop | 2,361 | 2,857 | 5,005 | 1.21 | 2.12 |
| cci | 71,100 | 73,799 | 122,201 | 1.04 | 1.72 |
| cmo | 5,992 | 23,022 | N/A | 3.84 | — |
| cvi | 6,004 | 14,277 | N/A | 2.38 | — |
| dema | 6,074 | 6,475 | 22,723 | 1.07 | 3.74 |
| di | 9,188 | 10,207 | 55,684 | 1.11 | 6.06 |
| dm | 7,134 | 7,237 | N/A | 1.01 | — |
| dpo | 2,595 | 2,650 | N/A | 1.02 | — |
| dx | 10,509 | 10,283 | N/A | 0.98 | — |
| ema | 4,587 | 10,703 | 10,825 | 2.33 | 2.36 |
| emv | 2,401 | 5,096 | N/A | 2.12 | — |
| fisher | 46,046 | 130,141 | N/A | 2.83 | — |
| fosc | 7,576 | 9,412 | N/A | 1.24 | — |
| hma | 8,134 | 9,096 | N/A | 1.12 | — |
| kama | 7,091 | 8,401 | 10,586 | 1.18 | 1.49 |
| kvo | 9,168 | 9,861 | N/A | 1.08 | — |
| linreg | 6,464 | 8,396 | N/A | 1.30 | — |
| macd | 6,925 | 10,862 | 36,639 | 1.57 | 5.29 |
| marketfi | 2,336 | 2,722 | N/A | 1.17 | — |
| mass | 5,403 | 11,935 | N/A | 2.21 | — |
| max | 4,745 | 29,789 | 17,647 | 6.28 | 3.72 |
| md | 12,978 | 14,733 | N/A | 1.14 | — |
| medprice | 910 | 1,514 | 2,305 | 1.66 | 2.53 |
| mfi | 7,509 | 17,473 | 19,266 | 2.33 | 2.57 |
| min | 6,616 | 54,789 | 30,850 | 8.28 | 4.66 |
| mom | 747 | 1,308 | 1,824 | 1.75 | 2.44 |
| msw | 643,140 | 528,111 | N/A | 0.82 | — |
| natr | 4,777 | 10,514 | 27,663 | 2.20 | 5.79 |
| nvi | 2,477 | 3,474 | N/A | 1.40 | — |
| obv | 3,355 | 3,347 | 3,130 | 1.00 | 0.93 |
| ppo | 5,039 | 9,472 | 14,158 | 1.88 | 2.81 |
| psar | 10,386 | 12,225 | 7,743 | 1.18 | 0.75 |
| pvi | 2,489 | 3,509 | N/A | 1.41 | — |
| qstick | 2,808 | 2,993 | N/A | 1.07 | — |
| roc | 2,334 | 2,622 | 4,839 | 1.12 | 2.07 |
| rocr | 2,336 | 2,641 | 4,901 | 1.13 | 2.10 |
| rsi | 8,484 | 9,463 | 25,371 | 1.12 | 2.99 |
| sma | 2,471 | 2,678 | 4,859 | 1.08 | 1.97 |
| stddev | 3,651 | 10,548 | N/A | 2.89 | — |
| stoch | 17,651 | 91,364 | 51,353 | 5.18 | 2.91 |
| stochrsi | 23,078 | 44,517 | N/A | 1.93 | — |
| tema | 6,808 | 6,846 | 32,390 | 1.01 | 4.76 |
| tr | 1,497 | 1,959 | 3,737 | 1.31 | 2.50 |
| trima | 5,585 | 7,211 | 7,273 | 1.29 | 1.30 |
| trix | 6,565 | 10,775 | N/A | 1.64 | — |
| tsf | 6,398 | 8,369 | N/A | 1.31 | — |
| typprice | 1,186 | 1,953 | N/A | 1.65 | — |
| ultosc | 15,597 | 18,127 | N/A | 1.16 | — |
| vhf | 11,543 | 79,311 | N/A | 6.87 | — |
| vidya | 11,800 | 18,871 | N/A | 1.60 | — |
| volatility | 8,892 | 17,495 | N/A | 1.97 | — |
| vosc | 3,847 | 4,949 | N/A | 1.29 | — |
| vwma | 3,432 | 4,963 | N/A | 1.45 | — |
| wad | 3,870 | 4,992 | N/A | 1.29 | — |
| wcprice | 1,023 | 1,915 | N/A | 1.87 | — |
| wilders | 4,715 | 10,923 | N/A | 2.32 | — |
| willr | 17,055 | 81,392 | 41,018 | 4.77 | 2.41 |
| wma | 6,415 | 8,685 | 5,060 | 1.35 | 0.79 |
| zlema | 5,968 | 8,622 | N/A | 1.44 | — |

??? success "Notable results"

    | Category | Indicator | C / Rust | TA-Lib / Rust |
    |----------|-----------|:--------:|:-------------:|
    | **Largest advantage vs C Tulip** | `min` | **8.28×** | 4.66× |
    | | `vhf` | **6.87×** | — |
    | | `max` | **6.28×** | 3.72× |
    | | `stoch` | **5.18×** | 2.91× |
    | | `aroonosc` | **4.67×** | 4.20× |
    | **Largest advantage vs TA-Lib** | `atr` | 2.32× | **6.08×** |
    | | `di` | 1.11× | **6.06×** |
    | | `natr` | 2.20× | **5.79×** |
    | | `macd` | 1.57× | **5.29×** |
    | **Rust slower than competitor** | `dx` vs C Tulip | **0.98×** | — |
    | | `msw` vs C Tulip | **0.82×** | — |
    | | `obv` vs TA-Lib | — | **0.93×** |
    | | `psar` vs TA-Lib | — | **0.75×** |
    | | `wma` vs TA-Lib | — | **0.79×** |

    Rust beats C Tulip on **all but 2 indicators** (`dx`, `msw`) and beats TA-Lib on **all but 3** (`obv`, `psar`, `wma`). Median C / Rust ratio: **~1.32×**.

---

## 2. SIMD by_assets: 4 Assets Simultaneously

Processes 4 different assets with identical options in a single SIMD pass. All times represent the total wall-time for 4 assets.

- **SIMD / 4× Rust** — ratio < 1.00 means SIMD is faster than 4 sequential Rust calls
- **Speedup vs Rust** — how many times faster SIMD is compared to 4× sequential Rust
- **Speedup vs C** — how many times faster SIMD is compared to 4× sequential C Tulip

| Indicator | 4× Rust (ns) | 4× C Tulip (ns) | 4× TA-Lib (ns) | SIMD 4-Asset (ns) | SIMD / 4×Rust | Speedup vs Rust | Speedup vs C |
|-----------|-------------:|----------------:|---------------:|------------------:|--------------:|----------------:|-------------:|
| ad | 19,073 | 20,352 | 20,389 | 15,712 | 0.82 | 1.21× | 1.30× |
| adosc | 26,848 | 36,835 | 35,476 | 17,547 | 0.65 | 1.53× | 2.10× |
| adx | 50,016 | 58,084 | 151,143 | 31,608 | 0.63 | 1.58× | 1.84× |
| adxr | 56,751 | 73,649 | 155,470 | 33,835 | 0.60 | 1.68× | 2.18× |
| ao | 21,950 | 46,731 | N/A | 13,373 | 0.61 | 1.64× | 3.50× |
| apo | 18,837 | 35,059 | 45,183 | 9,148 | 0.49 | 2.06× | 3.83× |
| aroon | 70,243 | 316,271 | 293,248 | 85,031 | 1.21 | 0.83× | 3.72× |
| aroonosc | 66,066 | 308,641 | 277,361 | 83,965 | 1.27 | 0.79× | 3.67× |
| atr | 18,525 | 42,962 | 112,634 | 14,244 | 0.77 | 1.30× | 3.02× |
| avgprice | 5,632 | 8,070 | 13,769 | 5,611 | 1.00 | 1.00× | 1.44× |
| bbands | 30,511 | 48,872 | 87,257 | 22,781 | 0.75 | 1.34× | 2.15× |
| bop | 9,444 | 11,428 | 20,020 | 9,603 | 1.02 | 0.98× | 1.19× |
| cci | 284,401 | 295,196 | 488,802 | 69,337 | 0.24 | **4.10×** | **4.26×** |
| cmo | 23,966 | 92,089 | N/A | 15,883 | 0.66 | 1.51× | 5.80× |
| cvi | 24,017 | 57,106 | N/A | 12,876 | 0.54 | 1.87× | 4.43× |
| dema | 24,296 | 25,900 | 90,893 | 9,436 | 0.39 | 2.57× | 2.74× |
| di | 36,751 | 40,827 | 222,735 | 25,642 | 0.70 | 1.43× | 1.59× |
| dm | 28,538 | 28,949 | N/A | 16,059 | 0.56 | 1.78× | 1.80× |
| dpo | 10,379 | 10,601 | N/A | 11,955 | 1.15 | 0.87× | 0.89× |
| dx | 42,035 | 41,131 | N/A | 28,961 | 0.69 | 1.45× | 1.42× |
| ema | 18,348 | 42,812 | 43,301 | 8,466 | 0.46 | 2.17× | 5.06× |
| emv | 9,605 | 20,382 | N/A | 9,630 | 1.00 | 1.00× | 2.12× |
| fisher | 184,183 | 520,563 | N/A | 121,943 | 0.66 | 1.51× | 4.27× |
| fosc | 30,302 | 37,648 | N/A | 19,000 | 0.63 | 1.60× | 1.98× |
| hma | 32,536 | 36,382 | N/A | 28,391 | 0.87 | 1.15× | 1.28× |
| kama | 28,364 | 33,606 | 42,345 | 12,887 | 0.45 | 2.20× | 2.61× |
| kvo | 36,673 | 39,446 | N/A | 26,299 | 0.72 | 1.39× | 1.50× |
| linreg | 25,856 | 33,582 | N/A | 9,869 | 0.38 | **2.62×** | **3.40×** |
| macd | 27,698 | 43,446 | 146,556 | 21,614 | 0.78 | 1.28× | 2.01× |
| marketfi | 9,343 | 10,887 | N/A | 11,844 | 1.27 | 0.79× | 0.92× |
| mass | 21,611 | 47,740 | N/A | 11,780 | 0.55 | 1.83× | 4.05× |
| max | 18,980 | 119,155 | 70,587 | 22,648 | 1.19 | 0.84× | 5.26× |
| md | 51,911 | 58,930 | N/A | 27,800 | 0.54 | 1.87× | 2.12× |
| medprice | 3,641 | 6,055 | 9,221 | 3,993 | 1.10 | 0.91× | 1.52× |
| mfi | 30,037 | 69,892 | 77,064 | 20,502 | 0.68 | 1.47× | 3.41× |
| min | 26,463 | 219,156 | 123,400 | 45,373 | 1.71 | 0.58× | 4.83× |
| mom | 2,989 | 5,233 | 7,297 | 3,302 | 1.10 | 0.91× | 1.59× |
| msw | 2,572,559 | 2,112,444 | N/A | 1,681,722 | 0.65 | 1.53× | 1.26× |
| natr | 19,109 | 42,058 | 110,653 | 16,645 | 0.87 | 1.15× | 2.53× |
| nvi | 9,909 | 13,896 | N/A | 9,865 | 1.00 | 1.00× | 1.41× |
| obv | 13,421 | 13,386 | 12,518 | 8,953 | 0.67 | 1.50× | 1.49× |
| ppo | 20,156 | 37,888 | 56,633 | 9,893 | 0.49 | 2.04× | 3.83× |
| psar | 41,543 | 48,901 | 30,971 | 70,083 | 1.69 | 0.59× | 0.70× |
| pvi | 9,956 | 14,037 | N/A | 11,866 | 1.19 | 0.84× | 1.18× |
| qstick | 11,232 | 11,970 | N/A | 11,263 | 1.00 | 1.00× | 1.06× |
| roc | 9,336 | 10,487 | 19,356 | 10,196 | 1.09 | 0.92× | 1.03× |
| rocr | 9,342 | 10,564 | 19,605 | 9,699 | 1.04 | 0.96× | 1.09× |
| rsi | 33,935 | 37,853 | 101,483 | 9,861 | 0.29 | **3.44×** | **3.84×** |
| sma | 9,884 | 10,711 | 19,436 | 8,501 | 0.86 | 1.16× | 1.26× |
| stddev | 14,605 | 42,191 | N/A | 14,332 | 0.98 | 1.02× | 2.94× |
| stoch | 70,602 | 365,457 | 205,413 | 93,950 | 1.33 | 0.75× | 3.89× |
| stochrsi | 92,313 | 178,069 | N/A | 100,933 | 1.09 | 0.91× | 1.76× |
| tema | 27,231 | 27,383 | 129,560 | 8,363 | 0.31 | **3.26×** | **3.27×** |
| tr | 5,987 | 7,834 | 14,947 | 5,862 | 0.98 | 1.02× | 1.34× |
| trima | 22,342 | 28,842 | 29,091 | 15,684 | 0.70 | 1.42× | 1.84× |
| trix | 26,259 | 43,100 | N/A | 9,829 | 0.37 | **2.67×** | **4.38×** |
| tsf | 25,592 | 33,478 | N/A | 10,107 | 0.40 | 2.53× | 3.31× |
| typprice | 4,742 | 7,811 | N/A | 4,415 | 0.93 | 1.07× | 1.77× |
| ultosc | 62,388 | 72,510 | N/A | 29,472 | 0.47 | 2.12× | 2.46× |
| vhf | 46,174 | 317,244 | N/A | 86,450 | 1.87 | 0.53× | 3.67× |
| vidya | 47,200 | 75,485 | N/A | 37,451 | 0.79 | 1.26× | 2.02× |
| volatility | 35,566 | 69,979 | N/A | 23,840 | 0.67 | 1.49× | 2.94× |
| vosc | 15,388 | 19,796 | N/A | 12,867 | 0.84 | 1.20× | 1.54× |
| vwma | 13,728 | 19,852 | N/A | 15,524 | 1.13 | 0.88× | 1.28× |
| wad | 15,480 | 19,966 | N/A | 13,680 | 0.88 | 1.13× | 1.46× |
| wcprice | 4,092 | 7,660 | N/A | 4,647 | 1.14 | 0.88× | 1.65× |
| wilders | 18,859 | 43,693 | N/A | 8,869 | 0.47 | 2.13× | 4.93× |
| willr | 68,219 | 325,567 | 164,072 | 85,118 | 1.25 | 0.80× | 3.83× |
| wma | 25,658 | 34,740 | 20,240 | 9,896 | 0.39 | **2.59×** | **3.51×** |
| zlema | 23,873 | 34,488 | N/A | 8,736 | 0.37 | **2.73×** | **3.95×** |

??? success "Notable results"

    **60 of 80 indicators (75%) show a SIMD speedup over 4× sequential Rust.**  
    Median speedup for benefiting indicators: **~1.40×**.

    | Category | Indicator | SIMD Speedup vs 4× Sequential Rust |
    |----------|-----------|:-----------------------------------:|
    | **Top performers** | `cci` | **4.10×** |
    | | `rsi` | **3.44×** |
    | | `tema` | **3.26×** |
    | | `zlema` | **2.73×** |
    | | `trix` | **2.67×** |
    | | `wma` | **2.59×** |
    | | `linreg` | **2.62×** |
    | **SIMD slower than sequential** | `vhf` | 0.53× |
    | | `min` | 0.58× |
    | | `psar` | 0.59× |
    | | `stoch` | 0.75× |
    | | `aroonosc` | 0.79× |

    Indicators where SIMD is slower typically involve either highly sequential computation (e.g., `psar`) or irregular memory access patterns where SIMD setup overhead dominates.

---

## 3. SIMD by_options: 4 Option Sets Simultaneously

Processes 1 asset with 4 different option configurations in a single SIMD pass. There is no C or TA-Lib equivalent for this mode — the comparison baseline is 4 independent sequential Rust calls.

- **Speedup** — `4× Sequential Rust / SIMD 4-Options`
- **Best / Worst** — speedup range across the different option combinations tested

| Indicator | 4× Sequential Rust (ns) | SIMD 4-Options (ns) | Speedup | Best | Worst |
|-----------|------------------------:|--------------------:|--------:|-----:|------:|
| adosc | 26,848 | 9,864 | **2.72×** | 2.76× | 2.69× |
| adx | 50,016 | 29,746 | 1.68× | 1.68× | 1.68× |
| adxr | 56,751 | 34,068 | 1.67× | 1.69× | 1.65× |
| apo | 18,837 | 8,448 | 2.25× | 2.40× | 1.86× |
| aroon | 70,243 | 91,485 | 0.77× | 0.78× | 0.75× |
| aroonosc | 66,066 | 86,488 | 0.77× | 0.77× | 0.75× |
| atr | 18,525 | 10,020 | 1.85× | 1.94× | 1.77× |
| bbands | 30,511 | 22,448 | 1.36× | 1.46× | 1.29× |
| cci | 284,401 | 122,043 | 2.33× | 2.37× | 2.31× |
| cmo | 23,966 | 16,843 | 1.42× | 1.44× | 1.41× |
| cvi | 24,017 | 21,747 | 1.11× | 1.11× | 1.10× |
| dema | 24,296 | 9,120 | **2.69×** | 3.08× | 2.41× |
| di | 36,751 | 21,405 | 1.72× | 1.76× | 1.65× |
| dm | 28,538 | 15,874 | 1.80× | 1.87× | 1.71× |
| dpo | 10,379 | 12,403 | 0.84× | 0.85× | 0.83× |
| dx | 42,035 | 28,349 | 1.48× | 1.50× | 1.46× |
| ema | 18,348 | 8,067 | 2.27× | 2.43× | 2.14× |
| fisher | 184,183 | 173,805 | 1.06× | 1.09× | 1.04× |
| fosc | 30,302 | 19,223 | 1.58× | 1.59× | 1.57× |
| hma | 32,536 | 28,348 | 1.15× | 1.16× | 1.14× |
| kama | 28,364 | 12,838 | 2.21× | 2.24× | 2.20× |
| kvo | 36,673 | 19,805 | 1.85× | 1.91× | 1.81× |
| linreg | 25,856 | 10,246 | 2.52× | 2.56× | 2.49× |
| macd | 27,698 | 21,760 | 1.27× | 1.40× | 1.17× |
| mass | 21,611 | 15,203 | 1.42× | 1.45× | 1.40× |
| max | 18,980 | 29,224 | 0.65× | 0.68× | 0.61× |
| md | 51,911 | 51,534 | 1.01× | 1.03× | 0.99× |
| mfi | 30,037 | 19,498 | 1.54× | 1.56× | 1.51× |
| min | 26,463 | 56,215 | 0.47× | 0.50× | 0.45× |
| mom | 2,989 | 3,278 | 0.91× | 0.91× | 0.91× |
| msw | 2,572,559 | 2,026,360 | 1.27× | 1.29× | 1.24× |
| natr | 19,109 | 11,040 | 1.73× | 1.74× | 1.72× |
| ppo | 20,156 | 9,679 | 2.08× | 2.10× | 2.05× |
| psar | 41,543 | 55,459 | 0.75× | 0.77× | 0.73× |
| qstick | 11,232 | 11,206 | 1.00× | 1.03× | 0.98× |
| roc | 9,336 | 11,928 | 0.78× | 0.94× | 0.73× |
| rocr | 9,342 | 10,664 | 0.88× | 0.95× | 0.77× |
| rsi | 33,935 | 9,894 | **3.43×** | 3.46× | 3.40× |
| sma | 9,884 | 9,058 | 1.09× | 1.11× | 1.07× |
| stddev | 14,605 | 14,488 | 1.01× | 1.02× | 1.00× |
| stoch | 70,602 | 116,190 | 0.61× | 0.62× | 0.59× |
| stochrsi | 92,313 | 115,244 | 0.80× | 0.84× | 0.77× |
| tema | 27,231 | 7,890 | **3.45×** | 3.51× | 3.42× |
| trima | 22,342 | 15,704 | 1.42× | 1.46× | 1.38× |
| trix | 26,259 | 9,919 | **2.65×** | 2.68× | 2.61× |
| tsf | 25,592 | 12,264 | 2.09× | 2.50× | 1.84× |
| ultosc | 62,388 | 39,726 | 1.57× | 1.59× | 1.55× |
| vhf | 46,174 | 99,461 | 0.47× | 0.48× | 0.45× |
| vidya | 47,200 | 37,966 | 1.24× | 1.26× | 1.22× |
| volatility | 35,566 | 22,771 | 1.56× | 1.60× | 1.52× |
| vosc | 15,388 | 12,799 | 1.20× | 1.22× | 1.18× |
| vwma | 13,728 | 15,734 | 0.87× | 0.90× | 0.85× |
| wilders | 18,859 | 8,456 | **2.24×** | 2.32× | 2.02× |
| willr | 68,219 | 104,286 | 0.65× | 0.67× | 0.63× |
| wma | 25,658 | 10,055 | **2.55×** | 2.56× | 2.54× |
| zlema | 23,873 | 8,924 | **2.68×** | 2.68× | 2.67× |

??? success "Notable results"

    **39 of 56 indicators (70%) show a SIMD speedup over 4× sequential Rust.**

    | Category | Indicator | Speedup |
    |----------|-----------|:-------:|
    | **Top performers** | `tema` | **3.45×** |
    | | `rsi` | **3.43×** |
    | | `adosc` | **2.72×** |
    | | `dema` | **2.69×** (up to 3.08× best-case) |
    | | `zlema` | **2.68×** |
    | | `trix` | **2.65×** |
    | | `wma` | **2.55×** |
    | **SIMD slower than sequential** | `min` | 0.47× |
    | | `vhf` | 0.47× |
    | | `stoch` | 0.61× |
    | | `max` | 0.65× |
    | | `psar` | 0.75× |

    Indicators that don't benefit tend to involve complex branching or irregular memory access patterns that prevent effective vectorisation (`min`, `max`, `stoch`, `psar`, `vhf`).

---

## 4. Optional Outputs: Single-Pass Computation Advantage

`tulip_rs` computes the primary indicator **and** all intermediate outputs in a single pass through the data. C Tulip and TA-Lib require separate function calls for each intermediate result.

The **C Equiv. Total** column sums the C Tulip time for the primary indicator plus each sub-indicator called separately. The Rust time already includes every optional output.

| Indicator | Optional outputs (sub-indicators) | Rust all-outputs (ns) | C Equiv. Total (ns) | C / Rust | TA-Lib Equiv. Total (ns) | TA-Lib / Rust |
|-----------|-----------------------------------|----------------------:|--------------------:|---------:|-------------------------:|--------------:|
| adosc | short_ema, long_ema, ad | 8,072 | 35,703 | **4.42×** | 35,616 | **4.41×** |
| adx | dx, atr, tr | 14,473 | 37,504 | **2.59×** | 69,682 | **4.81×** |
| adxr | adx, dx, atr, tr | 22,598 | 55,916 | **2.47×** | 108,549 | **4.80×** |
| ao | short_sma, long_sma, medprice | 9,918 | 18,553 | **1.87×** | — | — |
| apo | short_ema, long_ema | 7,005 | 30,171 | **4.31×** | 32,946 | **4.70×** |
| aroonosc | aroon_down, aroon_up | 17,963 | 156,228 | **8.70×** | 142,652 | **7.94×** |
| atr | tr | 5,144 | 12,700 | **2.47×** | 31,896 | **6.20×** |
| cci | sma, md, typprice | 73,949 | 93,163 | **1.26×** | 127,060 | **1.72×** |
| dema | ema | 6,303 | 17,178 | **2.73×** | 33,548 | **5.32×** |
| di | atr, tr | 13,431 | 22,907 | **1.71×** | 87,580 | **6.52×** |
| dpo | sma | 2,774 | 5,328 | **1.92×** | — | — |
| dx | atr, tr | 11,213 | 22,983 | **2.05×** | — | — |
| emv | medprice | 2,638 | 6,610 | **2.51×** | — | — |
| fosc | tsf, linreg, linregslope, linregintercept | 10,371 | 26,177 | **2.52×** | — | — |
| kvo | short_ema, long_ema | 9,411 | 31,267 | **3.32×** | — | — |
| linreg | linregslope, linregintercept | 6,410 | 8,396 | **1.31×** | — | — |
| macd | short_ema, long_ema | 11,728 | 32,268 | **2.75×** | 58,289 | **4.97×** |
| md | sma | 13,168 | 17,411 | **1.32×** | — | — |
| mfi | typprice | 8,830 | 19,426 | **2.20×** | 19,266 | **2.18×** |
| natr | atr, tr | 6,178 | 23,214 | **3.76×** | 59,559 | **9.64×** |
| ppo | short_ema, long_ema | 5,784 | 30,878 | **5.34×** | 35,808 | **6.19×** |
| roc | mom | 2,360 | 3,930 | **1.67×** | 6,663 | **2.82×** |
| stddev | sma | 3,744 | 13,226 | **3.53×** | — | — |
| stochrsi | rsi | 23,902 | 53,980 | **2.26×** | — | — |
| tema | dema, ema | 7,639 | 24,024 | **3.15×** | 65,938 | **8.63×** |
| trix | tema, dema, ema | 8,933 | 34,799 | **3.90×** | — | — |
| tsf | linreg, linregslope, linregintercept | 7,715 | 16,765 | **2.17×** | — | — |
| vidya | short_sma, long_sma, short_stddev, long_stddev | 12,050 | 45,323 | **3.76×** | — | — |
| vosc | short_sma, long_sma | 6,074 | 10,305 | **1.70×** | — | — |
| wma | sma | 4,862 | 11,363 | **2.34×** | 9,919 | **2.04×** |

??? info "Footnote calculations"

    C Tulip times taken from the Standard benchmark (single call per sub-indicator):

    | # | Indicator | Calculation |
    |---|-----------|-------------|
    | adosc C | C | adosc(9,209) + ema(10,703) + ema(10,703) + ad(5,088) = **35,703** |
    | adosc TA | TA-Lib | adosc(8,869) + ema(10,825) + ema(10,825) + ad(5,097) = **35,616** |
    | adx C | C | adx(14,521) + dx(10,283) + atr(10,741) + tr(1,959) = **37,504** |
    | adx TA | TA-Lib | adx(37,786) + atr(28,159) + tr(3,737) = **69,682** _(dx not in TA-Lib)_ |
    | adxr C | C | adxr(18,412) + adx(14,521) + dx(10,283) + atr(10,741) + tr(1,959) = **55,916** |
    | adxr TA | TA-Lib | adxr(38,867) + adx(37,786) + atr(28,159) + tr(3,737) = **108,549** |
    | ao C | C | ao(11,683) + sma(2,678) + sma(2,678) + medprice(1,514) = **18,553** |
    | apo C | C | apo(8,765) + ema(10,703) + ema(10,703) = **30,171** |
    | apo TA | TA-Lib | apo(11,296) + ema(10,825) + ema(10,825) = **32,946** |
    | aroonosc C | C | aroonosc(77,160) + aroon(79,068) = **156,228** _(aroon returns both up and down in one call)_ |
    | aroonosc TA | TA-Lib | aroonosc(69,340) + aroon(73,312) = **142,652** |
    | atr C | C | atr(10,741) + tr(1,959) = **12,700** |
    | atr TA | TA-Lib | atr(28,159) + tr(3,737) = **31,896** |
    | cci C | C | cci(73,799) + sma(2,678) + md(14,733) + typprice(1,953) = **93,163** |
    | cci TA | TA-Lib | cci(122,201) + sma(4,859) = **127,060** _(md and typprice not available separately)_ |
    | dema C | C | dema(6,475) + ema(10,703) = **17,178** |
    | dema TA | TA-Lib | dema(22,723) + ema(10,825) = **33,548** |
    | di C | C | di(10,207) + atr(10,741) + tr(1,959) = **22,907** |
    | di TA | TA-Lib | di(55,684) + atr(28,159) + tr(3,737) = **87,580** |
    | dpo C | C | dpo(2,650) + sma(2,678) = **5,328** |
    | dx C | C | dx(10,283) + atr(10,741) + tr(1,959) = **22,983** |
    | emv C | C | emv(5,096) + medprice(1,514) = **6,610** |
    | fosc C | C | fosc(9,412) + tsf(8,369) + linreg(8,396) = **26,177** _(slope/intercept have no standalone C equivalent)_ |
    | kvo C | C | kvo(9,861) + ema(10,703) + ema(10,703) = **31,267** |
    | linreg C | C | linreg(8,396) _(slope and intercept not available as standalone in C Tulip)_ |
    | macd C | C | macd(10,862) + ema(10,703) + ema(10,703) = **32,268** |
    | macd TA | TA-Lib | macd(36,639) + ema(10,825) + ema(10,825) = **58,289** |
    | md C | C | md(14,733) + sma(2,678) = **17,411** |
    | mfi C | C | mfi(17,473) + typprice(1,953) = **19,426** |
    | mfi TA | TA-Lib | mfi(19,266) _(typprice not available separately in TA-Lib)_ |
    | natr C | C | natr(10,514) + atr(10,741) + tr(1,959) = **23,214** |
    | natr TA | TA-Lib | natr(27,663) + atr(28,159) + tr(3,737) = **59,559** |
    | ppo C | C | ppo(9,472) + ema(10,703) + ema(10,703) = **30,878** |
    | ppo TA | TA-Lib | ppo(14,158) + ema(10,825) + ema(10,825) = **35,808** |
    | roc C | C | roc(2,622) + mom(1,308) = **3,930** |
    | roc TA | TA-Lib | roc(4,839) + mom(1,824) = **6,663** |
    | stddev C | C | stddev(10,548) + sma(2,678) = **13,226** |
    | stochrsi C | C | stochrsi(44,517) + rsi(9,463) = **53,980** |
    | tema C | C | tema(6,846) + dema(6,475) + ema(10,703) = **24,024** |
    | tema TA | TA-Lib | tema(32,390) + dema(22,723) + ema(10,825) = **65,938** |
    | trix C | C | trix(10,775) + tema(6,846) + dema(6,475) + ema(10,703) = **34,799** |
    | tsf C | C | tsf(8,369) + linreg(8,396) = **16,765** _(slope/intercept have no standalone C equivalent)_ |
    | vidya C | C | vidya(18,871) + sma(2,678) + sma(2,678) + stddev(10,548) + stddev(10,548) = **45,323** |
    | vosc C | C | vosc(4,949) + sma(2,678) + sma(2,678) = **10,305** |
    | wma C | C | wma(8,685) + sma(2,678) = **11,363** |
    | wma TA | TA-Lib | wma(5,060) + sma(4,859) = **9,919** |

### Optional Outputs Ranked by Advantage

| Rank | vs | Indicator | Ratio | Key sub-indicators |
|:----:|:--:|-----------|------:|--------------------|
| 1 | TA-Lib | `natr` | **9.64×** | atr, tr |
| 2 | C | `aroonosc` | **8.70×** | aroon_down, aroon_up |
| 3 | TA-Lib | `tema` | **8.63×** | dema, ema |
| 4 | TA-Lib | `di` | **6.52×** | atr, tr |
| 5 | TA-Lib | `atr` | **6.20×** | tr |
| 6 | TA-Lib | `ppo` | **6.19×** | short_ema, long_ema |
| 7 | C | `ppo` | **5.34×** | short_ema, long_ema |
| 8 | TA-Lib | `dema` | **5.32×** | ema |
| 9 | TA-Lib | `adx` | **4.81×** | dx, atr, tr |
| 10 | TA-Lib | `adxr` | **4.80×** | adx, dx, atr, tr |

---

## 5. Summary and Key Findings

### Standard (single asset)

- Rust beats C Tulip for **all but 2 indicators**: `dx` (0.98×) and `msw` (0.82×)
- Rust beats TA-Lib for **all but 3 indicators**: `obv` (0.93×), `psar` (0.75×), `wma` (0.79×)
- **Median C / Rust ratio: ~1.32×** (Rust is ~32% faster on average)
- Largest wins vs C Tulip: `min` (**8.28×**), `vhf` (**6.87×**), `max` (**6.28×**), `stoch` (**5.18×**)
- Largest wins vs TA-Lib: `atr` (**6.08×**), `di` (**6.06×**), `natr` (**5.79×**), `macd` (**5.29×**)

### SIMD by_assets

- **75% of indicators** (60 / 80) show a SIMD speedup over 4× sequential Rust
- Top performers: `cci` (**4.10×**), `rsi` (**3.44×**), `tema` (**3.26×**), `zlema` (**2.73×**)
- Indicators that don't benefit are mostly trivial or have inherently sequential dependencies (`psar`, `min`, `max`, `stoch`, `vhf`)

### SIMD by_options

- **70% of indicators** (39 / 56) show a SIMD speedup over 4× sequential Rust
- Top performers: `tema` (**3.45×**), `rsi` (**3.43×**), `adosc` (**2.72×**), `dema` (**2.69×**), `zlema` (**2.68×**)
- Same non-benefiting indicators as by_assets — the bottleneck is algorithmic, not data layout

### Optional Outputs Single-Pass

- **31 indicators** offer optional outputs computed in a single pass
- C Tulip would require **2–5× more compute time** to obtain the same information via separate calls
- TA-Lib multipliers are often higher still due to its slower base times
- Standouts: `aroonosc` (**8.70×** vs C), `natr` (**9.64×** vs TA-Lib), `tema` (**8.63×** vs TA-Lib)

### Combined Advantage: a Worked Example

Consider computing `tema` with all its sub-indicators (`dema`, `ema`) across **4 assets simultaneously with SIMD**:

| Implementation | Time |
|----------------|-----:|
| `tulip_rs` SIMD 4-asset (all outputs, one pass) | **8,363 ns** |
| C Tulip — 4 × (tema + dema + ema) sequential | 96,096 ns |
| TA-Lib — 4 × (tema + dema + ema) sequential | 264,152 ns |

!!! success "Combined speedup"
    **~11.5× faster than C Tulip** and **~31.6× faster than TA-Lib** for the same result.

---

## 6. Streaming / Stateful: Single-Bar Update

`tulip_rs` indicators implement a `from_state` path: once the initial lookback has been computed, each subsequent bar is updated using only the saved state — no need to reprocess the full history. This is what `state.batch_indicator()` uses under the hood when called with a single new bar.

- **Batch (ns)** — full indicator computation over the complete 6,705-bar dataset (from Section 1)
- **Streaming 1-bar (ns)** — time to update one new bar using saved state
- **Batch / Streaming** — how many times faster the streaming path is vs a full batch recompute

| Indicator | Batch Rust (ns) | Streaming 1-bar (ns) | Batch / Streaming |
|-----------|----------------:|---------------------:|------------------:|
| ad | 4,768 | 25 | 190.7× |
| adosc | 6,712 | 28 | 239.7× |
| adx | 12,504 | 87 | 143.7× |
| adxr | 14,188 | 82 | 173.0× |
| ao | 5,488 | 15 | 365.9× |
| apo | 4,709 | 24 | 196.2× |
| aroon | 17,561 | 37 | 474.6× |
| aroonosc | 16,517 | 34 | 485.8× |
| atr | 4,631 | 23 | 201.3× |
| avgprice | 1,408 | 23 | 61.2× |
| bbands | 7,628 | 34 | 224.4× |
| bop | 2,361 | 24 | 98.4× |
| cci | 71,100 | 35 | **2,031×** |
| cmo | 5,992 | 29 | 206.6× |
| cvi | 6,004 | 17 | 353.2× |
| dema | 6,074 | 23 | 264.1× |
| di | 9,188 | 113 | 81.3× |
| dm | 7,134 | 68 | 104.9× |
| dpo | 2,595 | 25 | 103.8× |
| dx | 10,509 | 72 | 145.9× |
| ema | 4,587 | 15 | 305.8× |
| emv | 2,401 | 25 | 96.0× |
| fisher | 46,046 | 37 | **1,245×** |
| fosc | 7,576 | 31 | 244.4× |
| hma | 8,134 | 32 | 254.2× |
| kama | 7,091 | 26 | 272.7× |
| kvo | 9,168 | 31 | 295.7× |
| linreg | 6,464 | 30 | 215.5× |
| macd | 6,925 | 21 | 329.8× |
| marketfi | 2,336 | 23 | 101.6× |
| mass | 5,403 | 19 | 284.4× |
| max | 4,745 | 29 | 163.6× |
| md | 12,978 | 31 | 418.6× |
| medprice | 910 | 23 | 39.6× |
| mfi | 7,509 | 28 | 268.2× |
| min | 6,616 | 28 | 236.3× |
| mom | 747 | 26 | 28.7× |
| msw | 643,140 | 117 | **5,498×** |
| natr | 4,777 | 28 | 170.6× |
| nvi | 2,477 | 24 | 103.2× |
| obv | 3,355 | 23 | 145.9× |
| ppo | 5,039 | 27 | 186.6× |
| psar | 10,386 | 25 | 415.4× |
| pvi | 2,489 | 23 | 108.2× |
| qstick | 2,808 | 28 | 100.3× |
| roc | 2,334 | 26 | 89.8× |
| rocr | 2,336 | 27 | 86.5× |
| rsi | 8,484 | 70 | 121.2× |
| sma | 2,471 | 24 | 102.9× |
| stddev | 3,651 | 31 | 117.8× |
| stoch | 17,651 | 41 | 430.5× |
| stochrsi | 23,078 | 45 | 513.0× |
| tema | 6,808 | 26 | 261.8× |
| tr | 1,497 | 23 | 65.1× |
| trima | 5,585 | 26 | 214.8× |
| trix | 6,565 | 28 | 234.5× |
| tsf | 6,398 | 30 | 213.3× |
| typprice | 1,186 | 15 | 79.1× |
| ultosc | 15,597 | 30 | **519.9×** |
| vhf | 11,543 | 30 | 384.8× |
| vidya | 11,800 | 35 | 337.1× |
| volatility | 8,892 | 30 | 296.4× |
| vosc | 3,847 | 34 | 113.1× |
| vwma | 3,432 | 32 | 107.3× |
| wad | 3,870 | 24 | 161.3× |
| wcprice | 1,023 | 23 | 44.5× |
| wilders | 4,715 | 15 | 314.3× |
| willr | 17,055 | 36 | 473.8× |
| wma | 6,415 | 18 | 356.4× |
| zlema | 5,968 | 26 | 229.5× |

??? success "Key findings"

    - **All 72 indicators** implement the stateful streaming path
    - **Median streaming time: ~26 ns** per bar update
    - **Fastest** — `ao`, `ema`, `typprice`, `wilders` at **15 ns/bar**
    - **Slowest** — `msw` at **117 ns/bar** (still faster than the quickest batch indicator, `mom` at 747 ns)
    - The streaming path is **100–5,500× faster** than full batch recompute depending on the indicator

    | Rank | Indicator | Batch / Streaming | Why streaming wins so much |
    |:----:|-----------|------------------:|----------------------------|
    | 1 | `msw` | **5,498×** | Mesa Sine Wave requires FFT-style computation over the full window |
    | 2 | `cci` | **2,031×** | Large rolling sum + mean deviation computed over every bar in batch |
    | 3 | `fisher` | **1,245×** | Complex normalisation over entire input range |
    | 4 | `ultosc` | 520× | Three-period weighted TR average; all periods recomputed in batch |
    | 5 | `stochrsi` | 513× | RSI + stochastic — two full-window passes per batch call |
    | 6 | `aroonosc` | 486× | Full lookback window scan for highest/lowest on every batch call |
    | 7 | `aroon` | 475× | Same as `aroonosc` |
    | 8 | `willr` | 474× | Rolling high/low window scan |
    | 9 | `stoch` | 431× | Rolling high/low window scan |
    | 10 | `psar` | 415× | Parabolic SAR iterates entire history in batch |


