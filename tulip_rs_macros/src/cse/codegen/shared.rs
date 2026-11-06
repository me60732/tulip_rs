//! Shared helpers extracted from `emit_scalar.rs` and `emit_assets.rs` to
//! eliminate duplicated steady-loop scaffolding code. These functions build
//! the hoists and body blocks that both renderers use, preserving exact token
//! output (behavior-identical) while reducing redundancy.

use proc_macro2::TokenStream;
use quote::quote;

use super::*;

/// Build lead-hoist declarations for sink slots: `let {ident}_{s} = vals.sink_lead[#s] as usize;`
/// Used by BOTH renderers to hoist per-sink leads once before the steady loop.
pub(super) fn sink_lead_hoists(p: &Pattern, idents: &[String]) -> Vec<TokenStream> {
    let mut lead_hoists = Vec::new();
    for (s_idx, sink_node) in p.sinks.iter().enumerate() {
        let ident_s = idents[*sink_node].clone();
        let l = format_ident!("{}_{}", ident_s, s_idx);
        lead_hoists.push(quote! { let #l = vals.sink_lead[#s_idx] as usize; });
    }
    lead_hoists
}

/// Collect Input slot sources from nodes, preserving first-seen order and
/// de-duplicating via `contains` check. Used by emitters to identify which
/// stream base pointers need hoisting.
pub(super) fn used_input_slots(p: &Pattern) -> Vec<usize> {
    let mut slots: Vec<usize> = Vec::new();
    for node in &p.nodes {
        if node.kind == Kind::Input && !slots.contains(&node.a) {
            slots.push(node.a);
        }
    }
    slots
}

/// SinkWindowSymbols: symbols derived per-sink/group for row-ref and warm-pointer builders.
/// Contains the Ident names needed by both `emit_scalar.rs` and `emit_assets.rs` when
/// building window offsets and pointer arithmetic for sink outputs grouped by shared node.
pub(super) struct SinkWindowSymbols {
    /// Group index (group_of[sidx])
    #[allow(dead_code)]
    pub(super) gidx: usize,
    /// Offset identifier for the group (`off{gidx}`)
    pub(super) off_g: Ident,
    /// Row size identifier for the group (`rsz{gidx}`)
    pub(super) rsz_g: Ident,
    /// Group minimum lead identifier (`minl_{ident_g}_{gidx}`)
    pub(super) g_minl: Ident,
    /// Sink-specific lead identifier (`{ident_s}_{sidx}`)
    pub(super) l_s: Ident,
}

/// Derive SinkWindowSymbols for a sink index within a pattern.
///
/// Used by BOTH renderers to build window offsets and warm-pointer arithmetic.
/// The derivation is identical in `emit_scalar.rs` (row_refs, warm_ptrs) and
/// `emit_assets.rs` (row_refs, warm_ptrs map), extracted here to eliminate duplication.
///
/// # Arguments
/// - `p`: the Pattern being generated
/// - `idents`: display-only naming for nodes (role names or fallback x{k})
/// - `group_members`: group -> list of sink indices in that group
/// - `group_of`: sink index -> group index
/// - `sidx`: sink index to derive symbols for
///
/// # Behavior
/// Matches existing per-sink derivation exactly:
/// - gidx = group_of[sidx]
/// - leader = group_members[gidx][0]
/// - ident_g = idents[p.sinks[leader]].clone()
/// - off_g = format_ident!("off{}", gidx)
/// - rsz_g = format_ident!("rsz{}", gidx)  (only used in emit_scalar row_refs and emit_assets row_refs/warm_ptrs)
/// - g_minl = format_ident!("minl_{}_{}", ident_g, gidx)
/// - l_s = format_ident!("{}_{}", idents[p.sinks[sidx]], sidx)
pub(super) fn sink_window_symbols(
    p: &Pattern,
    idents: &[String],
    group_members: &[Vec<usize>],
    group_of: &[usize],
    sidx: usize,
) -> SinkWindowSymbols {
    let gidx = group_of[sidx];
    let leader = group_members[gidx][0];
    let ident_g = idents[p.sinks[leader]].clone();
    let off_g = format_ident!("off{}", gidx);
    let rsz_g = format_ident!("rsz{}", gidx);
    let g_minl = format_ident!("minl_{}_{}", ident_g, gidx);
    let l_s = format_ident!("{}_{}", idents[p.sinks[sidx]], sidx);
    SinkWindowSymbols {
        gidx,
        off_g,
        rsz_g,
        g_minl,
        l_s,
    }
}

/// Build the sink-window offset expression: `off_g + (l_s - g_minl)`.
///
/// Used by BOTH renderers to compute the window start within a shared group row.
/// The arithmetic is identical in `emit_scalar.rs` (warm_ptrs, row_refs) and
/// `emit_assets.rs` (row_refs, warm_ptrs map), extracted here to eliminate duplication.
pub(super) fn sink_window_offset(sym: &SinkWindowSymbols) -> TokenStream {
    let off_g = &sym.off_g;
    let l_s = &sym.l_s;
    let g_minl = &sym.g_minl;
    quote! { #off_g + (#l_s - #g_minl) }
}

/// Build the sink-window length expression: `rsz_g.saturating_sub(l_s - g_minl)`.
///
/// Used by BOTH renderers to compute the window length within a shared group row.
/// The arithmetic is identical in `emit_scalar.rs` (row_refs) and
/// `emit_assets.rs` (row_refs), extracted here to eliminate duplication.
pub(super) fn sink_window_len(sym: &SinkWindowSymbols) -> TokenStream {
    let rsz_g = &sym.rsz_g;
    let l_s = &sym.l_s;
    let g_minl = &sym.g_minl;
    quote! { #rsz_g.saturating_sub(#l_s - #g_minl) }
}

/// Build row-ref window token for scalar emitter.
///
/// Generates:
/// ```text
/// if vals.sink_want[#sidx] {
///     crate::cse::query::RowRef::window(
///         &arena,
///         #off_expr,
///         #len_expr
///     )
/// } else {
///     crate::cse::query::RowRef::window(&arena, 0, 0)
/// }
/// ```
///
/// Used by `emit_scalar.rs` row_refs loop to build window tokens.
pub(super) fn scalar_row_ref_token(sym: &SinkWindowSymbols, sidx: usize) -> TokenStream {
    let off_expr = sink_window_offset(sym);
    let len_expr = sink_window_len(sym);
    quote! {
        if vals.sink_want[#sidx] {
            crate::cse::query::RowRef::window(
                &arena,
                #off_expr,
                #len_expr
            )
        } else {
            crate::cse::query::RowRef::window(&arena, 0, 0)
        }
    }
}

/// Build row-ref window token for assets emitter.
///
/// Generates:
/// ```text
/// if vals.sink_want[#sidx] {
///     crate::cse::query::RowRef::window(
///         &arena,
///         a * total + #off_expr,
///         #len_expr
///     )
/// } else {
///     crate::cse::query::RowRef::window(&arena, 0, 0)
/// }
/// ```
///
/// Used by `emit_assets.rs` row_refs loop to build window tokens.
pub(super) fn assets_row_ref_token(sym: &SinkWindowSymbols, sidx: usize) -> TokenStream {
    let off_expr = sink_window_offset(sym);
    let len_expr = sink_window_len(sym);
    quote! {
        if vals.sink_want[#sidx] {
            crate::cse::query::RowRef::window(
                &arena,
                a * total + #off_expr,
                #len_expr
            )
        } else {
            crate::cse::query::RowRef::window(&arena, 0, 0)
        }
    }
}

/// Build warm-ptr token for scalar emitter.
///
/// Generates:
/// ```text
/// ptrs[#sidx] = unsafe { base.add(#off_expr) };
/// ```
///
/// Used by `emit_scalar.rs` warm_ptrs loop to build pointer setup tokens.
pub(super) fn scalar_warm_ptr_token(sym: &SinkWindowSymbols, sidx: usize) -> TokenStream {
    let off_expr = sink_window_offset(sym);
    quote! { ptrs[#sidx] = unsafe { base.add(#off_expr) }; }
}

/// Build warm-ptr token for assets emitter.
///
/// Generates:
/// ```text
/// ptrs[#sidx] = unsafe { base.add(a * total + #off_expr) };
/// ```
///
/// Used by `emit_assets.rs` warm_ptrs map to build pointer setup tokens.
pub(super) fn assets_warm_ptr_token(sym: &SinkWindowSymbols, sidx: usize) -> TokenStream {
    let off_expr = sink_window_offset(sym);
    quote! { ptrs[#sidx] = unsafe { base.add(a * total + #off_expr) }; }
}

/// Build the steady loop body with optional-group support.
///
/// When `opt_group_ids` is empty, emits a single loop:
/// ```text
/// for i in 0..(n - gate) {
///     #body
///     #always_stores
/// }
/// ```
///
/// Otherwise, emits two loops (one with opt stores, one without) and an
/// `any_opt` guard outside to pick the copy once:
/// ```text
/// let any_opt = ...;
/// if any_opt {
///     for i in 0..(n - gate) {
///         #body
///         #always_stores
///         #opt_stores
///     }
/// } else {
///     for i in 0..(n - gate) {
///         #body_plain
///         #stores_plain
///     }
/// }
///
/// This matches the hand's `real` and `fisher_simd` steady-loop forms.
pub(super) fn steady_with_optional_groups(
    opt_group_ids: &[usize],
    body: &[TokenStream],
    always_stores: &[TokenStream],
    opt_stores: &[TokenStream],
) -> TokenStream {
    if opt_group_ids.is_empty() {
        quote! {
            for i in 0..(n - gate) {
                #( #body )*
                #( #always_stores )*
            }
        }
    } else {
        let mut any_opt = quote! { false };
        for &g in opt_group_ids {
            let w = format_ident!("want{}", g);
            any_opt = quote! { #any_opt | #w };
        }
        let body_plain = body.to_vec();
        let stores_plain = always_stores.to_vec();
        quote! {
            let any_opt = #any_opt;
            if any_opt {
                for i in 0..(n - gate) {
                    #( #body )*
                    #( #always_stores )*
                    #( #opt_stores )*
                }
            } else {
                for i in 0..(n - gate) {
                    #( #body_plain )*
                    #( #stores_plain )*
                }
            }
        }
    }
}
