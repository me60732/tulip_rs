//! # tulip_rs_shared
//!
//! The dependency-graph LEAF for `tulip_rs`: pure data + const-evaluable
//! helpers, depended on by the runtime crate, the build scripts, the
//! proc-macro crate (`tulip_rs_macros` — `pattern_template` + `request!`),
//! and downstream apps. Nothing here may depend on any other workspace
//! crate.
#![feature(adt_const_params, min_adt_const_params)]
//!
//! Organised by domain:
//!
//! - [`candlestick`] — candle bitmask encoding: bit positions, masks,
//!   variant encoders, `cdl_gap`/engulf return codes. Single source of truth
//!   shared by build.rs, the `pattern_template` proc macro, and runtime.
//! - [`cse`] — the CSE engine's recipe vocabulary: node/recipe/sink data
//!   types, the merge/interning rules, and (as it lands) the indicator
//!   recipe GRAPHS — so runtime and macro expansion consume the same data,
//!   one source, no transcription (see `proposals/Macro_Cse_Engine`).
//!
//! The crate root re-exports [`candlestick`]'s items for the historical
//! `tulip_rs_shared::CONSTANT` paths used by build scripts and macros; new
//! code should prefer the module path.

pub mod candlestick;
pub mod cse;

/// Back-compat: the candlestick constants lived at the crate root until the
/// domain split (2026-10-10). Root paths stay valid; module paths preferred.
pub use candlestick::*;
