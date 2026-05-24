//! SIMD-accelerated indicator implementations.
//!
//! This module provides three levels of SIMD parallelism:
//!
//! - **`*_simd.rs` files** — low-level SIMD state structs and per-bar `calc_simd` functions
//!   used as building blocks by the higher-level drivers.
//! - **`by_asset/`** — process `N` assets with the same options in a single SIMD pass.
//! - **`by_option/`** — process one asset with `N` different option sets in a single SIMD pass.
//!
//! The [`road_train`] module provides the generic `PrimeMover` scheduling engine
//! that drives both `by_asset` and `by_option` computations.

pub mod ad_simd;
pub mod adosc_simd;
pub mod adx_simd;
pub mod adxr_simd;
pub mod ao_simd;
pub mod apo_simd;
pub mod aroon_simd;
pub mod aroonosc_simd;
pub mod atr_simd;
pub mod avgprice_simd;
pub mod bbands_simd;
pub mod bop_simd;
pub mod cci_simd;
pub mod cmo_simd;
pub mod cvi_simd;
pub mod dema_simd;
pub mod di_simd;
pub mod dm_simd;
pub mod dpo_simd;
pub mod dx_simd;
pub mod ema_simd;
pub mod emv_simd;
pub mod fisher_simd;
pub mod fosc_simd;
pub mod hma_simd;
pub mod kama_simd;
pub mod kvo_simd;
pub mod linreg_simd;
pub mod macd_simd;
pub mod macros;
pub mod marketfi_simd;
pub mod mass_simd;
pub mod max_simd;
pub mod md_simd;
pub mod medprice_simd;
pub mod mfi_simd;
pub mod min_simd;
pub mod mom_simd;
pub mod msw_simd;
pub mod natr_simd;
pub mod nvi_simd;
pub mod obv_simd;
pub mod ppo_simd;
pub mod psar_simd;
pub mod pvi_simd;
pub mod qstick_simd;
pub(crate) mod road_train;
pub mod roc_simd;
pub mod rocr_simd;
pub mod rsi_simd;
pub mod simd_types;
pub mod sma_simd;
pub mod stddev_simd;
pub mod stoch_simd;
pub mod stochrsi_simd;
pub mod tema_simd;
pub mod tr_simd;
pub mod trima_simd;
pub mod trix_simd;
pub mod tsf_simd;
pub mod typprice_simd;
pub mod ultosc_simd;
pub mod vhf_simd;
pub mod vidya_simd;
pub mod volatility_simd;
pub mod vosc_simd;
pub mod vwma_simd;
pub mod wad_simd;
pub mod wcprice_simd;
pub mod wilders_simd;
pub mod willr_simd;
pub mod wma_simd;
pub mod zlema_simd;

#[cfg(feature = "simd_assets")]
pub mod by_asset;
#[cfg(feature = "simd_options")]
pub mod by_option;
