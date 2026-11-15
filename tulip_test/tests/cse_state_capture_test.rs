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
    use tulip_rs::cse::{erase_request, try_run, try_run_captured};
    use tulip_rs::cse::{Carries, ChainState, Planable, Request};
    use tulip_rs::indicator_types::Indicator;
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
}
