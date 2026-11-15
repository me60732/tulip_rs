//! Router FORM-policy gates (`cse::router::policy`).
//!
//! Three claims, each pinned separately:
//!
//! 1. the runtime packable-kind list is the SAME list the emitter uses — read
//!    out of the macro crate's source, because a proc macro cannot import the
//!    runtime crate and a hand-copied second list is exactly the "two
//!    implementations that drift" failure the rest of this engine keeps
//!    removing;
//! 2. the shared-pair request routes SCALAR and the distinct-period request
//!    routes PACKED (the user's two examples, 2026-11-15), which only the
//!    requested-vs-arena chain count can tell apart;
//! 3. a kernel is preferred over the hand, and the packed arm returns rows
//!    bit-identical to the scalar arm — otherwise routing would be a value
//!    change wearing a perf decision.
mod tests {
    use tulip_rs::cse::kernel::registry::{try_run, try_run_packed};
    use tulip_rs::cse::signature::{K_EMA, K_IIR, K_WILDERS};
    use tulip_rs::cse::{
        erase_request, form_for, packable_group, packable_requested, packable_slots,
        route_registered, Form, Planable, Request, Served,
    };
    use tulip_rs::indicators::apo::Apo;
    use tulip_rs::indicators::atr::Atr;
    use tulip_rs::indicators::cmo::Cmo;
    use tulip_rs::indicators::ema::Ema;
    use tulip_rs::indicators::macd::Macd;
    use tulip_rs::indicators::max::Max;
    use tulip_rs::indicators::ppo::Ppo;
    use tulip_rs::indicators::rsi::Rsi;
    use tulip_rs::indicators::sma::Sma;
    use tulip_rs::indicators::vhf::Vhf;

    fn series(n: usize, f: impl Fn(usize) -> f64) -> Vec<f64> {
        (0..n).map(f).collect()
    }
    fn wiggle() -> Vec<f64> {
        series(700, |i| {
            100.0 + (i as f64 * 0.013).sin() * 3.0 + (i as f64 * 0.11).cos() * 0.4
        })
    }
    /// Streams for a price-only arena: one column.
    fn price(s: &[f64]) -> [&[f64]; 1] {
        let b: &[f64] = s;
        [b]
    }

    /// Claim 1: `PACKABLE_KINDS` == `codegen::shape::chain_step_form`.
    #[test]
    fn packable_kinds_match_the_codegen_predicate() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tulip_rs_macros/src/cse/codegen/shape.rs"
        );
        let src = std::fs::read_to_string(path).expect("codegen shape.rs readable");
        let start = src
            .find("fn chain_step_form(kind: Kind) -> bool")
            .expect("predicate present");
        let body = &src[start..src.len().min(start + 400)];
        let open = body.find('{').expect("body opens");
        let body = &body[open..body.rfind('}').unwrap()];
        let mut named: Vec<&str> = Vec::new();
        for k in [
            "Ema",
            "Wilders",
            "Iir",
            "IirClamped",
            "Accum",
            "Sma",
            "RollSum",
        ] {
            if body.contains(&format!("Kind::{k}")) {
                named.push(k);
            }
        }
        let mut runtime: Vec<&str> = Vec::new();
        for tag in [K_EMA, K_WILDERS, K_IIR] {
            if tulip_rs::cse::PACKABLE_KINDS.contains(&tag) {
                runtime.push(match tag {
                    K_EMA => "Ema",
                    K_WILDERS => "Wilders",
                    _ => "Iir",
                });
            }
        }
        named.sort();
        runtime.sort();
        assert_eq!(
            named, runtime,
            "the runtime kind list drifted from chain_step_form: emitter={named:?} runtime={runtime:?}"
        );
        assert!(
            !runtime.contains(&"Sma"),
            "Sma is bucketed by the ring machinery, not by chain_step — see classify_packed"
        );
    }

    /// Claim 2a: `Apo(6,12) + Ppo(6,12) + Macd(6,12,9)` — the shared pair.
    /// Seven EMA chains were ASKED for; interning leaves three. That collapse is
    /// the engine's win and it belongs to the scalar kernel.
    #[test]
    fn shared_pair_routes_to_the_scalar_form() {
        let req = Request::new()
            .add(Apo::recipe(&[6.0, 12.0]), None)
            .add(Ppo::recipe(&[6.0, 12.0]), None)
            .add(Macd::recipe(&[6.0, 12.0, 9.0]), None);
        let (_arena, sig, _v, _o) = erase_request(&req).unwrap();
        let requested = packable_requested(&req);
        let slots = packable_slots(&sig);
        assert_eq!(
            requested, 7,
            "2 (apo) + 2 (ppo) + 3 (macd) EMA chains asked for"
        );
        assert_eq!(
            slots, 3,
            "interning must collapse the shared pair to 3 chains"
        );
        assert_eq!(
            packable_group(&sig),
            2,
            "group size ALONE would call this a packing candidate — it is the reason \
             the requested-vs-arena comparison exists"
        );
        assert_eq!(form_for(&sig, &req), Form::Scalar);
    }

    /// Claim 2b: the same three indicators at DISTINCT periods. Nothing dedups,
    /// six chains read one close column — that redundancy is per-lane, so the
    /// arena wants the packed kernel.
    #[test]
    fn distinct_period_chains_route_to_the_packed_form() {
        let req = Request::new()
            .add(Apo::recipe(&[6.0, 12.0]), None)
            .add(Ppo::recipe(&[8.0, 14.0]), None)
            .add(Macd::recipe(&[9.0, 18.0, 7.0]), None);
        let (_arena, sig, _v, _o) = erase_request(&req).unwrap();
        assert_eq!(
            packable_requested(&req),
            packable_slots(&sig),
            "nothing may collapse"
        );
        assert!(
            packable_group(&sig) >= 4,
            "four chains over the close column"
        );
        assert_eq!(form_for(&sig, &req), Form::Packed);
    }

    /// `atr_multi`'s shape: three ATRs at three periods. The `tr` subgraph
    /// interns, the three Wilders chains do not — so it is a packing candidate
    /// with dedup happening ONLY on the non-packable half, which is exactly the
    /// case the comparison is scoped to.
    #[test]
    fn three_atrs_over_one_tr_are_a_packing_candidate() {
        let req = Request::new()
            .add(Atr::recipe(&[9.0]), None)
            .add(Atr::recipe(&[14.0]), None)
            .add(Atr::recipe(&[21.0]), None);
        let (arena, sig, _v, _o) = erase_request(&req).unwrap();
        assert!(
            arena.nodes.len() < 3 * 12,
            "the tr walk should intern ({} nodes for three separate arenas would be ~36)",
            arena.nodes.len()
        );
        assert_eq!(packable_requested(&req), packable_slots(&sig));
        assert_eq!(packable_group(&sig), 3);
        assert_eq!(form_for(&sig, &req), Form::Packed);
    }

    /// Claim 3: a kernel beats the hand, and the packed arm must not change a
    /// value. Both cases are written in their row's DECLARED order — the same
    /// arena requested as `Apo, Ppo, Macd` instead of `Macd, Apo, Ppo` matches no
    /// arm at all, because request order is folded into the key (`bytes()` walks
    /// `sink_src[..n_sinks]`). That is E4's open blocker, not a routing defect,
    /// and it is why this gate lists registered shapes instead of synthesising
    /// them.
    #[test]
    fn packed_and_scalar_arms_return_the_same_bits() {
        let s = wiggle();
        let streams = price(&s);
        let cases: Vec<(&str, Request)> = vec![
            ("ema_chains", {
                let mut r = Request::new();
                for p in [12.0, 24.0, 10.0, 20.0, 6.0, 14.0] {
                    r = r.add(Ema::recipe(&[p]), None);
                }
                r
            }),
            ("ema_six", {
                Request::new()
                    .add(Macd::recipe(&[12.0, 24.0, 9.0]), None)
                    .add(Apo::recipe(&[10.0, 20.0]), None)
                    .add(Ppo::recipe(&[6.0, 14.0]), None)
            }),
        ];
        for (tag, req) in cases {
            let (_a, sig, vals, _o) = erase_request(&req).unwrap();
            let scalar = try_run(&sig, &vals, &streams)
                .unwrap_or_else(|| panic!("{tag}: scalar arm missing"));
            let packed = try_run_packed(&sig, &vals, &streams).unwrap_or_else(|| {
                panic!("{tag}: no packed kernel declared — this case needs a twin")
            });
            assert_eq!(scalar.len(), packed.len(), "{tag}: sink count");
            for (i, (g, w)) in scalar.iter().zip(packed.iter()).enumerate() {
                let (g, w) = (g.as_slice(), w.as_slice());
                assert_eq!(g.len(), w.len(), "{tag}: row {i} length");
                for (j, (x, y)) in g.iter().zip(w.iter()).enumerate() {
                    assert_eq!(
                        x.to_bits(),
                        y.to_bits(),
                        "{tag}: row {i} bit mismatch at {j}"
                    );
                }
            }
        }
    }

    /// The packed twin of `apo_ppo_shared_sl` is commented out in `kernel!`
    /// (the packed form of a shared-pair arena has nothing to vectorise beyond
    /// the scalar kernel), so a packed probe must miss and the route must still
    /// land on the SCALAR kernel rather than fall to the hand.
    #[test]
    fn a_missing_packed_twin_falls_through_to_the_scalar_kernel() {
        let s = wiggle();
        let req = Request::new()
            .add(Apo::recipe(&[12.0, 26.0]), None)
            .add(Ppo::recipe(&[12.0, 26.0]), None);
        let (_a, sig, vals, _o) = erase_request(&req).unwrap();
        assert!(
            try_run_packed(&sig, &vals, &price(&s)).is_none(),
            "apo_ppo_shared_sl must have no packed arm while its twin stays commented"
        );
        match route_registered(&req, &price(&s)) {
            Served::Kernel { form, .. } => assert_eq!(form, Form::Scalar),
            Served::HandOptions | Served::HandScalar => {
                panic!("expected a kernel, got a hand route")
            }
        }
    }

    /// A packing candidate with NO kernel at all reports the hand route that
    /// matches its shape: the hand answers six different EMA periods with its
    /// SIMD-over-option-sets driver, not with six scalar passes.
    #[test]
    fn unregistered_packing_candidate_reports_the_options_hand_route() {
        let s = wiggle();
        let req = Request::new()
            .add(Ema::recipe(&[9.0]), None)
            .add(Ema::recipe(&[21.0]), None)
            .add(Sma::recipe(&[14.0]), None);
        let (_a, sig, _v, _o) = erase_request(&req).unwrap();
        assert_eq!(form_for(&sig, &req), Form::Packed);
        assert!(matches!(
            route_registered(&req, &price(&s)),
            Served::HandOptions
        ));
    }

    /// And a shape that is not a packing candidate reports the plain scalar
    /// hand route — the two fallback arms must not be confused.
    #[test]
    fn unregistered_shared_shape_reports_the_scalar_hand_route() {
        let s = wiggle();
        let req = Request::new()
            .add(Max::recipe(&[9.0]), None)
            .add(Vhf::recipe(&[9.0]), None);
        let (_a, sig, _v, _o) = erase_request(&req).unwrap();
        assert_eq!(form_for(&sig, &req), Form::Scalar);
        assert!(matches!(
            route_registered(&req, &price(&s)),
            Served::HandScalar
        ));
        // Rsi + Cmo at DIFFERENT periods is registered as a shape (`rsi_cmo_shared_p`
        // erases structurally, so the periods live only in `Values`); used here to
        // show the route does not depend on the numbers matching, only the arena.
        let r2 = Request::new()
            .add(Rsi::recipe(&[14.0]), None)
            .add(Cmo::recipe(&[5.0]), None);
        match route_registered(&r2, &price(&s)) {
            Served::Kernel { .. } => {}
            _ => panic!("rsi_cmo_shared_p serves any (rsi, cmo) period pair"),
        }
    }

    /// `Served::rows()` is the accessor benches and gates reach for; the hand
    /// routes must be empty rather than panicking, so a caller cannot mistake an
    /// un-served request for a zero-row result.
    #[test]
    fn served_rows_are_empty_only_for_hand_routes() {
        let s = series(680, |i| 100.0 + i as f64 * 0.05);
        let req = Request::new()
            .add(Ema::recipe(&[9.0]), None)
            .add(Ema::recipe(&[21.0]), None)
            .add(Sma::recipe(&[14.0]), None);
        assert!(route_registered(&req, &price(&s)).rows().is_empty());
        let req2 = Request::new()
            .add(Apo::recipe(&[12.0, 26.0]), None)
            .add(Ppo::recipe(&[12.0, 26.0]), None);
        assert!(!route_registered(&req2, &price(&s)).rows().is_empty());
    }
}
