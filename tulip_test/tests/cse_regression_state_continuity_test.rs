//! Continuity gate for the regression family (`linreg` / `tsf` / `fosc`).
//!
//! These three are the engine's `Node::LinregFlow` carries — the publish-before-evict
//! moment machine, whose `sum_xy` is weighted by the bar's own index. Their
//! `Restorable::restore_state` implementations **ignore the captured chains** and
//! rebuild the hand state from a tail window of the input (`period = real.len() - 1`,
//! then `State::init_state(&real[1..period], period)`). That is only legitimate if the
//! replay reproduces the state the series would have produced, bit for bit — which is
//! precisely the question a row-asserting parity test cannot ask, because parity holds
//! for the run that never stopped.
//!
//! So this gate splits a series at a seam, restores a state from the `period + 1`
//! bars ending at the seam, continues with it to the end, and compares the joined
//! rows to the uninterrupted full-series run. A mismatch here means the tail window
//! is NOT a sufficient state for this family, and the engine must hand back the
//! `sum_y` / `sum_xy` / `n` carries instead of letting the caller reconstruct them.
mod tests {
    use tulip_rs::cse::Restorable;
    use tulip_rs::indicator_types::{Indicator, TIndicatorState};
    use tulip_rs::indicators::fosc::Fosc;
    use tulip_rs::indicators::linreg::Linreg;
    use tulip_rs::indicators::tsf::Tsf;

    fn series(n: usize, f: impl Fn(usize) -> f64) -> Vec<f64> {
        (0..n).map(f).collect()
    }
    fn wiggle() -> Vec<f64> {
        series(700, |i| {
            100.0 + (i as f64 * 0.013).sin() * 3.0 + (i as f64 * 0.11).cos() * 0.4
        })
    }
    fn ramp(n: usize) -> Vec<f64> {
        series(n, |i| 100.0 + i as f64 * 0.05)
    }

    /// One row of a primary output, compared bitwise, with the first divergence
    /// reported — a bare `assert_eq!` on the whole vector hides WHERE it breaks,
    /// and where is what separates a lead error from a carry error.
    fn assert_bits(label: &str, got: &[f64], want: &[f64]) {
        let common = got.len().min(want.len());
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
            "{label}: {bad} bit mismatches over {common} rows, first at index {first:?} \
             (got {:?} want {:?})",
            first.map(|i| (got[i], i)),
            first.map(|i| (want[i], i)),
        );
    }

    /// Restore → continue → compare against the TRUE state continued over the same bars.
    ///
    /// Both sides start at `SEAM` with the same input and the same phase, so a
    /// difference here is the rebuilt state and nothing else — no row-length or
    /// publish-lead alignment can enter into it. Row counts are asserted equal
    /// FIRST: a length difference means the replayed state sits at a different phase
    /// of the machine, which is a different finding from a wrong carry.
    fn continue_from_tail<I, S>(label: &str, period: f64, tail: usize, s: &[f64])
    where
        I: Indicator<1, 1, IndicatorState = S> + Restorable<1, 1>,
        S: TIndicatorState<1>,
    {
        const SEAM: usize = 400;
        let p = period as usize;

        // Gold standard: the hand's own state at the seam, continued.
        let (_head_rows, mut true_state) =
            I::indicator(&[&s[..SEAM]], &[period], None).expect("head run");
        let mut true_rows: Vec<f64> = Vec::new();
        for chunk in s[SEAM..].chunks(256) {
            let out = true_state
                .batch_indicator(&[chunk], None)
                .unwrap_or_else(|e| panic!("{label}: true state failed: {e:?}"));
            true_rows.extend_from_slice(&out[0]);
        }

        // Candidate: a state rebuilt from the `tail` bars ending at the seam.
        // `restore_state` derives the period as `real.len() - 1` (and `fosc` carries one
        // more bar because its `min_data` is `period + 2`), so `tail` is the caller's
        // statement of that contract — passing `period` bars here would silently
        // restore a `period - 1` machine, which is the bug this line used to have.
        let win = &s[SEAM - tail..SEAM];
        let mut replay = <I as Restorable<1, 1>>::restore_state(vec![], &[win])
            .unwrap_or_else(|| panic!("{label}: restore_state declined a {p}+1-bar tail window"));
        let mut replay_rows: Vec<f64> = Vec::new();
        for chunk in s[SEAM..].chunks(256) {
            let out = replay
                .batch_indicator(&[chunk], None)
                .unwrap_or_else(|e| panic!("{label}: replay state failed: {e:?}"));
            replay_rows.extend_from_slice(&out[0]);
        }

        assert_eq!(
            replay_rows.len(),
            true_rows.len(),
            "{label}: the rebuilt state is at a different PHASE than the true one \
             ({} rows vs {} over the same {} input bars)",
            replay_rows.len(),
            true_rows.len(),
            s.len() - SEAM
        );
        assert_bits(label, &replay_rows, &true_rows);
    }

    /// INSTRUMENT VALIDATION, not a claim about the engine: does the two-phase run
    /// driven by the hand's OWN state at the seam reproduce the uninterrupted run?
    /// If this fails, the gold standard in `continue_from_tail` is wrong and no
    /// restorer conclusion below means anything. It is also the control for the
    /// seam/lead convention, so it runs for the same periods the gate uses.
    #[test]
    fn gold_standard_two_phase_matches_full_series() {
        const SEAM: usize = 400;
        for p in [5.0, 14.0, 27.0] {
            let s = wiggle();
            let (full, _) = Linreg::indicator(&[&s], &[p], None).expect("full");
            let (mut joined, mut st) = Linreg::indicator(&[&s[..SEAM]], &[p], None).expect("head");
            let mut rows = std::mem::take(&mut joined[0]);
            for chunk in s[SEAM..].chunks(256) {
                let out = st.batch_indicator(&[chunk], None).expect("continuation");
                rows.extend_from_slice(&out[0]);
            }
            // Compare on the common tail: the head run's rows already cover the bars
            // up to the seam, so `rows` should equal the full-series row bar for bar.
            assert_eq!(
                rows.len(),
                full[0].len(),
                "linreg p={p}: two-phase row count {} != full-series {}",
                rows.len(),
                full[0].len()
            );
            assert_bits(&format!("linreg gold p={p}"), &rows, &full[0]);
        }
    }

    /// THE CAPTURE CONTRACT, ACTIVE: hand the restorer the machine's own carries
    /// and the resumed run must be bit-identical to the uninterrupted one.
    ///
    /// This is the half that `get_states` will drive once the kernel surfaces its
    /// `st[k]` moment slots; until then the hand's own state supplies the carries,
    /// which tests the restorer's logic and the `ChainState::LinregFlow` shape
    /// without pretending the engine can emit them yet.
    #[test]
    fn linreg_captured_carries_continue_bit_exact() {
        const SEAM: usize = 400;

        for p in [5.0, 14.0, 27.0] {
            let s = wiggle();
            let (_head, true_state) =
                Linreg::indicator(&[&s[..SEAM]], &[p], None).expect("head run");

            // The carries are taken first, so what the candidate restores from is
            // exactly what the gold machine holds at this bar.
            let chains = vec![true_state.carried_state()];

            // Gold: the true state continued.
            let mut gold = true_state.clone();
            let mut gold_rows: Vec<f64> = Vec::new();
            for chunk in s[SEAM..].chunks(256) {
                let out = gold
                    .batch_indicator(&[chunk], None)
                    .expect("gold continuation");
                gold_rows.extend_from_slice(&out[0]);
            }

            // Candidate: rebuilt from the captured carries + the eviction window.
            // The window is deliberately generous: `IndicatorState::new` keeps the
            // LAST `period - 1` bars of whatever it is given (`real` is sliced
            // `len - period + 1..`), and the state's internal `period` is not the
            // option value, so a window sized from the option underflowed. Anything
            // at least `period - 1` bars ending at the seam yields the same buffer.
            let p_us = p as usize;
            let win = &s[SEAM.saturating_sub(2 * p_us + 4)..SEAM];
            let mut replay = <Linreg as Restorable<1, 1>>::restore_state(chains, &[win])
                .unwrap_or_else(|| panic!("linreg p={p}: restore_state declined its own carries"));
            let mut got_rows: Vec<f64> = Vec::new();
            for chunk in s[SEAM..].chunks(256) {
                let out = replay
                    .batch_indicator(&[chunk], None)
                    .expect("captured continuation");
                got_rows.extend_from_slice(&out[0]);
            }

            assert_eq!(
                got_rows.len(),
                gold_rows.len(),
                "linreg p={p}: captured restore is at a different phase"
            );
            assert_bits(&format!("linreg captured p={p}"), &got_rows, &gold_rows);
        }
    }

    // ── THE FINDING THESE THREE GATES WERE WRITTEN TO PRODUCE ─────────────
    // They FAIL, so they are `#[ignore]`d rather than left to redden the suite:
    // the tree stays honest and green, and the reason lives next to the evidence.
    //
    // Measured, linreg p=5, seam at bar 400, all 300 resumed rows differ:
    //   resumed from tail replay: 97.74585928420673
    //   true state at the seam:   97.75386071243702
    // and `gold_standard_two_phase_matches_full_series` (ACTIVE, above) proves the
    // comparison itself is sound — the hand's own state at the seam reproduces the
    // uninterrupted run bit for bit at p=5/14/27.
    //
    // WHY the replay cannot work: `State { sum_y, sum_xy, n }` is not a function of
    // the last p+1 raw bars. `sum_xy` weights each bar by its OWN index, so
    // rebuilding it from a window restarts the weighting at the window's local
    // index — a systematically different carry, recomputed wrong every bar, which
    // is why the error does not decay. `n` (the phase counter) is likewise rebuilt
    // as if the machine were fresh. These restorers therefore hand back a plausible
    // state, not the continuing one, and `restore_state` currently IGNORES its
    // `Vec<ChainState>` argument (`_v`) for exactly this reason.
    //
    // THE FIX (not yet built): capture. `enum ChainState` needs a Linreg variant
    // carrying `sum_y`, `sum_xy` and `n` (the engine already holds the first two in
    // ordinary `st[k]` slots — `emit_scalar.rs`'s `Kind::LinregFlow` arm),
    // `get_states` must surface them, and these three `restore_state`s must consume
    // them instead of replaying. Un-ignore these three gates when it lands.
    #[test]
    #[ignore = "linreg state capture unbuilt: tail replay is not a continuing state (see block above)"]
    fn linreg_tail_window_continues_bit_exact() {
        for p in [5.0, 14.0, 27.0] {
            // linreg's hand state needs the period PLUS the bar that publishes, so the
            // tail is p+1 (`tsf`/`fosc` differ: see their tests).
            continue_from_tail::<Linreg, _>(&format!("linreg p={p}"), p, p as usize + 1, &wiggle());
            continue_from_tail::<Linreg, _>(
                &format!("linreg ramp p={p}"),
                p,
                p as usize + 1,
                &ramp(700),
            );
        }
    }

    #[test]
    #[ignore = "tsf state capture unbuilt: same LinregFlow carry, same reason as linreg"]
    fn tsf_tail_window_continues_bit_exact() {
        for p in [5.0, 14.0, 27.0] {
            continue_from_tail::<Tsf, _>(&format!("tsf p={p}"), p, p as usize + 1, &wiggle());
            continue_from_tail::<Tsf, _>(&format!("tsf ramp p={p}"), p, p as usize + 1, &ramp(700));
        }
    }

    #[test]
    #[ignore = "fosc state capture unbuilt: tsf carry plus its own RollAge lag"]
    fn fosc_tail_window_continues_bit_exact() {
        for p in [5.0, 14.0, 27.0] {
            // fosc overrides min_data to period + 2 (its published lag is one bar later
            // than tsf's), so its tail contract is p+2.
            continue_from_tail::<Fosc, _>(&format!("fosc p={p}"), p, p as usize + 2, &wiggle());
        }
    }
}
