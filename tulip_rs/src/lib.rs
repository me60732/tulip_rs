#![allow(clippy::all)]
#![feature(portable_simd)]
#![feature(adt_const_params)]

pub mod candle_indicators;
pub mod common;
pub mod common_simd;
pub mod cse;
pub mod indicator_types;
pub mod indicators;
pub mod macros;
pub mod math;
pub mod math_simd;
pub mod ring_buffer;
pub mod types;
