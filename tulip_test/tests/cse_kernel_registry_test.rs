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
    use tulip_rs::indicators::apo::Apo;
    use tulip_rs::indicators::macd::Macd;
    use tulip_rs::indicators::ppo::Ppo;

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
}
