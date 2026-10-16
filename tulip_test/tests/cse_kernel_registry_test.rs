//! Macro-fused kernel registry suite: dispatch, bit-exactness vs the
//! hand indicators, mirror aliasing, runtime optional flags, param reuse, and
//! the wide-pairing twins. Moved from `tulip_rs/src/cse/registry.rs` (tests)
//! to `tulip_test/tests/` 2026-10-10 — the registry API is all `pub`, so this
//! runs against the crate exactly as an app would.
use tulip_rs::cse::*;
use tulip_rs::indicator_types::Indicator;

#[cfg(test)]
mod tests {
    use super::*;
    use tulip_rs::cse::Planable;
    use tulip_rs::indicators::ad::Ad;
    use tulip_rs::indicators::adosc::Adosc;
    use tulip_rs::indicators::apo::Apo;
    use tulip_rs::indicators::macd::Macd;
    use tulip_rs::indicators::mass::Mass;
    use tulip_rs::indicators::mom::Mom;
    use tulip_rs::indicators::natr::Natr;
    use tulip_rs::indicators::ppo::Ppo;
    use tulip_rs::indicators::ultosc::Ultosc;

    fn apo_ppo_macd_shared_sl_req() -> Request {
        Request::new()
            .add(Apo::recipe(&[12.0, 26.0]), None)
            .add(Ppo::recipe(&[12.0, 26.0]), None)
            .add(Macd::recipe(&[12.0, 26.0, 9.0]), None)
    }
    fn apo_ppo_shared_sl_req() -> Request {
        Request::new()
            .add(Apo::recipe(&[12.0, 26.0]), None)
            .add(Ppo::recipe(&[12.0, 26.0]), None)
    }

    fn series(n: usize, f: impl Fn(usize) -> f64) -> Vec<f64> {
        (0..n).map(f).collect()
    }
    fn wiggle() -> Vec<f64> {
        series(700, |i| (i as f64 * 0.7).sin() * 10.0 + 100.0)
    }

    /// VALIDATION §1: the macro-time arena equals what runtime merge+erase
    /// derives from the real recipes — literal and generated share one truth.
    #[test]
    fn fused_sig_derived() {
        let (_a, sig, _, _) = erase_request(&apo_ppo_macd_shared_sl_req()).unwrap();
        assert_eq!(
            sig, SIG_apo_ppo_macd_shared_sl,
            "APO_PPO_MACD_SHARED_SL arena drifted from merge"
        );
        let (_b, sig2, _, _) = erase_request(&apo_ppo_shared_sl_req()).unwrap();
        assert_eq!(
            sig2, SIG_apo_ppo_shared_sl,
            "APO_PPO_SHARED_SL arena drifted from merge"
        );
    }

    /// Bit-exact vs hand on the classic trio — the whole M0 claim in one test.
    #[test]
    fn apo_ppo_macd_shared_sl_bit_exact() {
        use tulip_rs::indicator_types::Indicator;
        let data = [
            wiggle(),
            series(650, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0), // FLAT (0/0 edges inside)
        ];
        for s in &data {
            let streams = [s.as_slice()];
            let out = run_registered(&apo_ppo_macd_shared_sl_req(), &streams)
                .expect("apo_ppo_macd_shared_sl registered");
            assert_eq!(out.fp, SIG_apo_ppo_macd_shared_sl.hash());
            let inp = [s.as_slice()];
            let (apo, _) =
                tulip_rs::indicators::apo::Apo::indicator(&inp, &[12.0, 26.0], None).unwrap();
            let (ppo, _) =
                tulip_rs::indicators::ppo::Ppo::indicator(&inp, &[12.0, 26.0], None).unwrap();
            let (mac, _) =
                tulip_rs::indicators::macd::Macd::indicator(&inp, &[12.0, 26.0, 9.0], None)
                    .unwrap();
            // sinks order with None flags: Apo[d,es,el], Ppo[q,es,el], Macd[d,sig,hist,es,el]
            // primaries at 0(d), 3(q), 6(d), 7(sig), 8(hist); optionals disabled
            assert_eq!(out.rows[0].as_slice(), apo[0], "apo row");
            assert_eq!(out.rows[3].as_slice(), ppo[0], "ppo row");
            assert_eq!(out.rows[6].as_slice(), mac[0], "macd row");
            assert_eq!(out.rows[7].as_slice(), mac[1], "signal row");
            assert_eq!(out.rows[8].as_slice(), mac[2], "hist row");
        }
    }

    #[test]
    fn apo_ppo_shared_sl_bit_exact() {
        use tulip_rs::indicator_types::Indicator;
        let s = series(500, |i| {
            (i as f64 * 0.37).sin() * 5.0 + 100.0 + i as f64 * 0.02
        });
        let streams = [s.as_slice()];
        let out = run_registered(&apo_ppo_shared_sl_req(), &streams)
            .expect("apo_ppo_shared_sl registered");
        let inp = [s.as_slice()];
        let (apo, _) =
            tulip_rs::indicators::apo::Apo::indicator(&inp, &[12.0, 26.0], None).unwrap();
        let (ppo, _) =
            tulip_rs::indicators::ppo::Ppo::indicator(&inp, &[12.0, 26.0], None).unwrap();
        // sinks order with None flags: Apo[d,es,el], Ppo[q,es,el]
        // primaries at 0(d), 3(q); optionals disabled
        assert_eq!(out.rows[0].as_slice(), apo[0]);
        assert_eq!(out.rows[3].as_slice(), ppo[0]);
    }

    /// The design's central claim: NEW PARAMETER VALUES NEVER NEED A NEW
    /// KERNEL — same pattern, different periods, same compiled template.
    #[test]
    fn same_kernel_different_params() {
        use tulip_rs::indicator_types::Indicator;
        let req = Request::new()
            .add(Apo::recipe(&[9.0, 19.0]), None)
            .add(Ppo::recipe(&[9.0, 19.0]), None);
        let (_a, sig, _, _) = erase_request(&req).unwrap();
        assert_eq!(
            sig, SIG_apo_ppo_shared_sl,
            "period change must not change the signature"
        );
        let s = series(400, |i| (i as f64 * 0.37).sin() * 5.0 + 100.0);
        let streams = [s.as_slice()];
        let out = run_registered(&req, &streams).expect("reused template");
        let inp = [s.as_slice()];
        let (apo, _) = tulip_rs::indicators::apo::Apo::indicator(&inp, &[9.0, 19.0], None).unwrap();
        let (ppo, _) = tulip_rs::indicators::ppo::Ppo::indicator(&inp, &[9.0, 19.0], None).unwrap();
        // sinks order with None flags: Apo[d,es,el], Ppo[q,es,el]
        // primaries at 0(d), 3(q); optionals disabled
        assert_eq!(out.rows[0].as_slice(), apo[0], "apo 9/19 exact");
        assert_eq!(out.rows[3].as_slice(), ppo[0], "ppo 9/19 exact");
    }

    /// APO_PPO_MACD_SHARED_SL template serves any (short, long, signal) — including non-classic.
    #[test]
    fn apo_ppo_macd_shared_sl_named_params() {
        use tulip_rs::indicator_types::Indicator;
        let req = Request::new()
            .add(Apo::recipe(&[7.0, 21.0]), None)
            .add(Ppo::recipe(&[7.0, 21.0]), None)
            .add(Macd::recipe(&[7.0, 21.0, 5.0]), None);
        let s = series(420, |i| 100.0 + (i as f64 * 0.11).cos() * 6.0);
        let streams = [s.as_slice()];
        let out = run_registered(&req, &streams).expect("apo_ppo_macd_shared_sl template reused");
        let inp = [s.as_slice()];
        let (ppo, _) = tulip_rs::indicators::ppo::Ppo::indicator(&inp, &[7.0, 21.0], None).unwrap();
        let (mac, _) =
            tulip_rs::indicators::macd::Macd::indicator(&inp, &[7.0, 21.0, 5.0], None).unwrap();
        // sinks order with None flags: Apo[d,es,el], Ppo[q,es,el], Macd[d,sig,hist,es,el]
        // primaries at 0(d), 3(q), 6(d), 7(sig), 8(hist); optionals disabled
        assert_eq!(out.rows[3].as_slice(), ppo[0]);
        assert_eq!(out.rows[8].as_slice(), mac[2]);
    }

    /// A shape not in the registry (distinct pairs ⇒ no chain sharing) must
    /// fall through, never mis-serve.
    #[test]
    fn unregistered_shape_falls_back() {
        let req = Request::new()
            .add(Apo::recipe(&[12.0, 26.0]), None)
            .add(Ppo::recipe(&[13.0, 27.0]), None);
        let s = wiggle();
        assert!(run_registered(&req, &[s.as_slice()]).is_none());
    }

    /// Mirror test: node-3 sinks (apo line and macd line in apo_ppo_macd_shared_sl) must share
    /// the same arena and have identical content.
    #[test]
    fn apo_ppo_macd_shared_sl_mirror_rows_alias() {
        let s = wiggle();
        let streams = [s.as_slice()];
        let out = run_registered(&apo_ppo_macd_shared_sl_req(), &streams)
            .expect("apo_ppo_macd_shared_sl registered");
        // sinks order with None flags: Apo[d,es,el], Ppo[q,es,el], Macd[d,sig,hist,es,el]
        // d (node3) mirrors at 0 and 6; q (node6) at 3
        assert_eq!(
            out.rows[0].as_slice(),
            out.rows[6].as_slice(),
            "node-3 mirrors must match"
        );
        assert!(
            out.rows[0].shares_data_with(&out.rows[6]),
            "node-3 mirrors must share arena"
        );
        // Both mirrors carry node 3's lead (ema26 ⇒ 25): len = n - 25.
        assert_eq!(out.rows[0].len(), 700 - 25, "apo row length");
        assert_eq!(
            out.rows[6].len(),
            700 - 25,
            "macd row length (node-3 alias)"
        );
    }

    /// ATR family: verify macro-time arena equals runtime merge+erase.
    #[test]
    fn atr_natr_shared_p_sig_derived() {
        let req = Request::new()
            .add(tulip_rs::indicators::atr::Atr::recipe(&[14.0]), None)
            .add(tulip_rs::indicators::natr::Natr::recipe(&[14.0]), None);
        let (_a, sig, _, _) = erase_request(&req).unwrap();
        if sig != SIG_atr_natr_shared_p {
            eprintln!(
                "sig.len={}, sig.n_sinks={}, sig.n_streams={}",
                sig.len, sig.n_sinks, sig.n_streams
            );
            eprintln!(
                "SIG_atr_natr_shared_p.len={}, SIG_atr_natr_shared_p.n_sinks={}, SIG_atr_natr_shared_p.n_streams={}",
                SIG_atr_natr_shared_p.len, SIG_atr_natr_shared_p.n_sinks, SIG_atr_natr_shared_p.n_streams
            );
            eprintln!(
                "SIG_atr_natr_shared_p kinds: {:?}",
                &SIG_atr_natr_shared_p.kinds[..15]
            );
            eprintln!("a: {:?}", &sig.a[..15]);
            eprintln!(
                "SIG_atr_natr_shared_p a: {:?}",
                &SIG_atr_natr_shared_p.a[..15]
            );
            eprintln!("b: {:?}", &sig.b[..15]);
            eprintln!(
                "SIG_atr_natr_shared_p b: {:?}",
                &SIG_atr_natr_shared_p.b[..15]
            );
            eprintln!("c: {:?}", &sig.c[..15]);
            eprintln!(
                "SIG_atr_natr_shared_p c: {:?}",
                &SIG_atr_natr_shared_p.c[..15]
            );
            eprintln!(
                "sink_src[:n_sinks]: {:?}",
                &SIG_atr_natr_shared_p.sink_src[..SIG_atr_natr_shared_p.n_sinks as usize]
            );
        }
        assert_eq!(sig, SIG_atr_natr_shared_p, "ATR arena drifted from merge");
    }

    /// ATR family bit-exact vs hand on 3 series classes.
    #[test]
    fn atr_natr_shared_p_bit_exact() {
        use tulip_rs::indicator_types::Indicator;
        let data = [
            wiggle(),
            series(650, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0), // FLAT (0/0 edges inside)
        ];
        for s in &data {
            let streams = [s.as_slice(), s.as_slice(), s.as_slice()];
            let out = run_registered(&atr_natr_shared_p_req(), &streams).expect("atr registered");
            assert_eq!(out.fp, SIG_atr_natr_shared_p.hash());
            // ATR needs 3 inputs: high, low, close
            let inp = [s.as_slice(), s.as_slice(), s.as_slice()];
            let (atr, _) = tulip_rs::indicators::atr::Atr::indicator(&inp, &[14.0], None).unwrap();
            let (natr, _) =
                tulip_rs::indicators::natr::Natr::indicator(&inp, &[14.0], None).unwrap();
            // sinks order with None flags: [w(atr), tr(disabled), natr, atr-w(disabled), tr(disabled)]
            // primaries are at 0 and 2
            assert_eq!(out.rows[0].as_slice(), atr[0], "atr row");
            assert_eq!(out.rows[2].as_slice(), natr[0], "natr row");
        }
    }

    /// Same template serves different periods — atr(9) should share SIG_atr_natr_shared_p with atr(14).
    #[test]
    fn atr_natr_shared_p_same_kernel_different_params() {
        use tulip_rs::indicator_types::Indicator;
        let req = Request::new()
            .add(tulip_rs::indicators::atr::Atr::recipe(&[9.0]), None)
            .add(tulip_rs::indicators::natr::Natr::recipe(&[9.0]), None);
        let (_a, sig, _, _) = erase_request(&req).unwrap();
        assert_eq!(
            sig, SIG_atr_natr_shared_p,
            "period change must not change the signature"
        );
        let s = series(400, |i| (i as f64 * 0.37).sin() * 5.0 + 100.0);
        let streams = [s.as_slice(), s.as_slice(), s.as_slice()];
        let out = run_registered(&req, &streams).expect("reused template");
        // ATR needs 3 inputs: high, low, close
        let inp = [s.as_slice(), s.as_slice(), s.as_slice()];
        let (atr, _) = tulip_rs::indicators::atr::Atr::indicator(&inp, &[9.0], None).unwrap();
        let (natr, _) = tulip_rs::indicators::natr::Natr::indicator(&inp, &[9.0], None).unwrap();
        // sinks order with None flags: [w(atr), tr(disabled), natr, atr-w(disabled), tr(disabled)]
        // primaries are at 0 and 2
        assert_eq!(out.rows[0].as_slice(), atr[0], "atr 9 exact");
        assert_eq!(out.rows[2].as_slice(), natr[0], "natr 9 exact");
    }

    /// ATR pattern with distinct periods falls through (no chain sharing).
    #[test]
    fn atr_natr_shared_p_distinct_periods_falls_back() {
        let req = Request::new()
            .add(tulip_rs::indicators::atr::Atr::recipe(&[12.0]), None)
            .add(tulip_rs::indicators::natr::Natr::recipe(&[14.0]), None);
        let s = wiggle();
        assert!(run_registered(&req, &[s.as_slice()]).is_none());
    }

    fn atr_natr_shared_p_req() -> Request {
        Request::new()
            .add(tulip_rs::indicators::atr::Atr::recipe(&[14.0]), None)
            .add(tulip_rs::indicators::natr::Natr::recipe(&[14.0]), None)
    }

    use tulip_rs::indicators::supertrend::SuperTrend;

    /// Verify macro-time arena equals runtime merge+erase for natr+supertrend.
    /// Field-wise comparison (not just `sig == SIG`): on drift, the failure
    /// output IS the diff (slot order and edge wiring are where mirrors rot).
    #[test]
    fn natr_supertrend_shared_p_sig_derived() {
        let req = Request::new()
            .add(tulip_rs::indicators::natr::Natr::recipe(&[14.0]), None)
            .add(SuperTrend::recipe(&[14.0, 3.0]), None);
        let (_a, sig, _, _) = erase_request(&req).unwrap();
        let n = sig.len as usize;
        assert_eq!(
            sig.kinds[..n],
            SIG_natr_supertrend_shared_p.kinds[..n],
            "kinds"
        );
        assert_eq!(sig.a[..n], SIG_natr_supertrend_shared_p.a[..n], "a");
        assert_eq!(sig.b[..n], SIG_natr_supertrend_shared_p.b[..n], "b");
        assert_eq!(sig.c[..n], SIG_natr_supertrend_shared_p.c[..n], "c");
        assert_eq!(
            sig.sink_src[..sig.n_sinks as usize],
            SIG_natr_supertrend_shared_p.sink_src[..SIG_natr_supertrend_shared_p.n_sinks as usize],
            "sinks"
        );
        assert_eq!(
            sig, SIG_natr_supertrend_shared_p,
            "natr_supertrend arena drifted from merge"
        );
    }

    /// Bit-exact vs hand on natr+supertrend — the whole M0 claim in one test.
    #[test]
    fn natr_supertrend_shared_p_bit_exact() {
        use tulip_rs::indicator_types::Indicator;
        let data = [
            wiggle(),
            series(650, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0), // FLAT (0/0 edges inside)
        ];
        for s in &data {
            let streams = [s.as_slice(), s.as_slice(), s.as_slice()];
            let out = run_registered(&natr_supertrend_shared_p_req(), &streams)
                .expect("natr_supertrend registered");
            assert_eq!(out.fp, SIG_natr_supertrend_shared_p.hash());
            // natr + supertrend needs 3 inputs: high, low, close
            let inp = [s.as_slice(), s.as_slice(), s.as_slice()];
            let (natr, _) =
                tulip_rs::indicators::natr::Natr::indicator(&inp, &[14.0], None).unwrap();
            let (st, _) = SuperTrend::indicator(&inp, &[14.0, 3.0], None).unwrap();
            // sinks order with None flags: [w(atr), tr(disabled), natr, st, atr-w(disabled), tr(disabled), med(disabled)]
            // primaries are at 0 and 3
            assert_eq!(out.rows[0].as_slice(), natr[0], "natr row");
            assert_eq!(out.rows[3].as_slice(), st[0], "supertrend row");
        }
    }

    /// Runtime optional flags: enabling optionals changes WHICH rows are
    /// stored, never WHICH template — the request must HIT the registry
    /// (same Signature), enabled rows bit-exact vs the hand's optional
    /// outputs, disabled rows empty, same-node mirrors sharing the arena.
    #[test]
    fn optional_flags_runtime() {
        use tulip_rs::indicator_types::Indicator;
        let s = wiggle();
        let inp = [s.as_slice(), s.as_slice(), s.as_slice()];
        let req = Request::new()
            .add(
                tulip_rs::indicators::natr::Natr::recipe(&[14.0]),
                Some(&[true, false]),
            )
            .add(SuperTrend::recipe(&[14.0, 3.0]), Some(&[true, true, true]));
        let out = run_registered(&req, &inp).expect("optional-on must hit the registry");
        assert_eq!(out.fp, SIG_natr_supertrend_shared_p.hash());
        let (natr, _) =
            tulip_rs::indicators::natr::Natr::indicator(&inp, &[14.0], Some(&[true, false]))
                .unwrap();
        let (st, _) = SuperTrend::indicator(&inp, &[14.0, 3.0], Some(&[true, true, true])).unwrap();
        // sinks: [natr, atrN(on), trN(off), st, atrS(on), trS(on), med(on)]
        assert_eq!(out.rows[0].as_slice(), natr[0], "natr primary");
        assert_eq!(out.rows[1].as_slice(), natr[1], "natr's enabled atr row");
        assert!(out.rows[2].is_empty(), "disabled optional carries no row");
        assert_eq!(out.rows[3].as_slice(), st[0], "supertrend primary");
        assert_eq!(out.rows[4].as_slice(), st[1], "st atr mirror row");
        assert_eq!(out.rows[5].as_slice(), st[2], "st tr row");
        assert_eq!(out.rows[6].as_slice(), st[3], "st medprice row");
        // Both atr mirrors share ONE physical row (group store, windowed).
        assert!(out.rows[1].shares_data_with(&out.rows[4]));
    }

    /// Flag placement never perturbs the PRIMARY results: same data, flags
    /// off vs all on → identical primaries and identical template (fp).
    #[test]
    fn flags_never_change_primaries() {
        let s = wiggle();
        let inp = [s.as_slice(), s.as_slice(), s.as_slice()];
        let a = run_registered(&natr_supertrend_shared_p_req(), &inp).unwrap();
        let b = run_registered(
            &Request::new()
                .add(
                    tulip_rs::indicators::natr::Natr::recipe(&[14.0]),
                    Some(&[true, true]),
                )
                .add(SuperTrend::recipe(&[14.0, 3.0]), Some(&[true, true, true])),
            &inp,
        )
        .unwrap();
        assert_eq!(a.fp, b.fp, "flags must not fork the template");
        assert_eq!(a.rows[0].as_slice(), b.rows[0].as_slice());
        assert_eq!(a.rows[3].as_slice(), b.rows[3].as_slice());
        assert!(
            a.rows[1].is_empty() && !b.rows[1].is_empty(),
            "want bits differ"
        );
    }

    /// Same template serves different periods AND steps — natr(9)+supertrend(9,
    /// 2.0) routes to the same kernel as natr(14)+supertrend(14, 3.0): params
    /// are Values, not shape.
    #[test]
    fn natr_supertrend_shared_p_same_kernel_different_params() {
        use tulip_rs::indicator_types::Indicator;
        let req = Request::new()
            .add(tulip_rs::indicators::natr::Natr::recipe(&[9.0]), None)
            .add(SuperTrend::recipe(&[9.0, 2.0]), None);
        let (_a, sig, _, _) = erase_request(&req).unwrap();
        assert_eq!(
            sig, SIG_natr_supertrend_shared_p,
            "period change must not change the signature"
        );
        let s = series(400, |i| (i as f64 * 0.37).sin() * 5.0 + 100.0);
        let streams = [s.as_slice(), s.as_slice(), s.as_slice()];
        let out = run_registered(&req, &streams).expect("reused template");
        // natr + supertrend needs 3 inputs: high, low, close
        let inp = [s.as_slice(), s.as_slice(), s.as_slice()];
        let (natr, _) = tulip_rs::indicators::natr::Natr::indicator(&inp, &[9.0], None).unwrap();
        let (st, _) = SuperTrend::indicator(&inp, &[9.0, 2.0], None).unwrap();
        // sinks order with None flags: [w(atr), tr(disabled), natr, st, ...]
        // primaries are at 0 and 3
        assert_eq!(out.rows[0].as_slice(), natr[0], "natr 9 exact");
        assert_eq!(out.rows[3].as_slice(), st[0], "supertrend 9/2 exact");
    }

    /// NATR+Supertrend pattern with distinct periods falls through (no chain sharing).
    #[test]
    fn natr_supertrend_shared_p_distinct_periods_falls_back() {
        let req = Request::new()
            .add(tulip_rs::indicators::natr::Natr::recipe(&[12.0]), None)
            .add(SuperTrend::recipe(&[14.0, 3.0]), None);
        let s = wiggle();
        assert!(run_registered(&req, &[s.as_slice()]).is_none());
    }

    fn natr_supertrend_shared_p_req() -> Request {
        Request::new()
            .add(tulip_rs::indicators::natr::Natr::recipe(&[14.0]), None)
            .add(SuperTrend::recipe(&[14.0, 3.0]), None)
    }

    /// Helper for ema_six pattern (6 distinct EMAs + signal chain).
    fn ema_six_req() -> Request {
        Request::new()
            .add(
                tulip_rs::indicators::macd::Macd::recipe(&[12.0, 24.0, 9.0]),
                None,
            )
            .add(tulip_rs::indicators::apo::Apo::recipe(&[10.0, 20.0]), None)
            .add(tulip_rs::indicators::ppo::Ppo::recipe(&[6.0, 14.0]), None)
    }

    /// VALIDATION: the macro-time arena equals what runtime merge+erase derives.
    #[test]
    fn ema_six_sig_derived() {
        let (_a, sig, _, _) = erase_request(&ema_six_req()).unwrap();
        assert_eq!(sig, SIG_ema_six, "EMA_SIX arena drifted from merge");
        // Note: ema_six_p has identical arena (same nodes/sinks), only flag differs
    }

    /// Bit-exact vs hand on the 6-distinct-EMA shape — all five primary rows.
    #[test]
    fn ema_six_bit_exact() {
        use tulip_rs::indicator_types::Indicator;
        let data = [
            wiggle(),
            series(650, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0), // FLAT
        ];
        for s in &data {
            let streams = [s.as_slice()];
            let out = run_registered(&ema_six_req(), &streams).expect("ema_six registered");
            assert_eq!(out.fp, SIG_ema_six.hash());
            let inp = [s.as_slice()];
            // Macd: d(line), sig, hist (primaries); es, el (optionals disabled)
            // Apo: d(line) (primary); es, el (optionals disabled)
            // Ppo: q(ppo) (primary); es, el (optionals disabled)
            let (mac, _) =
                tulip_rs::indicators::macd::Macd::indicator(&inp, &[12.0, 24.0, 9.0], None)
                    .unwrap();
            let (apo, _) =
                tulip_rs::indicators::apo::Apo::indicator(&inp, &[10.0, 20.0], None).unwrap();
            let (ppo, _) =
                tulip_rs::indicators::ppo::Ppo::indicator(&inp, &[6.0, 14.0], None).unwrap();
            // sinks order with None flags:
            // Macd[d,sig,hist,es,el] → primaries at 0(d), 1(sig), 2(hist)
            // Macd optionals: es(3), el(4) (disabled)
            // Apo[d,es,el] → primary at 5(d)
            // Ppo[q,es,el] → primary at 8(q)
            assert_eq!(out.rows[0].as_slice(), mac[0], "macd d(line)");
            assert_eq!(out.rows[1].as_slice(), mac[1], "macd sig");
            assert_eq!(out.rows[2].as_slice(), mac[2], "macd hist");
            // Note: Apo d is at sink 5, Ppo q at sink 8 (Macd optionals are sinks 3-4)
            assert_eq!(out.rows[5].as_slice(), apo[0], "apo d(line)");
            assert_eq!(out.rows[8].as_slice(), ppo[0], "ppo q(ppo)");
        }
    }

    /// SIMD-correctness test: packed vs scalar twin must be byte-equal.
    #[test]
    fn ema_six_packed_matches_scalar() {
        let s = wiggle();
        let streams = [s.as_slice()];
        // Erase once for both (same signature)
        let (_a, _sig, vals, _) = erase_request(&ema_six_req()).unwrap();
        // Call both generated functions directly
        let out_scalar = ema_six(&vals, &streams);
        let out_packed = ema_six_p(&vals, &streams);
        assert_eq!(out_scalar.len(), out_packed.len(), "same number of rows");
        for (_i, (r_s, r_p)) in out_scalar.iter().zip(out_packed.iter()).enumerate() {
            assert_eq!(
                r_s.as_slice(),
                r_p.as_slice(),
                "row {_i} mismatch between scalar and packed"
            );
        }
    }

    /// Pattern sanity: verify node count and packing grouping via macro expansion.
    #[test]
    fn ema_six_packed_shape() {
        // Node count derivation from arms (all distinct names → no EMA dedup):
        // Macd(12,24,9): es(ema_short), el(ema_long), d(sub), sig(ema(signal)), hist(sub) = 5
        // Apo(10,20): es(ema_short), el(ema_long), d(sub) = 3
        // Ppo(6,14): es(ema_short), el(ema_long), d(sub), c100(const), m(mul), q(div) = 6
        // + Input node for price stream 0 = 1
        // Total: 5+3+6+1 = 15 nodes
        // Ema slots with same-source (stream 0): Macd es/el, Apo es/el, Ppo es/el = 6 EMA slots
        // Grouping: 6 → [4,2] (greedy chunk)
        // Test: run_registered works and the generated code compiles
        let out =
            run_registered(&ema_six_req(), &[wiggle().as_slice()]).expect("ema_six registered");
        assert_eq!(out.fp, SIG_ema_six.hash());
        // ema_six_p should also work with same signature (same arena)
        let (_a, sig, vals, _) = erase_request(&ema_six_req()).unwrap();
        let out_p = ema_six_p(&vals, &[wiggle().as_slice()]);
        assert_eq!(out_p.len(), out.rows.len(), "packed has same output count");
        let _ = sig;
    }

    // ── rolling family (Sma / Stddev / BBands) ───────────────────────────

    use tulip_rs::indicators::bbands::BBands;
    use tulip_rs::indicators::sma::Sma;
    use tulip_rs::indicators::stddev::StdDev;

    fn sma_stddev_req(sd_opt: Option<&[bool]>) -> Request {
        Request::new()
            .add(Sma::recipe(&[14.0]), None)
            .add(StdDev::recipe(&[14.0]), sd_opt)
    }
    fn bbands_req(w_opt: Option<&[bool]>) -> Request {
        Request::new()
            .add(BBands::recipe(&[20.0, 2.0]), None)
            .add(Wma::recipe(&[20.0]), w_opt)
    }

    #[test]
    fn sma_stddev_sig_derived() {
        let (_a, sig, _, _) = erase_request(&sma_stddev_req(None)).unwrap();
        assert_eq!(
            sig, SIG_sma_stddev_shared_p,
            "SMA_STDDEV arena drifted from merge"
        );
        let (_a, sig, _, _) = erase_request(&bbands_req(None)).unwrap();
        assert_eq!(
            sig, SIG_bbands_wma_shared_p,
            "BBANDS_WMA arena drifted from merge"
        );
    }

    /// The dedup claim: `Stddev(14)`'s optional sma mirror and the standalone
    /// `Sma(14)` request are ONE arena node — with the flag on, the mirror
    /// row ALIASES the sma row's physical memory (one recurrence, two rows).
    #[test]
    fn sma_stddev_mirror_rows_alias() {
        let s = wiggle();
        let streams = [s.as_slice()];
        let out = run_registered(&sma_stddev_req(Some(&[true])), &streams)
            .expect("sma_stddev registered");
        // sinks: Sma primary [0]=sma node, Stddev primary [1]=sd node,
        // Stddev optional [2]=sma node again.
        assert_eq!(
            out.rows[0].as_slice(),
            out.rows[2].as_slice(),
            "mirror content"
        );
        assert!(
            out.rows[0].shares_data_with(&out.rows[2]),
            "mirror must alias the arena row, never copy"
        );
    }

    /// Disabled optional prints EMPTY (same contract as every other pattern).
    #[test]
    fn sma_stddev_optional_is_runtime() {
        let s = wiggle();
        let streams = [s.as_slice()];
        let off = run_registered(&sma_stddev_req(None), &streams).unwrap();
        assert!(off.rows[2].is_empty(), "disabled mirror prints empty row");
        let on = run_registered(&sma_stddev_req(Some(&[true])), &streams).unwrap();
        assert!(!on.rows[2].is_empty());
        assert_eq!(
            off.rows[0].as_slice(),
            on.rows[0].as_slice(),
            "primaries invariant to the flag"
        );
        assert_eq!(
            off.rows[1].as_slice(),
            on.rows[1].as_slice(),
            "primaries invariant to the flag"
        );
    }

    /// Bit-exact vs hand on the rolling pair, on wiggle / ramp / FLAT (the
    /// flat series drives the stddev ε-floor every bar; the ramp drives the
    /// window step's cancel-everything case).
    #[test]
    fn sma_stddev_bit_exact() {
        let data = [
            wiggle(),
            series(650, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0), // FLAT: var = 0 → floor
        ];
        for s in &data {
            let streams = [s.as_slice()];
            let out = run_registered(&sma_stddev_req(Some(&[true])), &streams)
                .expect("sma_stddev registered");
            assert_eq!(out.fp, SIG_sma_stddev_shared_p.hash());
            let inp = [s.as_slice()];
            let (sma, _) = Sma::indicator(&inp, &[14.0], None).unwrap();
            let (sd, _) = StdDev::indicator(&inp, &[14.0], Some(&[true])).unwrap();
            assert_eq!(out.rows[0].as_slice(), sma[0], "sma row");
            assert_eq!(out.rows[1].as_slice(), sd[0], "stddev row");
            assert_eq!(out.rows[2].as_slice(), sd[1], "stddev's sma mirror");
        }
    }

    /// Params stay runtime: different periods route to the SAME kernel.
    #[test]
    fn sma_stddev_same_kernel_different_params() {
        let s = wiggle();
        let streams = [s.as_slice()];
        for p in [9.0, 20.0] {
            let req = Request::new()
                .add(Sma::recipe(&[p]), None)
                .add(StdDev::recipe(&[p]), None);
            let out = run_registered(&req, &streams).expect("same shape, any p");
            assert_eq!(out.fp, SIG_sma_stddev_shared_p.hash());
            let inp = [s.as_slice()];
            let (sma, _) = Sma::indicator(&inp, &[p], None).unwrap();
            assert_eq!(out.rows[0].as_slice(), sma[0], "sma p={p} row");
        }
    }

    /// Sma(14)+Stddev(9) erases to a DIFFERENT arena (two chains, extra
    /// mirror node) — the registry must decline it, never mis-serve.
    #[test]
    fn sma_stddev_distinct_periods_falls_back() {
        let req = Request::new()
            .add(Sma::recipe(&[14.0]), None)
            .add(StdDev::recipe(&[9.0]), None);
        let s = wiggle();
        assert!(run_registered(&req, &[s.as_slice()]).is_none());
    }

    /// Bit-exact vs hand on the MERGED shape: all three bbands rows + the
    /// wma row, incl. FLAT (middle = 100, bands at the ε floor) — and a
    /// second (p, k) through the SAME kernel to prove the band multiplier
    /// `k` is a runtime const binding, not a baked number.
    #[test]
    fn bbands_bit_exact() {
        let data = [
            wiggle(),
            series(650, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ];
        for s in &data {
            let streams = [s.as_slice()];
            let out = run_registered(&bbands_req(None), &streams).expect("bbands_wma registered");
            assert_eq!(out.fp, SIG_bbands_wma_shared_p.hash());
            let inp = [s.as_slice()];
            let (bb, _) = BBands::indicator(&inp, &[20.0, 2.0], None).unwrap();
            let (wma, _) = Wma::indicator(&inp, &[20.0], None).unwrap();
            // sinks: Bbands primaries [lo, mid, up], Wma primary [w],
            // Wma optional [sma mirror — disabled here].
            assert_eq!(out.rows[0].as_slice(), bb[0], "lower band");
            assert_eq!(out.rows[1].as_slice(), bb[1], "middle band");
            assert_eq!(out.rows[2].as_slice(), bb[2], "upper band");
            assert_eq!(out.rows[3].as_slice(), wma[0], "wma row");
            assert!(out.rows[4].is_empty(), "disabled mirror prints empty");
        }
        // runtime k, same kernel
        let s = wiggle();
        let streams = [s.as_slice()];
        for k in [1.5, 3.0] {
            let req = Request::new()
                .add(BBands::recipe(&[20.0, k]), None)
                .add(Wma::recipe(&[20.0]), None);
            let out = run_registered(&req, &streams).expect("any k, same kernel");
            let inp = [s.as_slice()];
            let (bb, _) = BBands::indicator(&inp, &[20.0, k], None).unwrap();
            assert_eq!(out.rows[0].as_slice(), bb[0], "lower k={k}");
            assert_eq!(out.rows[2].as_slice(), bb[2], "upper k={k}");
        }
    }

    /// The three-sink fusion: with the WMA mirror flag ON, its sma row is
    /// the SAME arena line as the bbands middle (and the Bbands-middle ≡
    /// standalone-Sma check below) — one recurrence, three sinks, one pass.
    #[test]
    fn bbands_wma_sma_mirrors_alias_one_node() {
        let s = wiggle();
        let streams = [s.as_slice()];
        let out = run_registered(&bbands_req(Some(&[true])), &streams).unwrap();
        assert_eq!(
            out.rows[1].as_slice(),
            out.rows[4].as_slice(),
            "middle and wma mirror contents"
        );
        assert!(
            out.rows[1].shares_data_with(&out.rows[4]),
            "both sma sinks must alias ONE arena row"
        );
    }

    /// The §4-style identity for the rolling family: bbands' middle row IS
    /// the standalone `Sma(20)` output bit-for-bit (one `Sma` node across
    /// the two requests — the emitter stores it once).
    #[test]
    fn bbands_middle_matches_standalone_sma() {
        let s = wiggle();
        let streams = [s.as_slice()];
        let bb = run_registered(&bbands_req(None), &streams).unwrap();
        let sma_req = Request::new()
            .add(Sma::recipe(&[20.0]), None)
            .add(StdDev::recipe(&[20.0]), None);
        let fam = run_registered(&sma_req, &streams).unwrap();
        assert_eq!(
            bb.rows[1].as_slice(),
            fam.rows[0].as_slice(),
            "middle ≡ sma"
        );
    }

    // ── sma + wma (the honest MA pair) & the 6-SMA wide shape ───────────

    use tulip_rs::indicators::wma::Wma;

    fn sma_wma_req(w_opt: Option<&[bool]>) -> Request {
        Request::new()
            .add(Sma::recipe(&[14.0]), None)
            .add(Wma::recipe(&[14.0]), w_opt)
    }
    /// Six distinct periods, one column — the rolling analogue of ema_six.
    fn sma_six_req() -> Request {
        let mut r = Request::new();
        for p in [3.0, 7.0, 14.0, 20.0, 50.0, 100.0] {
            r = r.add(Sma::recipe(&[p]), None);
        }
        r
    }

    #[test]
    fn sma_wma_sig_derived() {
        let (_a, sig, _, _) = erase_request(&sma_wma_req(None)).unwrap();
        assert_eq!(
            sig, SIG_sma_wma_shared_p,
            "SMA_WMA arena drifted from merge"
        );
    }

    /// Bit-exact vs hand on the MA pair — wma's trick order (weighted drops
    /// the PRE-update sum) is the whole risk surface here; FLAT and ramp
    /// exercise it, and the optional sma mirror of the WMA row aliases the
    /// standalone Sma row (ONE arena node, two sinks).
    #[test]
    fn sma_wma_bit_exact() {
        let data = [
            wiggle(),
            series(650, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ];
        for s in &data {
            let streams = [s.as_slice()];
            let out =
                run_registered(&sma_wma_req(Some(&[true])), &streams).expect("sma_wma registered");
            assert_eq!(out.fp, SIG_sma_wma_shared_p.hash());
            let inp = [s.as_slice()];
            let (sma, _) = Sma::indicator(&inp, &[14.0], None).unwrap();
            let (wma, _) = Wma::indicator(&inp, &[14.0], Some(&[true])).unwrap();
            // sinks: Sma primary [0]=sma node, Wma primary [1]=wma node,
            // Wma optional [2]=sma node again.
            assert_eq!(out.rows[0].as_slice(), sma[0], "sma row");
            assert_eq!(out.rows[1].as_slice(), wma[0], "wma row");
            assert_eq!(out.rows[2].as_slice(), wma[1], "wma's sma mirror");
            assert_eq!(
                out.rows[0].as_slice(),
                out.rows[2].as_slice(),
                "cross-request mirror content"
            );
            assert!(
                out.rows[0].shares_data_with(&out.rows[2]),
                "the two sma rows must alias ONE arena line"
            );
        }
    }

    #[test]
    fn sma_wma_same_kernel_different_params() {
        let s = wiggle();
        let streams = [s.as_slice()];
        for p in [9.0, 20.0] {
            let req = Request::new()
                .add(Sma::recipe(&[p]), None)
                .add(Wma::recipe(&[p]), None);
            let out = run_registered(&req, &streams).expect("same shape, any p");
            assert_eq!(out.fp, SIG_sma_wma_shared_p.hash());
            let inp = [s.as_slice()];
            let (wma, _) = Wma::indicator(&inp, &[p], None).unwrap();
            assert_eq!(out.rows[1].as_slice(), wma[0], "wma p={p} row");
        }
    }

    #[test]
    fn sma_wma_distinct_periods_falls_back() {
        let req = Request::new()
            .add(Sma::recipe(&[14.0]), None)
            .add(Wma::recipe(&[9.0]), None);
        let s = wiggle();
        assert!(run_registered(&req, &[s.as_slice()]).is_none());
    }

    #[test]
    fn sma_six_sig_derived() {
        let (_a, sig, _, _) = erase_request(&sma_six_req()).unwrap();
        assert_eq!(sig, SIG_sma_six, "SMA_SIX arena drifted from merge");
    }

    /// All six rows bit-exact vs six standalone hand calls — the same-
    /// column/different-period shape where NOTHING dedups except price.
    #[test]
    fn sma_six_bit_exact() {
        let data = [
            wiggle(),
            series(650, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ];
        for s in &data {
            let streams = [s.as_slice()];
            let out = run_registered(&sma_six_req(), &streams).expect("sma_six registered");
            assert_eq!(out.fp, SIG_sma_six.hash());
            let inp = [s.as_slice()];
            for (row, p) in (0..6).zip([3.0, 7.0, 14.0, 20.0, 50.0, 100.0]) {
                let (sma, _) = Sma::indicator(&inp, &[p], None).unwrap();
                assert_eq!(out.rows[row].as_slice(), sma[0], "sma p={p} row {row}");
            }
        }
    }

    /// SIMD-correctness gate: the packed twin (Simd<f64,4> + Simd<f64,2>
    /// lane stepping) must be BYTE-EQUAL to the scalar kernel.
    #[test]
    fn sma_six_packed_matches_scalar() {
        let s = wiggle();
        let streams = [s.as_slice()];
        let (_a, _sig, vals, _) = erase_request(&sma_six_req()).unwrap();
        let out_scalar = sma_six(&vals, &streams);
        let out_packed = sma_six_p(&vals, &streams);
        assert_eq!(out_scalar.len(), out_packed.len(), "same number of rows");
        for (i, (r_s, r_p)) in out_scalar.iter().zip(out_packed.iter()).enumerate() {
            assert_eq!(
                r_s.as_slice(),
                r_p.as_slice(),
                "row {i} mismatch between scalar and packed"
            );
        }
    }

    // ── ema_chains (pure multi-options) & atr_multi (packed Wilders lanes) ──

    use tulip_rs::indicators::atr::Atr;
    use tulip_rs::indicators::ema::Ema;

    fn ema_chains_req() -> Request {
        let mut r = Request::new();
        for p in [12.0, 24.0, 10.0, 20.0, 6.0, 14.0] {
            r = r.add(Ema::recipe(&[p]), None);
        }
        r
    }

    fn atr_multi_req(p1: f64, p2: f64, p3: f64) -> Request {
        Request::new()
            .add(Atr::recipe(&[p1]), None)
            .add(Atr::recipe(&[p2]), None)
            .add(Atr::recipe(&[p3]), None)
    }

    #[test]
    fn ema_chains_sig_derived() {
        let (_a, sig, _, _) = erase_request(&ema_chains_req()).unwrap();
        assert_eq!(sig, SIG_ema_chains, "EMA_CHAINS arena drifted from merge");
        let (_a, sig, _, _) = erase_request(&atr_multi_req(9.0, 14.0, 21.0)).unwrap();
        assert_eq!(sig, SIG_atr_multi, "ATR_MULTI arena drifted from merge");
    }

    #[test]
    fn ema_chains_bit_exact() {
        let data = [
            wiggle(),
            series(650, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ];
        for s in &data {
            let streams = [s.as_slice()];
            let out = run_registered(&ema_chains_req(), &streams).expect("ema_chains registered");
            let inp = [s.as_slice()];
            for (row, p) in (0..6).zip([12.0, 24.0, 10.0, 20.0, 6.0, 14.0]) {
                let (ema, _) = Ema::indicator(&inp, &[p], None).unwrap();
                assert_eq!(out.rows[row].as_slice(), ema[0], "ema p={p} row {row}");
            }
        }
    }

    #[test]
    fn ema_chains_packed_matches_scalar() {
        let s = wiggle();
        let streams = [s.as_slice()];
        let (_a, _sig, vals, _) = erase_request(&ema_chains_req()).unwrap();
        let out_scalar = ema_chains(&vals, &streams);
        let out_packed = ema_chains_p(&vals, &streams);
        for (i, (r_s, r_p)) in out_scalar.iter().zip(out_packed.iter()).enumerate() {
            assert_eq!(r_s.as_slice(), r_p.as_slice(), "row {i}: packed != scalar");
        }
    }

    /// Three ATRs, one shared tr subgraph: sinks per request are [w, tr]
    /// (optional tr disabled by None ⇒ primaries at rows 0/2/4).
    #[test]
    fn atr_multi_bit_exact() {
        let base = [
            wiggle(),
            series(650, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ];
        for s in &base {
            let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&atr_multi_req(9.0, 14.0, 21.0), &streams)
                .expect("atr_multi registered");
            let inp = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            for (row, p) in (0..3).zip([9.0, 14.0, 21.0]) {
                let (atr, _) = Atr::indicator(&inp, &[p], None).unwrap();
                assert_eq!(
                    out.rows[row * 2].as_slice(),
                    atr[0],
                    "atr p={p} (distinct lanes, one shared tr)"
                );
            }
        }
    }

    #[test]
    fn atr_multi_packed_matches_scalar() {
        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
        let (_a, _sig, vals, _) = erase_request(&atr_multi_req(9.0, 14.0, 21.0)).unwrap();
        let out_scalar = atr_multi(&vals, &streams);
        let out_packed = atr_multi_p(&vals, &streams);
        for (i, (r_s, r_p)) in out_scalar.iter().zip(out_packed.iter()).enumerate() {
            assert_eq!(
                r_s.as_slice(),
                r_p.as_slice(),
                "row {i}: packed Wilders lanes != scalar"
            );
        }
    }

    /// Equal RUNTIME periods collapse two Wilders chains into one node —
    /// the macro arena (distinct sentinels) differs → the registry must
    /// DECLINE, never mis-serve (tier-2 handles it).
    #[test]
    fn atr_multi_shared_period_falls_back() {
        let s = wiggle();
        let streams = [s.as_slice(), s.as_slice(), s.as_slice()];
        assert!(
            run_registered(&atr_multi_req(14.0, 14.0, 21.0), &streams).is_none(),
            "p1 == p2 erases to a smaller arena — must not hit the multi pattern"
        );
    }

    // ── the trail-rescan hub ────────────────────────────────────────────

    use tulip_rs::indicators::aroon::Aroon;
    use tulip_rs::indicators::aroonosc::AroonOsc;
    use tulip_rs::indicators::donchianchannel::DonchianChannel;
    use tulip_rs::indicators::ichimoku::Ichimoku;
    use tulip_rs::indicators::max::Max;
    use tulip_rs::indicators::min::Min;
    use tulip_rs::indicators::willr::Willr;

    /// Tie/plateau torture series — the rescan helpers' `>=`/`<` rules
    /// (rightmost occurrence) are the drift risk these shapes expose.
    fn plateaus() -> Vec<f64> {
        (0..700).map(|i| 100.0 + ((i % 9) / 2) as f64).collect()
    }

    fn max_min_req() -> Request {
        Request::new()
            .add(Max::recipe(&[14.0]), None)
            .add(Min::recipe(&[14.0]), None)
    }
    fn aroon_osc_req(o_opt: Option<&[bool]>) -> Request {
        Request::new()
            .add(Aroon::recipe(&[14.0]), None)
            .add(AroonOsc::recipe(&[14.0]), o_opt)
    }
    fn willr_donch_req(w_opt: Option<&[bool]>) -> Request {
        Request::new()
            .add(Willr::recipe(&[14.0]), w_opt)
            .add(DonchianChannel::recipe(&[14.0]), None)
    }
    fn ichimoku_req() -> Request {
        Request::new().add(Ichimoku::recipe(&[9.0, 26.0]), None)
    }

    #[test]
    fn hub_sigs_derived() {
        let (_a, sig, _, _) = erase_request(&max_min_req()).unwrap();
        assert_eq!(sig, SIG_max_min_shared_p, "MAX_MIN drift");
        let (_a, sig, _, _) = erase_request(&aroon_osc_req(None)).unwrap();
        assert_eq!(sig, SIG_aroon_osc_shared_p, "AROON_OSC drift");
        let (_a, sig, _, _) = erase_request(&willr_donch_req(None)).unwrap();
        assert_eq!(sig, SIG_willr_donchian_shared_p, "WILLR_DONCH drift");
        let (_a, sig, _, _) = erase_request(&ichimoku_req()).unwrap();
        assert_eq!(sig, SIG_ichimoku_s_m, "ICHIMOKU drift");
    }

    #[test]
    fn max_min_bit_exact() {
        let data = [
            wiggle(),
            series(650, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
            plateaus(), // ties everywhere: the trail rule's last-position contract
        ];
        for s in &data {
            let streams = [s.as_slice()];
            let out = run_registered(&max_min_req(), &streams).expect("max_min registered");
            let inp = [s.as_slice()];
            let (mx, _) = Max::indicator(&inp, &[14.0], None).unwrap();
            let (mn, _) = Min::indicator(&inp, &[14.0], None).unwrap();
            assert_eq!(out.rows[0].as_slice(), mx[0], "max row");
            assert_eq!(out.rows[1].as_slice(), mn[0], "min row");
        }
    }

    /// The aroon showcase: five sinks, ONE interned roll/trail/scaled-line
    /// graph; the osc's mirror rows ALIAS the aroon primaries. Plus the
    /// AROON WINDOW RULE pin against `Max(15)`: `Aroon(9)`-style windows are
    /// `(p, p)` = `p + 1` bars = `Max(p+1)`'s `(p+1, p)` — so on every bar
    /// where aroon's trail is 0 (the window max sits at the CURRENT bar),
    /// the independent `Max(15)` series must hold exactly `high[bar]`.
    #[test]
    fn aroon_osc_bit_exact() {
        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        let streams = [hi.as_slice(), lo.as_slice()];
        let inp = [hi.as_slice(), lo.as_slice()];
        let out =
            run_registered(&aroon_osc_req(Some(&[true, true])), &streams).expect("aroon_osc hit");
        let (ar, _) = Aroon::indicator(&inp, &[14.0], None).unwrap();
        let (ao, _) = AroonOsc::indicator(&inp, &[14.0], Some(&[true, true])).unwrap();
        // sinks: Aroon [down, up], AroonOsc [osc, down-mirror, up-mirror].
        assert_eq!(out.rows[0].as_slice(), ar[0], "aroon_down");
        assert_eq!(out.rows[1].as_slice(), ar[1], "aroon_up");
        assert_eq!(out.rows[2].as_slice(), ao[0], "aroonosc");
        assert_eq!(out.rows[3].as_slice(), ao[1], "osc mirror down");
        assert_eq!(out.rows[4].as_slice(), ao[2], "osc mirror up");
        assert!(
            out.rows[0].shares_data_with(&out.rows[3])
                && out.rows[1].shares_data_with(&out.rows[4]),
            "mirror rows must alias the interned aroon lines"
        );
        // Window rule vs the independent Max(15) pattern.
        let (mx15, _) = Max::indicator(&[hi.as_slice()], &[15.0], None).unwrap();
        let mult = 100.0 / 14.0;
        let mut fresh_max_bars = 0;
        for (k, &up) in out.rows[1].iter().enumerate() {
            let trail = 14.0 - up / mult;
            if trail.abs() < 1e-9 {
                // aroon's window max sits AT bar 14 + k ⇒ max(15) (same
                // 15-bar window!) must equal the current bar exactly.
                let bar = 14 + k;
                assert_eq!(
                    mx15[0][k].to_bits(),
                    hi[bar].to_bits(),
                    "Aroon(14)-window ≡ Max(15)-window at bar {bar}"
                );
                fresh_max_bars += 1;
            }
        }
        assert!(
            fresh_max_bars > 20,
            "plateau coverage, got {fresh_max_bars}"
        );
    }

    #[test]
    fn willr_donch_bit_exact() {
        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
        // willr optionals ON: their roll nodes ARE donchian's outputs.
        let out = run_registered(&willr_donch_req(Some(&[true, true])), &streams)
            .expect("willr_donch registered");
        let inp3 = [hi.as_slice(), lo.as_slice(), s.as_slice()];
        let inp2 = [hi.as_slice(), lo.as_slice()];
        let (wr, _) = Willr::indicator(&inp3, &[14.0], Some(&[true, true])).unwrap();
        let (dc, _) = DonchianChannel::indicator(&inp2, &[14.0], None).unwrap();
        // sinks: Willr [wr, mn, mx]; Donchian [mn, mid, mx].
        assert_eq!(out.rows[0].as_slice(), wr[0], "willr row");
        assert_eq!(out.rows[3].as_slice(), dc[0], "donchian lower");
        assert_eq!(out.rows[4].as_slice(), dc[1], "donchian middle");
        assert_eq!(out.rows[5].as_slice(), dc[2], "donchian upper");
        assert_eq!(out.rows[1].as_slice(), wr[1], "willr min mirror");
        assert_eq!(out.rows[2].as_slice(), wr[2], "willr max mirror");
        assert!(
            out.rows[1].shares_data_with(&out.rows[3])
                && out.rows[2].shares_data_with(&out.rows[5]),
            "the canonical (p, p−1) roll pair is ONE node per column across requests"
        );
    }

    #[test]
    fn ichimoku_bit_exact() {
        let s = wiggle();
        for data in [s, plateaus()] {
            let hi: Vec<f64> = data.iter().map(|v| v + 1.0).collect();
            let lo: Vec<f64> = data.iter().map(|v| v - 0.5).collect();
            let streams = [hi.as_slice(), lo.as_slice()];
            let out = run_registered(&ichimoku_req(), &streams).expect("ichimoku registered");
            let inp = [hi.as_slice(), lo.as_slice(), data.as_slice()];
            let (ichi, _) = Ichimoku::indicator(&inp, &[9.0, 26.0], None).unwrap();
            for (row, name) in [
                (0usize, "conversion"),
                (1, "base"),
                (2, "span_a"),
                (3, "span_b"),
            ] {
                assert_eq!(
                    out.rows[row].as_slice(),
                    ichi[row].as_slice(),
                    "ichimoku {name}"
                );
            }
        }
    }

    /// Aroon(9) + AroonOsc(14) binds two DIFFERENT windows where the
    /// pattern interns one graph — a different arena; must fall back.
    #[test]
    fn hub_distinct_periods_fall_back() {
        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        let streams = [hi.as_slice(), lo.as_slice()];
        let req = Request::new()
            .add(Aroon::recipe(&[9.0]), None)
            .add(AroonOsc::recipe(&[14.0]), None);
        assert!(run_registered(&req, &streams).is_none());
    }

    // ── ring-backed derived rolls (ao) ────────────────────────────────────

    use tulip_rs::indicators::ao::Ao;

    /// The arena `ao_s_m` erases to: one ring group, explicit primary lead
    /// 34 riding `Values`, and the rev-2 law for windows — a different
    /// (short, long) pair hits the SAME kernel, with the lead following the
    /// long window.
    #[test]
    fn ao_sig_derived() {
        let req = Request::new().add(Ao::recipe(&[]), None);
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_ao_s_m, "AO arena drifted from merge");
        assert_eq!(vals.sink_lead[0], 34, "primary lead (hand min_data 35)");

        let req2 = Request::new().add(Ao::recipe(&[7.0, 21.0]), None);
        let (_a, sig2, vals2, _) = erase_request(&req2).unwrap();
        assert_eq!(
            sig2, SIG_ao_s_m,
            "period change must not change the signature"
        );
        assert_eq!(vals2.sink_lead[0], 21, "lead follows the LONG window");
    }

    /// Bit-exact vs `Ao::indicator` on three regimes plus a series barely
    /// past the gate (n=40: warm covers bars 0..34, five steady bars).
    /// PRIMARY-ONLY: the optional mirrors' row heads are one bar off from
    /// the hand's init-stored zeros (engine long starts at defined-from
    /// 33, hand's LONG row at 34; short likewise) — that reconciliation is
    /// a step-C item, not papered over here.
    #[test]
    fn ao_bit_exact() {
        let data = [
            wiggle(),
            series(650, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
            series(40, |i| 90.0 + (i as f64 * 0.7).sin() * 4.0),
        ];
        for s in &data {
            let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
            let streams = [hi.as_slice(), lo.as_slice()];
            let out = run_registered(&Request::new().add(Ao::recipe(&[]), None), &streams)
                .expect("ao template hit");
            let inp = [hi.as_slice(), lo.as_slice()];
            let (ao, _) = Ao::indicator(&inp, &[], None).unwrap();
            assert_eq!(out.rows[0].as_slice(), ao[0], "ao row (n={})", s.len());
        }
    }

    // ── the bespoke HMA machine ─────────────────────────────────────────────

    use tulip_rs::indicators::hma::Hma;

    /// Params are values: p=9 and p=20 are the SAME kernel; publish lead
    /// `p + trunc(√p)` rides Values (20 → trunc 4.47 = 4 → lead 24).
    #[test]
    fn hma_sig_derived() {
        let req = Request::new().add(Hma::recipe(&[9.0]), None);
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_hma_n, "HMA arena drifted from merge");
        assert_eq!(vals.sink_lead[0], 9 + 3, "lead p + trunc sqrt(p)");
        let req2 = Request::new().add(Hma::recipe(&[20.0]), None);
        let (_a, sig2, vals2, _) = erase_request(&req2).unwrap();
        assert_eq!(sig2, SIG_hma_n, "period change keeps the kernel");
        assert_eq!(vals2.sink_lead[0], 20 + 4);
        // Non-square p exercises truncation: √14 → 3 → lead 17.
        let req3 = Request::new().add(Hma::recipe(&[14.0]), None);
        let (_a, sig3, vals3, _o3) = erase_request(&req3).unwrap();
        assert_eq!(sig3, SIG_hma_n);
        assert_eq!(vals3.sink_lead[0], 14 + 3);
    }

    /// Bit-exact vs `Hma::indicator` across square, non-square and large
    /// periods, on the usual regimes plus a series barely past the lead.
    #[test]
    fn hma_bit_exact() {
        for p in [2.0f64, 9.0, 14.0, 20.0, 48.0] {
            let period = p as usize;
            let m = p.sqrt() as usize;
            let lead = period + m;
            let data = [
                wiggle(),
                series(650, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
                series(lead + 3, |i| 90.0 + (i as f64 * 0.7).sin() * 4.0),
            ];
            for s in &data {
                let streams = [s.as_slice()];
                let out = run_registered(&Request::new().add(Hma::recipe(&[p]), None), &streams)
                    .expect("hma template hit");
                let inp = [s.as_slice()];
                let (hma, _) = Hma::indicator(&inp, &[p], None).unwrap();
                assert_eq!(
                    out.rows[0].as_slice(),
                    hma[0].as_slice(),
                    "hma row p={p} n={}",
                    s.len()
                );
            }
        }
    }

    // ── fisher: the IIR chain over the extremum ring ──────────────────────

    use tulip_rs::indicators::fisher::Fisher;

    /// PARITY LAW for fisher: 1e-12 RELATIVE vs the hand (the `Ln` host-call
    /// exception — Handover §6.41: `Node::Ln` calls the same libm as the
    /// hand's `ln_arg.ln()`, so the drift is the clamp's mul_add association
    /// budget, not transcendental); every OTHER pattern stays bit-exact.
    fn assert_rel_1e12(got: &[f64], want: &[f64], tag: &str) {
        assert_eq!(got.len(), want.len(), "{tag}: row length");
        for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
            let scale = g.abs().max(w.abs()).max(1.0);
            assert!(
                (g - w).abs() <= 1e-12 * scale,
                "{tag} idx {i}: engine {g:e} vs hand {w:e}"
            );
        }
    }

    #[test]
    fn fisher_sig_derived() {
        let req = Request::new().add(Fisher::recipe(&[9.0]), None);
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_fisher_n, "fisher arena drifted from merge");
        assert_eq!(
            vals.sink_lead[0], 8,
            "fish lead = period−1 (hand row[0] bar)"
        );
        assert_eq!(vals.sink_lead[1], 8, "signal shares the chain's lead");

        let req2 = Request::new().add(Fisher::recipe(&[14.0]), None);
        let (_a2, sig2, vals2, _) = erase_request(&req2).unwrap();
        assert_eq!(
            sig2, SIG_fisher_n,
            "period change must not change the signature"
        );
        assert_eq!(vals2.sink_lead[0], 13);
    }

    /// The hand pin: fisher AND signal rows, 1e-12 relative, across periods
    /// (incl. p=4 barely-past-gate shapes) and the usual regimes: wiggle,
    /// ramp, FLAT (min==max exercises the 1e-3 guard floor + clamp saturation
    /// path), and a short sine crossing the clamp thresholds.
    #[test]
    fn fisher_parity_vs_hand() {
        for p in [4.0f64, 9.0, 14.0, 28.0] {
            let period = p as usize;
            let lead = period - 1;
            let data = [
                wiggle(),
                series(650, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
                series(lead + 40, |i| 90.0 + (i as f64 * 0.9).sin() * 6.0),
            ];
            for s in &data {
                let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
                let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
                let streams = [hi.as_slice(), lo.as_slice()];
                let out = run_registered(&Request::new().add(Fisher::recipe(&[p]), None), &streams)
                    .expect("fisher template hit");
                let inp = [hi.as_slice(), lo.as_slice()];
                let (fisher, _) = Fisher::indicator(&inp, &[p], None).unwrap();
                assert_rel_1e12(
                    &out.rows[0],
                    &fisher[0],
                    &format!("fish p={p} n={}", s.len()),
                );
                assert_rel_1e12(
                    &out.rows[1],
                    &fisher[1],
                    &format!("signal p={p} n={}", s.len()),
                );
            }
        }
    }

    // ── stoch: the skip/steady_from pair (ring RollSums with init-form
    // bars — the shape that unlocked the two Values arrays) ───────────────

    use tulip_rs::cse::signature::K_ROLLSUM;
    use tulip_rs::indicators::stoch::Stoch;

    /// Bit-level row equality (NaN payloads included — the epsilon-floor
    /// regime can propagate one, and the spec is WYSIWYG transcription).
    fn assert_bits(got: &[f64], want: &[f64], tag: &str) {
        assert_eq!(got.len(), want.len(), "{tag}: row length");
        for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
            assert_eq!(g.to_bits(), w.to_bits(), "{tag}: bit mismatch at {i}");
        }
    }

    /// `stoch_kd` erases to: TWO ring groups (slow %K over the division
    /// column, %D over the %K line), each single-member (cap = its own
    /// period); the skip/steady_from law on Values — %K carries skip 2 and
    /// steady_from = k+ks+d, %D is plain. Sink leads: %K natural k+ks, %D
    /// pinned k+ks+d; a param change must not move the signature.
    #[test]
    fn stoch_sig_derived() {
        let req = Request::new().add(Stoch::recipe(&[14.0, 3.0, 3.0]), None);
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_stoch_kd, "stoch arena drifted from merge");
        assert_eq!(vals.sink_lead[0], 17, "%K lead = k+ks (hand row[0] bar)");
        assert_eq!(vals.sink_lead[1], 20, "%D lead = k+ks+d (pinned)");

        let rolls: Vec<usize> = (0..MAX).filter(|i| sig.kinds[*i] == K_ROLLSUM).collect();
        assert_eq!(rolls.len(), 2, "two ring rolls");
        let (k_line, d_line) = (rolls[0], rolls[1]);
        assert_eq!(vals.skip[k_line], 2, "slow %K discards fast's head pair");
        assert_eq!(vals.steady_from[k_line], 20, "init-form until %D starts");
        assert_eq!(vals.skip[d_line], 0);
        assert_eq!(vals.steady_from[d_line], 0);
        assert_eq!(vals.ring_of[k_line] as usize, k_line, "self-leader group");
        assert_eq!(vals.ring_cap[k_line], 3);
        assert_eq!(vals.ring_of[d_line] as usize, d_line);
        assert_eq!(vals.ring_cap[d_line], 3);

        let req2 = Request::new().add(Stoch::recipe(&[9.0, 5.0, 4.0]), None);
        let (_a2, sig2, vals2, _) = erase_request(&req2).unwrap();
        assert_eq!(
            sig2, SIG_stoch_kd,
            "period change must not change the signature"
        );
        assert_eq!(vals2.sink_lead[0], 14, "k+ks");
        assert_eq!(vals2.sink_lead[1], 18, "k+ks+d");
    }

    /// Bit-exact vs the hand across four option sets and five regimes —
    /// no `ln`, so FULL bit equality is the standard. The SPIKE series puts
    /// `Max(dif, ε)` on its floor with a non-zero numerator (fast %K
    /// explodes past f64::MAX; the inf/NaN arithmetic must transcribe
    /// statement-for-statement, including the init-form bars where the
    /// hand's eviction subtracts as its own statement).
    #[test]
    fn stoch_bit_exact() {
        for opts in [
            [14.0f64, 3.0, 3.0],
            [9.0, 3.0, 3.0],
            [5.0, 2.0, 3.0],
            [28.0, 4.0, 4.0],
        ] {
            let kp = opts[0] as usize;
            let lead = kp + opts[1] as usize + opts[2] as usize;
            let spike = series(lead + 60, |i| {
                if i % 17 == 0 && i >= kp + 2 {
                    101.0
                } else {
                    100.0
                }
            });
            let data = [
                wiggle(),
                series(650, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
                series(lead + 40, |i| 90.0 + (i as f64 * 0.9).sin() * 6.0),
                spike,
            ];
            for s in &data {
                let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
                let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
                let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
                let out = run_registered(&Request::new().add(Stoch::recipe(&opts), None), &streams)
                    .expect("stoch template hit");
                let inp = [hi.as_slice(), lo.as_slice(), s.as_slice()];
                let (stoch, _) = Stoch::indicator(&inp, &opts, None).unwrap();
                assert_bits(
                    &out.rows[0],
                    &stoch[0],
                    &format!("%K {:?} n={}", opts, s.len()),
                );
                assert_bits(
                    &out.rows[1],
                    &stoch[1],
                    &format!("%D {:?} n={}", opts, s.len()),
                );
            }
        }
    }

    /// The flagship triple: stoch's (p, p−1) high/low rolls ARE willr's and
    /// donchian's — TWO interned roll nodes serve eight output rows.
    #[test]
    fn stoch_willr_donchian_shared_sig() {
        let req = Request::new()
            .add(Stoch::recipe(&[14.0, 3.0, 3.0]), None)
            .add(Willr::recipe(&[14.0]), None)
            .add(DonchianChannel::recipe(&[14.0]), None);
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_stoch_willr_donchian_shared_p);
        let rolls: Vec<usize> = (0..MAX).filter(|i| sig.kinds[*i] == K_ROLLSUM).collect();
        assert_eq!(rolls.len(), 2, "only stoch's two mean rolls");
        assert_eq!(vals.skip[rolls[0]], 2);
    }

    /// Bit-exact vs the HAND across the triple's full sink layout: stoch
    /// [%K, %D], willr primary + two ASKED optional roll mirrors, donchian
    /// [lower, mid, upper]. The mirror rows ALIAS donchian's primaries —
    /// proof the (p, p−1) pair interned ACROSS all three requests.
    #[test]
    fn stoch_willr_donchian_bit_exact() {
        let s = wiggle();
        let hi: Vec<f64> = s.iter().map(|v| v + 1.0).collect();
        let lo: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
        let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
        let req = Request::new()
            .add(Stoch::recipe(&[14.0, 3.0, 3.0]), None)
            .add(Willr::recipe(&[14.0]), Some(&[true, true]))
            .add(DonchianChannel::recipe(&[14.0]), None);
        let out = run_registered(&req, &streams).expect("triple registered");
        let inp3 = [hi.as_slice(), lo.as_slice(), s.as_slice()];
        let inp2 = [hi.as_slice(), lo.as_slice()];
        let (stoch, _) = Stoch::indicator(&inp3, &[14.0, 3.0, 3.0], None).unwrap();
        let (wr, _) = Willr::indicator(&inp3, &[14.0], Some(&[true, true])).unwrap();
        let (dc, _) = DonchianChannel::indicator(&inp2, &[14.0], None).unwrap();
        assert_bits(&out.rows[0], &stoch[0], "triple %K");
        assert_bits(&out.rows[1], &stoch[1], "triple %D");
        assert_bits(&out.rows[2], &wr[0], "triple willr");
        assert_bits(&out.rows[5], &dc[0], "triple donch lower");
        assert_bits(&out.rows[6], &dc[1], "triple donch mid");
        assert_bits(&out.rows[7], &dc[2], "triple donch upper");
        assert_bits(&out.rows[3], &wr[1], "willr min mirror");
        assert_bits(&out.rows[4], &wr[2], "willr max mirror");
        assert!(
            out.rows[3].shares_data_with(&out.rows[5])
                && out.rows[4].shares_data_with(&out.rows[7]),
            "the (p, p−1) roll pair is ONE node per column across THREE requests"
        );
    }

    // ── rsi: mirror node + dual mean-Wilders chains ───────────────────────

    use tulip_rs::cse::signature::K_MIRROR;
    use tulip_rs::indicators::rsi::Rsi;

    /// `rsi_n` erases to: the pinned sink lead `period + 1` (hand
    /// `min_data = period + 2`), one `K_MIRROR` arena node (the `prev_real`
    /// state copy — computed, never stored), and the period law: a param
    /// change moves `Values`, not the signature.
    #[test]
    fn rsi_sig_derived() {
        let req = Request::new().add(Rsi::recipe(&[14.0]), None);
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_rsi_n, "rsi arena drifted from merge");
        assert_eq!(vals.sink_lead[0], 15, "hand row starts bar p+1");
        let mirrors: Vec<usize> = (0..MAX).filter(|i| sig.kinds[*i] == K_MIRROR).collect();
        assert_eq!(mirrors.len(), 1, "prev_real state mirror in arena");

        let req2 = Request::new().add(Rsi::recipe(&[9.0]), None);
        let (_a2, sig2, vals2, _) = erase_request(&req2).unwrap();
        assert_eq!(
            sig2, SIG_rsi_n,
            "period change must not change the signature"
        );
        assert_eq!(vals2.sink_lead[0], 10);
    }

    /// Bit-exact vs the hand across four periods and five regimes — no `ln`
    /// ⇒ FULL bit equality, NaN payloads included. FLAT drives the ratio to
    /// `0/0` (default NaN both engines, carried forever by the Wilders
    /// chains); the SPIKE regime puts `+∞` through `up−diff`-style movement
    /// where the hand's two-`Max` arms pin `+0.0` — the rewrite that would
    /// NaN-poison is NOT in the graph, so both stay finite here.
    #[test]
    fn rsi_bit_exact() {
        for p in [2.0f64, 7.0, 14.0, 21.0] {
            let lead = p as usize + 1;
            let data = [
                wiggle(),
                series(650, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
                series(lead + 40, |i| 90.0 + (i as f64 * 0.9).sin() * 6.0),
                series(lead + 60, |i| {
                    if i % 17 == 0 && i > lead {
                        101.0
                    } else {
                        100.0
                    }
                }),
            ];
            for s in &data {
                let streams = [s.as_slice()];
                let out = run_registered(&Request::new().add(Rsi::recipe(&[p]), None), &streams)
                    .expect("rsi template hit");
                let (rsi, _) = Rsi::indicator(&streams, &[p], None).unwrap();
                assert_bits(&out.rows[0], &rsi[0], &format!("rsi p={p} n={}", s.len()));
            }
        }
    }

    // ── stochrsi: the grafted-chain flagship ──────────────────────────────

    use tulip_rs::cse::signature::{K_ROLLMAX, K_ROLLMIN};
    use tulip_rs::indicators::stochrsi::StochRsi;

    /// The pair `Rsi(p) + Stochrsi(p)`: stochrsi's leaf grafts the rsi leaf,
    /// so the ENTIRE movement chain (Lag, both Subs, both Max gates, both
    /// Wilders chains, Add/Div/Mul) interns with the standalone rsi; the
    /// hand's `prev_real` Mirror rides the arena ONCE (both recipes declare
    /// the same `Mirror{real}` node). Leads: rsi `p+1`, stochrsi `2p`,
    /// optional rsi `p+1`. The two rolls over the derived rsi line are ONE
    /// ring group (cap `p`, two members).
    #[test]
    fn rsi_stochrsi_shared_sig() {
        let req = Request::new()
            .add(Rsi::recipe(&[14.0]), None)
            .add(StochRsi::recipe(&[14.0]), Some(&[true]));
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        assert_eq!(
            sig, SIG_rsi_stochrsi_shared_p,
            "rsi+stochrsi arena drifted from merge"
        );
        assert_eq!(vals.sink_lead[0], 15, "rsi lead p+1");
        assert_eq!(vals.sink_lead[1], 28, "stochrsi lead 2p");
        assert_eq!(vals.sink_lead[2], 15, "optional rsi mirror lead");
        let mirrors: Vec<usize> = (0..MAX).filter(|i| sig.kinds[*i] == K_MIRROR).collect();
        assert_eq!(mirrors.len(), 1, "one interned prev_real");
        let extrema: Vec<usize> = (0..MAX)
            .filter(|i| sig.kinds[*i] == K_ROLLMAX || sig.kinds[*i] == K_ROLLMIN)
            .collect();
        assert_eq!(extrema.len(), 2, "stochrsi's max/min over the rsi line");
        assert_eq!(
            vals.ring_of[extrema[0]] as usize, vals.ring_of[extrema[1]] as usize,
            "the pair shares ONE ring (same derived src)"
        );
        assert_eq!(vals.ring_cap[vals.ring_of[extrema[0]] as usize], 14);
    }

    /// Bit-exact vs BOTH hands across periods and the five regimes — FLAT
    /// matters most here: the rsi line goes `0/0 = NaN`, `kdif = max − min`
    /// is NaN, the `WhereLt` compare is false in BOTH directions so the
    /// guard falls through to the division and the hand PRINTS NaN — the
    /// engine must carry the same default-NaN bits through, not sanitize
    /// them. The optional rsi row must equal the standalone rsi row (same
    /// stored stream), and the alias check proves the arena interned them.
    #[test]
    fn rsi_stochrsi_bit_exact() {
        for p in [5.0f64, 14.0] {
            let lead = (p as usize) * 2;
            let data = [
                wiggle(),
                series(650, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
                series(lead + 40, |i| 90.0 + (i as f64 * 0.9).sin() * 6.0),
                series(lead + 60, |i| {
                    if i % 17 == 0 && i > lead {
                        101.0
                    } else {
                        100.0
                    }
                }),
            ];
            for s in &data {
                let streams = [s.as_slice()];
                let req = Request::new()
                    .add(Rsi::recipe(&[p]), None)
                    .add(StochRsi::recipe(&[p]), Some(&[true]));
                let out = run_registered(&req, &streams).expect("rsi_stochrsi template hit");
                let (rsi, _) = Rsi::indicator(&streams, &[p], None).unwrap();
                let (sr, _) = StochRsi::indicator(&streams, &[p], Some(&[true])).unwrap();
                assert_bits(
                    &out.rows[0],
                    &rsi[0],
                    &format!("pair rsi p={p} n={}", s.len()),
                );
                assert_bits(
                    &out.rows[1],
                    &sr[0],
                    &format!("pair stochrsi p={p} n={}", s.len()),
                );
                assert_bits(
                    &out.rows[2],
                    &sr[1],
                    &format!("pair rsi-mirror p={p} n={}", s.len()),
                );
                assert!(
                    out.rows[0].shares_data_with(&out.rows[2]),
                    "standalone rsi row and the optional mirror are ONE stream"
                );
            }
        }
    }

    // ── cmo: twin ring sums + the rsi+cmo movement-sharing pair ───────────

    use tulip_rs::cse::signature::{K_LAG, K_MAX};
    use tulip_rs::indicators::cmo::Cmo;

    /// `cmo_n` erases to: sink lead `period + 1` (hand prints from bar
    /// `p + 1`, `min_data = p + 2`), TWO ring groups (gain and loss sums —
    /// distinct derived srcs, each single-member, cap `p`, raw sums so
    /// `mults = 1.0`), TWO `K_LAG` nodes (`Lag{1}` for the movement,
    /// `Lag{p}` feeding `drop_real`), and TWO `K_MIRROR` state copies
    /// (`prev` on the close — the SAME node rsi carries — and `drop_real`).
    #[test]
    fn cmo_sig_derived() {
        let req = Request::new().add(Cmo::recipe(&[14.0]), None);
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_cmo_n, "cmo arena drifted from merge");
        assert_eq!(vals.sink_lead[0], 15, "hand row starts bar p+1");
        let rolls: Vec<usize> = (0..MAX).filter(|i| sig.kinds[*i] == K_ROLLSUM).collect();
        assert_eq!(rolls.len(), 2, "gain + loss sums on derived columns");
        for r in &rolls {
            assert_eq!(
                vals.ring_of[*r] as usize, *r,
                "own group — up and down are distinct srcs"
            );
            assert_eq!(vals.ring_cap[*r], 14);
        }
        let mirrors: Vec<usize> = (0..MAX).filter(|i| sig.kinds[*i] == K_MIRROR).collect();
        assert_eq!(mirrors.len(), 2, "prev + drop_real");
        let lags: Vec<usize> = (0..MAX).filter(|i| sig.kinds[*i] == K_LAG).collect();
        assert_eq!(lags.len(), 2, "Lag{{1}} + Lag{{p}}");
    }

    /// Bit-exact vs the hand across three periods and five regimes — no
    /// `ln` ⇒ FULL bit equality. RAMP runs `down_sum == 0` (the division
    /// lands the line on exactly 100.0), FLAT is the `0/0` NaN the ring sums
    /// carry forever, the SPIKE series pumps finite ±1 movement through the
    /// eviction path (`st += x − leave`, mults 1.0 — sum, not mean).
    #[test]
    fn cmo_bit_exact() {
        for p in [9.0f64, 14.0, 21.0] {
            let lead = p as usize + 1;
            let data = [
                wiggle(),
                series(650, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
                series(lead + 40, |i| 90.0 + (i as f64 * 0.9).sin() * 6.0),
                series(lead + 60, |i| {
                    if i % 17 == 0 && i > lead {
                        101.0
                    } else {
                        100.0
                    }
                }),
            ];
            for s in &data {
                let streams = [s.as_slice()];
                let out = run_registered(&Request::new().add(Cmo::recipe(&[p]), None), &streams)
                    .expect("cmo template hit");
                let (cmo, _) = Cmo::indicator(&streams, &[p], None).unwrap();
                assert_bits(&out.rows[0], &cmo[0], &format!("cmo p={p} n={}", s.len()));
            }
        }
    }

    /// The movement-sharing pair: Rsi + Cmo co-requested intern the WHOLE
    /// `up_down` stage — `Lag{1}`, both Sub arms, the zero Const, both Max
    /// gates, and the shared `Mirror{real}` state copy. The slot census IS
    /// the canary: a non-interning arena would show 4 Max, 3 Lag, 3 Mirror.
    #[test]
    fn rsi_cmo_shared_pair() {
        let req = Request::new()
            .add(Rsi::recipe(&[14.0]), None)
            .add(Cmo::recipe(&[14.0]), None);
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_rsi_cmo_shared_p, "rsi+cmo arena drifted");
        assert_eq!(vals.sink_lead[0], 15, "rsi lead");
        assert_eq!(vals.sink_lead[1], 15, "cmo lead");
        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        assert_eq!(count(K_MAX), 2, "up/down intern across recipes");
        assert_eq!(count(K_LAG), 2, "Lag{{1}} shared; Lag{{p}} cmo-only");
        assert_eq!(count(K_MIRROR), 2, "shared prev + drop_real");
        assert_eq!(count(K_ROLLSUM), 2, "cmo's twin ring sums");

        for s in [
            wiggle(),
            series(680, |_| 100.0),
            series(100, |i| 90.0 + (i as f64 * 0.9).sin() * 6.0),
        ] {
            let streams = [s.as_slice()];
            let out = run_registered(&req, &streams).expect("rsi+cmo registered");
            let (rsi, _) = Rsi::indicator(&streams, &[14.0], None).unwrap();
            let (cmo, _) = Cmo::indicator(&streams, &[14.0], None).unwrap();
            assert_bits(&out.rows[0], &rsi[0], "pair rsi row");
            assert_bits(&out.rows[1], &cmo[0], "pair cmo row");
        }
    }

    /// ULTOSC erases to: ONE binary `Min` (true_low) and ONE binary `Max`
    /// (true_high) over the shared `Lag{close,1}` anchor, and SIX ring
    /// `RollSum`s forming TWO groups of THREE members each (short/medium/long
    /// on the `bp` column, the same three on `tr`). Both group caps are
    /// `long_period`, which is what makes them fusable into one
    /// `MultiBuffer<2>` — the container the hand itself uses. Sink lead =
    /// `long_period` (hand `min_data = long + 1`).
    #[test]
    fn ultosc_sig_derived() {
        use tulip_rs::cse::kernel::registry::SIG_ultosc_s_m_l;
        use tulip_rs::cse::signature::{K_LAG, K_MAX, K_MIN, K_ROLLSUM};
        let req = Request::new().add(Ultosc::recipe(&[7.0, 14.0, 28.0]), None);
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_ultosc_s_m_l, "ultosc arena drifted from merge");
        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        assert_eq!(count(K_MIN), 1, "true_low: the only minNum node");
        assert_eq!(count(K_MAX), 1, "true_high: the only maxNum node");
        assert_eq!(count(K_LAG), 1, "prev_close is the shared anchor");
        assert_eq!(count(K_ROLLSUM), 6, "three windows on two columns");
        assert_eq!(vals.sink_lead[0], 28, "ultosc lead = long_period");

        // The container-relevant fact, read off Values (not re-derived): two
        // groups of three, both capped at the long window. `bundles()` is
        // pinned against the REAL arena in the macro suite; here we assert the
        // runtime binding that fused call is emitted from agrees with it.
        let rolls: Vec<usize> = (0..sig.len as usize)
            .filter(|&i| sig.kinds[i] == K_ROLLSUM)
            .collect();
        let mut groups: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
        for &m in &rolls {
            groups.entry(vals.ring_of[m] as usize).or_default().push(m);
        }
        assert_eq!(groups.len(), 2, "bp and tr are distinct ring groups");
        for (leader, members) in &groups {
            assert_eq!(members.len(), 3, "group {leader} carries three windows");
            assert_eq!(
                vals.ring_cap[*leader] as usize, 28,
                "group {leader} cap = max member period = long"
            );
        }
    }

    /// ULTOSC bit-exact vs the hand on three data classes. `hi = s + 2`,
    /// `lo = s − 1` keeps `tr > 0` so no `0/0` NaN appears; the FLAT class
    /// exercises exact ties through the ring (every window sum identical), and
    /// `assert_bits` would pin a NaN payload identically if one ever appeared.
    #[test]
    fn ultosc_bit_exact() {
        const OPTS: [f64; 3] = [7.0, 14.0, 28.0];
        let req = Request::new().add(Ultosc::recipe(&OPTS), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &streams).expect("ultosc registered");
            let (u, _) = Ultosc::indicator(&streams, &OPTS, None).unwrap();
            assert_bits(&out.rows[0], &u[0], "ultosc row");
        }
    }

    /// ULTOSC + NATR co-requested: the answer to "is `tr` being shared?" —
    /// **no**, and the counts say so rather than a timing hunch. The two
    /// indicators compute the SAME true range in two different GRAPHS: ultosc
    /// spells it `Sub(Max(hi,pc), Min(lo,pc))` (`tr_nodes` never builds a `Min`
    /// node), natr spells it `Max(Max(Sub(hi,lo),Abs(Sub(hi,pc))),Abs(Sub(lo,pc)))`
    /// through `atr_nodes`. CSE keys on kind + edges, so neither spelling can
    /// intern into the other: `K_MIN` stays 1 (ultosc only) and `K_ABS` stays 2
    /// (natr only). What DOES intern is the `Lag{close,1}` anchor — both
    /// recipes build the identical node, so `K_LAG` is 1 across the pair, and
    /// the three price Inputs are shared.
    ///
    /// Recorded because it is the interesting negative: `tr` is the hand's most
    /// duplicated quantity, and value-equality is not structure-equality.
    #[test]
    fn ultosc_natr_shared_pc_sig_derived() {
        use tulip_rs::cse::kernel::registry::SIG_ultosc_natr_shared_pc;
        use tulip_rs::cse::signature::{K_ABS, K_LAG, K_MIN, K_ROLLSUM};
        let pair = Request::new()
            .add(Ultosc::recipe(&[7.0, 14.0, 28.0]), None)
            .add(Natr::recipe(&[14.0]), None);
        let (merged, sig, vals, _) = erase_request(&pair).unwrap();
        assert_eq!(sig, SIG_ultosc_natr_shared_pc, "pair arena drifted");
        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();

        // Shared: one anchor read for both true ranges, one ring feed per group.
        assert_eq!(count(K_LAG), 1, "prev_close interns across the pair");
        // NOT shared: two independent tr graphs, by construction.
        assert_eq!(count(K_MIN), 1, "ultosc's true_low only — natr has no Min");
        assert_eq!(count(K_ABS), 2, "natr's |hi−pc| and |lo−pc| only");
        assert_eq!(count(K_ROLLSUM), 6, "ultosc's six folds; natr has none");
        // `vals` agrees with the arena the fused call is emitted from, and pins
        // the row order the hand pins below (ultosc primary, its two optionals,
        // then natr's primary).
        assert_eq!(merged.nodes.len(), sig.len as usize, "arena length");
        assert_eq!(vals.sink_lead[0], 28, "ultosc lead = long");
        assert_eq!(vals.sink_lead[3], 14, "natr lead = period");

        let solo_u =
            erase_request(&Request::new().add(Ultosc::recipe(&[7.0, 14.0, 28.0]), None)).unwrap();
        let solo_n = erase_request(&Request::new().add(Natr::recipe(&[14.0]), None)).unwrap();
        assert!(
            sig.len < solo_u.1.len + solo_n.1.len,
            "the pair dedups SOMETHING (the anchor + inputs): {} < {} + {}",
            sig.len,
            solo_u.1.len,
            solo_n.1.len
        );
        // And the pair still beats the two solo passes on every class.
        for s in [wiggle(), series(680, |_| 100.0)] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&pair, &streams).expect("pair registered");
            let (u, _) = Ultosc::indicator(&streams, &[7.0, 14.0, 28.0], None).unwrap();
            let (n, _) = Natr::indicator(&streams, &[14.0], None).unwrap();
            assert_bits(&out.rows[0], &u[0], "pair ultosc row");
            assert_bits(&out.rows[3], &n[0], "pair natr row");
        }
    }

    /// MASS erases to the chain→chain→divide→fold topology the registry had no
    /// other example of: one `Min`-free arena with `K_SUB` for the raw range,
    /// `K_MAX` guarding it with `Const(EPSILON)`, TWO chained `K_EMA`s at the
    /// hand's hardcoded period 9, one `K_DIV`, and exactly ONE ring group. The
    /// hand's magic `16` shows up as `feeds`/leads, not as a knob.
    #[test]
    fn mass_sig_derived() {
        use tulip_rs::cse::kernel::registry::SIG_mass_n;
        use tulip_rs::cse::signature::{K_DIV, K_EMA, K_ROLLSUM, K_SUB};
        let req = Request::new().add(Mass::recipe(&[25.0]), None);
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_mass_n, "mass arena drifted from merge");
        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        assert_eq!(count(K_EMA), 2, "ema9 over hl, ema9 over that");
        assert_eq!(count(K_DIV), 1, "ema / ema_signal");
        assert_eq!(count(K_SUB), 1, "raw high − low");
        assert_eq!(count(K_ROLLSUM), 1, "one ring group, no fusion partner");
        assert_eq!(vals.sink_lead[0], 40, "lead = period + 15");
        // The chain leads ARE the hand's `16`: ema1 = 0+9−1 = 8, ema2 = 8+9−1
        // = 16, and a derived roll feeds at lead(src) — so the ring's first
        // feed bar is 16 with no `skip` spelled anywhere.
        let rolls: Vec<usize> = (0..sig.len as usize)
            .filter(|&i| sig.kinds[i] == K_ROLLSUM)
            .collect();
        assert_eq!(vals.feeds[rolls[0]], 16, "ring feeds at bar 16");
        assert_eq!(vals.periods[rolls[0]], 25, "options[0] sizes the ring only");
    }

    /// MASS bit-exact vs the hand. `hi = s + 2 / lo = s − 1` keeps the range
    /// away from the `EPSILON` clamp; `hi = lo = s` drives it straight into it
    /// (every bar), which is where the hand's warm-up/steady agreement — fixed
    /// 2026-10-15 — has to hold. If the hand's extra `calc_ema` application at
    /// `i == 8` (assign, then recur, on the seed bar) diverges from the
    /// engine's `t == feed → emit src` rule, it shows HERE, on row 0.
    #[test]
    fn mass_bit_exact() {
        let req = Request::new().add(Mass::recipe(&[25.0]), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice()];
            let out = run_registered(&req, &streams).expect("mass registered");
            let (m, _) = Mass::indicator(&streams, &[25.0], None).unwrap();
            assert_bits(&out.rows[0], &m[0], "mass row (range)");

            let flat = [s.as_slice(), s.as_slice()];
            let out2 = run_registered(&req, &flat).expect("mass registered flat");
            let (m2, _) = Mass::indicator(&flat, &[25.0], None).unwrap();
            assert_bits(&out2.rows[0], &m2[0], "mass row (flat: high == low)");
        }
    }

    /// A positive, finite, varying volume column — `ad`/`adosc` are the first
    /// volume-consuming recipes, and every parity matrix here stays finite on
    /// purpose (the hand's guard is only comparable under `0.0 * volume == 0.0`).
    fn volume(n: usize) -> Vec<f64> {
        series(n, |i| 1_000.0 + (i as f64 * 0.3).sin() * 500.0 + 500.0)
    }

    /// AD's arena: four streams (the hand's `(high, low, close, volume)` order),
    /// ONE `Accum`, ONE `WhereLe`, and the structural proof that this is the
    /// engine's first volume-driven recurrence — the accumulator's SECOND
    /// operand is `Input{3}`, the volume column, not a price.
    #[test]
    fn ad_sig_derived() {
        use tulip_rs::cse::kernel::registry::SIG_ad;
        use tulip_rs::cse::signature::{K_ACCUM, K_ADD, K_CONST, K_DIV, K_INPUT, K_SUB, K_WHERELE};
        let req = Request::new().add(Ad::recipe(&[]), None);
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_ad, "ad arena drifted from merge");
        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        assert_eq!(count(K_ACCUM), 1, "the running total is ONE node");
        assert_eq!(count(K_WHERELE), 1, "the hand's early-return guard");
        assert_eq!(count(K_DIV), 1, "clv = numer / range");
        assert_eq!(count(K_SUB), 3, "range, (cl−lo), then −hi");
        assert_eq!(count(K_ADD), 1, "…+ close (the hand's association)");
        assert_eq!(count(K_INPUT), 4, "high, low, close, VOLUME");
        assert_eq!(count(K_CONST), 1, "the f64::EPSILON threshold");
        assert_eq!(sig.n_streams, 4, "four input columns");

        let acc = (0..MAX).find(|i| sig.kinds[*i] == K_ACCUM).unwrap();
        // The multiplier is the volume COLUMN, wired as a graph edge.
        let b = sig.b[acc] as usize;
        assert_eq!(sig.kinds[b], K_INPUT, "Accum{b} is an input stream");
        assert_eq!(sig.a[b], 3, "…and it is stream 3 = volume");
        // Lead 0 ⇒ it feeds from bar 0 (hand `State::new(0.0)`, min_data 1).
        assert_eq!(vals.feeds[acc], 0, "the accumulator feeds from bar 0");
        assert_eq!(vals.sink_lead[0], 0, "ad prints from bar 0");
    }

    /// AD bit-exact vs the hand over three finite classes, the third being the
    /// guard's torture case: `high == low` makes `range == 0 <= eps` on EVERY
    /// bar, so the hand returns its untouched carry and the whole row is +0.0.
    #[test]
    fn ad_bit_exact() {
        let req = Request::new().add(Ad::recipe(&[]), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let vol = volume(s.len());
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice(), vol.as_slice()];
            let out = run_registered(&req, &streams).expect("ad registered");
            let (a, _) = Ad::indicator(&streams, &[], None).unwrap();
            assert_bits(&out.rows[0], &a[0], "ad row");

            // high == low ⇒ the guard fires on every bar ⇒ the carry never moves.
            let flat = [s.as_slice(), s.as_slice(), s.as_slice(), vol.as_slice()];
            let out2 = run_registered(&req, &flat).expect("ad registered flat");
            let (a2, _) = Ad::indicator(&flat, &[], None).unwrap();
            assert_bits(&out2.rows[0], &a2[0], "ad row (guard: high == low)");
        }
    }

    /// The guard's boundary, on FINITE data. `range <= eps` is an EARLY return,
    /// so the `range == eps` bar takes the ZERO arm; `WhereLt` (`<`) would put it
    /// on the VALUE arm instead and start accumulating.
    ///
    /// Constructed exactly: `low = 0.0`, `high = f64::EPSILON` ⇒ `range = eps`
    /// to the bit (any larger anchor would round `hi − lo` away from `eps`), and
    /// `close = 0.0` ⇒ `numer = −eps` ⇒ `ratio = −1.0`, so a wrong polarity adds
    /// `-volume` and shifts EVERY later bar — the divergence is permanent, not
    /// one row. This is what makes the `WhereLe` choice tested rather than
    /// asserted: NaN poisoning can't be pinned here (the matrices are finite),
    /// but the boundary can.
    #[test]
    fn ad_guard_boundary_is_le_not_lt() {
        let req = Request::new().add(Ad::recipe(&[]), None);
        let n = 200;
        let mut hi: Vec<f64> = (0..n).map(|i| 100.0 + i as f64).collect();
        let mut lo: Vec<f64> = (0..n).map(|i| 98.0 + i as f64).collect();
        let cl: Vec<f64> = (0..n).map(|i| 99.0 + i as f64).collect();
        let vol = volume(n);
        // Three boundary bars: range == EPSILON exactly, close == 0.0.
        for i in [5usize, 60, 140] {
            hi[i] = f64::EPSILON;
            lo[i] = 0.0;
        }
        let mut close = cl;
        for i in [5usize, 60, 140] {
            close[i] = 0.0;
        }

        // Precondition, so the test can't pass for the wrong reason: the three
        // bars really do land ON the boundary (subtraction is exact here).
        for i in [5usize, 60, 140] {
            assert_eq!(
                hi[i] - lo[i],
                f64::EPSILON,
                "bar {i} must hit `range == eps` exactly"
            );
        }

        let streams = [
            hi.as_slice(),
            lo.as_slice(),
            close.as_slice(),
            vol.as_slice(),
        ];
        let out = run_registered(&req, &streams).expect("ad registered");
        let (a, _) = Ad::indicator(&streams, &[], None).unwrap();
        assert_bits(&out.rows[0], &a[0], "ad row (range == EPSILON bars)");
    }

    /// ADOSC's arena is the point of the whole exercise: the recurrence reads a
    /// DERIVED, volume-driven source. Both `Ema`s' `src` is the ONE `Accum`
    /// node, and `feeds` is that node's lead (0), so the hand's
    /// `init_state` seed-at-`i == 0`-then-step loop is the engine's ordinary
    /// derived-source convention with no knob. Primary lead = `long − 1`.
    #[test]
    fn adosc_sig_derived() {
        use tulip_rs::cse::kernel::registry::SIG_adosc_s_l;
        use tulip_rs::cse::signature::{K_ACCUM, K_EMA, K_SUB, K_WHERELE};
        let (short, long) = (3.0f64, 10.0f64);
        let req = Request::new().add(Adosc::recipe(&[short, long]), None);
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_adosc_s_l, "adosc arena drifted from merge");
        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        assert_eq!(count(K_ACCUM), 1, "the ad line is ONE accumulator");
        assert_eq!(count(K_WHERELE), 1, "and it is guarded once");
        assert_eq!(count(K_EMA), 2, "short and long, over that line");
        assert_eq!(count(K_SUB), 4, "range, (cl−lo), −hi, then ema_s − ema_l");

        let acc = (0..MAX).find(|i| sig.kinds[*i] == K_ACCUM).unwrap();
        let emas: Vec<usize> = (0..MAX).filter(|i| sig.kinds[*i] == K_EMA).collect();
        assert_eq!(emas.len(), 2);
        for e in &emas {
            assert_eq!(
                sig.a[*e] as usize, acc,
                "ema {e} chains onto the AD accumulator, not a price column"
            );
            assert_eq!(vals.feeds[*e], 0, "lead(ad) = 0 ⇒ the ema feeds from bar 0");
        }
        assert_eq!(vals.sink_lead[0], (long - 1.0) as u32, "lead = long − 1");
    }

    /// MOM(p) = real[i] − real[i−p]: the pattern that puts `Values::lag_n` in a
    /// PUBLISHED ROW. `cmo` already carries a multi-bar `Lag`, but it feeds only
    /// a `Mirror` (state, `get_states`, M1) — so a steady-loop depth bug there is
    /// invisible to every parity test ever written. Here the depth IS the value:
    /// read one bar back instead of `p` and the row is `real[i] − real[i−1]`,
    /// which differs from the hand at EVERY steady bar of a moving series.
    ///
    /// The warm prologue is NOT the suspect — `warmup.rs`'s `K_LAG` arm already
    /// reads `vals.lag_n[k]` with the hand's clamp. The defect is the steady
    /// loop's hard-coded `.sub(1)`, so the first wrong row lands at the first
    /// steady index, right after the warm region ends.
    #[test]
    fn mom_sig_derived() {
        use tulip_rs::cse::kernel::registry::SIG_mom_n;
        use tulip_rs::cse::signature::{K_INPUT, K_LAG, K_SUB};
        let req = Request::new().add(Mom::recipe(&[10.0]), None);
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_mom_n, "mom arena drifted from merge");
        let lag = (0..MAX).find(|i| sig.kinds[*i] == K_LAG).unwrap();
        assert_eq!(vals.lag_n[lag], 10, "the depth is a VALUE, not structure");
        assert_eq!(vals.sink_lead[0], 10, "lead = p (min_data p+1)");
        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        assert_eq!(count(K_LAG), 1);
        assert_eq!(count(K_SUB), 1);
        assert_eq!(count(K_INPUT), 1);
    }

    #[test]
    fn mom_bit_exact() {
        const P: [f64; 1] = [10.0];
        let req = Request::new().add(Mom::recipe(&P), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let streams = [s.as_slice()];
            let out = run_registered(&req, &streams).expect("mom registered");
            let (m, _) = Mom::indicator(&streams, &P, None).unwrap();
            assert_bits(&out.rows[0], &m[0], "mom row (lag depth = period)");
        }
    }

    /// ADOSC bit-exact vs the hand — the measured answer to "did the
    /// non-price input cost us anything" is the bench row; here the claim is
    /// only that it is CORRECT on three finite classes.
    #[test]
    fn adosc_bit_exact() {
        const OPTS: [f64; 2] = [3.0, 10.0];
        let req = Request::new().add(Adosc::recipe(&OPTS), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let vol = volume(s.len());
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice(), vol.as_slice()];
            let out = run_registered(&req, &streams).expect("adosc registered");
            let (a, _) = Adosc::indicator(&streams, &OPTS, None).unwrap();
            assert_bits(&out.rows[0], &a[0], "adosc row");

            let flat = [s.as_slice(), s.as_slice(), s.as_slice(), vol.as_slice()];
            let out2 = run_registered(&req, &flat).expect("adosc registered flat");
            let (a2, _) = Adosc::indicator(&flat, &OPTS, None).unwrap();
            assert_bits(&out2.rows[0], &a2[0], "adosc row (guard: high == low)");
        }
    }

    /// THE CROSS-CONVENTION CO-REQUEST, as THREE REAL RECIPES. This is the arena
    /// the registry used to refuse and hand-author as one composing recipe
    /// (`recipes::adosc_apo_ppo`, now deleted). The assertion that matters is the
    /// SOURCE each price chain names: `Apo`/`Ppo` bind price to `Input{0}` in
    /// THEIR OWN slots, and this arena's slot 0 is HIGH (adosc is OHLCV-shaped),
    /// so `merge`'s column-declared path must remap them onto arena stream 2 —
    /// the Close node. If the remap ever regressed to positional, the four price
    /// EMAs would read stream 0 and this gate would fail on `on_close`.
    #[test]
    fn adosc_apo_ppo_sig_derived() {
        use tulip_rs::cse::kernel::registry::SIG_adosc_apo_ppo;
        use tulip_rs::cse::signature::{K_ACCUM, K_EMA, K_INPUT, K_SUB, K_WHERELE};
        // adosc(3,10) · apo(12,26) · ppo(6,14) — all distinct, no accidental dedup.
        let req = Request::new()
            .add(Adosc::recipe(&[3.0, 10.0]), None)
            .add(Apo::recipe(&[12.0, 26.0]), None)
            .add(Ppo::recipe(&[6.0, 14.0]), None);
        let (_a, sig, vals, owners) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_adosc_apo_ppo, "mixed arena drifted from merge");
        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        assert_eq!(count(K_EMA), 6, "2 over the ad line + 4 over close");
        assert_eq!(count(K_ACCUM), 1, "ONE volume accumulator");
        assert_eq!(count(K_WHERELE), 1, "one guard");
        assert_eq!(count(K_SUB), 6, "range, −hi, adosc, apo line, ppo line, …");
        assert_eq!(count(K_INPUT), 4, "high, low, close, volume");

        let acc = (0..MAX).find(|i| sig.kinds[*i] == K_ACCUM).unwrap();
        // The close column: the Input node whose stream index is 2.
        let close_node = (0..MAX)
            .find(|i| sig.kinds[*i] == K_INPUT && sig.a[*i] == 2)
            .unwrap();
        let emas: Vec<usize> = (0..MAX).filter(|i| sig.kinds[*i] == K_EMA).collect();
        let on_ad: Vec<usize> = emas
            .iter()
            .filter(|e| sig.a[**e] as usize == acc)
            .copied()
            .collect();
        let on_close: Vec<usize> = emas
            .iter()
            .filter(|e| sig.a[**e] as usize == close_node)
            .copied()
            .collect();
        assert_eq!(on_ad.len(), 2, "adosc's two EMAs chain onto the AD line");
        assert_eq!(
            on_close.len(),
            4,
            "apo + ppo read the CLOSE column (stream 2) — the column-declared remap"
        );
        assert_eq!(emas.len(), 6, "every EMA is accounted for");

        // Primary leads, one per indicator, found by OWNER not by position: three
        // real recipes also carry their optional mirrors in the arena (want=false
        // rows), so sink positions are a property of the merge, not of the request.
        let lead_of = |req_idx: usize| -> u32 {
            let s = owners
                .iter()
                .position(|o| o.req == req_idx && o.primary)
                .unwrap_or_else(|| panic!("no primary sink for request {req_idx}"));
            vals.sink_lead[s]
        };
        assert_eq!(lead_of(0), 9, "adosc lead = 10 − 1");
        assert_eq!(lead_of(1), 25, "apo lead = 26 − 1");
        assert_eq!(lead_of(2), 14, "ppo lead = 14");
    }

    /// ADOSC + APO + PPO in one arena, bit-exact against the three HANDS — each
    /// called with its own arity (`Adosc` gets OHLCV, `Apo`/`Ppo` get close
    /// only). This is the test that would fail if the co-request had bound apo/ppo
    /// to stream 0: their rows would come out computed over the HIGH column while
    /// every `SIG_*` check still passed.
    #[test]
    fn adosc_apo_ppo_bit_exact() {
        let req = Request::new()
            .add(Adosc::recipe(&[3.0, 10.0]), None)
            .add(Apo::recipe(&[12.0, 26.0]), None)
            .add(Ppo::recipe(&[6.0, 14.0]), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let vol = volume(s.len());
            let ohlcv = [hi.as_slice(), lo.as_slice(), s.as_slice(), vol.as_slice()];
            let out = run_registered(&req, &ohlcv).expect("mixed arena registered");

            // Each hand on ITS OWN input array.
            let (os, _) = Adosc::indicator(&ohlcv, &[3.0, 10.0], None).unwrap();
            let (ap, _) = Apo::indicator(&[s.as_slice()], &[12.0, 26.0], None).unwrap();
            let (pp, _) = Ppo::indicator(&[s.as_slice()], &[6.0, 14.0], None).unwrap();
            assert_bits(row_for(&out, 0, true, 0), &os[0], "mixed arena: adosc row");
            assert_bits(row_for(&out, 1, true, 0), &ap[0], "mixed arena: apo row");
            assert_bits(row_for(&out, 2, true, 0), &pp[0], "mixed arena: ppo row");

            // The optional mirrors are in the arena but NOT wanted, so they must
            // come back empty — the runtime want-bit model, asserted rather than
            // assumed (this is what `rows.len()` grew to when three real recipes
            // replaced the one hand-composed recipe that declared no optionals).
            let disabled = out
                .owners
                .iter()
                .zip(out.rows.iter())
                .filter(|(o, _)| !o.primary)
                .count();
            assert!(disabled > 0, "three real recipes carry optional mirrors");
            for (o, r) in out.owners.iter().zip(out.rows.iter()) {
                if !o.primary {
                    assert!(r.is_empty(), "disabled optional {o:?} must be empty");
                }
            }
        }
    }

    /// The four-indicator dashboard arena, as FOUR REAL RECIPES. The assertion
    /// that matters is WHICH column the hub reads: `Apo`/`Ppo`/`Macd` bind price
    /// to `Input{0}` in their own slots, while `mass` binds `Input{0}=high` — so
    /// under positional binding the hub would have interned its close onto
    /// mass's HIGH column, and the arena would still erase, still match its
    /// `SIG_*`, still count 5 EMAs, with only the VALUES wrong. `mass` now
    /// declares `hl()` and the hub declares `price()`, so `merge` ranks the union
    /// {High:0, Low:1, Close:2} and the hub's EMAs land on stream 2 by derivation
    /// — which is what `on_close == 2` below now proves.
    #[test]
    fn mass_apo_ppo_macd_sig_derived() {
        use tulip_rs::cse::kernel::registry::SIG_mass_apo_ppo_macd;
        use tulip_rs::cse::signature::{K_DIV, K_EMA, K_INPUT, K_MAX, K_MUL, K_ROLLSUM, K_SUB};
        let req = Request::new()
            .add(Mass::recipe(&[40.0]), None)
            .add(Apo::recipe(&[12.0, 26.0]), None)
            .add(Ppo::recipe(&[12.0, 26.0]), None)
            .add(Macd::recipe(&[12.0, 26.0, 9.0]), None);
        let (_a, sig, vals, owners) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_mass_apo_ppo_macd, "arena drifted from merge");
        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        // 2 hub close-EMAs + 1 signal + mass's two 9-EMAs.
        assert_eq!(count(K_EMA), 5);
        assert_eq!(count(K_ROLLSUM), 1, "mass's one ring group");
        assert_eq!(count(K_MAX), 1, "the hl_diff epsilon clamp");
        assert_eq!(count(K_INPUT), 3, "high, low, CLOSE");

        let node_of_stream = |want: u32| -> usize {
            (0..MAX)
                .find(|i| sig.kinds[*i] == K_INPUT && sig.a[*i] == want)
                .unwrap()
        };
        let hi = node_of_stream(0);
        let lo = node_of_stream(1);
        let cl = node_of_stream(2);

        let emas: Vec<usize> = (0..MAX).filter(|i| sig.kinds[*i] == K_EMA).collect();
        let on_close: Vec<usize> = emas
            .iter()
            .filter(|e| sig.a[**e] as usize == cl)
            .copied()
            .collect();
        assert_eq!(
            on_close.len(),
            2,
            "the hub's short+long EMAs read CLOSE (stream 2), not the high column"
        );

        // mass's range is `Sub{Input{0}, Input{1}}` — the OHLC pair.
        let ranges: Vec<usize> = (0..MAX)
            .filter(|i| {
                sig.kinds[*i] == K_SUB && sig.a[*i] as usize == hi && sig.b[*i] as usize == lo
            })
            .collect();
        assert_eq!(ranges.len(), 1, "mass's high−low over streams 0 and 1");

        // Leads, found by OWNER not by position: four real recipes also carry
        // their optional mirrors in the arena (want=false), so sink indices are a
        // property of the merge. Requests are [mass, apo, ppo, macd] and macd
        // publishes THREE primaries (line, signal, hist).
        let lead_of = |req_idx: usize, primary: bool, idx: usize| -> u32 {
            let s = owners
                .iter()
                .position(|o| o.req == req_idx && o.primary == primary && o.idx == idx)
                .unwrap_or_else(|| panic!("no sink for req {req_idx} primary {primary} idx {idx}"));
            vals.sink_lead[s]
        };
        assert_eq!(lead_of(0, true, 0), 55, "mass lead = 40 + 15");
        assert_eq!(lead_of(1, true, 0), 25, "apo line lead = 26 − 1");
        assert_eq!(lead_of(2, true, 0), 26, "ppo lead = 26");
        assert_eq!(lead_of(3, true, 0), 25, "macd line lead = 26 − 1");
        assert_eq!(lead_of(3, true, 1), 33, "signal lead = 26 + 9 − 2");
        assert_eq!(lead_of(3, true, 2), 33, "hist lead = 26 + 9 − 2");

        // The dedup claim: APO, PPO and MACD share the ema_s/ema_l/line triple,
        // so the arena is strictly smaller than the four solo arenas added up.
        let solo = |r: Request| erase_request(&r).unwrap().1.len;
        let sum = solo(Request::new().add(Mass::recipe(&[40.0]), None))
            + solo(Request::new().add(Apo::recipe(&[12.0, 26.0]), None))
            + solo(Request::new().add(Ppo::recipe(&[12.0, 26.0]), None))
            + solo(Request::new().add(Macd::recipe(&[12.0, 26.0, 9.0]), None));
        assert!(
            sig.len < sum,
            "the arena dedups: {} < {} (four solos)",
            sig.len,
            sum
        );
        // And mass's share is ZERO — its source is high−low, not close.
        assert_eq!(count(K_MUL), 1, "only ppo's ×100");
        assert_eq!(count(K_DIV), 2, "ppo's ratio and mass's ratio");
    }

    /// Bit-exactness of the four-indicator arena against all FOUR hands, each
    /// called on its own input arity: `Mass` gets `[high, low]`, the hub gets
    /// `[close]`. This is the test that would fail if the slots were still
    /// positional — APO/PPO/MACD would come out over the HIGH column while every
    /// structural count in `mass_apo_ppo_macd_sig_derived` still passed.
    ///
    /// Rows are addressed by OWNER. Four real recipes also carry their optional
    /// mirrors as `want=false` sinks, so the arena has SIX primary rows (mass,
    /// apo, ppo, and macd's line/signal/hist) inside twelve sinks total — the
    /// composing recipe published five and declared no optionals.
    #[test]
    fn mass_apo_ppo_macd_bit_exact() {
        let req = Request::new()
            .add(Mass::recipe(&[40.0]), None)
            .add(Apo::recipe(&[12.0, 26.0]), None)
            .add(Ppo::recipe(&[12.0, 26.0]), None)
            .add(Macd::recipe(&[12.0, 26.0, 9.0]), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let arena = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &arena).expect("mass+hub arena registered");
            let primaries = out.owners.iter().filter(|o| o.primary).count();
            assert_eq!(primaries, 6, "mass, apo, ppo, macd line/signal/hist");

            let hub = [s.as_slice()];
            let (ap, _) = Apo::indicator(&hub, &[12.0, 26.0], None).unwrap();
            let (pp, _) = Ppo::indicator(&hub, &[12.0, 26.0], None).unwrap();
            let (mac, _) = Macd::indicator(&hub, &[12.0, 26.0, 9.0], None).unwrap();
            let two = [hi.as_slice(), lo.as_slice()];
            let (mm, _) = Mass::indicator(&two, &[40.0], None).unwrap();

            assert_bits(row_for(&out, 1, true, 0), &ap[0], "mass+hub: apo line");
            assert_bits(row_for(&out, 3, true, 0), &mac[0], "mass+hub: macd line");
            // THE DEDUP CLAIM, read off the rows: apo's line and macd's line are
            // the SAME interned Sub node, so the two primary rows must be
            // byte-identical to each other and to both hands.
            assert_bits(
                row_for(&out, 1, true, 0),
                row_for(&out, 3, true, 0),
                "mass+hub: macd line IS apo's line",
            );
            assert_bits(row_for(&out, 2, true, 0), &pp[0], "mass+hub: ppo");
            assert_bits(row_for(&out, 3, true, 1), &mac[1], "mass+hub: signal");
            assert_bits(row_for(&out, 3, true, 2), &mac[2], "mass+hub: histogram");
            assert_bits(row_for(&out, 0, true, 0), &mm[0], "mass+hub: mass index");

            // Every mirror the request did not ask for comes back empty.
            for (o, r) in out.owners.iter().zip(out.rows.iter()) {
                if !o.primary {
                    assert!(r.is_empty(), "disabled optional {o:?} must be empty");
                }
            }
        }
    }

    // ── THE DIRECTIONAL HUB: dm + di + dx + adx on ONE DM chain ───────────
    use tulip_rs::indicators::adx::Adx;
    use tulip_rs::indicators::di::Di;
    use tulip_rs::indicators::dm::Dm;
    use tulip_rs::indicators::dx::Dx;

    const HP: f64 = 14.0;

    fn hub_req() -> Request {
        Request::new()
            .add(Dm::recipe(&[HP]), None)
            .add(Di::recipe(&[HP]), None)
            .add(Dx::recipe(&[HP]), None)
            .add(Adx::recipe(&[HP]), None)
    }

    /// Locate a row by its OWNER (request, primary?, output index) instead of
    /// by a hand-counted sink position — the arena dedups `adx`'s optional `dx`
    /// onto `Dx`'s own primary node, so positional indices are a property of
    /// the merge, not of the request, and a gate that hard-codes them fails for
    /// wiring reasons when the numbers are right.
    fn row_for<'a>(out: &'a Outcome, req: usize, primary: bool, idx: usize) -> &'a [f64] {
        let i = out
            .owners
            .iter()
            .position(|o| o.req == req && o.primary == primary && o.idx == idx)
            .unwrap_or_else(|| {
                panic!(
                    "no sink for req {req} primary {primary} idx {idx}; owners = {:?}",
                    out.owners
                )
            });
        out.rows[i].as_slice()
    }

    /// Macro-time arena equals runtime merge+erase for the hub. This is the gate
    /// that a `Signature::b` drift FAILS: before the planner carried `composite`
    /// into `ANode.b`, a composite `Wilders` erased to `b=1` at runtime while the
    /// generated `SIG_*` said `b=0`, so no request could ever match the registry.
    #[test]
    fn dm_di_dx_adx_shared_p_sig_derived() {
        use tulip_rs::cse::kernel::registry::SIG_dm_di_dx_adx_shared_p;
        let req = hub_req();
        let (_a, sig, _, _) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_dm_di_dx_adx_shared_p, "hub arena drifted");
    }

    /// The claim the ADX port rests on: ONE arena carrying BOTH Wilders forms,
    /// each published by `erase` as a SOURCE SCALE (`invms`), not as a distinct
    /// node kind. `K_WILDERS` is 4 — the value `planner::Kind::Wilders::tag()`
    /// must equal and the `*_sig_derived` gates enforce, so scanning the tag here
    /// is scanning the same fact the emitter uses.
    ///
    /// The forms do NOT share a source any more: `di`'s denominator moved onto the
    /// shared MEAN ATR chain (the period cancels into its `100/p` constant), so the
    /// composites left are `dm`'s `+DM`/`−DM` accumulators over their own
    /// mutual-exclusion nodes. That removal is deliberate — a composite and a mean
    /// over the SAME `tr` made the emitter publish the sum into the mean slot's row
    /// (`adx_atr_mirror_is_the_shared_mean_chain_within_1e12` gates the fixed
    /// shape). The defect behind that is STILL OPEN for any future arena that
    /// re-pairs them; this test proves the magnitudes are published correctly, not
    /// that the emitter reads them per slot.
    #[test]
    fn dm_di_dx_adx_shared_p_carries_both_wilders_forms() {
        const K_WILDERS: u8 = 4;
        let req = hub_req();
        let (_a, sig, vals, _) = erase_request(&req).unwrap();
        let slots: Vec<usize> = (0..sig.len as usize)
            .filter(|k| sig.kinds[*k] == K_WILDERS)
            .collect();
        let composite: Vec<usize> = slots.iter().copied().filter(|k| sig.b[*k] == 1).collect();
        let mean: Vec<usize> = slots.iter().copied().filter(|k| sig.b[*k] == 0).collect();
        assert!(!composite.is_empty(), "hub has no COMPOSITE Wilders slot");
        assert!(!mean.is_empty(), "hub has no MEAN Wilders slot");

        let period = HP as f64;
        let want_mult = (period - 1.0) / period;
        for k in &composite {
            assert_eq!(vals.invms[*k], 1.0, "composite slot {k} folds the raw sum");
            assert_eq!(vals.mults[*k], want_mult, "composite slot {k} decay");
        }
        for k in &mean {
            assert!(
                (vals.invms[*k] - 1.0 / period).abs() <= f64::EPSILON,
                "mean slot {k} carries 1/period, got {}",
                vals.invms[*k]
            );
            assert_eq!(vals.mults[*k], want_mult, "mean slot {k} decay");
        }

        // No two Wilders may share a source while differing in form — that is the
        // shape the emitter still mishandles, so the hub must not contain it.
        for i in &slots {
            for j in slots.iter() {
                if j <= i {
                    continue;
                }
                if sig.a[*i] == sig.a[*j] {
                    assert_eq!(
                        sig.b[*i], sig.b[*j],
                        "slots {i}/{j} share a source with different Wilders forms \
                         — the emitter defect would fire"
                    );
                }
            }
        }
    }

    /// Primary rows bit-exact against all FOUR hand indicators. `Dm` is called
    /// on its own 2-arity, the rest on (high, low, close).
    #[test]
    fn dm_di_dx_adx_shared_p_bit_exact() {
        for base in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let hi: Vec<f64> = base.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = base.iter().map(|v| v - 1.0).collect();
            let out = run_registered(&hub_req(), &[hi.as_slice(), lo.as_slice(), &base])
                .expect("hub registered");

            let two = [hi.as_slice(), lo.as_slice()];
            let three = [hi.as_slice(), lo.as_slice(), base.as_slice()];
            let (dm, _) = Dm::indicator(&two, &[HP], None).unwrap();
            let (di, _) = Di::indicator(&three, &[HP], None).unwrap();
            let (dx, _) = Dx::indicator(&three, &[HP], None).unwrap();
            let (adx, _) = Adx::indicator(&three, &[HP], None).unwrap();

            assert_bits(row_for(&out, 0, true, 0), &dm[0], "hub: +DM");
            assert_bits(row_for(&out, 0, true, 1), &dm[1], "hub: -DM");
            // DI's denominator is the SHARED MEAN ATR chain now (the period
            // cancels into the `100/p` constant), so these two rows are RELATIVE,
            // not bit-exact: `(100/p)/meanATR` and the hand's `100/ΣATR` are the
            // same real value, rounded differently. DX/ADX do not read ATR at all
            // (DX is a ratio of the two DM sums) so they stay bit-exact.
            for (idx, (label, want)) in [("hub: +DI", &di[0]), ("hub: -DI", &di[1])]
                .into_iter()
                .enumerate()
            {
                let got = row_for(&out, 1, true, idx);
                assert_eq!(got.len(), want.len(), "{label}: row length");
                let mut max_rel = 0.0f64;
                for (g, w) in got.iter().zip(want.iter()) {
                    let scale = g.abs().max(w.abs()).max(1.0);
                    max_rel = max_rel.max((g - w).abs() / scale);
                }
                eprintln!("{label}: max relative error {max_rel:e}");
                assert!(max_rel <= 1e-12, "{label}: max rel {max_rel:e} > 1e-12");
                assert_rel_1e12(got, want, label);
            }
            assert_bits(row_for(&out, 2, true, 0), &dx[0], "hub: DX");
            assert_bits(row_for(&out, 3, true, 0), &adx[0], "hub: ADX");
        }
    }

    /// Packed twin vs scalar, ONE plan bound into both generated kernels —
    /// the same-binary A/B the twin convention exists for.
    ///
    /// This is now a VACUOUS-CORRECTNESS gate, and that is the measured state:
    /// the twin builds ZERO lane groups (`hub_packed_twin_builds_no_lane_group`
    /// in the macro crate asserts the absence), so it is the same straight-line
    /// code as the scalar kernel and byte-equality is expected, not evidence of a
    /// capability. The pair is kept because if these two ever STOP being
    /// byte-equal, a same-source mean/composite `Wilders` pair has come back and
    /// the published row is wrong — the failure mode measured on 2026-10-16 (the
    /// mean slot's row came out as the raw SUM, 15.2 → 40.4 where the hand prints
    /// ~6.1). Do not read a PASS here as mixed-form packing working.
    #[test]
    fn dm_di_dx_adx_shared_p_packed_matches_scalar() {
        use tulip_rs::cse::kernel::registry::{dm_di_dx_adx_shared_p, dm_di_dx_adx_shared_p_p};
        for base in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let hi: Vec<f64> = base.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = base.iter().map(|v| v - 1.0).collect();
            let (_a, _sig, vals, _) = erase_request(&hub_req()).unwrap();
            let streams = [hi.as_slice(), lo.as_slice(), base.as_slice()];
            let scalar = dm_di_dx_adx_shared_p(&vals, &streams);
            let packed = dm_di_dx_adx_shared_p_p(&vals, &streams);
            assert_eq!(scalar.len(), packed.len(), "same row count");
            for (i, (rs, rp)) in scalar.iter().zip(packed.iter()).enumerate() {
                assert_bits(rs.as_slice(), rp.as_slice(), &format!("hub packed row {i}"));
            }
        }
    }

    /// THE ATR-MIRROR PARITY MEASUREMENT. Hand `adx.rs` publishes its optional
    /// `atr` as `corrected(atr, inv_multiplier)` — the COMPOSITE running SUM
    /// scaled by 1/p — while this engine's `adx` recipe publishes the MEAN
    /// `Wilders(tr)` chain (the very node `atr`/`natr`/`supertrend` build). Those
    /// are equal in real arithmetic (mean seed = sum/p; and scaling the composite
    /// step `c = c*mult + s` by inv gives `(c*inv)*mult + s*inv`, the mean step
    /// exactly) but NOT bit-identical: the mean divides by `p` at the seam where
    /// the hand multiplies by `1/p`, so the two drift by ULPs.
    ///
    /// This is the gate that decides whether adx may share one ATR chain with the
    /// rest of the family, so it MEASURES the distance rather than trusting the
    /// comment: the mirror must be within 1e-12 of the HAND ADX row, and — the
    /// sharing claim itself — bit-identical to the standalone hand `Atr` row.
    #[test]
    fn adx_atr_mirror_is_the_shared_mean_chain_within_1e12() {
        for base in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let hi: Vec<f64> = base.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = base.iter().map(|v| v - 1.0).collect();
            let three = [hi.as_slice(), lo.as_slice(), base.as_slice()];

            // adx's optionals ON, read THROUGH THE HUB. A lone `Adx` request is a
            // legitimate pattern to register (user 2026-10-16 relaxed the old
            // absolute: lone shapes are diagnostics, allowed, just not expected to
            // win) — this test goes through the hub anyway because the hub is the
            // shape that SHIPS, and the mirror only exists in a merged arena.
            // Optional enable is a runtime want-bit, never an arena difference, so
            // this is still SIG_dm_di_dx_adx_shared_p. adx is request 3 of
            // [dm,di,dx,adx].
            let req = Request::new()
                .add(Dm::recipe(&[HP]), None)
                .add(Di::recipe(&[HP]), None)
                .add(Dx::recipe(&[HP]), None)
                .add(Adx::recipe(&[HP]), Some(&[true, true, true]));
            let out = run_registered(&req, &three).expect("hub template");
            let (hand, _) = Adx::indicator(&three, &[HP], Some(&[true, true, true])).unwrap();
            assert_bits(row_for(&out, 3, true, 0), &hand[0], "adx primary");
            assert_bits(row_for(&out, 3, false, 0), &hand[1], "adx dx mirror");
            // The mirror in question, and the hand's own tr row.
            assert_bits(row_for(&out, 3, false, 2), &hand[3], "adx tr mirror");

            let (atr_hand, _) =
                tulip_rs::indicators::atr::Atr::indicator(&three, &[HP], None).unwrap();
            // THE SHARING CLAIM, measured: adx's atr mirror IS the standalone ATR
            // chain, byte-for-byte — which is what lets one arena serve the whole
            // family. It is also the row that was the raw SUM before DI moved onto
            // the shared chain, so this assertion is the regression gate for that
            // defect.
            assert_bits(
                row_for(&out, 3, false, 1),
                &atr_hand[0],
                "adx's atr mirror is the SAME rows as Atr::indicator",
            );
            // And within 1e-12 of the HAND ADX row, which publishes `sum × inv`
            // rather than the mean chain (`atr.rs:60-62` divides only when
            // `!composite`) — same value, different rounding.
            assert_rel_1e12(
                row_for(&out, 3, false, 1),
                &hand[2],
                "adx atr mirror (engine mean chain vs hand sum*inv)",
            );
        }
    }

    // ── THE LONE-SHAPE FLOOR (`dm_n`, `di_n`, `dx_n`, `adx_n`) ─────────────
    // Registered ALONE as diagnostics, not shipping options — the question they
    // answer is how far behind the hand an un-merged shape of this family sits,
    // which is the denominator the hub's 4.6x needs (user 2026-10-16 relaxed the
    // old absolute ban on lone patterns: allowed for testing, not expected to win).
    //
    // THE ROUTE CHECK IS LOAD-BEARING. A request whose arena matches no `SIG_*`
    // does not fail — `try_run` returns `None` and the caller falls back to the
    // hand. A lone-shape BENCH measured that way is the hand timed twice and still
    // reports parity, so every row here asserts `sig == SIG_*` BEFORE any timing
    // claim is allowed to exist.
    #[test]
    fn lone_dm_di_dx_adx_atr_arenas_match_their_registered_signatures() {
        use tulip_rs::cse::kernel::registry::{
            SIG_adx_n, SIG_atr_n, SIG_di_n, SIG_dm_di_dx_adx_shared_p, SIG_dm_n, SIG_dx_n,
        };
        let cases: [(&str, Request, tulip_rs::cse::Signature); 5] = [
            (
                "dm_n",
                Request::new().add(Dm::recipe(&[HP]), None),
                SIG_dm_n,
            ),
            (
                "di_n",
                Request::new().add(Di::recipe(&[HP]), None),
                SIG_di_n,
            ),
            (
                "dx_n",
                Request::new().add(Dx::recipe(&[HP]), None),
                SIG_dx_n,
            ),
            (
                "adx_n",
                Request::new().add(Adx::recipe(&[HP]), None),
                SIG_adx_n,
            ),
            (
                "atr_n",
                Request::new().add(tulip_rs::indicators::atr::Atr::recipe(&[HP]), None),
                SIG_atr_n,
            ),
        ];
        for (name, req, want) in cases {
            let (_a, sig, _, _) = erase_request(&req)
                .unwrap_or_else(|| panic!("{name}: erase refused its own arena"));
            assert_eq!(sig, want, "{name}: arena drifted from the generated SIG");
            // A lone shape must also be DISTINCT from the hub, or the registry
            // would have collided at expansion; asserted here so the floor rows
            // cannot silently become the hub's twin.
            assert_ne!(sig, SIG_dm_di_dx_adx_shared_p, "{name} is not the hub");
        }
    }

    /// THE FLOOR vs THE HUB, per indicator: the same recipe run ALONE must
    /// produce the SAME ROWS as inside the merged hub. That is the claim fusion
    /// rests on — the merge changes SCHEDULING (one chain instead of four), never
    /// VALUES. It is the check the hand comparison alone cannot make: hand-vs-
    /// engine can be green on a shape whose merged form drifted, because BOTH sides
    /// would then be wrong the same way, and it cannot show that DI's denominator
    /// interned onto the shared ATR chain computes what DI's own chain computes.
    ///
    /// `dm`/`dx`/`adx` bit-exact vs the hand; `di` RELATIVE (<=1e-12) for the
    /// reason the hub test documents — its denominator reads the shared mean ATR
    /// chain, so `(100/p)/mean` rounds differently from the hand's `100/sum`.
    /// Against the HUB every four rows are bit-exact, including DI: same recipe,
    /// same arena node, same rounding.
    #[test]
    fn lone_dm_di_dx_adx_atr_match_the_hand_and_the_hub() {
        use tulip_rs::cse::kernel::registry::{SIG_adx_n, SIG_atr_n, SIG_di_n, SIG_dm_n, SIG_dx_n};
        for base in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let hi: Vec<f64> = base.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = base.iter().map(|v| v - 1.0).collect();
            let two = [hi.as_slice(), lo.as_slice()];
            let three = [hi.as_slice(), lo.as_slice(), base.as_slice()];
            let hub = run_registered(&hub_req(), &three).expect("hub registered");

            // dm: 2-arity (high, low) alone, against the hand AND the hub's dm rows.
            let (dm_hand, _) = Dm::indicator(&two, &[HP], None).unwrap();
            let dm = run_registered(&Request::new().add(Dm::recipe(&[HP]), None), &two)
                .expect("dm_n registered");
            assert_bits(row_for(&dm, 0, true, 0), &dm_hand[0], "lone dm: +DM");
            assert_bits(row_for(&dm, 0, true, 1), &dm_hand[1], "lone dm: -DM");
            assert_bits(
                row_for(&dm, 0, true, 0),
                row_for(&hub, 0, true, 0),
                "lone +DM == hub +DM",
            );
            assert_bits(
                row_for(&dm, 0, true, 1),
                row_for(&hub, 0, true, 1),
                "lone -DM == hub -DM",
            );

            // dx: ratio of the two DM sums, bit-exact both ways.
            let (dx_hand, _) = Dx::indicator(&three, &[HP], None).unwrap();
            let dx = run_registered(&Request::new().add(Dx::recipe(&[HP]), None), &three)
                .expect("dx_n registered");
            assert_bits(row_for(&dx, 0, true, 0), &dx_hand[0], "lone dx: DX");
            assert_bits(
                row_for(&dx, 0, true, 0),
                row_for(&hub, 2, true, 0),
                "lone DX == hub DX",
            );

            // adx: primary bit-exact both ways (requested primary-only, so no
            // mirror rows here — the mirror is the hub test's subject).
            let (adx_hand, _) = Adx::indicator(&three, &[HP], None).unwrap();
            let adx = run_registered(&Request::new().add(Adx::recipe(&[HP]), None), &three)
                .expect("adx_n registered");
            assert_bits(row_for(&adx, 0, true, 0), &adx_hand[0], "lone adx: ADX");
            assert_bits(
                row_for(&adx, 0, true, 0),
                row_for(&hub, 3, true, 0),
                "lone ADX == hub ADX",
            );

            // di: RELATIVE vs the hand (mean-chain scale), BIT-EXACT vs the hub —
            // the row this test exists for: DI's denominator is the interned shared
            // ATR node inside the hub and its own sub-graph copy alone.
            let (di_hand, _) = Di::indicator(&three, &[HP], None).unwrap();
            let di = run_registered(&Request::new().add(Di::recipe(&[HP]), None), &three)
                .expect("di_n registered");
            assert_rel_1e12(row_for(&di, 0, true, 0), &di_hand[0], "lone di: +DI");
            assert_rel_1e12(row_for(&di, 0, true, 1), &di_hand[1], "lone di: -DI");
            assert_bits(
                row_for(&di, 0, true, 0),
                row_for(&hub, 1, true, 0),
                "lone +DI == hub +DI",
            );
            assert_bits(
                row_for(&di, 0, true, 1),
                row_for(&hub, 1, true, 1),
                "lone -DI == hub -DI",
            );

            // atr: the DECOMPOSITION RUNG. DI's cost is (the whole dm chain) +
            // (the mean ATR chain) + two Mul/Max glue nodes, so without a lone
            // ATR row measured on the same bars, "why is DI 2x behind?" is
            // unanswerable. Bit-exact vs the hand — the same claim
            // `adx_atr_mirror_is_the_shared_mean_chain_within_1e12` makes through
            // adx's optional, now made directly.
            let (atr_hand, _) =
                tulip_rs::indicators::atr::Atr::indicator(&three, &[HP], None).unwrap();
            let atr = run_registered(
                &Request::new().add(tulip_rs::indicators::atr::Atr::recipe(&[HP]), None),
                &three,
            )
            .expect("atr_n registered");
            assert_bits(row_for(&atr, 0, true, 0), &atr_hand[0], "lone atr: ATR");

            // Every floor row must be served by ITS OWN kernel, not a fallback:
            // an unmatched arena returns None from try_run and the caller would
            // silently re-run the hand, making the bench floor meaningless.
            assert_eq!(dm.fp, SIG_dm_n.hash(), "dm floor row ran the dm kernel");
            assert_eq!(dx.fp, SIG_dx_n.hash(), "dx floor row ran the dx kernel");
            assert_eq!(adx.fp, SIG_adx_n.hash(), "adx floor row ran the adx kernel");
            assert_eq!(di.fp, SIG_di_n.hash(), "di floor row ran the di kernel");
            assert_eq!(atr.fp, SIG_atr_n.hash(), "atr floor row ran the atr kernel");
        }
    }

    // ── THE FULL DASHBOARD: seven indicators, ONE arena ────────────────────
    // dm + di + dx + adx + atr + natr + supertrend. This is the shape the user
    // predicted ("you should also be able to combine adx with natr and
    // supertrend"), and it only routes because DI moved onto the shared mean ATR
    // chain (d230b54) AND atr/natr/supertrend declared their columns (2fd22d8) —
    // otherwise `merge` would reject the arena outright — the silent positional
    // binding this replaced no longer exists as a fallback. Every recipe here is
    // column-declared, so High=0, Low=1, Close=2 by MEANING whatever the request
    // order.
    fn hub7_req() -> Request {
        Request::new()
            .add(Dm::recipe(&[HP]), None)
            .add(Di::recipe(&[HP]), None)
            .add(Dx::recipe(&[HP]), None)
            .add(Adx::recipe(&[HP]), None)
            .add(tulip_rs::indicators::atr::Atr::recipe(&[HP]), None)
            .add(Natr::recipe(&[HP]), None)
            .add(
                tulip_rs::indicators::supertrend::SuperTrend::recipe(&[HP, 3.0]),
                None,
            )
    }

    #[test]
    fn dm_di_dx_adx_atr_natr_supertrend_shared_p_sig_derived() {
        use tulip_rs::cse::kernel::registry::SIG_dm_di_dx_adx_atr_natr_supertrend_shared_p;
        let (_a, sig, _, _) = erase_request(&hub7_req()).unwrap();
        assert_eq!(
            sig, SIG_dm_di_dx_adx_atr_natr_supertrend_shared_p,
            "mega-hub arena drifted from the generated SIG"
        );
    }

    /// THE SHARING CLAIM, MEASURED. The registry comment asserts that `atr`,
    /// `natr` and `supertrend` add NO new recurrence because `adx` already grafts
    /// the identical mean-`Wilders(tr)` chain. A claim like that must not live
    /// only in prose: the seven-indicator arena has to hold the SAME `Wilders`
    /// slots as the four-indicator hub — two COMPOSITE (`+DM`/`−DM`) plus two MEAN
    /// (the ATR chain, and adx's smoothing over `dx`). Three extra indicators,
    /// zero extra accumulators, and the same (source, form) multiset proves they
    /// are the same interned nodes rather than merely the same count.
    #[test]
    fn dm_di_dx_adx_atr_natr_supertrend_interns_the_atr_chain_across_five_consumers() {
        const K_WILDERS: u8 = 4;
        let (_a, hub, _, _) = erase_request(&hub_req()).unwrap();
        let (_b, big, _, _) = erase_request(&hub7_req()).unwrap();

        let wilders = |sig: &tulip_rs::cse::Signature| -> Vec<(u32, u32)> {
            (0..sig.len as usize)
                .filter(|k| sig.kinds[*k] == K_WILDERS)
                .map(|k| (sig.a[k], sig.b[k]))
                .collect()
        };
        let (h, g) = (wilders(&hub), wilders(&big));
        assert_eq!(
            h.len(),
            4,
            "hub expected 2 composite + 2 mean accumulators, found {}",
            h.len()
        );
        assert_eq!(
            g.len(),
            h.len(),
            "the mega-hub added {} Wilders slot(s) where the registry comment promises \
             ZERO — atr/natr/supertrend must intern onto adx's existing chain",
            g.len() as i64 - h.len() as i64,
        );
        let (mut hs, mut gs) = (h, g);
        hs.sort();
        gs.sort();
        assert_eq!(
            gs, hs,
            "mega-hub Wilders nodes differ from the hub's — same count, different nodes"
        );

        // And no two Wilders may share a SOURCE while differing in FORM: the shape
        // the emitter still mishandles. Restated for the bigger arena, which is
        // where such a pair would most plausibly reappear as the family grows.
        let slots: Vec<usize> = (0..big.len as usize)
            .filter(|k| big.kinds[*k] == K_WILDERS)
            .collect();
        for i in &slots {
            for j in slots.iter() {
                if j <= i {
                    continue;
                }
                if big.a[*i] == big.a[*j] {
                    assert_eq!(
                        big.b[*i], big.b[*j],
                        "mega-hub slots {i}/{j} share a source with different Wilders \
                         forms — the open emitter defect would fire"
                    );
                }
            }
        }
    }

    /// Primary rows against all SEVEN hands. `dm`/`dx`/`adx`/`atr`/`natr`/
    /// `supertrend` bit-exact; `di` RELATIVE to 1e-12 (mean-chain scale — see the
    /// `di` doc). The ATR row is the load-bearing one: byte-identical to the
    /// standalone hand ATR inside an arena where five consumers read that node.
    #[test]
    fn dm_di_dx_adx_atr_natr_supertrend_shared_p_bit_exact() {
        for base in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let hi: Vec<f64> = base.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = base.iter().map(|v| v - 1.0).collect();
            let two = [hi.as_slice(), lo.as_slice()];
            let three = [hi.as_slice(), lo.as_slice(), base.as_slice()];
            let out = run_registered(&hub7_req(), &three).expect("mega-hub registered");

            let (dm, _) = Dm::indicator(&two, &[HP], None).unwrap();
            let (di, _) = Di::indicator(&three, &[HP], None).unwrap();
            let (dx, _) = Dx::indicator(&three, &[HP], None).unwrap();
            let (adx, _) = Adx::indicator(&three, &[HP], None).unwrap();
            let (atr, _) = tulip_rs::indicators::atr::Atr::indicator(&three, &[HP], None).unwrap();
            let (natr, _) = Natr::indicator(&three, &[HP], None).unwrap();
            let (st, _) =
                tulip_rs::indicators::supertrend::SuperTrend::indicator(&three, &[HP, 3.0], None)
                    .unwrap();

            assert_bits(row_for(&out, 0, true, 0), &dm[0], "hub7: +DM");
            assert_bits(row_for(&out, 0, true, 1), &dm[1], "hub7: -DM");
            for idx in [0usize, 1] {
                assert_rel_1e12(
                    row_for(&out, 1, true, idx),
                    &di[idx],
                    &format!("hub7: DI {idx}"),
                );
            }
            assert_bits(row_for(&out, 2, true, 0), &dx[0], "hub7: DX");
            assert_bits(row_for(&out, 3, true, 0), &adx[0], "hub7: ADX");
            assert_bits(row_for(&out, 4, true, 0), &atr[0], "hub7: ATR");
            assert_bits(row_for(&out, 5, true, 0), &natr[0], "hub7: NATR");
            assert_bits(row_for(&out, 6, true, 0), &st[0], "hub7: SUPERTREND");
        }
    }
}
