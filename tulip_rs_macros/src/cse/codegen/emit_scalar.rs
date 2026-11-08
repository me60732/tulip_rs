//! Scalar emitter: builds the main kernel function body for scalar CSE patterns.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::*;

// Additional imports from shared module needed for emit_scalar
use crate::cse::codegen::classify::Classified;
use crate::cse::codegen::shared::{
    emitter_context, scalar_row_ref_token, scalar_warm_ptr_token, sink_window_symbols,
};

/// Build hoists for scalar pattern.
fn build_scalar_hoists(
    p: &Pattern,
    idents: &[String],
    classify: &Classified,
) -> (
    Vec<TokenStream>, // pair_hoists
    Vec<TokenStream>, // scalar_hoists
    Vec<TokenStream>, // roll_hoists
    Vec<TokenStream>, // lag_hoists
    Vec<TokenStream>, // stream_hoists
    Vec<TokenStream>, // stb_hoists
    Vec<TokenStream>, // stc_ub_hoists
    Vec<TokenStream>, // stc_lb_hoists
    Vec<TokenStream>, // st_trend_hoists
    Vec<TokenStream>, // st_step_hoists
) {
    let mut pair_hoists = Vec::new();
    for (gi, (k0, k1, _stream)) in classify.groups.iter().enumerate() {
        let g = format_ident!("g{gi}");
        let gm = format_ident!("g{gi}_m");
        let gi_v = format_ident!("g{gi}_i");
        let (k0, k1) = (*k0, *k1);
        // For 4-lane groups: hoist all 4 slots; for 2-lane: just the pair
        // Find all slots in this group by scanning slot_group
        let mut group_slots: Vec<usize> = classify
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
    for k in classify.scalars.keys() {
        let ident_k = idents[*k].clone();
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

    // IirClampedPlain clamp values: load from consts at expansion time.
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::IirClampedPlain {
            let ident_k = idents[k].clone();
            let ck = format_ident!("{}_carry", ident_k);
            let mk = format_ident!("{}_mult", ident_k);
            let ik = format_ident!("{}_inv", ident_k);
            let ht = format_ident!("{}_hi_t", ident_k);
            let hv = format_ident!("{}_hi_v", ident_k);
            let lt = format_ident!("{}_lo_t", ident_k);
            let lv = format_ident!("{}_lo_v", ident_k);
            let (b, c, d, e) = (node.b, node.c.unwrap_or(0), node.d, node.e);
            let ki = k;
            scalar_hoists.push(quote! {
                let mut #ck = st[#ki];
                let #mk = vals.mults[#ki];
                let #ik = vals.invms[#ki];
                let #ht = vals.consts[#b];
                let #hv = vals.consts[#c];
                let #lt = vals.consts[#d];
                let #lv = vals.consts[#e];
            });
        }
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
    let mut ring_single_feeder: Vec<bool> = vec![false; p.nodes.len()];
    for g in p.plan.leaders(p.nodes.len()) {
        let members = p.plan.members_of(g, p.nodes.len());
        if members.len() == 1 {
            ring_single_feeder[members[0]] = true;
        }
    }

    let mut roll_hoists = Vec::new();
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::RollSum {
            let ident_k = idents[k].clone();
            let ck = format_ident!("{}_carry", ident_k);
            let mk = format_ident!("{}_mult", ident_k);
            let gk = format_ident!("{}_grp", ident_k);
            let ki = k;
            if ring_single_feeder[k] {
                // Single-feeder groups (e.g. stoch's two RollSum chains) are
                // structural cap owners in steady mode. Keep only the fields
                // actually used by that path.
                roll_hoists.push(quote! {
                    let mut #ck = st[#ki];
                    let #mk = vals.mults[#ki];
                    let #gk = vals.ring_of[#ki] as usize;
                });
            } else {
                let pk = format_ident!("{}_p", ident_k);
                let cp = format_ident!("{}_cap", ident_k);
                let evi = format_ident!("{}_evi", ident_k);
                roll_hoists.push(quote! {
                    let mut #ck = st[#ki];
                    let #mk = vals.mults[#ki];
                    let #pk = vals.periods[#ki] as usize;
                    let #gk = vals.ring_of[#ki] as usize;
                    let #cp = vals.ring_cap[#gk] as usize;
                    let #evi = #pk == #cp;
                });
            }
            continue;
        }
        if node.kind == Kind::Hma {
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
            Kind::Iir | Kind::IirClamped | Kind::Prev | Kind::Accum | Kind::VariableAlpha
        ) {
            continue;
        }
        let ident_k = idents[k].clone();
        let ck = format_ident!("{}_carry", ident_k);
        let ki = k;
        roll_hoists.push(quote! { let mut #ck = st[#ki]; });
        if matches!(node.kind, Kind::Prev | Kind::Accum | Kind::VariableAlpha) {
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

    // Input base pointers hoisted: steady-loop reads are unchecked raw loads
    // (the slice index's bounds check is provably-always-true to no one but
    // us — n is the min stream length — so LLVM keeps it).
    let mut used_streams: Vec<usize> = Vec::new();
    for sc in classify.scalars.values() {
        if let Scalar::Price(s) = sc {
            if !used_streams.contains(&s) {
                used_streams.push(*s);
            }
        }
    }
    for (k0, _, src_slot) in &classify.groups {
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
        // A third child edge. SuperTrend's close was the only kind handled here,
        // but `erase` also puts a CHILD in the 4th slot for the four selects (their
        // selected value `t` — `signature.rs:452 (K_WHEREGE, a, b, t)`) and for `Fma`
        // (its addend, `:522 (K_FMA, x, y, z)`), and the renderer's own arms demand it
        // (`node.c.expect("WhereLt: t operand")`, `node.c.expect("Fma: c operand")`).
        // A column that reaches the arena ONLY through that slot therefore got read by
        // the steady loop without ever being hoisted: `recipes::obv` is the first shape
        // to hit it (its volume is the `t` of both selects), and it failed to compile
        // with `s1` undeclared — `s3` once `tr` widened the arena to four columns. The
        // index differed because the slot is the column's RANK, which is exactly what
        // the column-declared remap assigns.
        if matches!(
            node.kind,
            Kind::SuperTrend
                | Kind::WhereGe
                | Kind::WhereGt
                | Kind::WhereLe
                | Kind::WhereLt
                | Kind::Fma
                | Kind::KvoFlow
                | Kind::NviRecur
                | Kind::PviRecur
        ) {
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
        // `KvoFlow`'s volume (d=volume) and `NviRecur`/`PviRecur`'s prev_volume
        // (d=prev_volume) reach the arena ONLY through the plain `d` slot, which
        // (unlike `c`) is never an `Option` and so was never checked by any of
        // the hoisting logic above -- the same unhoisted-stream bug class the
        // `c`-operand comment above already documents, just on a different slot.
        if matches!(node.kind, Kind::KvoFlow | Kind::NviRecur | Kind::PviRecur) {
            let d_is_input = p
                .nodes
                .get(node.d)
                .map(|n| matches!(n.kind, Kind::Input))
                .unwrap_or(false);
            if d_is_input {
                let stream = p.nodes[node.d].a;
                if !used_streams.contains(&stream) {
                    used_streams.push(stream);
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

    (
        pair_hoists,
        scalar_hoists,
        roll_hoists,
        lag_hoists,
        stream_hoists,
        stb_hoists,
        stc_ub_hoists,
        stc_lb_hoists,
        st_trend_hoists,
        st_step_hoists,
    )
}

/// Build scalar body: the main computation loop for all node kinds.
fn build_scalar_body(p: &Pattern, idents: &[String], classify: &Classified) -> Vec<TokenStream> {
    let mut body = Vec::new();

    // Lockstep RollSum groups with >1 members can share one ring feed per bar.
    // We precompute leader/member maps so the first member emits the shared
    // feed+leave block and siblings just consume their precomputed `leave`.
    let mut rollsum_group_members: Vec<Option<Vec<usize>>> = vec![None; p.nodes.len()];
    let mut rollsum_group_leader_of: Vec<Option<usize>> = vec![None; p.nodes.len()];
    for g in p.plan.leaders(p.nodes.len()) {
        if !p.plan.is_lockstep(g) {
            continue;
        }
        let members: Vec<usize> = p
            .plan
            .members_of(g, p.nodes.len())
            .into_iter()
            .filter(|&m| p.nodes[m].kind == Kind::RollSum)
            .collect();
        if members.len() <= 1 {
            continue;
        }
        let leader = members[0];
        rollsum_group_members[leader] = Some(members.clone());
        for m in members {
            rollsum_group_leader_of[m] = Some(leader);
        }
    }

    // Ring groups with exactly one member have exactly one feeder per bar;
    // steady code can call the no-idempotency-check fast path.
    let mut ring_single_feeder: Vec<bool> = vec![false; p.nodes.len()];
    for g in p.plan.leaders(p.nodes.len()) {
        let members = p.plan.members_of(g, p.nodes.len());
        if members.len() == 1 {
            ring_single_feeder[members[0]] = true;
        }
    }

    for (k, node) in p.nodes.iter().enumerate() {
        let xk = format_ident!("{}", idents[k]);
        match node.kind {
            Kind::Input => {
                // Read from the appropriate stream pointer
                let stream = node.a;
                let sp = format_ident!("s{}", stream);
                body.push(quote! { let #xk = unsafe { *#sp.add(i) }; });
            }
            Kind::Const => { /* hoisted below */ }
            Kind::Ema => {
                if let Some(&(gi, lane1)) = classify.slot_group.get(&k) {
                    // For packed mode: lane1=false means this is the first statement of a group
                    // (which binds all lanes); lane1=true means skip (bound by first statement)
                    if lane1 {
                        continue; // bound with lane0's statement
                    }
                    let (k0, k1, _stream) = classify.groups[gi];
                    let ga = format_ident!("ga{gi}");
                    let (k0, k1) = (k0, k1);
                    let g = format_ident!("g{gi}");
                    let gm = format_ident!("g{gi}_m");
                    let gi_v = format_ident!("g{gi}_i");
                    // Determine width by scanning slot_group for all slots in this group
                    let mut group_slots: Vec<usize> = classify
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
                    let src = match classify.scalars.get(&k) {
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
                            classify.scalars.len()
                        ),
                    };
                    let mk = format_ident!("{}_mult", ident_k);
                    let ik = format_ident!("{}_inv", ident_k);
                    let step = chain_step(&ck, &ik, &mk, &src);
                    body.push(quote! { let #xk = { #step }; });
                }
            }
            Kind::SeededEma => {
                let ident_k = idents[k].clone();
                let ck = format_ident!("{}_carry", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                let ik = format_ident!("{}_inv", ident_k);
                let src = ev(node.b, &idents);
                let step = chain_step(&ck, &ik, &mk, &src);
                body.push(quote! { let #xk = { #step }; });
            }
            Kind::Sub | Kind::Add | Kind::Mul | Kind::Div | Kind::Max | Kind::Min => {
                let (ea, eb) = (ev(node.a, &idents), ev(node.b, &idents));
                let v = Shape { lanes: false }.binop(&node.kind, &ea, &eb);
                body.push(quote! { let #xk = #v; });
            }
            Kind::Abs => {
                let ea = ev(node.a, &idents);
                let v = Shape { lanes: false }.abs(&ea);
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
                if let Some(&(gi, lane1)) = classify.slot_group.get(&k) {
                    if lane1 {
                        continue; // bound by lane0's group statement
                    }
                    let mut gslots: Vec<usize> = classify
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
            Kind::Sqrt => {
                // Single-operand sqrt host call — `K_SQRT`. Used by recipes
                // that need to compute std dev from variance manually (e.g.
                // volatility over derived rate-of-change values).
                let src = ev(node.a, &idents);
                body.push(quote! { let #xk = #src.sqrt(); });
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
                //
                // Optimization (engine-native): for lockstep RollSum groups
                // with multiple members, emit exactly one `feed` on the first
                // member and materialize all member leaves once. Siblings then
                // reuse those leaves instead of repeating idempotent `feed`
                // calls.
                let ident_k = idents[k].clone();
                let ck = format_ident!("{}_carry", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                let pk = format_ident!("{}_p", ident_k);
                let gk = format_ident!("{}_grp", ident_k);
                let evi = format_ident!("{}_evi", ident_k);

                if let Some(group_leader) = rollsum_group_leader_of[k] {
                    let group_members = rollsum_group_members[group_leader]
                        .as_ref()
                        .expect("rollsum group leader must have members");
                    let leader_ident = idents[group_leader].clone();
                    let leader_gk = format_ident!("{}_grp", leader_ident);
                    let src_var = format_ident!("roll_src_{}", group_leader);
                    let leave_k = format_ident!("roll_leave_{}", k);

                    if k == group_leader {
                        let leader_src = ev(p.nodes[group_leader].a, idents);
                        let leave_binds: Vec<TokenStream> = group_members
                            .iter()
                            .map(|&m| {
                                let mid = idents[m].clone();
                                let mp = format_ident!("{}_p", mid);
                                let mevi = format_ident!("{}_evi", mid);
                                let mleave = format_ident!("roll_leave_{}", m);
                                quote! {
                                    let #mleave = if #mevi {
                                        rings.evicted(#leader_gk)
                                    } else {
                                        rings.at(#leader_gk, #mp)
                                    };
                                }
                            })
                            .collect();
                        body.push(quote! {
                            let #src_var = #leader_src;
                            rings.feed(#leader_gk, t, #src_var);
                            #( #leave_binds )*
                        });
                    }

                    body.push(quote! {
                        let #xk = {
                            let x = #src_var;
                            let leave = #leave_k;
                            #ck += x - leave;
                            #ck * #mk
                        };
                    });
                } else {
                    let src = ev(node.a, &idents);
                    let leave = if ring_single_feeder[k] {
                        quote! { rings.feed_unique_take_evicted(#gk, x) }
                    } else {
                        quote! {
                            if #evi {
                                rings.feed_take_evicted(#gk, t, x)
                            } else {
                                rings.feed_take_at(#gk, t, x, #pk)
                            }
                        }
                    };
                    body.push(quote! {
                        let #xk = {
                            let x = #src;
                            // One push per group per bar (idempotent feed for
                            // shared groups, unique-feed fast path for single
                            // feeders), then the hand's post-push leave value:
                            // eviction for cap owner, age read for siblings.
                            let leave = #leave;
                            #ck += x - leave;
                            #ck * #mk
                        };
                    });
                }
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
                let leave = if ring_single_feeder[k] {
                    quote! { rings.feed_unique_take_at(#ki, diff, #mm1) }
                } else {
                    quote! { rings.feed_take_at(#ki, t, diff, #mm1) }
                };
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
                        let leave = #leave;
                        let out = #wss / #ww;
                        #wss -= #ss;
                        #ss -= leave;
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
                    let ge_or_le = if is_max {
                        quote! { >= }
                    } else {
                        quote! { <= }
                    };
                    let feed = if ring_single_feeder[k] {
                        quote! { rings.feed_unique(#gk, current); }
                    } else {
                        quote! { rings.feed(#gk, t, current); }
                    };
                    body.push(quote! {
                        let #xk = {
                            let current = #src;
                            #feed
                            #tc = #tc + 1;
                            if #pk <= #tc {
                                let (rv, ri) = #finder(rings.window(#gk, #lbk + 1));
                                #vc = rv;
                                #tc = #lbk - ri;
                            } else if current #ge_or_le #vc {
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
            Kind::IirClampedPlain => {
                // Plain IIR clamped: scalar plain multiply+add (NO mul_add fusion).
                let ident_k = idents[k].clone();
                let ck = format_ident!("{}_carry", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                let ik = format_ident!("{}_inv", ident_k);
                let src = ev(node.a, &idents);
                let ht = format_ident!("{}_hi_t", ident_k);
                let hv = format_ident!("{}_hi_v", ident_k);
                let lt = format_ident!("{}_lo_t", ident_k);
                let lv = format_ident!("{}_lo_v", ident_k);
                body.push(quote! {
                    let #xk = {
                        let mut nv = #mk * #src + #ik * #ck;
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
            Kind::VariableAlpha => {
                // EMA-style carry whose blend weight is a LIVE per-bar node
                // value (node.c), not a recipe-time constant -- so unlike
                // Ema/SeededEma there is no mults/invms hoist, only the carry
                // slot. No seed-bar branch is needed for validity (the steady
                // loop only ever runs at t > feed), BUT the Vidya shape's
                // hand (vidya.rs::init_state) spells its FIRST published bar
                // with plain multiply/add, not mul_add -- every later bar
                // (cycle's steady calc) uses mul_add. A genuine one-bar
                // spelling asymmetry in the hand itself, not a recipe choice.
                // SIG.d (node.d) selects the fold shape: 0=Ema, 1=Vidya.
                let ident_k = idents[k].clone();
                let ck = format_ident!("{}_carry", ident_k);
                let ki = k;
                let src = ev(node.b, &idents);
                let w = ev(node.c.expect("VariableAlpha requires c (mult)"), &idents);
                let step = if node.d == 0 {
                    quote! { #ck = #ck.mul_add(1.0 - #w, #src * #w); #ck }
                } else {
                    quote! {
                        #ck = if t == vals.feeds[#ki] as usize + 1 {
                            (#src - #ck) * #w + #ck
                        } else {
                            (#src - #ck).mul_add(#w, #ck)
                        };
                        #ck
                    }
                };
                body.push(quote! {
                    let #xk = { #step };
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
                if let Some(&(gi, lane1)) = classify.slot_group.get(&k) {
                    if lane1 {
                        continue; // bound with lane0's group statement
                    }
                    let mut group_slots: Vec<usize> = classify
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
            Kind::HighPass => {
                // One-pole HighPass recurrence (`K_HIGHPASS`): carry is
                // `hp_acc[k] = [y1, prev_real]`, re-derived from `warmup.rs`'s
                // `K_HIGHPASS` arm verbatim. Steady bars are all past the
                // chain's feed, so no gating is needed here (exec's warm
                // call already carried `hp_acc[k]` up to the seam).
                let ki = k;
                let real = ev(node.a, &idents);
                body.push(quote! {
                    let #xk = {
                        let real = #real;
                        let [y1, prev_real] = hp_acc[#ki];
                        let y = vals.mults[#ki].mul_add(y1, vals.invms[#ki] * (real - prev_real));
                        hp_acc[#ki] = [y, real];
                        y
                    };
                });
            }
            Kind::SuperSmoother => {
                // Two-pole SuperSmoother recurrence (`K_SUPERSMOOTHER`),
                // transcribed from `warmup.rs`'s arm verbatim; carry is
                // `ss_acc[k] = [y1, y2, prev_real]`.
                let ki = k;
                let real = ev(node.a, &idents);
                body.push(quote! {
                    let #xk = {
                        let real = #real;
                        let [y1, y2, prev_real] = ss_acc[#ki];
                        let y = vals.st_step[#ki].mul_add(
                            real + prev_real,
                            vals.mults[#ki].mul_add(y1, vals.invms[#ki] * y2),
                        );
                        ss_acc[#ki] = [y, y1, real];
                        y
                    };
                });
            }
            Kind::RollAge => {
                // Ring-backed age roll (`K_ROLLAGE`): push this bar, publish
                // the eviction. Steady bars are all past the feed, so the
                // ring is always full here (no `unwrap_or(0.0)` needed, same
                // convention as `RollSum`'s steady arm dropping the `Option`).
                let ki = k;
                let src = ev(node.a, &idents);
                let leave = if ring_single_feeder[k] {
                    quote! { rings.feed_unique_take_evicted(g, x) }
                } else {
                    quote! { rings.feed_take_evicted(g, t, x) }
                };
                body.push(quote! {
                    let #xk = {
                        let x = #src;
                        let g = vals.ring_of[#ki] as usize;
                        #leave
                    };
                });
            }
            Kind::HtInPhase => {
                // Centre tap of the shared 7-tap ring (`K_HTINPHASE`).
                let ki = k;
                let src = ev(node.a, &idents);
                let mid = if ring_single_feeder[k] {
                    quote! { rings.feed_unique_take_at(g, x, 3) }
                } else {
                    quote! { rings.feed_take_at(g, t, x, 3) }
                };
                body.push(quote! {
                    let #xk = {
                        let x = #src;
                        let g = vals.ring_of[#ki] as usize;
                        #mid
                    };
                });
            }
            Kind::HtQuadrature => {
                // FIR sum over the shared 7-tap ring (`K_HTQUADRATURE`),
                // transcribed from `warmup.rs`'s arm verbatim. Shares the
                // SAME ring group as the paired `HtInPhase` node -- `feed` is
                // idempotent per (group, bar), so pushing from both nodes in
                // the same bar is safe (one push wins, the rest are no-ops).
                let ki = k;
                let src = ev(node.a, &idents);
                let feed = if ring_single_feeder[k] {
                    quote! { rings.feed_unique(g, x); }
                } else {
                    quote! { rings.feed(g, t, x); }
                };
                body.push(quote! {
                    let #xk = {
                        let x = #src;
                        let g = vals.ring_of[#ki] as usize;
                        #feed
                        let q1 = 0.0962_f64.mul_add(rings.at(g, 0), 0.5769 * rings.at(g, 2));
                        let q2 = (-0.5769_f64).mul_add(rings.at(g, 4), -0.0962 * rings.at(g, 6));
                        q1 + q2
                    };
                });
            }
            Kind::MeanAbsDev => {
                // Mean Absolute Deviation rescan over a shared sma operand
                // (`K_MEANABSDEV`): `a`=src, `b`=sma. Raw vs ring source is a
                // COMPILE-TIME fact here (the node's own `a` operand kind),
                // mirroring `RollMax`/`RollMin`'s derived-vs-raw split.
                let ki = k;
                let sma_v = ev(node.b, &idents);
                if p.nodes[node.a].kind == Kind::Input {
                    let stream_idx = p.nodes[node.a].a;
                    body.push(quote! {
                        let #xk = {
                            let p = vals.periods[#ki] as usize;
                            let sma = #sma_v;
                            let mult = 1.0 / p as f64;
                            let base = unsafe { streams[#stream_idx].as_ptr() };
                            let start = t + 1 - p;
                            let window = unsafe { std::slice::from_raw_parts(base.add(start), p) };
                            crate::indicators::md::calc_md_simd::<4>(window, sma, mult)
                        };
                    });
                } else {
                    let src_v = ev(node.a, &idents);
                    let feed = if ring_single_feeder[k] {
                        quote! { rings.feed_unique(g, x); }
                    } else {
                        quote! { rings.feed(g, t, x); }
                    };
                    body.push(quote! {
                        let #xk = {
                            let p = vals.periods[#ki] as usize;
                            let sma = #sma_v;
                            let mult = 1.0 / p as f64;
                            let g = vals.ring_of[#ki] as usize;
                            let x = #src_v;
                            #feed
                            let window = rings.window(g, p);
                            crate::indicators::md::calc_md_simd::<4>(window, sma, mult)
                        };
                    });
                }
            }
            Kind::LinregFlow => {
                // linreg's moment carry (`K_LINREGFLOW`), publish-before-evict:
                // `out = st + x*mult; st = out - v[sub]`. `a`=src, `b`=sub.
                let ki = k;
                let src = ev(node.a, &idents);
                let sub_v = ev(node.b, &idents);
                body.push(quote! {
                    let #xk = {
                        let x = #src;
                        let m = vals.mults[#ki];
                        let out = st[#ki] + x * m;
                        st[#ki] = out - #sub_v;
                        out
                    };
                });
            }
            Kind::NviRecur => {
                // Negative Volume Index recurrence (`K_NVI`): `a`=close,
                // `b`=prev_close, `c`=volume, `d`=prev_volume. Steady bars
                // are all past the seed bar (exec's warm call already seeded
                // `st[k] = 1000.0` and carried it to the seam).
                let ki = k;
                let close = ev(node.a, &idents);
                let prev_close = ev(node.b, &idents);
                let volume = ev(node.c.expect("NviRecur: volume operand"), &idents);
                let prev_volume = ev(node.d, &idents);
                body.push(quote! {
                    let #xk = {
                        if #volume < #prev_volume {
                            st[#ki] = #close / #prev_close * st[#ki];
                        }
                        st[#ki]
                    };
                });
            }
            Kind::PviRecur => {
                // Positive Volume Index recurrence (`K_PVI`), the mirror
                // gate of `NviRecur`.
                let ki = k;
                let close = ev(node.a, &idents);
                let prev_close = ev(node.b, &idents);
                let volume = ev(node.c.expect("PviRecur: volume operand"), &idents);
                let prev_volume = ev(node.d, &idents);
                body.push(quote! {
                    let #xk = {
                        if #volume > #prev_volume {
                            st[#ki] = #close / #prev_close * st[#ki];
                        }
                        st[#ki]
                    };
                });
            }
            Kind::KvoFlow => {
                // Bespoke KVO flow machine (`K_KVOFLOW`): `a`=high, `b`=low,
                // `c`=close, `d`=volume. Carry `kvacc[k] = [prev_hlc, cm,
                // prev_hl]`; the trend flip-flop bool rides `st[k]`. Steady
                // bars are all past the seed bar (exec's warm call already
                // seeded `kvacc[k]`/`st[k]` from bar 0 vs bar 1 and carried
                // them to the seam) -- transcribed from `warmup.rs`'s arm
                // verbatim, minus the one-time `t == feed` seed branch.
                let ki = k;
                let high = ev(node.a, &idents);
                let low = ev(node.b, &idents);
                let close = ev(node.c.expect("KvoFlow: close operand"), &idents);
                let volume = ev(node.d, &idents);
                body.push(quote! {
                    let #xk = {
                        let hlc = #high + #low + #close;
                        let dm = #high - #low;
                        let [prev_hlc, cm, prev_hl] = kvacc[#ki];
                        let mut sign;
                        let mut cm = cm;
                        if st[#ki] < 0.5 && hlc > prev_hlc {
                            st[#ki] = 1.0;
                            cm = prev_hl;
                            sign = 1.0;
                        } else if st[#ki] >= 0.5 && hlc < prev_hlc {
                            st[#ki] = 0.0;
                            cm = prev_hl;
                            sign = -1.0;
                        } else {
                            sign = if st[#ki] >= 0.5 { 1.0 } else { -1.0 };
                        }
                        cm += dm.max(f64::EPSILON);
                        kvacc[#ki] = [hlc, cm, dm];
                        (dm / cm).mul_add(2.0, -1.0).abs() * #volume * 100.0 * sign
                    };
                });
            }
            Kind::Psar => {
                // Parabolic SAR (`K_PSAR`): bespoke 3-field carry in
                // kvacc[k] = [psar, extream, accel], trend bool in st[k]
                // (1.0/0.0). Steady bars are all past the seed bar (bar 1
                // -- exec's warm call already seeded kvacc[k]/st[k] and
                // carried them to the seam). Transcribed verbatim from
                // warmup.rs's K_PSAR steady arm (the hand's calc_unchecked).
                let ki = k;
                let stream_h = p.nodes[node.a].a;
                let stream_l = p.nodes[node.b].a;
                let sph = format_ident!("s{}", stream_h);
                let spl = format_ident!("s{}", stream_l);
                body.push(quote! {
                    let #xk = {
                        let afs = vals.mults[#ki];
                        let maxaf = vals.invms[#ki];
                        let h = unsafe { *#sph.add(i) };
                        let ph = unsafe { *#sph.add(i).sub(1) };
                        let oh = unsafe { *#sph.add(i).sub(2) };
                        let l = unsafe { *#spl.add(i) };
                        let pl = unsafe { *#spl.add(i).sub(1) };
                        let ol = unsafe { *#spl.add(i).sub(2) };
                        let [mut psar, mut extream, mut accel] = kvacc[#ki];
                        let mut uptrend = st[#ki] != 0.0;
                        psar = accel.mul_add(extream - psar, psar);
                        if uptrend {
                            if psar > ol {
                                psar = ol;
                            }
                            if psar > pl {
                                psar = pl;
                            }
                            if h > extream {
                                extream = h;
                                accel = (accel + afs).min(maxaf);
                            }
                        } else {
                            if psar < oh {
                                psar = oh;
                            }
                            if psar < ph {
                                psar = ph;
                            }
                            if l < extream {
                                extream = l;
                                accel = (accel + afs).min(maxaf);
                            }
                        }
                        if (uptrend && l < psar) || (!uptrend && h > psar) {
                            uptrend = !uptrend;
                            psar = extream;
                            accel = afs;
                            extream = if uptrend { h } else { l };
                        }
                        st[#ki] = if uptrend { 1.0 } else { 0.0 };
                        kvacc[#ki] = [psar, extream, accel];
                        psar
                    };
                });
            }
            Kind::SdftRotate => {
                // Sliding DFT rotation carry for `msw` (`K_SDFTROTATE`):
                // coupled 2-state carry in kvacc[k] = [rp, ip, lead_sine].
                // Steady bars are always past the seed (warmup.rs's K_SDFTROTATE
                // arm already seeded kvacc[k] and carried it to the seam), so
                // this is pure recurrence, no branch -- same standing as
                // LinregFlow's steady arm.
                let ki = k;
                let stream_s = p.nodes[node.a].a;
                let sps = format_ident!("s{}", stream_s);
                body.push(quote! {
                    let #xk = {
                        let wr = vals.mults[#ki];
                        let wi = vals.invms[#ki];
                        let period = vals.periods[#ki] as usize;
                        let new = unsafe { *#sps.add(i) };
                        let old = unsafe { *#sps.add(i).sub(period) };
                        let [rp, ip, _lead] = kvacc[#ki];
                        let rp2 = wr.mul_add(rp, -(wi * ip)) + (new - old);
                        let ip2 = wr.mul_add(ip, wi * rp);
                        let (sine, lead) = crate::indicators::msw::phase_from_rp_ip(rp2, ip2);
                        kvacc[#ki] = [rp2, ip2, lead];
                        sine
                    };
                });
            }
            Kind::AdaptiveMsw => {
                // Adaptive MESA Sine Wave DFT over a live period from `dc`.
                // `node.a` = raw source Input slot, `node.b` = dc node slot.
                let ki = k;
                let stream_s = p.nodes[node.a].a;
                let dc = ev(node.b, &idents);
                body.push(quote! {
                    let #xk = {
                        let t_abs = t;
                        let period = ((#dc + 0.5) as usize).clamp(6, (t_abs + 1).min(50));
                        let start = t_abs + 1 - period;
                        let window = &streams[#stream_s][start..=t_abs];
                        let (cos_tw, sin_tw) = crate::indicators::msw::twiddles_for_period(period);
                        let (rp, ip) = crate::indicators::msw::dot_product_simd::<8>(window, cos_tw, sin_tw);
                        let (sine, lead) = crate::indicators::msw::phase_from_rp_ip(rp, ip);
                        kvacc[#ki][2] = lead;
                        sine
                    };
                });
            }
            Kind::SdftLead => {
                // Thin companion read of its owner's cached lead_sine -- the
                // owner (node.a, an arena id) always has a FRESH kvacc value
                // this same bar because topological order places it earlier
                // in the k-loop (see `Node::SdftLead` doc).
                let owner_ki = node.a;
                body.push(quote! { let #xk = kvacc[#owner_ki][2]; });
            }
            Kind::HomodyneDiscriminator => {
                // Bespoke, non-shared 38-slot carry (K_HOMODYNEDISCRIMINATOR):
                // steady bars are always past every stage threshold (warmup's
                // interpreter already filled every ring and ran the
                // discriminator update up to the seam), so this is pure
                // unconditional recurrence -- `hd_step` transcribed verbatim
                // from `State<Warm>::calc`, same standing as LinregFlow's and
                // SdftRotate's steady arms.
                let ki = k;
                let src = ev(node.a, &idents);
                body.push(quote! {
                    let #xk = crate::indicators::homodynediscriminator::hd_step(
                        &mut hdacc[#ki],
                        #src,
                    );
                });
            }
            Kind::Cybercycle => {
                // Scalar steady recurrence. `s*` pointers are gate-anchored, so at
                // steady bar `i` (absolute `gate+i`) the past samples are `sub(1..5)`.
                let ki = k;
                let stream = p.nodes[node.a].a;
                let sp = format_ident!("s{}", stream);
                body.push(quote! {
                    let #xk = {
                        let coef = vals.mults[#ki];
                        let d1 = vals.invms[#ki];
                        let d2 = (d1 * 0.5) * (d1 * 0.5);
                        let [cycle_prev, cycle_prev2, _] = kvacc[#ki];

                        let price = unsafe { *#sp.add(i) };
                        let price_1 = unsafe { *#sp.add(i).sub(1) };
                        let p2 = unsafe { *#sp.add(i).sub(2) };
                        let p3 = unsafe { *#sp.add(i).sub(3) };
                        let p4 = unsafe { *#sp.add(i).sub(4) };
                        let p5 = unsafe { *#sp.add(i).sub(5) };

                        let smooth_ab = 2.0_f64.mul_add(price_1, price);
                        let smooth_cd = 2.0_f64.mul_add(p2, p3);
                        let smooth = (smooth_ab + smooth_cd) * (1.0 / 6.0);

                        let smooth_1_ab = 2.0_f64.mul_add(p2, price_1);
                        let smooth_1_cd = 2.0_f64.mul_add(p3, p4);
                        let smooth_1 = (smooth_1_ab + smooth_1_cd) * (1.0 / 6.0);

                        let smooth_2_ab = 2.0_f64.mul_add(p3, p2);
                        let smooth_2_cd = 2.0_f64.mul_add(p4, p5);
                        let smooth_2 = (smooth_2_ab + smooth_2_cd) * (1.0 / 6.0);

                        let smooth_diff = (-2.0_f64).mul_add(smooth_1, smooth) + smooth_2;
                        let cycle = coef.mul_add(smooth_diff, d1.mul_add(cycle_prev, -d2 * cycle_prev2));

                        kvacc[#ki] = [cycle, cycle_prev, 0.0];
                        cycle
                    };
                });
            }
            Kind::PeakDecay => {
                // PeakDecay envelope (`K_PEAKDECAY`): `peak = max(prev*decay, src)`.
                let ki = k;
                let src = ev(node.a, &idents);
                body.push(quote! {
                    let #xk = {
                        let decay = vals.mults[#ki];
                        st[#ki] = (st[#ki] * decay).max(#src);
                        st[#ki]
                    };
                });
            }
            Kind::InstantaneousTrendline => {
                // Ehlers Instantaneous Trendline recurrence.
                // Uses embedded HomodyneDiscriminator pipeline internally, then applies
                // 2-pole IIR with adaptive α = 2/(DC+1) computed live per bar from HD's smooth_period.
                // Steady bars are always past every warmup stage (warmup.rs already ran the full
                // HD pipeline up to the seam), so this is pure unconditional recurrence.
                let ki = k;
                let src = ev(node.a, &idents); // Current price from input stream
                body.push(quote! {
                    let #xk = crate::indicators::instantaneoustrendline::it_step(
                        &mut hdacc[#ki],
                        &mut kvacc[#ki],
                        #src,
                    );
                });
            }
            Kind::Mama => {
                // Ehlers MAMA/FAMA adaptive EMA recurrence.
                // Uses embedded HomodyneDiscriminator pipeline internally, then applies
                // adaptive α from phase delta to MAMA/FAMA.
                let ki = k;
                let src = ev(node.a, &idents); // Current price from input stream
                body.push(quote! {
                    let #xk = {
                        let (mama, _fama, alpha) = crate::indicators::mama::mama_step_with_alpha(
                            &mut hdacc[#ki],
                            &mut kvacc[#ki],
                            vals.consts[#ki],  // fast_limit
                            vals.st_step[#ki], // slow_limit
                            #src,
                        );
                        st[#ki] = alpha;
                        mama
                    };
                });
            }
            Kind::Fama => {
                // FAMA secondary output - reads Mama's output and Mama's alpha from st[src].
                let ki = k;
                let mama_slot = node.a as usize; // Mama's slot index
                let mama_ident = format_ident!("{}", idents[mama_slot]);
                body.push(quote! {
                    let #xk = {
                        let mama = #mama_ident;
                        let alpha = st[#mama_slot];
                        let fama_prev = kvacc[#ki][0];
                        let half_alpha = alpha.algebraic_mul(0.5);
                        let fama = half_alpha.mul_add(mama - fama_prev, fama_prev);
                        kvacc[#ki][0] = fama;
                        fama
                    };
                });
            }
        }
    }
    body
}

/// Build const hoists for Const nodes.
fn build_scalar_const_hoists(p: &Pattern, idents: &[String]) -> Vec<TokenStream> {
    let mut const_hoists = Vec::new();
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::Const {
            // Hoisted const uses its ident directly
            let xk = format_ident!("{}", idents[k]);
            const_hoists.push(quote! { let #xk = vals.consts[#k]; });
        }
    }
    const_hoists
}

/// Build stores: per-group store pointers + per-group store writes.
fn build_scalar_stores(
    p: &Pattern,
    idents: &[String],
    ctx: &crate::cse::codegen::shared::EmitterContext,
) -> (
    Vec<TokenStream>, // group_stores
    Vec<TokenStream>, // always_stores
    Vec<TokenStream>, // opt_stores
) {
    let mut group_stores: Vec<TokenStream> = Vec::new();
    let mut always_stores: Vec<TokenStream> = Vec::new();
    let mut opt_stores: Vec<TokenStream> = Vec::new();

    for (gidx, members) in ctx.group_members.iter().enumerate() {
        let leader = members[0];
        let ident_g = idents[p.sinks[leader]].clone();
        let q_g = format_ident!("row_{}_{}", ident_g, gidx);
        let off_g = format_ident!("off{}", gidx);
        let g_minl = format_ident!("minl_{}_{}", ident_g, gidx);

        group_stores.push(quote! {
            let #q_g = unsafe { base.add(#off_g + (gate - #g_minl)) };
        });

        let e = ev(p.sinks[leader], &idents);
        if ctx.group_is_optional[gidx] {
            let want_g = format_ident!("want{}", gidx);
            opt_stores.push(quote! { unsafe { if #want_g { *#q_g.add(i) = #e; } } });
        } else {
            always_stores.push(quote! { unsafe { *#q_g.add(i) = #e; } });
        }
    }

    (group_stores, always_stores, opt_stores)
}

/// Build returns: row refs and final return statement.
fn build_scalar_returns(
    p: &Pattern,
    idents: &[String],
    ctx: &crate::cse::codegen::shared::EmitterContext,
) -> Vec<TokenStream> {
    let mut row_refs: Vec<TokenStream> = Vec::new();
    for sidx in 0..p.sinks.len() {
        let sym = sink_window_symbols(p, &idents, &ctx.group_members, &ctx.group_of, sidx);
        row_refs.push(scalar_row_ref_token(&sym, sidx));
    }
    row_refs
}

/// Build early return (n==0 case).
fn build_scalar_early_return(p: &Pattern) -> TokenStream {
    let mut zero_row_refs: Vec<TokenStream> = Vec::new();
    for _sidx in 0..p.sinks.len() {
        zero_row_refs.push(quote! { crate::cse::query::RowRef::window(&arena, 0, 0) });
    }
    quote! {
        let arc: std::sync::Arc<[std::mem::MaybeUninit<f64>]> = std::sync::Arc::new_uninit_slice(0usize);
        let arena: std::sync::Arc<[f64]> = unsafe { arc.assume_init() };
        return vec![
            #(#zero_row_refs),*
        ];
    }
}

/// Build arena allocation.
fn build_scalar_arena_alloc(ctx: &crate::cse::codegen::shared::EmitterContext) -> TokenStream {
    let ng = ctx.group_members.len();
    if ng == 0 {
        quote! {
            let arc: std::sync::Arc<[std::mem::MaybeUninit<f64>]> = std::sync::Arc::new_uninit_slice(0usize);
            let base: *mut f64 = std::ptr::null_mut();
            let arena: std::sync::Arc<[f64]> = unsafe { arc.assume_init() };
        }
    } else {
        let total_expr = &ctx.total_expr;
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
    }
}

/// Build warm pointers setup.
fn build_scalar_warm_ptrs(
    p: &Pattern,
    idents: &[String],
    ctx: &crate::cse::codegen::shared::EmitterContext,
) -> Vec<TokenStream> {
    let mut warm_ptrs: Vec<TokenStream> = Vec::new();
    for sidx in 0..p.sinks.len() {
        let sym = sink_window_symbols(p, &idents, &ctx.group_members, &ctx.group_of, sidx);
        warm_ptrs.push(scalar_warm_ptr_token(&sym, sidx));
    }
    warm_ptrs
}

/// Main orchestrator: builds the kernel function by invoking stage helpers in sequence.
pub(super) fn run_fn(p: &Pattern) -> TokenStream {
    let name = format_ident!("{}", p.name);
    let sig = format_ident!("SIG_{}", name);

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

    // Build the shared emitter context (sink/window setup)
    let ctx = emitter_context(p, &idents);

    // Stage 1: Build hoists
    let (
        pair_hoists,
        scalar_hoists,
        roll_hoists,
        lag_hoists,
        stream_hoists,
        stb_hoists,
        stc_ub_hoists,
        stc_lb_hoists,
        st_trend_hoists,
        st_step_hoists,
    ) = build_scalar_hoists(p, &idents, &c);

    // Stage 2: Build body
    let body = build_scalar_body(p, &idents, &c);

    // Stage 3a: Build const hoists (after body so they're in reverse order)
    let const_hoists = build_scalar_const_hoists(p, &idents);

    // Stage 3b: Build stores
    let (group_stores, always_stores, opt_stores) = build_scalar_stores(p, &idents, &ctx);

    // Stage 4: Build returns and early return case
    let row_refs = build_scalar_returns(p, &idents, &ctx);
    let early_return = build_scalar_early_return(p);
    let arena_alloc = build_scalar_arena_alloc(&ctx);
    let warm_ptrs = build_scalar_warm_ptrs(p, &idents, &ctx);

    // Extract ctx fields for use in quote blocks
    let lead_hoists = &ctx.lead_hoists;
    let group_minl = &ctx.group_minl;
    let group_want = &ctx.group_want;
    let group_rsz = &ctx.group_rsz;
    let off_decl = &ctx.off_decl;

    // Build steady loop
    let steady =
        steady_with_optional_groups(&ctx.opt_group_ids, &body, &always_stores, &opt_stores);

    // Warmup scratch families that can be pattern-pruned.
    let needs_supertrend = p.nodes.iter().any(|n| n.kind == Kind::SuperTrend);
    let needs_stddev = p.nodes.iter().any(|n| n.kind == Kind::Stddev);
    let needs_wma = p.nodes.iter().any(|n| n.kind == Kind::Wma);
    let needs_hma = p.nodes.iter().any(|n| n.kind == Kind::Hma);
    let needs_highpass = p.nodes.iter().any(|n| n.kind == Kind::HighPass);
    let needs_supersmoother = p.nodes.iter().any(|n| n.kind == Kind::SuperSmoother);
    let needs_hdacc = p.nodes.iter().any(|n| {
        matches!(
            n.kind,
            Kind::HomodyneDiscriminator | Kind::InstantaneousTrendline | Kind::Mama
        )
    });

    // Heuristic guard (refined): keep full local scratch only for DERIVED
    // extrema (ring-backed RollMax/RollMin). Price-sourced extrema (e.g. stoch)
    // can use dummy scratch without paying large local zero-init arrays.
    let force_full_scratch = p
        .nodes
        .iter()
        .enumerate()
        .any(|(k, n)| matches!(n.kind, Kind::RollMax | Kind::RollMin) && p.plan.is_ring(k));

    let needs_supertrend = needs_supertrend || force_full_scratch;
    let needs_stddev = needs_stddev || force_full_scratch;
    let needs_wma = needs_wma || force_full_scratch;
    let needs_hma = needs_hma || force_full_scratch;
    let needs_highpass = needs_highpass || force_full_scratch;
    let needs_supersmoother = needs_supersmoother || force_full_scratch;
    let needs_hdacc = needs_hdacc || force_full_scratch;

    let supertrend_decl = if needs_supertrend {
        quote! {
            let mut stb = [0.0f64; crate::cse::MAX]; // SuperTrend prev_st
            let mut stc_ub = [0.0f64; crate::cse::MAX]; // SuperTrend prev_ub
            let mut stlb = [0.0f64; crate::cse::MAX]; // SuperTrend prev_lb
        }
    } else {
        quote! {}
    };
    let sumsq_decl = if needs_stddev {
        quote! { let mut sumsq = [0.0f64; crate::cse::MAX]; } // Stddev sum-of-squares carry
    } else {
        quote! {}
    };
    let wsum_decl = if needs_wma {
        quote! { let mut wsum = [0.0f64; crate::cse::MAX]; } // Wma weighted-sum carry
    } else {
        quote! {}
    };
    let hacc_decl = if needs_hma {
        quote! { let mut hacc = [[0.0f64; 7]; crate::cse::MAX]; }
    } else {
        quote! {}
    };
    let hp_acc_decl = if needs_highpass {
        quote! { let mut hp_acc = [[0.0f64; 2]; crate::cse::MAX]; }
    } else {
        quote! {}
    };
    let ss_acc_decl = if needs_supersmoother {
        quote! { let mut ss_acc = [[0.0f64; 3]; crate::cse::MAX]; }
    } else {
        quote! {}
    };
    let hdacc_decl = if needs_hdacc {
        quote! { let mut hdacc = [[0.0f64; 38]; crate::cse::MAX]; }
    } else {
        quote! {}
    };

    let stb_arg = if needs_supertrend {
        quote! { &mut stb }
    } else {
        quote! { &mut __dummies.stb }
    };
    let stc_ub_arg = if needs_supertrend {
        quote! { &mut stc_ub }
    } else {
        quote! { &mut __dummies.stc_ub }
    };
    let stlb_arg = if needs_supertrend {
        quote! { &mut stlb }
    } else {
        quote! { &mut __dummies.stlb }
    };
    let sumsq_arg = if needs_stddev {
        quote! { &mut sumsq }
    } else {
        quote! { &mut __dummies.sumsq }
    };
    let wsum_arg = if needs_wma {
        quote! { &mut wsum }
    } else {
        quote! { &mut __dummies.wsum }
    };
    let hacc_arg = if needs_hma {
        quote! { &mut hacc }
    } else {
        quote! { &mut __dummies.hacc }
    };
    let hp_acc_arg = if needs_highpass {
        quote! { &mut hp_acc }
    } else {
        quote! { &mut __dummies.hp_acc }
    };
    let ss_acc_arg = if needs_supersmoother {
        quote! { &mut ss_acc }
    } else {
        quote! { &mut __dummies.ss_acc }
    };
    let hdacc_arg = if needs_hdacc {
        quote! { &mut hdacc }
    } else {
        quote! { &mut __dummies.hd_acc }
    };

    let use_dummy_scratch = !needs_supertrend
        || !needs_stddev
        || !needs_wma
        || !needs_hma
        || !needs_highpass
        || !needs_supersmoother
        || !needs_hdacc;

    let warm_exec = if use_dummy_scratch {
        quote! {
            crate::cse::kernel::warmup::with_exec_scratch_dummies(|__dummies| {
                crate::cse::kernel::exec::<#sig, true>(
                    vals,
                    streams,
                    0,
                    gate,
                    &mut st,
                    #stb_arg,
                    #stc_ub_arg,
                    #stlb_arg,
                    #sumsq_arg,
                    #wsum_arg,
                    &mut trail,
                    &mut rings,
                    #hacc_arg,
                    &mut kvacc,
                    #hp_acc_arg,
                    #ss_acc_arg,
                    #hdacc_arg,
                    &mut v,
                    &mut ptrs,
                    &vals.sink_lead,
                );
            });
        }
    } else {
        quote! {
            crate::cse::kernel::exec::<#sig, true>(
                vals,
                streams,
                0,
                gate,
                &mut st,
                #stb_arg,
                #stc_ub_arg,
                #stlb_arg,
                #sumsq_arg,
                #wsum_arg,
                &mut trail,
                &mut rings,
                #hacc_arg,
                &mut kvacc,
                #hp_acc_arg,
                #ss_acc_arg,
                #hdacc_arg,
                &mut v,
                &mut ptrs,
                &vals.sink_lead,
            );
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
            #(#stream_hoists)*

            let mut st = [0.0f64; crate::cse::MAX];
            #supertrend_decl
            #sumsq_decl
            #wsum_decl
            let mut trail = [0usize; crate::cse::MAX]; // roll trail carry
            let mut rings = crate::cse::kernel::warmup::RingSet::new();
            #hacc_decl
            // Shared 3-field bespoke carry (kind-specific meaning; e.g.
            // Mama uses [mama_prev, fama_prev, prev_phase], alpha rides st[k]).
            let mut kvacc = [[0.0f64; 3]; crate::cse::MAX];
            #hp_acc_decl
            #ss_acc_decl
            #hdacc_decl
            let mut v = [0.0f64; crate::cse::MAX];
            // Warm-up bars: shared interpreter, cold path once per request.
            #warm_exec
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
            #(#group_stores)*

            // Steady loop hoists (pair)
            #(#pair_hoists)*

            // Steady loop hoists (scalar)
            #(#scalar_hoists)*

            // SuperTrend hoists
            #(#stb_hoists)*
            #(#stc_ub_hoists)*
            #(#stc_lb_hoists)*
            #(#st_trend_hoists)*
            #(#st_step_hoists)*

            // Rolling hoists
            #(#roll_hoists)*

            // Lag hoists
            #(#lag_hoists)*

            // Const hoists (after body so they're in reverse order of emission)
            #(#const_hoists)*

            #steady
            vec![#(#row_refs),*]
        }
    }
}
