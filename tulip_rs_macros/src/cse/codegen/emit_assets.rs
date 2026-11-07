//! Assets emitter: builds the main kernel function body for asset-lane CSE patterns.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::*;

// Additional imports from shared module needed for emit_assets
use crate::cse::codegen::shared::{
    assets_row_ref_token, assets_warm_ptr_token, emitter_context, sink_window_symbols,
    EmitterContext,
};

/// Build hoists for the assets (lane) form.
///
/// Returns: (stream_arr_hoists, carry_hoists, const_hoists, ring_replay).
fn build_assets_hoists(
    p: &Pattern,
    idents: &[String],
) -> (
    Vec<TokenStream>, // stream_arr_hoists
    Vec<TokenStream>, // carry_hoists
    Vec<TokenStream>, // const_hoists
    Vec<TokenStream>, // ring_replay
) {
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
                let ck = format_ident!("{}_carry", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                let ik = format_ident!("{}_inv", ident_k);
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
                let ki = k;
                let c1 = format_ident!("{}_it_prev", ident_k);
                let c2 = format_ident!("{}_it_prev2", ident_k);
                carry_hoists.push(quote! {
                    let mut #c1 = Simd::<f64, L>::from_array(std::array::from_fn(|a| kvaccw[a][#ki][0]));
                    let mut #c2 = Simd::<f64, L>::from_array(std::array::from_fn(|a| kvaccw[a][#ki][1]));
                });
            }
            Kind::Mama => {}
            Kind::IirClampedPlain => {
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
                let ck = format_ident!("{}_carry", ident_k);
                carry_hoists.push(quote! {
                    let mut #ck = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                });
            }
            Kind::Accum => {
                let ck = format_ident!("{}_carry", ident_k);
                carry_hoists.push(quote! {
                    let mut #ck = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                });
            }
            Kind::VariableAlpha => {
                let ck = format_ident!("{}_carry", ident_k);
                carry_hoists.push(quote! {
                    let mut #ck = Simd::<f64, L>::from_array(std::array::from_fn(|a| stw[a][#ki]));
                });
            }
            Kind::RollSum => {
                let ck = format_ident!("{}_carry", ident_k);
                let mk = format_ident!("{}_mult", ident_k);
                let pk = format_ident!("{}_p", ident_k);
                let gk = format_ident!("{}_grp", ident_k);
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

    // The batched lane ring engine
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
            ring_replay.push(quote! {
                let mut ringb = ringb.into_warm();
            });
        }
        if groups.iter().any(|&g| p.plan.is_lockstep(g)) {
            decls.push(quote! {
                let mut rings = crate::cse::kernel::warmup::SimdRingSet::<L>::new();
            });
            ring_replay.push(quote! {
                let mut rings = rings.into_warm();
            });
        }
        ring_replay.splice(0..0, decls);
    }

    (stream_arr_hoists, carry_hoists, const_hoists, ring_replay)
}

/// Build the steady body statements for the assets (lane) form.
fn build_assets_body(p: &Pattern, idents: &[String]) -> Vec<TokenStream> {
    let shape = Shape { lanes: true };
    // Define stream_slot helper - returns the source slot for a given node
    let stream_slot = |slot_src: usize| -> usize { p.nodes[slot_src].a };
    let mut body: Vec<TokenStream> = Vec::new();
    for (k, node) in p.nodes.iter().enumerate() {
        let xk = format_ident!("{}", idents[k]);
        let ea = ev(node.a, &idents);
        let eb = ev(node.b, &idents);
        match node.kind {
            Kind::Ln => {
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
                let ck = format_ident!("{}_carry", idents[k]);
                body.push(quote! {
                    let #xk = { let o = #ck; #ck = #ea; o };
                });
            }
            Kind::Accum => {
                let ck = format_ident!("{}_carry", idents[k]);
                body.push(quote! {
                    let #xk = { #ck = #ea.mul_add(#eb, #ck); #ck };
                });
            }
            Kind::Cybercycle => {
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
                let ki = k;
                let mama_slot = node.a as usize;
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
                let ck = format_ident!("{}_carry", idents[k]);
                let mk = format_ident!("{}_mult", idents[k]);
                let gk = format_ident!("{}_grp", idents[k]);
                if p.plan.is_lockstep(k) {
                    let ld = p.plan.leader[k] as usize;
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
                body.push(quote! { let #xk = #ea; });
            }
            Kind::Fma => {
                let ec = ev(node.c.expect("Fma: c operand"), &idents);
                body.push(quote! { let #xk = #ea.mul_add(#eb, #ec); });
            }
            Kind::Lag => {
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
            Kind::AdaptiveMsw => {
                let ki = k;
                let ss = stream_slot(node.a);
                let dc = ev(node.b, &idents);
                body.push(quote! {
                    let #xk = {
                        let dc_arr = #dc.to_array();
                        let out: [f64; L] = std::array::from_fn(|a| {
                            let t_abs = gate + i;
                            let period = ((dc_arr[a] + 0.5) as usize).clamp(6, (t_abs + 1).min(50));
                            let start = t_abs + 1 - period;
                            let window = &streams[a][#ss][start..=t_abs];
                            let (cos_tw, sin_tw) = crate::indicators::msw::twiddles_for_period(period);
                            let (rp, ip) = crate::indicators::msw::dot_product_simd::<8>(window, cos_tw, sin_tw);
                            let (sine, lead) = crate::indicators::msw::phase_from_rp_ip(rp, ip);
                            kvaccw[a][#ki][2] = lead;
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

    body
}

/// Build store declarations and scatter operations for the assets (lane) form.
fn build_assets_stores(
    p: &Pattern,
    idents: &[String],
    ctx: &EmitterContext,
) -> (
    Vec<TokenStream>, // row_arrays
    Vec<TokenStream>, // always_stores
    Vec<TokenStream>, // opt_stores
) {
    let mut row_arrays: Vec<TokenStream> = Vec::new();
    let mut always_stores: Vec<TokenStream> = Vec::new();
    let mut opt_stores: Vec<TokenStream> = Vec::new();

    for (gidx, members) in ctx.group_members.iter().enumerate() {
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

        if ctx.group_is_optional[gidx] {
            let want_g = format_ident!("want{}", gidx);
            opt_stores.push(quote! { if #want_g { #scatter } });
        } else {
            always_stores.push(scatter);
        }
    }

    (row_arrays, always_stores, opt_stores)
}

/// Build return-related code for the assets (lane) form.
fn build_assets_returns(
    p: &Pattern,
    idents: &[String],
    ctx: &EmitterContext,
) -> (
    Vec<TokenStream>, // row_refs
    Vec<TokenStream>, // warm_ptrs
) {
    let mut row_refs: Vec<TokenStream> = Vec::new();
    for sidx in 0..p.sinks.len() {
        let sym = sink_window_symbols(p, &idents, &ctx.group_members, &ctx.group_of, sidx);
        row_refs.push(assets_row_ref_token(&sym, sidx));
    }

    let warm_ptrs: Vec<TokenStream> = (0..p.sinks.len())
        .map(|sidx| {
            let sym = sink_window_symbols(p, &idents, &ctx.group_members, &ctx.group_of, sidx);
            assets_warm_ptr_token(&sym, sidx)
        })
        .collect();

    (row_refs, warm_ptrs)
}

pub(super) fn run_assets_fn(p: &Pattern) -> TokenStream {
    let name = format_ident!("{}_assets", p.name);
    let sig = format_ident!("SIG_{}", p.name);
    let idents = build_idents(p);

    // Build the shared emitter context (sink/window setup)
    let ctx = emitter_context(p, &idents);

    // Stage 1: Build hoists
    let (stream_arr_hoists, carry_hoists, const_hoists, ring_replay) =
        build_assets_hoists(p, &idents);

    // Stage 2: Build body
    let body = build_assets_body(p, &idents);

    // Stage 3a: Build stores
    let (row_arrays, always_stores, opt_stores) = build_assets_stores(p, &idents, &ctx);

    // Stage 3b: Build returns and warm pointers
    let (row_refs, warm_ptrs) = build_assets_returns(p, &idents, &ctx);

    // Extract ctx fields for use in quote blocks
    let total_expr = &ctx.total_expr;
    let lead_hoists = &ctx.lead_hoists;
    let group_minl = &ctx.group_minl;
    let group_want = &ctx.group_want;
    let group_rsz = &ctx.group_rsz;
    let off_decl = &ctx.off_decl;

    // Build steady loop
    let steady =
        steady_with_optional_groups(&ctx.opt_group_ids, &body, &always_stores, &opt_stores);

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
            // Warm [0, gate): the SAME scalar interpreter, once per asset
            let mut stw: Vec<[f64; crate::cse::MAX]> = vec![[0.0f64; crate::cse::MAX]; L];
            let mut stbw: Vec<[f64; crate::cse::MAX]> = vec![[0.0f64; crate::cse::MAX]; L];
            let mut stcubw: Vec<[f64; crate::cse::MAX]> = vec![[0.0f64; crate::cse::MAX]; L];
            let mut stlbw: Vec<[f64; crate::cse::MAX]> = vec![[0.0f64; crate::cse::MAX]; L];
            let mut sumsqw: Vec<[f64; crate::cse::MAX]> = vec![[0.0f64; crate::cse::MAX]; L];
            let mut wsumw: Vec<[f64; crate::cse::MAX]> = vec![[0.0f64; crate::cse::MAX]; L];
            let mut trailw: Vec<[usize; crate::cse::MAX]> = vec![[0usize; crate::cse::MAX]; L];
            // The ring/HMA carry states are PERSISTENT per asset
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
