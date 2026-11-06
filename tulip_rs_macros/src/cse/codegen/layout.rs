use super::*;

/// The output-row layout BOTH kernels share, derived from the planner's
/// `SinkPlan`: per-group min-lead folds, optional-group `want` flags, row
/// sizes, arena offsets, the total arena size, and which groups are
/// optional-only. The scalar and lane renderers emit these declarations
/// verbatim and differ only in how they WRITE a row (one pointer vs a
/// per-lane scatter), so the layout itself has exactly one implementation.
///
/// Why `want` is hoisted instead of tested per bar: a per-bar `if want` guard
/// costs ~2 cyc/bar because LLVM will not unswitch three independent guards
/// (measured 15.3 → 19.0 µs). Instead the steady loop is emitted twice and
/// `any_opt` picks the copy once outside it, so the primary-only path keeps
/// zero per-bar branches.
pub(super) struct RowLayout {
    pub(super) minl: Vec<TokenStream>,
    pub(super) want: Vec<TokenStream>,
    pub(super) rsz: Vec<TokenStream>,
    pub(super) off: Vec<TokenStream>,
    pub(super) total: TokenStream,
    pub(super) opt_group_ids: Vec<usize>,
}

pub(super) fn row_layout(p: &Pattern, idents: &[String]) -> RowLayout {
    let groups = &p.sink_plan.groups;
    let optional = &p.sink_plan.optional;
    let ng = groups.len();
    let mut minl = Vec::with_capacity(ng);
    let mut want = Vec::new();
    let mut rsz = Vec::with_capacity(ng);
    let mut off = Vec::with_capacity(ng);
    for (gidx, members) in groups.iter().enumerate() {
        // A group's min lead comes from its leader sink's node ident.
        let leader = members[0];
        let ident_g = idents[p.sinks[leader]].clone();
        let g = format_ident!("minl_{}_{}", ident_g, gidx);
        // Sink leads are named by the sink slot, to match `lead_hoists`.
        let l_leader = format_ident!("{}_{}", ident_g, leader);
        if members.len() == 1 {
            minl.push(quote! { let #g = #l_leader; });
        } else {
            let mut min_expr = quote! { #l_leader };
            for &m in &members[1..] {
                let m_ident = idents[p.sinks[m]].clone();
                let l_m = format_ident!("{}_{}", m_ident, m);
                min_expr = quote! { #min_expr.min(#l_m) };
            }
            minl.push(quote! { let #g = #min_expr; });
        }
        // Optional-only groups (no primary member) need a runtime want flag.
        if optional[gidx] {
            let gwant = format_ident!("want{}", gidx);
            let mut want_expr = quote! { false };
            for &m in members.iter() {
                want_expr = quote! { #want_expr | vals.sink_want[#m] };
            }
            want.push(quote! { let #gwant = #want_expr; });
        }
        let rsz_g = format_ident!("rsz{}", gidx);
        rsz.push(quote! { let #rsz_g = n.saturating_sub(#g); });
        let off_g = format_ident!("off{}", gidx);
        if gidx == 0 {
            off.push(quote! { let #off_g = 0usize; });
        } else {
            let off_prev = format_ident!("off{}", gidx - 1);
            let rsz_prev = format_ident!("rsz{}", gidx - 1);
            off.push(quote! { let #off_g = #off_prev + #rsz_prev; });
        }
    }
    let opt_group_ids = optional
        .iter()
        .enumerate()
        .filter(|(_, o)| **o)
        .map(|(g, _)| g)
        .collect();
    let total = if ng == 0 {
        quote! { 0usize }
    } else {
        let mut acc = quote! { rsz0 };
        for gidx in 1..ng {
            let rsz_gidx = format_ident!("rsz{}", gidx);
            acc = quote! { #acc + #rsz_gidx };
        }
        acc
    };
    RowLayout {
        minl,
        want,
        rsz,
        off,
        total,
        opt_group_ids,
    }
}
