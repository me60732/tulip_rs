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
    use tulip_rs::cse::{erase_request, try_run, try_run_captured};
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

    /// GATE 10 — the "must not shrink" rule as an executable refusal. Each of
    /// these kinds keeps its WINDOW inside `RingSet`, which Phase A does not
    /// export, so the decode must refuse the WHOLE request: a partial chain list
    /// would let a restorer rebuild a machine from a subset of its state and
    /// print numbers that look plausible. (Contrary to the obvious expectation,
    /// `Hma` is refused for its `√period` diff ring even though its six
    /// accumulators ARE captured.)
    #[test]
    fn ring_backed_tails_refuse_the_decode() {
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
                decoded.is_none(),
                "{label}: decoded {:?} — a ring-backed tail was answered with a \
                 partial state",
                decoded
            );
        }
    }

    // ── PHASE B4: one continuation gate per decode arm ──────────────────────

    /// The engine-fed continuation, shared by every single-input family: run the
    /// head through the captured kernel, decode the request's OWN tail slots,
    /// hand those chains to that indicator's `restore_state`, resume over the
    /// tail bars, and compare with the uninterrupted hand run after the rows its
    /// own head run printed.
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
    ) where
        I: Indicator<INPUTS, OP> + Restorable<INPUTS, OP>,
        I::IndicatorState: TIndicatorState<INPUTS>,
    {
        const SEAM: usize = 400;
        // Window scale is the LARGEST option, not the first: `min_data` is driven
        // by `options[OP-1]`, while a shape like supertrend carries a period AND a
        // multiplier — and a too-short window is accepted by some
        // `IndicatorState::new` and underflows in others.
        let pmax = options.iter().map(|&o| o as usize).max().unwrap_or(1);
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
        assert_eq!(eng.len(), rows_head.len(), "{label}: head row count");
        for i in 0..eng.len() {
            assert_bits(
                &format!("{label} engine head row {i}"),
                eng[i].as_slice(),
                rows_head[i].as_slice(),
            );
        }

        // One window per column, in recipe column order — the order `Restorable`
        // documents by prose only, which is why it is passed positionally here
        // rather than re-derived per family.
        let wins: Vec<&[f64]> = full
            .iter()
            .map(|c| &c[SEAM.saturating_sub(2 * pmax + 8)..SEAM])
            .collect();
        // NON-VACUITY, pinned rather than assumed. Control run 2026-11-15: with an
        // empty chain list ALL nine single-column restorers returned `None`
        // ("restorer declined"), so none of them falls back to a window replay —
        // the passing continuation below can only have come from the engine's own
        // carries. If a replay fallback is ever added to a family, THIS assertion
        // is what says so, instead of the gate quietly stopping meaning anything.
        // (`linreg` deliberately does not go through this helper: its restorer
        // KEEPS the documented replay fallback, and its proof is §R8's numbers —
        // a replay there diverges, so agreement proves the carries were used.)
        assert!(
            <I as Restorable<INPUTS, OP>>::restore_state(vec![], &wins).is_none(),
            "{label}: restored a state from NO captured chains — this family now \
             has a window-replay fallback, so the comparison below no longer proves \
             the engine's carries are used",
        );
        let mut st = <I as Restorable<INPUTS, OP>>::restore_state(chains, &wins)
            .unwrap_or_else(|| panic!("{label}: restorer declined the engine's own carries"));

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
        engine_resume_matches_full_run::<I, INPUTS, OP>(label, &req, options, full);
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

    /// `supertrend`'s tail is `[Wilders, Mirror, SuperTrend]` — measured, kinds
    /// `[4, 12, 15]` at slots `[11, 16, 15]`. Its own four-field arm and the
    /// `Wilders` arm both decode; the `Mirror` does not, because a mirror's value
    /// at the exit is its source's value on the LAST CONSUMED bar, and the steady
    /// loop keeps per-slot values in register locals — `v[k]` in the generated
    /// body is the array handed to `exec` for the warm bars, so it holds the seam,
    /// not the exit. Refusing here is the correct behaviour (the alternative is a
    /// partial state); the gap and its two candidate fixes are checklist item B5.
    #[test]
    fn supertrend_tail_refuses_on_its_mirror_slot() {
        use tulip_rs::cse::signature::{K_MIRROR, K_SUPERTREND, K_WILDERS};
        use tulip_rs::indicators::supertrend::SuperTrend;

        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        let req = Request::new().add(SuperTrend::recipe(&[14.0, 3.0]), None);
        let (sig, vals, _o, tails) = erase_request_tails(&req).expect("supertrend erases");
        assert_eq!(
            tails[0]
                .iter()
                .map(|&k| sig.kinds[k as usize])
                .collect::<Vec<u8>>(),
            vec![K_WILDERS, K_MIRROR, K_SUPERTREND],
            "supertrend's tail shape drifted",
        );

        let streams = [&hi[..], &lo[..], &s[..]];
        let mut car = Carries::default();
        try_run_captured(&sig, &vals, &streams, &mut car).expect("supertrend dispatchable");
        assert!(
            carries_to_chains(&sig, &vals, &car, &tails[0]).is_none(),
            "a Mirror slot decoded without a value export — B5 landed, so replace \
             this refusal gate with the continuation gate it was blocking",
        );

        // The refusal is exactly one slot deep: drop the mirror and the remaining
        // two arms do decode, which is what makes this a gap in the enum rather
        // than a broken `Supertrend` table entry.
        let without_mirror: Vec<tulip_rs::cse::Id> = tails[0]
            .iter()
            .copied()
            .filter(|&k| sig.kinds[k as usize] != K_MIRROR)
            .collect();
        let chains = carries_to_chains(&sig, &vals, &car, &without_mirror)
            .expect("Wilders + SuperTrend must decode on their own");
        assert_eq!(chains.len(), 2, "{chains:?}");
        assert!(
            matches!(chains[1], ChainState::Supertrend { .. }),
            "slot order should put SuperTrend second once the mirror is dropped, \
             got {:?}",
            chains[1]
        );
    }
}
