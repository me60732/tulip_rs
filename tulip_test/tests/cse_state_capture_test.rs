//! Phase A of kernel state capture (`proposals/Macro_Cse_Engine/state_capture_checklist.md`).
//!
//! The emitter now generates, per pattern, one body plus two entry points:
//! `{name}(vals, streams)` (rows only — every pre-existing caller) and
//! `{name}_captured(vals, streams, &mut Carries)` (rows + the carry arrays the
//! run ended with). These gates pin exactly that contract:
//!
//! 1. capture must not perturb the numbers — the two entry points return
//!    byte-identical rows, because the epilogue sits OUTSIDE the steady loop;
//! 2. the `Carries` the run leaves behind must be the HAND's own carries, not
//!    merely "some state of the right shape" (the R8 finding was that a
//!    plausible-looking state is still the wrong machine);
//! 3. the `try_run_captured` dispatcher must be the one filling them (the
//!    arms, not just the functions, are reachable).
//!
//! Ring-backed kinds are NOT captured yet (Phase C), so nothing here claims a
//! full-series round-trip for `RollSum`/`RollAge`/`Hma`/`Ht*` — only
//! `st`-slot families (`LinregFlow`, `Ema`) are gated.
mod tests {
    use tulip_rs::cse::kernel::registry::{
        ema_chains, ema_chains_captured, linreg_n, linreg_n_captured, SIG_linreg_n,
    };
    use tulip_rs::cse::signature::{K_EMA, K_LINREGFLOW};
    use tulip_rs::cse::{carries_to_chains, erase_request_tails};
    use tulip_rs::cse::{
        erase_request, run_registered, run_registered_with_states, try_run, try_run_captured,
    };
    use tulip_rs::cse::{Carries, ChainState, Planable, Request, Restorable};
    use tulip_rs::indicator_types::{Indicator, TIndicatorState};
    use tulip_rs::indicators::ema::Ema;
    use tulip_rs::indicators::linreg::Linreg;

    fn series(n: usize, f: impl Fn(usize) -> f64) -> Vec<f64> {
        (0..n).map(f).collect()
    }
    fn wiggle() -> Vec<f64> {
        series(700, |i| {
            100.0 + (i as f64 * 0.013).sin() * 3.0 + (i as f64 * 0.11).cos() * 0.4
        })
    }

    /// Bit-level row comparison with the first divergence named — a whole-vector
    /// `assert_eq!` hides WHERE two runs part, and where is what separates a
    /// lead error from a carry error.
    fn assert_bits(label: &str, got: &[f64], want: &[f64]) {
        let common = got.len().min(want.len());
        assert_eq!(
            got.len(),
            want.len(),
            "{label}: row length {} != {want:?}",
            got.len()
        );
        let mut bad = 0usize;
        let mut first = None;
        for i in 0..common {
            if got[i].to_bits() != want[i].to_bits() {
                bad += 1;
                if first.is_none() {
                    first = Some(i);
                }
            }
        }
        assert_eq!(
            bad,
            0,
            "{label}: {bad} bit mismatches over {common} rows, first at {first:?} \
             (got {:?} want {:?})",
            first.map(|i| (got[i], i)),
            first.map(|i| (want[i], i)),
        );
    }

    fn linreg_req(p: f64) -> Request {
        // The hand is called with BOTH optional flags on — that is the branch
        // the engine reproduces (see `linreg_planable_bit_exact`).
        Request::new().add(Linreg::recipe(&[p]), Some(&[true, true]))
    }

    fn ema_chains_req() -> Request {
        let mut r = Request::new();
        for p in [12.0, 24.0, 10.0, 20.0, 6.0, 14.0] {
            r = r.add(Ema::recipe(&[p]), None);
        }
        r
    }

    /// GATE 1 — capture is an epilogue, never a second arithmetic path.
    #[test]
    fn captured_rows_match_rows_only_wrapper() {
        for p in [2.0, 5.0, 14.0, 27.0] {
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let (_a, sig, vals, _o) = erase_request(&linreg_req(p)).expect("linreg erases");
                assert_eq!(
                    sig, SIG_linreg_n,
                    "linreg p={p}: arena drifted from the row"
                );
                let plain = linreg_n(&vals, &streams);
                let mut car = Carries::default();
                let got = linreg_n_captured(&vals, &streams, &mut car);
                assert_eq!(
                    got.len(),
                    plain.len(),
                    "linreg p={p}: captured row count != plain"
                );
                for (i, (g, w)) in got.iter().zip(plain.iter()).enumerate() {
                    assert_bits(&format!("linreg p={p} row {i} (captured vs plain)"), g, w);
                }
            }
        }

        let s = wiggle();
        let streams = [s.as_slice()];
        let (_a, _sig, vals, _o) = erase_request(&ema_chains_req()).expect("ema_chains erases");
        let plain = ema_chains(&vals, &streams);
        let mut car = Carries::default();
        let got = ema_chains_captured(&vals, &streams, &mut car);
        for (i, (g, w)) in got.iter().zip(plain.iter()).enumerate() {
            assert_bits(&format!("ema_chains row {i} (captured vs plain)"), g, w);
        }
    }

    /// GATE 2 — the dispatcher (`try_run_captured`) reaches the captured arms
    /// AND actually writes the caller's `Carries`. Rows must equal `try_run`'s.
    #[test]
    fn try_run_captured_dispatches_and_fills() {
        let p = 14.0;
        let s = wiggle();
        let streams = [s.as_slice()];
        let req = linreg_req(p);
        let (_a, sig, vals, _o) = erase_request(&req).expect("linreg erases");

        let plain = try_run(&sig, &vals, &streams).expect("linreg_n dispatchable");
        let mut car = Carries::default();
        let got = try_run_captured(&sig, &vals, &streams, &mut car).expect("captured arm missing");

        assert_eq!(got.len(), plain.len(), "dispatcher row count");
        for (i, (g, w)) in got.iter().zip(plain.iter()).enumerate() {
            assert_bits(&format!("try_run_captured row {i}"), g, w);
        }

        // The out-param must be WRITTEN — a green rows comparison above is also
        // satisfied by an epilogue that never ran, so prove the carries moved.
        let flow: Vec<usize> = (0..sig.len as usize)
            .filter(|&k| sig.kinds[k] == K_LINREGFLOW)
            .collect();
        assert_eq!(
            flow.len(),
            2,
            "linreg_n should hold exactly the sum_y/sum_xy moment slots, got {flow:?}"
        );
        let live = flow.iter().filter(|&&k| car.st[k] != 0.0).count();
        assert_eq!(live, flow.len(), "dispatcher left every moment slot zeroed");
    }

    /// GATE 3 — the captured `st` for `linreg_n` IS the hand's carry pair
    /// (the R9 contract, now fed by the engine instead of the hand's own state).
    #[test]
    fn captured_linregflow_matches_hand_carries() {
        for p in [2.0, 5.0, 14.0, 27.0] {
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let pp: [f64; 1] = [p];
                let (_a, sig, vals, _o) = erase_request(&linreg_req(p)).expect("linreg erases");

                let mut car = Carries::default();
                linreg_n_captured(&vals, &streams, &mut car);

                let (_h, hand) = Linreg::indicator(&streams, &pp, Some(&[true, true])).unwrap();
                let want = match hand.carried_state() {
                    ChainState::LinregFlow { sum_y, sum_xy, .. } => [sum_y, sum_xy],
                    other => panic!("linreg p={p}: hand carried {:?}, not LinregFlow", other),
                };
                let flow: Vec<usize> = (0..sig.len as usize)
                    .filter(|&k| sig.kinds[k] == K_LINREGFLOW)
                    .collect();
                assert_eq!(flow.len(), 2, "linreg p={p}: moment slots {flow:?}");

                // Compare as a SET (sorted by bits): which slot holds sum_y vs
                // sum_xy is the DECODER's question (Phase B), not this gate's —
                // here the claim is only that both carries survived the run.
                let mut got: Vec<u64> = flow.iter().map(|&k| car.st[k].to_bits()).collect();
                let mut exp: Vec<u64> = want.iter().map(|v| v.to_bits()).collect();
                got.sort_unstable();
                exp.sort_unstable();
                assert_eq!(
                    got,
                    exp,
                    "linreg p={p}: captured moments {:?} != hand carries {:?}",
                    flow.iter().map(|&k| car.st[k]).collect::<Vec<_>>(),
                    want,
                );
            }
        }
    }

    /// GATE 4 — same claim for the plain chain family: `st[k]` at an `Ema` slot
    /// equals the hand `EmaState::ema` for the period that slot carries.
    #[test]
    fn captured_ema_st_matches_hand_state() {
        let s = wiggle();
        let streams = [s.as_slice()];
        let inp = [s.as_slice()];
        let (_a, sig, vals, _o) = erase_request(&ema_chains_req()).expect("ema_chains erases");

        let mut car = Carries::default();
        let rows = ema_chains_captured(&vals, &streams, &mut car);

        let slots: Vec<usize> = (0..sig.len as usize)
            .filter(|&k| sig.kinds[k] == K_EMA)
            .collect();
        assert_eq!(
            slots.len(),
            6,
            "ema_chains is six distinct chains, got {slots:?}"
        );
        for k in slots {
            let p = vals.periods[k];
            assert_ne!(p, 0, "slot {k} is K_EMA but carries no period");
            let (h, st) = Ema::indicator(&inp, &[p as f64], None).unwrap();
            let last = h[0][h[0].len() - 1];
            assert_eq!(
                car.st[k].to_bits(),
                st.ema.to_bits(),
                "ema p={p} slot {k}: captured st = {}, hand state.ema = {} \
                 (hand row's last value = {})",
                car.st[k],
                st.ema,
                last,
            );
        }
        // rows were already compared against the hand by `ema_chains_bit_exact`;
        // this gate only reads them back to keep the failure message honest.
        assert_eq!(rows.len(), 6, "ema_chains publishes one row per chain");
    }

    /// GATE 5 — the PACKED lane writeback. A group accumulates in one `Simd`
    /// register and the members' carries live in its LANES, so the exit has to
    /// scatter them back per slot or `st[k]` reports the warm seam. Restricted
    /// to price-sourced chains (`a` is an `Input`): the signal EMA rides the
    /// diff line, so its carry is not a `close`-fed hand `Ema` state.
    #[test]
    fn captured_packed_lanes_match_hand_ema() {
        use tulip_rs::cse::kernel::registry::{ema_six_p, ema_six_p_captured};
        use tulip_rs::cse::signature::K_INPUT;
        use tulip_rs::indicators::apo::Apo;
        use tulip_rs::indicators::macd::Macd;
        use tulip_rs::indicators::ppo::Ppo;

        let s = wiggle();
        let streams = [s.as_slice()];
        let inp = [s.as_slice()];
        let req = Request::new()
            .add(Macd::recipe(&[12.0, 24.0, 9.0]), None)
            .add(Apo::recipe(&[10.0, 20.0]), None)
            .add(Ppo::recipe(&[6.0, 14.0]), None);
        let (_a, sig, vals, _o) = erase_request(&req).expect("ema_six erases");

        let mut car = Carries::default();
        let rows = ema_six_p_captured(&vals, &streams, &mut car);
        let plain = ema_six_p(&vals, &streams);
        for (i, (g, w)) in rows.iter().zip(plain.iter()).enumerate() {
            assert_bits(&format!("ema_six_p row {i} (captured vs plain)"), g, w);
        }

        let slots: Vec<usize> = (0..sig.len as usize)
            .filter(|&k| sig.kinds[k] == K_EMA && sig.kinds[sig.a[k] as usize] == K_INPUT)
            .collect();
        assert!(
            !slots.is_empty(),
            "ema_six_p has no price-sourced chain slot — the filter, not the \
             capture, is what broke"
        );
        for k in slots {
            let p = vals.periods[k];
            let (_h, st) = Ema::indicator(&inp, &[p as f64], None).unwrap();
            assert_eq!(
                car.st[k].to_bits(),
                st.ema.to_bits(),
                "ema_six_p slot {k} (lane writeback): captured st = {}, hand \
                 state.ema = {}",
                car.st[k],
                st.ema,
            );
        }
    }

    /// GATE 6 — ONE CLAIM PER SLOT. A grouped `Sma` accumulates in the lane
    /// register, which the group loop scatters back per slot; the roll-hoist loop
    /// *also* declared an `{ident}_carry` local that the group statement never
    /// touches. If both write back, the later write wins and the packed capture
    /// silently reports the warm seam — invisible to every row-asserting gate,
    /// which is exactly how the trap shipped once before (2026-10, and again
    /// here). Pinned against the scalar twin, whose carry local IS the live
    /// accumulator.
    #[test]
    fn captured_packed_sma_matches_scalar_carries() {
        use tulip_rs::cse::kernel::registry::{
            sma_six, sma_six_captured, sma_six_p, sma_six_p_captured,
        };
        use tulip_rs::cse::signature::K_SMA;
        use tulip_rs::indicators::sma::Sma;

        let s = wiggle();
        let streams = [s.as_slice()];
        let mut req = Request::new();
        for p in [3.0, 7.0, 14.0, 20.0, 50.0, 100.0] {
            req = req.add(Sma::recipe(&[p]), None);
        }
        let (_a, sig, vals, _o) = erase_request(&req).expect("sma_six erases");
        let slots: Vec<usize> = (0..sig.len as usize)
            .filter(|&k| sig.kinds[k] == K_SMA)
            .collect();
        assert_eq!(slots.len(), 6, "sma_six is six rolling sums, got {slots:?}");

        // Restate twin row-equality cheaply so the carry comparison below cannot
        // be comparing two different runs of different shapes.
        let mut cs = Carries::default();
        let rs = sma_six_captured(&vals, &streams, &mut cs);
        for (i, (g, w)) in rs.iter().zip(sma_six(&vals, &streams).iter()).enumerate() {
            assert_bits(&format!("sma_six row {i} (captured vs plain)"), g, w);
        }
        let mut cp = Carries::default();
        let rp = sma_six_p_captured(&vals, &streams, &mut cp);
        for (i, (g, w)) in rp.iter().zip(sma_six_p(&vals, &streams).iter()).enumerate() {
            assert_bits(&format!("sma_six_p row {i} (captured vs plain)"), g, w);
        }

        for k in slots {
            assert_eq!(
                cs.st[k].to_bits(),
                cp.st[k].to_bits(),
                "sma p={} slot {k}: scalar captured sum = {}, packed captured sum = {} \
                 — the packed slot is claimed twice and the stale write won",
                vals.periods[k],
                cs.st[k],
                cp.st[k],
            );
        }
    }

    // ── PHASE B: Carries → ChainState ─────────────────────────────────────

    /// GATE 7 — the decode hop, in the request's own `tail_inputs` order. The
    /// two `LinregFlow` nodes are ONE machine, so they must collapse to the
    /// single entry `Linreg::restore_state` matches, and the comparison is
    /// bitwise on all four fields: a shape-only check would pass on a swapped
    /// pairing (which is the failure this table is most likely to have).
    #[test]
    fn decoded_linregflow_equals_hand_carries() {
        const SEAM: usize = 400;
        for p in [2.0f64, 5.0, 14.0, 27.0] {
            let s = wiggle();
            let streams = [&s[..SEAM]];
            let req = linreg_req(p);
            let (sig, vals, _o, tails) = erase_request_tails(&req).expect("linreg erases");
            assert_eq!(tails.len(), 1, "one recipe in the request");
            assert_eq!(
                tails[0].len(),
                2,
                "linreg's tail is the two moment nodes, got {:?}",
                tails[0]
            );

            let mut car = Carries::default();
            linreg_n_captured(&vals, &streams, &mut car);
            let chains = carries_to_chains(&sig, &vals, &car, &tails[0]).expect("linreg decodes");
            let got = match chains.as_slice() {
                [ChainState::LinregFlow {
                    sum_y,
                    sum_xy,
                    n,
                    period,
                }] => (*sum_y, *sum_xy, *n, *period),
                other => panic!(
                    "linreg p={p}: decoded {} entries ({:?}), expected one LinregFlow",
                    other.len(),
                    other
                ),
            };

            let (_h, hand) = Linreg::indicator(&streams, &[p], Some(&[true, true])).unwrap();
            match hand.carried_state() {
                ChainState::LinregFlow {
                    sum_y,
                    sum_xy,
                    n,
                    period,
                } => {
                    assert_eq!(got.0.to_bits(), sum_y.to_bits(), "linreg p={p} sum_y");
                    assert_eq!(got.1.to_bits(), sum_xy.to_bits(), "linreg p={p} sum_xy");
                    assert_eq!(got.2.to_bits(), n.to_bits(), "linreg p={p} n");
                    assert_eq!(got.3, period, "linreg p={p} period");
                }
                other => panic!("linreg p={p}: hand carried {other:?}, not LinregFlow"),
            }
        }
    }

    /// GATE 8 — THE PAYLOAD. Nothing upstream of this line has to be believed:
    /// a state rebuilt from the ENGINE's own captured carries must continue a
    /// series bit-for-bit, which is precisely what §R8 proved the tail-window
    /// replay could not do (all 300 resumed rows differed and never converged).
    #[test]
    fn engine_capture_decodes_to_a_continuing_state() {
        const SEAM: usize = 400;
        for p in [5.0f64, 14.0, 27.0] {
            let s = wiggle();
            let pp: [f64; 1] = [p];
            let streams = [&s[..SEAM]];
            let req = linreg_req(p);
            let (sig, vals, _o, tails) = erase_request_tails(&req).expect("linreg erases");

            let mut car = Carries::default();
            let head = linreg_n_captured(&vals, &streams, &mut car);
            let chains = carries_to_chains(&sig, &vals, &car, &tails[0]).expect("decodes");

            let p_us = p as usize;
            let win = &s[SEAM.saturating_sub(2 * p_us + 4)..SEAM];
            let mut st = <Linreg as Restorable<1, 1>>::restore_state(chains, &[win])
                .expect("linreg restorer declined the engine's own carries");

            // The head must agree with the hand BEFORE the resume is judged, or
            // a wrong head could be blamed on the state (or masked by it).
            let (full, _) = Linreg::indicator(&[s.as_slice()], &pp, Some(&[true, true])).unwrap();
            let want_head = SEAM - p_us;
            assert_eq!(
                head[0].len(),
                want_head,
                "linreg p={p}: engine head row count"
            );
            assert_bits(
                &format!("linreg p={p} engine head"),
                head[0].as_slice(),
                &full[0][..want_head],
            );

            let mut got: Vec<f64> = Vec::new();
            for chunk in s[SEAM..].chunks(256) {
                let out = st
                    .batch_indicator(&[chunk], Some(&[true, true]))
                    .expect("continuation");
                got.extend_from_slice(&out[0]);
            }
            assert_eq!(
                got.len(),
                full[0].len() - want_head,
                "linreg p={p}: resumed row count"
            );
            assert_bits(
                &format!("linreg p={p} engine-captured continuation"),
                &got,
                &full[0][want_head..],
            );
        }
    }

    /// GATE 9 — the chain family decodes per REQUEST (each `ema_chains` member
    /// is its own recipe with its own single-slot tail), and each decoded
    /// `Ema { value, period }` equals the hand's own carried value.
    #[test]
    fn decoded_ema_chains_match_hand_states() {
        let s = wiggle();
        let streams = [s.as_slice()];
        let inp = [s.as_slice()];
        let req = ema_chains_req();
        let (sig, vals, _o, tails) = erase_request_tails(&req).expect("ema_chains erases");
        assert_eq!(tails.len(), 6, "one tail chain per request");

        let mut car = Carries::default();
        ema_chains_captured(&vals, &streams, &mut car);
        for (r, slots) in tails.iter().enumerate() {
            let chains = carries_to_chains(&sig, &vals, &car, slots)
                .unwrap_or_else(|| panic!("ema request {r} refused the decode"));
            let [ChainState::Ema { value, period }] = chains.as_slice() else {
                panic!("ema request {r}: expected one Ema chain, got {chains:?}");
            };
            let (_h, st) = Ema::indicator(&inp, &[*period as f64], None).unwrap();
            assert_eq!(
                value.to_bits(),
                st.ema.to_bits(),
                "ema p={period} decoded value = {value}, hand state.ema = {}",
                st.ema
            );
        }
    }

    /// GATE 10 (original) — INVERTED by Phase C, kept as the repo's pattern
    /// demands (a refusal test that opens is the proof an obligation landed).
    /// These three shapes used to be refused because their window lived in
    /// `RingSet` with no export; `RingExport` + `Carries.rings` + the decode arms
    /// now answer them, and this gate pins the STRUCTURAL fact (the chain list is
    /// produced at all) while `engine_carry_{hma,mass,adxr}_ring_continues_bit_exact`
    /// below pins that the chains are the RIGHT ones. What stays refused — and is
    /// now gated as refused — moved to `unmapped_ring_shapes_still_refuse_the_decode`.
    #[test]
    fn ring_kinds_decode_after_the_ring_export() {
        use tulip_rs::indicators::adxr::Adxr;
        use tulip_rs::indicators::hma::Hma;
        use tulip_rs::indicators::mass::Mass;

        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        // Column counts are the recipes' own arity (`recipes::hma` 1 stream,
        // `recipes::mass` high+low, `recipes::adxr` high+low+close) — a wrong
        // count does not refuse, it panics inside the generated kernel.
        let one = [&s[..]];
        let two = [&hi[..], &lo[..]];
        let three = [&hi[..], &lo[..], &s[..]];
        let cases: Vec<(&str, &[&[f64]], Request)> = vec![
            ("hma", &one, Request::new().add(Hma::recipe(&[9.0]), None)),
            ("mass", &two, Request::new().add(Mass::recipe(&[9.0]), None)),
            (
                "adxr",
                &three,
                Request::new().add(Adxr::recipe(&[14.0]), None),
            ),
        ];
        for (label, streams, req) in cases {
            let Some((sig, vals, _o, tails)) = erase_request_tails(&req) else {
                panic!("{label}: the arena no longer erases — the refusal has nothing to refuse");
            };
            let mut car = Carries::default();
            try_run_captured(&sig, &vals, streams, &mut car).unwrap_or_else(|| {
                panic!("{label}: not dispatchable through the captured arm — nothing was refused")
            });
            let decoded = carries_to_chains(&sig, &vals, &car, &tails[0]);
            assert!(
                decoded.is_some(),
                "{label}: the ring export stopped answering this tail ({} slots) — \
                 if this is deliberate, repoint the gate and the checklist together",
                tails[0].len(),
            );
        }
    }

    // ── PHASE B4: one continuation gate per decode arm ──────────────────────

    /// Which of the two honest proofs a family's continuation is required to
    /// supply. Choosing per kind is not a weakening: the two states are different
    /// objects, and a control that only makes sense for one of them proves
    /// nothing for the other (see the control's own comment).
    enum StateSource {
        /// The hand state is rebuilt from the engine's captured carries. The
        /// gate must additionally show the carries are LOAD-BEARING: the same
        /// restorer handed no chains must refuse.
        Carries,
        /// The hand state is a raw window (the recipe publishes an empty
        /// `tail_inputs`), so no carry is involved and none can be proven. The
        /// kind's own gate must instead pin the OPTIONS CONTRACT with a negative
        /// control, or the kind stays unwired.
        WindowOnly,
        /// The hand state holds NO data at all — a unit struct — so a chain
        /// control cannot fire and there is no parameter to get wrong. This mode
        /// makes no capture claim: what the gate proves is the weaker but still
        /// real statement that the router can promise this kind a resumable
        /// state whose continuation equals the uninterrupted hand, i.e. that its
        /// lead and warm-up wiring is right. A kind only enters here when its
        /// `IndicatorState` is verified to be a unit struct.
        Stateless,
    }

    /// The engine-fed continuation, shared by every single-input family: run the
    /// head through the captured kernel, decode the request's OWN tail slots,
    /// hand those chains plus the request's own options to that indicator's
    /// `restore_state_with`, resume over the tail bars, and compare with the
    /// uninterrupted hand run after the rows its own head run printed.
    ///
    /// A family passes only if its captured carries, its decode arm and its
    /// restorer all agree — which is what makes these the completion of the
    /// table rather than a restatement of Phase A/B. Primary row only, `None`
    /// optional flags on both sides: that is the hand's default call shape, and
    /// every case here is a single-request shape where engine sink order equals
    /// hand row order.
    fn engine_resume_matches_full_run<I, const INPUTS: usize, const OP: usize>(
        label: &str,
        req: &Request,
        options: &[f64; OP],
        full: [&[f64]; INPUTS],
        state_source: StateSource,
    ) where
        I: Indicator<INPUTS, OP> + Restorable<INPUTS, OP>,
        I::IndicatorState: TIndicatorState<INPUTS>,
    {
        const SEAM: usize = 400;
        // The window handed to the restorer is the FULL head column — exactly what
        // `run_registered_with_states` hands every restorer ("the windows passed to
        // each restorer are the FULL caller columns, not a trimmed tail"). The
        // helper used to synthesize a `2 * max_option + 8`-bar tail instead, and that
        // third convention was itself a bug generator: `ao` carries NO options, so it
        // received a 10-bar window while its state needs `LONG_PERIOD = 34` bars, and
        // it reported "restorer declined the engine's own carries" while the engine
        // had handed it no carries at all (measured 2026-11-16: decoded `[]` for tail
        // slots `[]`). A window rule that no production caller uses cannot be the
        // thing that proves a production contract.
        let heads: Vec<&[f64]> = full.iter().map(|c| &c[..SEAM]).collect();
        let streams_head: [&[f64]; INPUTS] = heads.try_into().ok().expect("head column count");

        let (sig, vals, _o, tails) =
            erase_request_tails(req).unwrap_or_else(|| panic!("{label}: arena no longer erases"));
        let mut car = Carries::default();
        let eng = try_run_captured(&sig, &vals, &streams_head, &mut car)
            .unwrap_or_else(|| panic!("{label}: not dispatchable through the captured arm"));
        let chains = carries_to_chains(&sig, &vals, &car, &tails[0]).unwrap_or_else(|| {
            let kinds: Vec<u8> = tails[0].iter().map(|&k| sig.kinds[k as usize]).collect();
            panic!(
                "{label}: the decode refused its own tail — slots {:?} kinds {:?}",
                tails[0], kinds
            )
        });

        let (rows_full, _) = <I as Indicator<INPUTS, OP>>::indicator(&full, options, None)
            .unwrap_or_else(|e| panic!("{label}: hand full-series run failed: {e:?}"));
        let (rows_head, _) = <I as Indicator<INPUTS, OP>>::indicator(&streams_head, options, None)
            .unwrap_or_else(|e| panic!("{label}: hand head run failed: {e:?}"));

        // The engine head is checked against the HAND head first: a wrong head
        // could otherwise hide behind a wrong state, or mask one.
        // The engine publishes the recipe's sinks; the hand returns primaries
        // THEN its optional rows — which `indicator(…, None)` still hands back as
        // EMPTY vectors (`adxr`: 1 primary + 4 optional rows). So the engine's row
        // set is a PREFIX of the hand's, in the same order, and comparing the
        // prefix is the honest subset check rather than a false equality.
        assert!(
            eng.len() <= rows_head.len(),
            "{label}: engine published {} rows but the hand's head run returned {} \
             — the engine cannot publish MORE than the hand",
            eng.len(),
            rows_head.len(),
        );
        assert!(
            !eng.is_empty(),
            "{label}: the engine published no rows at all"
        );
        for i in 0..eng.len() {
            assert_bits(
                &format!("{label} engine head row {i}"),
                eng[i].as_slice(),
                rows_head[i].as_slice(),
            );
        }

        // One window per column, in recipe column order — the order `Restorable`
        // documents by prose only, which is why it is passed positionally here
        // rather than re-derived per family. The bars are the ones the engine just
        // consumed, so a restorer that tail-slices and a restorer that re-inits
        // from the tail are both being tested against a real seam.
        let wins: Vec<&[f64]> = full.iter().map(|c| &c[..SEAM]).collect();
        // NON-VACUITY, pinned rather than assumed — in one of two modes, chosen by
        // the caller because the two kinds of state are proven differently.
        //
        // `Carries`: with an empty chain list the restorer must REFUSE. Control
        // 2026-11-15: every carry-driven family in this suite does, so the
        // bit-exact continuation below can only have come from the engine's own
        // carries. If a window replay is ever added to one of these families, the
        // gate says so instead of quietly stopping meaning anything. (`linreg`
        // deliberately does not use this helper: its restorer KEEPS the documented
        // replay fallback, and its proof is §R8's numbers.)
        //
        // `WindowOnly`: the state carries no chain BY DESIGN — the recipe
        // publishes an empty `tail_inputs` — so demanding a refusal here would be
        // a category error, not a control. The thing that is load-bearing for such
        // a kind is the OPTIONS CONTRACT, so its own gate must pin that
        // positively: see `engine_carry_mom_continues_bit_exact`, which asserts
        // the old window-length spelling resumes WRONG as well as asserting the
        // options spelling resumes bit-exactly. A window-only kind without that
        // negative control is not wireable. This split is what C11.3 replaced:
        // relaxing the control for everyone was tried on 2026-11-15 and falsified
        // by its own new assertion (`mom` resumed 300/300 rows wrong,
        // `1.0274769512599278` vs `0.5911711585153654`).
        match state_source {
            StateSource::Carries => assert!(
                <I as Restorable<INPUTS, OP>>::restore_state_with(
                    vec![],
                    &wins,
                    options.as_slice(),
                )
                .is_none(),
                "{label}: restored a state from NO captured chains — this family now \
                 has a window-replay fallback, so the comparison below no longer proves \
                 the engine's carries are used",
            ),
            StateSource::WindowOnly | StateSource::Stateless => {
                // Nothing to assert here BY DESIGN. `WindowOnly` moves the proof
                // into the kind's own gate (the options-contract control),
                // `Stateless` has no state content left to prove at all.
            }
        }
        // The failure message carries the DECODED SHAPES because that is the only
        // way to tell "this kind needs a chain the decode cannot export" from
        // "the decode exports a chain this kind never asked for" — and the second
        // one is a recipe that over-declares `tail_inputs`, not a capture gap.
        let restored = <I as Restorable<INPUTS, OP>>::restore_state_with(
            chains.clone(),
            &wins,
            options.as_slice(),
        );
        let mut st = restored.unwrap_or_else(|| {
            panic!(
                "{label}: restorer declined the engine's own chains — decoded {:?} for tail slots \
                 {:?} (kinds {:?}); compare that variant list against this kind's own \
                 `restore_state` pattern before theorising",
                chains,
                tails[0],
                tails[0]
                    .iter()
                    .map(|&k| sig.kinds[k as usize])
                    .collect::<Vec<u8>>(),
            )
        });

        let mut got: Vec<f64> = Vec::new();
        let tail_len = full[0].len() - SEAM;
        let mut off = 0usize;
        while off < tail_len {
            let end = (off + 256).min(tail_len);
            let pieces: Vec<&[f64]> = full.iter().map(|c| &c[SEAM + off..SEAM + end]).collect();
            let chunk: [&[f64]; INPUTS] = pieces.try_into().ok().expect("continuation columns");
            let out = st
                .batch_indicator(&chunk, None)
                .unwrap_or_else(|e| panic!("{label}: continuation failed: {e:?}"));
            got.extend_from_slice(&out[0]);
            off = end;
        }
        let want = &rows_full[0][rows_head[0].len()..];
        assert_eq!(
            got.len(),
            want.len(),
            "{label}: resumed {} rows, expected {} (head printed {}, full {})",
            got.len(),
            want.len(),
            rows_head[0].len(),
            rows_full[0].len(),
        );
        assert_bits(&format!("{label} engine-captured continuation"), &got, want);
    }

    fn engine_family<I, const INPUTS: usize, const OP: usize>(
        label: &str,
        recipe: tulip_rs::cse::Recipe,
        options: &[f64; OP],
        full: [&[f64]; INPUTS],
    ) where
        I: Indicator<INPUTS, OP> + Restorable<INPUTS, OP>,
        I::IndicatorState: TIndicatorState<INPUTS>,
    {
        let req = Request::new().add(recipe, None);
        engine_resume_matches_full_run::<I, INPUTS, OP>(
            label,
            &req,
            options,
            full,
            StateSource::Carries,
        );
    }

    /// The same continuation for a kind whose hand state is a raw window rather
    /// than decoded carries (`tail_inputs` empty by construction). No carry proof
    /// is possible for these, so the caller's own gate must pin the options
    /// contract instead — see `StateSource::WindowOnly`.
    fn engine_window_family<I, const INPUTS: usize, const OP: usize>(
        label: &str,
        recipe: tulip_rs::cse::Recipe,
        options: &[f64; OP],
        full: [&[f64]; INPUTS],
    ) where
        I: Indicator<INPUTS, OP> + Restorable<INPUTS, OP>,
        I::IndicatorState: TIndicatorState<INPUTS>,
    {
        let req = Request::new().add(recipe, None);
        engine_resume_matches_full_run::<I, INPUTS, OP>(
            label,
            &req,
            options,
            full,
            StateSource::WindowOnly,
        );
    }

    /// The same continuation for a kind whose hand state is a unit struct. See
    /// `StateSource::Stateless` for exactly what this does and does not claim.
    fn engine_stateless_family<I, const INPUTS: usize, const OP: usize>(
        label: &str,
        recipe: tulip_rs::cse::Recipe,
        options: &[f64; OP],
        full: [&[f64]; INPUTS],
    ) where
        I: Indicator<INPUTS, OP> + Restorable<INPUTS, OP>,
        I::IndicatorState: TIndicatorState<INPUTS>,
    {
        let req = Request::new().add(recipe, None);
        engine_resume_matches_full_run::<I, INPUTS, OP>(
            label,
            &req,
            options,
            full,
            StateSource::Stateless,
        );
    }

    /// The single-column families, one gate per decode arm.
    #[test]
    fn engine_carry_sma_continues_bit_exact() {
        use tulip_rs::indicators::sma::Sma;
        let s = wiggle();
        engine_family::<Sma, 1, 1>("sma", Sma::recipe(&[14.0]), &[14.0], [&s[..]]);
    }

    #[test]
    fn engine_carry_stddev_continues_bit_exact() {
        use tulip_rs::indicators::stddev::StdDev;
        let s = wiggle();
        engine_family::<StdDev, 1, 1>("stddev", StdDev::recipe(&[14.0]), &[14.0], [&s[..]]);
    }

    #[test]
    fn engine_carry_wma_continues_bit_exact() {
        use tulip_rs::indicators::wma::Wma;
        let s = wiggle();
        engine_family::<Wma, 1, 1>("wma", Wma::recipe(&[14.0]), &[14.0], [&s[..]]);
    }

    #[test]
    fn engine_carry_wilders_continues_bit_exact() {
        use tulip_rs::indicators::wilders::Wilders;
        let s = wiggle();
        engine_family::<Wilders, 1, 1>("wilders", Wilders::recipe(&[14.0]), &[14.0], [&s[..]]);
    }

    #[test]
    fn engine_carry_rollmax_continues_bit_exact() {
        use tulip_rs::indicators::max::Max;
        let s = wiggle();
        engine_family::<Max, 1, 1>("max", Max::recipe(&[14.0]), &[14.0], [&s[..]]);
    }

    #[test]
    fn engine_carry_rollmin_continues_bit_exact() {
        use tulip_rs::indicators::min::Min;
        let s = wiggle();
        engine_family::<Min, 1, 1>("min", Min::recipe(&[14.0]), &[14.0], [&s[..]]);
    }

    #[test]
    fn engine_carry_highpass_continues_bit_exact() {
        use tulip_rs::indicators::highpass::HighPass;
        let s = wiggle();
        engine_family::<HighPass, 1, 1>("highpass", HighPass::recipe(&[14.0]), &[14.0], [&s[..]]);
    }

    #[test]
    fn engine_carry_supersmoother_continues_bit_exact() {
        use tulip_rs::indicators::supersmoother::SuperSmoother;
        let s = wiggle();
        engine_family::<SuperSmoother, 1, 1>(
            "supersmoother",
            SuperSmoother::recipe(&[14.0]),
            &[14.0],
            [&s[..]],
        );
    }

    /// `SeededEma` decodes through the SAME `ChainState::Ema` arm as `Ema`, so this
    /// gate is the evidence that the arm covers the seeded shape (whose seed differs
    /// from its fold source) rather than only the plain one.
    #[test]
    fn engine_carry_seeded_ema_continues_bit_exact() {
        use tulip_rs::indicators::zlema::Zlema;
        let s = wiggle();
        engine_family::<Zlema, 1, 1>("zlema", Zlema::recipe(&[14.0]), &[14.0], [&s[..]]);
    }

    /// B5 landed. `supertrend`'s tail is `[Wilders, Mirror, SuperTrend]` (kinds
    /// `[4, 12, 15]` at slots `[11, 16, 15]`, measured); the mirror used to be the
    /// single slot that refused the whole request, because a mirror's exit value is
    /// its source's value on the LAST CONSUMED bar and every slot's value in the
    /// steady loop is a register local that dies with the iteration. The Mirror arm
    /// now stores it into `v[k]` — the array `exec`'s `K_MIRROR` arm already reads —
    /// the epilogue exports `v`, and the decode reads it back. This is the
    /// continuation gate that refusal was blocking, plus an independent read of the
    /// mirror slot against the raw price column: a `v[k]` that stopped being
    /// maintained would fail here instead of silently serving a warm-seam value.
    #[test]
    fn engine_carry_supertrend_continues_bit_exact() {
        use tulip_rs::cse::signature::{K_MIRROR, K_SUPERTREND, K_WILDERS};
        use tulip_rs::indicators::supertrend::SuperTrend;

        const SEAM: usize = 400;
        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        engine_family::<SuperTrend, 3, 2>(
            "supertrend",
            SuperTrend::recipe(&[14.0, 3.0]),
            &[14.0, 3.0],
            [&hi[..], &lo[..], &s[..]],
        );

        let req = Request::new().add(SuperTrend::recipe(&[14.0, 3.0]), None);
        let (sig, vals, _o, tails) = erase_request_tails(&req).expect("supertrend erases");
        let kinds: Vec<u8> = tails[0].iter().map(|&k| sig.kinds[k as usize]).collect();
        assert_eq!(
            kinds,
            vec![K_WILDERS, K_MIRROR, K_SUPERTREND],
            "supertrend's tail shape drifted",
        );
        let m = tails[0][1] as usize;
        let streams = [&hi[..], &lo[..], &s[..SEAM]];
        let mut car = Carries::default();
        try_run_captured(&sig, &vals, &streams, &mut car).expect("supertrend dispatchable");
        assert_eq!(
            car.v[m].to_bits(),
            s[SEAM - 1].to_bits(),
            "mirror slot {m}: stored value = {}, close[last consumed bar] = {}",
            car.v[m],
            s[SEAM - 1],
        );
    }

    /// GATE 10 (original) — INVERTED by Phase C, kept as the repo's pattern
    /// demands (a refusal test that opens is the proof an obligation landed).
    /// `hma`, `mass`, `adxr` (ring windows) and `hilbert` (the Ht ring, plus a
    /// recipe that declared no tails at all) are served now and gated as
    /// continuations; `kvo` was the last ring-family refusal and C5 closed it.
    ///
    /// So this gate pins a refusal of a DIFFERENT class: `msw`, whose tail is a
    /// `SdftRotate` node — a kind with no `ChainState` variant at all. Captured
    /// is not the same as decodable, and the router must withdraw states rather
    /// than invent one.
    #[test]
    fn unmapped_kinds_still_refuse_the_decode() {
        use tulip_rs::indicators::msw::Msw;

        let s = wiggle();
        let one = [&s[..]];
        let cases: Vec<(&str, &[&[f64]], Request)> =
            vec![("msw", &one, Request::new().add(Msw::recipe(&[14.0]), None))];
        for (label, streams, req) in cases {
            let Some((sig, vals, _o, tails)) = erase_request_tails(&req) else {
                panic!("{label}: the arena no longer erases — the refusal has nothing to refuse");
            };
            let mut car = Carries::default();
            try_run_captured(&sig, &vals, streams, &mut car).unwrap_or_else(|| {
                panic!("{label}: not dispatchable through the captured arm — nothing was refused")
            });
            let decoded = carries_to_chains(&sig, &vals, &car, &tails[0]);
            assert!(
                decoded.is_none(),
                "{label}: decoded {:?} — a kind with no `ChainState` variant was answered",
                decoded
            );
        }
    }

    /// C5 CLOSED. `kvo`'s third tail slot is the `KvoFlow` node, and the emitted
    /// steady arm stores `kvacc[k] = [hlc, cm, dm]` — which is precisely the NEXT
    /// bar's `[prev_hlc, cm, prev_hl]` — with the trend flip-flop riding `st[k]`
    /// as 1.0/0.0 (same convention `warmup.rs`'s `K_KVOFLOW` arm uses, so the
    /// warm→steady handoff is one rule, not two). The decode arm reads those four
    /// fields and `kvo::restore_state` matches `[Ema, Ema, KvoFlow]`, which is
    /// what `recipes::kvo` declares.
    #[test]
    fn engine_carry_kvo_continues_bit_exact() {
        use tulip_rs::indicators::kvo::Kvo;

        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        let vo: Vec<f64> = s
            .iter()
            .map(|v| 1_000_000.0 + (v * 7.0).sin() * 5_000.0)
            .collect();
        engine_family::<Kvo, 4, 2>(
            "kvo",
            Kvo::recipe(&[14.0, 34.0]),
            &[14.0, 34.0],
            [&hi[..], &lo[..], &s[..], &vo[..]],
        );
    }

    /// The same five kinds reached through the ROUTER, which is the surface a
    /// caller uses. `restorer_for` covered 61 of 93 `Restorable` impls, so
    /// capture + decode + a proven resume was still unreachable for `kvo`,
    /// `hilbert`, `supertrend`, `adxr` and `mass` until their arms and box
    /// variants were added. This gate asserts the promise (`Some`) for each —
    /// the resume itself is gated per kind above, so this is the wiring proof,
    /// not a second copy of it.
    #[test]
    fn router_now_reaches_the_newly_wired_kinds() {
        use tulip_rs::indicators::ad::Ad;
        use tulip_rs::indicators::adosc::Adosc;
        use tulip_rs::indicators::adx::Adx;
        use tulip_rs::indicators::adxr::Adxr;
        use tulip_rs::indicators::ao::Ao;
        use tulip_rs::indicators::atr::Atr;
        use tulip_rs::indicators::bbands::BBands;
        use tulip_rs::indicators::cci::Cci;
        use tulip_rs::indicators::chandelierexit::ChandelierExit;
        use tulip_rs::indicators::cvi::Cvi;
        use tulip_rs::indicators::di::Di;
        use tulip_rs::indicators::dm::Dm;
        use tulip_rs::indicators::dx::Dx;
        use tulip_rs::indicators::ef::Ef;
        use tulip_rs::indicators::elderray::Elderray;
        use tulip_rs::indicators::fisher::Fisher;
        use tulip_rs::indicators::hilberttransform::HilbertTransform;
        use tulip_rs::indicators::kvo::Kvo;
        use tulip_rs::indicators::mass::Mass;
        use tulip_rs::indicators::md::Md;
        use tulip_rs::indicators::medprice::Medprice;
        use tulip_rs::indicators::mom::Mom;
        use tulip_rs::indicators::natr::Natr;
        use tulip_rs::indicators::nvi::Nvi;
        use tulip_rs::indicators::pvi::Pvi;
        use tulip_rs::indicators::stoch::Stoch;
        use tulip_rs::indicators::stochrsi::StochRsi;
        use tulip_rs::indicators::supertrend::SuperTrend;
        use tulip_rs::indicators::tr::Tr;
        use tulip_rs::indicators::trvi::Trvi;
        use tulip_rs::indicators::ultosc::Ultosc;
        use tulip_rs::indicators::vhf::Vhf;

        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        let vo: Vec<f64> = s
            .iter()
            .map(|v| 1_000_000.0 + (v * 7.0).sin() * 5_000.0)
            .collect();
        let one = [&s[..]];
        let two = [&hi[..], &lo[..]];
        let three = [&hi[..], &lo[..], &s[..]];
        let four = [&hi[..], &lo[..], &s[..], &vo[..]];
        // `(close, volume)` — the column order the NVI/PVI recipes declare.
        let cv = [&s[..], &vo[..]];
        let cases: Vec<(&str, &[&[f64]], Request)> = vec![
            (
                "kvo",
                &four,
                Request::new().add(Kvo::recipe(&[14.0, 34.0]), None),
            ),
            (
                "hilbert",
                &one,
                Request::new().add(HilbertTransform::recipe(&[7.0, 9.0]), None),
            ),
            (
                "supertrend",
                &three,
                Request::new().add(SuperTrend::recipe(&[14.0, 3.0]), None),
            ),
            (
                "adxr",
                &three,
                Request::new().add(Adxr::recipe(&[14.0]), None),
            ),
            ("mass", &two, Request::new().add(Mass::recipe(&[9.0]), None)),
            ("dm", &two, Request::new().add(Dm::recipe(&[14.0]), None)),
            ("di", &three, Request::new().add(Di::recipe(&[14.0]), None)),
            ("dx", &three, Request::new().add(Dx::recipe(&[14.0]), None)),
            (
                "adx",
                &three,
                Request::new().add(Adx::recipe(&[14.0]), None),
            ),
            (
                "atr",
                &three,
                Request::new().add(Atr::recipe(&[14.0]), None),
            ),
            (
                "natr",
                &three,
                Request::new().add(Natr::recipe(&[14.0]), None),
            ),
            // C9 batch 2 — the seven kinds proven by the batch-2 gate. The eight
            // that failed it are NOT here: they must keep falling to the hand.
            ("tr", &three, Request::new().add(Tr::recipe(&[]), None)),
            ("ad", &four, Request::new().add(Ad::recipe(&[]), None)),
            (
                "bbands",
                &one,
                Request::new().add(BBands::recipe(&[20.0, 2.0]), None),
            ),
            ("cvi", &two, Request::new().add(Cvi::recipe(&[20.0]), None)),
            (
                "elderray",
                &three,
                Request::new().add(Elderray::recipe(&[10.0]), None),
            ),
            (
                "chandelierexit",
                &three,
                Request::new().add(ChandelierExit::recipe(&[22.0, 3.0]), None),
            ),
            (
                "trvi",
                &three,
                Request::new().add(Trvi::recipe(&[14.0]), None),
            ),
            // C11.3 first migration — gated by `engine_carry_mom_continues_bit_exact`
            // (both the options-fed resume and the window-length negative control).
            // Its state carries no chain, so this case is also the router-side proof
            // that the full-column window plus `recipe.options` restores the RIGHT
            // period: the same call before the shim produced a state for a different
            // indicator.
            ("mom", &one, Request::new().add(Mom::recipe(&[14.0]), None)),
            // C11.3: a stateless kind must still be reachable, or the promise is
            // withdrawn for want of an arm. Its state carries nothing, so the
            // claim here is only "the router answers it".
            (
                "medprice",
                &two,
                Request::new().add(Medprice::recipe(&[]), None),
            ),
            // C11.3: proven bit-exact by the batch-2 probe once the recipe's
            // `prev` mirror depth was corrected.
            ("ef", &one, Request::new().add(Ef::recipe(&[14.0]), None)),
            // C11.3: proven bit-exact by the same probe once `vhf::restore_state`
            // sliced the `period + 1` bars its `init_state` seeds from.
            ("vhf", &one, Request::new().add(Vhf::recipe(&[14.0]), None)),
            // C12: ao's state is carried now (two sums + the medprice ring), so
            // the router must answer it rather than withdraw the promise.
            ("ao", &two, Request::new().add(Ao::recipe(&[]), None)),
            // C12: gated by `engine_carry_cci_continues_bit_exact`.
            ("md", &one, Request::new().add(Md::recipe(&[14.0]), None)),
            (
                "cci",
                &three,
                Request::new().add(Cci::recipe(&[14.0]), None),
            ),
            // C12: gated by the batch-2 probe; carried, not rebuilt.
            (
                "stoch",
                &three,
                Request::new().add(Stoch::recipe(&[14.0, 3.0, 3.0]), None),
            ),
            // C12: gated by `engine_carry_adosc_continues_bit_exact`.
            (
                "adosc",
                &four,
                Request::new().add(Adosc::recipe(&[3.0, 10.0]), None),
            ),
            // C12: gated by `engine_carry_nvi_pvi_continue_bit_exact`. Column
            // order is the recipe's own: `(close, volume)`.
            ("nvi", &cv, Request::new().add(Nvi::recipe(&[]), None)),
            ("pvi", &cv, Request::new().add(Pvi::recipe(&[]), None)),
            // C11.5: gated by `engine_carry_fisher_continues_bit_exact`.
            (
                "fisher",
                &two,
                Request::new().add(Fisher::recipe(&[9.0]), None),
            ),
            // C12: gated by the batch-2 probe; six carried sums + a sliced ring.
            (
                "ultosc",
                &three,
                Request::new().add(Ultosc::recipe(&[7.0, 14.0, 28.0]), None),
            ),
            // C14: the last kind. Gated by the same probe once `decode` handed
            // over the roll's window and the graft handed over rsi's channels.
            (
                "stochrsi",
                &one,
                Request::new().add(StochRsi::recipe(&[14.0]), None),
            ),
        ];
        for (label, streams, req) in cases {
            let run = run_registered_with_states(&req, streams)
                .unwrap_or_else(|| panic!("{label}: no registered kernel serves it at all"));
            let states = run.states.unwrap_or_else(|| {
                panic!(
                    "{label}: rows served but states withdrawn — decode, box variant or \
                     restorer_for arm is still missing"
                )
            });
            assert_eq!(states.len(), 1, "{label}: one request, one state");
        }
    }

    // ── PHASE C: the ring export ──────────────────────────────────────────

    // ── C9 batch 1: gate FIRST, wire only what passes ─────────────────────

    /// The dm/di/dx/adx + atr/natr family tails are built entirely from kinds
    /// the table already decodes (`Wilders` for the smoothed sums, `Mirror` for
    /// the previous-bar prices, which B5 made reachable). So these six are the
    /// cheapest path from "captured" to "promiseable", and each is gated BEFORE
    /// its `restorer_for` arm goes in — wiring without a passing gate would
    /// promise a state on reasoning alone, which is the §R8 failure class.
    #[test]
    fn c9_batch1_dm_di_dx_adx_atr_natr_continue_bit_exact() {
        use tulip_rs::indicators::adx::Adx;
        use tulip_rs::indicators::atr::Atr;
        use tulip_rs::indicators::di::Di;
        use tulip_rs::indicators::dm::Dm;
        use tulip_rs::indicators::dx::Dx;
        use tulip_rs::indicators::natr::Natr;

        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        let two = [&hi[..], &lo[..]];
        let three = [&hi[..], &lo[..], &s[..]];

        // Each case is reported by name so a failure names the kind, not the loop.
        let mut failed: Vec<&str> = Vec::new();
        let r = std::panic::catch_unwind(|| {
            engine_family::<Dm, 2, 1>("dm", Dm::recipe(&[14.0]), &[14.0], two)
        });
        if r.is_err() {
            failed.push("dm");
        }
        let r = std::panic::catch_unwind(|| {
            engine_family::<Di, 3, 1>("di", Di::recipe(&[14.0]), &[14.0], three)
        });
        if r.is_err() {
            failed.push("di");
        }
        let r = std::panic::catch_unwind(|| {
            engine_family::<Dx, 3, 1>("dx", Dx::recipe(&[14.0]), &[14.0], three)
        });
        if r.is_err() {
            failed.push("dx");
        }
        let r = std::panic::catch_unwind(|| {
            engine_family::<Adx, 3, 1>("adx", Adx::recipe(&[14.0]), &[14.0], three)
        });
        if r.is_err() {
            failed.push("adx");
        }
        let r = std::panic::catch_unwind(|| {
            engine_family::<Atr, 3, 1>("atr", Atr::recipe(&[14.0]), &[14.0], three)
        });
        if r.is_err() {
            failed.push("atr");
        }
        let r = std::panic::catch_unwind(|| {
            engine_family::<Natr, 3, 1>("natr", Natr::recipe(&[14.0]), &[14.0], three)
        });
        if r.is_err() {
            failed.push("natr");
        }
        assert!(
            failed.is_empty(),
            "C9 batch 1 kinds that did NOT resume bit-exactly from engine captures: {:?} \
             (each is wired only after its gate passes)",
            failed
        );
    }

    /// C9 batch 2 — the group-1 kinds (tails built only from already-mapped
    /// slot kinds), gated BEFORE any arm goes in. One `catch_unwind` per kind so
    /// a single run names EVERY failure instead of stopping at the first: a
    /// partial batch must not look like a clean one. Columns follow the user's
    /// law (`open, high, low, close, volume`, subsets shifted left), so `I=1` is
    /// close, `I=2` high+low, `I=3` high+low+close, `I=4` high+low+close+volume;
    /// a head-row mismatch here would mean MY column order is wrong, not that the
    /// capture is, and the helper's own empty-chain assert keeps a replay
    /// fallback from passing silently.
    #[test]
    fn c9_batch2_group1_kinds_continue_bit_exact() {
        use tulip_rs::indicators::ad::Ad;
        use tulip_rs::indicators::ao::Ao;
        use tulip_rs::indicators::bbands::BBands;
        use tulip_rs::indicators::chandelierexit::ChandelierExit;
        use tulip_rs::indicators::cvi::Cvi;
        use tulip_rs::indicators::ef::Ef;
        use tulip_rs::indicators::elderray::Elderray;
        use tulip_rs::indicators::medprice::Medprice;
        use tulip_rs::indicators::mom::Mom;
        use tulip_rs::indicators::stoch::Stoch;
        use tulip_rs::indicators::stochrsi::StochRsi;
        use tulip_rs::indicators::tr::Tr;
        use tulip_rs::indicators::trvi::Trvi;
        use tulip_rs::indicators::ultosc::Ultosc;
        use tulip_rs::indicators::vhf::Vhf;

        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        let vo: Vec<f64> = s
            .iter()
            .map(|v| 1_000_000.0 + (v * 7.0).sin() * 5_000.0)
            .collect();
        let one = [&s[..]];
        let two = [&hi[..], &lo[..]];
        let three = [&hi[..], &lo[..], &s[..]];
        let four = [&hi[..], &lo[..], &s[..], &vo[..]];

        let mut failed: Vec<&str> = Vec::new();
        // The helper is a parameter because the three `StateSource` modes prove
        // different things, and picking the wrong one silently weakens a gate:
        // `engine_family` asserts the carries are load-bearing,
        // `engine_window_family` skips that control and requires the kind's own
        // gate to pin the options contract, `engine_stateless_family` skips it for
        // a unit-struct state that has no content to prove.
        macro_rules! probe {
            (
                $name:literal, $helper:ident, $ty:ident, $I:literal, $O:literal,
                $recipe:expr, $opts:expr, $cols:expr
            ) => {{
                let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    $helper::<$ty, $I, $O>($name, $recipe, $opts, $cols);
                }));
                if caught.is_err() {
                    failed.push($name);
                }
            }};
        }
        probe!("tr", engine_family, Tr, 3, 0, Tr::recipe(&[]), &[], three);
        probe!(
            "medprice",
            engine_stateless_family,
            Medprice,
            2,
            0,
            Medprice::recipe(&[]),
            &[],
            two
        );
        probe!("ao", engine_family, Ao, 2, 0, Ao::recipe(&[]), &[], two);
        probe!("ad", engine_family, Ad, 4, 0, Ad::recipe(&[]), &[], four);
        probe!(
            "mom",
            engine_window_family,
            Mom,
            1,
            1,
            Mom::recipe(&[14.0]),
            &[14.0],
            one
        );
        probe!(
            "ef",
            engine_family,
            Ef,
            1,
            1,
            Ef::recipe(&[14.0]),
            &[14.0],
            one
        );
        probe!(
            "vhf",
            engine_family,
            Vhf,
            1,
            1,
            Vhf::recipe(&[14.0]),
            &[14.0],
            one
        );
        probe!(
            "bbands",
            engine_family,
            BBands,
            1,
            2,
            BBands::recipe(&[20.0, 2.0]),
            &[20.0, 2.0],
            one
        );
        probe!(
            "cvi",
            engine_family,
            Cvi,
            2,
            1,
            Cvi::recipe(&[20.0]),
            &[20.0],
            two
        );
        probe!(
            "ultosc",
            engine_family,
            Ultosc,
            3,
            3,
            Ultosc::recipe(&[7.0, 14.0, 28.0]),
            &[7.0, 14.0, 28.0],
            three
        );
        probe!(
            "elderray",
            engine_family,
            Elderray,
            3,
            1,
            Elderray::recipe(&[10.0]),
            &[10.0],
            three
        );
        probe!(
            "chandelierexit",
            engine_family,
            ChandelierExit,
            3,
            2,
            ChandelierExit::recipe(&[22.0, 3.0]),
            &[22.0, 3.0],
            three
        );
        probe!(
            "stochrsi",
            engine_family,
            StochRsi,
            1,
            1,
            StochRsi::recipe(&[14.0]),
            &[14.0],
            one
        );
        probe!(
            "stoch",
            engine_family,
            Stoch,
            3,
            3,
            Stoch::recipe(&[14.0, 3.0, 3.0]),
            &[14.0, 3.0, 3.0],
            three
        );
        probe!(
            "trvi",
            engine_family,
            Trvi,
            3,
            1,
            Trvi::recipe(&[14.0]),
            &[14.0],
            three
        );
        // MEASURED outcome, pinned both ways so any change is a signal: ALL
        // fifteen kinds probed now resume bit-exactly and are wired into
        // `restorer_for`, so the pinned list is EMPTY — and it must stay that way.
        // Any kind reappearing here is a regression in capture, decode or the
        // restorer; deleting this assertion would be the wrong response to a
        // future failure, because the list is what makes a silent drop visible.
        //
        // How it emptied (2026-11-16, C11.3/C12/C13/C14): `mom` needed the period
        // from `recipe.options` rather than the window length; `medprice`'s state
        // is a unit struct, so the chain control was a category error for it;
        // `ef`/`md`/`cci`/`stoch`/`ultosc` matched `ChainState::Sma` where their own
        // recipes emit `RollSumRing`; `ef` additionally mirrored `Lag{real, 1}` one
        // bar behind the hand's `prev`; `ao`/`cci`/`stoch`/`ultosc`/`stochrsi` were
        // rebuilding through `init_state` while the values were already carried;
        // `adosc`/`nvi`/`pvi`/`fisher` needed only a gate (their `Mirror`/`Prev`
        // exports already existed — `K_PREV`'s decode arm was the missing line);
        // and `stochrsi` needed the roll's window handed over by `decode` plus the
        // grafted `rsi`'s own channels via `Grafted::tails`.
        let mut expected_fail: [&str; 0] = [];
        expected_fail.sort();
        failed.sort();
        assert_eq!(
            failed, expected_fail,
            "C9 batch 2: the set of kinds that did not resume bit-exactly changed. \
             Anything newly failing is a regression; anything newly PASSING must be wired \
             into restorer_for + IndicatorStateBox and removed from this expected list."
        );
    }

    /// C6 CLOSED. `hilbert` used to declare NO `tail_inputs` while its restorer
    /// required `[HighPass, SuperSmoother, HtRing]` — the chain list came back
    /// empty, so the engine could never resume it (the rows were fine; only
    /// state was unreachable). Fixed on both sides: the recipe now publishes the
    /// three slots in the restorer's order, `RingExport` carries a CHRONOLOGICAL
    /// window (`chrono`) alongside the raw slots, and the decode maps
    /// `K_HTINPHASE`/`K_HTQUADRATURE` to `HtRing` from `chrono` — because that
    /// consumer rebuilds a `FixedRingBuffer` by pushing in order and has no
    /// cursor field, so raw slot order would rebuild a rotated ring.
    /// This replaces the tripwire that pinned the gap.
    #[test]
    fn engine_carry_hilbert_ring_continues_bit_exact() {
        use tulip_rs::indicators::hilberttransform::HilbertTransform;

        let s = wiggle();
        engine_family::<HilbertTransform, 1, 2>(
            "hilbert",
            HilbertTransform::recipe(&[7.0, 9.0]),
            &[7.0, 9.0],
            [&s[..]],
        );
    }

    /// C11.3's first migration, gated BEFORE its `restorer_for` arm went in.
    /// `mom` is the kind that proved the missing contract: its hand state is the
    /// last `period` bars of `real` and its recipe publishes no `tail_inputs`, so
    /// a restore can only be right if it is TOLD the period. Both halves are
    /// asserted because the first alone would be vacuous:
    ///
    /// 1. `restore_state_with` (options-fed) resumes bit-exactly against the
    ///    uninterrupted hand — the promise `restorer_for` now makes;
    /// 2. `restore_state` (period from the window length), the spelling that
    ///    shipped before the shim, does NOT — which is what makes the options
    ///    vector load-bearing rather than merely tidy. The window handed over is
    ///    the same generous `2 * period + 8`-bar tail the shared helper builds,
    ///    i.e. the shape the router's full-column law produces.
    #[test]
    fn engine_carry_mom_continues_bit_exact() {
        use tulip_rs::indicators::mom::Mom;

        const SEAM: usize = 400;
        const PERIOD: usize = 14;
        let s = wiggle();
        engine_window_family::<Mom, 1, 1>("mom", Mom::recipe(&[14.0]), &[14.0], [&s[..]]);

        // Half 2 — the negative control: same seam, same window, same resume
        // loop, only the restore differs.
        let win = &s[SEAM - (2 * PERIOD + 8)..SEAM];
        let mut window_state = <Mom as Restorable<1, 1>>::restore_state(vec![], &[win])
            .expect("the window-length spelling accepts any non-empty window — that IS the bug");
        let (rows_full, _) = Mom::indicator(&[&s[..]], &[14.0], None).expect("hand full run");
        let (rows_head, _) = Mom::indicator(&[&s[..SEAM]], &[14.0], None).expect("hand head run");
        let want = &rows_full[0][rows_head[0].len()..];

        let mut got: Vec<f64> = Vec::new();
        let mut off = 0usize;
        while off < s.len() - SEAM {
            let end = (off + 256).min(s.len() - SEAM);
            let out = window_state
                .batch_indicator(&[&s[SEAM + off..SEAM + end]], None)
                .expect("window-length resume");
            got.extend_from_slice(&out[0]);
            off = end;
        }
        let mismatches = got.iter().zip(want.iter()).filter(|(a, b)| a != b).count();
        assert!(
            mismatches > 0,
            "the window-length-derived period now resumes every row identically to the hand \
             ({} of {}) — so either this helper stopped passing a generous window or \
             `restore_state` was retargeted at options. Until 2026-11-15 it restored period {} \
             instead of {PERIOD} and diverged on all {} rows; that divergence is the ONLY thing \
             proving the options vector is load-bearing here.",
            mismatches,
            want.len(),
            win.len(),
            want.len(),
        );
    }

    /// C12: `md` was on the "missing `ChainState` variant" list, which was wrong —
    /// its tail is a `Node::RollSum` and the restorer matched `ChainState::Sma`,
    /// so the engine's carries were declined on a pattern mismatch. This gate is
    /// what proved the corrected match resumes bit-exactly, and it is what the
    /// `restorer_for` arm is allowed to promise.
    #[test]
    fn engine_carry_md_continues_bit_exact() {
        use tulip_rs::indicators::md::Md;
        let s = wiggle();
        engine_family::<Md, 1, 1>("md", Md::recipe(&[14.0]), &[14.0], [&s[..]]);
    }

    /// C12 probe for `cci`: its `tail_inputs` is the typprice `RollSum`, whose
    /// `RollSumRing` now carries BOTH the sum and the ring, so a fully carried
    /// restore may be reachable without any new `ChainState` variant. This gate
    /// measures it — it is expected to FAIL until `cci::restore_state` stops
    /// re-initializing over the raw price columns.
    #[test]
    fn engine_carry_cci_continues_bit_exact() {
        use tulip_rs::indicators::cci::Cci;
        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        engine_family::<Cci, 3, 1>(
            "cci",
            Cci::recipe(&[14.0]),
            &[14.0],
            [&hi[..], &lo[..], &s[..]],
        );
    }

    /// C12: `Adosc` was listed as blocked on a missing `ChainState::Accum`
    /// variant. It is not: its recipe already publishes
    /// `[ema_short, ema_long, Mirror{src: ad}]`, and `Mirror` has been decodable
    /// since B5's value store — so the AD accumulator needs no new vocabulary,
    /// only this gate. Same mis-label family as `md`/`cci`.
    #[test]
    fn engine_carry_adosc_continues_bit_exact() {
        use tulip_rs::indicators::adosc::Adosc;
        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        let vo: Vec<f64> = s
            .iter()
            .map(|v| 1_000_000.0 + (v * 7.0).sin() * 5_000.0)
            .collect();
        engine_family::<Adosc, 4, 2>(
            "adosc",
            Adosc::recipe(&[3.0, 10.0]),
            &[3.0, 10.0],
            [&hi[..], &lo[..], &s[..], &vo[..]],
        );
    }

    /// C12: `nvi`/`pvi` were recorded as needing a new node kind because their
    /// published row IS a gated multiplicative carry. The recipe has since been
    /// written with `Node::NviRecur`/`PviRecur` plus three `Mirror`s (the carry,
    /// `close`, `volume`) and the restorers already match that shape — so the
    /// remaining question is purely whether the resume is bit-exact. Gated here
    /// rather than assumed, because every prior label on this list turned out to
    /// be stale.
    #[test]
    fn engine_carry_nvi_pvi_continue_bit_exact() {
        use tulip_rs::indicators::nvi::Nvi;
        use tulip_rs::indicators::pvi::Pvi;
        let s = wiggle();
        let vo: Vec<f64> = s
            .iter()
            .map(|v| 1_000_000.0 + (v * 7.0).sin() * 5_000.0)
            .collect();
        engine_family::<Nvi, 2, 0>("nvi", Nvi::recipe(&[]), &[], [&s[..], &vo[..]]);
        engine_family::<Pvi, 2, 0>("pvi", Pvi::recipe(&[]), &[], [&s[..], &vo[..]]);
    }

    /// C11.5: `fisher` is the last window-length kind. Its two IIR carries come
    /// off the chain, and its ring/min/max are re-seeded by `init_state` from an
    /// exactly-`period`-bar tail with the period read off the request's options —
    /// so the promise here is the same one `mom` established, applied to a kind
    /// whose restorer used to derive the period from the slice length.
    #[test]
    fn engine_carry_fisher_continues_bit_exact() {
        use tulip_rs::indicators::fisher::Fisher;
        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        engine_family::<Fisher, 2, 1>("fisher", Fisher::recipe(&[9.0]), &[9.0], [&hi[..], &lo[..]]);
    }

    // ── PHASE D: the router's state guarantee ─────────────────────────────

    /// The guarantee in the shape a caller actually consumes: ONE call gives the
    /// rows and, when every request's chains decode, the hand states to resume
    /// from — no direct kernel, no manual decode, no `Carries` in sight. The
    /// resume is then checked against the uninterrupted hand run, so "promised"
    /// means bit-exact, not merely "a state object came back".
    #[test]
    fn router_promises_states_that_continue_bit_exact() {
        use tulip_rs::cse::{IndicatorStateBox, RunWithStates};
        use tulip_rs::indicator_types::TIndicatorState;

        const SEAM: usize = 400;
        let s = wiggle();
        let p = 14.0f64;
        let pp: [f64; 1] = [p];
        let head = &s[..SEAM];
        let req = linreg_req(p);
        let streams = [&head[..]];
        let RunWithStates {
            outcome,
            states,
            form: _form,
        } = run_registered_with_states(&req, &streams).expect("linreg registered");
        let states = states.expect("linreg's chains decode, so the router must promise states");
        assert_eq!(states.len(), 1, "one request, one state");
        let mut st = match states.into_iter().next().unwrap() {
            IndicatorStateBox::Linreg(s) => s,
            _other => panic!(
                "expected the hand Linreg state box; got a different variant \
                 (IndicatorStateBox is not Debug — match on the variant you expected)"
            ),
        };
        let (full, _) = Linreg::indicator(&[s.as_slice()], &pp, Some(&[true, true])).unwrap();
        let want_head = outcome.rows[0].len();
        assert_bits(
            "router head",
            outcome.rows[0].as_slice(),
            &full[0][..want_head],
        );
        let mut got: Vec<f64> = Vec::new();
        for chunk in s[SEAM..].chunks(256) {
            let out = st
                .batch_indicator(&[&chunk[..]], Some(&[true, true]))
                .expect("router state continues");
            got.extend_from_slice(&out[0]);
        }
        assert_bits(
            "router-promised state continuation",
            &got,
            &full[0][want_head..],
        );
    }

    /// The other half of the contract, and the more dangerous one: when a tail
    /// does NOT decode, the rows still ship and `states` is `None` — never a
    /// partial list, never a wrong machine. `kvo` used to be this gate's case and
    /// C5 wired it (see `engine_carry_kvo_continues_bit_exact` /
    /// `router_now_reaches_the_newly_wired_kinds`), so the refusal is now pinned
    /// on `msw`, whose tail is a `SdftRotate` node with no `ChainState` variant
    /// at all — the honest boundary of the table.
    #[test]
    fn router_withdraws_states_without_losing_the_rows() {
        use tulip_rs::indicators::msw::Msw;

        let s = wiggle();
        let streams = [&s[..]];
        let req = Request::new().add(Msw::recipe(&[14.0]), None);
        let run = run_registered_with_states(&req, &streams).expect("msw is registered");
        assert!(
            !run.outcome.rows.is_empty(),
            "the rows must still be served"
        );
        assert!(
            run.states.is_none(),
            "msw got promised states {:?} — a kind with no ChainState variant was \
             answered, so replace this gate with the continuation gate it now blocks",
            run.states.map(|v| v.len())
        );
        // Same request through the plain runner: identical rows, no states at all.
        let plain = run_registered(&req, &streams).expect("msw registered the same way");
        for (i, (a, b)) in run.outcome.rows.iter().zip(plain.rows.iter()).enumerate() {
            assert_bits(&format!("msw row {i} (captured vs plain dispatch)"), a, b);
        }
    }

    // ── PHASE C (continued): the ring continuations ──────────────────────

    /// `hma`'s own `psqrt`-long diff ring, exported raw with `cursor = index*8`
    /// and rebuilt by its restorer into `Buffer<Warm>` — the first ring kind to
    /// round-trip, gated as a continuation rather than a shape check.
    #[test]
    fn engine_carry_hma_ring_continues_bit_exact() {
        use tulip_rs::indicators::hma::Hma;
        let s = wiggle();
        engine_family::<Hma, 1, 1>("hma", Hma::recipe(&[9.0]), &[9.0], [&s[..]]);
    }

    /// `mass`: a `RollSum` group whose consumer (`mass.rs:262`) asserts
    /// `ring.len() == period` before rebuilding the buffer — so an export of the
    /// wrong group, or a group whose capacity exceeds this node's period, cannot
    /// pass. Two columns, `high`/`low`.
    #[test]
    fn engine_carry_mass_ring_continues_bit_exact() {
        use tulip_rs::indicators::mass::Mass;
        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        engine_family::<Mass, 2, 1>("mass", Mass::recipe(&[9.0]), &[9.0], [&hi[..], &lo[..]]);
    }

    /// `adxr`: the eight-chain tail — seven for ADX (four `Wilders`, three
    /// `Mirror`, which only became decodable in B5) plus the `RollAge` ring this
    /// phase exports. It is therefore one gate over three different mapping
    /// families, and the deepest end-to-end proof in the suite.
    #[test]
    fn engine_carry_adxr_ring_continues_bit_exact() {
        use tulip_rs::indicators::adxr::Adxr;
        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        engine_family::<Adxr, 3, 1>(
            "adxr",
            Adxr::recipe(&[14.0]),
            &[14.0],
            [&hi[..], &lo[..], &s[..]],
        );
    }
}
