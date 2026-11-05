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
    use tulip_rs::indicators::highpass::HighPass;
    use tulip_rs::indicators::macd::Macd;
    use tulip_rs::indicators::mass::Mass;
    use tulip_rs::indicators::mom::Mom;
    use tulip_rs::indicators::natr::Natr;
    use tulip_rs::indicators::ppo::Ppo;
    use tulip_rs::indicators::roofingfilter::RoofingFilter;
    use tulip_rs::indicators::supersmoother::SuperSmoother;
    use tulip_rs::indicators::ultosc::Ultosc;
    use tulip_rs::indicators::volatility::Volatility;

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

    /// THE `Mom + Roc + Rocr` HUB — the sharing claim measured, not reasoned.
    /// All three bind `price()`, so the arena must hold ONE `Input`; roc and rocr each
    /// build `Lag{close, p}` with the same key → ONE `Lag`; roc's `Sub{close, prev}` is
    /// literally `mom`'s line → ONE `Sub`; and both denominators are
    /// `Max{prev, EPSILON}` → ONE `Max`. Two `Div`s are the only new arithmetic the
    /// third and second indicators add. Counts come from the arena; the assert message
    /// prints what was actually found, so a wrong expectation here is information
    /// rather than a silent pass.
    #[test]
    fn mom_roc_rocr_shared_interns_the_lag_the_mom_and_the_eps_floor() {
        use tulip_rs::cse::kernel::registry::SIG_mom_roc_rocr_shared_p;
        use tulip_rs::cse::signature::{K_ADD, K_DIV, K_INPUT, K_LAG, K_MAX, K_SUB};
        let req = Request::new()
            .add(tulip_rs::cse::recipes::mom(&[9.0]), None)
            .add(tulip_rs::cse::recipes::roc(&[9.0]), None)
            .add(tulip_rs::cse::recipes::rocr(&[9.0]), None);
        let (_a, sig, _v, owners) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_mom_roc_rocr_shared_p, "hub arena drifted");
        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        let (input, lag, sub, add, div, mx) = (
            count(K_INPUT),
            count(K_LAG),
            count(K_SUB),
            count(K_ADD),
            count(K_DIV),
            count(K_MAX),
        );
        assert_eq!(
            (input, lag, sub, add, div, mx),
            (1, 1, 1, 0, 2, 1),
            "arena observed input={input} lag={lag} sub={sub} add={add} div={div} \
             max={mx} — three price recipes should intern the close, the lag, the \
             mom Sub and the shared eps floor, adding only the two Divs"
        );
        // roc carries `mom` as an optional row and mom's own request is a primary, so
        // the arena must show BOTH for the same node.
        let mom_nodes = owners.iter().filter(|o| o.primary).count();
        assert_eq!(
            mom_nodes, 3,
            "three requested indicators publish three primaries"
        );
    }

    /// Same arena, bit-exact against the three HANDS — the proof the interning is
    /// identity and not a shared-but-wrong value.
    #[test]
    fn mom_roc_rocr_shared_bit_exact() {
        use tulip_rs::indicators::mom::Mom;
        use tulip_rs::indicators::roc::Roc;
        use tulip_rs::indicators::rocr::Rocr;
        let req = Request::new()
            .add(tulip_rs::cse::recipes::mom(&[9.0]), None)
            .add(tulip_rs::cse::recipes::roc(&[9.0]), None)
            .add(tulip_rs::cse::recipes::rocr(&[9.0]), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let streams = [s.as_slice()];
            let out = run_registered(&req, &streams).expect("mom+roc+rocr registered");
            let (m, _) = Mom::indicator(&streams, &[9.0], None).unwrap();
            let (r, _) = Roc::indicator(&streams, &[9.0], None).unwrap();
            let (q, _) = Rocr::indicator(&streams, &[9.0], None).unwrap();
            assert_bits(row_for(&out, 0, true, 0), &m[0], "hub: mom row");
            assert_bits(row_for(&out, 1, true, 0), &r[0], "hub: roc row");
            assert_bits(row_for(&out, 2, true, 0), &q[0], "hub: rocr row");
            // roc's `mom` optional mirror is in the arena but not wanted → empty.
            for (o, row) in out.owners.iter().zip(out.rows.iter()) {
                if !o.primary {
                    assert!(row.is_empty(), "disabled optional {o:?} must be empty");
                }
            }
        }
    }

    /// THE CALCULATED-INPUT HUB: `Medprice + Typprice + WcPrice` all build the same
    /// `Add{high, low}`, so the arena must hold exactly ONE of them feeding three
    /// sinks. This is the `E8` family where the same composite is otherwise computed
    /// once per indicator.
    #[test]
    fn calculated_inputs_share_one_hl_sum_across_three_indicators() {
        use tulip_rs::cse::kernel::registry::SIG_medprice_typprice_wcprice;
        use tulip_rs::cse::signature::{K_ADD, K_DIV, K_FMA, K_INPUT, K_MUL};
        let req = Request::new()
            .add(tulip_rs::cse::recipes::medprice(&[]), None)
            .add(tulip_rs::cse::recipes::typprice(&[]), None)
            .add(tulip_rs::cse::recipes::wcprice(&[]), None);
        let (_a, sig, _v, owners) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_medprice_typprice_wcprice, "arena drifted");
        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        let (input, add, mul, div, fma) = (
            count(K_INPUT),
            count(K_ADD),
            count(K_MUL),
            count(K_DIV),
            count(K_FMA),
        );
        assert_eq!(
            (input, add, mul, div, fma),
            (3, 2, 3, 0, 1),
            "arena observed input={input} add={add} mul={mul} div={div} fma={fma} — the \
             claim is that `high+low` is ONE shared Add feeding all three composites: \
             adds are hl_sum + typprice's `(h+l)+c` = 2, muls are the three scales \
             (0.5, 1/3, 0.25) = 3, and only wcprice contributes the Fma"
        );
        assert_eq!(
            owners.iter().filter(|o| o.primary).count(),
            3,
            "three primaries"
        );
    }

    #[test]
    fn calculated_inputs_bit_exact() {
        use tulip_rs::indicators::medprice::Medprice;
        use tulip_rs::indicators::typprice::Typprice;
        use tulip_rs::indicators::wcprice::WcPrice;
        let req = Request::new()
            .add(tulip_rs::cse::recipes::medprice(&[]), None)
            .add(tulip_rs::cse::recipes::typprice(&[]), None)
            .add(tulip_rs::cse::recipes::wcprice(&[]), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &streams).expect("composites registered");
            let (m, _) = Medprice::indicator(&[hi.as_slice(), lo.as_slice()], &[], None).unwrap();
            let (t, _) = Typprice::indicator(&streams, &[], None).unwrap();
            let (w, _) = WcPrice::indicator(&streams, &[], None).unwrap();
            assert_bits(row_for(&out, 0, true, 0), &m[0], "composites: medprice row");
            assert_bits(row_for(&out, 1, true, 0), &t[0], "composites: typprice row");
            assert_bits(row_for(&out, 2, true, 0), &w[0], "composites: wcprice row");
        }
    }

    /// THE FOUR LONE FLOORS OF THE SECOND TRANCHE, each hand called on ITS OWN
    /// arity. A registered row is reachable through `try_run`, so an ungated one is
    /// a latent wrong-value bug with a green suite -- the same class the column law
    /// closed. Each block also pins that the arena's stream count equals the hand's
    /// declared inputs: AvgPrice 4 (open,high,low,close), QStick 2 (open,close),
    /// Vwma 2 (close,volume), Vosc 1 (volume ONLY).
    #[test]
    fn calculated_tranche_lone_floors_bit_exact() {
        use tulip_rs::indicators::avgprice::AvgPrice;
        use tulip_rs::indicators::qstick::QStick;
        use tulip_rs::indicators::vosc::Vosc;
        use tulip_rs::indicators::vwma::Vwma;
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let op: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let vol = volume(s.len());

            let req_avg = Request::new().add(tulip_rs::cse::recipes::avgprice(&[]), None);
            let four = [op.as_slice(), hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req_avg, &four).expect("avgprice registered");
            let (a, _) = AvgPrice::indicator(&four, &[], None).unwrap();
            assert_bits(row_for(&out, 0, true, 0), &a[0], "avgprice row");

            let req_qs = Request::new().add(tulip_rs::cse::recipes::qstick(&[9.0]), None);
            let two_oc = [op.as_slice(), s.as_slice()];
            let out = run_registered(&req_qs, &two_oc).expect("qstick registered");
            let (q, _) = QStick::indicator(&two_oc, &[9.0], None).unwrap();
            assert_bits(row_for(&out, 0, true, 0), &q[0], "qstick row");

            let req_vw = Request::new().add(tulip_rs::cse::recipes::vwma(&[9.0]), None);
            let two_cv = [s.as_slice(), vol.as_slice()];
            let out = run_registered(&req_vw, &two_cv).expect("vwma registered");
            let (v, _) = Vwma::indicator(&two_cv, &[9.0], None).unwrap();
            assert_bits(row_for(&out, 0, true, 0), &v[0], "vwma row");

            let req_vo = Request::new().add(tulip_rs::cse::recipes::vosc(&[3.0, 5.0]), None);
            let one_v = [vol.as_slice()];
            let out = run_registered(&req_vo, &one_v).expect("vosc registered");
            let (o, _) = Vosc::indicator(&one_v, &[3.0, 5.0], None).unwrap();
            assert_bits(row_for(&out, 0, true, 0), &o[0], "vosc row");
        }

        // THE ZERO-VOLUME BLOCK. This is the ONLY input that reaches the hand's
        // `if denom == 0.0 { return 0.0 }` guard, and therefore the only thing that
        // tells a correct guard from an inverted one: positive prices and volumes
        // pass BOTH spellings, so the loop above alone would have certified the
        // buggy draft. The first draft spelled it `WhereGe{a: denom, b: 0, t: num}`
        // FEEDING the divide, which takes the value arm exactly at zero and computes
        // `num / 0.0` -- inf or NaN where the hand prints 0.0. The rewrite guards the
        // RESULT with `WhereLe{ Abs(denom) <= 0 }`, whose NaN behaviour also matches
        // (a NaN compare is false for `<=`, so the value arm runs, as in the hand).
        {
            let zero_vol = vec![0.0; 64];
            let prices: Vec<f64> = (0..64).map(|i| 100.0 + i as f64).collect();

            let req_vw = Request::new().add(tulip_rs::cse::recipes::vwma(&[5.0]), None);
            let two_cv = [prices.as_slice(), zero_vol.as_slice()];
            let out = run_registered(&req_vw, &two_cv).expect("vwma zero-volume");
            let (v, _) = Vwma::indicator(&two_cv, &[5.0], None).unwrap();
            assert_bits(
                row_for(&out, 0, true, 0),
                &v[0],
                "vwma ZERO-VOLUME row (guards the denom==0 branch)",
            );

            let req_vo = Request::new().add(tulip_rs::cse::recipes::vosc(&[3.0, 5.0]), None);
            let one_v = [zero_vol.as_slice()];
            let out = run_registered(&req_vo, &one_v).expect("vosc zero-volume");
            let (o, _) = Vosc::indicator(&one_v, &[3.0, 5.0], None).unwrap();
            assert_bits(
                row_for(&out, 0, true, 0),
                &o[0],
                "vosc ZERO-VOLUME row (guards the denom==0 branch)",
            );
        }
    }

    /// BOP'S OWN GATE. `bop_n: Bop();` (`registry.rs:316`) is a registered, reachable
    /// row and until this test it was the ONLY registered pattern with no hand-parity
    /// gate anywhere in the engine suites: every `bop` match under `tulip_test/tests/`
    /// lives in `bop_test.rs`, which compares the HAND against tulip C, never against a
    /// kernel. That leaves the row's stated purpose unproven -- `registry.rs:312-315`
    /// calls it "the live proof that `ohlc()`'s canonical ranking binds
    /// open=0/high=1/low=2/close=3 instead of silently reading a neighbouring column",
    /// and a ranking bug is exactly what this file cannot otherwise see. All four
    /// columns are therefore DISTINCT series here, so any mis-ranked slot changes every
    /// published value.
    #[test]
    fn bop_lone_floor_bit_exact() {
        use tulip_rs::indicators::bop::Bop;
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            // open, high, low and close deliberately differ, so a column-rank slip
            // (close read as high, say) moves every bar, not just some.
            let op: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();

            let four = [op.as_slice(), hi.as_slice(), lo.as_slice(), s.as_slice()];
            let req = Request::new().add(tulip_rs::cse::recipes::bop(&[]), None);
            let out = run_registered(&req, &four).expect("bop registered");
            let (b, _) = Bop::indicator(&four, &[], None).unwrap();
            assert_bits(row_for(&out, 0, true, 0), &b[0], "bop row");
        }

        // THE FLAT-BAR BLOCK. `high == low` makes the hand's
        // `(high - low).max(f64::EPSILON)` the denominator, so the row prints
        // `(close - open) / EPSILON` -- an enormous finite value, NOT 0.0 and NOT NaN.
        // Only `Node::Max` reproduces that: the `Where*` form an earlier draft reached
        // for pins 0.0 on the guarded case and diverges on every flat bar, and a
        // finite-price matrix that never flattens `high - low` cannot tell the two
        // spellings apart.
        {
            let n = 64;
            let flat = series(n, |_| 100.0);
            let op: Vec<f64> = flat.iter().map(|v| v - 0.25).collect();
            let cl: Vec<f64> = flat.iter().map(|v| v + 0.25).collect();
            let four = [
                op.as_slice(),
                flat.as_slice(),
                flat.as_slice(),
                cl.as_slice(),
            ];
            let req = Request::new().add(tulip_rs::cse::recipes::bop(&[]), None);
            let out = run_registered(&req, &four).expect("bop flat-bar registered");
            let (b, _) = Bop::indicator(&four, &[], None).unwrap();
            assert_bits(
                row_for(&out, 0, true, 0),
                &b[0],
                "bop FLAT-BAR row (guards the EPSILON floor)",
            );
        }
    }

    /// `Ad + ChaikinMf` share one arena. The expected count is DERIVED from the two
    /// graphs, not observed and back-filled: `intern` keys on the whole `Node`
    /// (`merge.rs:576`), so the pair must intern the four `Input` columns, the
    /// `Sub{high, low}` range AND the `Sub{close, low}` difference (both graphs build
    /// them over the same global children), plus the one `EPSILON` `Const` (its `bits`
    /// are part of the key, which is why it dedups even though `erase` drops the value
    /// from the Signature, `signature.rs:351-353`).
    /// The two money-flow NUMERATORS stay separate on purpose: the hand's
    /// `(close-low) - (high-close)` versus `ad_nodes`' `((close-low) - high) + close`.
    /// Different association ⇒ different bits, so a count that merged them would mean
    /// one of the two was transcribed wrong.
    #[test]
    fn ad_chaikinmf_shared_interns_range_and_eps_but_not_the_numerators() {
        use tulip_rs::cse::kernel::registry::SIG_ad_chaikinmf_shared;
        use tulip_rs::cse::signature::{
            K_ACCUM, K_ADD, K_CONST, K_DIV, K_INPUT, K_MAX, K_MUL, K_ROLLSUM, K_SUB, K_WHERELE,
        };
        let req = Request::new()
            .add(tulip_rs::cse::recipes::ad(&[]), None)
            .add(tulip_rs::cse::recipes::chaikinmf(&[9.0]), None);
        let (_a, sig, _v, _owners) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_ad_chaikinmf_shared, "hub arena drifted");

        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        let got = (
            count(K_INPUT),
            count(K_SUB),
            count(K_ADD),
            count(K_DIV),
            count(K_MAX),
            count(K_MUL),
            count(K_WHERELE),
            count(K_ACCUM),
            count(K_CONST),
            count(K_ROLLSUM),
        );
        // sub=5: ad's range, cl-lo, (cl-lo)-hi  +  cmf's hi-cl, its own numerator.
        // div=3: ad's ratio, cmf's clv, cmf's window ratio.   rollsum=2: the two windows.
        assert_eq!(
            got,
            (4, 5, 1, 3, 1, 1, 1, 1, 1, 2),
            "arena observed input/sub/add/div/max/mul/wherele/accum/const/rollsum = {got:?}"
        );
    }

    /// Bit-exact against BOTH hands, including the two cases that separate a correct
    /// transcription from a plausible one:
    /// * `high == low` — forces the per-bar `mfv` denominator into `Max(range, EPSILON)`.
    /// * all-zero volume — the hand's final `sums[0] / sums[1]` is UNGUARDED, so it
    ///   prints `0.0 / 0.0` = NaN. `vwma` and `vosc` both guard their ratio; copying that
    ///   neighbour here would print 0.0 and still pass any finite-data matrix.
    #[test]
    fn ad_chaikinmf_shared_bit_exact() {
        use tulip_rs::indicators::ad::Ad;
        use tulip_rs::indicators::chaikinmf::ChaikinMf;
        const P: f64 = 9.0;

        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let vol = volume(s.len());
            // column order is the hand's Info: high, low, close, volume.
            let four = [hi.as_slice(), lo.as_slice(), s.as_slice(), vol.as_slice()];
            let req = Request::new()
                .add(tulip_rs::cse::recipes::ad(&[]), None)
                .add(tulip_rs::cse::recipes::chaikinmf(&[P]), None);
            let out = run_registered(&req, &four).expect("ad + chaikinmf registered");

            let (a, _) = Ad::indicator(&four, &[], None).unwrap();
            assert_bits(row_for(&out, 0, true, 0), &a[0], "ad row in the cmf hub");
            let (c, _) = ChaikinMf::indicator(&four, &[P], None).unwrap();
            assert_bits(row_for(&out, 1, true, 0), &c[0], "chaikinmf row");
        }

        // FLAT RANGE: high == low every bar, so `(high-low)` is exactly 0.0 and the
        // epsilon floor is the whole denominator.
        {
            let n = 200;
            let px = series(n, |i| 100.0 + (i % 7) as f64);
            let vol = volume(n);
            let four = [px.as_slice(), px.as_slice(), px.as_slice(), vol.as_slice()];
            let req = Request::new()
                .add(tulip_rs::cse::recipes::ad(&[]), None)
                .add(tulip_rs::cse::recipes::chaikinmf(&[P]), None);
            let out = run_registered(&req, &four).expect("ad + chaikinmf flat-range");
            let (a, _) = Ad::indicator(&four, &[], None).unwrap();
            assert_bits(
                row_for(&out, 0, true, 0),
                &a[0],
                "ad row on HIGH == LOW (epsilon floor)",
            );
            let (c, _) = ChaikinMf::indicator(&four, &[P], None).unwrap();
            assert_bits(
                row_for(&out, 1, true, 0),
                &c[0],
                "chaikinmf row on HIGH == LOW (epsilon floor)",
            );
            // NON-VACUITY: with high == low == close the numerator is exactly 0.0, so the
            // hand prints `0.0 / EPSILON` = 0.0 and NOT `0.0 / 0.0`. A recipe that omitted
            // the `Max` floor would print NaN here and fail above; a test that never
            // reached the floor would pass either way. Pinning a zero (never a NaN) makes
            // this block evidence that the floor was actually hit.
            assert!(
                c[0].iter().all(|v| *v == 0.0),
                "chaikinmf on HIGH==LOW==CLOSE must print the epsilon floor's 0.0, got \
                 {:?}",
                &c[0][..c[0].len().min(4)]
            );
        }

        // ZERO VOLUME: the unguarded divide must stay unguarded.
        {
            let n = 200;
            let hi = series(n, |i| 101.0 + (i % 3) as f64);
            let lo = series(n, |i| 99.0 + (i % 3) as f64);
            let cl = series(n, |i| 100.0 + (i % 3) as f64);
            let zero_vol = vec![0.0; n];
            let four = [
                hi.as_slice(),
                lo.as_slice(),
                cl.as_slice(),
                zero_vol.as_slice(),
            ];
            let req = Request::new()
                .add(tulip_rs::cse::recipes::ad(&[]), None)
                .add(tulip_rs::cse::recipes::chaikinmf(&[P]), None);
            let out = run_registered(&req, &four).expect("ad + chaikinmf zero-volume");
            let (c, _) = ChaikinMf::indicator(&four, &[P], None).unwrap();
            assert_bits(
                row_for(&out, 1, true, 0),
                &c[0],
                "chaikinmf ZERO-VOLUME row (the hand prints 0/0 = NaN, not 0.0)",
            );
            // NON-VACUITY, same discipline as the flat-range block: the oracle row must
            // actually BE NaN here. If it were finite, this block would prove nothing about
            // the missing guard and `assert_bits` would pass for an uninteresting reason.
            assert!(
                c[0].iter().all(|v| v.is_nan()),
                "zero volume must drive the hand's unguarded `sums[0] / sums[1]` to NaN, got \
                 {:?}",
                &c[0][..c[0].len().min(4)]
            );
        }
    }

    /// VHF'S FIRST-EVER ARENA LOOK. `recipes::vhf` existed with a `Planable` impl but
    /// had NO planner arm and NO row, so this is the first time the graph was ever
    /// erased into a Signature -- nothing had ever checked it.
    ///
    /// The counts are DERIVED from the three graphs before running, which is the whole
    /// point of the hub: `max` and `min` each build one ring over the close column, and
    /// `vhf` builds the SAME two `RollMax{src: real, period, look_back: period-1}` /
    /// `RollMin{..}` nodes itself. `intern` keys on the whole `Node`, so all three
    /// consumers share ONE pair of rescans. Three separate arenas would total
    /// 2 + 2 + 11 = 15 nodes; the merged arena must total 11, with exactly one RollMax
    /// and one RollMin. If either count came out at 2, the "hub" is three floors wearing
    /// a trenchcoat and the row would be pure overhead.
    #[test]
    fn max_min_vhf_shared_interns_both_rings() {
        use tulip_rs::cse::kernel::registry::SIG_max_min_vhf_shared_p;
        use tulip_rs::cse::signature::{
            K_ABS, K_CONST, K_DIV, K_INPUT, K_LAG, K_MAX, K_ROLLMAX, K_ROLLMIN, K_ROLLSUM, K_SUB,
        };
        let req = Request::new()
            .add(tulip_rs::cse::recipes::max(&[9.0]), None)
            .add(tulip_rs::cse::recipes::min(&[9.0]), None)
            .add(tulip_rs::cse::recipes::vhf(&[9.0]), None);
        let (_a, sig, _v, owners) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_max_min_vhf_shared_p, "hub arena drifted");

        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        let got = (
            count(K_INPUT),
            count(K_ROLLMAX),
            count(K_ROLLMIN),
            count(K_LAG),
            count(K_SUB),
            count(K_ABS),
            count(K_ROLLSUM),
            count(K_CONST),
            count(K_MAX),
            count(K_DIV),
        );
        assert_eq!(
            got,
            (1, 1, 1, 1, 2, 1, 1, 1, 1, 1),
            "arena observed input/rollmax/rollmin/lag/sub/abs/rollsum/const/max/div = {got:?} \
             — the two extrema rings must appear ONCE each, shared by all three consumers"
        );
        assert_eq!(
            owners.iter().filter(|o| o.primary).count(),
            3,
            "three requested indicators publish three primaries"
        );
    }

    /// ...and the sharing is identity, not a shared-but-wrong value: every row bit-exact
    /// against its own HAND. The shapes include a FLAT series, because a rolling extremum
    /// is where cold-seed asymmetry hides -- on a trend the carried max/min self-corrects
    /// and a phantom seed is invisible, while flat data pins it. VHF is also the first
    /// consumer that reads BOTH extrema and their difference, so a wrong seed shows up in
    /// `range` rather than being divided away.
    #[test]
    fn max_min_vhf_shared_bit_exact() {
        use tulip_rs::indicators::max::Max;
        use tulip_rs::indicators::min::Min;
        use tulip_rs::indicators::vhf::Vhf;
        const P: f64 = 9.0;

        // The FLAT shape is listed with `must_move = false` on purpose, and the first run
        // taught why: on a constant series every abs-diff is 0 and `max == min`, so
        // `range / max(sum, EPSILON)` is `0 / EPSILON` = 0.0 for EVERY bar -- zero is the
        // CORRECT oracle, not an empty kernel. Pinning non-zero there would have failed on
        // a transcription that is actually right. The other three shapes must produce a
        // non-zero row, which is what separates "matches" from "both printed nothing".
        for (name, s, must_move) in [
            ("wiggle", wiggle(), true),
            ("ramp", series(680, |i| 100.0 + i as f64 * 0.05), true),
            ("flat", series(700, |_| 100.0), false),
            // monotone: the extremum is always the newest bar, so the carried value and
            // the rescan agree by construction and cannot mask a bad cold seed.
            ("monotone", series(400, |i| i as f64), true),
        ] {
            let one = [s.as_slice()];
            let req = Request::new()
                .add(tulip_rs::cse::recipes::max(&[P]), None)
                .add(tulip_rs::cse::recipes::min(&[P]), None)
                .add(tulip_rs::cse::recipes::vhf(&[P]), None);
            let out = run_registered(&req, &one).expect("max + min + vhf registered");

            let (mx, _) = Max::indicator(&one, &[P], None).unwrap();
            assert_bits(
                row_for(&out, 0, true, 0),
                &mx[0],
                &format!("max row [{name}]"),
            );
            let (mn, _) = Min::indicator(&one, &[P], None).unwrap();
            assert_bits(
                row_for(&out, 1, true, 0),
                &mn[0],
                &format!("min row [{name}]"),
            );
            let (v, _) = Vhf::indicator(&one, &[P], None).unwrap();
            let vhf_row = row_for(&out, 2, true, 0);
            assert_bits(vhf_row, &v[0], &format!("vhf row [{name}]"));

            if must_move {
                // Non-vacuity, checked against the ORACLE so the claim is about the data,
                // not about the kernel: this shape must make the hand print something
                // other than zero, so the equality above carried information.
                assert!(
                    v[0].iter().any(|x| *x != 0.0),
                    "the HAND's vhf row on {name} is all zeros -- the shape proves nothing"
                );
                assert!(
                    vhf_row.iter().any(|x| *x != 0.0),
                    "kernel vhf row on {name} is all zeros while the hand is not"
                );
            }
        }
    }

    /// VWAP + TYPPRICE: the calculated-inputs hub, measured. The hand `vwap.rs:44` calls
    /// `calc_typprice`, and `recipes::vwap` spells that subgraph node-for-node the way
    /// `recipes::typprice` spells it, so `intern` (keyed on the whole `Node`) must merge
    /// them -- and the column-declared remap is what makes it legal: typprice's
    /// `Input{0}` is HIGH and vwap's `Input{0}` is HIGH too, so the two `Add{high, low}`
    /// nodes are genuinely the same computation.
    ///
    /// The count is written to DISTINGUISH shared from un-shared, which is the whole point:
    /// if the subgraph did NOT intern, vwap would carry its own `Add` x2, its own `Mul` for
    /// tp, and its own `1/3` Const -- giving `add=4, mul=3, const=3, len=16`. Interning
    /// predicts `add=2, mul=2, const=2, len=13`. A test that asserted only "13" without
    /// deriving it would be a number read off a green run.
    #[test]
    fn vwap_typprice_shared_interns_the_typical_price_subgraph() {
        use tulip_rs::cse::kernel::registry::SIG_vwap_typprice_shared;
        use tulip_rs::cse::signature::{K_ACCUM, K_ADD, K_CONST, K_DIV, K_INPUT, K_MUL};
        let req = Request::new()
            .add(tulip_rs::cse::recipes::vwap(&[]), Some(&[true]))
            .add(tulip_rs::cse::recipes::typprice(&[]), None);
        let (arena, sig, _v, _owners) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_vwap_typprice_shared, "hub arena drifted");

        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        let got = (
            count(K_INPUT),
            count(K_ADD),
            count(K_MUL),
            count(K_CONST),
            count(K_ACCUM),
            count(K_DIV),
        );
        assert_eq!(
            got,
            (4, 2, 2, 2, 2, 1),
            "arena observed input/add/mul/const/accum/div = {got:?} -- add=4 or const=3 would \
             mean the typical-price chain was duplicated, not interned"
        );
        // 4 inputs + 2 adds + 2 muls + 2 consts + 2 accums + 1 div = 13.
        assert_eq!(
            arena.nodes.len(),
            sig.len as usize,
            "arena length must equal the erased node count"
        );
        assert_eq!(sig.len, 13, "expected 13 nodes, got {}", sig.len);
    }

    /// Bit-exact against BOTH hands, and the row-sharing proof: vwap's OPTIONAL
    /// `typprice` row and Typprice's PRIMARY row are the same arena node, so they must be
    /// bit-identical to each other AND to each hand's own typical price. Three independent
    /// comparisons -- two hands and the aliasing -- because an interned-but-wrong node
    /// would still alias perfectly against itself.
    ///
    /// The zero-volume block pins that vwap's `pv_sum / vol_sum` stays UNGUARDED: the hand
    /// has no `if vol_sum == 0`, so every bar must be `0.0 / 0.0` = NaN. `vwma` and `vosc`
    /// both guard their ratios, so a guard copied from a neighbour would print 0.0 and pass
    /// every finite-volume shape.
    #[test]
    fn vwap_typprice_shared_bit_exact() {
        use tulip_rs::indicators::typprice::Typprice;
        use tulip_rs::indicators::vwap::Vwap;

        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let vol = volume(s.len());
            let four = [hi.as_slice(), lo.as_slice(), s.as_slice(), vol.as_slice()];
            let three = [hi.as_slice(), lo.as_slice(), s.as_slice()];

            let req = Request::new()
                .add(tulip_rs::cse::recipes::vwap(&[]), Some(&[true]))
                .add(tulip_rs::cse::recipes::typprice(&[]), None);
            let out = run_registered(&req, &four).expect("vwap + typprice registered");

            let (v, _) = Vwap::indicator(&four, &[], Some(&[true])).unwrap();
            assert_bits(row_for(&out, 0, true, 0), &v[0], "vwap row");
            // vwap's optional typprice, against the typprice HAND and against the
            // Typprice indicator's own primary row in the SAME arena.
            let (t_self, _) = Typprice::indicator(&three, &[], None).unwrap();
            let engine_tp_opt = row_for(&out, 0, false, 0);
            let engine_tp_pri = row_for(&out, 1, true, 0);
            assert_bits(engine_tp_opt, &v[1], "vwap's optional typprice vs its hand");
            assert_bits(engine_tp_pri, &t_self[0], "typprice primary vs its hand");
            assert_bits(
                engine_tp_opt,
                engine_tp_pri,
                "the two sinks of the interned tp node must be the same bits",
            );
        }

        // ZERO VOLUME -> the unguarded divide.
        {
            let n = 200;
            let hi = series(n, |i| 101.0 + (i % 3) as f64);
            let lo = series(n, |i| 99.0 + (i % 3) as f64);
            let cl = series(n, |i| 100.0 + (i % 3) as f64);
            let zero_vol = vec![0.0; n];
            let four = [
                hi.as_slice(),
                lo.as_slice(),
                cl.as_slice(),
                zero_vol.as_slice(),
            ];
            let req = Request::new()
                .add(tulip_rs::cse::recipes::vwap(&[]), Some(&[true]))
                .add(tulip_rs::cse::recipes::typprice(&[]), None);
            let out = run_registered(&req, &four).expect("vwap zero-volume");
            let (v, _) = Vwap::indicator(&four, &[], Some(&[true])).unwrap();
            assert_bits(
                row_for(&out, 0, true, 0),
                &v[0],
                "vwap ZERO-VOLUME row (the hand prints 0/0 = NaN, not 0.0)",
            );
            // NON-VACUITY: the oracle must actually be NaN, else this block tests nothing
            // about the missing guard (same discipline as the chaikinmf zero-volume pin).
            assert!(
                v[0].iter().all(|x| x.is_nan()),
                "zero volume must drive the hand's unguarded `pv_sum / vol_sum` to NaN, got \
                 {:?}",
                &v[0][..v[0].len().min(4)]
            );
            // The interned tp row is INDEPENDENT of volume, so it must still be finite and
            // correct here -- a guard that bled into the shared node would show up on this
            // row too, and this is the only shape that separates the two.
            let (t_self, _) =
                Typprice::indicator(&[hi.as_slice(), lo.as_slice(), cl.as_slice()], &[], None)
                    .unwrap();
            assert_bits(
                row_for(&out, 1, true, 0),
                &t_self[0],
                "typprice row on ZERO VOLUME (shared node must stay finite)",
            );
        }
    }

    /// SMA + SMAENVELOPE: the envelope's middle band IS the standalone `Sma` node, so the
    /// pair must carry ONE roll. Counts DERIVED from the two graphs: the envelope needs
    /// input + `Sma` + `Const(pct)` + `Mul(step)` + `Sub(lower)` + `Add(upper)` = 6 nodes,
    /// and `sma` contributes only input + `Sma` (both already in that set). So interning
    /// predicts `K_SMA = 1, len = 6`; a duplicated roll predicts `K_SMA = 2, len = 7`.
    ///
    /// `n_sinks` is asserted too (4 = Sma's primary + the envelope's three), because this
    /// is the first hub whose second consumer publishes THREE primaries -- a sink-table bug
    /// would show up as a count, not as a wrong value.
    #[test]
    fn sma_smaenvelope_shared_interns_one_roll() {
        use tulip_rs::cse::kernel::registry::SIG_sma_smaenvelope_shared_p;
        use tulip_rs::cse::signature::{K_ADD, K_CONST, K_INPUT, K_MUL, K_SMA, K_SUB};
        let req = Request::new()
            .add(tulip_rs::cse::recipes::sma(&[9.0]), None)
            .add(tulip_rs::cse::recipes::smaenvelope(&[9.0, 5.0]), None);
        let (arena, sig, _v, _owners) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_sma_smaenvelope_shared_p, "hub arena drifted");

        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        let got = (
            count(K_INPUT),
            count(K_SMA),
            count(K_CONST),
            count(K_MUL),
            count(K_SUB),
            count(K_ADD),
        );
        assert_eq!(
            got,
            (1, 1, 1, 1, 1, 1),
            "arena observed input/sma/const/mul/sub/add = {got:?} -- sma=2 (len 7) would mean \
             the roll was duplicated instead of shared"
        );
        assert_eq!(arena.nodes.len(), 6, "six nodes expected");
        assert_eq!(
            sig.n_sinks, 4,
            "sma's primary + the envelope's lower/middle/upper"
        );
    }

    /// Bit-exact against BOTH hands, on the hub AND on the lone floor (`smaenvelope_n` is a
    /// separately compiled kernel, so the hub passing does not certify it), plus the
    /// aliasing claim: the envelope's middle row and standalone Sma's row are the same bits
    /// because they ARE the same node.
    ///
    /// The band-opening check is the non-vacuity guard: if `step` were wired to the wrong
    /// source (say the raw close instead of the roll), lower/middle/upper could still be
    /// self-consistent within the kernel yet differ from the hand -- and if the three sinks
    /// were pushed in the wrong order, comparing rows to rows would pass while BOTH were
    /// mislabelled. Requiring `lower < middle < upper` at every bar of a positive-price
    /// series pins the arithmetic to the bands' actual meaning.
    #[test]
    fn sma_smaenvelope_bit_exact_on_hub_and_floor() {
        use tulip_rs::indicators::sma::Sma;
        use tulip_rs::indicators::smaenvelope::SmaEnvelope;
        const P: f64 = 9.0;
        const M: f64 = 5.0;

        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let one = [s.as_slice()];
            let (sm, _) = Sma::indicator(&one, &[P], None).unwrap();
            let (e, _) = SmaEnvelope::indicator(&one, &[P, M], None).unwrap();
            // Info order: lower, middle, upper.
            assert!(
                e[0].iter().zip(&e[1]).all(|(l, m_)| l < m_)
                    && e[2].iter().zip(&e[1]).all(|(u, m_)| u > m_),
                "the HAND's bands are not ordered lower<middle<upper -- check the shape"
            );

            // THE HUB: two indicators, one roll.
            let hub = Request::new()
                .add(tulip_rs::cse::recipes::sma(&[P]), None)
                .add(tulip_rs::cse::recipes::smaenvelope(&[P, M]), None);
            let out = run_registered(&hub, &one).expect("sma + smaenvelope registered");
            assert_bits(row_for(&out, 0, true, 0), &sm[0], "sma row in the hub");
            assert_bits(row_for(&out, 1, true, 0), &e[0], "envelope lower [hub]");
            assert_bits(row_for(&out, 1, true, 1), &e[1], "envelope middle [hub]");
            assert_bits(row_for(&out, 1, true, 2), &e[2], "envelope upper [hub]");
            assert_bits(
                row_for(&out, 1, true, 1),
                row_for(&out, 0, true, 0),
                "the middle band and standalone sma are the same node, so same bits",
            );

            // THE LONE FLOOR: the separately compiled `smaenvelope_n` kernel.
            let floor = Request::new().add(tulip_rs::cse::recipes::smaenvelope(&[P, M]), None);
            let out = run_registered(&floor, &one).expect("smaenvelope floor registered");
            assert_bits(row_for(&out, 0, true, 0), &e[0], "envelope lower [floor]");
            assert_bits(row_for(&out, 0, true, 1), &e[1], "envelope middle [floor]");
            assert_bits(row_for(&out, 0, true, 2), &e[2], "envelope upper [floor]");
        }
    }

    /// TR + OBV: the column law proven in the ACCEPTANCE direction. `tr` binds close to
    /// stream 2 (`hlc()`), `obv` binds close to stream 0 (`cvol()`) -- the same COLUMN under
    /// two different indices. The merge renumbers by column before interning, so the arena
    /// must hold ONE `Lag{close, 1}`.
    ///
    /// Derived before running: 4 merged `Input`s (H,L,C,V); tr contributes `range`, `h_pc`,
    /// `l_pc` (3 Sub), 2 Abs, 2 Max; obv contributes 2 `WhereGt`, its own `delta` Sub, one
    /// `Const(1.0)`, one `Accum`. Total 4+1+4+2+2+2+1+1 = 17 with K_LAG = 1. Had the two
    /// closes NOT interned: K_LAG = 2 and 18 nodes. Every earlier test of the column law
    /// pinned the REJECTION half (merge panics when a recipe declares no columns); this is
    /// the acceptance half, and it is the half that makes cross-convention pairings legal.
    #[test]
    fn tr_obv_shared_interns_the_prev_close_lag_across_column_subsets() {
        use tulip_rs::cse::kernel::registry::SIG_tr_obv_shared;
        use tulip_rs::cse::signature::{
            K_ABS, K_ACCUM, K_CONST, K_INPUT, K_LAG, K_MAX, K_SUB, K_WHEREGT,
        };
        let req = Request::new()
            .add(tulip_rs::cse::recipes::tr(&[]), None)
            .add(tulip_rs::cse::recipes::obv(&[]), None);
        let (arena, sig, _v, _owners) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_tr_obv_shared, "hub arena drifted");

        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        assert_eq!(
            count(K_LAG),
            1,
            "prev-close lag must be SHARED across the two column subsets"
        );
        let got = (
            count(K_INPUT),
            count(K_LAG),
            count(K_SUB),
            count(K_ABS),
            count(K_MAX),
            count(K_WHEREGT),
            count(K_CONST),
            count(K_ACCUM),
        );
        assert_eq!(
            got,
            (4, 1, 4, 2, 2, 2, 1, 1),
            "arena observed input/lag/sub/abs/max/wheregt/const/accum = {got:?}"
        );
        assert_eq!(
            arena.nodes.len(),
            18,
            "18 nodes if the closes interned, 19 if not (17/18 before A5's tr Mirror)"
        );
    }

    /// Bit-exact against both hands on the hub, plus the lone `obv_n` floor, on shapes that
    /// each pin a DIFFERENT arm of the hand's three-way branch:
    /// * monotone up / down -- the `+= volume` and `-= volume` arms;
    /// * a PLATEAU (close repeats) -- the arm with NO `else`: the hand leaves the carry
    ///   untouched, and `0 - 0 = 0` is the only thing that can match it. A recipe that
    ///   treated equal closes as "down" would walk the line away from the hand here and
    ///   nowhere else, so this shape is the gate for that mistake;
    /// * FLAT -- the whole row must be 0.0, which is also the non-vacuity check for the
    ///   plateau case (a frozen carry looks identical to a zero carry on flat data alone).
    /// The NaN block is the float-class sweep the finite matrices cannot make. What it
    /// pins CHANGED when the run contradicted me: I first wrote that a NaN close freezes
    /// the line for the rest of the series, and the HAND REFUTED IT -- `prev_close = close`
    /// is assigned unconditionally, so a NaN suppresses exactly two bars (the NaN bar, then
    /// the bar whose `prev_close` is NaN) and then resumes. The engine matches it because
    /// `Lag` re-reads the stream rather than carrying `prev`, so it suppresses the same two
    /// rows by construction. `run_registered` on the lone floor only -- pairing it with
    /// `tr` here would also be testing `Max`'s NaN select, a different claim.
    #[test]
    fn tr_obv_bit_exact_on_hub_and_floors() {
        use tulip_rs::indicators::obv::Obv;
        use tulip_rs::indicators::tr::Tr;

        for (name, s) in [
            ("up", series(400, |i| i as f64)),
            ("down", series(400, |i| 400.0 - i as f64)),
            ("plateau", series(400, |i| 100.0 + ((i / 3) % 2) as f64)),
            ("flat", series(400, |_| 100.0)),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let vol = volume(s.len());
            let four = [hi.as_slice(), lo.as_slice(), s.as_slice(), vol.as_slice()];
            let hlc = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let cv = [s.as_slice(), vol.as_slice()];

            let hub = Request::new()
                .add(tulip_rs::cse::recipes::tr(&[]), None)
                .add(tulip_rs::cse::recipes::obv(&[]), None);
            let out = run_registered(&hub, &four).expect("tr + obv registered");
            let (t, _) = Tr::indicator(&hlc, &[], None).unwrap();
            assert_bits(
                row_for(&out, 0, true, 0),
                &t[0],
                &format!("tr row [{name}]"),
            );
            let (o, _) = Obv::indicator(&cv, &[], None).unwrap();
            let eng = row_for(&out, 1, true, 0);
            assert_bits(eng, &o[0], &format!("obv row [{name}]"));

            // Same values out of the lone floor kernel (a different code path).
            let floor = Request::new().add(tulip_rs::cse::recipes::obv(&[]), None);
            let out = run_registered(&floor, &cv).expect("obv floor registered");
            assert_bits(
                row_for(&out, 0, true, 0),
                &o[0],
                &format!("obv row [{name}] (floor)"),
            );

            if name == "plateau" {
                // NON-VACUITY for the equal-close arm: the row must actually contain both
                // a change and a no-change, or the shape never reached what it claims to
                // test.
                let frozen = o[0].windows(2).filter(|w| w[0] == w[1]).count();
                let moved = o[0].windows(2).filter(|w| w[0] != w[1]).count();
                assert!(
                    frozen > 0 && moved > 0,
                    "plateau shape reached neither the equal-close arm (frozen={frozen}) nor \
                     the change arms (moved={moved})"
                );
            }
        }

        // THE NON-FINITE CASE. `prev_close = close` runs UNCONDITIONALLY, so a NaN close
        // suppresses exactly TWO
        // bars -- the NaN bar itself (both compares false) and the NEXT bar, whose
        // `prev_close` is now NaN -- then the real prior close is back and the line moves
        // again. It does NOT freeze forever, which is what this block first asserted and
        // what the hand disproved on the run. The engine agrees because `Lag` re-reads the
        // stream rather than carrying `prev`, so it suppresses the same two rows by
        // construction. Finite-only parity matrices can never see this case.
        {
            let n = 64;
            let mut cl = series(n, |i| 100.0 + (i % 5) as f64);
            cl[20] = f64::NAN;
            let vol = volume(n);
            let cv = [cl.as_slice(), vol.as_slice()];
            let req = Request::new().add(tulip_rs::cse::recipes::obv(&[]), None);
            let out = run_registered(&req, &cv).expect("obv NaN registered");
            let (o, _) = Obv::indicator(&cv, &[], None).unwrap();
            assert_bits(
                row_for(&out, 0, true, 0),
                &o[0],
                "obv row with a NaN close (two-bar suppression, then resume)",
            );
            // NON-VACUITY in three parts: the line must actually move before the NaN, the
            // two suppressed rows must really be flat, and it must resume -- otherwise any
            // one of them could hold by accident.
            assert!(
                o[0][..18].windows(2).any(|w| w[0] != w[1]),
                "obv never moved before the NaN -- the shape proves nothing"
            );
            assert!(
                o[0][18] == o[0][19] && o[0][19] == o[0][20],
                "expected rows 18..=20 flat across the NaN close, got {:?}",
                &o[0][17..22]
            );
            assert!(
                o[0][21] != o[0][20],
                "expected the line to RESUME at row 21 once prev_close is a real bar, got \
                 {:?}",
                &o[0][19..23]
            );
        }
    }

    /// EMA + ATR + KELTNER: the merge the engine exists for, counted before it ran.
    /// Keltner's bands hang off `atr_nodes` (the same tr walk + mean-`Wilders` over the
    /// same `hlc()` columns that `Atr` builds) and its middle band is `Ema{src: close}`,
    /// the node `Ema(p)` builds. So three consumers, ONE of each.
    ///
    /// Derived, not observed: 3 `Input`s (H,L,C) + `Lag` 1 + `Sub` 4 (range, h-pc, l-pc,
    /// lower) + `Abs` 2 + `Max` 2 + `Wilders` 1 + `Ema` 1 + `Mul` 1 (atr*step) + `Add` 1
    /// (upper) + `Const` 1 (step) = 17 nodes, 8 sinks. Three separate arenas would total
    /// 2 + 12 + 17 = 31. A duplicated tr walk would show up here as `Abs = 4, Max = 4`.
    #[test]
    fn ema_atr_keltner_shared_interns_the_whole_atr_chain() {
        use tulip_rs::cse::kernel::registry::SIG_ema_atr_keltner_shared_p;
        use tulip_rs::cse::signature::{
            K_ABS, K_ADD, K_CONST, K_EMA, K_INPUT, K_LAG, K_MAX, K_MUL, K_SUB, K_WILDERS,
        };
        let req = Request::new()
            .add(tulip_rs::cse::recipes::ema(&[14.0]), None)
            .add(tulip_rs::cse::recipes::atr(&[14.0]), None)
            .add(tulip_rs::cse::recipes::keltnerchannel(&[14.0, 2.0]), None);
        let (arena, sig, _v, _owners) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_ema_atr_keltner_shared_p, "hub arena drifted");

        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        let got = (
            count(K_INPUT),
            count(K_LAG),
            count(K_SUB),
            count(K_ABS),
            count(K_MAX),
            count(K_WILDERS),
            count(K_EMA),
            count(K_MUL),
            count(K_ADD),
            count(K_CONST),
        );
        assert_eq!(
            got,
            (3, 1, 4, 2, 2, 1, 1, 1, 1, 1),
            "arena observed input/lag/sub/abs/max/wilders/ema/mul/add/const = {got:?} -- \
             abs=4 or max=4 would mean the tr walk was rebuilt instead of shared"
        );
        assert_eq!(
            arena.nodes.len(),
            18,
            "expected 18 nodes -- 17 plus atr's Mirror{{src: close}} tail-capture node (A5)"
        );
        assert_eq!(
            sig.n_sinks, 8,
            "1 ema + 1 atr + 1 atr's tr + 3 bands + 2 keltner mirrors"
        );
    }

    /// Bit-exact against all THREE hands, and the identity claims that make the sharing
    /// meaningful rather than merely compact: keltner's `atr` OPTIONAL must equal Atr's
    /// PRIMARY bit-for-bit (legal only because `keltnerchannel.rs` stores it UNCORRECTED,
    /// i.e. mean-mode -- adxr's ATR is composite and must be scaled, which is why that
    /// indicator cannot reuse a row this way), and keltner's `middle` must equal the
    /// standalone `Ema` row.
    ///
    /// Shapes and what each one actually carries. Corrected after the run refuted the
    /// first version of this comment, which called the third shape "flat" and claimed the
    /// bands collapse: with `hi = s + 2` and `lo = s - 1` the range is a CONSTANT 3.0, not
    /// zero, so `atr = 3`, `per = 6` and the bands open by exactly 6 about the middle.
    /// * wiggle / ramp -- ordinary values, varying ATR.
    /// * even-range -- constant close AND constant gap: `tr` is 3 at every bar, so `atr`
    ///   must be 3.0 (the mean-Wilders of a constant is that constant) and both bands must
    ///   sit 6.0 from the middle. This is the zero-drift check the zlema lesson says a
    ///   trend can hide, and it is what makes the `atr` OPTIONAL equality above non-vacuous.
    /// A true zero-range shape is deliberately NOT added here: keltner divides nothing, so
    /// high == low == close would only assert `lower == middle == upper`, and the even-range
    /// shape already pins the tighter fact (the exact seed). Vortex, which DOES divide,
    /// carries that case as its own block -- see atr_vortex_shared_bit_exact.
    #[test]
    fn ema_atr_keltner_shared_bit_exact() {
        use tulip_rs::indicators::atr::Atr;
        use tulip_rs::indicators::ema::Ema;
        use tulip_rs::indicators::keltnerchannel::KeltnerChannel;
        const P: f64 = 14.0;
        const M: f64 = 2.0;

        for (name, s) in [
            ("wiggle", wiggle()),
            ("ramp", series(680, |i| 100.0 + i as f64 * 0.05)),
            ("even-range", series(700, |_| 100.0)),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let hlc = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let one = [s.as_slice()];

            let req = Request::new()
                .add(tulip_rs::cse::recipes::ema(&[P]), None)
                // The optional flags must be asked for HERE as well as in the hand call:
                // mirrors are structurally in the arena (the count test above passes with
                // `None`) but their ROWS are empty unless wanted, so `None` here gave a
                // 0-length engine row against the hand's 699.
                .add(tulip_rs::cse::recipes::atr(&[P]), Some(&[true]))
                .add(
                    tulip_rs::cse::recipes::keltnerchannel(&[P, M]),
                    Some(&[true, true]),
                );
            let out = run_registered(&req, &hlc).expect("ema + atr + keltner registered");

            let (e, _) = Ema::indicator(&one, &[P], None).unwrap();
            assert_bits(
                row_for(&out, 0, true, 0),
                &e[0],
                &format!("ema row [{name}]"),
            );
            let (a, _) = Atr::indicator(&hlc, &[P], Some(&[true])).unwrap();
            assert_bits(
                row_for(&out, 1, true, 0),
                &a[0],
                &format!("atr row [{name}]"),
            );
            assert_bits(
                row_for(&out, 1, false, 0),
                &a[1],
                &format!("atr's tr mirror [{name}]"),
            );

            let (k, _) = KeltnerChannel::indicator(&hlc, &[P, M], Some(&[true, true])).unwrap();
            assert_bits(
                row_for(&out, 2, true, 0),
                &k[0],
                &format!("kc lower [{name}]"),
            );
            assert_bits(
                row_for(&out, 2, true, 1),
                &k[1],
                &format!("kc middle [{name}]"),
            );
            assert_bits(
                row_for(&out, 2, true, 2),
                &k[2],
                &format!("kc upper [{name}]"),
            );
            // The shared-node identities: same arena node, so same bits.
            assert_bits(
                row_for(&out, 2, false, 0),
                row_for(&out, 1, true, 0),
                &format!("kc's atr optional IS atr's primary [{name}]"),
            );
            assert_bits(
                row_for(&out, 2, false, 1),
                row_for(&out, 1, false, 0),
                &format!("kc's tr optional IS atr's tr mirror [{name}]"),
            );
            assert_bits(
                row_for(&out, 2, true, 1),
                row_for(&out, 0, true, 0),
                &format!("kc middle IS the standalone ema [{name}]"),
            );

            // Sign/order check against the HAND, run on EVERY shape. The earlier
            // `if name != "flat"` skip rested on the same mistaken reading of the fixture:
            // the gap is a constant 3.0, never zero, so `per = atr * 2 = 6.0` and the bands
            // open on all three. Left in place, that skip meant the third shape asserted
            // NOTHING about band order while the suite still reported green.
            assert!(
                k[0].iter().zip(&k[1]).all(|(l, m)| l < m)
                    && k[2].iter().zip(&k[1]).all(|(u, m)| u > m),
                "keltner bands not ordered lower<middle<upper on {name}"
            );
            assert!(
                k[0].iter().any(|v| *v != 0.0),
                "keltner lower row all-zero on {name} -- nothing was computed"
            );
            if name == "even-range" {
                // The identity this fixture exists to test: `tr` is exactly 3.0 at every
                // bar, so the mean-Wilders of a constant must be that constant -- `atr`
                // == 3.0, with no warm-up residue. A seed that disagreed with the steady
                // recurrence (the zlema failure mode) shows here as atr != 3.0 near the
                // front of the row, where a moving series only decays it out of sight.
                assert!(
                    a[0].iter().all(|x| *x == 3.0),
                    "even-range atr is not exactly 3.0 -- mean-Wilders seed drift: {:?}",
                    &a[0][..a[0].len().min(4)]
                );
            }
        }

        // The lone floor too: `keltnerchannel_n` is a separately compiled kernel, so the
        // hub passing does not certify it.
        {
            let s = wiggle();
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let hlc = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let floor = Request::new().add(tulip_rs::cse::recipes::keltnerchannel(&[P, M]), None);
            let out = run_registered(&floor, &hlc).expect("keltner floor registered");
            let (k, _) = KeltnerChannel::indicator(&hlc, &[P, M], Some(&[true, true])).unwrap();
            assert_bits(row_for(&out, 0, true, 0), &k[0], "kc lower [floor]");
            assert_bits(row_for(&out, 0, true, 1), &k[1], "kc middle [floor]");
            assert_bits(row_for(&out, 0, true, 2), &k[2], "kc upper [floor]");
        }
    }

    /// ATR + VORTEX: the tr walk shared, counted before it ran. `Atr` is 12 nodes alone
    /// (3 inputs + the 8-node tr walk + the Wilders) and `Vortex` is 22 (3 inputs + the SAME
    /// 8-node tr walk + 2 prev-value lags, 2 subs, 2 abs, 3 rolls, 2 divs). Interning the tr
    /// walk -- same `tr_nodes` spelling, same `hlc()` streams 0,1,2 -- predicts a merged 23.
    /// Not shared would show as `max=4, abs=6, input=6`.
    ///
    /// `n_sinks` is 5 with only 4 distinct sink NODES, because both indicators mirror the
    /// SAME interned `tr` -- that two-sinks-one-node shape is the sharing, made visible in
    /// the sink table and not just in the node counts.
    #[test]
    fn atr_vortex_shared_interns_the_tr_walk() {
        use tulip_rs::cse::kernel::registry::SIG_atr_vortex_shared_p;
        use tulip_rs::cse::signature::{
            K_ABS, K_DIV, K_INPUT, K_LAG, K_MAX, K_ROLLSUM, K_SUB, K_WILDERS,
        };
        let req = Request::new()
            .add(tulip_rs::cse::recipes::atr(&[14.0]), Some(&[true]))
            .add(tulip_rs::cse::recipes::vortex(&[14.0]), Some(&[true]));
        let (arena, sig, _v, _owners) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_atr_vortex_shared_p, "hub arena drifted");

        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        let got = (
            count(K_INPUT),
            count(K_LAG),
            count(K_SUB),
            count(K_ABS),
            count(K_MAX),
            count(K_WILDERS),
            count(K_ROLLSUM),
            count(K_DIV),
        );
        assert_eq!(
            got,
            (3, 3, 5, 4, 2, 1, 3, 2),
            "arena observed input/lag/sub/abs/max/wilders/rollsum/div = {got:?} -- max=4 or \
             abs=6 would mean the tr walk was rebuilt, not shared"
        );
        assert_eq!(
            arena.nodes.len(),
            24,
            "expected 24 nodes -- 23 plus atr's own Mirror{{src: close}} tail-capture node (A5)"
        );
        assert_eq!(
            sig.n_sinks, 5,
            "atr primary+tr, vortex two bands+tr: 5 sinks, and the two tr sinks alias one node"
        );
    }

    /// Bit-exact against both hands. The shapes are chosen for DIFFERENT reasons, and
    /// conflating them is how a green suite hides nothing:
    /// * wiggle / ramp carry ordinary VALUES -- varying denominators, bands finite and
    ///   non-zero, so a match there is a match on real arithmetic.
    /// * EVEN-RANGE (constant close, constant high-low gap) is a strong identity check the
    ///   other shapes cannot make: `hi = s+2`, `lo = s-1` gives `tr = 3`, `|high - low[1]| =
    ///   3` and `|low - high[1]| = 3`, so BOTH bands must be exactly `1.0` -- equal window
    ///   sums reached through different windows and different subtract operands. A kernel
    ///   that swapped `lag_hi`/`lag_lo`, or folded `hi-lo` instead of the lagged terms,
    ///   still lands near 1.0 on wiggle but breaks here.
    /// * ZERO-RANGE (high == low == close) is the float-class case the derived-roll and
    ///   float-sweep notes demand: every `tr` term is 0, so the denominator is 0 and the
    ///   hand's divide is UNGUARDED (`vm_sums / tr_sum_simd`, no `if md == 0`), printing
    ///   `0.0 / 0.0` = NaN. `vwma`/`vosc`/`bop` DO guard, so copying that neighbour would
    ///   print 0.0 and pass every finite shape. Pinned as all-NaN to make it evidence.
    #[test]
    fn atr_vortex_shared_bit_exact() {
        use tulip_rs::indicators::atr::Atr;
        use tulip_rs::indicators::vortex::Vortex;
        const P: f64 = 14.0;

        for (name, s) in [
            ("wiggle", wiggle()),
            ("ramp", series(680, |i| 100.0 + i as f64 * 0.05)),
            ("even-range", series(700, |_| 100.0)),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let hlc = [hi.as_slice(), lo.as_slice(), s.as_slice()];

            let req = Request::new()
                .add(tulip_rs::cse::recipes::atr(&[P]), Some(&[true]))
                .add(tulip_rs::cse::recipes::vortex(&[P]), Some(&[true]));
            let out = run_registered(&req, &hlc).expect("atr + vortex registered");

            let (a, _) = Atr::indicator(&hlc, &[P], Some(&[true])).unwrap();
            assert_bits(
                row_for(&out, 0, true, 0),
                &a[0],
                &format!("atr row [{name}]"),
            );
            assert_bits(
                row_for(&out, 0, false, 0),
                &a[1],
                &format!("atr's tr mirror [{name}]"),
            );

            let (v, _) = Vortex::indicator(&hlc, &[P], Some(&[true])).unwrap();
            assert_bits(row_for(&out, 1, true, 0), &v[0], &format!("vi_up [{name}]"));
            assert_bits(
                row_for(&out, 1, true, 1),
                &v[1],
                &format!("vi_down [{name}]"),
            );
            // Both tr mirrors are the SAME arena node, so they must be the same bits.
            assert_bits(
                row_for(&out, 1, false, 0),
                row_for(&out, 0, false, 0),
                &format!("vortex's tr mirror IS atr's tr mirror [{name}]"),
            );
            assert_bits(
                row_for(&out, 1, false, 0),
                &v[2],
                &format!("vortex tr mirror vs hand [{name}]"),
            );

            if name == "even-range" {
                // `vm_up = |hi - lo[1]| = 3`, `vm_dn = |lo - hi[1]| = 3`, `tr = max(3,2,1) = 3`
                // at every bar, so all three window sums accumulate the SAME float sequence
                // and each band is that value over itself: exactly 1.0, not approximately.
                // A kernel that folded `hi - lo` instead of the lagged terms, or swapped the
                // two prev-lags, breaks here while still matching on wiggle.
                assert!(
                    v[0].iter().all(|x| *x == 1.0) && v[1].iter().all(|x| *x == 1.0),
                    "even-range bands must be exactly 1.0, got up={:?} down={:?}",
                    &v[0][..v[0].len().min(4)],
                    &v[1][..v[1].len().min(4)]
                );
            } else {
                // Non-vacuity for the value shapes: the bands must actually be finite and
                // non-zero, or matching NaN-to-NaN would look like a pass.
                assert!(
                    v[0].iter().any(|x| *x != 0.0 && x.is_finite())
                        && v[1].iter().any(|x| *x != 0.0 && x.is_finite()),
                    "vi_up/vi_down are not finite non-zero on {name} -- shape proves nothing"
                );
            }

            // The lone floor is a separate compiled kernel; the hub does not certify it.
            let floor = Request::new().add(tulip_rs::cse::recipes::vortex(&[P]), Some(&[true]));
            let out = run_registered(&floor, &hlc).expect("vortex floor registered");
            assert_bits(
                row_for(&out, 0, true, 0),
                &v[0],
                &format!("vi_up [{name}] (floor)"),
            );
            assert_bits(
                row_for(&out, 0, true, 1),
                &v[1],
                &format!("vi_down [{name}] (floor)"),
            );
        }

        // ZERO-RANGE: high == low == close, so every `tr` term is 0 AND every `|high -
        // low[1]|`/`|low - high[1]|` term is 0 -- both window sums are exactly 0.0. The
        // hand divides `vm_sums / tr_sum_simd` with no guard, so it prints 0.0/0.0 = NaN,
        // and the kernel must too. `vwma` and `vosc` BOTH guard their ratio, so a guard
        // borrowed from either neighbour prints 0.0 here and passes every other shape in
        // this file -- this block is the only thing that separates the two spellings.
        {
            let px = series(300, |_| 100.0);
            let hlc = [px.as_slice(), px.as_slice(), px.as_slice()];
            let req = Request::new()
                .add(tulip_rs::cse::recipes::atr(&[P]), Some(&[true]))
                .add(tulip_rs::cse::recipes::vortex(&[P]), Some(&[true]));
            let out = run_registered(&req, &hlc).expect("atr + vortex zero-range");

            let (v, _) = Vortex::indicator(&hlc, &[P], Some(&[true])).unwrap();
            assert!(
                v[0].iter().all(|x| x.is_nan()) && v[1].iter().all(|x| x.is_nan()),
                "the HAND's unguarded 0/0 must be NaN on zero range, got up={:?} down={:?}",
                &v[0][..v[0].len().min(4)],
                &v[1][..v[1].len().min(4)]
            );
            assert_bits(
                row_for(&out, 1, true, 0),
                &v[0],
                "vi_up zero-range (unguarded 0/0 = NaN)",
            );
            assert_bits(
                row_for(&out, 1, true, 1),
                &v[1],
                "vi_down zero-range (unguarded 0/0 = NaN)",
            );
            // Atr on the same bars has NO divide, so its row is 0.0, not NaN -- the two
            // indicators differ at exactly this input class, which is why the shared
            // `tr` node must still be identical (it is the same node, so it is).
            let (a, _) = Atr::indicator(&hlc, &[P], Some(&[true])).unwrap();
            assert_bits(row_for(&out, 0, true, 0), &a[0], "atr row zero-range");
            assert!(
                a[0].iter().all(|x| *x == 0.0),
                "atr on zero range must be 0.0 (no divide), got {:?}",
                &a[0][..a[0].len().min(4)]
            );
        }
    }

    /// BOP + MARKETFI: sharing that is pure ARITHMETIC, not a recurrence. `bop` builds
    /// `Sub{high, low}` (its denominator's operand) and an `EPSILON` `Const`; `marketfi`
    /// needs that SAME range as its numerator and the same constant under its volume floor.
    /// `bop` binds close to stream 3 via `ohlc()`, `marketfi` binds volume to stream 2 via
    /// `hlvol()` -- five distinct columns in the union, so this also re-pins the remap on a
    /// new column set.
    ///
    /// Derived before running: input 5, sub 2, const 1, max 2, div 2 = 12 nodes. If the
    /// range `Sub` failed to intern: sub=3, len=13; if the `EPSILON` Const failed: const=2.
    /// The two `Max` nodes MUST stay distinct -- one floors the range, the other the volume.
    #[test]
    fn bop_marketfi_shared_interns_the_range_and_eps() {
        use tulip_rs::cse::kernel::registry::SIG_bop_marketfi_shared;
        use tulip_rs::cse::signature::{K_CONST, K_DIV, K_INPUT, K_MAX, K_SUB};
        let req = Request::new()
            .add(tulip_rs::cse::recipes::bop(&[]), None)
            .add(tulip_rs::cse::recipes::marketfi(&[]), None);
        let (arena, sig, _v, _owners) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_bop_marketfi_shared, "hub arena drifted");

        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        let got = (
            count(K_INPUT),
            count(K_SUB),
            count(K_CONST),
            count(K_MAX),
            count(K_DIV),
        );
        assert_eq!(
            got,
            (5, 2, 1, 2, 2),
            "arena observed input/sub/const/max/div = {got:?} -- sub=3 or const=2 would mean \
             the shared range/epsilon did not intern"
        );
        assert_eq!(arena.nodes.len(), 12, "expected 12 nodes");
    }

    /// Bit-exact against both hands on the hub AND the lone `marketfi_n` floor, plus a
    /// ZERO-VOLUME block: the hand writes `(high - low) / volume.max(f64::EPSILON)`, so a
    /// dead volume prints the range over EPSILON -- an enormous FINITE number. A `Where*`
    /// guard copied from `vwma`/`vosc` would print 0.0 there and pass every finite-volume
    /// shape, so this block is what distinguishes the two spellings.
    ///
    /// What this block does NOT prove, stated so nobody reads more into it: it is not a
    /// float-CLASS sweep. With volume exactly 0.0 and EPSILON positive there is no tie and
    /// no NaN, so it never reaches the one place the engine's `Max` (`select(a > b, a, b)`,
    /// taking the SECOND operand on an unordered compare) can diverge from `f64::max`
    /// (which keeps `self`) -- the signed-zero and NaN cases recorded in the float-class
    /// note. Pinning THOSE needs a fixture at `volume = EPSILON` exactly or a NaN lane,
    /// which is a separate, deliberate test and not this one.
    #[test]
    fn bop_marketfi_shared_bit_exact() {
        use tulip_rs::indicators::bop::Bop;
        use tulip_rs::indicators::marketfi::Marketfi;

        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let op: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let vol = volume(s.len());
            // Column order is the union in canonical rank: open, high, low, close, volume.
            let five = [
                op.as_slice(),
                hi.as_slice(),
                lo.as_slice(),
                s.as_slice(),
                vol.as_slice(),
            ];
            let ohlc = [op.as_slice(), hi.as_slice(), lo.as_slice(), s.as_slice()];
            let hlvol = [hi.as_slice(), lo.as_slice(), vol.as_slice()];

            let hub = Request::new()
                .add(tulip_rs::cse::recipes::bop(&[]), None)
                .add(tulip_rs::cse::recipes::marketfi(&[]), None);
            let out = run_registered(&hub, &five).expect("bop + marketfi registered");

            let (b, _) = Bop::indicator(&ohlc, &[], None).unwrap();
            assert_bits(
                row_for(&out, 0, true, 0),
                &b[0],
                "bop row in the marketfi hub",
            );
            let (m, _) = Marketfi::indicator(&hlvol, &[], None).unwrap();
            assert_bits(row_for(&out, 1, true, 0), &m[0], "marketfi row [hub]");

            let floor = Request::new().add(tulip_rs::cse::recipes::marketfi(&[]), None);
            let out = run_registered(&floor, &hlvol).expect("marketfi floor registered");
            assert_bits(
                row_for(&out, 0, true, 0),
                &m[0],
                "marketfi row [floor, hlvol columns]",
            );
        }

        // ZERO VOLUME: denominator is exactly EPSILON, so the row is huge and FINITE.
        {
            let n = 200;
            let hi = series(n, |i| 101.0 + (i % 3) as f64);
            let lo = series(n, |i| 99.0 + (i % 3) as f64);
            let op = series(n, |i| 100.0 + (i % 3) as f64);
            let zero_vol = vec![0.0; n];
            let five = [
                op.as_slice(),
                hi.as_slice(),
                lo.as_slice(),
                op.as_slice(),
                zero_vol.as_slice(),
            ];
            let hlvol = [hi.as_slice(), lo.as_slice(), zero_vol.as_slice()];
            let req = Request::new().add(tulip_rs::cse::recipes::marketfi(&[]), None);
            let out = run_registered(&req, &hlvol).expect("marketfi zero-volume");
            let (m, _) = Marketfi::indicator(&hlvol, &[], None).unwrap();
            assert_bits(
                row_for(&out, 0, true, 0),
                &m[0],
                "marketfi ZERO-VOLUME row (EPSILON floor, not a 0.0 guard)",
            );
            assert!(
                m[0].iter().all(|x| x.is_finite() && *x != 0.0),
                "zero volume must give the range over EPSILON -- huge and finite, got {:?}",
                &m[0][..m[0].len().min(4)]
            );
            // The hub must agree with the floor on the same data.
            let hub = Request::new()
                .add(tulip_rs::cse::recipes::bop(&[]), None)
                .add(tulip_rs::cse::recipes::marketfi(&[]), None);
            let out = run_registered(&hub, &five).expect("bop + marketfi zero-volume");
            assert_bits(
                row_for(&out, 1, true, 0),
                &m[0],
                "marketfi zero-volume [hub]",
            );
        }
    }

    /// THE CHAINED-EMA FLOORS plus the `Dema + Tema` interning claim, measured.
    /// `tema` owns a `DemaState`, so its ema1/ema2 and its `dema` node are exactly
    /// what the standalone `dema` request builds: the arena must hold THREE `Ema`
    /// nodes (ema1, ema2, ema3) not five, and ONE `dema` Fma, and tema's optional
    /// `dema` row must be bit-identical to dema's primary -- two requests, one
    /// stored stream.
    #[test]
    fn chained_ema_floors_and_dema_tema_hub() {
        use tulip_rs::cse::kernel::registry::SIG_dema_tema_shared_p;
        use tulip_rs::cse::signature::{K_EMA, K_FMA, K_MUL};
        use tulip_rs::indicators::dema::Dema;
        use tulip_rs::indicators::tema::Tema;

        let req_hub = Request::new()
            .add(tulip_rs::cse::recipes::dema(&[9.0]), None)
            .add(tulip_rs::cse::recipes::tema(&[9.0]), None);
        let (_a, sig, _v, _o) = erase_request(&req_hub).unwrap();
        assert_eq!(sig, SIG_dema_tema_shared_p, "hub arena drifted");
        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        let (ema, fma, mul) = (count(K_EMA), count(K_FMA), count(K_MUL));
        assert_eq!(
            (ema, fma, mul),
            (3, 3, 1),
            "arena observed ema={ema} fma={fma} mul={mul} -- expected THREE chained \
             EMAs (ema1/ema2/ema3 shared, not 2+3), and three Fmas: dema, tema's \
             inner and outer. Separate arenas would be 5 EMAs and 4 Fmas."
        );

        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let streams = [s.as_slice()];

            let out = run_registered(&req_hub, &streams).expect("dema+tema registered");
            let (d, _) = Dema::indicator(&streams, &[9.0], None).unwrap();
            let (t, _) = Tema::indicator(&streams, &[9.0], None).unwrap();
            assert_bits(row_for(&out, 0, true, 0), &d[0], "hub: dema row");
            assert_bits(row_for(&out, 1, true, 0), &t[0], "hub: tema row");
            // tema's optional `dema` mirror is not wanted -> empty; the published
            // dema row above is dema's OWN primary, so a non-empty mirror would
            // mean the two requests did not dedup onto one stored stream.
            for (o, row) in out.owners.iter().zip(out.rows.iter()) {
                if !o.primary {
                    assert!(row.is_empty(), "disabled optional {o:?} must be empty");
                }
            }

            for (name, req, hand) in [
                (
                    "dema_n",
                    Request::new().add(tulip_rs::cse::recipes::dema(&[9.0]), None),
                    Dema::indicator(&streams, &[9.0], None).unwrap().0,
                ),
                (
                    "tema_n",
                    Request::new().add(tulip_rs::cse::recipes::tema(&[9.0]), None),
                    Tema::indicator(&streams, &[9.0], None).unwrap().0,
                ),
            ] {
                let out = run_registered(&req, &streams)
                    .unwrap_or_else(|| panic!("{name} not registered"));
                assert_bits(row_for(&out, 0, true, 0), &hand[0], name);
            }
        }
    }

    /// THE TWO ATR ABLATION RUNGS, bit-exact against their own hands BEFORE any
    /// number was read off them. `tr_n` is the tr subgraph alone (no recurrence);
    /// `wilders_n` is the mean-`Wilders` recurrence alone over close (no tr walk).
    ///
    /// The leads are the interesting part, and a first reading got them wrong: both
    /// hand cycles run `for i in 0..len`, which looks like publish-from-0, but each
    /// writes into a buffer sized by `output_length = len - min_data + 1`, so the ROW
    /// starts at `min_data - 1` regardless. Measured at the registered leads (tr 1,
    /// wilders `p`), both rows agree with the hand bar-for-bar with NO shift on all
    /// three series -- so comparing shifted-by-lead here would be the bug, and the
    /// zero-shift comparison below is what the probe established before it was trusted.
    #[test]
    fn atr_ablation_rungs_bit_exact() {
        use tulip_rs::indicators::tr::Tr;
        use tulip_rs::indicators::wilders::Wilders;
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let hlc3: [&[f64]; 3] = [hi.as_slice(), lo.as_slice(), s.as_slice()];

            let req_tr = Request::new().add(tulip_rs::cse::recipes::tr(&[]), None);
            let out = run_registered(&req_tr, &hlc3).expect("tr_n registered");
            let hand = Tr::indicator(&hlc3, &[], None).unwrap().0;
            assert_bits(
                row_for(&out, 0, true, 0),
                &hand[0],
                "tr_n (tr subgraph, no recurrence)",
            );

            let req_w = Request::new().add(tulip_rs::cse::recipes::wilders(&[14.0]), None);
            let one = [s.as_slice()];
            let out = run_registered(&req_w, &one).expect("wilders_n registered");
            let hand = Wilders::indicator(&one, &[14.0], None).unwrap().0;
            assert_bits(
                row_for(&out, 0, true, 0),
                &hand[0],
                "wilders_n (mean Wilders, no tr walk)",
            );
        }
    }

    /// THE GATE THE `dpo` REWRITE WAS DONE FOR. `dpo`'s rolling mean is over a raw
    /// price Input, so it is spelled `Node::Sma` -- the same kind `sma()` uses. Had it
    /// stayed a `RollSum{mean:true}` the two would be DIFFERENT signature kinds
    /// (K_SMA=16 vs K_ROLLSUM=24) and this arena would carry the same rolling mean of
    /// `real` twice, buying nothing. Counts are read from the arena and printed on
    /// failure, so the interning is observed rather than asserted in a comment.
    #[test]
    fn sma_dpo_shared_p_interns_one_sma_and_is_bit_exact() {
        use tulip_rs::cse::kernel::registry::SIG_sma_dpo_shared_p;
        use tulip_rs::cse::signature::{K_INPUT, K_LAG, K_SMA, K_SUB};
        let req = Request::new()
            .add(tulip_rs::cse::recipes::sma(&[20.0]), None)
            .add(tulip_rs::cse::recipes::dpo(&[20.0]), None);
        let (_a, sig, _v, _o) = erase_request(&req).unwrap();
        assert_eq!(sig, SIG_sma_dpo_shared_p, "hub arena drifted");
        let count = |k: u8| (0..MAX).filter(|i| sig.kinds[*i] == k).count();
        let (input, sma, lag, sub) = (count(K_INPUT), count(K_SMA), count(K_LAG), count(K_SUB));
        assert_eq!(
            (input, sma, lag, sub),
            (1, 1, 1, 1),
            "arena observed input={input} sma={sma} lag={lag} sub={sub} -- one close \
             Input, ONE rolling mean shared by the standalone Sma and dpo's detrend, \
             dpo's look-back Lag, and dpo's Sub"
        );

        use tulip_rs::indicators::dpo::Dpo;
        use tulip_rs::indicators::sma::Sma;
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let streams = [s.as_slice()];
            let out = run_registered(&req, &streams).expect("sma+dpo registered");
            let (sm, _) = Sma::indicator(&streams, &[20.0], None).unwrap();
            let (dp, _) = Dpo::indicator(&streams, &[20.0], None).unwrap();
            assert_bits(
                row_for(&out, 0, true, 0),
                &sm[0],
                "sma+dpo: standalone Sma row",
            );
            assert_bits(row_for(&out, 1, true, 0), &dp[0], "sma+dpo: dpo row");
            // dpo's own `sma` optional mirror is NOT wanted; the published Sma row
            // above comes from the standalone request, so a non-empty mirror here
            // would mean the two sinks were not deduped to the one interned node.
            for (o, row) in out.owners.iter().zip(out.rows.iter()) {
                if !o.primary {
                    assert!(row.is_empty(), "disabled optional {o:?} must be empty");
                }
            }
        }
    }

    /// SAME-SOURCE PACKED TWIN vs SCALAR, byte-for-byte, over three series. The
    /// twin shares the arena's `Signature`, so `try_run` reaches the SCALAR kernel
    /// first — the twin must be called DIRECTLY, exactly as the bench rows do, or
    /// the comparison silently measures the same kernel twice and passes vacuously.
    /// Groups here are `(Ema,close)` = 4-wide and `(Ema,AD)` = 2-wide, both from one
    /// broadcast source each; the cross-source arm that merged them into one group
    /// was measured slower and is NOT registered.
    #[test]
    fn adosc_apo_ppo_packed_matches_scalar() {
        use tulip_rs::cse::kernel::registry::adosc_apo_ppo_p;
        let req = Request::new()
            .add(Adosc::recipe(&[3.0, 10.0]), None)
            .add(Apo::recipe(&[12.0, 26.0]), None)
            .add(Ppo::recipe(&[6.0, 14.0]), None);
        for base in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let hi: Vec<f64> = base.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = base.iter().map(|v| v - 1.0).collect();
            let vol = volume(base.len());
            let streams = [
                hi.as_slice(),
                lo.as_slice(),
                base.as_slice(),
                vol.as_slice(),
            ];
            let (_a, _sig, vals, _) = erase_request(&req).unwrap();
            let scalar = run_registered(&req, &streams).expect("scalar arm registered");
            let packed = adosc_apo_ppo_p(&vals, &streams);
            assert_eq!(packed.len(), scalar.rows.len(), "same row count");
            for (i, (rs, rp)) in packed.iter().zip(scalar.rows.iter()).enumerate() {
                assert_bits(
                    rp.as_slice(),
                    rs.as_slice(),
                    &format!("packed row {i} vs scalar"),
                );
            }
        }
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

    #[test]
    fn nvi_bit_exact() {
        use tulip_rs::indicators::nvi::Nvi;
        let req = Request::new().add(Nvi::recipe(&[]), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let vol = volume(s.len());
            let streams = [s.as_slice(), vol.as_slice()];
            let out = run_registered(&req, &streams).expect("nvi registered");
            let (n, _) = Nvi::indicator(&streams, &[], None).unwrap();
            assert_bits(&out.rows[0], &n[0], "nvi row");
        }
    }

    #[test]
    fn pvi_bit_exact() {
        use tulip_rs::indicators::pvi::Pvi;
        let req = Request::new().add(Pvi::recipe(&[]), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let vol = volume(s.len());
            let streams = [s.as_slice(), vol.as_slice()];
            let out = run_registered(&req, &streams).expect("pvi registered");
            let (p, _) = Pvi::indicator(&streams, &[], None).unwrap();
            assert_bits(&out.rows[0], &p[0], "pvi row");
        }
    }

    #[test]
    fn adxr_bit_exact() {
        use tulip_rs::indicators::adxr::Adxr;
        const P: [f64; 1] = [14.0];
        let req = Request::new().add(Adxr::recipe(&P), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &streams).expect("adxr registered");
            let (a, _) = Adxr::indicator(&streams, &P, None).unwrap();
            assert_bits(&out.rows[0], &a[0], "adxr row");
        }
    }

    #[test]
    fn md_bit_exact() {
        use tulip_rs::indicators::md::Md;
        const P: [f64; 1] = [14.0];
        let req = Request::new().add(Md::recipe(&P), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let streams = [s.as_slice()];
            let out = run_registered(&req, &streams).expect("md registered");
            let (m, _) = Md::indicator(&streams, &P, None).unwrap();
            assert_bits(&out.rows[0], &m[0], "md row");
        }
    }

    #[test]
    fn cci_bit_exact() {
        use tulip_rs::indicators::cci::Cci;
        const P: [f64; 1] = [14.0];
        let req = Request::new().add(Cci::recipe(&P), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &streams).expect("cci registered");
            let (c, _) = Cci::indicator(&streams, &P, None).unwrap();
            assert_bits(&out.rows[0], &c[0], "cci row");
        }
    }

    #[test]
    fn highpass_planable_bit_exact() {
        for p in [7.0, 20.0, 48.0] {
            let req = Request::new().add(HighPass::recipe(&[p]), None);
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("highpass registered");
                let (h, _) = HighPass::indicator(&streams, &[p], None).unwrap();
                assert_bits(&out.rows[0], &h[0], "highpass row [Planable]");
            }
        }
    }

    #[test]
    fn supersmoother_planable_bit_exact() {
        for p in [7.0, 20.0, 48.0] {
            let req = Request::new().add(SuperSmoother::recipe(&[p]), None);
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("supersmoother registered");
                let (s, _) = SuperSmoother::indicator(&streams, &[p], None).unwrap();
                assert_bits(&out.rows[0], &s[0], "supersmoother row [Planable]");
            }
        }
    }

    #[test]
    fn roofingfilter_planable_bit_exact() {
        for p in [[10.0, 48.0], [5.0, 20.0]] {
            let req = Request::new().add(RoofingFilter::recipe(&p), Some(&[true]));
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("roofingfilter registered");
                let (r, _) = RoofingFilter::indicator(&streams, &p, Some(&[true])).unwrap();
                // roofingfilter: primary=roofing(node 2), optional=highpass(node 1)
                assert_bits(&out.rows[0], &r[0], "roofing filter [Planable]");
                assert_bits(&out.rows[1], &r[1], "highpass intermediate [Planable]");
            }
        }
    }

    #[test]
    fn hilberttransform_bit_exact() {
        use tulip_rs::indicators::hilberttransform::HilbertTransform;
        for p in [[10.0_f64, 48.0_f64], [6.0, 20.0]] {
            let req = Request::new().add(HilbertTransform::recipe(&p), None);
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("hilberttransform registered");
                let (h, _) = HilbertTransform::indicator(&streams, &p, None).unwrap();
                assert_bits(&out.rows[0], &h[0], "hilberttransform in_phase");
                assert_bits(&out.rows[1], &h[1], "hilberttransform quadrature");
            }
        }
    }

    #[test]
    fn cvi_bit_exact() {
        use tulip_rs::indicators::cvi::Cvi;
        const P: [f64; 1] = [10.0];
        let req = Request::new().add(Cvi::recipe(&P), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice()];
            let out = run_registered(&req, &streams).expect("cvi registered");
            let (c, _) = Cvi::indicator(&streams, &P, None).unwrap();
            assert_bits(&out.rows[0], &c[0], "cvi row");
        }
    }

    #[test]
    fn kvo_bit_exact() {
        use tulip_rs::indicators::kvo::Kvo;
        const P: [f64; 2] = [34.0, 55.0];
        let req = Request::new().add(Kvo::recipe(&P), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let vol = volume(s.len());
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice(), vol.as_slice()];
            let out = run_registered(&req, &streams).expect("kvo registered");
            let (k, _) = Kvo::indicator(&streams, &P, None).unwrap();
            assert_bits(&out.rows[0], &k[0], "kvo row");
        }
    }

    #[test]
    fn chandelierexit_bit_exact() {
        use tulip_rs::indicators::chandelierexit::ChandelierExit;
        const P: [f64; 2] = [22.0, 3.0];
        let req = Request::new().add(ChandelierExit::recipe(&P), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &streams).expect("chandelierexit registered");
            let (ce, _) = ChandelierExit::indicator(&streams, &P, None).unwrap();
            assert_bits(&out.rows[0], &ce[0], "chandelierexit long row");
            assert_bits(&out.rows[1], &ce[1], "chandelierexit short row");
        }
    }

    #[test]
    fn elderray_bit_exact() {
        use tulip_rs::indicators::elderray::Elderray;
        const P: [f64; 1] = [13.0];
        let req = Request::new().add(Elderray::recipe(&P), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &streams).expect("elderray registered");
            let (er, _) = Elderray::indicator(&streams, &P, None).unwrap();
            assert_bits(&out.rows[0], &er[0], "elderray bull row");
            assert_bits(&out.rows[1], &er[1], "elderray bear row");
        }
    }

    #[test]
    fn trvi_bit_exact() {
        use tulip_rs::indicators::trvi::Trvi;
        const P: [f64; 1] = [13.0];
        let req = Request::new().add(Trvi::recipe(&P), Some(&[true, true]));
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &streams).expect("trvi registered");
            let (t, _) = Trvi::indicator(&streams, &P, Some(&[true, true])).unwrap();
            assert_bits(&out.rows[0], &t[0], "trvi row");
            assert_bits(&out.rows[1], &t[1], "trvi tr row");
            assert_bits(&out.rows[2], &t[2], "trvi ema row");
        }
    }

    #[test]
    fn dema_bit_exact() {
        use tulip_rs::indicators::dema::Dema;
        for p in [5.0, 14.0] {
            let pp: [f64; 1] = [p];
            let req = Request::new().add(Dema::recipe(&pp), Some(&[true]));
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("dema registered");
                let (d, _) = Dema::indicator(&streams, &pp, Some(&[true])).unwrap();
                assert_bits(&out.rows[0], &d[0], "dema row");
                assert_bits(&out.rows[1], &d[1], "dema ema row");
            }
        }
    }

    #[test]
    fn tema_bit_exact() {
        use tulip_rs::indicators::tema::Tema;
        for p in [5.0, 14.0] {
            let pp: [f64; 1] = [p];
            let req = Request::new().add(Tema::recipe(&pp), Some(&[true, true]));
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("tema registered");
                let (t, _) = Tema::indicator(&streams, &pp, Some(&[true, true])).unwrap();
                assert_bits(&out.rows[0], &t[0], "tema row");
                assert_bits(&out.rows[1], &t[1], "tema dema row");
                assert_bits(&out.rows[2], &t[2], "tema ema row");
            }
        }
    }

    #[test]
    fn trima_bit_exact() {
        use tulip_rs::indicators::trima::Trima;
        for p in [5.0, 14.0, 15.0] {
            let pp: [f64; 1] = [p];
            let req = Request::new().add(Trima::recipe(&pp), None);
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("trima registered");
                let (t, _) = Trima::indicator(&streams, &pp, None).unwrap();
                assert_bits(&out.rows[0], &t[0], "trima row");
            }
        }
    }

    #[test]
    fn trix_bit_exact() {
        use tulip_rs::indicators::trix::Trix;
        for p in [5.0, 14.0] {
            let pp: [f64; 1] = [p];
            let req = Request::new().add(Trix::recipe(&pp), Some(&[true, true, true]));
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("trix registered");
                let (t, _) = Trix::indicator(&streams, &pp, Some(&[true, true, true])).unwrap();
                assert_bits(&out.rows[0], &t[0], "trix row");
                assert_bits(&out.rows[1], &t[1], "trix tema row");
                assert_bits(&out.rows[2], &t[2], "trix dema row");
                assert_bits(&out.rows[3], &t[3], "trix ema row");
            }
        }
    }

    #[test]
    fn roc_bit_exact() {
        use tulip_rs::indicators::roc::Roc;
        for p in [5.0, 14.0] {
            let pp: [f64; 1] = [p];
            let req = Request::new().add(Roc::recipe(&pp), Some(&[true]));
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("roc registered");
                let (r, _) = Roc::indicator(&streams, &pp, Some(&[true])).unwrap();
                assert_bits(&out.rows[0], &r[0], "roc row");
                assert_bits(&out.rows[1], &r[1], "roc mom row");
            }
        }
    }

    #[test]
    fn rocr_bit_exact() {
        use tulip_rs::indicators::rocr::Rocr;
        for p in [5.0, 14.0] {
            let pp: [f64; 1] = [p];
            let req = Request::new().add(Rocr::recipe(&pp), None);
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("rocr registered");
                let (r, _) = Rocr::indicator(&streams, &pp, None).unwrap();
                assert_bits(&out.rows[0], &r[0], "rocr row");
            }
        }
    }

    #[test]
    fn dpo_bit_exact() {
        use tulip_rs::indicators::dpo::Dpo;
        for p in [5.0, 14.0] {
            let pp: [f64; 1] = [p];
            let req = Request::new().add(Dpo::recipe(&pp), Some(&[true]));
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("dpo registered");
                let (d, _) = Dpo::indicator(&streams, &pp, Some(&[true])).unwrap();
                assert_bits(&out.rows[0], &d[0], "dpo row");
                assert_bits(&out.rows[1], &d[1], "dpo sma row");
            }
        }
    }

    #[test]
    fn typprice_bit_exact() {
        use tulip_rs::indicators::typprice::Typprice;
        let pp: [f64; 0] = [];
        let req = Request::new().add(Typprice::recipe(&pp), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &streams).expect("typprice registered");
            let (t, _) = Typprice::indicator(&streams, &pp, None).unwrap();
            assert_bits(&out.rows[0], &t[0], "typprice row");
        }
    }

    #[test]
    fn wcprice_bit_exact() {
        use tulip_rs::indicators::wcprice::WcPrice;
        let pp: [f64; 0] = [];
        let req = Request::new().add(WcPrice::recipe(&pp), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &streams).expect("wcprice registered");
            let (w, _) = WcPrice::indicator(&streams, &pp, None).unwrap();
            assert_bits(&out.rows[0], &w[0], "wcprice row");
        }
    }

    #[test]
    fn avgprice_bit_exact() {
        use tulip_rs::indicators::avgprice::AvgPrice;
        let pp: [f64; 0] = [];
        let req = Request::new().add(AvgPrice::recipe(&pp), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let op: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [op.as_slice(), hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &streams).expect("avgprice registered");
            let (a, _) = AvgPrice::indicator(&streams, &pp, None).unwrap();
            assert_bits(&out.rows[0], &a[0], "avgprice row");
        }
    }

    #[test]
    fn smaenvelope_bit_exact() {
        use tulip_rs::indicators::smaenvelope::SmaEnvelope;
        for (p, pct) in [(5.0, 2.5), (14.0, 1.0)] {
            let pp: [f64; 2] = [p, pct];
            let req = Request::new().add(SmaEnvelope::recipe(&pp), None);
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("smaenvelope registered");
                let (se, _) = SmaEnvelope::indicator(&streams, &pp, None).unwrap();
                assert_bits(&out.rows[0], &se[0], "smaenvelope lower row");
                assert_bits(&out.rows[1], &se[1], "smaenvelope middle row");
                assert_bits(&out.rows[2], &se[2], "smaenvelope upper row");
            }
        }
    }

    // THE FOUR VOLUME/PRICE HUBS VIA THEIR OWN `Planable` FORWARDERS. Every
    // pre-existing gate for these kinds builds its request from the free
    // function `recipes::x(&[..])`, so the hand module's one-line forwarder was
    // the one thing in the engine no test reached -- the `registry.rs:406`
    // orphan shape ("a `Planable` impl that silently promises more than it
    // delivers"), here in its inverse: impl + arm + row + recipe all live, only
    // the trait call site missing. Same assertions, same columns, same
    // lead/order as the hub gates above -- only the entry point differs, so a
    // forwarder that drifted from its free function fails here and nowhere else.

    #[test]
    fn bop_planable_bit_exact() {
        use tulip_rs::indicators::bop::Bop;
        let pp: [f64; 0] = [];
        let req = Request::new().add(Bop::recipe(&pp), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let op: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [op.as_slice(), hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &streams).expect("bop via Planable registered");
            let (b, _) = Bop::indicator(&streams, &pp, None).unwrap();
            assert_bits(&out.rows[0], &b[0], "bop row [Planable]");
        }
    }

    #[test]
    fn emv_planable_bit_exact() {
        use tulip_rs::indicators::emv::Emv;
        let pp: [f64; 0] = [];
        let req = Request::new().add(Emv::recipe(&pp), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let vol = volume(s.len());
            let streams = [hi.as_slice(), lo.as_slice(), vol.as_slice()];
            let out = run_registered(&req, &streams).expect("emv via Planable registered");
            let (e, _) = Emv::indicator(&streams, &pp, None).unwrap();
            assert_bits(&out.rows[0], &e[0], "emv row [Planable]");
        }
    }

    #[test]
    fn marketfi_planable_bit_exact() {
        use tulip_rs::indicators::marketfi::Marketfi;
        let pp: [f64; 0] = [];
        let req = Request::new().add(Marketfi::recipe(&pp), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let vol: Vec<f64> = s.iter().map(|v| 1000.0 + v).collect();
            let streams = [hi.as_slice(), lo.as_slice(), vol.as_slice()];
            let out = run_registered(&req, &streams).expect("marketfi via Planable registered");
            let (m, _) = Marketfi::indicator(&streams, &pp, None).unwrap();
            assert_bits(&out.rows[0], &m[0], "marketfi row [Planable]");
        }
    }

    #[test]
    fn vortex_planable_bit_exact() {
        use tulip_rs::indicators::vortex::Vortex;
        const P: f64 = 14.0;
        let pp: [f64; 1] = [P];
        let req = Request::new().add(Vortex::recipe(&pp), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &streams).expect("vortex via Planable registered");
            let (v, _) = Vortex::indicator(&streams, &pp, None).unwrap();
            assert_bits(&out.rows[0], &v[0], "vi_up row [Planable]");
            assert_bits(&out.rows[1], &v[1], "vi_down row [Planable]");
        }
    }

    #[test]
    fn keltnerchannel_planable_bit_exact() {
        use tulip_rs::indicators::keltnerchannel::KeltnerChannel;
        const P: f64 = 14.0;
        const M: f64 = 2.0;
        let pp: [f64; 2] = [P, M];
        let req = Request::new().add(KeltnerChannel::recipe(&pp), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &streams).expect("keltner via Planable registered");
            let (k, _) = KeltnerChannel::indicator(&streams, &pp, None).unwrap();
            assert_bits(&out.rows[0], &k[0], "kc lower [Planable]");
            assert_bits(&out.rows[1], &k[1], "kc middle [Planable]");
            assert_bits(&out.rows[2], &k[2], "kc upper [Planable]");
        }
    }

    // THE OTHER SIX VOLUME/PRICE HUBS VIA THEIR OWN `Planable` FORWARDERS. Same
    // reason as the four above: every pre-existing gate for these kinds calls the
    // free function `recipes::x(&[..])`, so without these the hand module's
    // forwarder compiles with nothing exercising it. Column sets copied from the
    // existing gates for each kind -- NOT re-derived -- because a mis-ranked slot
    // is invisible to a green suite (the positional-stream trap).

    #[test]
    fn obv_planable_bit_exact() {
        use tulip_rs::indicators::obv::Obv;
        let req = Request::new().add(Obv::recipe(&[]), None);
        for s in [
            series(400, |i| i as f64),
            series(400, |i| 400.0 - i as f64),
            // The plateau carries TIES (`close == prev`), which is the only shape
            // that separates the two `WhereGt` spelling from a `WhereGe` one.
            series(400, |i| 100.0 + ((i / 3) % 2) as f64),
            series(400, |_| 100.0),
        ] {
            let vol = volume(s.len());
            let cv = [s.as_slice(), vol.as_slice()];
            let out = run_registered(&req, &cv).expect("obv via Planable registered");
            let (o, _) = Obv::indicator(&cv, &[], None).unwrap();
            assert_bits(&out.rows[0], &o[0], "obv row [Planable]");
        }
    }

    #[test]
    fn wad_planable_bit_exact() {
        use tulip_rs::indicators::wad::Wad;
        let pp: [f64; 0] = [];
        let req = Request::new().add(Wad::recipe(&pp), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(680, |i| 200.0 - i as f64 * 0.05),
            series(680, |i| 100.0 + ((i / 3) % 2) as f64),
            series(680, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let streams = [hi.as_slice(), lo.as_slice(), s.as_slice()];
            let out = run_registered(&req, &streams).expect("wad via Planable registered");
            let (w, _) = Wad::indicator(&streams, &pp, None).unwrap();
            assert_bits(&out.rows[0], &w[0], "wad row [Planable]");
        }
    }

    #[test]
    fn mfi_planable_bit_exact() {
        use tulip_rs::indicators::mfi::Mfi;
        for p in [1.0, 2.0, 5.0, 14.0] {
            let pp: [f64; 1] = [p];
            let req = Request::new().add(Mfi::recipe(&pp), None);
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |i| 200.0 - i as f64 * 0.05),
                series(680, |_| 100.0),
                series(p as usize + 1, |i| 100.0 + i as f64),
            ] {
                let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
                let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
                let vol = volume(s.len());
                let streams = [hi.as_slice(), lo.as_slice(), s.as_slice(), vol.as_slice()];
                let out = run_registered(&req, &streams).expect("mfi via Planable registered");
                let (m, _) = Mfi::indicator(&streams, &pp, None).unwrap();
                assert_bits(&out.rows[0], &m[0], "mfi row [Planable]");
            }
        }
    }

    #[test]
    fn vwap_planable_bit_exact() {
        use tulip_rs::indicators::vwap::Vwap;
        let req = Request::new().add(Vwap::recipe(&[]), Some(&[true]));
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let vol = volume(s.len());
            let four = [hi.as_slice(), lo.as_slice(), s.as_slice(), vol.as_slice()];
            let out = run_registered(&req, &four).expect("vwap via Planable registered");
            let (v, _) = Vwap::indicator(&four, &[], Some(&[true])).unwrap();
            assert_bits(&out.rows[0], &v[0], "vwap row [Planable]");
            assert_bits(&out.rows[1], &v[1], "vwap typprice optional [Planable]");
        }
    }

    #[test]
    fn vwma_planable_bit_exact() {
        use tulip_rs::indicators::vwma::Vwma;
        let req = Request::new().add(Vwma::recipe(&[9.0]), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let vol = volume(s.len());
            let two_cv = [s.as_slice(), vol.as_slice()];
            let out = run_registered(&req, &two_cv).expect("vwma via Planable registered");
            let (v, _) = Vwma::indicator(&two_cv, &[9.0], None).unwrap();
            assert_bits(&out.rows[0], &v[0], "vwma row [Planable]");
        }
        // ZERO VOLUME: the only input that reaches the `WhereLe` pin. Positive
        // prices and volumes pass both a correct guard and an inverted one.
        {
            let n = 200;
            let s = series(n, |i| 100.0 + (i % 3) as f64);
            let zero = vec![0.0; n];
            let two_cv = [s.as_slice(), zero.as_slice()];
            let out = run_registered(&req, &two_cv).expect("vwma zero-volume via Planable");
            let (v, _) = Vwma::indicator(&two_cv, &[9.0], None).unwrap();
            assert_bits(&out.rows[0], &v[0], "vwma ZERO-VOLUME row [Planable]");
        }
    }

    #[test]
    fn vosc_planable_bit_exact() {
        use tulip_rs::indicators::vosc::Vosc;
        let req = Request::new().add(Vosc::recipe(&[3.0, 5.0]), None);
        for n in [680usize, 700] {
            let vol = volume(n);
            let one_v = [vol.as_slice()];
            let out = run_registered(&req, &one_v).expect("vosc via Planable registered");
            let (v, _) = Vosc::indicator(&one_v, &[3.0, 5.0], None).unwrap();
            assert_bits(&out.rows[0], &v[0], "vosc row [Planable]");
        }
    }

    #[test]
    fn qstick_planable_bit_exact() {
        use tulip_rs::indicators::qstick::QStick;
        let req = Request::new().add(QStick::recipe(&[9.0]), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let op: Vec<f64> = s.iter().map(|v| v - 0.5).collect();
            let two_oc = [op.as_slice(), s.as_slice()];
            let out = run_registered(&req, &two_oc).expect("qstick via Planable registered");
            let (q, _) = QStick::indicator(&two_oc, &[9.0], None).unwrap();
            assert_bits(&out.rows[0], &q[0], "qstick row [Planable]");
        }
    }

    #[test]
    fn chaikinmf_planable_bit_exact() {
        use tulip_rs::indicators::chaikinmf::ChaikinMf;
        let req = Request::new().add(ChaikinMf::recipe(&[9.0]), None);
        for s in [
            wiggle(),
            series(680, |i| 100.0 + i as f64 * 0.05),
            series(700, |_| 100.0),
        ] {
            let hi: Vec<f64> = s.iter().map(|v| v + 2.0).collect();
            let lo: Vec<f64> = s.iter().map(|v| v - 1.0).collect();
            let vol = volume(s.len());
            let four = [hi.as_slice(), lo.as_slice(), s.as_slice(), vol.as_slice()];
            let out = run_registered(&req, &four).expect("chaikinmf via Planable registered");
            let (c, _) = ChaikinMf::indicator(&four, &[9.0], None).unwrap();
            assert_bits(&out.rows[0], &c[0], "chaikinmf row [Planable]");
        }
    }

    /// `linreg` via `Node::LinregFlow`, the publish-before-evict moment carry.
    ///
    /// BOTH optional rows are checked, not just the primary: the engine always
    /// runs the hand's `calc` path, so the whole three-row triple is the
    /// contract. The hand is therefore called with `Some(&[true, true])`, which
    /// is the branch that produces these rows — its default `partial_calc`
    /// branch is a DIFFERENT expression (it collapses the same two moments
    /// through pre-multiplied `xy_coef`/`y_coef`) and disagrees by up to
    /// ~9e-16 relative, inside the authorised 1e-12 tolerance.
    ///
    /// The wiggle and ramp series are load-bearing, not decoration: a FLAT
    /// series cannot see the window-fold shape at all. Measured — swapping
    /// `sum_y` for a `RollSum`-folded window leaves flat at 0/698 but breaks
    /// 697/698 on ramp p=2 and 685/686 on wiggle p=14, because the hand evicts
    /// AFTER publishing while every `RollSum` region evicts inside what it
    /// prints. A flat-only gate would wave that through.
    #[test]
    fn linreg_planable_bit_exact() {
        use tulip_rs::indicators::linreg::Linreg;
        for p in [2.0, 5.0, 14.0, 27.0] {
            let pp: [f64; 1] = [p];
            let req = Request::new().add(Linreg::recipe(&pp), Some(&[true, true]));
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("linreg via Planable registered");
                let (h, _) = Linreg::indicator(&streams, &pp, Some(&[true, true])).unwrap();
                assert_bits(&out.rows[0], &h[0], "linreg row [Planable]");
                assert_bits(&out.rows[1], &h[1], "linregslope row [Planable]");
                assert_bits(&out.rows[2], &h[2], "linregintercept row [Planable]");
            }
        }
    }

    /// `tsf` = the linreg chain + ONE `Fma`. Gated against the hand's four rows
    /// in its own return order, `[tsf, linreg, slope, intercept]`.
    ///
    /// The `linreg` row here is the point: `cycle_tsf` calls `calc`
    /// unconditionally, so it must agree with the standalone `linreg` request
    /// bar for bar -- and it does, which is the premise that lets the pair share
    /// one chain (asserted structurally by `linreg_tsf_shares_one_chain` below).
    #[test]
    fn tsf_planable_bit_exact() {
        use tulip_rs::indicators::tsf::Tsf;
        for p in [2.0, 5.0, 14.0, 27.0] {
            let pp: [f64; 1] = [p];
            let req = Request::new().add(Tsf::recipe(&pp), Some(&[true, true, true]));
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("tsf via Planable registered");
                let (h, _) = Tsf::indicator(&streams, &pp, Some(&[true, true, true])).unwrap();
                assert_bits(&out.rows[0], &h[0], "tsf row [Planable]");
                assert_bits(&out.rows[1], &h[1], "tsf/linreg row [Planable]");
                assert_bits(&out.rows[2], &h[2], "tsf/linregslope row [Planable]");
                assert_bits(&out.rows[3], &h[3], "tsf/linregintercept row [Planable]");
            }
        }
    }

    /// The sharing claim, measured rather than asserted in prose: `linreg` and
    /// `tsf` request the SAME moment chain, so the merged arena must carry TWO
    /// `K_LINREGFLOW` nodes in total -- not four. Two hand-written copies of a
    /// chain this size drift eventually; both recipes call `recipes::linreg_chain`
    /// precisely so this test cannot go red for that reason.
    #[test]
    fn linreg_tsf_shares_one_chain() {
        use tulip_rs::cse::kernel::registry::SIG_linreg_tsf_shared_p;
        use tulip_rs::cse::signature::K_LINREGFLOW;
        use tulip_rs::indicators::linreg::Linreg;
        use tulip_rs::indicators::tsf::Tsf;
        let pair = Request::new()
            .add(Linreg::recipe(&[14.0]), Some(&[true, true]))
            .add(Tsf::recipe(&[14.0]), Some(&[true, true, true]));
        let (merged, sig, _, _) = erase_request(&pair).unwrap();
        assert_eq!(sig, SIG_linreg_tsf_shared_p, "pair arena drifted");
        assert_eq!(merged.nodes.len(), sig.len as usize, "arena length");
        let count = |kinds: &[u8; MAX], len: u32| {
            kinds[..len as usize]
                .iter()
                .filter(|&&x| x == K_LINREGFLOW)
                .count()
        };
        let solo_l = erase_request(&Request::new().add(Linreg::recipe(&[14.0]), None)).unwrap();
        let solo_t = erase_request(&Request::new().add(Tsf::recipe(&[14.0]), None)).unwrap();
        // Counted off each arena's OWN live slots, so the merged number is
        // compared against the solos instead of asserted from a guess.
        assert_eq!(
            (
                count(&solo_l.1.kinds, solo_l.1.len),
                count(&solo_t.1.kinds, solo_t.1.len)
            ),
            (2, 2),
            "each request solo owns one moment pair"
        );
        assert_eq!(
            count(&sig.kinds, sig.len),
            2,
            "the pair shares ONE moment chain (two LinregFlow nodes), not two pairs"
        );
        // MEASURED, then pinned: the pair costs exactly tsf's own arena,
        // because linreg's 14 nodes are a strict SUBSET of tsf's 16 -- the two
        // extra are tsf's `Const(n+1)` and its one `Fma`. So adding `linreg` to
        // a `tsf` request is FREE: 0 extra nodes against the 14 a second chain
        // would cost. The loose `<` would have stayed green if the subset
        // property ever broke while some other pair still deduped, so the exact
        // sizes are the assertion.
        assert_eq!(sig.len, 16, "merged arena: tsf's 16 nodes, no more");
        assert_eq!(solo_l.1.len, 14, "solo linreg: the shared chain alone");
        assert_eq!(solo_t.1.len, 16, "solo tsf: the chain + Const + Fma");
        assert_eq!(
            sig.len, solo_t.1.len,
            "linreg adds NOTHING to tsf's arena -- it is a strict subset"
        );
    }

    /// `fosc` = the tsf chain + `RollAge{tsf,1}` + Sub/Mul/Div, gated on all FIVE
    /// rows in the hand's return order `[fosc, tsf, linreg, slope, intercept]`.
    ///
    /// The flat series is load-bearing, not decoration: fosc's published lag is
    /// a ring over a COMPUTED source (the tsf node), and a derived-source roll
    /// seeds from the source's materialized value -- a phantom `0.0` there is
    /// invisible to a trend and only shows up on flat/all-equal data. Same class
    /// that poisoned `RollMin` before the cold-start init pass materialized
    /// bar 0.
    ///
    /// Row LENGTHS differ on purpose: fosc overrides `Fosc::min_data` to
    /// `options[0] + 2` so its own row starts at bar
    /// `period + 1`, while the four re-published regression rows keep tsf's
    /// lead `period`. `assert_bits` length-asserts, so a wrong lead on either
    /// family fails here instead of silently truncating.
    #[test]
    fn fosc_planable_bit_exact() {
        use tulip_rs::indicators::fosc::Fosc;
        for p in [2.0, 5.0, 14.0, 27.0] {
            let pp: [f64; 1] = [p];
            let req = Request::new().add(Fosc::recipe(&pp), Some(&[true, true, true, true]));
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("fosc via Planable registered");
                let (h, _) =
                    Fosc::indicator(&streams, &pp, Some(&[true, true, true, true])).unwrap();
                assert_bits(&out.rows[0], &h[0], "fosc row [Planable]");
                assert_bits(&out.rows[1], &h[1], "fosc/tsf row [Planable]");
                assert_bits(&out.rows[2], &h[2], "fosc/linreg row [Planable]");
                assert_bits(&out.rows[3], &h[3], "fosc/linregslope row [Planable]");
                assert_bits(&out.rows[4], &h[4], "fosc/linregintercept row [Planable]");
            }
        }
    }

    /// Three requests, ONE moment chain: `linreg + tsf + fosc` must cost exactly
    /// what `fosc` alone costs, because fosc's arena is a superset of tsf's
    /// which is a superset of linreg's. Asserted RELATIONALLY against solo
    /// fosc, so the number is measured, not a constant I typed from a guess.
    #[test]
    fn linreg_tsf_fosc_share_one_chain() {
        use tulip_rs::cse::kernel::registry::SIG_linreg_tsf_fosc_shared_p;
        use tulip_rs::cse::signature::K_LINREGFLOW;
        use tulip_rs::indicators::fosc::Fosc;
        use tulip_rs::indicators::linreg::Linreg;
        use tulip_rs::indicators::tsf::Tsf;
        let trio = Request::new()
            .add(Linreg::recipe(&[14.0]), None)
            .add(Tsf::recipe(&[14.0]), None)
            .add(Fosc::recipe(&[14.0]), None);
        let (_, sig, _, _) = erase_request(&trio).unwrap();
        assert_eq!(sig, SIG_linreg_tsf_fosc_shared_p, "trio arena drifted");
        let moments = |kinds: &[u8; MAX], len: u32| {
            kinds[..len as usize]
                .iter()
                .filter(|&&x| x == K_LINREGFLOW)
                .count()
        };
        let solo_f = erase_request(&Request::new().add(Fosc::recipe(&[14.0]), None)).unwrap();
        // ONE moment chain across all three requests, not three.
        assert_eq!(moments(&sig.kinds, sig.len), 2, "trio shares one chain");
        assert_eq!(
            sig.len, solo_f.1.len,
            "linreg + tsf add NOTHING to fosc's arena: {} vs {}",
            sig.len, solo_f.1.len
        );
    }

    /// ZLEMA(p) seeds its EMA from real[lag-1] (zlema.rs:42, lag = ((p-1)/2).max(1)),
    /// which the engine spells via Node::SeededEma{feed: Some(lag-1)}. Test multiple
    /// periods (odd and even, since lag depends on (period-1)/2) and multiple series
    /// shapes (ramp, flat, wiggle/noise). Flat-series-only tests hide real bugs — the
    /// seed gap is invisible to a constant series.
    #[test]
    fn zlema_planable_bit_exact() {
        use tulip_rs::indicators::zlema::Zlema;
        for p in [2.0, 5.0, 14.0, 27.0] {
            let pp: [f64; 1] = [p];
            let req = Request::new().add(Zlema::recipe(&pp), None);
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("zlema via Planable registered");
                let (h, _) = Zlema::indicator(&streams, &pp, None).unwrap();
                assert_bits(&out.rows[0], &h[0], "zlema row [Planable]");
            }
        }
    }

    /// KAMA(p) uses VariableAlphaEma with live w from efficiency_ratio^2.
    /// Seeds at bar period-1, output starts at bar period.
    ///
    #[test]
    fn kama_planable_bit_exact() {
        use tulip_rs::indicators::kama::Kama;
        for p in [5.0, 10.0, 30.0] {
            let pp: [f64; 1] = [p];
            // Request with both primary and optional output enabled
            let req = Request::new().add(Kama::recipe(&pp), Some(&[true, true]));
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("kama via Planable registered");
                let (rows, _state) = Kama::indicator(&streams, &pp, Some(&[true, true])).unwrap();
                assert_bits(&out.rows[0], &rows[0], "kama row [Planable]");
                assert_bits(&out.rows[1], &rows[1], "kama ef row [Planable]");
            }
        }
    }

    /// VIDYA(short, long, alpha) uses VariableAlphaVidya with live w from (short_stddev/long_stddev)*alpha.
    /// Seeds at bar long_period-2, output starts at bar long_period-1.
    ///
    /// PARKED (2026-11-03): `recipes::vidya`'s optional outputs (short_sma,
    /// long_sma, short_stddev, long_stddev) are now bit-exact against the
    /// hand (fixed this session: they need an explicit `Sink::iface_lead(_,
    /// period)`, not plain `iface()` -- `iface()`'s lead=0 is a sentinel
    /// meaning "use the node's bare *structural* lead", which for the shared
    /// `Stddev` node's `K_STDDEV` exec arm is `period` already (its `t < p`
    /// branch returns 0.0 for `t` in `0..period`, first real value at
    /// `t == period`) -- so this part was never actually broken, `iface()`
    /// just wasn't passing that lead through to the SINK's own trim.
    ///
    /// The PRIMARY row is still wrong and NOT a trim bug: VIDYA's hand
    /// computes its first published value (absolute bar `long_period-1`)
    /// from a ONE-TIME, non-rolling, "naive" stddev over the raw window
    /// `real[0..=long_period-1]` (`State::init_state`'s `sum_long`/
    /// `sum_sq_long` loop, accumulated unconditionally every iteration,
    /// published once, outside any roll) -- *before* its rolling
    /// `StddevState::calc` state exists at all. The shared `recipes::stddev`
    /// node has no such naive-then-roll duality: structurally it is NOT
    /// valid until `t == period` (confirmed by direct `erase_request`
    /// inspection this session: `vals.feeds[k]` for the `VariableAlpha` node
    /// is correctly `18` as recipe-specified, but `v[long_sd_id]` at
    /// `t == 19` is still its pre-validity `0.0`, so the `WhereLt` guard
    /// zeroes `w`, and the Vidya fold `(src-carry).mul_add(0, carry)`
    /// degenerates to a no-op publish of the raw seed -- measured directly:
    /// `out.rows[0][0] == real[long_period-2]` bit-for-bit, the UNFOLDED
    /// seed, not the hand's folded value).
    ///
    /// Shifting `feed` to `long_period-1` (so the fold waits until both
    /// stddevs are structurally valid at `t == long_period`) would fix the
    /// mechanism but publish one bar LATER than the hand's actual first
    /// value -- the hand's first bar is fundamentally a different formula
    /// (naive window) from every subsequent bar (rolling window), and nothing
    /// in the current `Node::Stddev`/`VariableAlpha` vocabulary can express
    /// "naive on the seed bar, rolling from the next bar on" within ONE
    /// shared node. Fixing this for real needs either a dedicated one-off
    /// naive-stddev subgraph just for VIDYA's seed bar (duplicate, not
    /// shared), or a new `Node::Stddev` variant/field carrying that
    /// seed-vs-roll duality generally (closer in cost to `VariableAlpha`
    /// itself than to the `SeededEma`-style `feed` override already built).
    /// Un-ignore once one of those is built; see `ENGINE_CHECKLIST.md`'s F1
    /// vidya entry for the live status.
    #[test]
    fn vidya_planable_bit_exact() {
        use tulip_rs::indicators::vidya::Vidya;
        const OPTS: [([f64; 3], &[&str]); 3] = [
            ([5.0, 20.0, 0.5], &["short", "long", "alpha"]),
            ([9.0, 30.0, 0.65], &["short", "long", "alpha"]),
            ([3.0, 10.0, 0.2], &["short", "long", "alpha"]),
        ];
        for &(ref opts, _) in &OPTS {
            let req =
                Request::new().add(Vidya::recipe(opts), Some(&[true, true, true, true, true]));
            for s in [
                wiggle(),
                series(680, |i| 100.0 + i as f64 * 0.05),
                series(680, |_| 100.0),
            ] {
                let streams = [s.as_slice()];
                let out = run_registered(&req, &streams).expect("vidya via Planable registered");
                let (rows, _state) =
                    Vidya::indicator(&streams, opts, Some(&[true, true, true, true, true]))
                        .unwrap();
                assert_bits(&out.rows[0], &rows[0], "vidya row [Planable]");
                assert_bits(&out.rows[1], &rows[1], "vidya short_sma row [Planable]");
                assert_bits(&out.rows[2], &rows[2], "vidya long_sma row [Planable]");
                assert_bits(&out.rows[3], &rows[3], "vidya short_stddev row [Planable]");
                assert_bits(&out.rows[4], &rows[4], "vidya long_stddev row [Planable]");
            }
        }
    }

    /// `psar` = raw high/low directly feeding the bespoke `Node::Psar` carry
    /// (no graft chain, unlike `SuperTrend`). Bar 1 is the seed (the hand's
    /// `State::new`/`calc` at i=1 skip the lookback-2 clamp
    /// `calc_unchecked` applies from bar 2 on) -- this gate is the proof
    /// that seed-vs-steady split is wired correctly across BOTH the
    /// `warmup.rs` interpreter (bar 1 only, standalone) and `codegen.rs`'s
    /// steady loop (bar 2 on).
    #[test]
    fn psar_planable_bit_exact() {
        use tulip_rs::indicators::psar::Psar;
        for (af_step, max_af) in [(0.02, 0.2), (0.01, 0.1), (0.05, 0.3)] {
            let opts: [f64; 2] = [af_step, max_af];
            let req = Request::new().add(Psar::recipe(&opts), None);
            for n in [3usize, 40, 700] {
                let high = series(n, |i| (i as f64 * 0.31).sin() * 8.0 + 105.0);
                let low = series(n, |i| (i as f64 * 0.31).sin() * 8.0 + 95.0);
                let streams = [high.as_slice(), low.as_slice()];
                let out = run_registered(&req, &streams).expect("psar via Planable registered");
                let inp = [high.as_slice(), low.as_slice()];
                let (h, _) = Psar::indicator(&inp, &opts, None).unwrap();
                assert_bits(&out.rows[0], &h[0], "psar row [Planable]");
            }
        }
    }

    /// `msw` = MESA Sine Wave: a single raw price stream feeding a sliding-DFT
    /// rotation carry (`Node::SdftRotate`, publishes `sine`) plus a companion
    /// `Node::SdftLead` node (publishes the cached `lead_sine`). This gate
    /// covers periods both <=7 and >7 (the hand's N=4 vs N=8 SIMD-width seed
    /// dot-product split) and both the seed bar and steady bars.
    #[test]
    fn msw_planable_bit_exact() {
        use tulip_rs::indicators::msw::Msw;
        for period in [6usize, 7, 14, 27, 50] {
            let opts: [f64; 1] = [period as f64];
            let req = Request::new().add(Msw::recipe(&opts), None);
            for n in [period + 5, period + 40, period + 700] {
                let prices = series(n, |i| (i as f64 * 0.31).sin() * 8.0 + 105.0);
                let streams = [prices.as_slice()];
                let out = run_registered(&req, &streams).expect("msw via Planable registered");
                let inp = [prices.as_slice()];
                let (h, _) = Msw::indicator(&inp, &opts, None).unwrap();
                assert_bits(&out.rows[0], &h[0], "msw sine row [Planable]");
                assert_bits(&out.rows[1], &h[1], "msw lead row [Planable]");
            }
        }
    }

    /// Volatility standalone floor — parity vs hand (1e-12 relative).
    ///
    /// The CSE volatility recipe computes rate-of-change values via lag/sub/div,
    /// then uses two rolling sums to compute variance (mean_sq - mean²), and finally
    /// scales stddev by the annual factor 252.sqrt(). This matches the hand's
    /// semantics: `((real[i] - prev_real) / prev_real)` as rate-of-change, then
    /// std dev over those values with ANNUAL multiplier.
    #[test]
    fn volatility_planable_bit_exact() {
        let s = series(700, |i| {
            (i as f64 * 0.37).sin() * 10.0 + 100.0 + i as f64 * 0.02
        });
        for period in [6usize, 7, 14, 27, 50] {
            let opts: [f64; 1] = [period as f64];
            let req = Request::new().add(Volatility::recipe(&opts), None);
            let streams = [s.as_slice()];
            let out = run_registered(&req, &streams)
                .unwrap_or_else(|| panic!("volatility: registry miss - pattern not registered"));
            assert_eq!(out.fp, SIG_volatility_n.hash());
            let inp = [s.as_slice()];
            let (h, _) = Volatility::indicator(&inp, &opts, None).unwrap();
            // CSE computes variance via RollSum nodes on derived ROC values;
            // hand uses stddev internal two-moment state. Floating-point rounding
            // differs slightly but stays within 1e-12 relative tolerance.
            assert_rel_1e12(&out.rows[0], &h[0], &format!("volatility p={period}"));
        }
    }

    /// `homodynediscriminator` = Ehlers Homodyne Discriminator: one raw price stream
    /// feeding a single bespoke carry node (`Node::HomodyneDiscriminator`, 7-state
    /// ring-backed recurrence). Fixed 23-bar warmup (bars 0-21), lead 22. No user
    /// options.
    #[test]
    fn homodynediscriminator_planable_bit_exact() {
        use tulip_rs::indicators::homodynediscriminator::HomodyneDiscriminator;
        let req = Request::new().add(HomodyneDiscriminator::recipe(&[]), None);
        for n in [23usize, 40, 700] {
            let prices = series(n, |i| (i as f64 * 0.31).sin() * 8.0 + 105.0);
            let streams = [prices.as_slice()];
            let out = run_registered(&req, &streams)
                .expect("homodynediscriminator via Planable registered");
            let inp = [prices.as_slice()];
            let (h, _) = HomodyneDiscriminator::indicator(&inp, &[], None).unwrap();
            assert_bits(
                &out.rows[0],
                &h[0],
                "homodynediscriminator dc_period row [Planable]",
            );
        }
    }

    #[test]
    fn cybercycle_planable_bit_exact() {
        use tulip_rs::indicators::cybercycle::Cybercycle;
        for alpha in [0.2, 0.5, 0.8] {
            let req = Request::new().add(Cybercycle::recipe(&[alpha]), None);
            for n in [7usize, 40, 700] {
                let prices = series(n, |i| (i as f64 * 0.31).sin() * 8.0 + 105.0);
                let streams = [prices.as_slice()];
                let out =
                    run_registered(&req, &streams).expect("cybercycle via Planable registered");
                let inp = [prices.as_slice()];
                let (c, _) = Cybercycle::indicator(&inp, &[alpha], None).unwrap();
                assert_bits(
                    &out.rows[0],
                    &c[0],
                    &format!("cybercycle row [Planable] alpha={}", alpha),
                );
            }
        }
    }

    #[test]
    fn instantaneoustrendline_planable_bit_exact() {
        use tulip_rs::indicators::instantaneoustrendline::InstantaneousTrendline;
        // First test with primary output only to check basic functionality
        let req = Request::new().add(InstantaneousTrendline::recipe(&[]), None);
        for n in [23usize, 40, 700] {
            let prices = series(n, |i| (i as f64 * 0.31).sin() * 8.0 + 105.0);
            let streams = [prices.as_slice()];
            let out = run_registered(&req, &streams)
                .expect("instantaneoustrendline via Planable registered");
            let inp = [prices.as_slice()];
            // IT has primary trendline only (no optionals in recipe yet)
            let (results, _) = InstantaneousTrendline::indicator(&inp, &[], None).unwrap();

            assert_bits(
                &out.rows[0],
                &results[0],
                "instantaneoustrendline trendline row [Planable]",
            );
        }
    }

    #[test]
    fn mama_planable_bit_exact() {
        use tulip_rs::indicators::mama::Mama;
        // Test with default options (fast_limit=0.5, slow_limit=0.05)
        let fast_limit = 0.5;
        let slow_limit = 0.05;
        let req = Request::new().add(Mama::recipe(&[fast_limit, slow_limit]), None);
        for n in [23usize, 40, 700] {
            let prices = series(n, |i| (i as f64 * 0.31).sin() * 8.0 + 105.0);
            let streams = [prices.as_slice()];
            let out = run_registered(&req, &streams).expect("mama via Planable registered");
            let inp = [prices.as_slice()];
            // Mama has primary outputs (mama, fama) and optionals (dc_period, alpha)
            let (results, _) = Mama::indicator(
                &inp,
                &[fast_limit, slow_limit],
                None, // no optional outputs in test
            )
            .unwrap();

            assert_bits(
                &out.rows[0],
                &results[0],
                &format!("mama mama row [Planable] n={}", n),
            );
            assert_bits(
                &out.rows[1],
                &results[1],
                &format!("mama fama row [Planable] n={}", n),
            );
        }
    }

    #[test]
    fn trendmode_planable_bit_exact() {
        use tulip_rs::indicators::trendmode::TrendMode;
        for alpha in [0.07, 0.2] {
            let req = Request::new().add(TrendMode::recipe(&[alpha]), None);
            for n in [56usize, 100, 700] {
                let prices = series(n, |i| (i as f64 * 0.31).sin() * 8.0 + 105.0);
                let streams = [prices.as_slice()];
                let out =
                    run_registered(&req, &streams).expect("trendmode via Planable registered");
                let inp = [prices.as_slice()];
                // trendmode has primary output (trend) and optionals (cycle, peak)
                let (results, _state) =
                    TrendMode::indicator(&inp, &[alpha], Some(&[true, true])).unwrap();
                assert_bits(
                    &out.rows[0],
                    &results[0],
                    &format!("trendmode row [Planable] alpha={} n={}", alpha, n),
                );
            }
        }
    }

    #[test]
    fn ccfisher_planable_bit_exact() {
        use tulip_rs::indicators::ccfisher::CcFisher;
        for alpha in [0.07, 0.2] {
            let req = Request::new().add(CcFisher::recipe(&[alpha]), None);
            for n in [56usize, 100, 700] {
                let prices = series(n, |i| (i as f64 * 0.31).sin() * 8.0 + 105.0);
                let streams = [prices.as_slice()];
                let out = run_registered(&req, &streams).expect("ccfisher via Planable registered");
                let inp = [prices.as_slice()];
                // ccfisher has primary outputs (fisher, signal) and optionals (trendmode, cycle, peak)
                let (results, _state) =
                    CcFisher::indicator(&inp, &[alpha], Some(&[true, true, true])).unwrap();
                assert_bits(
                    &out.rows[0],
                    &results[0],
                    &format!("ccfisher fisher row [Planable] alpha={} n={}", alpha, n),
                );
                assert_bits(
                    &out.rows[1],
                    &results[1],
                    &format!("ccfisher signal row [Planable] alpha={} n={}", alpha, n),
                );
            }
        }
    }
}
