//! Procedural macros for the TulipRS technical analysis library.
//!
//! [`candlestick`](candlestick) — the `#[pattern_template]` attribute macro
//! generating the const pattern definitions in each candlestick pattern
//! module; the build script scans them into the registry.
//!
//! A proc-macro crate cannot be depended on by the code that uses it, so the
//! macro lives here rather than inside `tulip_rs`. Shared encodings come from
//! `tulip_rs_shared` (`candlestick/`).

//! The crate root holds the proc-macro ENTRY point — rustc requires
//! `#[proc_macro]`/`#[proc_macro_attribute]` items at the root, so it is a
//! one-line forwarder into its domain module.

use proc_macro::TokenStream;

mod candlestick;

/// Candlestick pattern generator: expands the bar-requirement attribute
/// syntax into the const `PatternDefinition` + mask builder for one
/// pattern module (the build script scans those into the registry).
#[proc_macro_attribute]
pub fn pattern_template(attr: TokenStream, item: TokenStream) -> TokenStream {
    candlestick::pattern_template_impl(attr, item)
}
