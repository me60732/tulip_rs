//! Pattern -> straight-line Rust source. One function per pattern; the
//! steady loop is emitted in ARENA ORDER (dependency-correct by SSA
//! construction), one statement per slot: `Simd<f64, 2>` pair steps extract
//! both lanes to `x{k}` locals, scalar chains and glue read only earlier
//! slots, every number comes from `Values` by slot — params stay runtime.

use crate::cse::planner::{Kind, Pattern};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::collections::HashMap;

struct Classified {
    groups: Vec<(usize, usize, usize)>, // (k0, k1, stream) pair groups
    slot_group: HashMap<usize, (usize, bool)>, // slot -> (group, is_lane1)
    scalars: HashMap<usize, Scalar>,    // Ema slots kept scalar
}

enum Scalar {
    Price(usize),
    Chain(usize),
}
#[cfg(debug_assertions)]
impl std::fmt::Debug for Scalar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scalar::Price(s) => write!(f, "Price({})", s),
            Scalar::Chain(a) => write!(f, "Chain({})", a),
        }
    }
}

fn classify(p: &Pattern) -> Classified {
    let mut c = Classified {
        groups: vec![],
        slot_group: HashMap::new(),
        scalars: HashMap::new(),
    };
    let mut k = 0;
    while k + 1 < p.nodes.len() {
        let (x, y) = (&p.nodes[k], &p.nodes[k + 1]);
        if x.kind == Kind::Ema
            && y.kind == Kind::Ema
            && p.plan.is_price_src(k)
            && p.plan.is_price_src(k + 1)
            && x.a == y.a
        {
            c.groups.push((k, k + 1, x.a)); // SOURCE SLOT id, as in classify_packed
            c.slot_group.insert(k, (c.groups.len() - 1, false));
            c.slot_group.insert(k + 1, (c.groups.len() - 1, true));
            k += 2;
            continue;
        }
        k += 1;
    }
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::Ema && !c.slot_group.contains_key(&k) {
            c.scalars.insert(
                k,
                if p.plan.is_price_src(k) {
                    Scalar::Price(p.nodes[node.a].a)
                } else {
                    Scalar::Chain(node.a)
                },
            );
        }
    }
    c
}

/// Packed classify: group same-SOURCE chains into 4/2 lane groups — Ema
/// and Wilders over any shared source node (price Input or a derived node
/// like `tr`), Sma over price Inputs only. Remainder single slots stay
/// scalar (the measured rule: +6.6% packed on `sma_six`, per-shape).
fn classify_packed(p: &Pattern) -> Classified {
    let mut c = Classified {
        groups: vec![],
        slot_group: HashMap::new(),
        scalars: HashMap::new(),
    };

    // Bucket by (kind, SOURCE SLOT). Ema/Wilders pack over any source node
    // (price Input or a shared derived node like `tr` — lanes of one
    // formula reading ONE src value, e.g. atr_multi's three Wilders over
    // the interned tr node); Sma packs only price-sourced (its step reads
    // the stream `t − p` back, derived sources would need the host anchor).
    let mut buckets: std::collections::HashMap<(Kind, usize), Vec<usize>> =
        std::collections::HashMap::new();
    for (k, node) in p.nodes.iter().enumerate() {
        let packable = match node.kind {
            k if chain_step_form(k) => true,
            Kind::Sma => p.plan.is_price_src(k),
            _ => false,
        };
        if packable {
            // Buckets stay per-KIND even though the LAW above is per-form: the
            // unmatched-candidate fallback that sends a lone chain to a scalar
            // hoist is written for `Kind::Ema` only (see `classify`'s `scalars`
            // arms), so mixing Ema/Iir/Wilders lanes here would leave an
            // unmatched `Iir` neither grouped nor scalar-held. Cross-form
            // buckets are the next step, and they need that fallback widened
            // first -- not a bucket-key change alone.
            buckets.entry((node.kind, node.a)).or_default().push(k);
        }
    }

    // DEBUG: print bucket contents for packed patterns
    if std::env::var("CSE_FUSED_DEBUG").is_ok() && p.packed {
        eprintln!("classify_packed buckets:");
        for (key, slots) in &buckets {
            eprintln!("  bucket {:?}: {:?}", key, slots);
        }
        eprintln!(
            "  total EMA slots: {}",
            buckets.values().map(|v| v.len()).sum::<usize>()
        );
    }

    // Process buckets in kind-then-stream order, chunk into 4/2 groups
    let mut bucket_keys: Vec<(Kind, usize)> = buckets.keys().copied().collect();
    bucket_keys.sort_by_key(|(kind, stream)| (*kind as u8, *stream));

    let mut group_idx = 0usize;
    for key in bucket_keys {
        let src = key.1;
        let mut slots = buckets.remove(&key).unwrap_or_default();
        if slots.is_empty() {
            continue;
        }
        // DEBUG: print slots before chunking
        if std::env::var("CSE_FUSED_DEBUG").is_ok() && p.packed {
            eprintln!(
                "classify_packed stream {} slots before chunking: {:?}",
                src, slots
            );
        }
        // Greedy chunking: prefer 4, then 2, remainder scalar
        while slots.len() >= 4 {
            let k0 = slots[0];
            let k1 = slots[1];
            let k2 = slots[2];
            let k3 = slots[3];
            if std::env::var("CSE_FUSED_DEBUG").is_ok() && p.packed {
                eprintln!(
                    "classify_packed created 4-lane group {} with slots [{},{},{},{}]",
                    group_idx, k0, k1, k2, k3
                );
            }
            c.groups.push((k0, k1, src));
            // All 4 slots are part of this group; only k0 triggers the statement
            c.slot_group.insert(k0, (group_idx, false));
            c.slot_group.insert(k1, (group_idx, true));
            c.slot_group.insert(k2, (group_idx, true));
            c.slot_group.insert(k3, (group_idx, true));
            slots = slots[4..].to_vec();
            group_idx += 1;
        }
        while slots.len() >= 2 {
            let k0 = slots[0];
            let k1 = slots[1];
            if std::env::var("CSE_FUSED_DEBUG").is_ok() && p.packed {
                eprintln!(
                    "classify_packed created 2-lane group {} with slots [{},{}]",
                    group_idx, k0, k1
                );
            }
            c.groups.push((k0, k1, src));
            c.slot_group.insert(k0, (group_idx, false));
            c.slot_group.insert(k1, (group_idx, true));
            slots = slots[2..].to_vec();
            group_idx += 1;
        }
        // Remaining single slots go to scalars (Ema chains only — a
        // leftover Sma slot takes its scalar ROLL path from `roll_hoists`,
        // never a chain scalar hoist).
        for k in &slots {
            if p.nodes[*k].kind == Kind::Ema {
                c.scalars.insert(
                    *k,
                    if p.plan.is_price_src(*k) {
                        Scalar::Price(p.nodes[p.nodes[*k].a].a)
                    } else {
                        Scalar::Chain(p.nodes[*k].a)
                    },
                );
            }
        }
    }

    // Add any remaining EMA nodes that weren't grouped (chained EMAs)
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::Ema && !c.slot_group.contains_key(&k) && !c.scalars.contains_key(&k) {
            c.scalars.insert(
                k,
                if p.plan.is_price_src(k) {
                    Scalar::Price(p.nodes[node.a].a)
                } else {
                    Scalar::Chain(node.a)
                },
            );
        }
    }

    // Add Wilders to scalars (never packed in this task)
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::Wilders
            && !c.slot_group.contains_key(&k)
            && !c.scalars.contains_key(&k)
        {
            c.scalars.insert(
                k,
                if p.plan.is_price_src(k) {
                    Scalar::Price(p.nodes[node.a].a)
                } else {
                    Scalar::Chain(node.a)
                },
            );
        }
    }

    c
}

/// value expression of a slot's current-bar value (uniform `x{k}` binding).
fn ev(i: usize, idents: &[String]) -> TokenStream {
    let id = format_ident!("{}", idents[i]);
    quote! { #id }
}

/// Build identifier for a slot based on its optional name.
/// Sanitize: lowercase, non-alphanumeric→`_`, leading digit→prefix `_`.
/// Deduplicate: on collision append `_{k}` to the LATER node's ident.
fn build_idents(pattern: &Pattern) -> Vec<String> {
    let mut idents: Vec<String> = Vec::with_capacity(pattern.nodes.len());
    let mut used: HashMap<String, usize> = HashMap::new();

    // Rust keywords that need suffix
    const RUST_KEYWORDS: &[&str] = &[
        "type", "const", "let", "if", "else", "match", "loop", "move", "ref", "mut", "fn", "pub",
        "use", "mod", "self", "super", "crate", "as", "in", "box", "where", "dyn", "async",
        "await", "static", "extern", "unsafe", "struct", "enum", "impl", "trait", "virtual",
        "final", "macro", "return", "yield", "break", "continue",
    ];

    for (i, opt_name) in pattern.names.iter().enumerate() {
        let ident = if let Some(name) = opt_name {
            // Sanitize: lowercase
            let mut s = name.to_lowercase();

            // Replace non-alphanumeric with _
            s = s
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect::<String>();

            // If starts with digit, prefix with _
            if s.starts_with(|c: char| c.is_ascii_digit()) {
                s.insert(0, '_');
            }

            // If is a Rust keyword, append _
            if RUST_KEYWORDS.contains(&s.as_str()) {
                s.push('_');
            }

            s
        } else {
            format!("x{}", i)
        };

        // Deduplicate: on collision, append _{k} where k is the next available
        let base = ident.clone();
        let mut final_ident = ident;
        while used.contains_key(&final_ident) {
            final_ident = format!("{}_{}", base, used[&base]);
            used.insert(base.clone(), used[&base] + 1);
        }
        used.insert(final_ident.clone(), 1);
        idents.push(final_ident);
    }

    idents
}

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

fn sig_const(p: &Pattern) -> TokenStream {
    let name = format_ident!("SIG_{}", p.name);
    let kinds: Vec<u8> = p.nodes.iter().map(|x| x.kind.tag()).collect();
    let a: Vec<u32> = p.nodes.iter().map(|x| x.a as u32).collect();
    let b: Vec<u32> = p.nodes.iter().map(|x| x.b as u32).collect();
    // For SuperTrend, Fma, and the Where guards, c is the third operand id;
    // for others it's 0
    let c: Vec<u32> = p
        .nodes
        .iter()
        .map(|x| {
            if matches!(
                x.kind,
                Kind::SuperTrend
                    | Kind::Fma
                    | Kind::WhereGe
                    | Kind::WhereLt
                    | Kind::WhereGt
                    | Kind::WhereLe
                    | Kind::IirClamped
            ) {
                x.c.unwrap_or(0) as u32
            } else {
                0u32
            }
        })
        .collect();
    // K_IIRCLAMPED's lo_t/lo_v refs (every other kind: 0, matching erase).
    let d: Vec<u32> = p.nodes.iter().map(|x| x.d as u32).collect();
    let e: Vec<u32> = p.nodes.iter().map(|x| x.e as u32).collect();
    let sinks: Vec<u32> = p.sinks.iter().map(|s| *s as u32).collect();
    // Compute n_streams from Input nodes (max stream index + 1)
    let max_stream = p
        .nodes
        .iter()
        .filter_map(|n| {
            if matches!(n.kind, Kind::Input) {
                Some(n.a)
            } else {
                None
            }
        })
        .fold(0usize, |acc, s| acc.max(s));
    let n_streams = max_stream as u32 + 1;
    quote! {
        pub const #name: crate::cse::Signature = crate::cse::Signature::build(
            &[#(#kinds),*],
            &[#(#a),*],
            &[#(#b),*],
            &[#(#c),*],
            &[#(#d),*],
            &[#(#e),*],
            &[#(#sinks),*],
            #n_streams,
        );
    }
}

/// The element type a steady loop runs on: `f64` for the scalar kernel,
/// `Simd<f64, L>` for the lane kernel. This is the merge's core — the two
/// renderers carry the SAME arithmetic and used to each hold their own copy
/// of every line below, which is exactly how the paths drifted once (the
/// non-mirror ring split landed on the lane path only). An arm that differs
/// in STRUCTURE, not spelling — `WhereGe`/`WhereLt` (a branch is the
/// hand's NaN contract, a select is the lane form), `Ln` (libm vs the hand's
/// SIMD polynomial), the trail rescans, `SuperTrend`, `Hma` — is deliberately
/// NOT here: those are per-path by design, not by accident.
struct Shape {
    lanes: bool,
}

impl Shape {
    /// The six shared binary ops. `Max`/`Min` are IEEE maxNum/minNum both ways
    /// — the hand's own NaN rule, spelled `f64::max`/`f64::min` on scalars and
    /// `simd_max`/`simd_min` on lanes (its own split, `ultosc.rs` vs
    /// `ultosc_simd.rs`). Per bar each is ONE instruction on either axis.
    fn binop(&self, kind: &Kind, ea: &TokenStream, eb: &TokenStream) -> TokenStream {
        match kind {
            Kind::Sub => quote! { #ea - #eb },
            Kind::Add => quote! { #ea + #eb },
            Kind::Mul => quote! { #ea * #eb },
            Kind::Div => quote! { #ea / #eb },
            Kind::Max => {
                if self.lanes {
                    quote! { #ea.simd_max(#eb) }
                } else {
                    quote! { #ea.max(#eb) }
                }
            }
            Kind::Min => {
                if self.lanes {
                    quote! { #ea.simd_min(#eb) }
                } else {
                    quote! { #ea.min(#eb) }
                }
            }
            other => unreachable!("binop called with {other:?}"),
        }
    }

    /// `|a|` — the same call on both element types (portable_simd provides
    /// `abs` for `Simd<f64, L>`).
    fn abs(&self, ea: &TokenStream) -> TokenStream {
        quote! { #ea.abs() }
    }
}

/// One chain step: `st = st.mul_add(first, src * second); st` — one fmul then
/// one fma, in the hand's operand order, element type notwithstanding (both
/// `f64` and `Simd<f64, L>` carry `mul_add`, so the recurrence needs no
/// per-shape spelling; only `binop`'s `max` and the branch/select kinds do).
///
/// `Ema`/`Iir` pass `(inv, mult)`; mean-`Wilders` passes `(mult, inv)` — the
/// SAME two instructions with the magnitudes in the other slots, which is why
/// they are two lanes of one formula rather than two emitter rules (see the
/// `ema::calc` / `wilders::calc` pair in the hand modules). Per-slot: the
/// steady loop pays 1 fmul + 1 fma + 1 register carry.
fn chain_step(carry: &Ident, first: &Ident, second: &Ident, src: &TokenStream) -> TokenStream {
    quote! { #carry = #carry.mul_add(#first, #src * #second); #carry }
}

/// THE ONE LANE-PACKABILITY LAW, stated next to the helper it describes: a node is
/// a lane-group candidate iff its steady step lowers to `chain_step` — one broadcast
/// source, two per-slot weights read from `Values::mults`/`invms`, one carry in
/// `st[k]`. Whatever recipe built it. A group of these is N lanes of ONE formula, so
/// an `Ema(9)` beside an `Ema(21)` beside an `Iir` over the same source is one
/// mul + one fma for all of them, not per chain.
///
/// `Iir` is included because it lowers to the same helper with the same arrangement
/// as `Ema` (`chain_step(&ck, &ik, &mk, &src)`, both sites), so it needs no new group
/// statement — the earlier whitelist naming only Ema/Wilders was an omission, not a
/// judgement. Excluded ON EVIDENCE, not taste:
/// · `IirClamped` carries four clamp thresholds through `b/c/d/e`; a group statement
///   hoists only two per-slot vectors, so the thresholds cannot vary per lane without
///   a new statement.
/// · `Accum`'s multiplier is a NODE (volume), not a value — it hoists no weights at
///   all, so there is nothing to broadcast.
/// · `Sma`/`RollSum`/`Stddev`/`Wma`/rings are a different formula (window fold with
///   age), grouped by the ring machinery; `Sma` is bucketed separately below.
fn chain_step_form(kind: Kind) -> bool {
    matches!(kind, Kind::Ema | Kind::Wilders | Kind::Iir)
}

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
struct RowLayout {
    minl: Vec<TokenStream>,
    want: Vec<TokenStream>,
    rsz: Vec<TokenStream>,
    off: Vec<TokenStream>,
    total: TokenStream,
    opt_group_ids: Vec<usize>,
}

fn row_layout(p: &Pattern, idents: &[String]) -> RowLayout {
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

fn run_fn(p: &Pattern) -> TokenStream {
    let name = format_ident!("{}", p.name);
    let sig = format_ident!("SIG_{}", name);
    let shape = Shape { lanes: false };

    // Build idents for display-only naming (role names or fallback x{k})
    let idents = build_idents(p);

    // Use packed classify if this pattern is packed; otherwise use standard pair classify
    let mut c = if p.packed {
        classify_packed(p)
    } else {
        classify(p)
    };
    if std::env::var("CSE_FUSED_PAIR").is_err() && !p.packed {
        // pairing is opt-IN for now: extraction cost exceeded the packed-fma
        // saving on the measured trio; the machinery stays for lane-widening
        // (M2/GPU). Un-paired Ema slots fall back to scalar chains.
        c.groups.clear();
        c.slot_group.clear();
    }
    // After clearing slot_group, add all remaining EMA slots to scalars
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::Ema && !c.slot_group.contains_key(&k) && !c.scalars.contains_key(&k) {
            c.scalars.insert(
                k,
                if p.plan.is_price_src(k) {
                    Scalar::Price(p.nodes[node.a].a)
                } else {
                    Scalar::Chain(node.a)
                },
            );
        }
    }
    // Add Wilders slots to scalars for hoisting (they need st[] carry like Ema)
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::Wilders
            && !c.slot_group.contains_key(&k)
            && !c.scalars.contains_key(&k)
        {
            c.scalars.insert(
                k,
                if p.plan.is_price_src(k) {
                    Scalar::Price(p.nodes[node.a].a)
                } else {
                    Scalar::Chain(node.a)
                },
            );
        }
    }

    // ---- steady-loop hoists ----
    let mut pair_hoists = Vec::new();
    for (gi, (k0, k1, _stream)) in c.groups.iter().enumerate() {
        let g = format_ident!("g{gi}");
        let gm = format_ident!("g{gi}_m");
        let gi_v = format_ident!("g{gi}_i");
        let (k0, k1) = (*k0, *k1);
        // For 4-lane groups: hoist all 4 slots; for 2-lane: just the pair
        // Find all slots in this group by scanning slot_group
        let mut group_slots: Vec<usize> = c
            .slot_group
            .iter()
            .filter(|(_, (g, _))| *g == gi)
            .map(|(s, _)| *s)
            .collect();
        group_slots.sort();
        if std::env::var("CSE_FUSED_DEBUG").is_ok() && p.packed {
            eprintln!(
                "codegen hoist for group {}: k0={},k1={}, group_slots={:?}, w={}",
                gi,
                k0,
                k1,
                group_slots,
                group_slots.len()
            );
        }
        let w = group_slots.len();
        let is_sma = p.nodes[k0].kind == Kind::Sma;
        match w {
            4 if is_sma => {
                let k2 = group_slots[2];
                let k3 = group_slots[3];
                // Sma lanes: carry = the window SUM, `gm` = the `1/p`
                // multipliers; the step needs no invms vector (add/sub
                // formula, see the group statement).
                pair_hoists.push(quote! {
                    let mut #g = Simd::<f64, 4>::from_array([st[#k0], st[#k1], st[#k2], st[#k3]]);
                    let #gm = Simd::<f64, 4>::from_array([vals.mults[#k0], vals.mults[#k1], vals.mults[#k2], vals.mults[#k3]]);
                });
            }
            4 => {
                let k2 = group_slots[2];
                let k3 = group_slots[3];
                if std::env::var("CSE_FUSED_DEBUG").is_ok() && p.packed {
                    eprintln!(
                        "codegen hoist 4-lane: k0={},k1={},k2={},k3={}",
                        k0, k1, k2, k3
                    );
                }
                pair_hoists.push(quote! {
                    let mut #g = Simd::<f64, 4>::from_array([st[#k0], st[#k1], st[#k2], st[#k3]]);
                    let #gm = Simd::<f64, 4>::from_array([vals.mults[#k0], vals.mults[#k1], vals.mults[#k2], vals.mults[#k3]]);
                    let #gi_v = Simd::<f64, 4>::from_array([vals.invms[#k0], vals.invms[#k1], vals.invms[#k2], vals.invms[#k3]]);
                });
            }
            _ if is_sma => {
                pair_hoists.push(quote! {
                    let mut #g = Simd::<f64, 2>::from_array([st[#k0], st[#k1]]);
                    let #gm = Simd::<f64, 2>::from_array([vals.mults[#k0], vals.mults[#k1]]);
                });
            }
            _ => {
                pair_hoists.push(quote! {
                    let mut #g = Simd::from_array([st[#k0], st[#k1]]);
                    let #gm = Simd::from_array([vals.mults[#k0], vals.mults[#k1]]);
                    let #gi_v = Simd::from_array([vals.invms[#k0], vals.invms[#k1]]);
                });
            }
        }
    }
    let mut scalar_hoists = Vec::new();
    for k in c.scalars.keys() {
        // Get the node's ident at slot k
        let ident_k = idents[*k].clone();
        // Scalar carry/params should be {ident}_carry, {ident}_mult, {ident}_inv
        let ck = format_ident!("{}_carry", ident_k);
        let mk = format_ident!("{}_mult", ident_k);
        let ik = format_ident!("{}_inv", ident_k);
        let ki = *k;
        scalar_hoists.push(quote! {
            let mut #ck = st[#ki];
            let #mk = vals.mults[#ki];
            let #ik = vals.invms[#ki];
        });
    }
    // SuperTrend band state hoists: prev_st=stb[k], prev_ub=stc_ub[k],
    // prev_lb=stlb[k], and the carried trend bool in the node's OWN st[k]
    // slot as 1.0/0.0 (st nodes carry no chain value) — same convention as
    // kernel::exec's K_SUPERTREND arm, so the warm->steady handoff is exact.
    let mut stb_hoists = Vec::new();
    let mut stc_ub_hoists = Vec::new();
    let mut stc_lb_hoists = Vec::new();
    let mut st_trend_hoists = Vec::new();
    let mut st_step_hoists = Vec::new();
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::SuperTrend {
            let kib = format_ident!("stb{k}");
            let kic_ub = format_ident!("stcub{k}");
            let kic_lb = format_ident!("stclb{k}");
            let kit = format_ident!("tr{k}");
            let ki = k;
            stb_hoists.push(quote! { let mut #kib = stb[#ki]; });
            stc_ub_hoists.push(quote! { let mut #kic_ub = stc_ub[#ki]; });
            stc_lb_hoists.push(quote! { let mut #kic_lb = stlb[#ki]; });
            st_trend_hoists.push(quote! { let mut #kit = st[#ki]; });
            let sk = format_ident!("st_step{k}");
            st_step_hoists.push(quote! { let #sk = vals.st_step[#ki]; });
        }
    }
    // Rolling carry/params hoists (Sma/Stddev/Wma): the window step needs
    // the sum carry, its runtime params, and a BASE POINTER to the leaving
    // bar (`prevp` = the pre-sliced stream pointer moved back one period,
    // so the steady leaving read is `prevp.add(i)` — one add, one load,
    // the hand's `real[j]` shape exactly).
    let mut roll_hoists = Vec::new();
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::RollSum {
            // Ring-backed roll: carry = the window SUM (seeded from exec's
            // st[]), plus the group coordinates read once per run: leader
            // slot, capacity, and the compile-once flag saying whether THIS
            // roll is the cap owner (leave = eviction) or shorter (leave =
            // age read at p) — the hand's ao.rs `calc` split, no per-bar
            // comparison.
            let ident_k = idents[k].clone();
            let ck = format_ident!("{}_carry", ident_k);
            let mk = format_ident!("{}_mult", ident_k);
            let pk = format_ident!("{}_p", ident_k);
            let gk = format_ident!("{}_grp", ident_k);
            let cp = format_ident!("{}_cap", ident_k);
            let evi = format_ident!("{}_evi", ident_k);
            let ki = k;
            roll_hoists.push(quote! {
                let mut #ck = st[#ki];
                let #mk = vals.mults[#ki];
                let #pk = vals.periods[#ki] as usize;
                let #gk = vals.ring_of[#ki] as usize;
                let #cp = vals.ring_cap[#gk] as usize;
                let #evi = #pk == #cp;
            });
            continue;
        }
        if node.kind == Kind::Hma {
            // Bespoke machine: destructure the seven accumulators INTO
            // named locals after warm (hacc is then dead — the steady loop
            // owns them, LLVM keeps them live), plus every per-node
            // constant the hand's step region needs and the two
            // pre-sliced leaving pointers (bars ≥ gate > n ⇒ pure step
            // region: stage1/stage2/tail all step form).
            let ik = idents[k].clone();
            let (s1, w1) = (format_ident!("{ik}_s1"), format_ident!("{ik}_w1"));
            let (s2, w2) = (format_ident!("{ik}_s2"), format_ident!("{ik}_w2"));
            let (wss, ss, fd) = (
                format_ident!("{ik}_wss"),
                format_ident!("{ik}_ss"),
                format_ident!("{ik}_fd"),
            );
            let (n1, n2, p1, p2f) = (
                format_ident!("{ik}_n"),
                format_ident!("{ik}_n2"),
                format_ident!("{ik}_p1"),
                format_ident!("{ik}_p2f"),
            );
            let (m1, m2, mp, ww) = (
                format_ident!("{ik}_m1"),
                format_ident!("{ik}_m2"),
                format_ident!("{ik}_mp"),
                format_ident!("{ik}_ww"),
            );
            let (mm, mm1) = (format_ident!("{ik}_m"), format_ident!("{ik}_m1s"));
            let (pv1, pv2) = (format_ident!("{ik}_pv1"), format_ident!("{ik}_pv2"));
            let sbase = format_ident!("s{}", p.nodes[node.a].a);
            let ki = k;
            roll_hoists.push(quote! {
                let [mut #s1, mut #w1, mut #s2, mut #w2,
                     mut #wss, mut #ss, mut #fd] = hacc[#ki];
                let #n1 = vals.periods[#ki] as usize;
                let #n2 = vals.roll_lb[#ki] as usize;
                let #p1 = vals.periods[#ki] as f64;
                let #p2f = vals.roll_lb[#ki] as f64;
                let #m1 = vals.mults[#ki];
                let #m2 = vals.invms[#ki];
                let #mp = vals.st_step[#ki];
                let #ww = vals.consts[#ki];
                let #mm = vals.lag_n[#ki] as usize;
                let #mm1 = #mm - 1usize;
                let #pv1 = unsafe { #sbase.sub(#n1) };
                let #pv2 = unsafe { #sbase.sub(#n2) };
                let _ = &mut #fd; // fold-only accumulator: steady never touches it
            });
            continue;
        }
        if matches!(
            node.kind,
            Kind::Sma | Kind::Stddev | Kind::Wma | Kind::RollMax | Kind::RollMin
        ) {
            // Ring-vs-pointer is the published plan's fact, not a kind scan:
            // an extremum over a DERIVED source is ring-backed, one over a
            // price stream keeps the pointer form. A Sma/Stddev/Wma over a
            // derived source has no ring AND no stream slice to walk, so it
            // stays unemittable (needs the host anchor).
            let derived_roll = p.plan.is_ring(k);
            if !p.plan.is_price_src(k) && !derived_roll {
                panic!(
                    "kernel!: rolling node {k} over a derived source is not emittable (needs the host anchor)"
                );
            }
            let ident_k = idents[k].clone();
            let ck = format_ident!("{}_carry", ident_k);
            let pk = format_ident!("{}_p", ident_k);
            let ki = k;
            let sp = format_ident!("s{}", p.nodes[node.a].a);
            if derived_roll {
                // EXTREMUM RING form (fisher): the rescan window rides the
                // group ring like the sums do — hoist carries, trigger,
                // window span, and the group coords. Steady bars are all
                // past the fold (sink lead ≥ roll lead = feed + period−1),
                // so the emitted arm is the pure step.
                let tc = format_ident!("{}_trail", ident_k);
                let lbk = format_ident!("{}_lb", ident_k);
                let gk = format_ident!("{}_grp", ident_k);
                roll_hoists.push(quote! {
                    let mut #ck = st[#ki];
                    let mut #tc = trail[#ki];
                    let #pk = vals.periods[#ki] as usize;
                    let #lbk = vals.roll_lb[#ki] as usize;
                    let #gk = vals.ring_of[#ki] as usize;
                });
                continue;
            }
            if matches!(node.kind, Kind::RollMax | Kind::RollMin) {
                // Trail-rescan roll: value+trail carries, the trigger p, the
                // window (`look_back + 1`), and a WINDOW BASE POINTER —
                // `wb = s0.sub(look_back)` so the rescan slice at steady bar
                // i is `from_raw_parts(wb.add(i), win)`: the hand's
                // `real[i − lb ..= i]`, one add one slice, zero re-clamps.
                let tc = format_ident!("{}_trail", ident_k);
                let lb = format_ident!("{}_lb", ident_k);
                let win = format_ident!("{}_win", ident_k);
                let wb = format_ident!("{}_wb", ident_k);
                roll_hoists.push(quote! {
                    let mut #ck = st[#ki];
                    let mut #tc = trail[#ki];
                    let #pk = vals.periods[#ki] as usize;
                    let #lb = vals.roll_lb[#ki] as usize;
                    let #win = #lb + 1usize;
                    let #wb = unsafe { #sp.sub(#lb) };
                });
                continue;
            }
            roll_hoists.push(quote! {
                let mut #ck = st[#ki];
                let #pk = vals.periods[#ki] as usize;
            });
            let mk = format_ident!("{}_mult", ident_k);
            let pv = format_ident!("{}_prevp", ident_k);
            roll_hoists.push(quote! {
                let #mk = vals.mults[#ki];
                let #pv = unsafe { #sp.sub(#pk) };
            });
            match node.kind {
                Kind::Stddev => {
                    let sq = format_ident!("{}_sq", ident_k);
                    roll_hoists.push(quote! { let mut #sq = sumsq[#ki]; });
                }
                Kind::Wma => {
                    let wc = format_ident!("{}_weighted", ident_k);
                    let nk = format_ident!("{}_n", ident_k);
                    let wgt = format_ident!("{}_wgt", ident_k);
                    roll_hoists.push(quote! {
                        let mut #wc = wsum[#ki];
                        let #nk = vals.periods[#ki] as f64;
                        let #wgt = vals.mults[#ki];
                    });
                }
                _ => {}
            }
        }
    }
    // IIR-family carry hoists (Kind::Iir/IirClamped/Prev/Accum): the recurrence
    // locals ride `st[k]` (exec seeded/carried them — idling below feed
    // left the hand's zero init untouched), weights are VALUES
    // (`mults`=a/`invms`=b), and IirClamped's four clamp Consts read by
    // slot through the signature's b/c/d/e refs (compile-time literals).
    // Accum is the same carry slot with NO weights — its multiplier is a node
    // (volume), not a value, so it hoists like `Prev` and stops there.
    for (k, node) in p.nodes.iter().enumerate() {
        if !matches!(
            node.kind,
            Kind::Iir | Kind::IirClamped | Kind::Prev | Kind::Accum
        ) {
            continue;
        }
        let ident_k = idents[k].clone();
        let ck = format_ident!("{}_carry", ident_k);
        let ki = k;
        roll_hoists.push(quote! { let mut #ck = st[#ki]; });
        if matches!(node.kind, Kind::Prev | Kind::Accum) {
            continue;
        }
        let mk = format_ident!("{}_mult", ident_k);
        let ik = format_ident!("{}_inv", ident_k);
        roll_hoists.push(quote! {
            let #mk = vals.mults[#ki];
            let #ik = vals.invms[#ki];
        });
        if node.kind == Kind::IirClamped {
            let ht = format_ident!("{}_hi_t", ident_k);
            let hv = format_ident!("{}_hi_v", ident_k);
            let lt = format_ident!("{}_lo_t", ident_k);
            let lv = format_ident!("{}_lo_v", ident_k);
            let (b, c, d, e) = (node.b, node.c.unwrap_or(0), node.d, node.e);
            roll_hoists.push(quote! {
                let #ht = vals.consts[#b];
                let #hv = vals.consts[#c];
                let #lt = vals.consts[#d];
                let #lv = vals.consts[#e];
            });
        }
    }
    // Lag depths are VALUES (`Values::lag_n`), never structure — the same rev-2
    // law that keeps every period out of the Signature. Hoisted once per node so
    // the steady loop carries a register instead of a load; legality (depth ≤
    // max_gate + 1) is decided in `erase`, not here.
    let mut lag_hoists = Vec::new();
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind != Kind::Lag {
            continue;
        }
        let lk = format_ident!("{}_lag", idents[k]);
        let ki = k;
        lag_hoists.push(quote! { let #lk = vals.lag_n[#ki] as usize; });
    }
    let mut lead_hoists = Vec::new();
    for s in 0..p.sinks.len() {
        // Sink slot s points to node p.sinks[s]; use that node's ident + suffix _{s}
        let sink_node = p.sinks[s];
        let ident_s = idents[sink_node].clone();
        let l = format_ident!("{}_{}", ident_s, s);
        let s_idx = s; // capture the value
        lead_hoists.push(quote! { let #l = vals.sink_lead[#s_idx] as usize; });
    }
    // Input base pointers hoisted: steady-loop reads are unchecked raw loads
    // (the slice index's bounds check is provably-always-true to no one but
    // us — n is the min stream length — so LLVM keeps it).
    let mut used_streams: Vec<usize> = Vec::new();
    for sc in c.scalars.values() {
        if let Scalar::Price(s) = sc {
            if !used_streams.contains(s) {
                used_streams.push(*s);
            }
        }
    }
    for (k0, _, src_slot) in &c.groups {
        // The group statement reads the SOURCE NODE's bound local; the
        // raw stream pointer is only needed when that source is an Input
        // (a price-sourced chain). Derived sources (e.g. `tr`) pull their
        // streams through the generic per-node scan below. The mode is the
        // plan's published fact read at the group's first member (all
        // members share the source, so any one answers for the group).
        if p.plan.is_price_src(*k0) {
            let s = p.nodes[*src_slot].a;
            if !used_streams.contains(&s) {
                used_streams.push(s);
            }
        }
    }
    // Collect streams from nodes that read Inputs directly (Lag, Abs, Sub, etc.)
    for (k, node) in p.nodes.iter().enumerate() {
        if matches!(node.kind, Kind::Input | Kind::Const) {
            continue; // leaf nodes don't read
        }
        // The a-operand question IS the plan's published stream mode for this
        // slot (`Price` = reads its source straight off a stream); the b and
        // c operands stay plain arena edges — which operand sits where is
        // wiring, not a layout decision.
        let a_is_input = p.plan.is_price_src(k);
        // `Kind::Wilders` carries the COMPOSITE flag in `b` (mirroring
        // `Signature::b`, which `erase` writes and `sig_const` copies), so for
        // that kind `b` is NOT an arena edge: dereferencing it here would
        // resolve slot 1 — an unrelated node — and hoist a stream this node
        // never reads. Wilders reaches its source through `a`/`is_price_src`
        // and the scalar/group collection above.
        let b_is_input = node.kind != Kind::Wilders
            && p.nodes
                .get(node.b)
                .map(|n| matches!(n.kind, Kind::Input))
                .unwrap_or(false);
        if a_is_input {
            let stream = p.nodes[node.a].a;
            if !used_streams.contains(&stream) {
                used_streams.push(stream);
            }
        }
        if b_is_input {
            let stream = p.nodes[node.b].a;
            if !used_streams.contains(&stream) {
                used_streams.push(stream);
            }
        }
        // SuperTrend has a third input (close)
        if node.kind == Kind::SuperTrend {
            if let Some(c) = node.c {
                let c_is_input = p
                    .nodes
                    .get(c)
                    .map(|n| matches!(n.kind, Kind::Input))
                    .unwrap_or(false);
                if c_is_input {
                    let stream = p.nodes[c].a;
                    if !used_streams.contains(&stream) {
                        used_streams.push(stream);
                    }
                }
            }
        }
    }
    let stream_hoists: Vec<TokenStream> = used_streams
        .iter()
        .map(|s| {
            let sp = format_ident!("s{}", s);
            // Pre-sliced like the hand (`real = &inputs[0][start..]`): the
            // steady loop reads from an already-advanced base, index `i` only.
            quote! { let #sp = unsafe { streams[#s].as_ptr().add(gate) }; }
        })
        .collect();
    let stream_ptr = |s: usize| {
        let sp = format_ident!("s{}", s);
        quote! { unsafe { *#sp.add(i) } }
    };

    // ---- steady-loop body, arena order ----
    let mut body = Vec::new();
    for (k, node) in p.nodes.iter().enumerate() {
        let xk = format_ident!("{}", idents[k]);
        match node.kind {
            Kind::Input => {
                // Read from the appropriate stream pointer
                let stream = node.a;
                let srcp = stream_ptr(stream);
                body.push(quote! { let #xk = #srcp; });
            }
            Kind::Const => { /* hoisted below */ }
            Kind::Ema => {
                if let Some(&(gi, lane1)) = c.slot_group.get(&k) {
                    // For packed mode: lane1=false means this is the first statement of a group
                    // (which binds all lanes); lane1=true means skip (bound by first statement)
                    if lane1 {
                        continue; // bound with lane0's statement
                    }
                    let (k0, k1, _stream) = c.groups[gi];
                    let ga = format_ident!("ga{gi}");
                    let (k0, k1) = (k0, k1);
                    let g = format_ident!("g{gi}");
                    let gm = format_ident!("g{gi}_m");
                    let gi_v = format_ident!("g{gi}_i");
                    // Determine width by scanning slot_group for all slots in this group
                    let mut group_slots: Vec<usize> = c
                        .slot_group
                        .iter()
                        .filter(|(_, (g, _))| *g == gi)
                        .map(|(s, _)| *s)
                        .collect();
                    group_slots.sort();
                    let w = group_slots.len();
                    match w {
                        4 => {
                            let k2 = group_slots[2];
                            let k3 = group_slots[3];
                            // Lane value binds x{k0}/x{k1}/x{k2}/x{k3} must become the idents
                            let x0 = format_ident!("{}", idents[k0]);
                            let x1 = format_ident!("{}", idents[k1]);
                            let x2 = format_ident!("{}", idents[k2]);
                            let x3 = format_ident!("{}", idents[k3]);
                            // Splat the bound input local (one load per stream per bar)
                            // — same source-slot reference rule as the scalar arms.
                            let srcp = ev(p.nodes[k0].a, &idents);
                            body.push(quote! {
                                #g = #g.mul_add(#gi_v, Simd::<f64, 4>::splat(#srcp) * #gm);
                                let #ga = #g.to_array();
                                let #x0 = #ga[0];
                                let #x1 = #ga[1];
                                let #x2 = #ga[2];
                                let #x3 = #ga[3];
                            });
                        }
                        _ => {
                            // Lane value binds x{k0}/x{k1} must become the idents
                            let x0 = format_ident!("{}", idents[k0]);
                            let x1 = format_ident!("{}", idents[k1]);
                            // Splat the bound input local (one load per stream per bar)
                            // — same source-slot reference rule as the scalar arms.
                            let srcp = ev(p.nodes[k0].a, &idents);
                            body.push(quote! {
                                #g = #g.mul_add(#gi_v, Simd::splat(#srcp) * #gm);
                                let #ga = #g.to_array();
                                let #x0 = #ga[0];
                                let #x1 = #ga[1];
                            });
                        }
                    }
                } else {
                    // Get the node's ident at slot k for scalar hoists
                    let ident_k = idents[k].clone();
                    let ck = format_ident!("{}_carry", ident_k);
                    let src = match c.scalars.get(&k) {
                        Some(_) => {
                            // consumers reference the source SLOT's bound local
                            // (Inputs emit theirs first — one load per stream per
                            // bar, no re-derived load expressions; the old
                            // Scalar::Price path re-emitted the raw load and line
                            // 493 even passed a slot id where a stream was read).
                            let e = ev(node.a, &idents);
                            quote! { #e }
                        }
                        None => panic!(
                            "run_fn EMA: scalars key {} not found (len={})",
                            k,
                            c.scalars.len()
                        ),
                    };
                    let mk = format_ident!("{}_mult", ident_k);
                    let ik = format_ident!("{}_inv", ident_k);
                    let step = chain_step(&ck, &ik, &mk, &src);
                    body.push(quote! { let #xk = { #step }; });
                }
            }
            Kind::Sub | Kind::Add | Kind::Mul | Kind::Div | Kind::Max | Kind::Min => {
                let (ea, eb) = (ev(node.a, &idents), ev(node.b, &idents));
                let v = shape.binop(&node.kind, &ea, &eb);
                body.push(quote! { let #xk = #v; });
            }
            Kind::Abs => {
                let ea = ev(node.a, &idents);
                let v = shape.abs(&ea);
                body.push(quote! { let #xk = #v; });
            }
            Kind::Mirror => {
                // State mirror (rsi's `prev_real`, cmo's `prev`/`drop_real`):
                // an identity copy computed every bar, feeding NO sink — it
                // exists for the chunk-exit state store (`get_states`, M1).
                // LLVM dead-code-eliminates it when nothing reads `x{k}`;
                // exec's `K_MIRROR` arm is the same no-op on the warm path.
                let ea = ev(node.a, &idents);
                body.push(quote! { let #xk = #ea; });
            }
            Kind::Fma => {
                // single-rounded: one `mul_add`, matching kernel K_FMA and
                // the hand's `x.mul_add(y, z)` (bbands' band edges).
                let (ea, eb, ec) = (
                    ev(node.a, &idents),
                    ev(node.b, &idents),
                    ev(node.c.expect("Fma: c operand"), &idents),
                );
                body.push(quote! { let #xk = #ea.mul_add(#eb, #ec); });
            }
            Kind::Lag => {
                // steady read is `n` bars back, `n` being the hoisted VALUE
                // (`vals.lag_n[k]`) — `s{stream}` is already pre-sliced at
                // `gate`, so `.sub(n)` lands on bar `gate + i − n`. The depth is
                // NOT structure: hard-coding one here was the masked bug `mom`
                // exposes. Legality (n ≤ max_gate + 1) is enforced in `erase`.
                let stream = p.nodes[node.a].a;
                let sp = format_ident!("s{}", stream);
                let lk = format_ident!("{}_lag", idents[k]);
                body.push(quote! { let #xk = unsafe { *#sp.add(i).sub(#lk) }; });
            }
            Kind::Sma => {
                // hand `cycle_sma` step verbatim: `sum += real[i] −
                // real[i−p]`, value `sum · (1/p)`. Carries/params are the
                // hoisted per-node locals; `prevp.add(i)` is the leaving
                // bar (one add, one load). Packed groups (sma_six_p) step
                // the lane SUMS as one SIMD vector; the per-lane leaving
                // loads stay scalar (each lane has its own period), and
                // the VALUES extract once for the row stores.
                if let Some(&(gi, lane1)) = c.slot_group.get(&k) {
                    if lane1 {
                        continue; // bound by lane0's group statement
                    }
                    let mut gslots: Vec<usize> = c
                        .slot_group
                        .iter()
                        .filter(|(_, (g, _))| *g == gi)
                        .map(|(s, _)| *s)
                        .collect();
                    gslots.sort();
                    let w = gslots.len();
                    let g = format_ident!("g{gi}");
                    let gm = format_ident!("g{gi}_m");
                    let ga = format_ident!("ga{gi}");
                    let gv = format_ident!("gv{gi}");
                    let price = ev(node.a, &idents);
                    let prevs: Vec<TokenStream> = gslots
                        .iter()
                        .map(|s| {
                            let pv = format_ident!("{}_prevp", idents[*s]);
                            quote! { unsafe { *#pv.add(i) } }
                        })
                        .collect();
                    let binds: Vec<TokenStream> = (0..w)
                        .map(|l| {
                            let id = format_ident!("{}", idents[gslots[l]]);
                            quote! { let #id = #ga[#l]; }
                        })
                        .collect();
                    let ty = if w == 4 {
                        quote! { Simd::<f64, 4> }
                    } else {
                        quote! { Simd::<f64, 2> }
                    };
                    body.push(quote! {
                        #g = #g + (#ty::splat(#price) - #ty::from_array([#(#prevs),*]));
                        let #gv = #g * #gm;
                        let #ga = #gv.to_array();
                        #( #binds )*
                    });
                } else {
                    let ident_k = idents[k].clone();
                    let ck = format_ident!("{}_carry", ident_k);
                    let mk = format_ident!("{}_mult", ident_k);
                    let pv = format_ident!("{}_prevp", ident_k);
                    let src = ev(node.a, &idents);
                    body.push(quote! {
                        let #xk = {
                            let prev = unsafe { *#pv.add(i) };
                            #ck = #ck + (#src - prev);
                            #ck * #mk
                        };
                    });
                }
            }
            Kind::Stddev => {
                // hand `stddev::State::calc` op for op: sma step, mean,
                // sum-of-squares step, two-moment variance in the hand's
                // association, flat-floor sqrt. ONE prev load, bound local.
                let ident_k = idents[k].clone();
                let ck = format_ident!("{}_carry", ident_k);
                let sq = format_ident!("{}_sq", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                let pv = format_ident!("{}_prevp", ident_k);
                let src = ev(node.a, &idents);
                body.push(quote! {
                    let #xk = {
                        let prev = unsafe { *#pv.add(i) };
                        #ck = #ck + (#src - prev);
                        let mean = #ck * #mk;
                        #sq = #sq + #src.mul_add(#src, -(prev * prev));
                        let var = #sq.mul_add(#mk, -(mean * mean));
                        var.sqrt().max(f64::EPSILON)
                    };
                });
            }
            Kind::Wma => {
                // hand `wma::State::calc` order EXACTLY: weighted drops
                // the PRE-update window sum, the sum steps, the entering
                // bar lands at weight `n`, one multiply by the weight
                // reciprocal.
                let ident_k = idents[k].clone();
                let ck = format_ident!("{}_carry", ident_k);
                let wc = format_ident!("{}_weighted", ident_k);
                let nk = format_ident!("{}_n", ident_k);
                let wgt = format_ident!("{}_wgt", ident_k);
                let pv = format_ident!("{}_prevp", ident_k);
                let src = ev(node.a, &idents);
                body.push(quote! {
                    let #xk = {
                        let prev = unsafe { *#pv.add(i) };
                        #wc = #wc - #ck;
                        #ck = #ck + (#src - prev);
                        #wc = #wc + #src * #nk;
                        #wc * #wgt
                    };
                });
            }
            Kind::RollSum => {
                // Steady bars are all past the seam (gate > every roll's
                // feed+p), so the arm is the pure step: push, take the
                // leaving bar (eviction for the cap owner, age read for a
                // shorter sibling — the hoisted flag), hand `sma::calc`
                // form `st += x − leave; st·mult`. The ring object is
                // exec's from the warm phase — the fold/eviction state
                // carries over, bit-continuous across the seam.
                let ident_k = idents[k].clone();
                let ck = format_ident!("{}_carry", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                let pk = format_ident!("{}_p", ident_k);
                let gk = format_ident!("{}_grp", ident_k);
                let evi = format_ident!("{}_evi", ident_k);
                let src = ev(node.a, &idents);
                body.push(quote! {
                    let #xk = {
                        let x = #src;
                        // One push per group per bar (idempotent feed —
                        // exec's warm phase obeys the same rule); then the
                        // hand's post-push read: eviction for the cap owner,
                        // age read for shorter siblings. The authority is
                        // `Warm` past the seam, so the eviction is a bare f64
                        // — no `Option`, no per-bar `expect`, no fill branch.
                        rings.feed(#gk, gate + i, x);
                        let leave = if #evi {
                            rings.evicted(#gk)
                        } else {
                            rings.at(#gk, #pk)
                        };
                        #ck += x - leave;
                        #ck * #mk
                    };
                });
            }
            Kind::Hma => {
                // Hand `hma.rs::calc` warm region, statement for statement
                // (gate > n+m ⇒ every steady bar is a step bar): stage1
                // WMA step, stage2 WMA step, `2·o2 − o1`, then the tail's
                // push/divide/subtract order EXACTLY as the hand —
                // `wss −= ss` before `ss −= front()` reads the post-push
                // oldest (age m−1, same slot as the hand's `front()`).
                let ik = idents[k].clone();
                let (s1, w1) = (format_ident!("{ik}_s1"), format_ident!("{ik}_w1"));
                let (s2, w2) = (format_ident!("{ik}_s2"), format_ident!("{ik}_w2"));
                let (wss, ss) = (format_ident!("{ik}_wss"), format_ident!("{ik}_ss"));
                let (p1, p2f) = (format_ident!("{ik}_p1"), format_ident!("{ik}_p2f"));
                let (m1, m2, mp, ww) = (
                    format_ident!("{ik}_m1"),
                    format_ident!("{ik}_m2"),
                    format_ident!("{ik}_mp"),
                    format_ident!("{ik}_ww"),
                );
                let mm1 = format_ident!("{ik}_m1s");
                let (pv1, pv2) = (format_ident!("{ik}_pv1"), format_ident!("{ik}_pv2"));
                let src = ev(node.a, &idents);
                let ki = k;
                body.push(quote! {
                    let #xk = {
                        let x = #src;
                        let old1 = unsafe { *#pv1.add(i) };
                        #w1 -= #s1;
                        #s1 += x - old1;
                        #w1 += x * #p1;
                        let o1 = #w1 * #m1;
                        let old2 = unsafe { *#pv2.add(i) };
                        #w2 -= #s2;
                        #s2 += x - old2;
                        #w2 += x * #p2f;
                        let o2 = #w2 * #m2;
                        let diff = 2.0 * o2 - o1;
                        #wss += diff * #mp;
                        #ss += diff;
                        rings.feed(#ki, gate + i, diff);
                        let out = #wss / #ww;
                        #wss -= #ss;
                        #ss -= rings.at(#ki, #mm1);
                        out
                    };
                });
            }
            Kind::Trail => {
                // The roll's carried trail projected as a value (aroon's
                // `(p − trail)` arithmetic consumes it downstream). No own
                // carry: the ident of the ROLL node resolves the trail slot.
                let roll_ident = format_ident!("{}_trail", idents[node.a]);
                body.push(quote! { let #xk = #roll_ident as f64; });
            }
            Kind::WhereGe => {
                let (ea, eb, ec) = (
                    ev(node.a, &idents),
                    ev(node.b, &idents),
                    ev(node.c.expect("WhereGe: t operand"), &idents),
                );
                body.push(quote! {
                    let #xk = if #ea >= #eb { #ec } else { 0.0 };
                });
            }
            Kind::WhereLt => {
                // the hand's denominator-guard form (identical NaN
                // behaviour): `<` takes the pinned zero, else the value.
                let (ea, eb, ec) = (
                    ev(node.a, &idents),
                    ev(node.b, &idents),
                    ev(node.c.expect("WhereLt: t operand"), &idents),
                );
                body.push(quote! {
                    let #xk = if #ea < #eb { 0.0 } else { #ec };
                });
            }
            Kind::WhereGt => {
                let (ea, eb, ec) = (
                    ev(node.a, &idents),
                    ev(node.b, &idents),
                    ev(node.c.expect("WhereGt: t operand"), &idents),
                );
                body.push(quote! {
                    let #xk = if #ea > #eb { #ec } else { 0.0 };
                });
            }
            Kind::WhereLe => {
                // An EARLY-RETURN guard (`if a <= b { return carry }`): the
                // compute branch is `!(a <= b)`, so `<=` pins the zero and a
                // NaN compare — false for `<=` — runs the value arm and
                // poisons, exactly like the hand.
                let (ea, eb, ec) = (
                    ev(node.a, &idents),
                    ev(node.b, &idents),
                    ev(node.c.expect("WhereLe: t operand"), &idents),
                );
                body.push(quote! {
                    let #xk = if #ea <= #eb { 0.0 } else { #ec };
                });
            }
            Kind::RollMax | Kind::RollMin => {
                let ident_k = idents[k].clone();
                let vc = format_ident!("{}_carry", ident_k);
                let tc = format_ident!("{}_trail", ident_k);
                let pk = format_ident!("{}_p", ident_k);
                let src = ev(node.a, &idents);
                let is_max = node.kind == Kind::RollMax;
                if p.plan.is_ring(k) {
                    // EXTREMUM RING step (fisher): feed the group (idempotent
                    // — one push per src per bar, shared with any sums), then
                    // the same trail machine as the pointer arm. The rescan
                    // walks the MIRROR window (`RingSet::window` — contiguous,
                    // oldest→newest) with the pointer arm's OWN
                    // `find_*_simd::<4>`: the last-position tie and the
                    // `trail = lb − ri` rule cannot drift by construction.
                    let lbk = format_ident!("{}_lb", ident_k);
                    let gk = format_ident!("{}_grp", ident_k);
                    let finder = if is_max {
                        quote! { crate::indicators::max::find_max_simd::<4> }
                    } else {
                        quote! { crate::indicators::min::find_min_simd::<4> }
                    };
                    let ge = if is_max {
                        quote! { >= }
                    } else {
                        quote! { <= }
                    };
                    body.push(quote! {
                        let #xk = {
                            let current = #src;
                            rings.feed(#gk, gate + i, current);
                            #tc = #tc + 1;
                            if #pk <= #tc {
                                let (rv, ri) = #finder(rings.window(#gk, #lbk + 1));
                                #vc = rv;
                                #tc = #lbk - ri;
                            } else if current #ge #vc {
                                #vc = current;
                                #tc = 0;
                            }
                            #vc
                        };
                    });
                    continue;
                }
                let win = format_ident!("{}_win", ident_k);
                let wb = format_ident!("{}_wb", ident_k);
                let finder = if is_max {
                    quote! { crate::indicators::max::find_max_simd::<4> }
                } else {
                    quote! { crate::indicators::min::find_min_simd::<4> }
                };
                let ge_or_le = if is_max {
                    quote! { >= }
                } else {
                    quote! { <= }
                };
                body.push(quote! {
                    let #xk = {
                        #tc = #tc + 1;
                        if #pk <= #tc {
                            let w = unsafe {
                                ::core::slice::from_raw_parts(#wb.add(i), #win)
                            };
                            let (rv, ri) = #finder(w);
                            #vc = rv;
                            #tc = #win - 1 - ri;
                        } else {
                            let current = #src;
                            if current #ge_or_le #vc {
                                #vc = current;
                                #tc = 0;
                            }
                        }
                        #vc
                    };
                });
            }
            Kind::Ln => {
                let src = ev(node.a, &idents);
                body.push(quote! { let #xk = #src.ln(); });
            }
            Kind::Iir | Kind::IirClamped => {
                // Steady bars are all past the chain's feed (gate ≥ every
                // sink lead ≥ the chain lead ≥ its feed), so the arm is the
                // pure step — same convention as the Ema/Wilders arms; the
                // zero-init carry was preserved by exec's idle bars.
                let ident_k = idents[k].clone();
                let ck = format_ident!("{}_carry", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                let ik = format_ident!("{}_inv", ident_k);
                let src = ev(node.a, &idents);
                if node.kind == Kind::Iir {
                    let step = chain_step(&ck, &ik, &mk, &src);
                    body.push(quote! { let #xk = { #step }; });
                } else {
                    let ht = format_ident!("{}_hi_t", ident_k);
                    let hv = format_ident!("{}_hi_v", ident_k);
                    let lt = format_ident!("{}_lo_t", ident_k);
                    let lv = format_ident!("{}_lo_v", ident_k);
                    body.push(quote! {
                        let #xk = {
                            let mut nv = #mk.mul_add(#src, #ck * #ik);
                            if nv > #ht {
                                nv = #hv;
                            } else if nv < #lt {
                                nv = #lv;
                            }
                            #ck = nv;
                            nv
                        };
                    });
                }
            }
            Kind::Prev => {
                // Delayed copy (K_PREV doc): emit the slot, then shadow this
                // bar's src value into it for the NEXT bar. Arena order puts
                // the src chain's statement before this one, so `#ea` here IS
                // this bar's fresh chain value.
                let ident_k = idents[k].clone();
                let ck = format_ident!("{}_carry", ident_k);
                let src = ev(node.a, &idents);
                body.push(quote! {
                    let #xk = { let x = #ck; #ck = #src; x };
                });
            }
            Kind::Accum => {
                // Single-rounding running total (Node::Accum doc): the ONE fma
                // the hand spells, `ratio.mul_add(volume, st)` — and the
                // published value IS the carry, so no separate sink read.
                let ident_k = idents[k].clone();
                let ck = format_ident!("{}_carry", ident_k);
                let (ea, eb) = (ev(node.a, &idents), ev(node.b, &idents));
                body.push(quote! {
                    let #xk = { #ck = #ea.mul_add(#eb, #ck); #ck };
                });
            }
            Kind::Wilders => {
                // Wilders steady step mirrors kernel's K_WILDERS mean mode:
                // `st[k] = st[k].mul_add(vals.mults[k], val * vals.invms[k])`
                // — the EMA step with the two magnitudes in SWAPPED slots
                // (the one-formula identity), so a Wilders bucket packs like
                // an Ema group with that order preserved (lane values of a
                // shared derived source — atr_multi's three chains over one
                // `tr` node — read `ev(src)` exactly like price chains).
                if let Some(&(gi, lane1)) = c.slot_group.get(&k) {
                    if lane1 {
                        continue; // bound with lane0's group statement
                    }
                    let mut group_slots: Vec<usize> = c
                        .slot_group
                        .iter()
                        .filter(|(_, (g, _))| *g == gi)
                        .map(|(s, _)| *s)
                        .collect();
                    group_slots.sort();
                    let w = group_slots.len();
                    let g = format_ident!("g{gi}");
                    let gm = format_ident!("g{gi}_m");
                    let gi_v = format_ident!("g{gi}_i");
                    let ga = format_ident!("ga{gi}");
                    let srcp = ev(node.a, &idents);
                    if w == 4 {
                        let x0 = format_ident!("{}", idents[group_slots[0]]);
                        let x1 = format_ident!("{}", idents[group_slots[1]]);
                        let x2 = format_ident!("{}", idents[group_slots[2]]);
                        let x3 = format_ident!("{}", idents[group_slots[3]]);
                        body.push(quote! {
                            #g = #g.mul_add(#gm, Simd::<f64, 4>::splat(#srcp) * #gi_v);
                            let #ga = #g.to_array();
                            let #x0 = #ga[0];
                            let #x1 = #ga[1];
                            let #x2 = #ga[2];
                            let #x3 = #ga[3];
                        });
                    } else {
                        let x0 = format_ident!("{}", idents[group_slots[0]]);
                        let x1 = format_ident!("{}", idents[group_slots[1]]);
                        body.push(quote! {
                            #g = #g.mul_add(#gm, Simd::splat(#srcp) * #gi_v);
                            let #ga = #g.to_array();
                            let #x0 = #ga[0];
                            let #x1 = #ga[1];
                        });
                    }
                } else {
                    // All Wilders now have scalar hoists via the extended scalars map
                    // Get the node's ident at slot k for scalar hoists
                    let ident_k = idents[k].clone();
                    let ck = format_ident!("{}_carry", ident_k);
                    let chain_v = ev(node.a, &idents);
                    let mk = format_ident!("{}_mult", ident_k);
                    let ik = format_ident!("{}_inv", ident_k);
                    let step = chain_step(&ck, &mk, &ik, &chain_v);
                    body.push(quote! { let #xk = { #step }; });
                }
            }
            Kind::SuperTrend => {
                // SuperTrend steady step — transcribes kernel::exec's
                // K_SUPERTREND arm exactly (which transcribes the hand
                // `calc_st`): atr=a, medprice=b, close=c; carries prev_st,
                // prev_ub, prev_lb in locals, trend in tr{k} (from st[k]).
                let atr_v = ev(node.a, &idents);
                let med_v = ev(node.b, &idents);
                let close_v = if let Some(c) = node.c {
                    ev(c, &idents)
                } else {
                    panic!("SuperTrend: close input not set");
                };
                let step = format_ident!("st_step{k}");
                let stb_prev = format_ident!("stb{k}");
                let stc_ub = format_ident!("stcub{k}");
                let stc_lb = format_ident!("stclb{k}");
                let tr = format_ident!("tr{k}");
                // The node's VALUE binding must be its ident
                let xk_ident = format_ident!("{}", idents[k]);
                body.push(quote! {
                    let w = #step * #atr_v;
                    let mut ub = #med_v + w;
                    let mut lb = #med_v - w;
                    let crosses_up = #close_v > #stb_prev;
                    let crosses_down = #close_v < #stb_prev;
                    let trend = crosses_up || (#tr != 0.0 && !crosses_down);
                    let #xk_ident = if trend {
                        lb = #stc_lb.max(lb);
                        lb
                    } else {
                        ub = #stc_ub.min(ub);
                        ub
                    };
                    #tr = if trend { 1.0 } else { 0.0 };
                    #stb_prev = #xk_ident;
                    #stc_lb = lb;
                    #stc_ub = ub;
                });
            }
        }
    }
    let mut const_hoists = Vec::new();
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::Const {
            // Hoisted const uses its ident directly
            let xk = format_ident!("{}", idents[k]);
            const_hoists.push(quote! { let #xk = vals.consts[#k]; });
        }
    }

    // ---- arena dedup setup: the planner's sink partition ----
    // Sinks sharing a node share ONE physical row (`apo` line ≡ `macd` line
    // in the trio: 5 sinks, 4 rows, 4 stores per bar). `group_members[g][0]`
    // is the group's leader sink; `group_of` maps sink → GROUP index (the
    // leader's SINK index would diverge the moment a duplicate precedes a new
    // group — the bug that once made `group_members[g]` read out of range).
    // Both renderers read this one structure; neither rebuilds it.
    let group_members = &p.sink_plan.groups;
    let group_of = &p.sink_plan.group_of;
    let group_is_optional = &p.sink_plan.optional;
    // Now: G groups, each with leader (first appearance) and members
    let ng = group_members.len();

    // The row layout (min leads, want flags, row sizes, arena offsets, total)
    // is shared with the lane kernel — one implementation in `row_layout`.
    let row = row_layout(p, &idents);
    let group_minl = &row.minl;
    let group_want = &row.want;
    let group_rsz = &row.rsz;
    let off_decl = &row.off;
    let total_expr = &row.total;
    let opt_group_ids = &row.opt_group_ids;

    // Arena allocation (exact jit.rs pattern)
    let arena_alloc = if ng == 0 {
        quote! {
            let arc: std::sync::Arc<[std::mem::MaybeUninit<f64>]> = std::sync::Arc::new_uninit_slice(0usize);
            let base: *mut f64 = std::ptr::null_mut();
            let arena: std::sync::Arc<[f64]> = unsafe { arc.assume_init() };
        }
    } else {
        quote! {
            let total = #total_expr;
            let mut arc: std::sync::Arc<[std::mem::MaybeUninit<f64>]> =
                std::sync::Arc::new_uninit_slice(total);
            let base: *mut f64 = if total == 0 {
                std::ptr::null_mut()
            } else {
                // uniquely owned (fresh, refcount 1) => `get_mut` yields the slice;
                // the compiled loop writes through `base` while it is the only
                // holder, exactly as the former `Rc` did. `Arc` only so the
                // resulting `Results` is `Send` (threading across symbols, §3.3).
                std::sync::Arc::get_mut(&mut arc)
                    .expect("fresh arena is uniquely owned")
                    .as_mut_ptr()
                    .cast::<f64>()
            };
            let arena: std::sync::Arc<[f64]> = unsafe { arc.assume_init() };
        }
    };

    // ---- warm exec ptrs setup (per-sink, aliased via arena) ----
    let mut warm_ptrs: Vec<TokenStream> = Vec::new();
    for sidx in 0..p.sinks.len() {
        let gidx = group_of[sidx];
        // Get the ident for this group
        let leader_g = group_members[gidx][0];
        let ident_g = idents[p.sinks[leader_g]].clone();
        let g = format_ident!("minl_{}_{}", ident_g, gidx);
        // Keep off as plain numeric per task requirement
        let off_g = format_ident!("off{}", gidx);
        // Build sink lead var name using the node's ident
        let sink_node_sidx = p.sinks[sidx];
        let ident_s = idents[sink_node_sidx].clone();
        let l_s = format_ident!("{}_{}", ident_s, sidx);
        // ptr = base + off_g + (lead_s - minl_g): this sink's window inside
        // the shared group row.
        warm_ptrs.push(quote! {
            ptrs[#sidx] = unsafe { base.add(#off_g + (#l_s - #g)) };
        });
    }

    // ---- steady loop: group store pointers (hoisted once per group) ----
    let mut group_stores: Vec<TokenStream> = Vec::new();
    for (gidx, members) in group_members.iter().enumerate() {
        let leader = members[0];
        let ident_g = idents[p.sinks[leader]].clone();
        let q_g = format_ident!("row_{}_{}", ident_g, gidx);
        // Keep off as plain numeric per task requirement
        let off_g = format_ident!("off{}", gidx);
        let g_minl = format_ident!("minl_{}_{}", ident_g, gidx);
        // Pre-advanced past the warm region AND the group's lead (the hand's
        // `slice_outputs_start!` equivalent): the steady store is `q.add(i)`.
        group_stores.push(quote! {
            let #q_g = unsafe { base.add(#off_g + (gate - #g_minl)) };
        });
    }

    // ---- steady stores: ONE PER GROUP (the dedup: sinks sharing a node —
    // e.g. apo line and macd line are both node 3 — share one physical row
    // and one store, aliased downstream by window offsets). Groups split on
    // role: primary-member groups store unconditionally; optional-only
    // groups are want-guarded and appear ONLY in the any_opt loop copy.
    let mut always_stores: Vec<TokenStream> = Vec::new();
    let mut opt_stores: Vec<TokenStream> = Vec::new();
    for (gidx, members) in group_members.iter().enumerate() {
        let leader = members[0];
        let ident_g = idents[p.sinks[leader]].clone();
        let e = ev(p.sinks[members[0]], &idents);
        let q_g = format_ident!("row_{}_{}", ident_g, gidx);
        if group_is_optional[gidx] {
            let want_g = format_ident!("want{}", gidx);
            opt_stores.push(quote! { unsafe { if #want_g { *#q_g.add(i) = #e; } } });
        } else {
            always_stores.push(quote! { unsafe { *#q_g.add(i) = #e; } });
        }
    }

    // ---- return RowRef windows per sink (in sink order) ----
    let mut row_refs: Vec<TokenStream> = Vec::new();
    for sidx in 0..p.sinks.len() {
        let gidx = group_of[sidx];
        // Keep off and rsz as plain numeric per task requirement
        let leader_g = group_members[gidx][0];
        let ident_g = idents[p.sinks[leader_g]].clone();
        let off_g = format_ident!("off{}", gidx);
        let rsz_g = format_ident!("rsz{}", gidx);
        let g_minl = format_ident!("minl_{}_{}", ident_g, gidx);
        // Build sink lead var name using the node's ident
        let sink_node_sidx = p.sinks[sidx];
        let ident_s = idents[sink_node_sidx].clone();
        let l_s = format_ident!("{}_{}", ident_s, sidx);
        // Disabled sinks get zero-length windows (is_empty() == true)
        row_refs.push(quote! {
            if vals.sink_want[#sidx] {
                crate::cse::query::RowRef::window(
                    &arena,
                    #off_g + (#l_s - #g_minl),
                    #rsz_g.saturating_sub(#l_s - #g_minl)
                )
            } else {
                crate::cse::query::RowRef::window(&arena, 0, 0) // disabled: empty row
            }
        });
    }

    // Early return case (n==0): build empty arena with zero-length windows
    let mut zero_row_refs: Vec<TokenStream> = Vec::new();
    for _sidx in 0..p.sinks.len() {
        zero_row_refs.push(quote! { crate::cse::query::RowRef::window(&arena, 0, 0) });
    }
    let early_return = quote! {
        let arc: std::sync::Arc<[std::mem::MaybeUninit<f64>]> = std::sync::Arc::new_uninit_slice(0usize);
        let arena: std::sync::Arc<[f64]> = unsafe { arc.assume_init() };
        return vec![
            #(#zero_row_refs),*
        ];
    };

    // ---- steady loop: ONE specialized copy for the common primary-only
    // path (no flag loads, no branches, store set identical to the
    // pre-flags kernel), one copy with want-guarded optional stores. The
    // choice is one runtime branch OUTSIDE the loop (WYSIWYG: the guards we
    // measured as +2 cyc/bar when left inside). ----
    let steady = if opt_group_ids.is_empty() {
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
        let body_plain = body.clone();
        let stores_plain = always_stores.clone();
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
    };

    quote! {
        #[allow(unused_variables, unused_mut)]
        pub fn #name(vals: &crate::cse::Values, streams: &[&[f64]]) -> Vec<crate::cse::query::RowRef> {
            use ::std::simd::Simd;
            #[allow(unused_imports)]
            use ::std::simd::num::SimdFloat;
            #[allow(unused_imports)]
            use ::std::simd::StdFloat;
            let n = streams.iter().map(|s| s.len()).min().unwrap_or(0);
            let gate = (vals.max_gate as usize + 1).min(n);

            if n == 0 {
                #early_return
            }

            // Per-sink leads (hoisted once per request).
            #( #lead_hoists )*
            // Per-group min lead computation
            #( #group_minl )*
            // Per-group want hoists (runtime, before steady loop)
            #( #group_want )*
            // Per-group row size (depends on n)
            #( #group_rsz )*

            // Arena offsets (computed from row sizes)
            #( #off_decl )*

            // Arena allocation
            #arena_alloc

            let mut ptrs: [*mut f64; crate::cse::MAX_SINKS] = [::core::ptr::null_mut(); crate::cse::MAX_SINKS];
            #(
                #warm_ptrs
            )*

            // Steady loop hoists (stream pointers)
            #( #stream_hoists )*

            let mut st = [0.0f64; crate::cse::MAX];
            let mut stb = [0.0f64; crate::cse::MAX]; // SuperTrend prev_st
            let mut stc_ub = [0.0f64; crate::cse::MAX]; // SuperTrend prev_ub
            let mut stlb = [0.0f64; crate::cse::MAX]; // SuperTrend prev_lb
            let mut sumsq = [0.0f64; crate::cse::MAX]; // Stddev sum-of-squares carry
            let mut wsum = [0.0f64; crate::cse::MAX]; // Wma weighted-sum carry
            let mut trail = [0usize; crate::cse::MAX]; // roll trail carry
            let mut rings = crate::cse::kernel::warmup::RingSet::new();
            let mut hacc = [[0.0f64; 7]; crate::cse::MAX];
            let mut v = [0.0f64; crate::cse::MAX];
            // Warm-up bars: shared interpreter, cold path once per request.
            crate::cse::kernel::exec::<#sig, true>(vals, streams, 0, gate, &mut st, &mut stb, &mut stc_ub, &mut stlb, &mut sumsq, &mut wsum, &mut trail, &mut rings, &mut hacc, &mut v, &mut ptrs, &vals.sink_lead);
            if gate >= n {
                return vec![#(#row_refs),*];
            }
            // Fill-state transition at the seam: `exec` ran the warm bars on
            // the `Cold` authority (correct — that IS warmup), and every
            // steady bar from here drives `Warm`. `into_warm` debug-asserts
            // each group `is_full`, so the fullness this path always assumed
            // (the arm-level `.expect` never fired) is now checked once at the
            // seam instead of branched on every bar.
            let mut rings = rings.into_warm();

            // Steady loop: group store pointers (hoisted once per group before loop)
            #( #group_stores )*
            #( #pair_hoists )*
            #( #scalar_hoists )*
            #( #stb_hoists )*
            #( #stc_ub_hoists )*
            #( #stc_lb_hoists )*
            #( #st_trend_hoists )*
            #( #st_step_hoists )*
            #( #roll_hoists )*
            #( #lag_hoists )*
            #( #const_hoists )*

            #steady
            vec![#(#row_refs),*]
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
// M2 — the asset axis: `<name>_assets<const L>`, L assets through ONE
// (Signature, Values). `by_assets` layout for inputs (asset-major,
// field-minor, `&[&[&[f64]]; L]`) and rows out (`Vec<Vec<RowRef>>` per
// asset, sink order). Every per-bar linear operation (chains, glue, sums,
// fma/selects) becomes a `Simd<f64, L>` lane step — NO cross-lane
// extraction anywhere, the condition packing needed to win. Branchy
// per-lane kinds keep exactness by construction: rolls rescan the lanes a
// bitmask says need it, through the same `find_*_simd::<4>` the scalar path
// and the hand use; the Supertrend block becomes the branchless
// all-selects form of the same formula (selects compute both arms, pick
// one — bit-identical). Warm bars run the existing scalar `exec` once per
// asset (the warm path stays ONE authority), each asset's carry arrays then
// gathering into the lane vectors. `Values` is asset-invariant — one plan
// serves every L; a lane kernel equals the scalar kernel per asset bit for
// bit (registry-pinned), so this axis can never fork results.
// ──────────────────────────────────────────────────────────────────────────
fn run_assets_fn(p: &Pattern) -> TokenStream {
    let name = format_ident!("{}_assets", p.name);
    let sig = format_ident!("SIG_{}", p.name);
    let shape = Shape { lanes: true };
    let idents = build_idents(p);

    // ---- sink groups: the planner's partition, the SAME structure the scalar
    // renderer reads (`Pattern::sink_plan`). One implementation of the
    // row-dedup rule, so the two kernels cannot disagree about how many rows
    // exist, who leads each one, or which are optional-only.
    let group_members = &p.sink_plan.groups;
    let group_of = &p.sink_plan.group_of;
    let group_is_optional = &p.sink_plan.optional;

    // ---- per-sink lead hoists + per-group min-lead / want / rsz / offs ----
    let mut lead_hoists: Vec<TokenStream> = Vec::new();
    for s in 0..p.sinks.len() {
        let sink_node = p.sinks[s];
        let ident_s = idents[sink_node].clone();
        let l = format_ident!("{}_{}", ident_s, s);
        lead_hoists.push(quote! { let #l = vals.sink_lead[#s] as usize; });
    }
    // The row layout is the SAME function the scalar kernel calls — the two
    // kernels differ only in how they write a row, not in how they size them.
    let row = row_layout(p, &idents);
    let group_minl = &row.minl;
    let group_want = &row.want;
    let group_rsz = &row.rsz;
    let off_decl = &row.off;
    let total_expr = &row.total;
    let opt_group_ids = &row.opt_group_ids;

    // ---- stream base arrays (one per Input slot actually read) ----
    let mut used_slots: Vec<usize> = Vec::new();
    for node in &p.nodes {
        if node.kind == Kind::Input && !used_slots.contains(&node.a) {
            used_slots.push(node.a);
        }
    }
    let stream_arr_hoists: Vec<TokenStream> = used_slots
        .iter()
        .map(|s| {
            let sfa = format_ident!("sp{}", s);
            quote! {
                let #sfa: [*const f64; L] = std::array::from_fn(
                    |a| unsafe { streams[a][#s].as_ptr().add(gate) },
                );
            }
        })
        .collect();
    let stream_slot = |slot_src: usize| -> usize { p.nodes[slot_src].a };

    // ---- carry/param lane hoists, arena order ----
    let mut carry_hoists: Vec<TokenStream> = Vec::new();
    for (k, node) in p.nodes.iter().enumerate() {
        let ident_k = idents[k].clone();
        let ki = k;
        match node.kind {
            Kind::Ema | Kind::Wilders => {
                let ck = format_ident!("{}_carry", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                let ik = format_ident!("{}_inv", ident_k);
                carry_hoists.push(quote! {
                    let mut #ck = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                    let #mk = Simd::<f64, L>::splat(vals.mults[#ki]);
                    let #ik = Simd::<f64, L>::splat(vals.invms[#ki]);
                });
            }
            Kind::Sma | Kind::Stddev | Kind::Wma => {
                let ck = format_ident!("{}_carry", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                let pk = format_ident!("{}_p", ident_k);
                let pv = format_ident!("{}_prevp", ident_k);
                let s = stream_slot(node.a);
                let sfa = format_ident!("sp{}", s);
                // exec's `st[k]` IS the window SUM for these kinds (fold bars
                // emit 0 but accumulate into st), and `sumsq`/`wsum` are
                // exec-maintained too — the per-asset warm left them exact,
                // so the lane seeds straight from those arrays.
                carry_hoists.push(quote! {
                    let mut #ck = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                    let #mk = Simd::<f64, L>::splat(vals.mults[#ki]);
                    let #pk = vals.periods[#ki] as usize;
                    let #pv: [*const f64; L] = std::array::from_fn(|a| unsafe { #sfa[a].sub(#pk) });
                });
                if node.kind == Kind::Stddev {
                    let sq = format_ident!("{}_sq", ident_k);
                    carry_hoists.push(quote! {
                        let mut #sq = Simd::<f64, L>::from_array(std::array::from_fn(|a| sumsqw[a][#ki]));
                    });
                }
                if node.kind == Kind::Wma {
                    let wc = format_ident!("{}_weighted", ident_k);
                    let nk = format_ident!("{}_n", ident_k);
                    let wgt = format_ident!("{}_wgt", ident_k);
                    carry_hoists.push(quote! {
                        let mut #wc = Simd::<f64, L>::from_array(std::array::from_fn(|a| wsumw[a][#ki]));
                        let #nk = Simd::<f64, L>::splat(vals.periods[#ki] as f64);
                        let #wgt = Simd::<f64, L>::splat(vals.mults[#ki]);
                    });
                }
            }
            Kind::RollMax | Kind::RollMin => {
                let ck = format_ident!("{}_carry", ident_k);
                let tc = format_ident!("{}_trail", ident_k);
                let pk = format_ident!("{}_p", ident_k);
                let lb = format_ident!("{}_lb", ident_k);
                if p.plan.is_ring(k) {
                    // Derived source: EXTREMUM RING lane hoist (fisher). The
                    // group coordinates are ARENA facts — one (leader, cap)
                    // pair drives all L ring instances; carries seed from
                    // exec's per-asset st[]/trail[] exactly like the pointer
                    // lanes. No window pointers: the mask-dispatched body arm
                    // reads each lane's OWN contiguous mirror slice through
                    // `RingSetL::window` — the hand `min_chuncked` shape.
                    let gk = format_ident!("{}_grp", ident_k);
                    carry_hoists.push(quote! {
                        let mut #ck = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                        let mut #tc: Simd<usize, L> = Simd::from_array(std::array::from_fn(|a| trailw[a][#ki]));
                        let #pk = vals.periods[#ki] as usize;
                        let #lb = vals.roll_lb[#ki] as usize;
                        let #gk = vals.ring_of[#ki] as usize;
                    });
                    continue;
                }
                let wn = format_ident!("{}_win", ident_k);
                let wb = format_ident!("{}_wb", ident_k);
                let s = stream_slot(node.a);
                let sfa = format_ident!("sp{}", s);
                carry_hoists.push(quote! {
                    let mut #ck = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                    let mut #tc: Simd<usize, L> = Simd::from_array(std::array::from_fn(|a| trailw[a][#ki]));
                    let #pk = vals.periods[#ki] as usize;
                    let #lb = vals.roll_lb[#ki] as usize;
                    let #wn = #lb + 1usize;
                    let #wb: [*const f64; L] = std::array::from_fn(|a| unsafe { #sfa[a].sub(#lb) });
                });
            }
            Kind::SuperTrend => {
                let stb = format_ident!("stb{}", k);
                let stc_ub = format_ident!("stcub{}", k);
                let stc_lb = format_ident!("stclb{}", k);
                let tr = format_ident!("tr{}", k);
                let step = format_ident!("st_step{}", k);
                carry_hoists.push(quote! {
                    let mut #stb = Simd::<f64, L>::from_array(std::array::from_fn(|a| stbw[a][#ki]));
                    let mut #stc_ub = Simd::<f64, L>::from_array(std::array::from_fn(|a| stcubw[a][#ki]));
                    let mut #stc_lb = Simd::<f64, L>::from_array(std::array::from_fn(|a| stlbw[a][#ki]));
                    let mut #tr = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                    let #step = Simd::<f64, L>::splat(vals.st_step[#ki]);
                });
            }
            Kind::Iir | Kind::IirClamped => {
                // Lane IIR: same `st[k]` carry exec seeded per asset (the
                // idle bars left the hand's zero init untouched), weights
                // splat once. IirClamped's four clamp Consts resolve their
                // arena slots at EXPANSION time (b/c/d/e refs) and splat.
                let ck = format_ident!("{}_carry", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                let ik = format_ident!("{}_inv", ident_k);
                let ki = k;
                carry_hoists.push(quote! {
                    let mut #ck = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                    let #mk = Simd::<f64, L>::splat(vals.mults[#ki]);
                    let #ik = Simd::<f64, L>::splat(vals.invms[#ki]);
                });
                if node.kind == Kind::IirClamped {
                    let (ht, hv, lt, lv) = (
                        format_ident!("{}_hi_t", ident_k),
                        format_ident!("{}_hi_v", ident_k),
                        format_ident!("{}_lo_t", ident_k),
                        format_ident!("{}_lo_v", ident_k),
                    );
                    let (b, c, d, e) = (node.b, node.c.unwrap_or(0), node.d, node.e);
                    carry_hoists.push(quote! {
                        let #ht = Simd::<f64, L>::splat(vals.consts[#b]);
                        let #hv = Simd::<f64, L>::splat(vals.consts[#c]);
                        let #lt = Simd::<f64, L>::splat(vals.consts[#d]);
                        let #lv = Simd::<f64, L>::splat(vals.consts[#e]);
                    });
                }
            }
            Kind::Prev => {
                // Lane delayed copy: `st[k]` after exec's warm holds the src
                // chain's LAST warm bar — the same seeded slot the scalar
                // steady arm reads; the arm below shadows each bar's src
                // value forward per lane.
                let ck = format_ident!("{}_carry", ident_k);
                let ki = k;
                carry_hoists.push(quote! {
                    let mut #ck = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                });
            }
            Kind::Accum => {
                // Lane running total: the carry slot ONLY — no `mult`/`inv`,
                // because the multiplier is a NODE (volume), not a value.
                let ck = format_ident!("{}_carry", ident_k);
                let ki = k;
                carry_hoists.push(quote! {
                    let mut #ck = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                });
            }
            Kind::RollSum => {
                // Lane ring roll: carry = the window SUM (exec's `st[k]`,
                // seeded per asset exactly like the Sma-family lanes). The
                // group coordinates are ARENA facts the planner publishes —
                // one leader slot drives all L ring instances.
                //
                // A LOCKSTEP (pure-sum) group needs no capacity and no
                // cap-owner flag: its leave comes from the group leader's one
                // fused `feed_periods` call, whose pre-push age read is the
                // eviction at `p == cap` and the old `at(p)` otherwise. A
                // RESCAN group still compares `p == cap` here because it rides
                // the lane-major batched authority, which has no fused form.
                let ck = format_ident!("{}_carry", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                let pk = format_ident!("{}_p", ident_k);
                let gk = format_ident!("{}_grp", ident_k);
                let ki = k;
                if p.plan.is_lockstep(k) {
                    carry_hoists.push(quote! {
                        let mut #ck = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                        let #mk = Simd::<f64, L>::splat(vals.mults[#ki]);
                        let #pk = vals.periods[#ki] as usize;
                        let #gk = vals.ring_of[#ki] as usize;
                    });
                    continue;
                }
                let cp = format_ident!("{}_cap", ident_k);
                let evi = format_ident!("{}_evi", ident_k);
                carry_hoists.push(quote! {
                    let mut #ck = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                    let #mk = Simd::<f64, L>::splat(vals.mults[#ki]);
                    let #pk = vals.periods[#ki] as usize;
                    let #gk = vals.ring_of[#ki] as usize;
                    let #cp = vals.ring_cap[#gk] as usize;
                    let #evi = #pk == #cp;
                });
            }
            Kind::Hma => {
                // Lane HMA: the seven accumulators gather from the per-asset
                // carry vectors (`haccw`); `fd` is fold-only and stays dead
                // here (mirrors the scalar hoist's `let _ = &mut fd`). The
                // two leaving-window pointers are per-asset, gate-anchored
                // like the Sma lanes' `pv` — `sfa` already points at bar
                // `gate`, so `.sub(n)` lands on the window-exit bar.
                let ik = idents[k].clone();
                let (s1, w1) = (format_ident!("{ik}_s1"), format_ident!("{ik}_w1"));
                let (s2, w2) = (format_ident!("{ik}_s2"), format_ident!("{ik}_w2"));
                let (wss, ss) = (format_ident!("{ik}_wss"), format_ident!("{ik}_ss"));
                let (n1, n2, p1, p2f) = (
                    format_ident!("{ik}_n"),
                    format_ident!("{ik}_n2"),
                    format_ident!("{ik}_p1"),
                    format_ident!("{ik}_p2f"),
                );
                let (m1, m2, mp, ww) = (
                    format_ident!("{ik}_m1"),
                    format_ident!("{ik}_m2"),
                    format_ident!("{ik}_mp"),
                    format_ident!("{ik}_ww"),
                );
                let (mm, mm1) = (format_ident!("{ik}_m"), format_ident!("{ik}_m1s"));
                let (pv1, pv2) = (format_ident!("{ik}_pv1"), format_ident!("{ik}_pv2"));
                let s = stream_slot(node.a);
                let sfa = format_ident!("sp{}", s);
                let ki = k;
                carry_hoists.push(quote! {
                    let mut #s1 = Simd::<f64, L>::from_array(std::array::from_fn(|a| haccw[a][#ki][0]));
                    let mut #w1 = Simd::<f64, L>::from_array(std::array::from_fn(|a| haccw[a][#ki][1]));
                    let mut #s2 = Simd::<f64, L>::from_array(std::array::from_fn(|a| haccw[a][#ki][2]));
                    let mut #w2 = Simd::<f64, L>::from_array(std::array::from_fn(|a| haccw[a][#ki][3]));
                    let mut #wss = Simd::<f64, L>::from_array(std::array::from_fn(|a| haccw[a][#ki][4]));
                    let mut #ss = Simd::<f64, L>::from_array(std::array::from_fn(|a| haccw[a][#ki][5]));
                    let #n1 = vals.periods[#ki] as usize;
                    let #n2 = vals.roll_lb[#ki] as usize;
                    let #p1 = Simd::<f64, L>::splat(vals.periods[#ki] as f64);
                    let #p2f = Simd::<f64, L>::splat(vals.roll_lb[#ki] as f64);
                    let #m1 = Simd::<f64, L>::splat(vals.mults[#ki]);
                    let #m2 = Simd::<f64, L>::splat(vals.invms[#ki]);
                    let #mp = Simd::<f64, L>::splat(vals.st_step[#ki]);
                    let #ww = Simd::<f64, L>::splat(vals.consts[#ki]);
                    let #mm = vals.lag_n[#ki] as usize;
                    let #mm1 = #mm - 1usize;
                    let #pv1: [*const f64; L] = std::array::from_fn(|a| unsafe { #sfa[a].sub(#n1) });
                    let #pv2: [*const f64; L] = std::array::from_fn(|a| unsafe { #sfa[a].sub(#n2) });
                });
            }
            Kind::Lag => {
                // Lane Lag depth is the same runtime VALUE as the scalar arm
                // (one per-node `usize`, identical for every lane — the lanes
                // differ in SERIES, never in depth).
                let lk = format_ident!("{}_lag", ident_k);
                carry_hoists.push(quote! { let #lk = vals.lag_n[#ki] as usize; });
            }
            _ => {}
        }
    }
    let mut const_hoists: Vec<TokenStream> = Vec::new();
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::Const {
            let xk = format_ident!("{}", idents[k]);
            const_hoists.push(quote! { let #xk = Simd::<f64, L>::splat(vals.consts[#k]); });
        }
    }

    // The batched lane ring engine: the per-asset warmed `ringw` rings are
    // REPLAYED once at the seam into ONE OF TWO authorities, chosen by the
    // group class the planner publishes on the key (`p.plan`, the same
    // `lowering::ring_facts` that `Signature::build` and `erase` run — this
    // renderer reads the decision, it does not make one):
    // * a pure-SUM group takes `SimdRingSet<L>` — the VECTOR-ELEMENT layout
    //   (slots are whole `Simd<f64, L>` bars, the hand's `cvi_simd` assets
    //   shape): ONE non-mirror store per group per bar, the leave as ONE
    //   vector read, no `[f64; L]` gather/scatter at either end. Legal because
    //   the LOCKSTEP invariant holds: same bar, same age, same cap in every
    //   lane.
    // * a group carrying a `RollMax`/`RollMin` stays lane-major (`RingSetL`,
    //   one `MultiMirrorBuffer` per group) — its rescan triggers are per-lane
    //   and fire on different bars, and only that walk needs one lane's
    //   contiguous mirror window, which the vector layout cannot give. Mirror
    //   law: only min/max buffers mirror; sums never rescan. Measured at the
    //   split's landing: ao lanes 236→147.5 µs, fisher 540→494, fused
    //   ao+fisher 778→548.6 vs the hand pair's 607.9.
    // The replay walks the DISTINCT GROUP LEADERS in arena order (first roll
    // over an interned src — sums and derived extrema share one leader), O
    // (cap·L) per group at the seam. The HMA machine's of-one √p window stays
    // on the per-asset `ringw` (class `BESPOKE`): the batched form measured
    // +48 % slower there (hma lanes 166.9 vs 113.3 µs, same-binary A/B) —
    // batching pays where the L× protocol calls dominate, not where one
    // scalar push inlines into the consumer's bespoke tail.
    // The planner's container choice, honoured not re-derived: a bundled group
    // is fed and read through its bundle's ONE multi-series buffer, so it
    // leaves the per-group list entirely.
    let bundled: Vec<usize> = p.bundles.iter().flat_map(|b| b.groups.clone()).collect();
    let groups: Vec<usize> = p
        .plan
        .leaders(p.nodes.len())
        .into_iter()
        .filter(|&k| p.plan.is_batched(k) && !bundled.contains(&k))
        .collect();
    let mut ring_replay: Vec<TokenStream> = Vec::new();
    {
        for &k in &groups {
            let replay = if p.plan.is_rescan(k) {
                quote! { ringb.feed_batch(g, cap, gate - 1 - age, v); }
            } else {
                quote! {
                    rings.feed(g, cap, gate - 1 - age, Simd::<f64, L>::from_array(v));
                }
            };
            ring_replay.push(quote! {
                {
                    let g = #k;
                    let cap = vals.ring_cap[g] as usize;
                    let lo = vals.feeds[#k] as usize;
                    if gate > lo {
                        let nrep = std::cmp::min(cap, gate - lo);
                        for age in (0..nrep).rev() {
                            let v: [f64; L] =
                                std::array::from_fn(|a| ringw[a].at(g, age));
                            #replay
                        }
                    }
                }
            });
        }
        let mut decls: Vec<TokenStream> = Vec::new();
        // The planner's fused containers: ONE multi-series buffer per bundle.
        // Capacity is the max member period across the bundle (a runtime
        // NUMBER on `Values`, never a pattern fact) and the replay start is the
        // earliest feed. Those two choices are provably safe together: if every
        // group is individually full at the seam, the bundle is too, because
        // `gate - lo_b >= gate - lo_g >= cap_g` for the widest group, and
        // `cap_b = cap_g` for that group.
        for (bi, b) in p.bundles.iter().enumerate() {
            let bu = format_ident!("bu{}", bi);
            let bb = b.groups.len();
            let cap = b
                .groups
                .iter()
                .map(|&g| quote! { vals.ring_cap[#g] as usize })
                .reduce(|acc, e| quote! { std::cmp::max(#acc, #e) })
                .expect("a bundle has at least two groups");
            let lo = b
                .groups
                .iter()
                .map(|&g| quote! { vals.feeds[#g] as usize })
                .reduce(|acc, e| quote! { std::cmp::min(#acc, #e) })
                .expect("a bundle has at least two groups");
            decls.push(quote! {
                let mut #bu = crate::cse::kernel::warmup::BundleRings::<L, #bb>::new(#cap);
            });
            // Every series reads its OWN history; a series whose group is
            // narrower than the bundle clamps to its own last slot. That slot
            // sits deeper than the series ever asks for in the steady loop, so
            // its value is inert — and the clamp is what keeps the read inside
            // the ring (`period_to_idx` wraps by ONE capacity, so a raw
            // bundle-deep age would index out of bounds, not around).
            let fills = b.groups.iter().map(|&g| {
                quote! {
                    Simd::<f64, L>::from_array(std::array::from_fn(|a| {
                        ringw[a].at(#g, age.min(vals.ring_cap[#g] as usize - 1))
                    }))
                }
            });
            ring_replay.push(quote! {
                {
                    let cap = #cap;
                    let lo = #lo;
                    if gate > lo {
                        let nrep = std::cmp::min(cap, gate - lo);
                        for age in (0..nrep).rev() {
                            let v: [Simd<f64, L>; #bb] = [ #(#fills),* ];
                            #bu.fill(v);
                        }
                    }
                }
            });
            ring_replay.push(quote! {
                let mut #bu = #bu.into_warm();
            });
        }
        if groups.iter().any(|&g| p.plan.takes_mirror(g)) {
            decls.push(quote! {
                let mut ringb = crate::cse::kernel::warmup::RingSetL::<L>::new();
            });
            // The replay has filled every rescan-capable group, so the steady
            // loop drives `Warm` (`feed_batch` loses `cap`, the eviction loses
            // its `Option` and the arm its per-bar `expect`). `window()` is
            // unaffected — `get_slices_by_period` is in MultiMirrorBuffer's
            // SHARED `impl<B, S, T>` block, so the mirror's contiguous
            // per-lane slice survives the fill-state change.
            ring_replay.push(quote! {
                let mut ringb = ringb.into_warm();
            });
        }
        if groups.iter().any(|&g| p.plan.is_lockstep(g)) {
            decls.push(quote! {
                let mut rings = crate::cse::kernel::warmup::SimdRingSet::<L>::new();
            });
            // The seam replay has filled every pure-sum group, so the steady
            // loop drives the WARM authority: `into_full` makes fullness a
            // type-level fact, which is what lets `feed` drop the `cap`
            // argument and the eviction drop its `Option` (so the arm below
            // drops a per-bar `expect`). In a debug build a group the replay
            // left short — gate - lo < cap — trips `into_full`'s assert
            // instead of quietly reading a zero slot.
            ring_replay.push(quote! {
                let mut rings = rings.into_warm();
            });
        }
        ring_replay.splice(0..0, decls);
    }

    // ---- steady body, lane form, arena order ----
    let mut body: Vec<TokenStream> = Vec::new();
    for (k, node) in p.nodes.iter().enumerate() {
        let xk = format_ident!("{}", idents[k]);
        let ea = ev(node.a, &idents);
        let eb = ev(node.b, &idents);
        match node.kind {
            Kind::Ln => {
                // Lane ln = the hand's own SIMD polynomial (`ln_unchecked` —
                // the exact call `fisher_simd::calc_fisher` makes; std has no
                // vector log, per-lane libm would be N call sites). Legal
                // un-checked on fisher's domain: val1 clamps to [-0.999,
                // 0.999] ⇒ arg = (1+v)/(1-v) ∈ [~5e-4, ~1999] — never ≤0/
                // NaN/inf/subnormal, so the fast path always runs. ≤ 20 ULP
                // from libm (which the SCALAR arm keeps per §6.41), so this
                // pattern's lanes-vs-scalar gate is 1e-12 RELATIVE.
                body.push(quote! {
                    let #xk = unsafe { crate::math_simd::ln_unchecked(#ea) };
                });
            }
            Kind::Iir => {
                let ck = format_ident!("{}_carry", idents[k]);
                let mk = format_ident!("{}_mult", idents[k]);
                let ik = format_ident!("{}_inv", idents[k]);
                let step = chain_step(&ck, &ik, &mk, &ea);
                body.push(quote! { let #xk = { #step }; });
            }
            Kind::IirClamped => {
                // Text-for-text the hand `fisher_simd::calc_fisher`: the ONE
                // mul_add in its operand order (mult·inner + inv·prev, a/b
                // not exact halves — association is visible), then the
                // nested selects ON THE PRE-CLAMP VALUE (inner `lt` tests nv,
                // like the hand), and the CLAMPED number is what carries.
                let ck = format_ident!("{}_carry", idents[k]);
                let mk = format_ident!("{}_mult", idents[k]);
                let ik = format_ident!("{}_inv", idents[k]);
                let (ht, hv, lt, lv) = (
                    format_ident!("{}_hi_t", idents[k]),
                    format_ident!("{}_hi_v", idents[k]),
                    format_ident!("{}_lo_t", idents[k]),
                    format_ident!("{}_lo_v", idents[k]),
                );
                body.push(quote! {
                    let #xk = {
                        let nv = #mk.mul_add(#ea, #ck * #ik);
                        let v = nv
                            .simd_gt(#ht)
                            .select(#hv, nv.simd_lt(#lt).select(#lv, nv));
                        #ck = v;
                        v
                    };
                });
            }
            Kind::Prev => {
                // Lane delayed copy: emit the slot, shadow this bar's src
                // vector into it (arena order = src already updated).
                let ck = format_ident!("{}_carry", idents[k]);
                body.push(quote! {
                    let #xk = { let o = #ck; #ck = #ea; o };
                });
            }
            Kind::Accum => {
                // Lane running total — one fused instruction per lane on the
                // pre-bound operands (`#ea`/`#eb` are bound once above the
                // match, the same pair the selects use): the multiplier `#eb`
                // is a NODE, and for `ad` it is the volume column.
                let ck = format_ident!("{}_carry", idents[k]);
                body.push(quote! {
                    let #xk = { #ck = #ea.mul_add(#eb, #ck); #ck };
                });
            }
            Kind::RollSum => {
                // Lane ring step — ONE group op per bar on the authority the
                // published class selects. A pure-SUM (lockstep) group takes
                // the vector-element `rings` WITH the fold: the group leader
                // issues ONE `feed_periods` carrying every member's age and
                // each member indexes its own leave out of the returned array.
                // Per group per bar that costs one store, one advance and N
                // pre-push age reads — no `last_t` idempotency branch, no
                // eviction stash, no cap compare (the pre-push read at
                // `p == cap` IS the eviction). A group carrying a
                // `RollMax`/`RollMin` stays on the lane-major `ringb`: its
                // rescan needs one lane's contiguous mirror window, which the
                // vector layout cannot give, so it keeps the hoisted
                // cap-owner flag.
                let ck = format_ident!("{}_carry", idents[k]);
                let mk = format_ident!("{}_mult", idents[k]);
                let gk = format_ident!("{}_grp", idents[k]);
                if p.plan.is_lockstep(k) {
                    let ld = p.plan.leader[k] as usize;
                    // The planner's container decision, read not re-derived: a
                    // group inside a bundle pushes through the bundle's ONE
                    // multi-series buffer instead of `rings`. The fused call is
                    // issued at the bundle's FIRST member (the plan guarantees
                    // every bundled source precedes it) and costs one push and
                    // one advance for the whole bundle; each member then folds
                    // against its own `(series, rank)` leave, which is the same
                    // bar the per-group fold would have read.
                    let fused = p
                        .bundles
                        .iter()
                        .position(|b| b.groups.contains(&ld))
                        .map(|bi| (bi, &p.bundles[bi]));
                    if let Some((bi, bundle)) = fused {
                        let bu = format_ident!("bu{}", bi);
                        let buv = format_ident!("buv{}", bi);
                        let s = bundle.series_of(ld).expect("group is in the bundle");
                        let j = bundle
                            .members
                            .iter()
                            .position(|&m| m == k)
                            .expect("member is in the bundle");
                        let bb = bundle.groups.len();
                        let nn = bundle.members.len();
                        if j == 0 {
                            let vals: Vec<TokenStream> = bundle
                                .groups
                                .iter()
                                .map(|&g| {
                                    let ea_g = ev(p.nodes[g].a, &idents);
                                    quote! { #ea_g }
                                })
                                .collect();
                            let ages: Vec<TokenStream> = bundle
                                .members
                                .iter()
                                .map(|&m| {
                                    let pm = format_ident!("{}_p", idents[m]);
                                    quote! { #pm }
                                })
                                .collect();
                            body.push(quote! {
                                let #buv: [[Simd<f64, L>; #nn]; #bb] =
                                    #bu.feed([#(#vals),*], [#(#ages),*]);
                                let #xk = { #ck = #ck + (#ea - #buv[#s][#j]); #ck * #mk };
                            });
                        } else {
                            body.push(quote! {
                                let #xk = { #ck = #ck + (#ea - #buv[#s][#j]); #ck * #mk };
                            });
                        }
                        continue;
                    }
                    let members = p.plan.members_of(ld, p.nodes.len());
                    let idx = members
                        .iter()
                        .position(|&m| m == k)
                        .expect("roll is a member of its own group");
                    let gv = format_ident!("gv{}", ld);
                    let n = members.len();
                    let ages: Vec<TokenStream> = members
                        .iter()
                        .map(|&m| {
                            let pm = format_ident!("{}_p", idents[m]);
                            quote! { #pm }
                        })
                        .collect();
                    if p.plan.is_leader(k) {
                        body.push(quote! {
                            let #gv: [Simd<f64, L>; #n] =
                                rings.feed_periods(#gk, #ea, [#(#ages),*]);
                            let #xk = { #ck = #ck + (#ea - #gv[#idx]); #ck * #mk };
                        });
                    } else {
                        body.push(quote! {
                            let #xk = { #ck = #ck + (#ea - #gv[#idx]); #ck * #mk };
                        });
                    }
                    continue;
                }
                let pk = format_ident!("{}_p", idents[k]);
                let evi = format_ident!("{}_evi", idents[k]);
                body.push(quote! {
                    let #xk = {
                        ringb.feed_batch(#gk, gate + i, #ea.to_array());
                        let leaves = if #evi {
                            ringb.evicted_batch(#gk)
                        } else {
                            ringb.at_batch(#gk, #pk)
                        };
                        #ck = #ck + (#ea - Simd::<f64, L>::from_array(leaves));
                        #ck * #mk
                    };
                });
            }
            Kind::Hma => {
                // Lane HMA — the scalar statement-for-statement mirror:
                // two WMA steps, `2·o2 − o1`, then the tail's EXACT
                // divide-before-subtract order. Per-lane ring ops: feed
                // `diff` into each asset's own ring, read age m−1 after
                // the push (the hand's post-push `front()` slot).
                let ik = idents[k].clone();
                let (s1, w1) = (format_ident!("{ik}_s1"), format_ident!("{ik}_w1"));
                let (s2, w2) = (format_ident!("{ik}_s2"), format_ident!("{ik}_w2"));
                let (wss, ss) = (format_ident!("{ik}_wss"), format_ident!("{ik}_ss"));
                let (p1, p2f) = (format_ident!("{ik}_p1"), format_ident!("{ik}_p2f"));
                let (m1, m2, mp, ww) = (
                    format_ident!("{ik}_m1"),
                    format_ident!("{ik}_m2"),
                    format_ident!("{ik}_mp"),
                    format_ident!("{ik}_ww"),
                );
                let (mm, mm1) = (format_ident!("{ik}_m"), format_ident!("{ik}_m1s"));
                let (pv1, pv2) = (format_ident!("{ik}_pv1"), format_ident!("{ik}_pv2"));
                let ki = k;
                body.push(quote! {
                    let #xk = {
                        let old1 = Simd::<f64, L>::from_array(std::array::from_fn(
                            |a| unsafe { *#pv1[a].add(i) },
                        ));
                        #w1 = #w1 - #s1;
                        #s1 = #s1 + (#ea - old1);
                        #w1 = #w1 + #ea * #p1;
                        let o1 = #w1 * #m1;
                        let old2 = Simd::<f64, L>::from_array(std::array::from_fn(
                            |a| unsafe { *#pv2[a].add(i) },
                        ));
                        #w2 = #w2 - #s2;
                        #s2 = #s2 + (#ea - old2);
                        #w2 = #w2 + #ea * #p2f;
                        let o2 = #w2 * #m2;
                        let diff = Simd::<f64, L>::splat(2.0f64) * o2 - o1;
                        #wss = #wss + diff * #mp;
                        #ss = #ss + diff;
                        let ds = diff.to_array();
                        for a in 0..L {
                            ringw[a].feed(#ki, #mm, gate + i, ds[a]);
                        }
                        let out = #wss / #ww;
                        #wss = #wss - #ss;
                        let mut olds = [0.0f64; L];
                        for a in 0..L {
                            olds[a] = ringw[a].at(#ki, #mm1);
                        }
                        #ss = #ss - Simd::<f64, L>::from_array(olds);
                        out
                    };
                });
            }
            Kind::Input => {
                // `node.a` IS the stream slot for an Input node (the scalar
                // arm reads `s{node.a}` directly) — `gather(node.a)` would
                // double-index it as a node reference.
                let sfa = format_ident!("sp{}", node.a);
                body.push(quote! {
                    let #xk = Simd::<f64, L>::from_array(std::array::from_fn(
                        |a| unsafe { *#sfa[a].add(i) },
                    ));
                });
            }
            Kind::Const => {}
            Kind::Ema => {
                let ck = format_ident!("{}_carry", idents[k]);
                let mk = format_ident!("{}_mult", idents[k]);
                let ik = format_ident!("{}_inv", idents[k]);
                let step = chain_step(&ck, &ik, &mk, &ea);
                body.push(quote! { let #xk = { #step }; });
            }
            Kind::Wilders => {
                let ck = format_ident!("{}_carry", idents[k]);
                let mk = format_ident!("{}_mult", idents[k]);
                let ik = format_ident!("{}_inv", idents[k]);
                let step = chain_step(&ck, &mk, &ik, &ea);
                body.push(quote! { let #xk = { #step }; });
            }
            Kind::Sub | Kind::Add | Kind::Mul | Kind::Div | Kind::Max | Kind::Min => {
                let v = shape.binop(&node.kind, &ea, &eb);
                body.push(quote! { let #xk = #v; });
            }
            Kind::Abs => {
                let v = shape.abs(&ea);
                body.push(quote! { let #xk = #v; });
            }
            Kind::Mirror => {
                // identity copy — state mirror (see the scalar arm); the
                // sink graph never reads it, LLVM DCEs the dead lane copy.
                body.push(quote! { let #xk = #ea; });
            }
            Kind::Fma => {
                let ec = ev(node.c.expect("Fma: c operand"), &idents);
                body.push(quote! { let #xk = #ea.mul_add(#eb, #ec); });
            }
            Kind::Lag => {
                // Depth is the hoisted VALUE (`vals.lag_n[k]`), not a literal —
                // `sp{s}` is pre-sliced at `gate`, so every lane reads
                // `bar gate + i − n`. Legality is enforced in `erase`.
                let s = stream_slot(node.a);
                let sfa = format_ident!("sp{}", s);
                let lk = format_ident!("{}_lag", idents[k]);
                body.push(quote! {
                    let #xk = Simd::<f64, L>::from_array(std::array::from_fn(
                        |a| unsafe { *#sfa[a].add(i).sub(#lk) },
                    ));
                });
            }
            Kind::WhereGe => {
                let ec = ev(node.c.expect("WhereGe: t operand"), &idents);
                body.push(quote! {
                    let #xk = #ea.simd_ge(#eb).select(#ec, Simd::<f64, L>::splat(0.0f64));
                });
            }
            Kind::WhereLt => {
                let ec = ev(node.c.expect("WhereLt: t operand"), &idents);
                body.push(quote! {
                    let #xk = #ea.simd_lt(#eb).select(Simd::<f64, L>::splat(0.0f64), #ec);
                });
            }
            Kind::WhereGt => {
                let ec = ev(node.c.expect("WhereGt: t operand"), &idents);
                body.push(quote! {
                    let #xk = #ea.simd_gt(#eb).select(#ec, Simd::<f64, L>::splat(0.0f64));
                });
            }
            Kind::WhereLe => {
                // Branchless, and NaN-correct the same way the scalar arm is:
                // `simd_le` is false for a NaN `a`, so the VALUE lane runs and
                // poisons — the hand's early-return guard, not a zero-fill.
                let ec = ev(node.c.expect("WhereLe: t operand"), &idents);
                body.push(quote! {
                    let #xk = #ea.simd_le(#eb).select(Simd::<f64, L>::splat(0.0f64), #ec);
                });
            }
            Kind::Sma => {
                let ck = format_ident!("{}_carry", idents[k]);
                let mk = format_ident!("{}_mult", idents[k]);
                let pv = format_ident!("{}_prevp", idents[k]);
                body.push(quote! {
                    let #xk = {
                        let prev = Simd::<f64, L>::from_array(std::array::from_fn(
                            |a| unsafe { *#pv[a].add(i) },
                        ));
                        #ck = #ck + (#ea - prev);
                        #ck * #mk
                    };
                });
            }
            Kind::Stddev => {
                let ck = format_ident!("{}_carry", idents[k]);
                let sq = format_ident!("{}_sq", idents[k]);
                let mk = format_ident!("{}_mult", idents[k]);
                let pv = format_ident!("{}_prevp", idents[k]);
                body.push(quote! {
                    let #xk = {
                        let prev = Simd::<f64, L>::from_array(std::array::from_fn(
                            |a| unsafe { *#pv[a].add(i) },
                        ));
                        #ck = #ck + (#ea - prev);
                        let mean = #ck * #mk;
                        #sq = #sq + #ea.mul_add(#ea, -(prev * prev));
                        let var = #sq.mul_add(#mk, -(mean * mean));
                        var.sqrt().simd_max(Simd::<f64, L>::splat(f64::EPSILON))
                    };
                });
            }
            Kind::Wma => {
                let ck = format_ident!("{}_carry", idents[k]);
                let wc = format_ident!("{}_weighted", idents[k]);
                let nk = format_ident!("{}_n", idents[k]);
                let wgt = format_ident!("{}_wgt", idents[k]);
                let pv = format_ident!("{}_prevp", idents[k]);
                body.push(quote! {
                    let #xk = {
                        let prev = Simd::<f64, L>::from_array(std::array::from_fn(
                            |a| unsafe { *#pv[a].add(i) },
                        ));
                        #wc = #wc - #ck;
                        #ck = #ck + (#ea - prev);
                        #wc = #wc + #ea * #nk;
                        #wc * #wgt
                    };
                });
            }
            Kind::RollMax | Kind::RollMin => {
                let vc = format_ident!("{}_carry", idents[k]);
                let tc = format_ident!("{}_trail", idents[k]);
                let wb = format_ident!("{}_wb", idents[k]);
                let wn = format_ident!("{}_win", idents[k]);
                let pk = format_ident!("{}_p", idents[k]);
                let is_max = node.kind == Kind::RollMax;
                let finder = if is_max {
                    quote! { crate::indicators::max::find_max_simd::<4> }
                } else {
                    quote! { crate::indicators::min::find_min_simd::<4> }
                };
                let ge_or_le = if is_max {
                    quote! { simd_ge }
                } else {
                    quote! { simd_le }
                };
                if p.plan.is_ring(k) {
                    // Derived source (fisher): each lane's OWN ring instance
                    // feeds once per bar (idempotent per instance — sibling
                    // members calling is the arena's normal), then the
                    // pointer arm's EXACT trail machine: +1, trigger mask,
                    // fold select. Only DUE lanes walk a window — each its
                    // own lane's contiguous mirror slice via `RingSet::window`
                    // — the hand `min_chuncked` mask dispatch; same finder,
                    // same `trail = lb − ri` rule, cannot drift.
                    let gk = format_ident!("{}_grp", idents[k]);
                    let lbk = format_ident!("{}_lb", idents[k]);
                    body.push(quote! {
                        let #xk = {
                            ringb.feed_batch(#gk, gate + i, #ea.to_array());
                            #tc = #tc + Simd::<usize, L>::splat(1usize);
                            let resc = Simd::<usize, L>::splat(#pk).simd_le(#tc);
                            let fold = #ea.#ge_or_le(#vc);
                            let mut nv = fold.select(#ea, #vc);
                            let mut nt = fold.select(Simd::<usize, L>::splat(0usize), #tc);
                            let mask = resc.to_bitmask();
                            if mask != 0 {
                                let wins = ringb.window(#gk, #lbk + 1);
                                let mut va = nv.to_array();
                                let mut ta = nt.to_array();
                                let mut lane = 0usize;
                                while lane < L {
                                    if mask & (1u64 << lane) != 0 {
                                        let (rv, ri) = #finder(wins[lane]);
                                        va[lane] = rv;
                                        ta[lane] = #lbk - ri;
                                    }
                                    lane += 1;
                                }
                                nv = Simd::<f64, L>::from_array(va);
                                nt = Simd::<usize, L>::from_array(ta);
                            }
                            #vc = nv;
                            #tc = nt;
                            #vc
                        };
                    });
                    continue;
                }
                body.push(quote! {
                    let #xk = {
                        #tc = #tc + Simd::<usize, L>::splat(1usize);
                        let resc = Simd::<usize, L>::splat(#pk).simd_le(#tc);
                        let fold = #ea.#ge_or_le(#vc);
                        let mut nv = fold.select(#ea, #vc);
                        let mut nt = fold.select(Simd::<usize, L>::splat(0usize), #tc);
                        let mask = resc.to_bitmask();
                        if mask != 0 {
                            let mut va = nv.to_array();
                            let mut ta = nt.to_array();
                            let mut lane = 0usize;
                            while lane < L {
                                if mask & (1u64 << lane) != 0 {
                                    let w = unsafe {
                                        ::core::slice::from_raw_parts(#wb[lane].add(i), #wn)
                                    };
                                    let (rv, ri) = #finder(w);
                                    va[lane] = rv;
                                    ta[lane] = #wn - 1 - ri;
                                }
                                lane += 1;
                            }
                            nv = Simd::<f64, L>::from_array(va);
                            nt = Simd::<usize, L>::from_array(ta);
                        }
                        #vc = nv;
                        #tc = nt;
                        #vc
                    };
                });
            }
            Kind::Trail => {
                let roll_tc = format_ident!("{}_trail", idents[node.a]);
                body.push(quote! {
                    let #xk = Simd::<f64, L>::from_array(#roll_tc.to_array().map(|t| t as f64));
                });
            }
            Kind::SuperTrend => {
                let stb = format_ident!("stb{}", k);
                let stc_ub = format_ident!("stcub{}", k);
                let stc_lb = format_ident!("stclb{}", k);
                let tr = format_ident!("tr{}", k);
                let step = format_ident!("st_step{}", k);
                let ec = ev(node.c.expect("SuperTrend: close operand"), &idents);
                // The branchless form of the SAME block the scalar kernel
                // runs: every value each branch could produce is computed,
                // selects pick per lane. max/min/select are exact, so the
                // lanes agree bit-for-bit with the scalar branch version.
                body.push(quote! {
                    let w = #step * #ea;
                    let ub0 = #eb + w;
                    let lb0 = #eb - w;
                    let crosses_up = #ec.simd_gt(#stb);
                    let crosses_down = #ec.simd_lt(#stb);
                    let trend = crosses_up
                        | (#tr.simd_ne(Simd::<f64, L>::splat(0.0f64)) & !crosses_down);
                    let lb_c = #stc_lb.simd_max(lb0);
                    let ub_c = #stc_ub.simd_min(ub0);
                    let stv = trend.select(lb_c, ub_c);
                    let #xk = stv;
                    #tr = trend.select(Simd::<f64, L>::splat(1.0f64), Simd::<f64, L>::splat(0.0f64));
                    #stb = stv;
                    #stc_lb = trend.select(lb_c, lb0);
                    #stc_ub = trend.select(ub0, ub_c);
                });
            }
        }
    }

    // ---- lane store arrays + scatter stores ----
    let mut row_arrays: Vec<TokenStream> = Vec::new();
    let mut always_stores: Vec<TokenStream> = Vec::new();
    let mut opt_stores: Vec<TokenStream> = Vec::new();
    for (gidx, members) in group_members.iter().enumerate() {
        let leader = members[0];
        let ident_g = idents[p.sinks[leader]].clone();
        let q_g = format_ident!("row_{}_{}", ident_g, gidx);
        let off_g = format_ident!("off{}", gidx);
        let g_minl = format_ident!("minl_{}_{}", ident_g, gidx);
        row_arrays.push(quote! {
            let #q_g: [*mut f64; L] = std::array::from_fn(|a| unsafe {
                base.add(a * total + #off_g + (gate - #g_minl))
            });
        });
        let e = ev(p.sinks[leader], &idents);
        let scatter = quote! {
            {
                let va = #e.to_array();
                let mut lane = 0usize;
                while lane < L {
                    unsafe { *#q_g[lane].add(i) = va[lane]; }
                    lane += 1;
                }
            }
        };
        if group_is_optional[gidx] {
            let want_g = format_ident!("want{}", gidx);
            opt_stores.push(quote! { if #want_g { #scatter } });
        } else {
            always_stores.push(scatter);
        }
    }

    // ---- RowRef windows per asset (one shared arena, asset-major) ----
    let mut row_refs: Vec<TokenStream> = Vec::new();
    for sidx in 0..p.sinks.len() {
        let gidx = group_of[sidx];
        let leader = group_members[gidx][0];
        let ident_g = idents[p.sinks[leader]].clone();
        let off_g = format_ident!("off{}", gidx);
        let rsz_g = format_ident!("rsz{}", gidx);
        let g_minl = format_ident!("minl_{}_{}", ident_g, gidx);
        let l_s = format_ident!("{}_{}", idents[p.sinks[sidx]], sidx);
        row_refs.push(quote! {
            if vals.sink_want[#sidx] {
                crate::cse::query::RowRef::window(
                    &arena,
                    a * total + #off_g + (#l_s - #g_minl),
                    #rsz_g.saturating_sub(#l_s - #g_minl),
                )
            } else {
                crate::cse::query::RowRef::window(&arena, 0, 0)
            }
        });
    }
    let n_sinks = p.sinks.len();
    let warm_ptrs: Vec<TokenStream> = (0..n_sinks)
        .map(|sidx| {
            let gidx = group_of[sidx];
            let leader = group_members[gidx][0];
            let ident_g = idents[p.sinks[leader]].clone();
            let off_g = format_ident!("off{}", gidx);
            let g = format_ident!("minl_{}_{}", ident_g, gidx);
            let l_s = format_ident!("{}_{}", idents[p.sinks[sidx]], sidx);
            quote! {
                ptrs[#sidx] = unsafe { base.add(a * total + (#off_g + (#l_s - #g))) };
            }
        })
        .collect();

    let steady = if opt_group_ids.is_empty() {
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
        let body_plain = body.clone();
        let stores_plain = always_stores.clone();
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
    };

    quote! {
        #[allow(unused_variables, unused_mut, unused_parens, clippy::needless_range_loop)]
        pub fn #name<const L: usize>(
            vals: &crate::cse::Values,
            streams: &[&[&[f64]]; L],
        ) -> Vec<Vec<crate::cse::query::RowRef>> {
            use ::std::simd::Simd;
            #[allow(unused_imports)]
            use ::std::simd::num::{SimdFloat, SimdInt};
            #[allow(unused_imports)]
            use ::std::simd::Select;
            #[allow(unused_imports)]
            use ::std::simd::cmp::{SimdPartialEq, SimdPartialOrd};
            #[allow(unused_imports)]
            use ::std::simd::StdFloat;
            let n = streams
                .iter()
                .flat_map(|ass| ass.iter().map(|s| s.len()))
                .min()
                .unwrap_or(0);
            let gate = (vals.max_gate as usize + 1).min(n);
            #( #lead_hoists )*
            #( #group_minl )*
            #( #group_want )*
            #( #group_rsz )*
            #( #off_decl )*
            let total = #total_expr;
            let total_all = total * L;
            let mut arc: std::sync::Arc<[std::mem::MaybeUninit<f64>]> =
                std::sync::Arc::new_uninit_slice(total_all);
            let base: *mut f64 = if total_all == 0 {
                std::ptr::null_mut()
            } else {
                std::sync::Arc::get_mut(&mut arc)
                    .expect("fresh arena is uniquely owned")
                    .as_mut_ptr()
                    .cast::<f64>()
            };
            let arena: std::sync::Arc<[f64]> = unsafe { arc.assume_init() };
            // Warm [0, gate): the SAME scalar interpreter, once per asset —
            // the seed/lead authority stays single, per-asset carries land in
            // the Vec below for the lane gather.
            let mut stw: Vec<[f64; crate::cse::MAX]> = vec![[0.0f64; crate::cse::MAX]; L];
            let mut stbw: Vec<[f64; crate::cse::MAX]> = vec![[0.0f64; crate::cse::MAX]; L];
            let mut stcubw: Vec<[f64; crate::cse::MAX]> = vec![[0.0f64; crate::cse::MAX]; L];
            let mut stlbw: Vec<[f64; crate::cse::MAX]> = vec![[0.0f64; crate::cse::MAX]; L];
            let mut sumsqw: Vec<[f64; crate::cse::MAX]> = vec![[0.0f64; crate::cse::MAX]; L];
            let mut wsumw: Vec<[f64; crate::cse::MAX]> = vec![[0.0f64; crate::cse::MAX]; L];
            let mut trailw: Vec<[usize; crate::cse::MAX]> = vec![[0usize; crate::cse::MAX]; L];
            // The ring/HMA carry states are PERSISTENT per asset: the warm
            // exec fills them, the steady lane arms read the same instances
            // (mirrors the scalar kernel, where one `rings`/`hacc` lives for
            // the whole run). Non-ring patterns pay only the Vec allocation.
            let mut ringw: Vec<crate::cse::kernel::warmup::RingSet> =
                (0..L)
                    .map(|_| crate::cse::kernel::warmup::RingSet::new())
                    .collect();
            let mut haccw: Vec<[[f64; 7]; crate::cse::MAX]> =
                vec![[[0.0f64; 7]; crate::cse::MAX]; L];
            if n > 0 {
                for a in 0..L {
                    let mut ptrs: [*mut f64; crate::cse::MAX_SINKS] =
                        [::core::ptr::null_mut(); crate::cse::MAX_SINKS];
                    #( #warm_ptrs )*
                    let mut v = [0.0f64; crate::cse::MAX];
                    crate::cse::kernel::exec::<#sig, true>(
                        vals,
                        streams[a],
                        0,
                        gate,
                        &mut stw[a],
                        &mut stbw[a],
                        &mut stcubw[a],
                        &mut stlbw[a],
                        &mut sumsqw[a],
                        &mut wsumw[a],
                        &mut trailw[a],
                        &mut ringw[a],
                        &mut haccw[a],
                        &mut v,
                        &mut ptrs,
                        &vals.sink_lead,
                    );
                }
            }
            if gate >= n {
                return std::array::from_fn::<Vec<crate::cse::query::RowRef>, L, _>(|a| {
                    vec![#( #row_refs ),*]
                })
                .to_vec();
            }
            #( #ring_replay )*
            #( #stream_arr_hoists )*
            #( #row_arrays )*
            #( #carry_hoists )*
            #( #const_hoists )*
            #steady
            std::array::from_fn::<Vec<crate::cse::query::RowRef>, L, _>(|a| {
                vec![#( #row_refs ),*]
            })
            .to_vec()
        }
    }
}
