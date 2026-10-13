//! The generated kernel registry — the single `kernel!` call site.
//!
//! Expansion (compile time, per pattern): `pub const SIG_<name>` erased
//! shapes, straight-line scalar kernels, `_assets<const L>` lane kernels for
//! the non-packed patterns, and the `try_run`/`try_run_assets` dispatchers.
//! A miss is `None` — tier-2/3 territory by design, never a runtime compile.

tulip_rs_macros::kernel! {
    // The classic sharing graph: apo+ppo+macd on one (s,l) pair, signal g.
    apo_ppo_macd_shared_sl: Apo(s, l), Ppo(s, l), Macd(s, l, g);
    //apo_ppo_macd_shared_sl_p: packed Apo(s, l), Ppo(s, l), Macd(s, l, g);
    // apo + ppo over the same pair.
    apo_ppo_shared_sl: Apo(s, l), Ppo(s, l);
    //apo_ppo_shared_sl_p: packed Apo(s, l), Ppo(s, l);
    // ATR family: atr = wilders(tr(p)), natr = (atr/close)*100 sharing tr+wilders
    atr_natr_shared_p: Atr(p), Natr(p);
    //natr_supertrend_shared_p: packed Atr(p), Natr(p);
    // NATR + SuperTrend: BOTH share the mean `Wilders(tr)` ATR node → dedup to one chain
    natr_supertrend_shared_p: Natr(p), Supertrend(p, 3.0);
    //natr_supertrend_shared_p_p: packed Natr(p), Supertrend(p, 3.0);

    // The 6-distinct-EMA shape: Macd(12,24,9), Apo(10,20), Ppo(6,14) erases to
    // SIX price-sourced Ema chains (no dedup) + 1 chained signal Ema + glue.
    ema_six: Macd(s1, l1, g), Apo(s2, l2), Ppo(s3, l3);
    ema_six_p: packed Macd(s1, l1, g), Apo(s2, l2), Ppo(s3, l3);

    // Rolling family, price-sourced: the stddev's optional sma mirror
    // interns onto the Sma node (one recurrence, two rows per request AND
    // across the two requests at a shared period).
    sma_stddev_shared_p: Sma(p), Stddev(p);
    // The two moving averages co-requested on one column: the standalone
    // Sma row interns onto the very Sma node wma mirrors (one arena node,
    // two sinks) — the hand pair pays two full passes over the series.
    sma_wma_shared_p: Sma(p), Wma(p);
    // Bollinger bands PAIRED with WMA on one period (the engine is a MERGE
    // engine — single-indicator shapes are the hand's home turf): one
    // `Sma(p)` node serves the bbands middle, the wma mirror, AND wma's
    // interned optional — three sinks, one recurrence, one pass.
    bbands_wma_shared_p: Bbands(p, m), Wma(p);

    // The 6-distinct-SMA shape (six periods, one column) — the rolling
    // analogue of `ema_six`: no dedup between the chains, and the packed
    // twin runs them as `Simd<f64,4>` + `Simd<f64,2>` lane groups.
    sma_six: Sma(p1), Sma(p2), Sma(p3), Sma(p4), Sma(p5), Sma(p6);
    sma_six_p: packed Sma(p1), Sma(p2), Sma(p3), Sma(p4), Sma(p5), Sma(p6);

    // Six distinct EMA CHAINS on one close series — the PURE multi-options
    // shape (rival: `Ema::indicator_by_options::<6>`); the packed twin
    // steps six recurrences as `Simd<f64,4>` + `Simd<f64,2>`.
    ema_chains: Ema(p1), Ema(p2), Ema(p3), Ema(p4), Ema(p5), Ema(p6);
    ema_chains_p: packed Ema(p1), Ema(p2), Ema(p3), Ema(p4), Ema(p5), Ema(p6);

    // Three ATRs at DIFFERENT periods on one (h,l,c): the tr subgraph
    // interns to ONE node, the three mean-`Wilders(tr)` chains are lanes of
    // one formula over a shared DERIVED source — the packing candidate the
    // ROADMAP named (rival: `Atr::indicator_by_options::<3>`).
    atr_multi: Atr(p1), Atr(p2), Atr(p3);
    atr_multi_p: packed Atr(p1), Atr(p2), Atr(p3);

    // ── trail-rescan hub ─────────────────────────────────────────────────
    // Max+Min on one period (over the SAME column they are the two faces of
    // one range; the anti-correlation rule only says their RESCANS don't
    // fuse into one walk — the arena still shares every upstream node).
    max_min_shared_p: Max(p), Min(p);
    // The aroon showcase: aroonosc costs ONE extra fsub + store on the
    // interned graph (the hand runs a second full pass over both columns).
    aroon_osc_shared_p: Aroon(p), AroonOsc(p);
    // willr + donchian: the same canonical (p, p−1) roll pair, four output
    // rows off two rolls, one Add/Abs/Div glue set.
    willr_donchian_shared_p: Willr(p), DonchianChannel(p);
    // The densest hub: six rolls (two columns × three windows), four
    // combines, one pass.
    ichimoku_s_m: Ichimoku(s, m);
}
