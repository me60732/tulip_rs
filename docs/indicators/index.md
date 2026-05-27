# Indicators — Overview

TulipRS implements 75+ technical indicators organised into six categories; every indicator follows the same universal calling convention and returns a serialisable `IndicatorState` alongside its outputs.

---

## Indicator Categories

| Category | Indicators |
|---|---|
| **[Moving Averages](moving_averages/sma.md)** | [SMA](moving_averages/sma.md), [EMA](moving_averages/ema.md), [WMA](moving_averages/wma.md), [DEMA](moving_averages/dema.md), [TEMA](moving_averages/tema.md), [TRIMA](moving_averages/trima.md), [HMA](moving_averages/hma.md), [ZLEMA](moving_averages/zlema.md), [KAMA](moving_averages/kama.md), [VIDYA](moving_averages/vidya.md), [VWMA](moving_averages/vwma.md), [Wilders](moving_averages/wilders.md) |
| **[Oscillators](oscillators/rsi.md)** | [RSI](oscillators/rsi.md), [MACD](oscillators/macd.md), [Stochastic](oscillators/stochastic_oscillator.md), [StochRSI](oscillators/stochrsi.md), [Williams %R](oscillators/williams_r.md), [CCI](oscillators/cci.md), [CMO](oscillators/cmo.md), [Ultimate Oscillator](oscillators/ultimate_oscillator.md), [AO](oscillators/ao.md), [Fisher Transform](oscillators/fisher_transform.md), [FOSC](oscillators/fosc.md), [MSW](oscillators/msw.md) |
| **[Trend](trend/adx.md)** | [PPO](trend/ppo.md), [APO](trend/apo.md), [ADX](trend/adx.md), [ADXR](trend/adxr.md), [DM](trend/dm.md), [DI](trend/di.md), [DX](trend/dx.md), [Aroon](trend/aroon.md), [Aroon Oscillator](trend/aroon_oscillator.md), [PSAR](trend/psar.md) |
| **[Volatility](volatility/bbands.md)** | [BBands](volatility/bbands.md), [ATR](volatility/atr.md), [NATR](volatility/natr.md), [TR](volatility/tr.md), [StdDev](volatility/stddev.md), [Volatility](volatility/volatility.md), [VHF](volatility/vhf.md), [CVI](volatility/cvi.md) |
| **[Volume](volume/ad.md)** | [AD](volume/ad.md), [ADOSC](volume/adosc.md), [OBV](volume/obv.md), [MFI](volume/mfi.md), [NVI](volume/nvi.md), [PVI](volume/pvi.md), [VOSC](volume/vosc.md), [KVO](volume/kvo.md), [EMV](volume/emv.md), [WAD](volume/wad.md) |
| **[Price & Statistical](price_statistical/avgprice.md)** | [AvgPrice](price_statistical/avgprice.md), [MedPrice](price_statistical/medprice.md), [TypPrice](price_statistical/typprice.md), [WCPrice](price_statistical/wcprice.md), [Max](price_statistical/max.md), [Min](price_statistical/min.md), [MOM](price_statistical/mom.md), [ROC](price_statistical/roc.md), [ROCR](price_statistical/rocr.md), [BOP](price_statistical/bop.md), [LinReg](price_statistical/linreg.md), [TSF](price_statistical/tsf.md), [TRIX](price_statistical/trix.md), [DPO](price_statistical/dpo.md), [Mass](price_statistical/mass.md), [MD](price_statistical/md.md), [MarketFi](price_statistical/marketfi.md), [QStick](price_statistical/qstick.md), [PivotPoint](price_statistical/pivotpoint.md) |

---

## Page Structure

Every indicator has its own page with **Rust**, **Python**, and **Node.js** tabs. Each page contains:

- **Basic** — a full call followed by a state-continuation example.
- **Optional Outputs** — shown only for indicators that expose intermediate series.
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
