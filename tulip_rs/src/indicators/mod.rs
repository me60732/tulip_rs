//! Scalar (single-asset) technical indicator implementations.
//!
//! Each sub-module exposes a self-contained indicator with a consistent API:
//! - [`info`] — static metadata (name, inputs, options, outputs)
//! - [`min_data`] — minimum input bars required
//! - [`min_data_accuracy`] — minimum bars for a given decimal precision
//! - [`output_length`] — output slice length for a given input length
//! - [`indicator`] — full computation returning outputs and a streaming [`IndicatorState`]
//! - [`IndicatorState::batch_indicator`] — streaming update from saved state
//!
//! SIMD-parallel variants are in [`crate::indicators::simd_indicators`].

pub mod ad;
pub mod adosc;
pub mod adx;
pub mod adxr;
pub mod ao;
pub mod apo;
pub mod aroon;
pub mod aroonosc;
pub mod atr;
pub mod avgprice;
pub mod bbands;
pub mod bop;
pub mod candlestick;
pub mod cci;
pub mod cmo;
pub mod cvi;
pub mod dema;
pub mod di;
pub mod dm;
pub mod dpo;
pub mod dx;
pub mod ema;
pub mod emv;
pub mod fisher;
pub mod fosc;
pub mod hma;
pub mod kama;
pub mod kvo;
pub mod linreg;
pub mod macd;
pub mod marketfi;
pub mod mass;
pub mod max;
pub mod md;
pub mod medprice;
pub mod mfi;
pub mod min;
pub mod mom;
pub mod msw;
pub mod natr;
pub mod nvi;
pub mod obv;
pub mod pivotpoint;
pub mod ppo;
pub mod psar;
pub mod pvi;
pub mod qstick;
pub mod roc;
pub mod rocr;
pub mod rsi;
pub mod sma;
pub mod stddev;
pub mod stoch;
pub mod stochrsi;
pub mod tema;
pub mod tr;
pub mod trima;
pub mod trix;
pub mod tsf;
pub mod typprice;
pub mod ultosc;
pub mod vhf;
pub mod vidya;
pub mod volatility;
pub mod vosc;
pub mod vwma;
pub mod wad;
pub mod wcprice;
pub mod wilders;
pub mod willr;
pub mod wma;
pub mod zlema;


pub mod simd_indicators;
