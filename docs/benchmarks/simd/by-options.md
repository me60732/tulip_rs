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
    | cci | 122,803 | 2.56x | 2.43x | 2.42x | 2.44x |
    | cmo | 12,529 | 2.01x | 7.80x | 7.39x | 8.04x |
    | cvi | 15,153 | 1.38x | 3.86x | 3.82x | 3.90x |
    | dema | 9,033 | 2.82x | 3.02x | 2.72x | 3.16x |
    | di | 19,688 | 3.48x | 2.15x | 2.02x | 2.27x |
    | dm | 17,988 | 2.22x | 1.57x | 1.38x | 1.67x |
    | donchianchannel | 119,596 | 0.44x | 1.94x | 1.89x | 1.99x |
    | dpo | 13,748 | 0.79x | 0.82x | 0.77x | 0.86x |
    | dx | 14,746 | 2.56x | 1.92x | 1.86x | 1.95x |
    | elderray | 16,604 | 1.47x | 3.73x | 3.49x | 3.87x |
    | ema | 10,142 | 1.90x | 4.38x | 4.04x | 4.77x |
    | fisher | 184,963 | 1.06x | 2.92x | 2.89x | 2.99x |
    | fosc | 20,049 | 1.62x | 1.95x | 1.95x | 1.96x |
    | hma | 28,031 | 1.33x | 1.40x | 1.39x | 1.41x |
    | kama | 12,833 | 2.32x | 2.73x | 2.72x | 2.74x |
    | kvo | 18,065 | 2.00x | 2.07x | 2.05x | 2.09x |
    | linreg | 11,269 | 2.36x | 3.33x | 3.14x | 3.50x |
    | macd | 23,503 | 1.08x | 2.39x | 2.16x | 2.54x |
    | mass | 12,882 | 2.03x | 3.82x | 3.70x | 4.00x |
    | max | 27,480 | 0.76x | 4.44x | 4.22x | 4.72x |
    | md | 55,496 | 1.00x | 1.07x | 1.07x | 1.08x |
    | mfi | 18,037 | 1.77x | 2.74x | 2.70x | 2.76x |
    | min | 48,048 | 0.59x | 4.73x | 4.54x | 4.92x |
    | mom | 3,656 | 0.98x | 1.61x | 1.57x | 1.65x |
    | msw | 232,335 | 2.23x | 25.12x | 25.02x | 25.24x |
    | natr | 11,240 | 1.77x | 3.92x | 3.69x | 4.09x |
    | ppo | 11,194 | 1.81x | 3.46x | 3.38x | 3.57x |
    | psar | 62,801 | 0.56x | 0.58x | 0.57x | 0.59x |
    | qstick | 10,267 | 1.00x | 1.19x | 1.18x | 1.19x |
    | roc | 10,649 | 0.92x | 1.04x | 0.97x | 1.07x |
    | rocr | 11,217 | 0.88x | 0.99x | 0.89x | 1.06x |
    | rsi | 10,737 | 1.86x | 4.13x | 4.08x | 4.16x |
    | sma | 9,724 | 1.05x | 1.13x | 1.12x | 1.13x |
    | stddev | 15,431 | 0.99x | 1.93x | 1.91x | 1.94x |
    | stoch | 119,704 | 0.69x | 1.67x | 1.61x | 1.71x |
    | stochrsi | 119,136 | 0.95x | 1.59x | 1.54x | 1.63x |
    | tema | 8,315 | 3.13x | 3.29x | 3.12x | 3.45x |
    | trima | 16,130 | 1.39x | 1.77x | 1.76x | 1.79x |
    | trix | 10,270 | 2.41x | 4.29x | 4.29x | 4.30x |
    | tsf | 10,487 | 2.51x | 3.28x | 3.26x | 3.30x |
    | ultosc | 42,688 | 1.57x | 1.71x | 1.70x | 1.74x |
    | vhf | 110,222 | 0.50x | 1.43x | 1.34x | 1.47x |
    | vidya | 38,428 | 1.26x | 1.00x | 1.00x | 1.00x |
    | volatility | 22,844 | 1.59x | 2.36x | 1.59x | 3.13x |
    | vosc | 13,451 | 1.18x | 1.51x | 1.44x | 1.54x |
    | vwma | 15,981 | 0.87x | 1.27x | 1.27x | 1.28x |
    | wilders | 8,266 | 2.30x | 5.31x | 5.28x | 5.33x |
    | willr | 95,139 | 0.69x | 1.62x | 1.55x | 1.70x |
    | wma | 9,022 | 2.84x | 3.86x | 3.67x | 3.96x |
    | zlema | 9,264 | 2.58x | 3.72x | 3.33x | 3.86x |

=== "Python Binding"

    Competitor: **tulip_rs_python** PyO3 binding.
    See [Python Binding](../python.md) for setup and how to run.

    | Indicator | SIMD 4-Options (ns) | Speedup vs Rust | Speedup vs tulip_rs_python | Best | Worst |
    |-----------|--------------------:|-----------------:|----------------------------:|-----:|------:|
    | adosc | 14,957 | 2.11x | 2.03x | 1.63x | 2.94x |
    | adx | 36,889 | 2.55x | 3.71x | 2.75x | 4.91x |
    | adxr | 33,885 | 2.16x | 2.37x | 2.00x | 2.74x |
    | apo | 14,457 | 2.04x | 1.67x | 1.17x | 2.04x |
    | aroon | 83,843 | 0.81x | 1.43x | 0.98x | 1.82x |
    | aroonosc | 84,290 | 0.86x | 1.22x | 1.07x | 1.58x |
    | atr | 23,968 | 1.92x | 1.23x | 0.80x | 1.51x |
    | bbands | 50,801 | 1.17x | 1.02x | 0.58x | 1.39x |
    | ccfisher | 295,752 | 3.26x | 3.18x | 3.07x | 3.29x |
    | cci | 132,518 | 2.56x | 2.64x | 2.02x | 2.96x |
    | chaikinmf | 23,802 | 1.42x | 2.01x | 1.72x | 2.21x |
    | chandelierexit | 81,172 | 0.61x | 1.53x | 1.33x | 1.78x |
    | cmo | 18,593 | 2.01x | 1.84x | 1.11x | 2.27x |
    | cvi | 53,393 | 1.38x | 0.47x | 0.40x | 0.56x |
    | cybercycle | 21,800 | 3.15x | 3.23x | 3.12x | 3.53x |
    | dema | 22,325 | 2.82x | 1.39x | 0.77x | 2.07x |
    | di | 32,934 | 3.48x | 3.47x | 2.91x | 4.37x |
    | dm | 18,673 | 2.22x | 4.47x | 2.67x | 5.73x |
    | donchianchannel | 99,648 | 0.44x | 0.60x | 0.49x | 0.65x |
    | dpo | 15,324 | 0.79x | 1.12x | 0.62x | 1.39x |
    | dx | 21,111 | 2.56x | 3.22x | 2.57x | 3.92x |
    | ef | 12,371 | 1.39x | 2.50x | 1.88x | 3.79x |
    | elderray | 27,880 | 1.47x | 1.22x | 1.01x | 1.40x |
    | ema | 13,986 | 1.90x | 1.92x | 1.48x | 3.00x |
    | fisher | 255,285 | 1.06x | 1.22x | 1.13x | 1.29x |
    | fosc | 21,607 | 1.62x | 6.69x | 6.64x | 6.71x |
    | highpass | 14,174 | 1.81x | 2.50x | 1.77x | 3.28x |
    | hilberttransform | 40,285 | 2.31x | 1.99x | 1.51x | 2.59x |
    | hma | 47,058 | 1.33x | 1.23x | 0.63x | 1.92x |
    | ichimoku | 325,690 | 0.77x | 0.96x | 0.92x | 1.05x |
    | kama | 21,073 | 2.32x | 1.89x | 1.44x | 2.29x |
    | keltnerchannel | 38,217 | 0.89x | 1.24x | 0.94x | 1.68x |
    | kvo | 21,157 | 2.00x | 2.70x | 2.06x | 3.14x |
    | linreg | 30,181 | 2.36x | 1.90x | 0.65x | 3.59x |
    | macd | 38,712 | 1.08x | 1.08x | 0.79x | 1.37x |
    | mama | 259,162 | 3.50x | 3.49x | 3.43x | 3.55x |
    | mass | 18,864 | 2.03x | 2.12x | 1.28x | 2.77x |
    | max | 25,779 | 0.76x | 1.03x | 0.94x | 1.21x |
    | md | 69,064 | 1.00x | 1.17x | 1.01x | 1.35x |
    | mfi | 24,709 | 1.77x | 2.04x | 1.43x | 2.34x |
    | min | 46,401 | 0.59x | 0.82x | 0.58x | 1.22x |
    | mom | 7,266 | 0.98x | 1.09x | 0.69x | 1.77x |
    | msw | 233,732 | 2.23x | 2.32x | 2.22x | 2.41x |
    | natr | 14,234 | 1.77x | 6.08x | 5.28x | 6.64x |
    | ppo | 25,873 | 1.81x | 1.41x | 0.69x | 2.22x |
    | psar | 65,460 | 0.56x | 1.49x | 1.16x | 2.14x |
    | qstick | 11,812 | 1.00x | 1.19x | 1.08x | 1.38x |
    | roc | 16,301 | 0.92x | 1.22x | 0.69x | 2.00x |
    | rocr | 11,761 | 0.88x | 1.05x | 0.97x | 1.28x |
    | roofingfilter | 14,688 | 3.52x | 3.49x | 3.06x | 4.28x |
    | rsi | 17,123 | 1.86x | 1.63x | 1.50x | 1.82x |
    | sma | 16,591 | 1.05x | 1.49x | 1.18x | 1.80x |
    | smaenvelope | 51,599 | 1.23x | 0.77x | 0.43x | 1.16x |
    | stddev | 16,896 | 0.99x | 1.83x | 1.80x | 1.88x |
    | stoch | 103,139 | 0.69x | 1.06x | 0.94x | 1.12x |
    | stochrsi | 139,801 | 0.95x | 0.84x | 0.70x | 0.98x |
    | supersmoother | 15,822 | 3.47x | 3.58x | 2.63x | 4.67x |
    | supertrend | 24,045 | 2.49x | 4.56x | 3.47x | 5.90x |
    | tema | 23,796 | 3.13x | 1.36x | 0.94x | 1.72x |
    | trendmode | 228,043 | 3.86x | 3.94x | 3.87x | 4.00x |
    | trima | 18,046 | 1.39x | 1.47x | 1.24x | 2.03x |
    | trix | 14,414 | 2.41x | 4.74x | 4.22x | 5.15x |
    | trvi | 22,504 | 1.06x | 1.14x | 1.11x | 1.15x |
    | tsf | 17,065 | 2.51x | 4.40x | 2.18x | 5.96x |
    | ultosc | 43,677 | 1.57x | 2.66x | 1.78x | 3.45x |
    | vhf | 109,496 | 0.50x | 0.83x | 0.60x | 1.10x |
    | vidya | 43,549 | 1.26x | 1.61x | 1.24x | 2.02x |
    | volatility | 52,056 | 1.59x | 1.47x | 1.35x | 1.72x |
    | vortex | 44,286 | 1.29x | 1.58x | 1.37x | 2.01x |
    | vosc | 16,823 | 1.18x | 1.81x | 1.22x | 3.19x |
    | vwma | 17,834 | 0.87x | 1.54x | 1.18x | 1.97x |
    | wilders | 13,485 | 2.30x | 1.86x | 1.42x | 2.71x |
    | willr | 87,476 | 0.69x | 0.91x | 0.75x | 1.07x |
    | wma | 13,440 | 2.84x | 2.12x | 1.23x | 2.93x |
    | zlema | 13,165 | 2.58x | 2.17x | 1.72x | 2.49x |

=== "Node Binding"

    Competitor: **tulip_rs_node** napi-rs binding.
    See [Node Binding](../node.md) for setup and how to run.

    | Indicator | SIMD 4-Options (ns) | Speedup vs Rust | Speedup vs tulip_rs_node | Best | Worst |
    |-----------|--------------------:|-----------------:|--------------------------:|-----:|------:|
    | adosc | 32,530 | 2.11x | 1.10x | 1.03x | 1.26x |
    | adx | 31,823 | 2.55x | 1.79x | 1.72x | 1.84x |
    | adxr | 44,893 | 2.16x | 1.79x | 1.63x | 1.87x |
    | apo | 24,924 | 2.04x | 1.13x | 1.11x | 1.18x |
    | aroon | 144,170 | 0.81x | 0.81x | 0.78x | 0.83x |
    | aroonosc | 133,521 | 0.86x | 0.77x | 0.76x | 0.79x |
    | atr | 28,963 | 1.92x | 1.24x | 0.99x | 1.62x |
    | bbands | 65,707 | 1.17x | 0.92x | 0.63x | 1.12x |
    | ccfisher | 303,865 | 3.26x | 3.10x | 3.07x | 3.14x |
    | cci | 157,779 | 2.56x | 2.15x | 2.11x | 2.16x |
    | chaikinmf | 32,946 | 1.42x | 1.44x | 1.38x | 1.49x |
    | chandelierexit | 128,765 | 0.61x | 0.80x | 0.76x | 0.88x |
    | cmo | 27,846 | 2.01x | 1.27x | 1.16x | 1.35x |
    | cvi | 33,555 | 1.38x | 1.00x | 0.86x | 1.05x |
    | cybercycle | 33,709 | 3.15x | 2.25x | 2.16x | 2.36x |
    | dema | 26,015 | 2.82x | 1.22x | 1.21x | 1.24x |
    | di | 54,912 | 3.48x | 1.98x | 1.88x | 2.09x |
    | dm | 37,749 | 2.22x | 1.78x | 1.43x | 1.91x |
    | donchianchannel | 128,339 | 0.44x | 0.66x | 0.62x | 0.73x |
    | dpo | 24,136 | 0.79x | 0.93x | 0.89x | 0.99x |
    | dx | 35,778 | 2.56x | 1.33x | 1.30x | 1.37x |
    | ef | 26,991 | 1.39x | 1.02x | 1.01x | 1.04x |
    | elderray | 41,252 | 1.47x | 1.69x | 1.38x | 2.04x |
    | ema | 26,736 | 1.90x | 1.04x | 1.03x | 1.05x |
    | fisher | 272,444 | 1.06x | 1.08x | 1.06x | 1.10x |
    | fosc | 34,146 | 1.62x | 4.54x | 4.50x | 4.59x |
    | highpass | 25,769 | 1.81x | 1.05x | 1.00x | 1.18x |
    | hilberttransform | 45,911 | 2.31x | 1.90x | 1.87x | 1.95x |
    | hma | 42,841 | 1.33x | 1.03x | 0.95x | 1.15x |
    | ichimoku | 381,868 | 0.77x | 0.91x | 0.89x | 0.93x |
    | kama | 29,405 | 2.32x | 1.29x | 1.22x | 1.39x |
    | keltnerchannel | 62,633 | 0.89x | 1.09x | 1.03x | 1.23x |
    | kvo | 31,579 | 2.00x | 1.50x | 1.45x | 1.56x |
    | linreg | 23,707 | 2.36x | 1.59x | 1.57x | 1.61x |
    | macd | 56,592 | 1.08x | 1.27x | 1.20x | 1.36x |
    | mama | 277,646 | 3.50x | 3.28x | 3.25x | 3.29x |
    | mass | 29,908 | 2.03x | 1.19x | 1.08x | 1.37x |
    | max | 43,186 | 0.76x | 1.07x | 0.98x | 1.20x |
    | md | 73,268 | 1.00x | 1.01x | 0.99x | 1.02x |
    | mfi | 41,326 | 1.77x | 1.23x | 1.08x | 1.44x |
    | min | 65,897 | 0.59x | 0.86x | 0.81x | 0.91x |
    | mom | 23,253 | 0.98x | 0.89x | 0.82x | 1.03x |
    | msw | 250,317 | 2.23x | 2.21x | 2.18x | 2.23x |
    | natr | 29,966 | 1.77x | 1.14x | 1.09x | 1.23x |
    | ppo | 24,987 | 1.81x | 1.28x | 1.25x | 1.32x |
    | psar | 73,515 | 0.56x | 0.71x | 0.70x | 0.71x |
    | qstick | 25,433 | 1.00x | 0.96x | 0.95x | 0.98x |
    | roc | 25,200 | 0.92x | 1.05x | 0.83x | 1.56x |
    | rocr | 27,237 | 0.88x | 0.76x | 0.75x | 0.77x |
    | roofingfilter | 29,662 | 3.52x | 1.77x | 1.56x | 2.17x |
    | rsi | 21,329 | 1.86x | 1.45x | 1.43x | 1.46x |
    | sma | 22,858 | 1.05x | 1.10x | 0.95x | 1.27x |
    | smaenvelope | 51,585 | 1.23x | 1.20x | 1.10x | 1.26x |
    | stddev | 26,348 | 0.99x | 1.40x | 1.38x | 1.42x |
    | stoch | 163,670 | 0.69x | 0.84x | 0.79x | 0.91x |
    | stochrsi | 137,440 | 0.95x | 0.67x | 0.66x | 0.68x |
    | supersmoother | 27,862 | 3.47x | 1.83x | 1.79x | 1.86x |
    | supertrend | 36,162 | 2.49x | 1.75x | 1.69x | 1.84x |
    | tema | 30,093 | 3.13x | 1.24x | 1.20x | 1.33x |
    | trendmode | 241,931 | 3.86x | 3.67x | 3.65x | 3.69x |
    | trima | 28,296 | 1.39x | 1.19x | 1.06x | 1.52x |
    | trix | 31,914 | 2.41x | 1.21x | 0.99x | 1.36x |
    | trvi | 39,376 | 1.06x | 0.90x | 0.84x | 0.98x |
    | tsf | 30,817 | 2.51x | 1.19x | 1.18x | 1.19x |
    | ultosc | 54,548 | 1.57x | 1.28x | 1.27x | 1.29x |
    | vhf | 142,106 | 0.50x | 0.47x | 0.44x | 0.49x |
    | vidya | 53,608 | 1.26x | 1.11x | 1.09x | 1.13x |
    | volatility | 58,843 | 1.59x | 0.95x | 0.94x | 0.95x |
    | vortex | 56,041 | 1.29x | 1.49x | 1.41x | 1.67x |
    | vosc | 26,196 | 1.18x | 1.12x | 1.11x | 1.16x |
    | vwma | 26,613 | 0.87x | 1.18x | 1.07x | 1.46x |
    | wilders | 21,177 | 2.30x | 1.43x | 1.30x | 1.73x |
    | willr | 114,332 | 0.69x | 0.67x | 0.64x | 0.70x |
    | wma | 23,748 | 2.84x | 1.62x | 1.35x | 1.92x |
    | zlema | 21,312 | 2.58x | 1.48x | 1.45x | 1.51x |

??? success "Notable results - by_options"

    **54 of 75 indicators (72%) show a SIMD speedup over 4x sequential Rust.**

    | Category | Indicator | Speedup |
    |----------|-----------|:-------:|
    | **Top performers** | `trendmode` | **3.86x** |
    | **Top performers** | `roofingfilter` | **3.52x** |
    | **Top performers** | `mama` | **3.50x** |
    | **Top performers** | `di` | **3.48x** |
    | **Top performers** | `supersmoother` | **3.47x** |
    | **Top performers** | `ccfisher` | **3.26x** |
    | **Top performers** | `cybercycle` | **3.15x** |
    | **Top performers** | `tema` | **3.13x** |
    | **Top performers** | `wma` | **2.84x** |
    | **Notable improvement** | `msw` | **2.23x** (SDFT optimisation) |
    | **SIMD slower than sequential** | `donchianchannel` | 0.44x |
    | **SIMD slower than sequential** | `vhf` | 0.50x |
    | **SIMD slower than sequential** | `psar` | 0.56x |
    | **SIMD slower than sequential** | `min` | 0.59x |

    Indicators that don't benefit tend to involve complex branching or irregular memory access patterns that prevent effective vectorisation.
