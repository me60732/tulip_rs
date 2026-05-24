//! SIMD-parallel entry points for the Parabolic SAR (PSAR) indicator.
//!
//! This module re-exports [`indicator_by_assets`] and [`indicator_by_options`] from
//! their respective driver sub-modules. All SIMD computation is implemented there.

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::psar::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::psar::indicator_by_options;
