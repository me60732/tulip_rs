# Indicators — Overview

TulipRS implements 75+ technical indicators organised into six categories; every indicator follows the same universal calling convention and returns a serialisable `IndicatorState` alongside its outputs.

---

## Indicator Categories

| Category | Indicators |
|---|---|
| **[Moving Averages](moving_averages.md)** | SMA, EMA, WMA, DEMA, TEMA, TRIMA, HMA, ZLEMA, KAMA, VIDYA, VWMA, Wilders |
| **[Oscillators](oscillators.md)** | RSI, MACD, Stochastic, StochRSI, Williams %R, CCI, CMO, Ultimate Oscillator, AO, Fisher Transform, FOSC, MSW |
| **[Trend](trend.md)** | PPO, APO, ADX, ADXR, DM, DI, DX, Aroon, Aroon Oscillator, PSAR |
| **[Volatility](volatility.md)** | Bollinger Bands, ATR, NATR, TR, StdDev, Volatility, VHF, CVI |
| **[Volume](volume.md)** | AD, ADOSC, OBV, MFI, NVI, PVI, VOSC, KVO, EMV, WAD |
| **[Price & Statistical](price_statistical.md)** | AvgPrice, MedPrice, TypPrice, WCPrice, Max, Min, MOM, ROC, ROCR, BOP, LinReg, TSF, TRIX, DPO, Mass, MD, MarketFi, QStick, PivotPoint |

---

## Page Structure

Every indicator page contains **Rust** and **Python** tabs. Inside each tab you will find:

- **Basic** — a full call followed by a state-continuation example.
- **SIMD** — two sub-sections:
    - **By assets** — apply the same options to N assets in one CPU pass (`indicator_by_assets::<N>` / `simd_by_assets`). N must be 2, 4, 8, or 16.
    - **By options** — apply N option sets to the same asset in one CPU pass (`indicator_by_options::<N>` / `simd_by_options`). Not shown for indicators that have zero options.

---

## Running the Built-In Examples

Each indicator ships with a self-contained Rust example. Run any of them with:

```bash
cargo run --example ti_sma_example
cargo run --example ti_ema_example
cargo run --example ti_macd_example
cargo run --example ti_stoch_example
# … and so on for every indicator
```

Example binaries live in `examples/indicators/` and are named `ti_<module>_example.rs`.
