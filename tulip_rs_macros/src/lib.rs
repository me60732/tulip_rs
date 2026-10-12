//! Procedural macros for the TulipRS technical analysis library — two
//! families, one crate (Rust forces every proc macro into a proc-macro
//! crate, and a proc-macro crate cannot be depended on by the code that
//! uses it, so both macro worlds live here rather than inside `tulip_rs`):
//!
//! * [`candlestick`](candlestick) — the `#[pattern_template]` attribute
//!   macro generating the const pattern definitions in each candlestick
//!   pattern module; the build script scans them into the registry.
//! * [`cse`](cse) — the `request!` macro: recipe graphs → CSE merge →
//!   straight-line fused Rust kernels, emitted at compile time (the JIT's
//!   pipeline, emitting Rust instead of CLIF; the engine superseded it,
//!   archived in the top-level `old_jit/`).
//!
//! The shared vocabulary both families and the runtime consume is
//! `tulip_rs_shared` (`candlestick/` encodings, `cse/` recipe graph).

//! The crate root holds only the two proc-macro ENTRY points — rustc
//! requires `#[proc_macro]`/`#[proc_macro_attribute]` items at the root,
//! so each is a one-line forwarder into its domain module.

use proc_macro::TokenStream;

mod candlestick;
mod cse;

/// Candlestick pattern generator: expands the bar-requirement attribute
/// syntax into the const `PatternDefinition` + mask builder for one
/// pattern module (the build script scans those into the registry).
#[proc_macro_attribute]
pub fn pattern_template(attr: TokenStream, item: TokenStream) -> TokenStream {
    candlestick::pattern_template_impl(attr, item)
}

/// Recipe graphs → CSE merge → straight-line fused Rust kernels + the
/// `SIG_*` consts + `try_run` dispatch, expanded at compile time.
#[proc_macro]
pub fn request(input: TokenStream) -> TokenStream {
    cse::request_impl(input)
}
