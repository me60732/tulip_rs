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
