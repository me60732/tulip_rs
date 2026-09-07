# SIMD by_assets: 4 Assets Simultaneously

Processes 4 different assets with the same options in a single 256-bit AVX2 `f64x4` SIMD pass. Times represent the total wall-time for all 4 assets. All times are nanoseconds (ns); lower is better.

- **Speedup vs Rust** - how many times faster SIMD is compared to 4x sequential native Rust calls
- **Speedup vs [binding/library]** - how many times faster SIMD is compared to 4x sequential calls of that competitor

=== "vs C"

    | Indicator | SIMD 4-Asset (ns) | Speedup vs Rust | Speedup vs C | Speedup vs TA-Lib |
    |-----------|------------------:|-----------------:|-------------:|------------------:|
    | ad | 16,266 | 1.18x | 1.27x | 1.26x |
    | adaptivemsw | 1,785,341 | 1.18x | --- | --- |
    | adosc | 18,352 | 1.23x | 2.04x | 1.93x |
    | adx | 21,309 | 2.01x | 3.12x | 7.23x |
    | adxr | 24,689 | 2.42x | 2.92x | 6.43x |
    | ao | 14,487 | 1.50x | 1.35x | --- |
    | apo | 10,305 | 1.89x | 3.51x | 4.18x |
    | aroon | 76,294 | 0.98x | 4.29x | 2.26x |
    | aroonosc | 77,794 | 1.00x | 4.21x | 2.07x |
    | atr | 13,289 | 1.45x | 3.32x | 8.20x |
    | avgprice | 5,677 | 1.01x | 1.44x | 1.61x |
    | bbands | 26,007 | 1.01x | 1.39x | 3.25x |
    | bop | 9,708 | 1.01x | 1.18x | 2.11x |
    | ccfisher | 275,801 | 3.31x | --- | --- |
    | cci | 72,834 | 4.34x | 4.10x | 6.16x |
    | chaikinmf | 20,817 | 1.38x | --- | --- |
    | chandelierexit | 112,590 | 0.66x | --- | --- |
    | cmo | 11,736 | 2.14x | 8.32x | --- |
    | cvi | 13,168 | 1.58x | 4.44x | --- |
    | cybercycle | 56,695 | 1.18x | --- | --- |
    | dema | 9,415 | 2.69x | 2.88x | 9.62x |
    | di | 23,032 | 2.97x | 1.83x | 9.95x |
    | dm | 18,078 | 2.20x | 1.55x | --- |
    | donchianchannel | 116,011 | 0.45x | 2.00x | --- |
    | dpo | 14,045 | 0.77x | 0.80x | --- |
    | dx | 19,111 | 1.97x | 1.48x | --- |
    | ef | 10,709 | 1.46x | --- | --- |
    | elderray | 18,261 | 1.33x | 3.38x | --- |
    | ema | 8,824 | 2.17x | 5.01x | 4.98x |
    | emv | 9,728 | 1.01x | 2.11x | --- |
    | fisher | 176,887 | 1.11x | 3.05x | --- |
    | fosc | 19,617 | 1.66x | 2.00x | --- |
    | highpass | 10,395 | 1.85x | --- | --- |
    | hilberttransform | 24,733 | 2.31x | --- | --- |
    | hma | 17,334 | 2.16x | 2.26x | --- |
    | homodynediscriminator | 298,453 | 2.91x | --- | 2.75x |
    | ichimoku | 402,544 | 0.69x | --- | --- |
    | instantaneoustrendline | 358,442 | 2.48x | --- | 2.74x |
    | kama | 12,728 | 2.34x | 2.75x | 3.49x |
    | keltnerchannel | 26,530 | 1.03x | --- | --- |
    | kvo | 26,431 | 1.36x | 1.41x | --- |
    | linreg | 10,928 | 2.42x | 3.42x | --- |
    | macd | 23,917 | 1.06x | 2.34x | 6.00x |
    | mama | 256,679 | 3.43x | --- | 3.50x |
    | marketfi | 12,414 | 0.79x | 0.91x | --- |
    | mass | 13,436 | 1.95x | 3.66x | --- |
    | max | 21,430 | 0.97x | 5.70x | 2.73x |
    | md | 31,794 | 1.80x | 1.87x | --- |
    | medprice | 3,915 | 0.89x | 1.54x | 1.58x |
    | mfi | 21,861 | 1.46x | 2.26x | 3.31x |
    | min | 33,635 | 0.82x | 6.74x | 3.36x |
    | mom | 3,415 | 1.05x | 1.72x | 1.58x |
    | msw | 370,377 | 1.40x | 15.75x | --- |
    | natr | 13,126 | 1.51x | 3.35x | 8.31x |
    | nvi | 11,288 | 0.95x | 1.60x | --- |
    | obv | 10,457 | 1.34x | 1.33x | 1.38x |
    | ppo | 10,403 | 1.95x | 3.72x | 5.65x |
    | psar | 73,128 | 0.47x | 0.50x | 0.46x |
    | pvi | 10,631 | 0.96x | 4.30x | --- |
    | qstick | 10,432 | 0.99x | 1.17x | --- |
    | roc | 10,429 | 0.93x | 1.06x | 1.96x |
    | rocr | 10,531 | 0.93x | 1.05x | 1.94x |
    | roofingfilter | 11,828 | 3.63x | --- | --- |
    | rsi | 10,442 | 1.91x | 4.25x | 10.04x |
    | sma | 9,516 | 1.08x | 1.15x | 2.11x |
    | smaenvelope | 24,671 | 1.15x | --- | --- |
    | stddev | 15,157 | 1.00x | 1.96x | --- |
    | stoch | 90,638 | 0.67x | --- | --- |
    | stochrsi | 135,799 | 0.83x | 1.39x | --- |
    | supersmoother | 12,074 | 3.40x | --- | --- |
    | supertrend | 26,401 | 2.21x | --- | --- |
    | tema | 9,141 | 2.84x | 2.99x | 14.17x |
    | tr | 6,387 | 0.94x | 1.35x | 0.61x |
    | trendmode | 264,906 | 3.38x | --- | --- |
    | trima | 12,460 | 1.80x | 2.29x | 2.28x |
    | trix | 10,210 | 2.43x | 4.32x | --- |
    | trvi | 16,770 | 1.45x | --- | --- |
    | tsf | 12,217 | 2.15x | 2.82x | --- |
    | typprice | 4,445 | 0.93x | 1.62x | --- |
    | ultosc | 31,940 | 2.10x | 2.29x | --- |
    | vhf | 82,635 | 0.65x | 1.90x | --- |
    | vidya | 38,168 | 1.27x | 0.25x | --- |
    | volatility | 24,214 | 1.50x | 1.11x | --- |
    | vortex | 23,096 | 1.36x | --- | --- |
    | vosc | 12,947 | 1.22x | 1.57x | --- |
    | vwap | 15,343 | 0.90x | --- | --- |
    | vwma | 15,796 | 0.88x | 1.29x | --- |
    | wad | 13,772 | 1.12x | 1.47x | --- |
    | wcprice | 4,387 | 0.97x | 1.63x | --- |
    | wilders | 9,060 | 2.10x | 4.84x | --- |
    | willr | 83,071 | 0.79x | 1.86x | 2.02x |
    | wma | 8,711 | 2.94x | 3.99x | 2.33x |
    | zlema | 8,938 | 2.67x | 3.84x | --- |

=== "vs Python Libraries"

    Reference libraries: **ta** (bukosabino/ta) and **pandas_ta** (twopirllc/pandas-ta),
    each called once per asset and summed to give the equivalent 4-asset sequential cost.

    | Indicator | SIMD 4-Asset (ns) | Speedup vs tulip_rs_python | Speedup vs ta | Speedup vs pandas_ta |
    |-----------|------------------:|----------------------------:|--------------:|----------------------:|
    | ad | 18,629 | 1.16x | 32.53x | --- |
    | adaptivemsw | 1,763,951 | 1.22x | --- | --- |
    | adosc | 20,422 | 1.44x | --- | --- |
    | adx | 28,972 | 4.35x | 3045.82x | 472.07x |
    | adxr | 27,711 | 2.88x | --- | --- |
    | ao | 29,303 | 1.27x | 25.99x | 24.98x |
    | apo | 13,243 | 1.77x | --- | 36.00x |
    | aroon | 106,300 | 1.14x | 383.51x | 505.00x |
    | aroonosc | 85,564 | 1.18x | --- | 634.63x |
    | atr | 17,235 | 1.66x | 2698.11x | 223.79x |
    | avgprice | 8,207 | 1.02x | --- | 25.91x |
    | bbands | 37,552 | 1.25x | 25.32x | 60.24x |
    | bop | 12,044 | 2.82x | --- | 64.52x |
    | ccfisher | 403,568 | 2.34x | --- | --- |
    | cci | 81,383 | 4.29x | 1293.88x | 1109.47x |
    | chaikinmf | 26,704 | 1.77x | 42.07x | 49.49x |
    | chandelierexit | 114,215 | 1.02x | --- | 61.72x |
    | cmo | 16,065 | 1.99x | --- | 140.92x |
    | cvi | 21,816 | 1.16x | --- | --- |
    | cybercycle | 62,945 | 1.12x | --- | --- |
    | dema | 22,327 | 1.17x | 24.12x | 51.95x |
    | di | 42,638 | 2.63x | --- | 196.86x |
    | dm | 19,735 | 3.70x | --- | 424.44x |
    | donchianchannel | 107,771 | 0.55x | 10.32x | 15.21x |
    | dpo | 18,602 | 0.84x | 28.13x | 28.24x |
    | dx | 22,535 | 2.79x | --- | 607.74x |
    | ef | 13,119 | 2.25x | --- | 49.85x |
    | elderray | 26,718 | 1.26x | --- | 45.67x |
    | ema | 15,866 | 1.68x | 14.48x | 30.00x |
    | emv | 14,628 | 2.63x | 46.95x | 96.49x |
    | fisher | 287,728 | 1.09x | --- | 112.20x |
    | fosc | 21,340 | 6.77x | --- | --- |
    | highpass | 27,399 | 1.39x | --- | --- |
    | hilberttransform | 46,338 | 1.71x | --- | --- |
    | hma | 24,792 | 1.93x | 1561.37x | 99.01x |
    | homodynediscriminator | 300,347 | 2.99x | --- | --- |
    | ichimoku | 423,649 | 0.74x | --- | 12.85x |
    | instantaneoustrendline | 362,830 | 2.48x | --- | 3.76x |
    | kama | 16,608 | 2.39x | 934.51x | 2593.68x |
    | keltnerchannel | 44,999 | 0.99x | 37.49x | 112.43x |
    | kvo | 34,442 | 1.68x | --- | 136.50x |
    | linreg | 15,171 | 2.40x | --- | 4718.38x |
    | macd | 31,843 | 1.23x | 24.54x | 102.21x |
    | mama | 262,113 | 3.45x | --- | 4.32x |
    | marketfi | 14,723 | 2.19x | --- | --- |
    | mass | 25,194 | 1.49x | 33.58x | 79.58x |
    | max | 39,518 | 0.65x | --- | --- |
    | md | 70,981 | 1.20x | --- | 1241.14x |
    | medprice | 5,825 | 1.34x | --- | 27.20x |
    | mfi | 26,143 | 1.84x | 4594.13x | 29.77x |
    | min | 43,753 | 0.87x | --- | --- |
    | mom | 5,785 | 1.20x | 15.08x | 21.59x |
    | msw | 281,148 | 1.92x | --- | --- |
    | natr | 20,539 | 4.23x | --- | 209.64x |
    | nvi | 55,028 | 0.60x | 4759.78x | 52.48x |
    | obv | 27,769 | 0.76x | 18.62x | 52.59x |
    | ppo | 28,301 | 1.08x | 33.82x | 65.57x |
    | psar | 85,935 | 1.12x | 9219.05x | 237.76x |
    | pvi | 55,016 | 0.47x | --- | 21.36x |
    | qstick | 14,210 | 0.97x | --- | --- |
    | roc | 11,632 | 1.46x | 33.96x | 12.09x |
    | rocr | 12,244 | 0.99x | --- | 10.75x |
    | roofingfilter | 15,517 | 3.29x | --- | --- |
    | rsi | 21,207 | 1.31x | 92.15x | 95.25x |
    | sma | 14,441 | 1.68x | 22.06x | 94.49x |
    | smaenvelope | 30,752 | 1.13x | --- | --- |
    | stddev | 17,644 | 1.76x | --- | 30.94x |
    | stoch | 97,448 | 1.11x | 12.24x | 41.48x |
    | stochrsi | 125,360 | 0.90x | 29.17x | 35.22x |
    | supersmoother | 17,469 | 3.15x | --- | 9.92x |
    | supertrend | 36,259 | 2.95x | --- | 5010.09x |
    | tema | 21,501 | 1.47x | 40.78x | 77.00x |
    | tr | 26,112 | 1.30x | --- | 117.60x |
    | trendmode | 253,976 | 3.54x | --- | --- |
    | trima | 20,787 | 1.28x | --- | 18.21x |
    | trix | 25,136 | 1.93x | 48.38x | 121.83x |
    | trvi | 20,049 | 1.27x | --- | --- |
    | tsf | 17,633 | 3.92x | --- | 4281.36x |
    | typprice | 6,733 | 1.03x | --- | 26.83x |
    | ultosc | 55,286 | 2.16x | 118.43x | 142.44x |
    | vhf | 104,492 | 0.82x | --- | 18.99x |
    | vidya | 52,306 | 1.26x | --- | 6343.84x |
    | volatility | 31,706 | 2.41x | --- | 17.37x |
    | vortex | 59,214 | 1.12x | 62.38x | 86.06x |
    | vosc | 15,177 | 2.00x | --- | --- |
    | vwap | 20,953 | 1.29x | --- | 250.32x |
    | vwma | 18,028 | 1.48x | --- | 39.66x |
    | wad | 22,486 | 0.79x | --- | --- |
    | wcprice | 7,770 | 1.18x | --- | 32.95x |
    | wilders | 13,889 | 1.78x | --- | 19.47x |
    | willr | 84,882 | 0.93x | 13.91x | 16.10x |
    | wma | 15,164 | 1.76x | 849.87x | 55.49x |
    | zlema | 12,869 | 2.17x | --- | 69.65x |

=== "vs Node Libraries"

    Reference libraries: **technicalindicators** (anandanand84/technicalindicators) and
    **indicatorts** (Onur Cinar/indicatorts), each called once per asset and summed to give
    the equivalent 4-asset sequential cost.

    | Indicator | SIMD 4-Asset (ns) | Speedup vs tulip_rs_node | Speedup vs technicalindicators | Speedup vs indicatorts |
    |-----------|------------------:|--------------------------:|--------------------------------:|------------------------:|
    | ad | 77,186 | 0.52x | 10.65x | 2.22x |
    | adaptivemsw | 1,981,441 | 1.09x | --- | --- |
    | adosc | 35,898 | 0.99x | --- | --- |
    | adx | 38,235 | 1.48x | 127.14x | --- |
    | adxr | 43,070 | 1.86x | --- | --- |
    | ao | 33,481 | 1.04x | 129.26x | 3.57x |
    | apo | 26,984 | 1.05x | --- | 6.34x |
    | aroon | 139,107 | 0.83x | --- | 20.88x |
    | aroonosc | 129,748 | 0.79x | --- | --- |
    | atr | 29,491 | 1.22x | 54.01x | 34.64x |
    | avgprice | 27,861 | 0.79x | --- | --- |
    | bbands | 113,593 | 0.51x | 208.36x | 10.38x |
    | bop | 28,146 | 0.87x | --- | 2.49x |
    | ccfisher | 417,852 | 2.27x | --- | --- |
    | cci | 114,773 | 3.10x | 168.86x | 2.36x |
    | chaikinmf | 37,514 | 1.26x | --- | 5.77x |
    | chandelierexit | 133,830 | 0.77x | --- | 30.20x |
    | cmo | 26,422 | 1.33x | --- | --- |
    | cvi | 32,008 | 1.04x | --- | --- |
    | cybercycle | 78,352 | 0.97x | --- | --- |
    | dema | 27,810 | 1.14x | --- | 6.78x |
    | di | 86,398 | 1.25x | --- | --- |
    | dm | 43,676 | 1.52x | --- | --- |
    | donchianchannel | 132,891 | 0.63x | --- | 24.42x |
    | dpo | 23,861 | 0.94x | --- | --- |
    | dx | 38,929 | 1.22x | --- | --- |
    | ef | 27,216 | 1.02x | --- | --- |
    | elderray | 42,465 | 1.61x | --- | --- |
    | ema | 27,230 | 1.02x | 21.82x | 2.99x |
    | emv | 28,887 | 0.82x | --- | 7.81x |
    | fisher | 285,810 | 1.04x | --- | --- |
    | fosc | 34,514 | 4.49x | --- | --- |
    | highpass | 27,996 | 0.96x | --- | --- |
    | hilberttransform | 45,394 | 1.92x | --- | --- |
    | hma | 38,227 | 1.15x | --- | --- |
    | homodynediscriminator | 316,316 | 2.79x | --- | --- |
    | ichimoku | 482,907 | 0.72x | 34.48x | --- |
    | instantaneoustrendline | 378,682 | 2.38x | --- | --- |
    | kama | 31,292 | 1.21x | --- | --- |
    | keltnerchannel | 101,641 | 0.68x | --- | 11.14x |
    | kvo | 41,164 | 1.15x | --- | --- |
    | linreg | 24,229 | 1.56x | --- | --- |
    | macd | 70,494 | 0.98x | 35.24x | 3.46x |
    | mama | 275,836 | 3.30x | --- | --- |
    | marketfi | 29,367 | 0.92x | --- | --- |
    | mass | 30,942 | 1.14x | --- | --- |
    | max | 42,696 | 1.06x | --- | 52.59x |
    | md | 58,967 | 1.24x | --- | --- |
    | medprice | 25,629 | 0.86x | --- | --- |
    | mfi | 39,395 | 1.28x | 196.22x | 26.77x |
    | min | 59,463 | 0.95x | --- | 33.87x |
    | mom | 23,799 | 0.87x | --- | --- |
    | msw | 328,544 | 1.68x | --- | --- |
    | natr | 31,579 | 1.08x | --- | --- |
    | nvi | 29,180 | 0.77x | --- | 3.69x |
    | obv | 29,380 | 0.93x | 25.78x | 2.68x |
    | ppo | 25,580 | 1.25x | --- | 12.50x |
    | psar | 80,002 | 0.64x | 11.27x | 2.30x |
    | pvi | 23,663 | 1.00x | --- | --- |
    | qstick | 26,986 | 0.91x | --- | 2.50x |
    | roc | 26,309 | 0.96x | 89.86x | 2.14x |
    | rocr | 27,230 | 0.76x | --- | --- |
    | roofingfilter | 28,449 | 1.82x | --- | --- |
    | rsi | 23,133 | 1.33x | 159.05x | 18.79x |
    | sma | 22,399 | 1.10x | 78.29x | 2.40x |
    | smaenvelope | 57,716 | 1.06x | --- | --- |
    | stddev | 26,539 | 1.39x | --- | --- |
    | stoch | 176,729 | 0.78x | 40.57x | 13.41x |
    | stochrsi | 155,750 | 0.60x | 97.58x | --- |
    | supersmoother | 26,228 | 1.94x | --- | --- |
    | supertrend | 41,416 | 1.53x | --- | --- |
    | tema | 31,616 | 1.18x | --- | 9.85x |
    | tr | 28,008 | 1.01x | --- | 35.47x |
    | trendmode | 266,240 | 3.33x | --- | --- |
    | trima | 29,872 | 1.11x | --- | 3.05x |
    | trix | 33,994 | 1.12x | 82.92x | 8.92x |
    | trvi | 35,686 | 1.00x | --- | --- |
    | tsf | 29,512 | 1.24x | --- | --- |
    | typprice | 26,605 | 0.89x | --- | 2.66x |
    | ultosc | 45,850 | 1.52x | --- | --- |
    | vhf | 137,849 | 0.48x | --- | --- |
    | vidya | 53,879 | 1.10x | --- | --- |
    | volatility | 36,445 | 1.53x | --- | --- |
    | vortex | 67,549 | 1.23x | --- | 18.13x |
    | vosc | 26,491 | 1.11x | --- | --- |
    | vwap | 31,972 | 1.25x | 31.64x | --- |
    | vwma | 28,041 | 1.10x | --- | 4.04x |
    | wad | 27,256 | 1.02x | --- | --- |
    | wcprice | 26,054 | 0.84x | --- | --- |
    | wilders | 22,078 | 1.35x | 21.82x | 7.25x |
    | willr | 114,832 | 0.67x | 50.56x | 28.72x |
    | wma | 23,785 | 1.61x | 614.37x | --- |
    | zlema | 21,820 | 1.45x | --- | --- |

??? success "Notable results - by_assets"

    **69 of 93 indicators (74%) show a SIMD speedup over 4x sequential Rust.**
    Median speedup for benefiting indicators: **~1.80x**.

    | Category | Indicator | SIMD Speedup vs 4x Sequential Rust |
    |----------|-----------|:-----------------------------------:|
    | **Top performers** | `cci` | **4.34x** |
    | **Top performers** | `roofingfilter` | **3.63x** |
    | **Top performers** | `mama` | **3.43x** |
    | **Top performers** | `supersmoother` | **3.40x** |
    | **Top performers** | `trendmode` | **3.38x** |
    | **Top performers** | `ccfisher` | **3.31x** |
    | **Top performers** | `di` | **2.97x** |
    | **Top performers** | `wma` | **2.94x** |
    | **Top performers** | `homodynediscriminator` | **2.91x** |
    | **Top performers** | `tema` | **2.84x** |
    | **Notable improvement** | `msw` | 1.40x (SDFT optimisation) |
    | **SIMD slower than sequential** | `donchianchannel` | 0.45x |
    | **SIMD slower than sequential** | `psar` | 0.47x |
    | **SIMD slower than sequential** | `vhf` | 0.65x |
    | **SIMD slower than sequential** | `chandelierexit` | 0.66x |
    | **SIMD slower than sequential** | `stoch` | 0.67x |

    Indicators where SIMD is slower typically involve highly sequential computation or irregular memory access patterns where SIMD setup overhead dominates.
