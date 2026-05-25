# TulipRS

**High-performance technical analysis in Rust.**

TulipRS is a production-ready Rust library implementing 100+ technical indicators and 60+ candlestick patterns with first-class SIMD acceleration. Indicators run on scalar data or on multiple assets / multiple option sets simultaneously using portable SIMD intrinsics. Every indicator returns a serialisable `IndicatorState` alongside its outputs, enabling incremental streaming computation without reprocessing historical data. Native Rust and Python are both fully supported; additional language bindings are planned.

---

## Features at a Glance

| Capability | Detail |
|---|---|
| **Indicators** | 100+ (moving averages, oscillators, trend, volatility, volume, price/statistical) |
| **Candlestick Patterns** | 60+ patterns with bullish/bearish forecasting |
| **SIMD — by assets** | Same options applied to N assets in one CPU pass (`indicator_by_assets::<N>`) |
| **SIMD — by options** | N option sets applied to one asset in one CPU pass (`indicator_by_options::<N>`) |
| **State management** | Every indicator returns a serialisable `IndicatorState` for streaming / incremental use |
| **Languages** | Rust (native), Python (`tulip_rs_python` via PyO3) |

---

## Quick Example

=== "Rust"

    ```rust
    use tulip_rs::indicators::sma::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, state) = indicator(&[close.as_slice()], &[5.0], None).unwrap();

    println!("{:?}", outputs[0]); // SMA(5) values
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.sma.indicator([close], [5.0])

    print(outputs[0])  # SMA(5) values
    ```

---

## Documentation Pages

| Page | Description |
|---|---|
| [Getting Started](getting_started.md) | Installation, feature flags, calling convention, and first examples |
| [Indicators — Overview](indicators/index.md) | Full indicator index with inputs, options, and output counts |
| [Moving Averages](indicators/moving_averages.md) | SMA, EMA, WMA, DEMA, TEMA, TRIMA, HMA, ZLEMA, KAMA, VIDYA, VWMA, Wilders |
| [Oscillators](indicators/oscillators.md) | RSI, MACD, Stoch, StochRSI, Williams %R, CCI, CMO, UltOsc, AO, Fisher, FOSC, MSW |
| [Trend](indicators/trend.md) | PPO, APO, ADX, ADXR, DM, DI, DX, Aroon, AroonOsc, PSAR |
| [Volatility](indicators/volatility.md) | BBands, ATR, NATR, TR, StdDev, Volatility, VHF, CVI |
| [Volume](indicators/volume.md) | AD, ADOSC, OBV, MFI, NVI, PVI, VOSC, KVO, EMV, WAD |
| [Price & Statistical](indicators/price_statistical.md) | AvgPrice, MedPrice, TypPrice, WCPrice, Max, Min, MOM, ROC, ROCR, BOP, LinReg, TSF, TRIX, DPO, Mass, MD, MarketFi, QStick, PivotPoint |
| [Candlestick Patterns](candlestick_patterns.md) | 60+ patterns, forecast types, Rust and Python usage |
| [Indicator API](indicator_api.md) | `info()`, optional outputs, `min_data`, `min_data_accuracy` |
| [SIMD](simd.md) | Conceptual overview: by-assets and by-options modes, lane counts, when to use each |
| [State Management](state_management.md) | Streaming computation, chunked processing, JSON serialisation |
| [Language Bindings](language_bindings.md) | Python (PyO3/maturin) details, result object API, planned bindings |
| [Benchmarks](benchmarks.md) | Comparison against Tulip Indicators (C) and TA-Lib, methodology, how to run |

---

## Language Support

| Language | Status | Package |
|---|---|---|
| **Rust** | ✅ Native | `tulip_rs` (this crate) |
| **Python** | ✅ Supported | [`tulip_rs_python`](https://github.com/me60732/tulip_rs_python) |
| Node.js / WASM | 🔜 Planned | — |
| R | 🔜 Planned | — |
| Julia | 🔜 Planned | — |
