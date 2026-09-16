# SIMD by_options: 4 Option Sets Simultaneously

Processes 1 asset with 4 different option configurations in a single SIMD pass. There is no TA-Lib equivalent for this mode - the comparison baseline is 4 independent sequential calls (C_tulip, native Rust, or a binding's own sequential call, depending on the column). All times are nanoseconds (ns); lower is better.

- **Speedup vs Rust / vs binding / vs C** - `4x Sequential (ns) / SIMD 4-Options (ns)`
- **Best / Worst** - speedup range across the different option combinations tested

=== "vs C"

    Reference: **C_tulip**, called once per option-set and summed to give the equivalent
    4-option-set sequential cost for the same asset. There is no TA-Lib equivalent for this mode.

    | Indicator | SIMD 4-Options (ns) | Speedup vs Rust | Speedup vs C | Best | Worst |
    |-----------|--------------------:|-----------------:|-------------:|-----:|------:|
    | adosc | 10,722 | 2.11x | 3.50x | 3.36x | 3.67x |
    | adx | 16,822 | 2.55x | 3.95x | 3.73x | 4.15x |
    | adxr | 27,674 | 2.16x | 2.60x | 2.40x | 2.78x |
    | apo | 9,593 | 2.04x | 3.78x | 3.60x | 4.02x |
    | aroon | 92,891 | 0.81x | 3.52x | 3.43x | 3.63x |
    | aroonosc | 91,194 | 0.86x | 3.59x | 3.48x | 3.74x |
    | atr | 10,056 | 1.92x | 4.39x | 4.08x | 4.60x |
    | bbands | 22,486 | 1.17x | 1.60x | 1.56x | 1.66x |
    | cci | 119,243 | 2.63x | 2.52x | 2.51x | 2.53x |
    | cmo | 12,529 | 2.01x | 7.80x | 7.39x | 8.04x |
    | cvi | 15,153 | 1.38x | 3.86x | 3.82x | 3.90x |
    | dema | 9,033 | 2.82x | 3.02x | 2.72x | 3.16x |
    | di | 19,688 | 3.48x | 2.15x | 2.02x | 2.27x |
    | dm | 17,988 | 2.22x | 1.57x | 1.38x | 1.67x |
    | donchianchannel | 119,596 | 0.44x | 1.94x | 1.89x | 1.99x |
    | dpo | 13,748 | 0.79x | 0.82x | 0.77x | 0.86x |
    | dx | 14,746 | 2.56x | 1.92x | 1.86x | 1.95x |
    | elderray | 16,604 | 1.47x | 3.73x | 3.49x | 3.87x |
    | ema | 8,669 | 2.21x | 5.09x | 4.89x | 5.29x |
    | fisher | 187,945 | 1.04x | 2.87x | 2.81x | 2.96x |
    | fosc | 20,039 | 1.62x | 1.96x | 1.95x | 1.96x |
    | hma | 28,031 | 1.33x | 1.40x | 1.39x | 1.41x |
    | kama | 12,833 | 2.32x | 2.73x | 2.72x | 2.74x |
    | kvo | 18,065 | 2.00x | 2.07x | 2.05x | 2.09x |
    | linreg | 11,280 | 2.74x | 2.43x | 0.90x | 3.36x |
    | macd | 23,503 | 1.08x | 2.39x | 2.16x | 2.54x |
    | mass | 12,882 | 2.03x | 3.82x | 3.70x | 4.00x |
    | max | 27,480 | 0.76x | 4.44x | 4.22x | 4.72x |
    | md | 55,496 | 1.00x | 1.07x | 1.07x | 1.08x |
    | mfi | 18,037 | 1.77x | 2.74x | 2.70x | 2.76x |
    | min | 48,048 | 0.59x | 4.73x | 4.54x | 4.92x |
    | mom | 3,656 | 0.98x | 1.61x | 1.57x | 1.65x |
    | msw | 147,040 | 3.54x | 40.10x | 40.10x | 40.10x |
    | natr | 11,240 | 1.77x | 3.92x | 3.69x | 4.09x |
    | ppo | 11,194 | 1.81x | 3.46x | 3.38x | 3.57x |
    | psar | 62,801 | 0.56x | 0.58x | 0.57x | 0.59x |
    | qstick | 10,267 | 1.00x | 1.19x | 1.18x | 1.19x |
    | roc | 10,649 | 0.92x | 1.04x | 0.97x | 1.07x |
    | rocr | 11,217 | 0.88x | 0.99x | 0.89x | 1.06x |
    | rsi | 10,737 | 1.86x | 4.13x | 4.08x | 4.16x |
    | sma | 9,724 | 1.05x | 1.13x | 1.12x | 1.13x |
    | stddev | 15,431 | 0.99x | 1.93x | 1.91x | 1.94x |
    | stoch | 118,833 | 0.69x | 3.19x | 3.06x | 3.37x |
    | stochrsi | 117,900 | 0.65x | 1.47x | 1.44x | 1.50x |
    | tema | 9,594 | 2.86x | 2.92x | 2.54x | 3.26x |
    | trima | 16,656 | 1.35x | 1.80x | 1.79x | 1.80x |
    | trix | 11,661 | 2.21x | 3.81x | 3.43x | 4.04x |
    | tsf | 11,371 | 2.34x | 3.30x | 2.97x | 3.48x |
    | ultosc | 67,072 | 0.93x | 1.11x | 1.07x | 1.14x |
    | vhf | 97,967 | 0.56x | 3.20x | 3.05x | 3.36x |
    | vidya | 38,955 | 1.25x | 1.99x | 1.98x | 1.99x |
    | volatility | 22,330 | 1.22x | 3.26x | 3.26x | 3.27x |
    | vosc | 13,701 | 1.06x | 1.50x | 1.50x | 1.50x |
    | vwma | 16,799 | 0.83x | 1.22x | 1.21x | 1.22x |
    | wilders | 10,519 | 1.83x | 4.21x | 3.71x | 4.39x |
    | willr | 91,740 | 0.75x | 3.76x | 3.63x | 3.91x |
    | wma | 10,150 | 1.95x | 3.44x | 3.25x | 3.73x |
    | zlema | 10,147 | 2.37x | 3.43x | 3.20x | 3.67x |

=== "Python Binding"

    Competitor: **tulip_rs_python** PyO3 binding.
    See [Python Binding](../python.md) for setup and how to run.

    | Indicator | SIMD 4-Options (ns) | Speedup vs Rust | Speedup vs tulip_rs_python | Best | Worst |
    |-----------|--------------------:|-----------------:|----------------------------:|-----:|------:|
    | adosc | 19,742 | 2.11x | 1.41x | 1.01x | 1.82x |
    | adx | 39,774 | 2.55x | 2.50x | 1.35x | 3.45x |
    | adxr | 40,260 | 2.16x | 2.32x | 1.95x | 2.99x |
    | apo | 18,772 | 2.04x | 1.56x | 1.11x | 2.46x |
    | aroon | 96,641 | 0.81x | 1.47x | 1.05x | 1.99x |
    | aroonosc | 100,194 | 0.86x | 1.34x | 1.08x | 1.85x |
    | atr | 19,303 | 1.92x | 2.92x | 2.03x | 4.08x |
    | bbands | 41,312 | 1.17x | 1.44x | 0.98x | 1.84x |
    | ccfisher | 291,025 | 3.26x | 3.15x | 3.13x | 3.17x |
    | cci | 123,824 | 2.63x | 2.99x | 2.72x | 3.20x |
    | chaikinmf | 29,182 | 1.42x | 1.47x | 1.04x | 1.75x |
    | chandelierexit | 86,147 | 0.61x | 1.56x | 1.46x | 1.65x |
    | cmo | 14,122 | 2.01x | 2.40x | 1.97x | 2.86x |
    | cvi | 20,292 | 1.38x | 1.49x | 1.13x | 1.67x |
    | cybercycle | 21,650 | 3.15x | 3.19x | 3.15x | 3.25x |
    | dema | 20,070 | 2.82x | 1.59x | 1.16x | 1.90x |
    | di | 33,902 | 3.48x | 5.24x | 4.03x | 7.64x |
    | dm | 30,040 | 2.22x | 3.40x | 2.68x | 3.90x |
    | donchianchannel | 103,600 | 0.44x | 0.65x | 0.53x | 0.77x |
    | dpo | 20,024 | 0.79x | 0.66x | 0.43x | 0.86x |
    | dx | 23,061 | 2.56x | 5.28x | 4.14x | 6.67x |
    | ef | 15,017 | 1.39x | 1.70x | 1.64x | 1.81x |
    | elderray | 18,924 | 1.47x | 1.83x | 1.48x | 2.24x |
    | ema | 15,428 | 2.21x | 1.58x | 1.35x | 1.93x |
    | fisher | 248,727 | 1.04x | 1.31x | 1.19x | 1.41x |
    | fosc | 14,083 | 1.62x | 2.47x | 2.45x | 2.48x |
    | highpass | 11,095 | 1.81x | 2.35x | 1.87x | 2.94x |
    | hilberttransform | 34,764 | 2.31x | 2.24x | 1.78x | 2.65x |
    | hma | 34,730 | 1.33x | 2.41x | 1.52x | 3.14x |
    | ichimoku | 334,842 | 0.77x | 1.13x | 1.04x | 1.20x |
    | kama | 22,076 | 2.32x | 4.73x | 1.08x | 7.31x |
    | keltnerchannel | 38,011 | 0.89x | 2.27x | 1.54x | 2.76x |
    | kvo | 26,728 | 2.00x | 3.86x | 2.83x | 4.55x |
    | linreg | 24,519 | 2.74x | 4.22x | 2.19x | 6.22x |
    | macd | 57,972 | 1.08x | 1.71x | 1.59x | 1.86x |
    | mama | 258,057 | 3.50x | 3.47x | 3.42x | 3.52x |
    | mass | 24,621 | 2.03x | 1.24x | 1.08x | 1.44x |
    | max | 27,766 | 0.76x | 1.24x | 0.75x | 2.19x |
    | md | 62,561 | 1.00x | 2.52x | 2.01x | 3.01x |
    | mfi | 35,655 | 1.77x | 1.23x | 1.10x | 1.55x |
    | min | 57,113 | 0.59x | 0.86x | 0.62x | 1.28x |
    | mom | 5,209 | 0.98x | 1.04x | 0.88x | 1.23x |
    | msw | 147,230 | 3.54x | 3.61x | 3.56x | 3.67x |
    | natr | 12,467 | 1.77x | 5.29x | 2.61x | 7.76x |
    | ppo | 24,492 | 1.81x | 1.70x | 0.76x | 2.03x |
    | psar | 77,464 | 0.56x | 1.50x | 1.30x | 1.63x |
    | qstick | 23,994 | 1.00x | 0.88x | 0.38x | 1.38x |
    | roc | 22,088 | 0.92x | 1.40x | 0.84x | 1.93x |
    | rocr | 14,720 | 0.88x | 2.51x | 1.49x | 3.58x |
    | roofingfilter | 25,027 | 3.52x | 2.59x | 1.53x | 3.60x |
    | rsi | 17,873 | 1.86x | 1.70x | 1.29x | 1.86x |
    | sma | 19,112 | 1.05x | 0.95x | 0.72x | 1.05x |
    | smaenvelope | 40,112 | 1.23x | 1.07x | 0.77x | 1.57x |
    | stddev | 35,395 | 0.99x | 1.26x | 0.43x | 1.87x |
    | stoch | 107,257 | 0.69x | 1.17x | 1.07x | 1.34x |
    | stochrsi | 125,647 | 0.65x | 1.27x | 0.92x | 1.51x |
    | supersmoother | 14,821 | 3.12x | 3.08x | 2.97x | 3.25x |
    | supertrend | 26,229 | 2.19x | 4.60x | 3.52x | 5.41x |
    | tema | 16,238 | 2.86x | 2.30x | 1.68x | 3.44x |
    | trendmode | 227,861 | 3.85x | 3.87x | 3.84x | 3.93x |
    | trima | 26,507 | 1.35x | 1.26x | 0.54x | 1.82x |
    | trix | 14,334 | 2.21x | 5.14x | 3.25x | 6.14x |
    | trvi | 37,233 | 1.38x | 0.79x | 0.42x | 1.12x |
    | tsf | 19,684 | 2.34x | 3.46x | 1.87x | 4.78x |
    | ultosc | 43,766 | 0.93x | 2.00x | 1.70x | 2.27x |
    | vhf | 105,627 | 0.56x | 0.63x | 0.52x | 0.78x |
    | vidya | 40,978 | 1.25x | 2.06x | 1.40x | 2.69x |
    | volatility | 50,107 | 1.22x | 1.11x | 0.99x | 1.42x |
    | vortex | 46,229 | 1.26x | 1.82x | 1.28x | 2.26x |
    | vosc | 22,283 | 1.06x | 1.25x | 0.93x | 1.65x |
    | vwma | 30,400 | 0.83x | 1.42x | 0.51x | 2.55x |
    | wilders | 14,875 | 1.83x | 1.63x | 1.46x | 1.79x |
    | willr | 92,275 | 0.75x | 0.93x | 0.89x | 0.98x |
    | wma | 11,038 | 1.95x | 2.21x | 1.86x | 2.54x |
    | zlema | 11,191 | 2.37x | 3.14x | 2.64x | 3.52x |

=== "Node Binding"

    Competitor: **tulip_rs_node** napi-rs binding.
    See [Node Binding](../node.md) for setup and how to run.

    | Indicator | SIMD 4-Options (ns) | Speedup vs Rust | Speedup vs tulip_rs_node | Best | Worst |
    |-----------|--------------------:|-----------------:|--------------------------:|-----:|------:|
    | adosc | 31,289 | 2.11x | 1.22x | 1.11x | 1.42x |
    | adx | 33,165 | 2.55x | 1.68x | 1.56x | 1.74x |
    | adxr | 46,864 | 2.16x | 1.78x | 1.68x | 1.88x |
    | apo | 31,389 | 2.04x | 1.03x | 0.92x | 1.20x |
    | aroon | 143,798 | 0.81x | 0.80x | 0.78x | 0.82x |
    | aroonosc | 135,741 | 0.86x | 0.75x | 0.74x | 0.76x |
    | atr | 30,653 | 1.92x | 1.15x | 1.03x | 1.31x |
    | bbands | 67,790 | 1.17x | 0.95x | 0.84x | 1.12x |
    | ccfisher | 298,531 | 3.26x | 3.14x | 3.11x | 3.17x |
    | cci | 151,568 | 2.63x | 2.25x | 2.22x | 2.27x |
    | chaikinmf | 33,571 | 1.42x | 1.54x | 1.45x | 1.78x |
    | chandelierexit | 147,018 | 0.61x | 0.72x | 0.68x | 0.75x |
    | cmo | 27,107 | 2.01x | 1.44x | 1.29x | 1.90x |
    | cvi | 31,639 | 1.38x | 1.07x | 1.05x | 1.11x |
    | cybercycle | 33,380 | 3.15x | 2.27x | 2.18x | 2.48x |
    | dema | 23,610 | 2.82x | 1.35x | 1.28x | 1.42x |
    | di | 52,006 | 3.48x | 2.13x | 1.65x | 2.48x |
    | dm | 45,208 | 2.22x | 1.54x | 1.23x | 1.94x |
    | donchianchannel | 133,390 | 0.44x | 0.62x | 0.58x | 0.64x |
    | dpo | 26,686 | 0.79x | 0.86x | 0.79x | 0.99x |
    | dx | 35,546 | 2.56x | 1.39x | 1.34x | 1.45x |
    | ef | 27,045 | 1.39x | 1.02x | 1.00x | 1.05x |
    | elderray | 53,494 | 1.47x | 1.27x | 0.97x | 1.47x |
    | ema | 27,026 | 2.21x | 1.15x | 1.03x | 1.27x |
    | fisher | 274,736 | 1.04x | 1.07x | 1.05x | 1.09x |
    | fosc | 29,044 | 1.62x | 1.54x | 1.38x | 1.76x |
    | highpass | 27,049 | 1.81x | 1.00x | 0.99x | 1.01x |
    | hilberttransform | 56,191 | 2.31x | 1.64x | 1.58x | 1.76x |
    | hma | 43,792 | 1.33x | 0.98x | 0.97x | 0.99x |
    | ichimoku | 383,447 | 0.77x | 0.92x | 0.91x | 0.93x |
    | kama | 24,557 | 2.32x | 1.53x | 1.45x | 1.70x |
    | keltnerchannel | 138,170 | 0.89x | 0.84x | 0.30x | 1.38x |
    | kvo | 34,192 | 2.00x | 1.43x | 1.38x | 1.54x |
    | linreg | 27,691 | 2.74x | 1.39x | 1.34x | 1.43x |
    | macd | 53,881 | 1.08x | 1.34x | 1.27x | 1.44x |
    | mama | 274,763 | 3.50x | 3.34x | 3.31x | 3.38x |
    | mass | 36,716 | 2.03x | 1.27x | 0.53x | 2.04x |
    | max | 43,414 | 0.76x | 1.01x | 0.92x | 1.13x |
    | md | 70,527 | 1.00x | 1.03x | 0.96x | 1.20x |
    | mfi | 39,266 | 1.77x | 1.34x | 1.20x | 1.53x |
    | min | 67,594 | 0.59x | 0.90x | 0.85x | 0.94x |
    | mom | 22,423 | 0.98x | 0.88x | 0.85x | 0.92x |
    | msw | 164,844 | 3.54x | 3.39x | 3.35x | 3.44x |
    | natr | 30,122 | 1.77x | 1.15x | 1.08x | 1.22x |
    | ppo | 32,358 | 1.81x | 1.04x | 0.99x | 1.11x |
    | psar | 76,669 | 0.56x | 0.68x | 0.66x | 0.70x |
    | qstick | 25,549 | 1.00x | 0.97x | 0.96x | 0.99x |
    | roc | 25,325 | 0.92x | 1.02x | 0.84x | 1.46x |
    | rocr | 27,927 | 0.88x | 0.75x | 0.69x | 0.79x |
    | roofingfilter | 30,756 | 3.52x | 1.65x | 1.60x | 1.70x |
    | rsi | 24,472 | 1.86x | 1.28x | 1.14x | 1.44x |
    | sma | 27,305 | 1.05x | 1.06x | 0.86x | 1.19x |
    | smaenvelope | 75,480 | 1.23x | 0.93x | 0.87x | 0.98x |
    | stddev | 28,218 | 0.99x | 1.34x | 1.30x | 1.38x |
    | stoch | 174,305 | 0.69x | 0.81x | 0.78x | 0.82x |
    | stochrsi | 139,586 | 0.65x | 0.70x | 0.68x | 0.72x |
    | supersmoother | 27,864 | 3.12x | 1.92x | 1.82x | 2.18x |
    | supertrend | 36,431 | 2.19x | 1.73x | 1.66x | 1.79x |
    | tema | 30,503 | 2.86x | 1.21x | 1.20x | 1.23x |
    | trendmode | 248,410 | 3.85x | 3.60x | 3.47x | 3.66x |
    | trima | 28,644 | 1.35x | 1.08x | 1.06x | 1.10x |
    | trix | 29,436 | 2.21x | 1.39x | 1.23x | 1.68x |
    | trvi | 37,439 | 1.38x | 1.05x | 0.96x | 1.23x |
    | tsf | 26,792 | 2.34x | 1.45x | 1.41x | 1.51x |
    | ultosc | 55,289 | 0.93x | 1.41x | 1.31x | 1.69x |
    | vhf | 140,792 | 0.56x | 0.41x | 0.40x | 0.41x |
    | vidya | 64,413 | 1.25x | 1.00x | 0.63x | 1.16x |
    | volatility | 59,719 | 1.22x | 0.94x | 0.91x | 0.96x |
    | vortex | 48,552 | 1.26x | 1.70x | 1.62x | 1.76x |
    | vosc | 29,054 | 1.06x | 1.03x | 0.97x | 1.09x |
    | vwma | 27,043 | 0.83x | 1.33x | 1.08x | 1.98x |
    | wilders | 24,369 | 1.83x | 1.24x | 1.13x | 1.44x |
    | willr | 117,239 | 0.75x | 0.68x | 0.64x | 0.72x |
    | wma | 22,974 | 1.95x | 1.73x | 1.45x | 2.05x |
    | zlema | 21,404 | 2.37x | 1.44x | 1.42x | 1.47x |

=== "C Binding"

    Competitor: **tulip_rs_ffi_c** — the `extern "C"` FFI binding's own SIMD pass.
    See [C Binding](../c.md) for setup and how to run.

    | Indicator | SIMD 4-Options (ns) | Speedup vs Rust | Speedup vs tulip_rs_ffi_c | Best | Worst |
    |-----------|--------------------:|-----------------:|----------------------------:|-----:|------:|
    | adosc | 14,753 | 2.11x | 1.82x | 1.81x | 1.84x |
    | adx | 18,596 | 2.55x | 6.86x | 5.19x | 7.83x |
    | adxr | 27,603 | 2.16x | 5.06x | 4.21x | 5.62x |
    | apo | 9,995 | 2.04x | 1.99x | 1.88x | 2.06x |
    | aroon | 117,573 | 0.81x | 1.14x | 1.12x | 1.16x |
    | aroonosc | 119,881 | 0.86x | 1.22x | 1.10x | 1.27x |
    | atr | 10,254 | 1.92x | 2.22x | 2.03x | 2.75x |
    | bbands | 23,567 | 1.17x | 1.31x | 1.28x | 1.33x |
    | ccfisher | 280,412 | 3.26x | 3.28x | 3.24x | 3.30x |
    | cci | 121,745 | 2.63x | 2.57x | 2.45x | 2.62x |
    | chaikinmf | 21,766 | 1.42x | 1.79x | 1.61x | 1.85x |
    | chandelierexit | 118,369 | 0.61x | 1.28x | 1.18x | 1.33x |
    | cmo | 12,929 | 2.01x | 2.16x | 1.88x | 2.25x |
    | cvi | 15,315 | 1.38x | 1.30x | 1.24x | 1.33x |
    | cybercycle | 20,399 | 3.15x | 3.31x | 3.26x | 3.35x |
    | dema | 10,651 | 2.82x | 2.21x | 2.09x | 2.26x |
    | di | 20,077 | 3.48x | 6.08x | 5.62x | 6.54x |
    | dm | 17,688 | 2.22x | 6.03x | 5.50x | 6.94x |
    | donchianchannel | 169,085 | 0.44x | 0.91x | 0.86x | 0.94x |
    | dpo | 13,581 | 0.79x | 0.84x | 0.81x | 0.86x |
    | dx | 16,333 | 2.56x | 8.80x | 8.19x | 9.71x |
    | ef | 11,924 | 1.39x | 1.64x | 1.45x | 1.71x |
    | elderray | 16,990 | 1.47x | 1.46x | 1.38x | 1.51x |
    | ema | 9,324 | 2.21x | 2.10x | 1.95x | 2.18x |
    | fisher | 256,703 | 1.04x | 1.21x | 1.19x | 1.22x |
    | fosc | 12,317 | 1.62x | 2.73x | 2.60x | 2.80x |
    | highpass | 10,348 | 1.81x | 1.89x | 1.68x | 1.99x |
    | hma | 24,106 | 1.33x | 1.48x | 1.38x | 1.55x |
    | ichimoku | 425,180 | 0.77x | 1.02x | 0.99x | 1.03x |
    | kama | 13,610 | 2.32x | 2.09x | 1.93x | 2.17x |
    | keltnerchannel | 44,696 | 0.89x | 0.61x | 0.60x | 0.64x |
    | kvo | 18,816 | 2.00x | 3.58x | 3.44x | 3.70x |
    | linreg | 11,707 | 2.74x | 2.52x | 2.17x | 2.66x |
    | macd | 44,262 | 1.08x | 0.58x | 0.57x | 0.58x |
    | mama | 256,356 | 3.50x | 3.46x | 3.44x | 3.50x |
    | mass | 12,945 | 2.03x | 1.98x | 1.97x | 1.99x |
    | max | 66,724 | 0.76x | 1.00x | 0.96x | 1.03x |
    | md | 61,729 | 1.00x | 1.01x | 1.00x | 1.03x |
    | mfi | 19,390 | 1.77x | 1.86x | 1.78x | 1.91x |
    | min | 80,974 | 0.59x | 0.91x | 0.88x | 0.93x |
    | mom | 4,067 | 0.98x | 0.97x | 0.84x | 1.09x |
    | msw | 143,297 | 3.54x | 4.10x | 4.06x | 4.13x |
    | natr | 11,884 | 1.77x | 2.00x | 1.81x | 2.08x |
    | pivotpoint | 544 | --- | 1.08x | 0.93x | 1.28x |
    | ppo | 11,301 | 1.81x | 1.99x | 1.87x | 2.04x |
    | psar | 80,370 | 0.56x | 1.01x | 1.01x | 1.02x |
    | qstick | 10,736 | 1.00x | 1.00x | 0.99x | 1.02x |
    | roc | 11,250 | 0.92x | 0.89x | 0.78x | 0.93x |
    | rocr | 10,968 | 0.88x | 0.90x | 0.85x | 0.92x |
    | roofingfilter | 12,576 | 3.52x | 3.44x | 3.24x | 3.53x |
    | rsi | 11,061 | 1.86x | 1.98x | 1.87x | 2.02x |
    | sma | 10,519 | 1.05x | 1.00x | 0.87x | 1.04x |
    | smaenvelope | 23,750 | 1.23x | 1.16x | 1.08x | 1.20x |
    | stddev | 15,810 | 0.99x | 1.83x | 1.72x | 1.87x |
    | stoch | 140,013 | 0.69x | 1.03x | 1.01x | 1.04x |
    | stochrsi | 149,597 | 0.65x | 1.37x | 1.31x | 1.41x |
    | supersmoother | 13,267 | 3.12x | 3.27x | 3.08x | 3.43x |
    | supertrend | 22,966 | 2.19x | 2.65x | 2.57x | 2.72x |
    | tema | 10,352 | 2.86x | 2.69x | 2.43x | 2.99x |
    | trendmode | 227,135 | 3.85x | 3.89x | 3.87x | 3.92x |
    | trima | 17,490 | 1.35x | 1.27x | 1.15x | 1.31x |
    | trix | 11,768 | 2.21x | 2.33x | 2.06x | 2.42x |
    | trvi | 16,734 | 1.38x | 1.46x | 1.26x | 1.65x |
    | tsf | 11,571 | 2.34x | 2.43x | 2.18x | 2.51x |
    | ultosc | 39,458 | 0.93x | 1.58x | 1.55x | 1.61x |
    | vhf | 136,822 | 0.56x | 1.00x | 0.96x | 1.02x |
    | vidya | 40,576 | 1.25x | 1.21x | 1.20x | 1.21x |
    | volatility | 22,964 | 1.22x | 2.13x | 1.99x | 2.20x |
    | vortex | 25,481 | 1.26x | 1.21x | 1.11x | 1.25x |
    | vosc | 14,439 | 1.06x | 1.42x | 1.37x | 1.45x |
    | vwma | 17,290 | 0.83x | 1.15x | 1.10x | 1.16x |
    | wilders | 9,090 | 1.83x | 2.18x | 2.05x | 2.33x |
    | willr | 121,231 | 0.75x | 1.03x | 0.94x | 1.07x |
    | wma | 10,085 | 1.95x | 1.94x | 1.75x | 2.01x |
    | zlema | 9,915 | 2.37x | 2.44x | 2.23x | 2.52x |

=== "Go Binding"

    Competitor: **tulip_rs_go** — the cgo binding's own SIMD pass.
    See [Go Binding](../go.md) for setup and how to run.

    | Indicator | SIMD 4-Options (ns) | Speedup vs Rust | Speedup vs tulip_rs_go | Best | Worst |
    |-----------|--------------------:|-----------------:|-------------------------:|-----:|------:|
    | adosc | 17,964 | 2.11x | 1.95x | 1.84x | 2.07x |
    | adx | 32,353 | 2.55x | 2.33x | 1.57x | 3.52x |
    | adxr | 34,084 | 2.16x | 2.27x | 1.13x | 3.15x |
    | apo | 29,802 | 2.04x | 1.16x | 0.99x | 1.41x |
    | aroon | 132,002 | 0.81x | 1.08x | 0.90x | 1.25x |
    | aroonosc | 103,238 | 0.86x | 0.84x | 0.79x | 0.87x |
    | atr | 26,427 | 1.92x | 1.47x | 1.07x | 1.76x |
    | bbands | 89,693 | 1.17x | 0.69x | 0.46x | 1.12x |
    | ccfisher | 324,130 | 3.26x | 2.89x | 2.80x | 2.93x |
    | cci | 135,624 | 2.63x | 2.91x | 2.37x | 3.29x |
    | chaikinmf | 37,326 | 1.42x | 2.57x | 2.02x | 2.95x |
    | chandelierexit | 116,521 | 0.61x | 1.73x | 1.45x | 1.99x |
    | cmo | 20,618 | 2.01x | 2.66x | 2.14x | 3.13x |
    | cvi | 22,682 | 1.38x | 1.57x | 1.13x | 2.33x |
    | cybercycle | 34,857 | 3.15x | 2.51x | 1.99x | 2.97x |
    | dema | 20,245 | 2.82x | 3.30x | 2.38x | 5.10x |
    | di | 70,500 | 3.48x | 1.16x | 0.93x | 1.38x |
    | dm | 60,236 | 2.22x | 0.92x | 0.68x | 1.27x |
    | donchianchannel | 188,237 | 0.44x | 0.81x | 0.68x | 1.00x |
    | dpo | 32,524 | 0.79x | 0.53x | 0.47x | 0.67x |
    | dx | 32,093 | 2.56x | 1.39x | 1.24x | 1.60x |
    | ef | 27,474 | 1.39x | 1.25x | 0.90x | 1.71x |
    | elderray | 67,786 | 1.47x | 0.52x | 0.45x | 0.64x |
    | ema | 21,233 | 2.21x | 2.18x | 1.51x | 3.65x |
    | fisher | 308,210 | 1.04x | 0.94x | 0.85x | 1.05x |
    | fosc | 14,985 | 1.62x | 2.65x | 2.32x | 3.42x |
    | highpass | 14,022 | 1.81x | 1.62x | 1.22x | 2.03x |
    | hilberttransform | 69,608 | 2.31x | 0.97x | 0.90x | 1.01x |
    | hma | 30,843 | 1.33x | 1.70x | 1.33x | 2.39x |
    | ichimoku | 452,276 | 0.77x | 0.84x | 0.78x | 0.87x |
    | kama | 21,101 | 2.32x | 2.72x | 1.45x | 3.38x |
    | keltnerchannel | 99,059 | 0.89x | 0.42x | 0.36x | 0.50x |
    | kvo | 26,062 | 2.00x | 1.93x | 1.74x | 2.31x |
    | linreg | 13,681 | 2.74x | 2.44x | 1.80x | 2.84x |
    | macd | 97,598 | 1.08x | 0.43x | 0.28x | 0.60x |
    | mama | 303,594 | 3.50x | 3.00x | 2.90x | 3.10x |
    | mass | 28,908 | 2.03x | 1.02x | 0.79x | 1.13x |
    | max | 72,327 | 0.76x | 0.52x | 0.32x | 0.69x |
    | md | 65,341 | 1.00x | 1.00x | 0.92x | 1.04x |
    | mfi | 24,726 | 1.77x | 2.94x | 1.74x | 5.69x |
    | min | 57,166 | 0.59x | 0.96x | 0.68x | 1.57x |
    | mom | 5,655 | 0.98x | 1.64x | 1.44x | 1.75x |
    | msw | 179,563 | 3.54x | 3.15x | 2.73x | 3.83x |
    | natr | 27,174 | 1.77x | 1.55x | 0.96x | 2.70x |
    | pivotpoint | 2,363 | --- | 2.00x | 1.61x | 2.58x |
    | ppo | 13,897 | 1.81x | 5.04x | 3.63x | 5.99x |
    | psar | 61,452 | 0.56x | 0.79x | 0.67x | 0.85x |
    | qstick | 13,174 | 1.00x | 3.23x | 1.83x | 5.27x |
    | roc | 17,827 | 0.92x | 1.00x | 0.70x | 1.37x |
    | rocr | 17,391 | 0.88x | 0.73x | 0.61x | 0.84x |
    | roofingfilter | 22,293 | 3.52x | 2.30x | 1.90x | 3.26x |
    | rsi | 35,725 | 1.86x | 1.83x | 0.63x | 3.21x |
    | sma | 14,865 | 1.05x | 1.63x | 1.14x | 2.28x |
    | smaenvelope | 96,779 | 1.23x | 0.44x | 0.32x | 0.52x |
    | stddev | 26,980 | 0.99x | 2.90x | 1.05x | 3.89x |
    | stoch | 143,807 | 0.69x | 0.96x | 0.70x | 1.10x |
    | stochrsi | 121,726 | 0.65x | 0.70x | 0.69x | 0.72x |
    | supersmoother | 15,637 | 3.12x | 4.16x | 3.53x | 5.11x |
    | supertrend | 25,275 | 2.19x | 4.38x | 3.06x | 5.97x |
    | tema | 16,112 | 2.86x | 2.50x | 2.18x | 2.87x |
    | trendmode | 230,819 | 3.85x | 3.92x | 3.84x | 3.97x |
    | trima | 18,405 | 1.35x | 3.05x | 2.49x | 3.53x |
    | trix | 12,775 | 2.21x | 4.69x | 3.20x | 6.58x |
    | trvi | 22,020 | 1.38x | 1.61x | 1.03x | 2.45x |
    | tsf | 13,299 | 2.34x | 2.63x | 2.20x | 3.50x |

??? success "Notable results - by_options"

    **53 of 75 indicators (71%) show a SIMD speedup over 4x sequential Rust.**

    | Category | Indicator | Speedup |
    |----------|-----------|:-------:|
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
