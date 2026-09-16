# Standard Performance: Rust


=== "vs C"

    Competitors: **Tulip Indicators (C)** and **TA-Lib (C)**

    | Indicator | Rust (ns) | C Tulip (ns) | TA-Lib (ns) | Speedup vs C | Speedup vs TA-Lib |
    |-----------|----------:|-------------:|------------:|-------------:|-------------------:|
    | ad | 4,799 | 5,156 | 5,135 | 1.07x | 1.07x |
    | adaptivemsw | 524,568 | N/A | N/A | --- | --- |
    | adosc | 5,657 | 9,368 | 8,873 | 1.66x | 1.57x |
    | adx | 10,729 | 16,607 | 38,508 | 1.55x | 3.59x |
    | adxr | 14,956 | 17,999 | 39,689 | 1.20x | 2.65x |
    | ao | 5,427 | 4,899 | N/A | 0.90x | --- |
    | apo | 4,872 | 9,038 | 10,772 | 1.86x | 2.21x |
    | aroon | 18,771 | 81,785 | 43,139 | 4.36x | 2.30x |
    | aroonosc | 19,553 | 81,958 | 40,258 | 4.19x | 2.06x |
    | atr | 4,825 | 11,015 | 27,256 | 2.28x | 5.65x |
    | avgprice | 1,433 | 2,046 | 2,279 | 1.43x | 1.59x |
    | bbands | 6,574 | 9,014 | 21,149 | 1.37x | 3.22x |
    | bop | 2,440 | 2,853 | 5,129 | 1.17x | 2.10x |
    | ccfisher | 227,893 | N/A | N/A | --- | --- |
    | cci | 78,669 | N/A | N/A | --- | --- |
    | chaikinmf | 7,201 | N/A | N/A | --- | --- |
    | chandelierexit | 18,411 | N/A | N/A | --- | --- |
    | cmo | 6,284 | 24,420 | N/A | 3.89x | --- |
    | cvi | 5,211 | 14,619 | N/A | 2.81x | --- |
    | cybercycle | 16,656 | N/A | N/A | --- | --- |
    | dema | 6,336 | 6,786 | 22,652 | 1.07x | 3.58x |
    | di | 17,080 | 10,544 | 57,271 | 0.62x | 3.35x |
    | dm | 9,943 | 6,998 | N/A | 0.70x | --- |
    | donchianchannel | 13,219 | 57,871 | N/A | 4.38x | --- |
    | dpo | 2,701 | 2,813 | N/A | 1.04x | --- |
    | dx | 9,426 | 7,077 | N/A | 0.75x | --- |
    | ef | 3,909 | N/A | N/A | --- | --- |
    | elderray | 6,065 | 15,432 | N/A | 2.54x | --- |
    | ema | 4,784 | 11,023 | N/A | 2.30x | --- |
    | emv | 2,449 | 5,131 | N/A | 2.10x | --- |
    | fisher | 48,681 | 134,957 | N/A | 2.77x | --- |
    | fosc | 8,082 | N/A | N/A | --- | --- |
    | highpass | 4,816 | N/A | N/A | --- | --- |
    | hilberttransform | 14,314 | N/A | 203,912 | --- | 14.25x |
    | hma | 9,349 | 9,809 | N/A | 1.05x | --- |
    | homodynediscriminator | 216,975 | N/A | 204,995 | --- | 0.94x |
    | ichimoku | 69,547 | N/A | N/A | --- | --- |
    | instantaneoustrendline | 222,460 | N/A | 245,511 | --- | 1.10x |
    | kama | 7,436 | 8,759 | 11,091 | 1.18x | 1.49x |
    | keltnerchannel | 6,863 | N/A | N/A | --- | --- |
    | kvo | 9,015 | 9,330 | N/A | 1.03x | --- |
    | linreg | 7,527 | 9,341 | N/A | 1.24x | --- |
    | macd | 6,331 | 13,977 | 35,882 | 2.21x | 5.67x |
    | mama | 219,845 | N/A | 224,906 | --- | 1.02x |
    | marketfi | 2,443 | 2,821 | N/A | 1.15x | --- |
    | mass | 6,534 | 12,280 | N/A | 1.88x | --- |
    | max | 5,203 | 30,519 | 14,631 | 5.87x | 2.81x |
    | md | 13,911 | 14,891 | N/A | 1.07x | --- |
    | medprice | 872 | 1,503 | 1,543 | 1.72x | 1.77x |
    | mfi | 7,987 | 12,335 | 18,074 | 1.54x | 2.26x |
    | min | 7,022 | 56,685 | 28,260 | 8.07x | 4.02x |
    | mom | 897 | 1,472 | 1,348 | 1.64x | 1.50x |
    | msw | 129,478 | 1,474,075 | N/A | 11.38x | --- |
    | natr | 4,954 | 10,986 | 27,257 | 2.22x | 5.50x |
    | nvi | 2,678 | 4,506 | N/A | 1.68x | --- |
    | obv | 3,513 | 3,472 | 3,614 | 0.99x | 1.03x |
    | ppo | 5,062 | 9,670 | 14,685 | 1.91x | 2.90x |
    | psar | 8,748 | 9,067 | 8,340 | 1.04x | 0.95x |
    | pvi | 2,544 | 11,431 | N/A | 4.49x | --- |
    | qstick | 2,571 | 3,045 | N/A | 1.18x | --- |
    | roc | 2,437 | 2,754 | 5,101 | 1.13x | 2.09x |
    | rocr | 2,448 | 2,761 | 5,095 | 1.13x | 2.08x |
    | roofingfilter | 10,739 | N/A | N/A | --- | --- |
    | rsi | 4,994 | 11,088 | 26,199 | 2.22x | 5.25x |
    | sma | 2,558 | 2,736 | 5,031 | 1.07x | 1.97x |
    | smaenvelope | 7,103 | N/A | N/A | --- | --- |
    | stddev | 3,795 | 7,428 | N/A | 1.96x | --- |
    | stoch | 20,383 | 94,714 | 53,785 | 4.65x | 2.64x |
    | stochrsi | 18,980 | 43,266 | N/A | 2.28x | --- |
    | supersmoother | 10,738 | N/A | N/A | --- | --- |
    | supertrend | 12,074 | N/A | N/A | --- | --- |
    | tema | 6,789 | 6,931 | 33,495 | 1.02x | 4.93x |
    | tr | 1,393 | 2,218 | 2,345 | 1.59x | 1.68x |
    | trendmode | 219,344 | N/A | N/A | --- | --- |
    | trima | 5,626 | 7,474 | 7,463 | 1.33x | 1.33x |
    | trix | 6,410 | 11,057 | N/A | 1.73x | --- |
    | trvi | 5,499 | N/A | N/A | --- | --- |
    | tsf | 6,630 | 9,342 | N/A | 1.41x | --- |
    | typprice | 1,086 | 1,854 | N/A | 1.71x | --- |
    | ultosc | 15,579 | 18,520 | N/A | 1.19x | --- |
    | vhf | 13,790 | 78,281 | N/A | 5.68x | --- |
    | vidya | 12,207 | 19,324 | N/A | 1.58x | --- |
    | volatility | 6,830 | 18,218 | N/A | 2.67x | --- |
    | vortex | 8,075 | N/A | N/A | --- | --- |
    | vosc | 3,622 | 5,148 | N/A | 1.42x | --- |
    | vwap | 3,158 | N/A | N/A | --- | --- |
    | vwma | 3,501 | 5,110 | N/A | 1.46x | --- |
    | wad | 3,970 | 5,068 | N/A | 1.28x | --- |
    | wcprice | 1,075 | 1,892 | N/A | 1.76x | --- |
    | wilders | 4,777 | 11,009 | N/A | 2.30x | --- |
    | willr | 17,282 | 86,290 | 40,705 | 4.99x | 2.36x |
    | wma | 4,943 | 8,693 | 5,117 | 1.76x | 1.04x |
    | zlema | 5,985 | 8,657 | N/A | 1.45x | --- |

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


=== "vs Rust"

    Competitors: **RustTa** (19 indicators) and **kand** v0.2 (36 indicators) — both pure-Rust, TA-Lib–inspired libraries.

    Note: kand uses NaN-padded full-length outputs (processing all n bars), while tulip_rs outputs only the valid computed bars. Both perform comparable per-call validation.

    | Indicator | Rust (ns) | RustTa (ns) | Speedup vs RustTa | Kand (ns) | Speedup vs Kand |
    |-----------|----------:|------------:|-------------------:|----------:|------------------:|
    | ad | 4,799 | N/A | --- | 5,122 | 1.07x |
    | adaptivemsw | 524,568 | N/A | --- | N/A | --- |
    | adosc | 5,657 | N/A | --- | 24,475 | 4.33x |
    | adx | 10,729 | N/A | --- | 75,368 | 7.02x |
    | adxr | 14,956 | N/A | --- | 77,179 | 5.16x |
    | ao | 5,427 | N/A | --- | N/A | --- |
    | apo | 4,872 | N/A | --- | N/A | --- |
    | aroon | 18,771 | N/A | --- | 374,882 | 19.97x |
    | aroonosc | 19,553 | N/A | --- | 376,160 | 19.24x |
    | atr | 4,825 | 9,668 | 2.00x | 22,797 | 4.72x |
    | avgprice | 1,433 | N/A | --- | N/A | --- |
    | bbands | 6,574 | 8,588 | 1.31x | 22,698 | 3.45x |
    | bop | 2,440 | N/A | --- | 2,868 | 1.18x |
    | ccfisher | 227,893 | N/A | --- | N/A | --- |
    | cci | 78,669 | N/A | --- | N/A | --- |
    | chaikinmf | 7,201 | N/A | --- | N/A | --- |
    | chandelierexit | 18,411 | 43,934 | 2.39x | N/A | --- |
    | cmo | 6,284 | N/A | --- | N/A | --- |
    | cvi | 5,211 | N/A | --- | N/A | --- |
    | cybercycle | 16,656 | N/A | --- | N/A | --- |
    | dema | 6,336 | N/A | --- | 23,734 | 3.75x |
    | di | 17,080 | N/A | --- | 24,927 | 1.46x |
    | dm | 9,943 | N/A | --- | 22,777 | 2.29x |
    | donchianchannel | 13,219 | N/A | --- | N/A | --- |
    | dpo | 2,701 | N/A | --- | N/A | --- |
    | dx | 9,426 | N/A | --- | 52,370 | 5.56x |
    | ef | 3,909 | 26,496 | 6.78x | N/A | --- |
    | elderray | 6,065 | N/A | --- | N/A | --- |
    | ema | 4,784 | 8,297 | 1.73x | 8,646 | 1.81x |
    | emv | 2,449 | N/A | --- | N/A | --- |
    | fisher | 48,681 | N/A | --- | N/A | --- |
    | fosc | 8,082 | N/A | --- | N/A | --- |
    | highpass | 4,816 | N/A | --- | N/A | --- |
    | hilberttransform | 14,314 | N/A | --- | N/A | --- |
    | hma | 9,349 | N/A | --- | N/A | --- |
    | homodynediscriminator | 216,975 | N/A | --- | N/A | --- |
    | ichimoku | 69,547 | N/A | --- | N/A | --- |
    | instantaneoustrendline | 222,460 | N/A | --- | N/A | --- |
    | kama | 7,436 | N/A | --- | N/A | --- |
    | keltnerchannel | 6,863 | 12,904 | 1.88x | N/A | --- |
    | kvo | 9,015 | N/A | --- | N/A | --- |
    | linreg | 7,527 | N/A | --- | N/A | --- |
    | macd | 6,331 | 8,312 | 1.31x | 29,485 | 4.66x |
    | mama | 219,845 | N/A | --- | N/A | --- |
    | marketfi | 2,443 | N/A | --- | N/A | --- |
    | mass | 6,534 | N/A | --- | N/A | --- |
    | max | 5,203 | 14,210 | 2.73x | N/A | --- |
    | md | 13,911 | 36,889 | 2.65x | N/A | --- |
    | medprice | 872 | N/A | --- | 1,500 | 1.72x |
    | mfi | 7,987 | 23,839 | 2.98x | 288,370 | 36.10x |
    | min | 7,022 | 25,409 | 3.62x | N/A | --- |
    | mom | 897 | N/A | --- | 1,407 | 1.57x |
    | msw | 129,478 | N/A | --- | N/A | --- |
    | natr | 4,954 | N/A | --- | 25,717 | 5.19x |
    | nvi | 2,678 | N/A | --- | N/A | --- |
    | obv | 3,513 | 5,500 | 1.57x | 3,763 | 1.07x |
    | ppo | 5,062 | 8,305 | 1.64x | N/A | --- |
    | psar | 8,748 | N/A | --- | 18,264 | 2.09x |
    | pvi | 2,544 | N/A | --- | N/A | --- |
    | qstick | 2,571 | N/A | --- | N/A | --- |
    | roc | 2,437 | 3,069 | 1.26x | 2,809 | 1.15x |
    | rocr | 2,448 | N/A | --- | 2,725 | 1.11x |
    | roofingfilter | 10,739 | N/A | --- | N/A | --- |
    | rsi | 4,994 | 8,299 | 1.66x | 24,013 | 4.81x |
    | sma | 2,558 | 4,777 | 1.87x | 5,061 | 1.98x |
    | smaenvelope | 7,103 | N/A | --- | N/A | --- |
    | stddev | 3,795 | 8,822 | 2.32x | N/A | --- |
    | stoch | 20,383 | 49,042 | 2.41x | 406,857 | 19.96x |
    | stochrsi | 18,980 | N/A | --- | N/A | --- |
    | supersmoother | 10,738 | N/A | --- | N/A | --- |
    | supertrend | 12,074 | N/A | --- | 45,031 | 3.73x |
    | tema | 6,789 | N/A | --- | 28,063 | 4.13x |
    | tr | 1,393 | 7,873 | 5.65x | 2,223 | 1.60x |
    | trendmode | 219,344 | N/A | --- | N/A | --- |
    | trima | 5,626 | N/A | --- | 10,527 | 1.87x |
    | trix | 6,410 | N/A | --- | 29,230 | 4.56x |
    | trvi | 5,499 | N/A | --- | N/A | --- |
    | tsf | 6,630 | N/A | --- | N/A | --- |
    | typprice | 1,086 | N/A | --- | 2,808 | 2.59x |
    | ultosc | 15,579 | N/A | --- | N/A | --- |
    | vhf | 13,790 | N/A | --- | N/A | --- |
    | vidya | 12,207 | N/A | --- | N/A | --- |
    | volatility | 6,830 | N/A | --- | N/A | --- |
    | vortex | 8,075 | N/A | --- | N/A | --- |
    | vosc | 3,622 | N/A | --- | N/A | --- |
    | vwap | 3,158 | N/A | --- | 8,798 | 2.79x |
    | vwma | 3,501 | N/A | --- | N/A | --- |
    | wad | 3,970 | N/A | --- | N/A | --- |
    | wcprice | 1,075 | N/A | --- | 1,715 | 1.60x |
    | wilders | 4,777 | N/A | --- | N/A | --- |
    | willr | 17,282 | N/A | --- | 359,303 | 20.79x |
    | wma | 4,943 | N/A | --- | 55,128 | 11.15x |
    | zlema | 5,985 | N/A | --- | N/A | --- |

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
