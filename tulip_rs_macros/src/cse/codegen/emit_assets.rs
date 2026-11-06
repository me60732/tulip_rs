use super::*;

use shared::{assets_row_ref_token, assets_warm_ptr_token, sink_window_symbols};

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
pub(super) fn run_assets_fn(p: &Pattern) -> TokenStream {
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
    let lead_hoists = sink_lead_hoists(p, &idents);
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
    let used_slots = used_input_slots(p);
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
            Kind::Ema | Kind::Wilders | Kind::SeededEma => {
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
            Kind::Cybercycle => {
                // Lane Cybercycle carry + coefficients.
                let c1 = format_ident!("{}_cycle_prev", ident_k);
                let c2 = format_ident!("{}_cycle_prev2", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                let ik = format_ident!("{}_inv", ident_k);
                carry_hoists.push(quote! {
                    let mut #c1 = Simd::<f64, L>::from_array(std::array::from_fn(|a| kvaccw[a][#ki][0]));
                    let mut #c2 = Simd::<f64, L>::from_array(std::array::from_fn(|a| kvaccw[a][#ki][1]));
                    let #mk = Simd::<f64, L>::splat(vals.mults[#ki]);
                    let #ik = Simd::<f64, L>::splat(vals.invms[#ki]);
                });
            }
            Kind::PeakDecay => {
                let ck = format_ident!("{}_carry", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                carry_hoists.push(quote! {
                    let mut #ck = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                    let #mk = Simd::<f64, L>::splat(vals.mults[#ki]);
                });
            }
            Kind::InstantaneousTrendline => {
                // Lane InstantaneousTrendline carry in kvaccw (3-field: [it_prev, it_prev2, 0.0]).
                let ki = k;
                let c1 = format_ident!("{}_it_prev", ident_k);
                let c2 = format_ident!("{}_it_prev2", ident_k);
                carry_hoists.push(quote! {
                    let mut #c1 = Simd::<f64, L>::from_array(std::array::from_fn(|a| kvaccw[a][#ki][0]));
                    let mut #c2 = Simd::<f64, L>::from_array(std::array::from_fn(|a| kvaccw[a][#ki][1]));
                });
            }
            Kind::Mama => {
                // Lane MAMA carry in kvaccw ([mama_prev, fama_prev, prev_phase]).
                // Alpha is written to stw[a][k] each bar for FAMA to consume.
            }
            Kind::IirClampedPlain => {
                // Plain IIR clamped: lane form with plain mul+add (NO mul_add fusion).
                let ck = format_ident!("{}_carry", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                let ik = format_ident!("{}_inv", ident_k);
                let (ht, hv, lt, lv) = (
                    format_ident!("{}_hi_t", ident_k),
                    format_ident!("{}_hi_v", ident_k),
                    format_ident!("{}_lo_t", ident_k),
                    format_ident!("{}_lo_v", ident_k),
                );
                let (b, c, d, e) = (node.b, node.c.unwrap_or(0), node.d, node.e);
                carry_hoists.push(quote! {
                    let mut #ck = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                    let #mk = Simd::<f64, L>::splat(vals.mults[#ki]);
                    let #ik = Simd::<f64, L>::splat(vals.invms[#ki]);
                    let #ht = Simd::<f64, L>::splat(vals.consts[#b]);
                    let #hv = Simd::<f64, L>::splat(vals.consts[#c]);
                    let #lt = Simd::<f64, L>::splat(vals.consts[#d]);
                    let #lv = Simd::<f64, L>::splat(vals.consts[#e]);
                });
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
            Kind::VariableAlpha => {
                // Lane carry only — same standing as Accum: the blend
                // weight is a live per-bar NODE value (node.c), not a
                // `mults`/`invms` constant, so there is nothing to splat.
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
            Kind::Cybercycle => {
                // Lane Cybercycle steady recurrence.
                let stream = p.nodes[node.a].a;
                let sfa = format_ident!("sp{}", stream);
                let c1 = format_ident!("{}_cycle_prev", idents[k]);
                let c2 = format_ident!("{}_cycle_prev2", idents[k]);
                let mk = format_ident!("{}_mult", idents[k]);
                let ik = format_ident!("{}_inv", idents[k]);
                body.push(quote! {
                    let #xk = {
                        let half = Simd::<f64, L>::splat(0.5);
                        let d2 = (#ik * half) * (#ik * half);

                        let price = Simd::<f64, L>::from_array(std::array::from_fn(
                            |a| unsafe { *#sfa[a].add(i) },
                        ));
                        let price_1 = Simd::<f64, L>::from_array(std::array::from_fn(
                            |a| unsafe { *#sfa[a].add(i).sub(1) },
                        ));
                        let p2 = Simd::<f64, L>::from_array(std::array::from_fn(
                            |a| unsafe { *#sfa[a].add(i).sub(2) },
                        ));
                        let p3 = Simd::<f64, L>::from_array(std::array::from_fn(
                            |a| unsafe { *#sfa[a].add(i).sub(3) },
                        ));
                        let p4 = Simd::<f64, L>::from_array(std::array::from_fn(
                            |a| unsafe { *#sfa[a].add(i).sub(4) },
                        ));
                        let p5 = Simd::<f64, L>::from_array(std::array::from_fn(
                            |a| unsafe { *#sfa[a].add(i).sub(5) },
                        ));

                        let smooth =
                            ((price_1 * Simd::<f64, L>::splat(2.0) + price)
                                + (p2 * Simd::<f64, L>::splat(2.0) + p3))
                                * Simd::<f64, L>::splat(1.0 / 6.0);
                        let smooth_1 =
                            ((p2 * Simd::<f64, L>::splat(2.0) + price_1)
                                + (p3 * Simd::<f64, L>::splat(2.0) + p4))
                                * Simd::<f64, L>::splat(1.0 / 6.0);
                        let smooth_2 =
                            ((p3 * Simd::<f64, L>::splat(2.0) + p2)
                                + (p4 * Simd::<f64, L>::splat(2.0) + p5))
                                * Simd::<f64, L>::splat(1.0 / 6.0);

                        let smooth_diff = smooth - smooth_1 * Simd::<f64, L>::splat(2.0) + smooth_2;
                        let cycle = #mk.mul_add(smooth_diff, #ik.mul_add(#c1, -(d2 * #c2)));
                        #c2 = #c1;
                        #c1 = cycle;
                        cycle
                    };
                });
            }
            Kind::PeakDecay => {
                let ck = format_ident!("{}_carry", idents[k]);
                let mk = format_ident!("{}_mult", idents[k]);
                body.push(quote! {
                    let #xk = {
                        #ck = (#ck * #mk).simd_max(#ea);
                        #ck
                    };
                });
            }
            Kind::InstantaneousTrendline => {
                // Lane InstantaneousTrendline recurrence, scalar-per-lane for exactness.
                let ki = k;
                body.push(quote! {
                    let #xk = {
                        let src_arr = #ea.to_array();
                        let out_arr: [f64; L] = std::array::from_fn(|a| {
                            crate::indicators::instantaneoustrendline::it_step(
                                &mut hdaccw[a][#ki],
                                &mut kvaccw[a][#ki],
                                src_arr[a],
                            )
                        });
                        Simd::<f64, L>::from_array(out_arr)
                    };
                });
            }
            Kind::Mama => {
                // Lane MAMA/FAMA adaptive EMA recurrence.
                // Uses embedded HD pipeline per lane and writes alpha to stw[a][k].
                let ki = k;
                body.push(quote! {
                    let #xk = {
                        let src_arr = #ea.to_array();
                        let out_arr: [f64; L] = std::array::from_fn(|a| {
                            let (mama, _fama, alpha) = crate::indicators::mama::mama_step_with_alpha(
                                &mut hdaccw[a][#ki],
                                &mut kvaccw[a][#ki],
                                vals.consts[#ki],
                                vals.st_step[#ki],
                                src_arr[a],
                            );
                            stw[a][#ki] = alpha;
                            mama
                        });
                        Simd::<f64, L>::from_array(out_arr)
                    };
                });
            }
            Kind::Fama => {
                // Lane FAMA secondary output - reads Mama output from src and alpha from stw[src].
                let ki = k;
                let mama_slot = node.a as usize; // Mama's slot index
                body.push(quote! {
                    let #xk = {
                        let mama = #ea;
                        let alpha = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#mama_slot]));
                        let fama_prev = Simd::<f64, L>::from_array(std::array::from_fn(|a| kvaccw[a][#ki][0]));
                        let half_alpha = alpha * Simd::<f64, L>::splat(0.5);
                        let fama = half_alpha.mul_add(mama - fama_prev, fama_prev);
                        let fama_arr = fama.to_array();
                        for a in 0..L {
                            kvaccw[a][#ki][0] = fama_arr[a];
                        }
                        fama
                    };
                });
            }
            Kind::IirClampedPlain => {
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
                        let nv = #mk * #ea + #ik * #ck;
                        let v = nv
                            .simd_gt(#ht)
                            .select(#hv, nv.simd_lt(#lt).select(#lv, nv));
                        #ck = v;
                        v
                    };
                });
            }
            Kind::VariableAlpha => {
                // Lane fold: src is `#eb` (node.b), live weight is node.c
                // (not pre-bound above, read fresh here same as the scalar
                // arm). No seed-bar branch for validity (gate is always
                // strictly above feed). The Vidya shape's one-bar spelling
                // asymmetry (see the scalar arm's comment) applies per-lane
                // too, using the splatted first-fold-bar mask.
                let ck = format_ident!("{}_carry", idents[k]);
                let ec = ev(node.c.expect("VariableAlpha: mult operand"), &idents);
                let ki = k;
                let step = if node.d == 0 {
                    quote! { #ck = #ck.mul_add(Simd::<f64, L>::splat(1.0) - #ec, #eb * #ec); #ck }
                } else {
                    quote! {
                        #ck = if gate + i == vals.feeds[#ki] as usize + 1 {
                            (#eb - #ck) * #ec + #ck
                        } else {
                            (#eb - #ck).mul_add(#ec, #ck)
                        };
                        #ck
                    }
                };
                body.push(quote! {
                    let #xk = { #step };
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
            Kind::SeededEma => {
                // Same fma as `Kind::Ema`, but the fold value is `node.b`
                // (the FOLD source, e.g. `tr`) -- `node.a` (the SEED) is
                // only read at the warm-up seed bar, already baked into the
                // carry this lane's warm left in `stw`.
                let ck = format_ident!("{}_carry", idents[k]);
                let mk = format_ident!("{}_mult", idents[k]);
                let ik = format_ident!("{}_inv", idents[k]);
                let step = chain_step(&ck, &ik, &mk, &eb);
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
            Kind::Sqrt => {
                let src = ev(node.a, &idents);
                body.push(quote! { let #xk = #src.sqrt(); });
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
            Kind::HighPass => {
                let ki = k;
                body.push(quote! {
                    let #xk = {
                        let real_arr = #ea.to_array();
                        let out: [f64; L] = std::array::from_fn(|a| {
                            let real = real_arr[a];
                            let [y1, prev_real] = hp_accw[a][#ki];
                            let y = vals.mults[#ki].mul_add(y1, vals.invms[#ki] * (real - prev_real));
                            hp_accw[a][#ki] = [y, real];
                            y
                        });
                        Simd::<f64, L>::from_array(out)
                    };
                });
            }
            Kind::SuperSmoother => {
                let ki = k;
                body.push(quote! {
                    let #xk = {
                        let real_arr = #ea.to_array();
                        let out: [f64; L] = std::array::from_fn(|a| {
                            let real = real_arr[a];
                            let [y1, y2, prev_real] = ss_accw[a][#ki];
                            let y = vals.st_step[#ki].mul_add(
                                real + prev_real,
                                vals.mults[#ki].mul_add(y1, vals.invms[#ki] * y2),
                            );
                            ss_accw[a][#ki] = [y, y1, real];
                            y
                        });
                        Simd::<f64, L>::from_array(out)
                    };
                });
            }
            Kind::RollAge => {
                let ki = k;
                body.push(quote! {
                    let #xk = {
                        let x_arr = #ea.to_array();
                        let g = vals.ring_of[#ki] as usize;
                        let cap = vals.ring_cap[g] as usize;
                        let out: [f64; L] = std::array::from_fn(|a| {
                            ringw[a].feed(g, cap, gate + i, x_arr[a]);
                            ringw[a].evicted(g).unwrap_or(0.0)
                        });
                        Simd::<f64, L>::from_array(out)
                    };
                });
            }
            Kind::HtInPhase => {
                let ki = k;
                body.push(quote! {
                    let #xk = {
                        let x_arr = #ea.to_array();
                        let g = vals.ring_of[#ki] as usize;
                        let cap = vals.ring_cap[g] as usize;
                        let out: [f64; L] = std::array::from_fn(|a| {
                            ringw[a].feed(g, cap, gate + i, x_arr[a]);
                            ringw[a].at(g, 3)
                        });
                        Simd::<f64, L>::from_array(out)
                    };
                });
            }
            Kind::HtQuadrature => {
                let ki = k;
                body.push(quote! {
                    let #xk = {
                        let x_arr = #ea.to_array();
                        let g = vals.ring_of[#ki] as usize;
                        let cap = vals.ring_cap[g] as usize;
                        let out: [f64; L] = std::array::from_fn(|a| {
                            ringw[a].feed(g, cap, gate + i, x_arr[a]);
                            let q1 = 0.0962_f64.mul_add(ringw[a].at(g, 0), 0.5769 * ringw[a].at(g, 2));
                            let q2 = (-0.5769_f64).mul_add(ringw[a].at(g, 4), -0.0962 * ringw[a].at(g, 6));
                            q1 + q2
                        });
                        Simd::<f64, L>::from_array(out)
                    };
                });
            }
            Kind::MeanAbsDev => {
                let ki = k;
                if p.nodes[node.a].kind == Kind::Input {
                    let s = stream_slot(node.a);
                    let sfa = format_ident!("sp{}", s);
                    body.push(quote! {
                        let #xk = {
                            let p = vals.periods[#ki] as usize;
                            let sma_arr = #eb.to_array();
                            let mult = 1.0 / p as f64;
                            let out: [f64; L] = std::array::from_fn(|a| {
                                let base = unsafe {
                                    #sfa[a].offset((i as isize) + 1 - (p as isize))
                                };
                                let window = unsafe { std::slice::from_raw_parts(base, p) };
                                crate::indicators::md::calc_md_simd::<4>(window, sma_arr[a], mult)
                            });
                            Simd::<f64, L>::from_array(out)
                        };
                    });
                } else {
                    body.push(quote! {
                        let #xk = {
                            let p = vals.periods[#ki] as usize;
                            let sma_arr = #eb.to_array();
                            let mult = 1.0 / p as f64;
                            let x_arr = #ea.to_array();
                            let g = vals.ring_of[#ki] as usize;
                            let cap = vals.ring_cap[g] as usize;
                            let out: [f64; L] = std::array::from_fn(|a| {
                                ringw[a].feed(g, cap, gate + i, x_arr[a]);
                                let window = ringw[a].window(g, p);
                                crate::indicators::md::calc_md_simd::<4>(window, sma_arr[a], mult)
                            });
                            Simd::<f64, L>::from_array(out)
                        };
                    });
                }
            }
            Kind::LinregFlow => {
                let ki = k;
                body.push(quote! {
                    let #xk = {
                        let x_arr = #ea.to_array();
                        let sub_arr = #eb.to_array();
                        let m = vals.mults[#ki];
                        let out: [f64; L] = std::array::from_fn(|a| {
                            let out = stw[a][#ki] + x_arr[a] * m;
                            stw[a][#ki] = out - sub_arr[a];
                            out
                        });
                        Simd::<f64, L>::from_array(out)
                    };
                });
            }
            Kind::NviRecur => {
                let ki = k;
                let ec = ev(node.c.expect("NviRecur: volume operand"), &idents);
                let ed = ev(node.d, &idents);
                body.push(quote! {
                    let #xk = {
                        let close_arr = #ea.to_array();
                        let prev_close_arr = #eb.to_array();
                        let volume_arr = #ec.to_array();
                        let prev_volume_arr = #ed.to_array();
                        let out: [f64; L] = std::array::from_fn(|a| {
                            if volume_arr[a] < prev_volume_arr[a] {
                                stw[a][#ki] = close_arr[a] / prev_close_arr[a] * stw[a][#ki];
                            }
                            stw[a][#ki]
                        });
                        Simd::<f64, L>::from_array(out)
                    };
                });
            }
            Kind::PviRecur => {
                let ki = k;
                let ec = ev(node.c.expect("PviRecur: volume operand"), &idents);
                let ed = ev(node.d, &idents);
                body.push(quote! {
                    let #xk = {
                        let close_arr = #ea.to_array();
                        let prev_close_arr = #eb.to_array();
                        let volume_arr = #ec.to_array();
                        let prev_volume_arr = #ed.to_array();
                        let out: [f64; L] = std::array::from_fn(|a| {
                            if volume_arr[a] > prev_volume_arr[a] {
                                stw[a][#ki] = close_arr[a] / prev_close_arr[a] * stw[a][#ki];
                            }
                            stw[a][#ki]
                        });
                        Simd::<f64, L>::from_array(out)
                    };
                });
            }
            Kind::KvoFlow => {
                let ki = k;
                let ec = ev(node.c.expect("KvoFlow: close operand"), &idents);
                let ed = ev(node.d, &idents);
                body.push(quote! {
                    let #xk = {
                        let high_arr = #ea.to_array();
                        let low_arr = #eb.to_array();
                        let close_arr = #ec.to_array();
                        let volume_arr = #ed.to_array();
                        let out: [f64; L] = std::array::from_fn(|a| {
                            let hlc = high_arr[a] + low_arr[a] + close_arr[a];
                            let dm = high_arr[a] - low_arr[a];
                            let [prev_hlc, cm, prev_hl] = kvaccw[a][#ki];
                            let mut sign;
                            let mut cm = cm;
                            if stw[a][#ki] < 0.5 && hlc > prev_hlc {
                                stw[a][#ki] = 1.0;
                                cm = prev_hl;
                                sign = 1.0;
                            } else if stw[a][#ki] >= 0.5 && hlc < prev_hlc {
                                stw[a][#ki] = 0.0;
                                cm = prev_hl;
                                sign = -1.0;
                            } else {
                                sign = if stw[a][#ki] >= 0.5 { 1.0 } else { -1.0 };
                            }
                            cm += dm.max(f64::EPSILON);
                            kvaccw[a][#ki] = [hlc, cm, dm];
                            (dm / cm).mul_add(2.0, -1.0).abs() * volume_arr[a] * 100.0 * sign
                        });
                        Simd::<f64, L>::from_array(out)
                    };
                });
            }
            Kind::Psar => {
                // Per-lane scalar transcription of the scalar K_PSAR arm --
                // same standing as the other (0,0) scalar-only kinds (see
                // `lane_formula`): this is required for `run_assets_fn` to
                // stay exhaustive, not because real by-assets requests are
                // expected to exercise it.
                let ki = k;
                let sh = stream_slot(node.a);
                let sl = stream_slot(node.b);
                let sfah = format_ident!("sp{}", sh);
                let sfal = format_ident!("sp{}", sl);
                body.push(quote! {
                    let #xk = {
                        let afs = vals.mults[#ki];
                        let maxaf = vals.invms[#ki];
                        let out: [f64; L] = std::array::from_fn(|a| {
                            let h = unsafe { *#sfah[a].add(i) };
                            let ph = unsafe { *#sfah[a].add(i).sub(1) };
                            let oh = unsafe { *#sfah[a].add(i).sub(2) };
                            let l = unsafe { *#sfal[a].add(i) };
                            let pl = unsafe { *#sfal[a].add(i).sub(1) };
                            let ol = unsafe { *#sfal[a].add(i).sub(2) };
                            let [mut psar, mut extream, mut accel] = kvaccw[a][#ki];
                            let mut uptrend = stw[a][#ki] != 0.0;
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
                            stw[a][#ki] = if uptrend { 1.0 } else { 0.0 };
                            kvaccw[a][#ki] = [psar, extream, accel];
                            psar
                        });
                        Simd::<f64, L>::from_array(out)
                    };
                });
            }
            Kind::SdftRotate => {
                // Per-lane scalar transcription of the scalar K_SDFTROTATE
                // arm -- same standing as Kind::Psar's run_assets_fn arm:
                // required for exhaustiveness, not because by-assets requests
                // are expected to exercise a (0,0) scalar-only kind.
                let ki = k;
                let ss = stream_slot(node.a);
                let sfas = format_ident!("sp{}", ss);
                body.push(quote! {
                    let #xk = {
                        let wr = vals.mults[#ki];
                        let wi = vals.invms[#ki];
                        let period = vals.periods[#ki] as usize;
                        let out: [f64; L] = std::array::from_fn(|a| {
                            let new = unsafe { *#sfas[a].add(i) };
                            let old = unsafe { *#sfas[a].add(i).sub(period) };
                            let [rp, ip, _lead] = kvaccw[a][#ki];
                            let rp2 = wr.mul_add(rp, -(wi * ip)) + (new - old);
                            let ip2 = wr.mul_add(ip, wi * rp);
                            let (sine, lead) =
                                crate::indicators::msw::phase_from_rp_ip(rp2, ip2);
                            kvaccw[a][#ki] = [rp2, ip2, lead];
                            sine
                        });
                        Simd::<f64, L>::from_array(out)
                    };
                });
            }
            Kind::SdftLead => {
                let owner_ki = node.a;
                body.push(quote! {
                    let #xk = {
                        let out: [f64; L] = std::array::from_fn(|a| kvaccw[a][#owner_ki][2]);
                        Simd::<f64, L>::from_array(out)
                    };
                });
            }
            Kind::HomodyneDiscriminator => {
                // Per-lane scalar transcription -- same standing as
                // Kind::SdftRotate's run_assets_fn arm: required for
                // exhaustiveness, not because by-assets requests are
                // expected to exercise this (0,0) scalar-only kind.
                let ki = k;
                let ss = stream_slot(node.a);
                let sfas = format_ident!("sp{}", ss);
                body.push(quote! {
                    let #xk = {
                        let out: [f64; L] = std::array::from_fn(|a| {
                            let x = unsafe { *#sfas[a].add(i) };
                            crate::indicators::homodynediscriminator::hd_step(
                                &mut hdaccw[a][#ki],
                                x,
                            )
                        });
                        Simd::<f64, L>::from_array(out)
                    };
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
        let sym = sink_window_symbols(p, &idents, group_members, group_of, sidx);
        row_refs.push(assets_row_ref_token(&sym, sidx));
    }
    let warm_ptrs: Vec<TokenStream> = (0..p.sinks.len())
        .map(|sidx| {
            let sym = sink_window_symbols(p, &idents, group_members, group_of, sidx);
            assets_warm_ptr_token(&sym, sidx)
        })
        .collect();

    let steady = steady_with_optional_groups(opt_group_ids, &body, &always_stores, &opt_stores);

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
            // Shared 3-field bespoke carry (kind-specific meaning per node).
            let mut kvaccw: Vec<[[f64; 3]; crate::cse::MAX]> =
                vec![[[0.0f64; 3]; crate::cse::MAX]; L];
            let mut hp_accw: Vec<[[f64; 2]; crate::cse::MAX]> =
                vec![[[0.0f64; 2]; crate::cse::MAX]; L];
            let mut ss_accw: Vec<[[f64; 3]; crate::cse::MAX]> =
                vec![[[0.0f64; 3]; crate::cse::MAX]; L];
            let mut hdaccw: Vec<[[f64; 38]; crate::cse::MAX]> =
                vec![[[0.0f64; 38]; crate::cse::MAX]; L];
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
                        &mut kvaccw[a],
                        &mut hp_accw[a],
                        &mut ss_accw[a],
                        &mut hdaccw[a],
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
