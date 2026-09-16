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
    | cci | 71,967 | 3.30x | --- | --- |
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
    | ema | 9,497 | 2.02x | 4.64x | --- |
    | emv | 9,728 | 1.01x | 2.11x | --- |
    | fisher | 178,104 | 1.09x | 3.03x | --- |
    | fosc | 11,510 | 0.81x | --- | --- |
    | highpass | 10,395 | 1.85x | --- | --- |
    | hilberttransform | 24,733 | 2.31x | --- | --- |
    | hma | 17,334 | 2.16x | 2.26x | --- |
    | homodynediscriminator | 298,453 | 2.91x | --- | 2.75x |
    | ichimoku | 402,544 | 0.69x | --- | --- |
    | instantaneoustrendline | 358,442 | 2.48x | --- | 2.74x |
    | kama | 12,728 | 2.34x | 2.75x | 3.49x |
    | keltnerchannel | 26,530 | 1.03x | --- | --- |
    | kvo | 26,431 | 1.36x | 1.41x | --- |
    | linreg | 9,885 | 3.05x | 2.13x | --- |
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
    | msw | 298,731 | 1.73x | 4.93x | --- |
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
    | stoch | 90,638 | 0.90x | 4.18x | 2.37x |
    | stochrsi | 134,154 | 0.57x | 1.29x | --- |
    | supersmoother | 12,154 | 3.53x | --- | --- |
    | supertrend | 25,150 | 1.92x | --- | --- |
    | tema | 10,444 | 2.60x | 2.65x | 12.83x |
    | tr | 5,595 | 1.00x | 1.59x | 1.68x |
    | trendmode | 251,480 | 3.49x | --- | --- |
    | trima | 13,096 | 1.72x | 2.28x | 2.28x |
    | trix | 10,496 | 2.44x | 4.21x | --- |
    | trvi | 17,567 | 1.25x | --- | --- |
    | tsf | 10,981 | 2.41x | 3.40x | --- |
    | typprice | 4,619 | 0.94x | 1.61x | --- |
    | ultosc | 32,730 | 1.90x | 2.26x | --- |
    | vhf | 77,292 | 0.70x | 4.05x | --- |
    | vidya | 38,448 | 1.27x | 2.01x | --- |
    | volatility | 24,966 | 1.09x | 2.92x | --- |
    | vortex | 25,523 | 1.26x | --- | --- |
    | vosc | 14,743 | 0.98x | 1.40x | --- |
    | vwap | 16,274 | 0.78x | --- | --- |
    | vwma | 16,507 | 0.85x | 1.24x | --- |
    | wad | 13,998 | 1.13x | 1.45x | --- |
    | wcprice | 4,451 | 0.97x | 1.70x | --- |
    | wilders | 9,255 | 2.06x | 4.76x | --- |
    | willr | 80,508 | 0.86x | 4.29x | 2.02x |
    | wma | 9,213 | 2.15x | 3.77x | 2.22x |
    | zlema | 9,193 | 2.60x | 3.77x | --- |

=== "vs Python Libraries"

    Reference libraries: **ta** (bukosabino/ta) and **pandas_ta** (twopirllc/pandas-ta),
    each called once per asset and summed to give the equivalent 4-asset sequential cost.

    | Indicator | SIMD 4-Asset (ns) | Speedup vs tulip_rs_python | Speedup vs ta | Speedup vs pandas_ta |
    |-----------|------------------:|----------------------------:|--------------:|----------------------:|
    | ad | 21,349 | 1.04x | 29.23x | --- |
    | adaptivemsw | 1,749,595 | 1.22x | --- | --- |
    | adosc | 26,119 | 1.03x | --- | --- |
    | adx | 38,458 | 2.44x | 2247.31x | 356.58x |
    | adxr | 33,943 | 2.68x | --- | --- |
    | ao | 19,973 | 1.49x | 38.13x | 43.36x |
    | apo | 15,504 | 1.74x | --- | 36.28x |
    | aroon | 110,379 | 1.26x | 376.00x | 459.82x |
    | aroonosc | 84,327 | 1.57x | --- | 604.36x |
    | atr | 18,725 | 2.79x | 2435.18x | 203.32x |
    | avgprice | 10,566 | 1.00x | --- | 20.28x |
    | bbands | 42,803 | 1.34x | 22.33x | 52.39x |
    | bop | 12,027 | 1.91x | --- | 62.79x |
    | ccfisher | 407,977 | 2.26x | --- | --- |
    | cci | 74,580 | 4.99x | 1384.85x | 1177.74x |
    | chaikinmf | 24,363 | 1.69x | 45.78x | 54.30x |
    | chandelierexit | 116,929 | 1.12x | --- | 59.60x |
    | cmo | 17,001 | 1.92x | --- | 130.66x |
    | cvi | 26,142 | 1.09x | --- | --- |
    | cybercycle | 62,003 | 1.11x | --- | --- |
    | dema | 21,694 | 1.42x | 24.71x | 52.95x |
    | di | 42,898 | 3.98x | --- | 196.12x |
    | dm | 42,069 | 2.29x | --- | 200.64x |
    | donchianchannel | 102,738 | 0.65x | 10.63x | 15.85x |
    | dpo | 15,876 | 0.79x | 31.21x | 36.65x |
    | dx | 31,029 | 3.57x | --- | 439.51x |
    | ef | 29,926 | 0.84x | --- | 20.62x |
    | elderray | 20,662 | 1.66x | --- | 58.41x |
    | ema | 16,430 | 1.47x | 14.01x | 28.95x |
    | emv | 12,270 | 2.55x | 57.94x | 108.33x |
    | fisher | 289,797 | 1.13x | --- | 108.48x |
    | fosc | 14,902 | 2.33x | --- | --- |
    | highpass | 15,866 | 1.65x | --- | --- |
    | hilberttransform | 31,188 | 2.45x | --- | --- |
    | hma | 27,371 | 2.93x | 1492.80x | 89.54x |
    | homodynediscriminator | 324,236 | 2.74x | --- | --- |
    | ichimoku | 425,758 | 0.89x | --- | 12.90x |
    | instantaneoustrendline | 386,210 | 2.31x | --- | 3.66x |
    | kama | 23,354 | 3.18x | 661.37x | 1787.83x |
    | keltnerchannel | 39,856 | 2.09x | 42.12x | 127.74x |
    | kvo | 38,239 | 2.04x | --- | 124.48x |
    | linreg | 28,105 | 3.30x | --- | 2459.66x |
    | macd | 49,404 | 1.89x | 15.98x | 65.38x |
    | mama | 260,644 | 3.44x | --- | 4.18x |
    | marketfi | 23,926 | 0.63x | --- | --- |
    | mass | 28,446 | 1.08x | 29.62x | 68.77x |
    | max | 47,572 | 0.71x | --- | --- |
    | md | 46,579 | 3.16x | --- | 1864.92x |
    | medprice | 5,600 | 1.03x | --- | 23.20x |
    | mfi | 27,176 | 1.54x | 4376.75x | 28.98x |
    | min | 46,022 | 1.00x | --- | --- |
    | mom | 7,756 | 0.69x | 10.46x | 14.19x |
    | msw | 177,770 | 2.99x | --- | --- |
    | natr | 31,461 | 2.10x | --- | 136.55x |
    | nvi | 12,087 | 3.09x | 21824.36x | 240.90x |
    | obv | 26,178 | 0.65x | 17.90x | 55.59x |
    | ppo | 17,906 | 2.01x | 51.56x | 106.63x |
    | psar | 77,770 | 1.28x | 10234.82x | 254.21x |
    | pvi | 15,538 | 1.41x | --- | 73.95x |
    | qstick | 28,451 | 0.69x | --- | --- |
    | roc | 13,958 | 1.85x | 30.31x | 9.66x |
    | rocr | 14,333 | 2.47x | --- | 12.84x |
    | roofingfilter | 22,836 | 2.58x | --- | --- |
    | rsi | 15,786 | 1.88x | 121.05x | 127.10x |
    | sma | 32,027 | 0.53x | 9.26x | 45.54x |
    | smaenvelope | 47,609 | 0.87x | --- | --- |
    | stddev | 16,741 | 1.93x | --- | 33.52x |
    | stoch | 100,000 | 1.24x | 11.98x | 40.18x |
    | stochrsi | 126,157 | 1.25x | 28.82x | 34.79x |
    | supersmoother | 15,280 | 2.98x | --- | 10.70x |
    | supertrend | 36,713 | 3.27x | --- | 4811.28x |
    | tema | 17,047 | 2.12x | 50.49x | 97.97x |
    | tr | 34,219 | 0.96x | --- | 90.36x |
    | trendmode | 253,897 | 3.47x | --- | --- |
    | trima | 16,847 | 1.73x | --- | 25.99x |
    | trix | 23,418 | 2.81x | 52.42x | 130.73x |
    | trvi | 21,071 | 1.23x | --- | --- |
    | tsf | 14,372 | 4.70x | --- | 4857.15x |
    | typprice | 6,864 | 1.00x | --- | 26.40x |
    | ultosc | 33,208 | 2.59x | 196.07x | 234.50x |
    | vhf | 99,818 | 0.64x | --- | 19.72x |
    | vidya | 45,142 | 1.80x | --- | 7326.49x |
    | volatility | 32,777 | 1.65x | --- | 17.41x |
    | vortex | 55,011 | 1.46x | 67.00x | 92.30x |
    | vosc | 17,941 | 1.35x | --- | --- |
    | vwap | 32,648 | 1.03x | --- | 165.89x |
    | vwma | 32,955 | 0.89x | --- | 24.60x |
    | wad | 18,097 | 1.37x | --- | --- |
    | wcprice | 6,712 | 1.42x | --- | 29.19x |
    | wilders | 14,007 | 1.67x | --- | 18.84x |
    | willr | 89,824 | 0.92x | 13.31x | 15.12x |
    | wma | 14,309 | 1.70x | 943.03x | 59.26x |
    | zlema | 13,660 | 2.54x | --- | 66.54x |

=== "vs Node Libraries"

    Reference libraries: **technicalindicators** (anandanand84/technicalindicators) and
    **indicatorts** (Onur Cinar/indicatorts), each called once per asset and summed to give
    the equivalent 4-asset sequential cost.

    | Indicator | SIMD 4-Asset (ns) | Speedup vs tulip_rs_node | Speedup vs technicalindicators | Speedup vs indicatorts |
    |-----------|------------------:|--------------------------:|--------------------------------:|------------------------:|
    | ad | 78,544 | 0.50x | 11.30x | 2.35x |
    | adaptivemsw | 1,982,918 | 1.09x | --- | --- |
    | adosc | 38,378 | 1.00x | --- | --- |
    | adx | 41,344 | 1.34x | 129.19x | --- |
    | adxr | 46,119 | 1.80x | --- | --- |
    | ao | 45,479 | 0.81x | 109.47x | 3.11x |
    | apo | 30,510 | 1.05x | --- | 6.03x |
    | aroon | 142,786 | 0.81x | --- | 20.96x |
    | aroonosc | 131,531 | 0.78x | --- | --- |
    | atr | 29,751 | 1.18x | 56.58x | 36.25x |
    | avgprice | 27,057 | 1.08x | --- | --- |
    | bbands | 109,653 | 0.59x | 234.70x | 11.00x |
    | bop | 26,270 | 0.93x | --- | 2.71x |
    | ccfisher | 415,190 | 2.27x | --- | --- |
    | cci | 112,221 | 3.19x | 172.12x | 2.36x |
    | chaikinmf | 37,410 | 1.38x | --- | 5.86x |
    | chandelierexit | 135,786 | 0.78x | --- | 29.86x |
    | cmo | 27,706 | 1.40x | --- | --- |
    | cvi | 30,955 | 1.09x | --- | --- |
    | cybercycle | 79,091 | 0.96x | --- | --- |
    | dema | 24,464 | 1.30x | --- | 7.59x |
    | di | 73,882 | 1.47x | --- | --- |
    | dm | 49,712 | 1.36x | --- | --- |
    | donchianchannel | 131,020 | 0.63x | --- | 24.80x |
    | dpo | 23,790 | 0.96x | --- | --- |
    | dx | 36,151 | 1.36x | --- | --- |
    | ef | 27,465 | 1.01x | --- | --- |
    | elderray | 56,110 | 1.18x | --- | --- |
    | ema | 26,911 | 1.13x | 22.03x | 2.98x |
    | emv | 30,095 | 0.85x | --- | 7.82x |
    | fisher | 290,518 | 1.02x | --- | --- |
    | fosc | 31,858 | 1.39x | --- | --- |
    | highpass | 25,192 | 1.07x | --- | --- |
    | hilberttransform | 57,088 | 1.61x | --- | --- |
    | hma | 38,158 | 1.12x | --- | --- |
    | homodynediscriminator | 317,146 | 2.80x | --- | --- |
    | ichimoku | 490,250 | 0.72x | 35.17x | --- |
    | instantaneoustrendline | 380,345 | 2.39x | --- | --- |
    | kama | 25,756 | 1.45x | --- | --- |
    | keltnerchannel | 77,272 | 0.93x | --- | 14.87x |
    | kvo | 43,757 | 1.11x | --- | --- |
    | linreg | 27,662 | 1.38x | --- | --- |
    | macd | 97,883 | 0.73x | 25.07x | 2.48x |
    | mama | 277,456 | 3.31x | --- | --- |
    | marketfi | 25,064 | 0.93x | --- | --- |
    | mass | 30,183 | 1.30x | --- | --- |
    | max | 36,312 | 1.19x | --- | 62.34x |
    | md | 56,809 | 1.29x | --- | --- |
    | medprice | 24,571 | 0.90x | --- | --- |
    | mfi | 37,805 | 1.37x | 214.71x | 27.60x |
    | min | 60,540 | 1.00x | --- | 33.58x |
    | mom | 23,222 | 0.85x | --- | --- |
    | msw | 236,446 | 2.36x | --- | --- |
    | natr | 32,397 | 1.07x | --- | --- |
    | nvi | 30,793 | 0.74x | --- | 3.51x |
    | obv | 32,409 | 0.77x | 24.99x | 2.42x |
    | ppo | 30,262 | 1.11x | --- | 10.45x |
    | psar | 82,427 | 0.63x | 11.10x | 2.23x |
    | pvi | 23,228 | 1.02x | --- | --- |
    | qstick | 26,794 | 0.92x | --- | 2.29x |
    | roc | 28,640 | 0.88x | 88.89x | 1.85x |
    | rocr | 26,438 | 0.79x | --- | --- |
    | roofingfilter | 31,278 | 1.63x | --- | --- |
    | rsi | 28,654 | 1.07x | 129.19x | 14.86x |
    | sma | 29,364 | 0.95x | 60.87x | 1.56x |
    | smaenvelope | 59,080 | 1.17x | --- | --- |
    | stddev | 29,077 | 1.29x | --- | --- |
    | stoch | 172,652 | 0.81x | 40.58x | 13.85x |
    | stochrsi | 156,845 | 0.62x | 95.91x | --- |
    | supersmoother | 25,048 | 2.11x | --- | --- |
    | supertrend | 42,568 | 1.47x | --- | --- |
    | tema | 32,229 | 1.15x | --- | 9.82x |
    | tr | 31,474 | 0.75x | --- | 30.05x |
    | trendmode | 271,804 | 3.29x | --- | --- |
    | trima | 27,407 | 1.13x | --- | 3.17x |
    | trix | 32,872 | 1.23x | 91.18x | 9.13x |
    | trvi | 36,471 | 1.07x | --- | --- |
    | tsf | 27,003 | 1.44x | --- | --- |
    | typprice | 26,669 | 0.93x | --- | 2.65x |
    | ultosc | 46,316 | 1.67x | --- | --- |
    | vhf | 141,932 | 0.40x | --- | --- |
    | vidya | 53,904 | 1.12x | --- | --- |
    | volatility | 49,030 | 1.14x | --- | --- |
    | vortex | 69,288 | 1.19x | --- | 18.58x |
    | vosc | 27,709 | 1.08x | --- | --- |
    | vwap | 30,431 | 1.12x | 39.47x | --- |
    | vwma | 28,617 | 1.25x | --- | 4.41x |
    | wad | 30,349 | 0.96x | --- | --- |
    | wcprice | 26,767 | 0.99x | --- | --- |
    | wilders | 26,883 | 1.12x | 21.16x | 5.97x |
    | willr | 118,300 | 0.67x | 55.20x | 28.63x |
    | wma | 25,913 | 1.50x | 564.47x | --- |
    | zlema | 21,586 | 1.43x | --- | --- |

=== "C Binding"

    Competitors: **C_tulip** (Tulip Indicators) and **TA-Lib**, each called once per asset and
    summed to give the equivalent 4-asset sequential cost; the `tulip_rs_ffi_c` column folds in
    the same binding's own 4 sequential calls as the SIMD speedup baseline.

    | Indicator | SIMD 4-Asset (ns) | Speedup vs tulip_rs_ffi_c | Speedup vs C | Speedup vs TA-Lib |
    |-----------|------------------:|--------------------------:|-------------:|------------------:|
    | ad | 106,851 | 4.51x | 4.22x | 4.30x |
    | adaptivemsw | 1,788,600 | 7.99x | --- | --- |
    | adosc | 19,430 | 1.38x | 1.91x | 1.82x |
    | adx | 23,138 | 5.41x | 4.70x | 6.77x |
    | adxr | 30,618 | 4.51x | 4.01x | 5.29x |
    | ao | 17,631 | 5.39x | 4.15x | --- |
    | apo | 10,986 | 1.81x | 3.15x | 3.85x |
    | aroon | 116,153 | 1.15x | 3.19x | 1.59x |
    | aroonosc | 115,410 | 1.26x | 3.12x | 1.50x |
    | atr | 13,738 | 1.62x | 3.10x | 7.80x |
    | avgprice | 14,078 | 2.92x | 1.75x | 2.09x |
    | bbands | 25,355 | 1.21x | 1.13x | 3.08x |
    | bop | 10,503 | 4.02x | 3.67x | 7.25x |
    | ccfisher | 307,337 | 4.44x | --- | --- |
    | cci | 74,065 | 4.25x | 3.99x | 6.04x |
    | chaikinmf | 21,881 | 1.77x | --- | --- |
    | chandelierexit | 146,756 | 1.02x | --- | --- |
    | cmo | 12,465 | 2.22x | 10.97x | 8.36x |
    | cvi | 12,797 | 1.55x | 4.43x | --- |
    | cybercycle | 56,203 | 4.80x | --- | --- |
    | dema | 9,744 | 2.41x | 2.64x | 9.31x |
    | di | 23,563 | 5.13x | 4.15x | 9.63x |
    | dm | 19,580 | 5.43x | 4.10x | --- |
    | donchianchannel | 176,426 | 0.87x | 1.52x | --- |
    | dpo | 13,058 | 0.84x | 0.74x | --- |
    | dx | 25,302 | 5.51x | 3.86x | 6.14x |
    | ef | 11,726 | 1.66x | --- | --- |
    | elderray | 19,176 | 1.29x | 2.22x | --- |
    | ema | 9,514 | 2.05x | 4.49x | 4.47x |
    | emv | 10,317 | 1.28x | 1.85x | --- |
    | fisher | 297,228 | 1.05x | 1.15x | --- |
    | fosc | 12,250 | 2.74x | 3.09x | --- |
    | highpass | 10,277 | 1.89x | --- | --- |
    | hma | 18,017 | 1.97x | 2.09x | --- |
    | ichimoku | 524,057 | 0.82x | --- | --- |
    | kama | 13,488 | 2.11x | 2.49x | 3.21x |
    | keltnerchannel | 48,925 | 0.56x | --- | --- |
    | kvo | 27,637 | 2.35x | 2.35x | --- |
    | linreg | 11,395 | 2.57x | 3.16x | 20.48x |
    | macd | 46,627 | 0.54x | 1.07x | 2.94x |
    | mama | 259,587 | 3.42x | --- | 3.74x |
    | marketfi | 14,142 | 3.53x | 2.72x | --- |
    | mass | 13,169 | 1.94x | 3.63x | --- |
    | max | 62,967 | 1.06x | 1.87x | 1.19x |
    | md | 46,174 | 1.26x | 1.38x | --- |
    | medprice | 5,039 | 0.98x | 0.91x | 0.76x |
    | mfi | 23,827 | 1.51x | 5.36x | 4.14x |
    | min | 74,682 | 0.98x | 2.01x | 1.20x |
    | mom | 3,922 | 0.99x | 0.92x | 0.82x |
    | msw | 173,335 | 3.38x | 15.41x | --- |
    | natr | 13,873 | 1.71x | 3.07x | 7.71x |
    | nvi | 13,733 | 3.75x | 4.40x | --- |
    | obv | 11,156 | 5.69x | 6.66x | 6.50x |
    | pivotpoint | 855 | 0.68x | --- | --- |
    | ppo | 11,614 | 1.93x | 3.21x | 4.90x |
    | psar | 81,569 | 1.20x | 1.52x | 1.45x |
    | pvi | 12,879 | 4.00x | 4.71x | --- |
    | qstick | 11,060 | 0.97x | 0.99x | --- |
    | roc | 10,780 | 0.92x | 0.88x | 1.76x |
    | rocr | 10,555 | 0.94x | 0.90x | 1.82x |
    | roofingfilter | 12,458 | 3.47x | --- | --- |
    | rsi | 10,935 | 1.99x | 6.77x | 9.48x |
    | sma | 9,783 | 1.06x | 0.98x | 1.93x |
    | smaenvelope | 27,824 | 0.98x | --- | --- |
    | stddev | 15,652 | 1.85x | 1.83x | 4.22x |
    | stoch | 146,652 | 0.98x | 2.64x | 1.69x |
    | stochrsi | 170,414 | 1.21x | 1.40x | 1.59x |
    | supersmoother | 13,520 | 3.20x | --- | --- |
    | supertrend | 25,996 | 2.33x | --- | --- |
    | tema | 9,970 | 2.77x | 2.67x | 13.28x |
    | tr | 6,790 | 1.12x | 1.01x | 1.23x |
    | trendmode | 256,121 | 5.19x | --- | --- |
    | trima | 14,156 | 1.56x | 2.01x | 2.01x |
    | trix | 11,118 | 2.45x | 3.84x | 13.13x |
    | trvi | 16,764 | 1.44x | --- | --- |
    | tsf | 11,124 | 2.52x | 3.28x | 20.72x |
    | typprice | 5,045 | 1.09x | 1.03x | 1.92x |
    | ultosc | 36,667 | 1.70x | 1.69x | 5.07x |
    | vhf | 133,674 | 1.02x | 1.96x | --- |
    | vidya | 39,953 | 1.23x | 1.89x | --- |
    | volatility | 25,984 | 1.88x | 2.75x | --- |
    | vortex | 26,144 | 1.18x | --- | --- |
    | vosc | 13,984 | 1.46x | 1.39x | --- |
    | vwap | 18,104 | 4.33x | --- | --- |
    | vwma | 16,712 | 1.18x | 1.14x | --- |
    | wad | 16,314 | 14.68x | 16.62x | --- |
    | wcprice | 5,571 | 3.41x | 3.75x | --- |
    | wilders | 10,034 | 1.98x | 4.28x | --- |
    | willr | 113,952 | 1.10x | 3.18x | 1.83x |
    | wma | 10,777 | 1.81x | 3.09x | 1.80x |
    | zlema | 9,754 | 2.47x | 3.39x | --- |

=== "Go Binding"

    Reference: **cinar/indicator/v2** (pure Go), called once per asset and summed to give the
    equivalent 4-asset sequential cost; the `tulip_rs_go` column folds in the binding's own
    4 sequential calls as the SIMD speedup baseline. cinar is only compared where param-compatible.

    | Indicator | SIMD 4-Asset (ns) | Speedup vs tulip_rs_go | Speedup vs cinar |
    |-----------|------------------:|-----------------------:|-----------------:|
    | ad | 19,451 | 2.19x | 2005.10x |
    | adaptivemsw | 1,783,527 | 1.20x | --- |
    | adosc | 26,872 | 1.28x | 2261.69x |
    | adx | 36,639 | 2.03x | --- |
    | adxr | 38,204 | 1.77x | --- |
    | ao | 19,704 | 1.36x | 3476.54x |
    | apo | 27,803 | 1.21x | 499.68x |
    | aroon | 136,184 | 0.97x | 408.60x |
    | aroonosc | 93,395 | 0.92x | --- |
    | atr | 23,594 | 1.47x | 2146.77x |
    | avgprice | 21,142 | 0.39x | --- |
    | bbands | 95,941 | 0.58x | 785.25x |
    | bop | 75,831 | 0.43x | 487.75x |
    | ccfisher | 323,048 | 2.90x | --- |
    | cci | 94,578 | 4.14x | 1324.78x |
    | chaikinmf | 54,101 | 1.59x | 1597.77x |
    | chandelierexit | 138,245 | 1.43x | 684.27x |
    | cmo | 26,054 | 2.05x | --- |
    | cvi | 31,549 | 1.03x | --- |
    | cybercycle | 77,454 | 1.12x | --- |
    | dema | 24,432 | 2.62x | 1816.85x |
    | di | 68,756 | 1.16x | --- |
    | dm | 62,529 | 0.87x | --- |
    | donchianchannel | 165,341 | 0.89x | 431.67x |
    | dpo | 25,934 | 0.62x | 1948.95x |
    | dx | 40,751 | 1.08x | --- |
    | ef | 27,149 | 1.23x | --- |
    | elderray | 69,274 | 0.50x | 629.66x |
    | ema | 25,351 | 1.61x | 397.95x |
    | emv | 27,928 | 1.33x | 3132.54x |
    | fisher | 319,302 | 0.89x | 306.37x |
    | fosc | 21,476 | 1.86x | --- |
    | highpass | 11,385 | 1.91x | --- |
    | hilberttransform | 70,559 | 0.95x | --- |
    | hma | 40,408 | 1.31x | 1416.47x |
    | homodynediscriminator | 326,869 | 2.82x | --- |
    | ichimoku | 573,268 | 0.66x | --- |
    | instantaneoustrendline | 376,069 | 2.38x | --- |
    | kama | 27,052 | 2.12x | 3683.13x |
    | keltnerchannel | 100,680 | 0.41x | --- |
    | kvo | 38,679 | 1.28x | --- |
    | linreg | 20,936 | 1.56x | --- |
    | macd | 99,730 | 0.41x | --- |
    | mama | 305,721 | 2.98x | --- |
    | marketfi | 32,952 | 0.48x | --- |
    | mass | 24,955 | 1.17x | --- |
    | max | 58,458 | 0.57x | 420.26x |
    | md | 37,722 | 1.77x | --- |
    | medprice | 5,815 | 1.50x | --- |
    | mfi | 45,012 | 1.51x | 2169.70x |
    | min | 45,069 | 1.12x | 811.18x |
    | mom | 5,669 | 1.62x | --- |
    | msw | 237,598 | 2.33x | --- |
    | natr | 37,972 | 1.02x | --- |
    | nvi | 12,292 | 2.85x | 6315.80x |
    | obv | 48,892 | 0.72x | 379.44x |
    | pivotpoint | 2,869 | 1.60x | --- |
    | ppo | 15,345 | 4.38x | 5391.91x |
    | psar | 62,821 | 0.76x | --- |
    | pvi | 14,044 | 1.04x | --- |
    | qstick | 16,025 | 2.64x | 3190.39x |
    | roc | 19,650 | 0.89x | 600.54x |
    | rocr | 18,394 | 0.68x | --- |
    | roofingfilter | 17,424 | 2.85x | --- |
    | rsi | 21,223 | 2.22x | 2944.84x |
    | sma | 14,354 | 1.61x | 2078.18x |
    | smaenvelope | 105,527 | 0.36x | 473.44x |
    | stddev | 18,007 | 3.36x | 1168.70x |
    | stoch | 102,611 | 1.32x | 731.82x |
    | stochrsi | 147,393 | 0.58x | --- |
    | supersmoother | 17,254 | 3.74x | --- |
    | supertrend | 39,293 | 2.78x | 3090.08x |
    | tema | 16,590 | 2.24x | 4007.39x |
    | tr | 8,239 | 1.32x | 3220.12x |
    | trendmode | 252,551 | 3.58x | --- |
    | trima | 17,300 | 2.69x | 2970.68x |
    | trix | 16,107 | 3.79x | 3345.02x |
    | trvi | 30,811 | 1.08x | --- |
    | tsf | 13,608 | 2.53x | --- |
    | typprice | 15,804 | 0.74x | 1825.04x |

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
