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

    // ── ring-backed derived rolls ───────────────────────────────────────
    // AO: two mean RollSums over the ONE interned medprice column — the
    // first shape whose ring is a derived-source `Buffer<Cold>` (the hand's
    // own type), capacity = group max (34), the 5-window leaving bar read
    // by age. Scalar kernel only until the lane ring arm lands (ROADMAP
    // step C); the lane dispatch correctly misses it meanwhile.
    ao_s_m: Ao(s, m);
    // Hull MA: the bespoke machine (E3 stage 3) — two inner WMA engines
    // and the √p diff ring inside ONE node. Scalar-only until step C,
    // like the ring rolls.
    hma_n: Hma(n);
    // Fisher Transform: the IIR/clamp/ln/prev chain over an EXTREMUM ring
    // on the derived medprice column (the fourth ring shape). Scalar-only
    // until the vec-ring wave gives it lanes + the packed twin (ROADMAP
    // "M3-rings step C+"). Parity law: 1e-12 RELATIVE vs the hand.
    fisher_n: Fisher(p);
    // THE sharing showcase the old JIT failed at: AO and Fisher over ONE
    // interned medprice - co-requesting adds zero arithmetic nodes at the
    // merge (canary in cse_merge_test); one med compute and one ring feed
    // serve all four rolls (2 sums + 2 extrema). This is the shape that
    // must beat the JIT's ~3.5x medprice loss vs the hand.
    ao_fisher_shared_med: Ao(s, m), Fisher(p);
    // Stochastic: the shapes that unlocked RollSum skip/steady_from — slow
    // %K is a mean with skip:2 over the division column, %D a mean over the
    // slow line (two ring groups), both riding the canonical (k, k−1)
    // price-roll trail pair.
    stoch_kd: Stoch(k, s, d);
    // The flagship triple: stoch's fast-%K rolls ARE willr's and donchian's
    // (p, p−1) pair — two interned rolls feed all six output rows.
    stoch_willr_donchian_shared_p: Stoch(p, s, d), Willr(p), DonchianChannel(p);
    // RSI: two mean-Wilders chains over the Max-gated movement arms plus the
    // 100·up/(up+down) glue; carries the engine's first lowered `Mirror`
    // node (the hand's `prev_real` state copy — identity, computed, never
    // stored). Scalar pair with the hand's `Rsi::indicator`; lanes ride the
    // Wilders arm proven on the atr family.
    rsi_n: Rsi(p);
    // THE sharing flagship: stochrsi GRAFTS the whole rsi chain, so a
    // co-requested `Rsi(p) + Stochrsi(p)` interns the movement gates, both
    // Wilders chains, the division and the ×100 — stochrsi's optional RSI
    // row and standalone rsi's primary ARE the same stored stream (one
    // arena, three sinks, two extra ring-group members on the derived rsi
    // column's extrema pair).
    rsi_stochrsi_shared_p: Rsi(p), Stochrsi(p);
    // CMO: two raw ring RollSums over the derived gain/loss columns (two
    // ring groups), the movement chain RSI already computes, and two state
    // Mirrors (`prev` = the close, `drop_real` = the close `p` bars back).
    cmo_n: Cmo(p);
    // The movement-sharing pair: `up_down(v, prev)` is the SAME node set in
    // both recipes — Lag, diff, both Max gates intern once; RSI Wilder-
    // smooths them, CMO rolls them up. Two rings, two sinks, one pass.
    rsi_cmo_shared_p: Rsi(p), Cmo(p);
    // ULTOSC is the B-axis container test the rest of the registry cannot
    // make: `bp` and `tr` are TWO independent pure-sum groups of THREE members
    // each (short, medium, long), so the planner's `bundles()` names ONE fused
    // `MultiBuffer<2>` for all six folds — the container the hand itself chose
    // (`ultosc.rs` imports `MultiBuffer as Buffer` and pushes `[bp, tr]` in one
    // call). The per-group fold cannot reach this: it leaves two buffer calls
    // per bar where one exists. Three sinks (`ultosc` lead = long, `tr`/`bp`
    // lead 1) publish the hand's own primary rows.
    ultosc_s_m_l: Ultosc(s, m, l);
    // MASS: one ring group over a TWO-stage EMA chain (`Max(Sub(hi,lo),eps)` →
    // `ema9` → `ema9` → divide), so it is the registry's chain→chain→Div→fold
    // topology test — the hand's magic `16`s are the chain leads, not a knob.
    mass_n: Mass(p);
    // The pair the user asked for, to ANSWER A STRUCTURAL QUESTION by node
    // count rather than by timing: ultosc's `tr` is `Max(hi,pc) − Min(lo,pc)`
    // while natr's is `Max(Max(hi−lo,|hi−pc|),|lo−pc|)` — equal in value,
    // different GRAPHS, so CSE (which keys on kind+edges) cannot intern them.
    // What DOES share is the `Lag{close,1}` anchor. Pinned by
    // `ultosc_natr_shared_pc_sig_derived`.
    ultosc_natr_shared_pc: Ultosc(s, m, l), Natr(p);
    // AD: the registry's first VOLUME-consuming pattern and its first
    // `Accum` — one running total over `clv·volume` with the hand's
    // early-return guard (`Node::WhereLe`) and a lead-0 sink. No options.
    ad: Ad();
    // ADOSC: the first indicator whose RECURRENCE reads a derived source
    // rather than a column — two `Ema`s chained onto the AD accumulator.
    // This is the shape the old JIT was tried with (`adosc + apo + ppo`) and
    // failed measurably on; here it costs no new machinery, because a
    // derived-source Ema is the same arm a price-source one uses.
    adosc_s_l: Adosc(s, l);
    // NOT registered: a cross-convention co-request. Stream slots are purely
    // POSITIONAL and nothing in `merge`/`erase` checks that two recipes agree on
    // what `Input{i}` means — price recipes (`apo`/`ppo`/`ema`/`sma`) bind
    // `Input{0}=close`, OHLC recipes (`ultosc`/`mass`/`atr`/`ad`/`adosc`) bind
    // `Input{0}=high`. Every pattern above pairs within ONE convention, so a
    // mismatch has never had a chance to fire. `Adosc + Apo + Ppo` as three
    // recipes would intern apo/ppo's "close" onto the adosc HIGH column: a
    // silently wrong arena that still passes every `SIG_*` equality test,
    // because the drift is in the recipe, not the erase.
    // The mixed-input shape the old JIT lost on is therefore authored as ONE
    // recipe that owns the arena (inline the apo/ppo chain over `Input{2}`),
    // exactly as `stochrsi` inlines the rsi chain instead of co-requesting it.
    adosc_apo_ppo: AdoscApoPpo(s, l, a1, b1, a2, b2);
    // MOM: the pattern that makes `Values::lag_n` observable IN A ROW — the
    // hand pairs bar i with bar i−p, so a steady-loop depth other than `p` is a
    // wrong value, not a state-only artefact. `cmo` carries the same
    // `Lag{…, period}` but feeds it only to a `Mirror`, which is why the depth
    // bug stayed masked.
    mom_n: Mom(p);
    // MASS + THE HUB, one arena: mass is an OHLC shape and apo/ppo/macd are
    // price shapes, so this is a COMPOSING recipe, never a co-request (slots are
    // positional — see the `Adosc + Apo + Ppo` note above). The hub's three
    // requests share ONE `ema_s`/`ema_l`/`line` triple, so four indicators cost
    // three EMA chains + one ring group; mass's own chain is independent and is
    // merged for the PASS, not for the math.
    mass_apo_ppo_macd: MassApoPpoMacd(p, s, l, g);
    // THE DIRECTIONAL HUB: `dm`, `di`, `dx` and `adx` all build (or graft) the
    // SAME `+DM`/`−DM` mutual-exclusion graph and its two COMPOSITE (raw-sum)
    // `Wilders` accumulators, so one request computes directional movement ONCE
    // and feeds four indicators — plus `adx`'s composed mean-`Wilders(tr)` ATR,
    // interned with the ATR family's own chain whenever that is co-requested.
    // This is the widest sharing shape in the registry and the first to carry
    // BOTH Wilders forms in one arena: `di`'s denominator is the raw SUM
    // (`composite: true`, the hand's `partial_calc`) while `adx`'s smoothing is
    // the MEAN, distinguished by `Signature::b` and published by `erase` as the
    // source scale (`invms` = 1.0 vs 1/period) — no second emitter form.
    // All four declare columns (`hl`/`hlc`), so the arena is interned by
    // MEANING: High=0, Low=1, Close=2 whatever the request order.
    dm_di_dx_adx_shared_p: Dm(p), Di(p), Dx(p), Adx(p);
}
