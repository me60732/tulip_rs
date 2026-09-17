# SIMD by_assets: 4 Assets Simultaneously

Processes 4 different assets with the same options in a single 256-bit AVX2 `f64x4` SIMD pass. Times represent the total wall-time for all 4 assets. All times are nanoseconds (ns); lower is better.

- **Speedup vs Rust** - how many times faster SIMD is compared to 4x sequential native Rust calls
- **Speedup vs [binding/library]** - how many times faster SIMD is compared to 4x sequential calls of that competitor

=== "vs Rust"

    Reference libraries: **RustTa** (bukosabino/ta) and **Kand** (kand-labs/kand),
    each called once per asset and summed to give the equivalent 4-asset sequential cost.
    Kand uses NaN-padding which may affect some indicators.

    | Indicator | SIMD 4-Asset (ns) | Speedup vs Rust | Speedup vs RustTa | Speedup vs Kand |
    |-----------|------------------:|-----------------:|-------------------:|----------------:|
    | ad | 16,266 | 1.18x | --- | 1.26x |
    | adaptivemsw | 1,785,341 | 1.18x | --- | --- |
    | adosc | 18,352 | 1.23x | --- | 5.33x |
    | adx | 21,309 | 2.01x | --- | 14.15x |
    | adxr | 24,689 | 2.42x | --- | 12.50x |
    | ao | 14,487 | 1.50x | --- | --- |
    | apo | 10,305 | 1.89x | --- | --- |
    | aroon | 76,294 | 0.98x | --- | 19.65x |
    | aroonosc | 77,794 | 1.00x | --- | 19.34x |
    | atr | 13,289 | 1.45x | 2.91x | 6.86x |
    | avgprice | 5,677 | 1.01x | --- | --- |
    | bbands | 23,726 | 0.88x | --- | --- |
    | bop | 9,708 | 1.01x | --- | 1.18x |
    | ccfisher | 275,801 | 3.31x | --- | --- |
    | cci | 71,967 | 3.30x | --- | --- |
    | chaikinmf | 20,817 | 1.38x | --- | --- |
    | chandelierexit | 112,590 | 0.66x | 1.56x | --- |
    | cmo | 11,736 | 2.14x | --- | --- |
    | cvi | 13,168 | 1.58x | --- | --- |
    | cybercycle | 56,695 | 1.18x | --- | --- |
    | dema | 9,415 | 2.69x | --- | 10.08x |
    | di | 23,032 | 2.97x | --- | 4.33x |
    | dm | 18,078 | 2.20x | --- | 5.04x |
    | donchianchannel | 116,011 | 0.45x | --- | --- |
    | dpo | 14,045 | 0.77x | --- | --- |
    | dx | 19,111 | 1.97x | --- | 10.96x |
    | ef | 10,709 | 1.46x | 9.90x | --- |
    | elderray | 18,261 | 1.33x | --- | --- |
    | ema | 9,497 | 2.02x | 3.49x | 3.64x |
    | emv | 9,728 | 1.01x | --- | --- |
    | fisher | 178,104 | 1.09x | --- | --- |
    | fosc | 11,510 | 0.81x | --- | --- |
    | highpass | 10,395 | 1.85x | --- | --- |
    | hilberttransform | 24,733 | 2.31x | --- | --- |
    | hma | 17,334 | 2.16x | --- | --- |
    | homodynediscriminator | 298,453 | 2.91x | --- | --- |
    | ichimoku | 402,544 | 0.69x | --- | --- |
    | instantaneoustrendline | 358,442 | 2.48x | --- | --- |
    | kama | 12,728 | 2.34x | --- | --- |
    | keltnerchannel | 26,530 | 1.03x | 1.95x | --- |
    | kvo | 26,431 | 1.36x | --- | --- |
    | linreg | 9,885 | 3.05x | --- | --- |
    | macd | 23,917 | 1.06x | 1.39x | 4.93x |
    | mama | 256,679 | 3.43x | --- | --- |
    | marketfi | 12,414 | 0.79x | --- | --- |
    | mass | 13,436 | 1.95x | --- | --- |
    | max | 21,430 | 0.97x | 2.65x | --- |
    | md | 31,794 | 1.80x | 4.64x | --- |
    | medprice | 3,915 | 0.89x | --- | 1.53x |
    | mfi | 21,861 | 1.46x | 4.36x | 52.76x |
    | min | 33,635 | 0.82x | 3.02x | --- |
    | mom | 3,415 | 1.05x | --- | 1.65x |
    | msw | 298,731 | 1.73x | --- | --- |
    | natr | 13,126 | 1.51x | --- | 7.84x |
    | nvi | 11,288 | 0.95x | --- | --- |
    | obv | 10,457 | 1.34x | 2.10x | 1.44x |
    | ppo | 10,403 | 1.95x | 3.19x | --- |
    | psar | 73,128 | 0.47x | --- | 1.00x |
    | pvi | 10,631 | 0.96x | --- | --- |
    | qstick | 10,432 | 0.99x | --- | --- |
    | roc | 10,429 | 0.93x | 1.18x | 1.08x |
    | rocr | 10,531 | 0.93x | --- | 1.04x |
    | roofingfilter | 11,828 | 3.63x | --- | --- |
    | rsi | 10,442 | 1.91x | 3.18x | 9.20x |
    | sma | 9,516 | 1.08x | 2.01x | 2.13x |
    | smaenvelope | 24,671 | 1.15x | --- | --- |
    | stddev | 15,157 | 1.00x | 2.33x | --- |
    | stoch | 90,638 | 0.90x | 2.16x | 17.96x |
    | stochrsi | 134,154 | 0.57x | --- | --- |
    | supersmoother | 12,154 | 3.53x | --- | --- |
    | supertrend | 25,150 | 1.92x | --- | 7.16x |
    | tema | 10,444 | 2.60x | --- | 10.75x |
    | tr | 5,595 | 1.00x | 5.63x | 1.59x |
    | trendmode | 251,480 | 3.49x | --- | --- |
    | trima | 13,096 | 1.72x | --- | 3.22x |
    | trix | 10,496 | 2.44x | --- | 11.14x |
    | trvi | 17,567 | 1.25x | --- | --- |
    | tsf | 10,981 | 2.41x | --- | --- |
    | typprice | 4,619 | 0.94x | --- | 2.43x |
    | ultosc | 32,730 | 1.90x | --- | --- |
    | vhf | 77,292 | 0.70x | --- | --- |
    | vidya | 38,448 | 1.27x | --- | --- |
    | volatility | 24,966 | 1.09x | --- | --- |
    | vortex | 25,523 | 1.26x | --- | --- |
    | vosc | 14,743 | 0.98x | --- | --- |
    | vwap | 16,274 | 0.78x | --- | 2.16x |
    | vwma | 16,507 | 0.85x | --- | --- |
    | wad | 13,998 | 1.13x | --- | --- |
    | wcprice | 4,451 | 0.97x | --- | 1.54x |
    | wilders | 9,255 | 2.06x | --- | --- |
    | willr | 80,508 | 0.86x | --- | 17.85x |
    | wma | 9,213 | 2.15x | --- | 23.93x |
    | zlema | 9,193 | 2.60x | --- | --- |

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
    | bbands | 23,726 | 0.88x | --- | --- |
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
    | bbands | 93,447 | 0.87x | 268.13x | 12.75x |
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
    | ad | 16,128 | 4.77x | 4.71x | 4.72x |
    | adaptivemsw | 1,744,942 | 4.86x | --- | --- |
    | adosc | 17,159 | 1.32x | 2.09x | 1.99x |
    | adx | 21,169 | 2.04x | 1.88x | 7.21x |
    | adxr | 25,370 | 2.12x | 2.07x | 6.19x |
    | ao | 13,256 | 6.79x | 5.46x | --- |
    | apo | 9,706 | 2.01x | 3.58x | 4.27x |
    | aroon | 75,769 | 0.90x | 4.24x | 2.04x |
    | aroonosc | 83,020 | 0.96x | 3.95x | 1.74x |
    | atr | 13,009 | 1.48x | 3.27x | 8.23x |
    | avgprice | 5,859 | 3.87x | 3.76x | 4.59x |
    | bbands | 39,222 | 0.74x | 0.73x | 1.99x |
    | bop | 9,906 | 3.94x | 3.86x | 7.68x |
    | ccfisher | 287,813 | 4.77x | --- | --- |
    | cci | 72,282 | 4.38x | 4.09x | 6.19x |
    | chaikinmf | 20,790 | 1.84x | --- | --- |
    | chandelierexit | 104,451 | 0.66x | --- | --- |
    | cmo | 11,896 | 2.29x | 2.22x | 8.71x |
    | cvi | 12,476 | 1.56x | 4.52x | --- |
    | cybercycle | 55,974 | 4.76x | --- | --- |
    | dema | 9,212 | 2.52x | 2.79x | 9.68x |
    | di | 23,427 | 3.02x | 1.68x | 9.73x |
    | dm | 17,420 | 2.07x | 1.38x | --- |
    | donchianchannel | 168,582 | 0.31x | 1.16x | --- |
    | dpo | 10,864 | 0.92x | 0.89x | --- |
    | dx | 19,403 | 2.02x | 1.55x | 7.81x |
    | ef | 10,951 | 1.76x | --- | --- |
    | elderray | 18,122 | 1.34x | 2.36x | --- |
    | ema | 9,422 | 2.04x | 4.53x | 4.52x |
    | emv | 10,004 | 0.98x | 1.92x | --- |
    | fisher | 277,589 | 0.88x | 1.12x | --- |
    | fosc | 11,586 | 2.84x | 3.23x | --- |
    | highpass | 10,003 | 1.93x | --- | --- |
    | hma | 17,538 | 1.97x | 2.15x | --- |
    | ichimoku | 534,046 | 0.55x | --- | --- |
    | kama | 12,703 | 2.20x | 2.64x | 3.37x |
    | keltnerchannel | 77,000 | 0.35x | --- | --- |
    | kvo | 26,378 | 1.38x | 1.35x | --- |
    | linreg | 10,422 | 2.79x | 3.41x | 21.87x |
    | macd | 74,099 | 0.34x | 0.67x | 1.86x |
    | mama | 256,812 | 3.42x | --- | 3.49x |
    | marketfi | 12,684 | 3.07x | 3.01x | --- |
    | mass | 12,951 | 1.76x | 3.66x | --- |
    | max | 29,255 | 0.82x | 2.75x | 1.44x |
    | md | 35,023 | 1.79x | 1.81x | --- |
    | medprice | 4,091 | 0.87x | 0.82x | 0.86x |
    | mfi | 22,336 | 1.55x | 2.12x | 1.76x |
    | min | 39,550 | 0.65x | 2.77x | 1.48x |
    | mom | 3,537 | 0.90x | 0.84x | 0.87x |
    | msw | 172,265 | 3.01x | 15.47x | --- |
    | natr | 13,040 | 1.73x | 3.26x | 8.20x |
    | nvi | 10,508 | 1.02x | 1.40x | --- |
    | obv | 9,908 | 1.44x | 2.67x | 1.56x |
    | pivotpoint | 457 | 0.77x | --- | --- |
    | ppo | 10,470 | 2.11x | 3.55x | 5.43x |
    | psar | 57,526 | 0.88x | 0.95x | 0.79x |
    | pvi | 10,559 | 1.04x | 1.37x | --- |
    | qstick | 10,703 | 0.98x | 0.95x | --- |
    | roc | 10,406 | 0.93x | 0.91x | 1.82x |
    | rocr | 10,294 | 0.94x | 0.92x | 1.83x |
    | roofingfilter | 11,792 | 3.65x | --- | --- |
    | rsi | 10,348 | 2.07x | 4.73x | 9.98x |
    | sma | 9,982 | 0.99x | 0.94x | 1.88x |
    | smaenvelope | 24,102 | 1.14x | --- | --- |
    | stddev | 15,225 | 1.89x | 1.87x | 4.36x |
    | stoch | 92,919 | 0.93x | 3.95x | 2.30x |
    | stochrsi | 135,456 | 0.57x | 1.00x | 1.48x |
    | supersmoother | 12,702 | 3.38x | --- | --- |
    | supertrend | 25,209 | 1.89x | --- | --- |
    | tema | 9,211 | 2.95x | 2.86x | 14.37x |
    | tr | 5,785 | 0.96x | 0.99x | 1.32x |
    | trendmode | 250,617 | 5.30x | --- | --- |
    | trima | 13,051 | 1.65x | 2.18x | 2.18x |
    | trix | 10,599 | 2.55x | 4.02x | 13.74x |
    | trvi | 15,715 | 1.47x | --- | --- |
    | tsf | 10,753 | 2.58x | 3.38x | 21.48x |
    | typprice | 4,783 | 0.99x | 1.04x | 1.99x |
    | ultosc | 30,845 | 2.01x | 2.00x | 5.92x |
    | vhf | 93,624 | 0.57x | 2.12x | --- |
    | vidya | 38,960 | 1.23x | 1.94x | --- |
    | volatility | 24,990 | 1.94x | 2.86x | --- |
    | vortex | 24,631 | 1.29x | --- | --- |
    | vosc | 13,578 | 1.42x | 1.41x | --- |
    | vwap | 16,158 | 4.76x | --- | --- |
    | vwma | 16,298 | 1.18x | 1.16x | --- |
    | wad | 14,165 | 4.29x | 5.22x | --- |
    | wcprice | 4,775 | 4.01x | 4.19x | --- |
    | wilders | 9,591 | 2.00x | 4.43x | --- |
    | willr | 80,915 | 0.71x | 4.09x | 2.06x |
    | wma | 9,338 | 2.06x | 3.56x | 2.04x |
    | zlema | 9,466 | 2.53x | 3.50x | --- |

=== "Go Binding"

    Reference: **cinar/indicator/v2** (pure Go), called once per asset and summed to give the
    equivalent 4-asset sequential cost; the `tulip_rs_go` column folds in the binding's own
    4 sequential calls as the SIMD speedup baseline. cinar is only compared where param-compatible.

    | Indicator | SIMD 4-Asset (ns) | Speedup vs tulip_rs_go | Speedup vs cinar |
    |-----------|------------------:|-----------------------:|-----------------:|
    | ad | 20,414 | 1.07x | 1782.15x |
    | adaptivemsw | 1,774,221 | 1.19x | --- |
    | adosc | 23,652 | 1.12x | 3917.51x |
    | adx | 26,695 | 1.76x | --- |
    | adxr | 30,170 | 1.92x | --- |
    | ao | 16,109 | 1.58x | 5606.49x |
    | apo | 13,540 | 1.61x | 1525.03x |
    | aroon | 107,753 | 0.66x | 512.18x |
    | aroonosc | 86,796 | 0.97x | --- |
    | atr | 18,288 | 1.19x | 3677.33x |
    | avgprice | 10,961 | 0.70x | --- |
    | bbands | 85,958 | 0.46x | 923.22x |
    | bop | 16,215 | 0.77x | 1835.70x |
    | ccfisher | 305,619 | 3.00x | --- |
    | cci | 79,060 | 4.05x | 1675.29x |
    | chaikinmf | 30,183 | 1.35x | 2998.97x |
    | chandelierexit | 146,298 | 0.50x | 648.17x |
    | cmo | 17,931 | 1.66x | --- |
    | cvi | 17,260 | 1.32x | --- |
    | cybercycle | 64,409 | 1.08x | --- |
    | dema | 14,599 | 1.76x | 4006.81x |
    | di | 58,401 | 1.32x | --- |
    | dm | 48,427 | 0.80x | --- |
    | donchianchannel | 161,777 | 0.35x | 465.53x |
    | dpo | 17,508 | 0.71x | 4333.38x |
    | dx | 26,546 | 1.61x | --- |
    | ef | 16,624 | 1.31x | --- |
    | elderray | 54,514 | 0.55x | 1189.11x |
    | ema | 13,895 | 1.56x | 941.41x |
    | emv | 14,829 | 0.85x | 6331.26x |
    | fisher | 322,807 | 0.80x | 418.10x |
    | fosc | 17,533 | 2.02x | --- |
    | highpass | 15,486 | 1.40x | --- |
    | hilberttransform | 59,434 | 1.01x | --- |
    | hma | 21,641 | 1.72x | 3412.25x |
    | homodynediscriminator | 303,174 | 2.88x | --- |
    | ichimoku | 556,878 | 0.55x | --- |
    | instantaneoustrendline | 366,879 | 2.42x | --- |
    | kama | 18,402 | 1.66x | 6341.97x |
    | keltnerchannel | 85,889 | 0.39x | --- |
    | kvo | 33,270 | 1.23x | --- |
    | linreg | 16,469 | 1.91x | --- |
    | macd | 85,076 | 0.38x | --- |
    | mama | 287,729 | 3.07x | --- |
    | marketfi | 19,793 | 0.62x | --- |
    | mass | 16,877 | 1.57x | --- |
    | max | 35,909 | 0.74x | 1409.87x |
    | md | 40,005 | 1.63x | --- |
    | medprice | 6,262 | 1.05x | --- |
    | mfi | 28,463 | 1.37x | 3441.48x |
    | min | 42,743 | 0.67x | 1038.06x |
    | mom | 5,539 | 1.06x | --- |
    | msw | 234,590 | 2.24x | --- |
    | natr | 17,803 | 1.41x | --- |
    | nvi | 14,911 | 0.90x | 5922.97x |
    | obv | 16,308 | 1.03x | 1415.63x |
    | pivotpoint | 3,371 | 0.87x | --- |
    | ppo | 13,799 | 1.78x | 6590.74x |
    | psar | 60,753 | 0.76x | --- |
    | pvi | 15,030 | 0.89x | --- |
    | qstick | 12,500 | 1.04x | 5006.63x |
    | roc | 14,481 | 0.83x | 1008.60x |
    | rocr | 13,421 | 0.91x | --- |
    | roofingfilter | 15,110 | 3.01x | --- |
    | rsi | 14,243 | 1.69x | 6263.51x |
    | sma | 14,543 | 0.85x | 2998.84x |
    | smaenvelope | 82,586 | 0.41x | 969.94x |
    | stddev | 17,251 | 1.80x | 1966.87x |
    | stoch | 100,611 | 0.90x | 728.38x |
    | stochrsi | 142,194 | 0.57x | --- |
    | supersmoother | 15,669 | 2.90x | --- |
    | supertrend | 30,131 | 1.79x | 4661.85x |
    | tema | 14,004 | 2.12x | 7501.81x |
    | tr | 8,396 | 1.00x | 5119.43x |
    | trendmode | 251,923 | 3.51x | --- |
    | trima | 16,510 | 1.45x | 4172.96x |
    | trix | 14,252 | 2.06x | 5906.43x |
    | trvi | 20,149 | 1.31x | --- |
    | tsf | 14,363 | 2.10x | --- |
    | typprice | 6,899 | 1.00x | 5895.92x |
    | ultosc | 33,283 | 1.93x | 4075.84x |
    | vhf | 97,555 | 0.58x | --- |
    | vidya | 41,077 | 1.24x | --- |
    | volatility | 27,377 | 1.87x | --- |
    | vortex | 32,092 | 1.12x | --- |
    | vosc | 15,536 | 1.40x | --- |
    | vwap | 19,428 | 1.12x | --- |
    | vwma | 18,312 | 1.19x | 4303.34x |
    | wad | 17,593 | 1.02x | --- |
    | wcprice | 6,643 | 1.02x | 4864.15x |
    | wilders | 13,165 | 1.65x | 583.24x |
    | willr | 84,859 | 0.72x | 1159.09x |
    | wma | 14,582 | 1.48x | 1260.15x |
    | zlema | 13,727 | 1.93x | --- |

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
