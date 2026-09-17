# SIMD by_options: 4 Option Sets Simultaneously

Processes 1 asset with 4 different option configurations in a single SIMD pass. Neither C_tulip, TA-Lib, RustTa, nor Kand have a native by_options SIMD mode, so their comparison baseline is computed as 4x their single-option-set average call time (i.e. 4 independent sequential calls). All times are nanoseconds (ns); lower is better.

- **Speedup vs binding scalar** - `SIMD 4-Options (ns) / Binding's own sequential calls` (tulip_rs_python, tulip_rs_node, tulip_rs_ffi_c, tulip_rs_go)
- **Speedup vs competitor libraries** - `4x Competitor's single-call avg time / SIMD 4-Options (ns)`

=== "vs Rust"

    Reference libraries: **RustTa** (bukosabino/ta) and **Kand** (kand-labs/kand),
    each called once per option-set and summed to give the equivalent 4-option-set sequential cost.
    Kand uses NaN-padding which may affect some indicators.

    | Indicator | SIMD 4-Options (ns) | Speedup vs Rust | Speedup vs RustTa | Speedup vs Kand |
    |-----------|--------------------:|-----------------:|-------------------:|----------------:|
    | adosc | 10,722 | 2.11x | --- | 9.13x |
    | adx | 16,822 | 2.55x | --- | 17.92x |
    | adxr | 27,674 | 2.16x | --- | 11.16x |
    | apo | 9,593 | 2.04x | --- | --- |
    | aroon | 92,891 | 0.81x | --- | 16.14x |
    | aroonosc | 91,194 | 0.86x | --- | 16.50x |
    | atr | 10,056 | 1.92x | 3.85x | 9.07x |
    | bbands | 23,016 | 0.93x | --- | --- |
    | ccfisher | 279,742 | 3.26x | --- | --- |
    | cci | 119,200 | 2.63x | --- | --- |
    | chaikinmf | 20,332 | 1.42x | --- | --- |
    | chandelierexit | 120,173 | 0.61x | 1.46x | --- |
    | cmo | 12,529 | 2.01x | --- | --- |
    | cvi | 15,153 | 1.38x | --- | --- |
    | cybercycle | 21,151 | 3.15x | --- | --- |
    | dema | 9,033 | 2.82x | --- | 10.51x |
    | di | 19,688 | 3.48x | --- | 5.06x |
    | dm | 17,988 | 2.22x | --- | 5.06x |
    | donchianchannel | 119,596 | 0.44x | --- | --- |
    | dpo | 13,748 | 0.79x | --- | --- |
    | dx | 14,746 | 2.56x | --- | 14.21x |
    | ef | 11,242 | 1.39x | 9.43x | --- |
    | elderray | 16,604 | 1.47x | --- | --- |
    | ema | 8,669 | 2.21x | 3.83x | 3.99x |
    | fisher | 187,945 | 1.04x | --- | --- |
    | fosc | 12,479 | 1.62x | --- | --- |
    | highpass | 10,672 | 1.81x | --- | --- |
    | hilberttransform | 24,843 | 2.31x | --- | --- |
    | hma | 28,031 | 1.33x | --- | --- |
    | ichimoku | 360,258 | 0.77x | --- | --- |
    | kama | 12,833 | 2.32x | --- | --- |
    | keltnerchannel | 30,882 | 0.89x | 1.67x | --- |
    | kvo | 18,065 | 2.00x | --- | --- |
    | linreg | 11,047 | 2.74x | --- | --- |
    | macd | 23,503 | 1.08x | 1.41x | 5.02x |
    | mama | 262,591 | 3.50x | --- | --- |
    | mass | 12,882 | 2.03x | --- | --- |
    | max | 27,480 | 0.76x | 2.07x | --- |
    | md | 55,496 | 1.00x | 2.66x | --- |
    | mfi | 18,037 | 1.77x | 5.29x | 63.95x |
    | min | 48,048 | 0.59x | 2.12x | --- |
    | mom | 3,656 | 0.98x | --- | 1.54x |
    | msw | 146,336 | 3.54x | --- | --- |
    | natr | 11,240 | 1.77x | --- | 9.15x |
    | ppo | 11,194 | 1.81x | 2.97x | --- |
    | psar | 62,801 | 0.56x | --- | 1.16x |
    | qstick | 10,267 | 1.00x | --- | --- |
    | roc | 10,649 | 0.92x | 1.15x | 1.06x |
    | rocr | 11,217 | 0.88x | --- | 0.97x |
    | roofingfilter | 12,210 | 3.52x | --- | --- |
    | rsi | 10,737 | 1.86x | 3.09x | 8.95x |
    | sma | 9,724 | 1.05x | 1.97x | 2.08x |
    | smaenvelope | 23,187 | 1.23x | --- | --- |
    | stddev | 15,431 | 0.99x | 2.29x | --- |
    | stoch | 118,833 | 0.69x | 1.65x | 13.70x |
    | stochrsi | 117,900 | 0.65x | --- | --- |
    | supersmoother | 13,770 | 3.12x | --- | --- |
    | supertrend | 22,096 | 2.19x | --- | 8.15x |
    | tema | 9,594 | 2.86x | --- | 11.70x |
    | trendmode | 228,155 | 3.85x | --- | --- |
    | trima | 16,656 | 1.35x | --- | 2.53x |
    | trix | 11,661 | 2.21x | --- | 10.03x |
    | trvi | 15,962 | 1.38x | --- | --- |
    | tsf | 11,371 | 2.34x | --- | --- |
    | ultosc | 67,072 | 0.93x | --- | --- |
    | vhf | 97,967 | 0.56x | --- | --- |
    | vidya | 38,955 | 1.25x | --- | --- |
    | volatility | 22,330 | 1.22x | --- | --- |
    | vortex | 25,792 | 1.26x | --- | --- |
    | vosc | 13,701 | 1.06x | --- | --- |
    | vwma | 16,799 | 0.83x | --- | --- |
    | wilders | 10,519 | 1.83x | --- | --- |
    | willr | 91,740 | 0.75x | --- | 15.67x |
    | wma | 10,150 | 1.95x | --- | 21.73x |
    | zlema | 10,147 | 2.37x | --- | --- |

=== "vs C"

    Reference: **C_tulip** and **TA-Lib**, each called once per option-set and summed to give the equivalent
    4-option-set sequential cost for the same asset. There is no native SIMD equivalent for TA-Lib in this mode.

    | Indicator | SIMD 4-Options (ns) | Speedup vs Rust | Speedup vs C | Speedup vs TA-Lib |
    |-----------|--------------------:|-----------------:|-------------:|------------------:|
    | adosc | 10,722 | 2.11x | 3.50x | 3.19x |
    | adx | 16,822 | 2.55x | 3.95x | 9.08x |
    | adxr | 27,674 | 2.16x | 2.60x | 5.68x |
    | apo | 9,593 | 2.04x | 3.78x | 4.32x |
    | aroon | 92,891 | 0.81x | 3.52x | 1.66x |
    | aroonosc | 91,194 | 0.86x | 3.59x | 1.58x |
    | atr | 10,056 | 1.92x | 4.39x | 10.65x |
    | bbands | 22,486 | 0.93x | 1.60x | 3.46x |
    | cci | 119,243 | 2.63x | 2.52x | 3.75x |
    | cmo | 12,529 | 2.01x | 7.80x | 8.27x |
    | cvi | 15,153 | 1.38x | 3.86x | --- |
    | dema | 9,033 | 2.82x | 3.02x | 9.87x |
    | di | 19,688 | 3.48x | 2.15x | 11.58x |
    | dm | 17,988 | 2.22x | 1.57x | --- |
    | donchianchannel | 119,596 | 0.44x | 1.94x | --- |
    | dpo | 13,748 | 0.79x | 0.82x | --- |
    | dx | 14,746 | 2.56x | 1.92x | 10.28x |
    | elderray | 16,604 | 1.47x | 3.73x | --- |
    | ema | 8,669 | 2.21x | 5.09x | 4.91x |
    | fisher | 187,945 | 1.04x | 2.87x | --- |
    | fosc | 20,039 | 1.62x | 1.96x | --- |
    | hma | 28,031 | 1.33x | 1.40x | --- |
    | kama | 12,833 | 2.32x | 2.73x | 3.34x |
    | kvo | 18,065 | 2.00x | 2.07x | --- |
    | linreg | 11,280 | 2.74x | 2.43x | 20.21x |
    | macd | 23,503 | 1.08x | 2.39x | 5.86x |
    | mass | 12,882 | 2.03x | 3.82x | --- |
    | max | 27,480 | 0.76x | 4.44x | 1.53x |
    | md | 55,496 | 1.00x | 1.07x | --- |
    | mfi | 18,037 | 1.77x | 2.74x | 2.19x |
    | min | 48,048 | 0.59x | 4.73x | 1.22x |
    | mom | 3,656 | 0.98x | 1.61x | 0.84x |
    | msw | 147,040 | 3.54x | 40.10x | --- |
    | natr | 11,240 | 1.77x | 3.92x | 9.52x |
    | ppo | 11,194 | 1.81x | 3.46x | 5.08x |
    | psar | 62,801 | 0.56x | 0.58x | 0.54x |
    | qstick | 10,267 | 1.00x | 1.19x | --- |
    | roc | 10,649 | 0.92x | 1.04x | 1.78x |
    | rocr | 11,217 | 0.88x | 0.99x | 1.68x |
    | rsi | 10,737 | 1.86x | 4.13x | 9.62x |
    | sma | 9,724 | 1.05x | 1.13x | 1.93x |
    | stddev | 15,431 | 0.99x | 1.93x | 4.30x |
    | stoch | 118,833 | 0.69x | 3.19x | 1.80x |
    | stochrsi | 117,900 | 0.65x | 1.47x | 1.70x |
    | tema | 9,594 | 2.86x | 2.92x | 13.80x |
    | trima | 16,656 | 1.35x | 1.80x | 1.71x |
    | trix | 11,661 | 2.21x | 3.81x | 12.49x |
    | tsf | 11,371 | 2.34x | 3.30x | 20.31x |
    | ultosc | 67,072 | 0.93x | 1.11x | 2.72x |
    | vhf | 97,967 | 0.56x | 3.20x | --- |
    | vidya | 38,955 | 1.25x | 1.99x | --- |
    | volatility | 22,330 | 1.22x | 3.26x | --- |
    | vosc | 13,701 | 1.06x | 1.50x | --- |
    | vwma | 16,799 | 0.83x | 1.22x | --- |
    | wilders | 10,519 | 1.83x | 4.21x | --- |
    | willr | 91,740 | 0.75x | 3.76x | 1.81x |
    | wma | 10,150 | 1.95x | 3.44x | 1.88x |
    | zlema | 10,147 | 2.37x | 3.43x | --- |

=== "Python Binding"

    Reference: **tulip_rs_python** PyO3 binding. Competitor speedups are computed as 4x the competitor's average single-call time,
    since these libraries have no SIMD-equivalent batched mode.

    | Indicator | SIMD 4-Options (ns) | Speedup vs tulip_rs_python | Speedup vs ta | Speedup vs pandas_ta |
    |-----------|--------------------:|----------------------------:|-------------:|-------------------:|
    | adosc | 19,742 | 1.41x | --- | --- |
    | adx | 39,774 | 2.50x | 2172.95x | 344.78x |
    | adxr | 40,260 | 2.32x | --- | --- |
    | apo | 18,772 | 1.56x | --- | 29.97x |
    | aroon | 96,641 | 1.47x | 429.45x | 525.19x |
    | aroonosc | 100,194 | 1.34x | --- | 508.65x |
    | atr | 19,303 | 2.92x | 2362.27x | 197.23x |
    | bbands | 41,312 | 1.44x | 23.14x | 54.28x |
    | ccfisher | 291,025 | 3.15x | --- | --- |
    | cci | 123,824 | 2.99x | 834.10x | 709.36x |
    | chaikinmf | 29,182 | 1.47x | 38.22x | 45.34x |
    | chandelierexit | 86,147 | 1.56x | --- | 80.89x |
    | cmo | 14,122 | 2.40x | --- | 157.30x |
    | cvi | 20,292 | 1.49x | --- | --- |
    | cybercycle | 21,650 | 3.19x | --- | --- |
    | dema | 20,070 | 1.59x | 26.71x | 57.23x |
    | di | 33,902 | 5.24x | --- | 248.16x |
    | dm | 30,040 | 3.40x | --- | 280.99x |
    | donchianchannel | 103,600 | 0.65x | 10.54x | 15.72x |
    | dpo | 20,024 | 0.66x | 24.75x | 29.05x |
    | dx | 23,061 | 5.28x | --- | 591.36x |
    | ef | 15,017 | 1.70x | --- | 41.10x |
    | elderray | 18,924 | 1.83x | --- | 63.77x |
    | ema | 15,428 | 1.58x | 14.92x | 30.83x |
    | fisher | 248,727 | 1.31x | --- | 126.39x |
    | fosc | 14,083 | 2.47x | --- | --- |
    | highpass | 11,095 | 2.35x | --- | --- |
    | hilberttransform | 34,764 | 2.24x | --- | --- |
    | hma | 34,730 | 2.41x | 1176.49x | 70.57x |
    | ichimoku | 334,842 | 1.13x | --- | 16.40x |
    | kama | 22,076 | 4.73x | 699.65x | 1891.33x |
    | keltnerchannel | 38,011 | 2.27x | 44.17x | 133.94x |
    | kvo | 26,728 | 3.86x | --- | 178.09x |
    | linreg | 24,519 | 4.22x | --- | 2819.40x |
    | macd | 57,972 | 1.71x | 13.62x | 55.72x |
    | mama | 258,057 | 3.47x | --- | 4.22x |
    | mass | 24,621 | 1.24x | 34.22x | 79.46x |
    | max | 27,766 | 1.24x | --- | --- |
    | md | 62,561 | 2.52x | --- | 1388.50x |
    | mfi | 35,655 | 1.23x | 3335.93x | 22.09x |
    | min | 57,113 | 0.86x | --- | --- |
    | mom | 5,209 | 1.04x | 15.58x | 21.14x |
    | msw | 147,230 | 3.61x | --- | --- |
    | natr | 12,467 | 5.29x | --- | 344.59x |
    | ppo | 24,492 | 1.70x | 37.70x | 77.96x |
    | psar | 77,464 | 1.50x | 10275.25x | 255.21x |
    | qstick | 23,994 | 0.88x | --- | --- |
    | roc | 22,088 | 1.40x | 19.15x | 6.11x |
    | rocr | 14,720 | 2.51x | --- | 12.50x |
    | roofingfilter | 25,027 | 2.59x | --- | --- |
    | rsi | 17,873 | 1.70x | 106.92x | 112.26x |
    | sma | 19,112 | 0.95x | 15.52x | 76.32x |
    | smaenvelope | 40,112 | 1.07x | --- | --- |
    | stddev | 35,395 | 1.26x | --- | 15.85x |
    | stoch | 107,257 | 1.17x | 11.17x | 37.46x |
    | stochrsi | 125,647 | 1.27x | 28.93x | 34.93x |
    | supersmoother | 14,821 | 3.08x | --- | 11.03x |
    | supertrend | 26,229 | 4.60x | --- | 6734.40x |
    | tema | 16,238 | 2.30x | 53.00x | 102.85x |
    | trendmode | 227,861 | 3.87x | --- | --- |
    | trima | 26,507 | 1.26x | --- | 16.52x |
    | trix | 14,334 | 5.14x | 85.63x | 213.58x |
    | trvi | 37,233 | 0.79x | --- | --- |
    | tsf | 19,684 | 3.46x | --- | 3546.38x |
    | ultosc | 43,766 | 2.00x | 148.77x | 177.93x |
    | vhf | 105,627 | 0.63x | --- | 18.64x |
    | vidya | 40,978 | 2.06x | --- | 8070.98x |
    | volatility | 50,107 | 1.11x | --- | 11.39x |
    | vortex | 46,229 | 1.82x | 79.73x | 109.83x |
    | vosc | 22,283 | 1.25x | --- | --- |
    | vwma | 30,400 | 1.42x | --- | 26.66x |
    | wilders | 14,875 | 1.63x | --- | 17.74x |
    | willr | 92,275 | 0.93x | 12.96x | 14.71x |
    | wma | 11,038 | 2.21x | 1222.49x | 76.83x |
    | zlema | 11,191 | 3.14x | --- | 81.22x |

=== "Node Binding"

    Reference: **tulip_rs_node** napi-rs binding. Competitor speedups are computed as 4x the competitor's average single-call time,
    since these libraries have no SIMD-equivalent batched mode.

    | Indicator | SIMD 4-Options (ns) | Speedup vs tulip_rs_node | Speedup vs technicalindicators | Speedup vs indicatorts |
    |-----------|--------------------:|--------------------------:|------------------------------:|---------------------:|
    | adosc | 31,289 | 1.22x | --- | --- |
    | adx | 33,165 | 1.68x | 161.05x | --- |
    | adxr | 46,864 | 1.78x | --- | --- |
    | apo | 31,389 | 1.03x | --- | 5.86x |
    | aroon | 143,798 | 0.80x | --- | 20.81x |
    | aroonosc | 135,741 | 0.75x | --- | --- |
    | atr | 30,653 | 1.15x | 54.92x | 35.19x |
    | bbands | 68,371 | 1.15x | 366.47x | 17.42x |
    | ccfisher | 298,531 | 3.14x | --- | --- |
    | cci | 151,568 | 2.25x | 127.44x | 1.75x |
    | chaikinmf | 33,571 | 1.54x | --- | 6.53x |
    | chandelierexit | 147,018 | 0.72x | --- | 27.58x |
    | cmo | 27,107 | 1.44x | --- | --- |
    | cvi | 31,639 | 1.07x | --- | --- |
    | cybercycle | 33,380 | 2.27x | --- | --- |
    | dema | 23,610 | 1.35x | --- | 7.86x |
    | di | 52,006 | 2.13x | --- | --- |
    | dm | 45,208 | 1.54x | --- | --- |
    | donchianchannel | 133,390 | 0.62x | --- | 24.36x |
    | dpo | 26,686 | 0.86x | --- | --- |
    | dx | 35,546 | 1.39x | --- | --- |
    | ef | 27,045 | 1.02x | --- | --- |
    | elderray | 53,494 | 1.27x | --- | --- |
    | ema | 27,026 | 1.15x | 21.93x | 2.97x |
    | fisher | 274,736 | 1.07x | --- | --- |
    | fosc | 29,044 | 1.54x | --- | --- |
    | highpass | 27,049 | 1.00x | --- | --- |
    | hilberttransform | 56,191 | 1.64x | --- | --- |
    | hma | 43,792 | 0.98x | --- | --- |
    | ichimoku | 383,447 | 0.92x | 44.96x | --- |
    | kama | 24,557 | 1.53x | --- | --- |
    | keltnerchannel | 138,170 | 0.84x | --- | 8.32x |
    | kvo | 34,192 | 1.43x | --- | --- |
    | linreg | 27,691 | 1.39x | --- | --- |
    | macd | 53,881 | 1.34x | 45.55x | 4.50x |
    | mama | 274,763 | 3.34x | --- | --- |
    | mass | 36,716 | 1.27x | --- | --- |
    | max | 43,414 | 1.01x | --- | 52.14x |
    | md | 70,527 | 1.03x | --- | --- |
    | mfi | 39,266 | 1.34x | 206.72x | 26.57x |
    | min | 67,594 | 0.90x | --- | 30.08x |
    | mom | 22,423 | 0.88x | --- | --- |
    | msw | 164,844 | 3.39x | --- | --- |
    | natr | 30,122 | 1.15x | --- | --- |
    | ppo | 32,358 | 1.04x | --- | 9.77x |
    | psar | 76,669 | 0.68x | 11.93x | 2.40x |
    | qstick | 25,549 | 0.97x | --- | 2.40x |
    | roc | 25,325 | 1.02x | 100.53x | 2.10x |
    | rocr | 27,927 | 0.75x | --- | --- |
    | roofingfilter | 30,756 | 1.65x | --- | --- |
    | rsi | 24,472 | 1.28x | 151.27x | 17.39x |
    | sma | 27,305 | 1.06x | 65.46x | 1.68x |
    | smaenvelope | 75,480 | 0.93x | --- | --- |
    | stddev | 28,218 | 1.34x | --- | --- |
    | stoch | 174,305 | 0.81x | 40.20x | 13.72x |
    | stochrsi | 139,586 | 0.70x | 107.76x | --- |
    | supersmoother | 27,864 | 1.92x | --- | --- |
    | supertrend | 36,431 | 1.73x | --- | --- |
    | tema | 30,503 | 1.21x | --- | 10.37x |
    | trendmode | 248,410 | 3.60x | --- | --- |
    | trima | 28,644 | 1.08x | --- | 3.03x |
    | trix | 29,436 | 1.39x | 101.82x | 10.20x |
    | trvi | 37,439 | 1.05x | --- | --- |
    | tsf | 26,792 | 1.45x | --- | --- |
    | ultosc | 55,289 | 1.41x | --- | --- |
    | vhf | 140,792 | 0.41x | --- | --- |
    | vidya | 64,413 | 1.00x | --- | --- |
    | volatility | 59,719 | 0.94x | --- | --- |
    | vortex | 48,552 | 1.70x | --- | 26.52x |
    | vosc | 29,054 | 1.03x | --- | --- |
    | vwma | 27,043 | 1.33x | --- | 4.67x |
    | wilders | 24,369 | 1.24x | 23.35x | 6.59x |
    | willr | 117,239 | 0.68x | 55.70x | 28.89x |
    | wma | 22,974 | 1.73x | 636.68x | --- |
    | zlema | 21,404 | 1.44x | --- | --- |

=== "C Binding"

    Reference: **tulip_rs_ffi_c** — the `extern "C"` FFI binding's own SIMD pass. Competitor speedups are computed as 4x the competitor's average single-call time,
    since these libraries have no SIMD-equivalent batched mode.

    | Indicator | SIMD 4-Options (ns) | Speedup vs tulip_rs_ffi_c | Speedup vs C_tulip | Speedup vs TA-Lib |
    |-----------|--------------------:|----------------------------:|------------------:|-----------------:|
    | adosc | 10,921 | 2.08x | 3.29x | 3.13x |
    | adx | 16,961 | 2.55x | 2.34x | 9.00x |
    | adxr | 25,063 | 2.14x | 2.09x | 6.27x |
    | apo | 9,422 | 2.07x | 3.68x | 4.40x |
    | aroon | 88,659 | 0.78x | 3.62x | 1.74x |
    | aroonosc | 91,629 | 0.87x | 3.58x | 1.57x |
    | atr | 9,888 | 1.95x | 4.30x | 10.83x |
    | bbands | 38,420 | 0.76x | 0.74x | 2.03x |
    | ccfisher | 278,860 | 3.27x | --- | --- |
    | cci | 119,598 | 2.63x | 2.47x | 3.74x |
    | chaikinmf | 20,576 | 1.86x | --- | --- |
    | chandelierexit | 84,549 | 0.82x | --- | --- |
    | cmo | 12,290 | 2.22x | 2.15x | 8.44x |
    | cvi | 14,810 | 1.32x | 3.81x | --- |
    | cybercycle | 20,313 | 3.29x | --- | --- |
    | dema | 10,316 | 2.25x | 2.49x | 8.65x |
    | di | 20,290 | 3.49x | 1.94x | 11.23x |
    | dm | 16,635 | 2.17x | 1.44x | --- |
    | donchianchannel | 170,730 | 0.31x | 1.15x | --- |
    | dpo | 13,096 | 0.76x | 0.74x | --- |
    | dx | 15,476 | 2.54x | 1.94x | 9.80x |
    | ef | 11,343 | 1.70x | --- | --- |
    | elderray | 18,125 | 1.34x | 2.36x | --- |
    | ema | 8,411 | 2.28x | 5.08x | 5.06x |
    | fisher | 248,277 | 0.98x | 1.25x | --- |
    | fosc | 11,893 | 2.77x | 3.15x | --- |
    | highpass | 10,052 | 1.92x | --- | --- |
    | hma | 22,868 | 1.51x | 1.65x | --- |
    | ichimoku | 433,661 | 0.68x | --- | --- |
    | kama | 13,211 | 2.12x | 2.54x | 3.24x |
    | keltnerchannel | 74,249 | 0.36x | --- | --- |
    | kvo | 18,364 | 1.98x | 1.94x | --- |
    | linreg | 11,312 | 2.57x | 3.14x | 20.15x |
    | macd | 73,060 | 0.35x | 0.68x | 1.88x |
    | mama | 256,314 | 3.43x | --- | 3.50x |
    | mass | 12,277 | 1.86x | 3.86x | --- |
    | max | 32,986 | 0.72x | 2.44x | 1.28x |
    | md | 61,517 | 0.99x | 1.03x | --- |
    | mfi | 18,381 | 1.89x | 2.58x | 2.14x |
    | min | 50,998 | 0.52x | 2.15x | 1.15x |
    | mom | 3,487 | 0.91x | 0.85x | 0.88x |
    | msw | 142,929 | 3.63x | 18.64x | --- |
    | natr | 11,130 | 2.02x | 3.82x | 9.61x |
    | pivotpoint | 414 | 0.85x | --- | --- |
    | ppo | 10,879 | 2.03x | 3.42x | 5.23x |
    | psar | 54,401 | 0.79x | 0.75x | 0.63x |
    | qstick | 10,651 | 0.98x | 0.95x | --- |
    | roc | 10,619 | 0.92x | 0.89x | 1.79x |
    | rocr | 10,838 | 0.90x | 0.87x | 1.74x |
    | roofingfilter | 12,231 | 3.52x | --- | --- |
    | rsi | 10,686 | 2.00x | 4.58x | 9.67x |
    | sma | 10,017 | 0.99x | 0.94x | 1.87x |
    | smaenvelope | 22,795 | 1.23x | --- | --- |
    | stddev | 15,491 | 1.86x | 1.84x | 4.28x |
    | stoch | 120,646 | 0.72x | 3.05x | 1.77x |
    | stochrsi | 114,567 | 0.67x | 1.18x | 1.75x |
    | supersmoother | 13,151 | 3.27x | --- | --- |
    | supertrend | 22,278 | 2.14x | --- | --- |
    | tema | 9,332 | 2.91x | 2.82x | 14.18x |
    | trendmode | 226,222 | 3.89x | --- | --- |
    | trima | 16,773 | 1.28x | 1.70x | 1.69x |
    | trix | 11,137 | 2.42x | 3.83x | 13.08x |
    | trvi | 16,206 | 1.42x | --- | --- |
    | tsf | 11,108 | 2.50x | 3.27x | 20.79x |
    | ultosc | 38,532 | 1.61x | 1.60x | 4.74x |
    | vhf | 105,089 | 0.51x | 1.89x | --- |
    | vidya | 39,572 | 1.22x | 1.91x | --- |
    | volatility | 22,328 | 2.17x | 3.20x | --- |
    | vortex | 24,609 | 1.29x | --- | --- |
    | vosc | 13,739 | 1.41x | 1.40x | --- |
    | vwma | 16,631 | 1.16x | 1.14x | --- |
    | wilders | 9,181 | 2.09x | 4.63x | --- |
    | willr | 96,195 | 0.61x | 3.44x | 1.73x |
    | wma | 9,924 | 1.94x | 3.35x | 1.92x |
    | zlema | 10,080 | 2.38x | 3.29x | --- |

=== "Go Binding"

    Reference: **tulip_rs_go** — the cgo binding's own SIMD pass. Competitor speedups are computed as 4x the competitor's average single-call time,
    since these libraries have no SIMD-equivalent batched mode.

    | Indicator | SIMD 4-Options (ns) | Speedup vs tulip_rs_go | Speedup vs cinar |
    |-----------|--------------------:|-------------------------:|---------------:|
    | adosc | 13,447 | 1.97x | 6890.53x |
    | adx | 19,063 | 2.46x | --- |
    | adxr | 27,189 | 2.14x | --- |
    | apo | 12,599 | 1.73x | 1638.93x |
    | aroon | 116,412 | 0.62x | 474.08x |
    | aroonosc | 94,614 | 0.89x | --- |
    | atr | 14,286 | 1.53x | 4707.47x |
    | bbands | 86,052 | 0.47x | 922.21x |
    | ccfisher | 306,878 | 2.99x | --- |
    | cci | 124,137 | 2.56x | 1066.96x |
    | chaikinmf | 26,783 | 1.52x | 3379.68x |
    | chandelierexit | 117,227 | 0.62x | 808.91x |
    | cmo | 15,978 | 1.87x | --- |
    | cvi | 18,234 | 1.25x | --- |
    | cybercycle | 26,055 | 2.66x | --- |
    | dema | 13,371 | 1.92x | 4374.80x |
    | di | 50,425 | 1.53x | --- |
    | dm | 46,149 | 0.84x | --- |
    | donchianchannel | 166,142 | 0.34x | 453.30x |
    | dpo | 18,204 | 0.68x | 4167.70x |
    | dx | 19,708 | 2.17x | --- |
    | ef | 17,180 | 1.27x | --- |
    | elderray | 49,393 | 0.60x | 1312.39x |
    | ema | 12,979 | 1.68x | 1007.85x |
    | fisher | 284,345 | 0.90x | 474.65x |
    | fosc | 17,443 | 2.03x | --- |
    | highpass | 14,396 | 1.51x | --- |
    | hilberttransform | 58,154 | 1.03x | --- |
    | hma | 27,256 | 1.37x | 2709.30x |
    | ichimoku | 438,802 | 0.71x | --- |
    | kama | 18,937 | 1.61x | 6162.80x |
    | keltnerchannel | 85,846 | 0.39x | --- |
    | kvo | 20,781 | 1.97x | --- |
    | linreg | 15,993 | 1.97x | --- |
    | macd | 83,577 | 0.39x | --- |
    | mama | 287,259 | 3.08x | --- |
    | mass | 17,311 | 1.54x | --- |
    | max | 40,578 | 0.66x | 1247.65x |
    | md | 63,277 | 1.01x | --- |
    | mfi | 20,631 | 1.89x | 4747.94x |
    | min | 55,331 | 0.53x | 801.90x |
    | mom | 5,163 | 1.14x | --- |
    | msw | 147,098 | 3.57x | --- |
    | natr | 15,201 | 1.65x | --- |
    | pivotpoint | 2,149 | 1.37x | --- |
    | ppo | 14,594 | 1.69x | 6231.71x |
    | psar | 56,346 | 0.82x | --- |
    | qstick | 12,204 | 1.06x | 5128.06x |
    | roc | 13,720 | 0.88x | 1064.55x |
    | rocr | 13,776 | 0.89x | --- |
    | roofingfilter | 14,646 | 3.11x | --- |
    | rsi | 14,751 | 1.63x | 6047.81x |
    | sma | 14,127 | 0.87x | 3087.15x |
    | smaenvelope | 80,564 | 0.43x | 994.29x |
    | stddev | 17,092 | 1.82x | 1985.17x |
    | stoch | 124,809 | 0.73x | 587.16x |
    | stochrsi | 132,118 | 0.62x | --- |
    | supersmoother | 15,947 | 2.85x | --- |
    | supertrend | 24,497 | 2.20x | 5734.02x |
    | tema | 13,375 | 2.22x | 7854.61x |
    | trendmode | 228,567 | 3.87x | --- |
    | trima | 18,514 | 1.30x | 3721.27x |
    | trix | 14,247 | 2.07x | 5908.50x |
    | trvi | 19,611 | 1.35x | --- |
    | tsf | 13,819 | 2.18x | --- |
    | ultosc | 40,488 | 1.59x | 3350.53x |
    | vhf | 109,841 | 0.52x | --- |
    | vidya | 41,392 | 1.23x | --- |
    | volatility | 24,190 | 2.12x | --- |
    | vortex | 30,504 | 1.19x | --- |
    | vosc | 15,318 | 1.42x | --- |
    | vwma | 18,487 | 1.18x | 4262.61x |
    | wilders | 12,066 | 1.80x | 636.36x |
    | willr | 98,145 | 0.63x | 1002.19x |
    | wma | 14,953 | 1.45x | 1228.89x |
    | zlema | 14,277 | 1.86x | --- |

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
