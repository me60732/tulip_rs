#![allow(clippy::all)]
#![cfg_attr(feature = "portable_simd", feature(portable_simd))]
//#![cfg_attr(feature = "portable_simd")]

//pub mod candle_indicators;
//pub mod candle_types;
//pub mod cdlcommon;
//pub mod cldcommontypes;
pub mod common;
pub mod indicator_types;
pub mod indicators;
pub mod macros;
pub mod math;
pub mod common_simd;
pub mod ring_buffer;
pub mod types;

#[cfg(any(feature = "simd_options", feature = "simd_assets"))]
pub mod math_simd;
