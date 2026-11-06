//! Pattern -> straight-line Rust source.
//!
//! Architecture (post Phase 3 refactor):
//! - `classify`: scalar/pair/packed grouping metadata.
//! - `idents`, `layout`, `shape`, `signature`: small, focused builders.
//! - `shared`: emitter-shared token builders and `EmitterContext`.
//! - `emit_scalar`: scalar kernel emitter (`run_fn`) staged as
//!   hoists/body/stores/returns orchestration.
//! - `emit_assets`: asset-lane kernel emitter (`run_assets_fn`) staged with
//!   the same high-level build phases.
//!
//! One function is emitted per pattern; the steady loop is emitted in ARENA
//! ORDER (dependency-correct by SSA construction), one statement per slot:
//! `Simd<f64, 2>` pair steps extract both lanes to `x{k}` locals, scalar
//! chains and glue read only earlier slots, and every parameter remains runtime
//! via `Values`.

use crate::cse::planner::{Kind, Pattern};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::collections::HashMap;

mod classify;
mod idents;
mod layout;
mod shape;
mod shared;
mod signature;

mod emit_assets;
mod emit_scalar;

use classify::{classify, classify_packed, Scalar};
use emit_assets::run_assets_fn;
use emit_scalar::run_fn;
use idents::build_idents;
use layout::row_layout;
use shape::{chain_step, chain_step_form, ev, Shape};
use shared::{steady_with_optional_groups, used_input_slots};
use signature::sig_const;

pub fn expand(patterns: &[Pattern]) -> TokenStream {
    assert!(!patterns.is_empty(), "kernel!: no patterns given");
    for a in 0..patterns.len() {
        for b in (a + 1)..patterns.len() {
            if patterns[a].nodes == patterns[b].nodes && patterns[a].sinks == patterns[b].sinks {
                // Allow twin scalar+packed patterns (same arena, different packed flag)
                if patterns[a].packed == patterns[b].packed {
                    panic!(
                        "kernel!: patterns {:?} and {:?} have identical arenas — one template serves both",
                        patterns[a].name, patterns[b].name
                    );
                }
            }
        }
    }
    let sigs: Vec<TokenStream> = patterns.iter().map(sig_const).collect();
    let fns: Vec<TokenStream> = patterns.iter().map(run_fn).collect();
    // Asset-lane kernels (`<name>_assets<const L>`): the non-packed twins —
    // the asset axis IS the lane axis for these shapes; period-packing per
    // asset would vectorize the same work twice. Bit-equal to the scalar
    // kernel per asset (pinned by `*_assets_match_scalar` registry tests) —
    // EXCEPT fisher's lanes, where `Ln` runs the hand's SIMD `ln_unchecked`
    // (≤ 20-ULP from libm; the parity test compares 1e-12 relative).
    // All ring shapes are laneable since 2026-10-13: each asset gets its
    // own RingSet/HMA carry (group coordinates are per-arena facts), the
    // derived extrema ride the per-lane mask-dispatched mirror-window walk
    // (the hand's `min_chuncked` shape), and the IIR/clamp/prev chain is
    // plain vector ops with the hand `fisher_simd`'s exact forms.
    // CAPABILITY: EVERY arena gets an asset kernel. A packed pattern normally rides
    // its non-packed twin's `_assets` — the asset renderer matches on NODE KINDS and
    // never consults `packed` or the group classification, so the twin's kernel is
    // the same code for the same `Signature`, and emitting twice would only collide.
    // But "normally" was carrying the whole guarantee: a packed pattern registered
    // WITHOUT a twin got no `_assets` and no dispatcher arm, so `try_run_assets`
    // returned `None` and the caller fell back to tier-2/3 — a silently missing axis,
    // green suite included. Emitting for exactly those arenas removes the failure
    // mode, and it keeps `expand` usable with a LONE packed pattern, which the unit
    // probes in `mod.rs` do on purpose.
    let asset_pats: Vec<&Pattern> = patterns
        .iter()
        .filter(|p| {
            !p.packed
                || !patterns
                    .iter()
                    .any(|o| !o.packed && o.nodes == p.nodes && o.sinks == p.sinks)
        })
        .collect();
    let asset_fns: Vec<TokenStream> = asset_pats.iter().map(|p| run_assets_fn(p)).collect();
    let asset_calls: Vec<TokenStream> = asset_pats
        .iter()
        .map(|p| {
            let f = format_ident!("{}_assets", p.name);
            let s = format_ident!("SIG_{}", p.name);
            quote! { if *sig == #s { return Some(#f::<L>(vals, streams)); } }
        })
        .collect();
    let idents: Vec<_> = patterns
        .iter()
        .map(|p| format_ident!("{}", &p.name))
        .collect();
    let sig_idents: Vec<_> = idents.iter().map(|n| format_ident!("SIG_{n}")).collect();

    let out = quote! {
        #( #sigs )*
        #( #fns )*
        #( #asset_fns )*

        /// Dispatch: `Some(rows)` when the runtime-erased shape equals a
        /// generated pattern; `None` -> caller falls back (tier 2/3).
        pub fn try_run(
            sig: &crate::cse::Signature,
            vals: &crate::cse::Values,
            streams: &[&[f64]],
        ) -> Option<Vec<crate::cse::query::RowRef>> {
            #(
                if *sig == #sig_idents {
                    return Some(#idents(vals, streams));
                }
            )*
            None
        }

        /// Asset-lane dispatch: `L` assets through ONE `(sig, vals)` — the
        /// `indicator_by_assets` shape for merged patterns. Rows per asset
        /// in sink order; `None` for an unregistered signature.
        pub fn try_run_assets<const L: usize>(
            sig: &crate::cse::Signature,
            vals: &crate::cse::Values,
            streams: &[&[&[f64]]; L],
        ) -> Option<Vec<Vec<crate::cse::query::RowRef>>> {
            #( #asset_calls )*
            None
        }
    };
    if std::env::var("CSE_FUSED_DUMP").is_ok() {
        let _ = std::fs::write("/tmp/cse_fused_expanded.rs", out.to_string());
    }
    out
}
