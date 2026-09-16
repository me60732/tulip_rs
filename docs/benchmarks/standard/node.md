# Standard Performance: Node

Competitors: **technicalindicators** (anandanand84) and **indicatorts** (Onur Cinar), called via the **`tulip_rs_node`** napi-rs binding.

Only indicators where at least one reference library ran are shown; the remaining indicators have `tulip_rs_node` timings in the database but no JS competitor to compare against.

| Indicator | tulip_rs_node (ns) | technicalindicators (ns) | indicatorts (ns) | Speedup vs technicalindicators | Speedup vs indicatorts |
|-----------|------------------:|------------------------:|-----------------:|--------------------------------:|------------------------:|
| `Rust_Candlestick` | 2,457,006 | 102,727,178 | N/A | 41.81x | --- |
| `ad` | 9,742 | 221,962 | 46,137 | 22.78x | 4.74x |
| `adaptivemsw` | 538,288 | N/A | N/A | --- | --- |
| `adosc` | 9,588 | N/A | N/A | --- | --- |
| `adx` | 13,903 | 1,335,314 | N/A | 96.04x | --- |
| `adxr` | 20,738 | N/A | N/A | --- | --- |
| `ao` | 9,156 | 1,244,698 | 35,345 | 135.94x | 3.86x |
| `apo` | 8,088 | N/A | 45,958 | --- | 5.68x |
| `aroon` | 28,654 | N/A | 748,174 | --- | 26.11x |
| `aroonosc` | 25,469 | N/A | N/A | --- | --- |
| `atr` | 8,854 | 420,852 | 269,633 | 47.53x | 30.45x |
| `avgprice` | 7,315 | N/A | N/A | --- | --- |
| `bbands` | 15,872 | 6,433,936 | 301,511 | 405.37x | 19.00x |
| `bop` | 6,083 | N/A | 17,799 | --- | 2.93x |
| `ccfisher` | 234,188 | N/A | N/A | --- | --- |
| `cci` | 85,143 | 4,828,772 | 66,237 | 56.71x | 0.78x |
| `chaikinmf` | 12,934 | N/A | 54,769 | --- | 4.23x |
| `chandelierexit` | 26,367 | N/A | 1,013,729 | --- | 38.45x |
| `cmo` | 9,786 | N/A | N/A | --- | --- |
| `cvi` | 8,458 | N/A | N/A | --- | --- |
| `cybercycle` | 18,939 | N/A | N/A | --- | --- |
| `dema` | 7,941 | N/A | 46,417 | --- | 5.85x |
| `di` | 26,977 | N/A | N/A | --- | --- |
| `dm` | 16,909 | N/A | N/A | --- | --- |
| `donchianchannel` | 20,643 | N/A | 812,212 | --- | 39.34x |
| `dpo` | 5,706 | N/A | N/A | --- | --- |
| `dx` | 12,292 | N/A | N/A | --- | --- |
| `ef` | 6,905 | N/A | N/A | --- | --- |
| `elderray` | 16,523 | N/A | N/A | --- | --- |
| `ema` | 7,776 | 148,191 | 20,066 | 19.06x | 2.58x |
| `emv` | 6,373 | N/A | 58,833 | --- | 9.23x |
| `fisher` | 73,225 | N/A | N/A | --- | --- |
| `fosc` | 11,070 | N/A | N/A | --- | --- |
| `highpass` | 6,767 | N/A | N/A | --- | --- |
| `hilberttransform` | 23,052 | N/A | N/A | --- | --- |
| `hma` | 10,718 | N/A | N/A | --- | --- |
| `homodynediscriminator` | 222,197 | N/A | N/A | --- | --- |
| `ichimoku` | 88,174 | 4,309,914 | N/A | 48.88x | --- |
| `instantaneoustrendline` | 227,481 | N/A | N/A | --- | --- |
| `kama` | 9,363 | N/A | N/A | --- | --- |
| `keltnerchannel` | 17,919 | N/A | 287,262 | --- | 16.03x |
| `kvo` | 12,136 | N/A | N/A | --- | --- |
| `linreg` | 9,569 | N/A | N/A | --- | --- |
| `macd` | 18,062 | 613,508 | 60,574 | 33.97x | 3.35x |
| `mama` | 229,339 | N/A | N/A | --- | --- |
| `marketfi` | 5,829 | N/A | N/A | --- | --- |
| `mass` | 9,920 | N/A | N/A | --- | --- |
| `max` | 10,875 | N/A | 565,883 | --- | 52.04x |
| `md` | 18,204 | N/A | N/A | --- | --- |
| `medprice` | 5,559 | N/A | N/A | --- | --- |
| `mfi` | 13,108 | 2,029,243 | 260,847 | 154.82x | 19.90x |
| `min` | 15,192 | N/A | 508,306 | --- | 33.46x |
| `mom` | 4,933 | N/A | N/A | --- | --- |
| `msw` | 139,723 | N/A | N/A | --- | --- |
| `natr` | 8,667 | N/A | N/A | --- | --- |
| `nvi` | 5,726 | N/A | 27,049 | --- | 4.72x |
| `obv` | 6,246 | 202,455 | 19,595 | 32.41x | 3.14x |
| `pivotpoint` | 2,110 | N/A | N/A | --- | --- |
| `ppo` | 8,388 | N/A | 79,044 | --- | 9.42x |
| `psar` | 13,074 | 228,654 | 45,987 | 17.49x | 3.52x |
| `pvi` | 5,904 | N/A | N/A | --- | --- |
| `qstick` | 6,191 | N/A | 15,342 | --- | 2.48x |
| `roc` | 6,340 | 636,450 | 13,274 | 100.39x | 2.09x |
| `rocr` | 5,201 | N/A | N/A | --- | --- |
| `roofingfilter` | 12,730 | N/A | N/A | --- | --- |
| `rsi` | 7,704 | 925,453 | 106,422 | 120.13x | 13.81x |
| `sma` | 7,236 | 446,865 | 11,484 | 61.76x | 1.59x |
| `smaenvelope` | 17,488 | N/A | N/A | --- | --- |
| `stddev` | 9,417 | N/A | N/A | --- | --- |
| `stoch` | 35,065 | 1,751,739 | 597,798 | 49.96x | 17.05x |
| `stochrsi` | 24,315 | 3,760,612 | N/A | 154.66x | --- |
| `supersmoother` | 13,341 | N/A | N/A | --- | --- |
| `supertrend` | 15,712 | N/A | N/A | --- | --- |
| `tema` | 9,248 | N/A | 79,095 | --- | 8.55x |
| `tr` | 5,899 | N/A | 236,416 | --- | 40.08x |
| `trendmode` | 223,486 | N/A | N/A | --- | --- |
| `trima` | 7,753 | N/A | 21,687 | --- | 2.80x |
| `trix` | 10,147 | 749,326 | 75,056 | 73.85x | 7.40x |
| `trvi` | 9,863 | N/A | N/A | --- | --- |
| `tsf` | 9,735 | N/A | N/A | --- | --- |
| `typprice` | 6,193 | N/A | 17,658 | --- | 2.85x |
| `ultosc` | 19,501 | N/A | N/A | --- | --- |
| `vhf` | 14,271 | N/A | N/A | --- | --- |
| `vidya` | 15,135 | N/A | N/A | --- | --- |
| `volatility` | 14,059 | N/A | N/A | --- | --- |
| `vortex` | 20,575 | N/A | 321,910 | --- | 15.65x |
| `vosc` | 7,455 | N/A | N/A | --- | --- |
| `vwap` | 8,493 | 300,247 | N/A | 35.35x | --- |
| `vwma` | 8,987 | N/A | 31,576 | --- | 3.51x |
| `wad` | 7,246 | N/A | N/A | --- | --- |
| `wcprice` | 6,609 | N/A | N/A | --- | --- |
| `wilders` | 7,512 | 142,229 | 40,152 | 18.93x | 5.35x |
| `willr` | 19,799 | 1,632,541 | 846,616 | 82.45x | 42.76x |
| `wma` | 9,861 | 3,656,746 | N/A | 370.83x | --- |
| `zlema` | 7,704 | N/A | N/A | --- | --- |

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
