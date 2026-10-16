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
    // THE CROSS-CONVENTION CO-REQUEST — the shape this registry used to refuse.
    // It used to be authored as ONE hand-owning recipe (`AdoscApoPpo`, which
    // inlined the apo/ppo chains over `Input{2}`) because stream slots were
    // purely POSITIONAL: price recipes bind `Input{0}=close` while OHLC recipes
    // bind `Input{0}=high`, so three co-requested recipes would have interned
    // apo/ppo's close onto adosc's HIGH column — a silently wrong arena that
    // still passed every `SIG_*` equality test, because the drift was in the
    // recipe, not the erase.
    // That hazard is gone by CONSTRUCTION, not by care: all three recipes now
    // declare columns (`adosc` = hlcvol, `apo`/`ppo` = price), so `merge` takes
    // the `slot_maps` path and remaps each recipe's slots into the sorted column
    // union {High:0, Low:1, Close:2, Volume:3}. apo's `Input{0}` therefore lands
    // on arena stream 2 = Close — the exact binding the composing recipe spelled
    // by hand, now derived. `adosc_apo_ppo_sig_derived` pins it: 6 `Ema`s, 4
    // `Input`s, ONE `Accum`, and all four price EMAs reading the stream-2 node.
    // The hand rival is three separate passes (adosc on OHLCV, apo and ppo on
    // close), with periods all DISTINCT (3/10, 12/26, 6/14) so nothing dedups by
    // accident.
    adosc_apo_ppo: Adosc(s, l), Apo(a1, b1), Ppo(a2, b2);
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
    // interned with the ATR family's own chain and with `di`'s denominator
    // whenever either is co-requested.
    // This is the widest sharing shape in the registry. It carries both Wilders
    // FORMS in one arena — `dm`'s two `+DM`/`−DM` accumulators are COMPOSITE
    // (`composite: true`, the hand's `partial_calc`) while `di`'s denominator and
    // `adx`'s smoothing are MEAN — distinguished by `Signature::b` and published
    // by `erase` as the source scale (`invms` = 1.0 vs 1/period), not as a second
    // emitter form. The two forms never share a SOURCE here: `di` moved onto the
    // shared mean ATR chain (2026-10-16) precisely because a mean slot and a
    // composite slot over one interned `tr` makes the emitter publish the raw SUM
    // into the mean slot's row — see `di`'s doc and
    // `hub_packed_twin_builds_no_lane_group`.
    // All four declare columns (`hl`/`hlc`), so the arena is interned by
    // MEANING: High=0, Low=1, Close=2 whatever the request order.
    dm_di_dx_adx_shared_p: Dm(p), Di(p), Dx(p), Adx(p);
    // THE PACKED TWIN — kept as the guard that says whether the mixed-form shape
    // has come back, NOT as a capability. `classify_packed` buckets by
    // `(kind, source)`, and since `di` moved onto the mean chain every `Wilders`
    // in this arena has its own source (`+DM`/`−DM` over their own mutual-
    // exclusion nodes, the mean ATR over `tr`, the ADX smoothing over `dx`), so
    // this twin builds ZERO lane groups: it is an exact duplicate of the scalar
    // kernel and the hub's measured win is pure CSE dedup, not packing.
    // `hub_packed_twin_builds_no_lane_group` asserts that. If it ever fails
    // because a group appeared, a same-source mean/composite pair is back and the
    // published row must be checked against the hand before the group is believed.
    dm_di_dx_adx_shared_p_p: packed Dm(p), Di(p), Dx(p), Adx(p);
    // THE FULL DASHBOARD: the directional hub PLUS the volatility family, seven
    // indicators on ONE arena. This is the shape the user predicted when he said
    // "if so you should also be able to combine adx with natr and supertrend" —
    // and it is only possible because `di` moved onto the SHARED mean ATR chain
    // (d230b54) and `atr`/`natr`/`supertrend` declared their columns (2fd22d8), so
    // the whole request interns by MEANING instead of falling back to positional
    // binding.
    //
    // WHAT IS ACTUALLY SHARED, per node (not per indicator):
    //   · ONE `+DM`/`−DM` mutual-exclusion graph + its two COMPOSITE `Wilders`
    //     accumulators → `dm`, `di`, `dx`, `adx` (four consumers).
    //   · ONE `tr` graph and ONE mean `Wilders(tr)` ATR chain → `di`'s
    //     denominator, `adx`'s composed mirror, `atr`, `natr`, `supertrend`
    //     (FIVE consumers). `atr`/`natr`/`supertrend` add NO new recurrence:
    //     `adx` already grafts the identical chain.
    //   · `supertrend`'s `medprice` and `natr`'s `Div`/`Mul` are its own glue.
    // The hand rival is SEVEN separate passes, each recomputing TR and the Wilders
    // mean it needs — which is why this fusion is genuine and not the
    // own-optional-output artifact: every line here is a DISTINCT request, and the
    // optional mirrors are disabled (primary-only), so nothing is being paid for
    // twice on one side of the comparison.
    dm_di_dx_adx_atr_natr_supertrend_shared_p: Dm(p), Di(p), Dx(p), Adx(p), Atr(p), Natr(p),
        Supertrend(p, 3.0);
    // ── THE LONE-SHAPE FLOOR (diagnostics, not shipping options) ───────────
    // The four hub members registered INDIVIDUALLY, to answer a question the
    // merged rows cannot: how far behind the hand does an un-merged shape of this
    // family sit? User 2026-10-16 relaxed the old absolute prohibition on lone
    // patterns ("we can do this for testing, just don't expect it to win, but it
    // shouldn't be too far behind"), so these rows are the SCALAR-AXIS
    // DENOMINATOR — the hub's 4.6x is only meaningful next to them. Each pays
    // what the merge removes: `dm` alone still needs its two composite chains,
    // `di` and `adx` each rebuild the whole `+DM`/`−DM` → `dx` subgraph, and
    // `adx` additionally composes the mean ATR chain. Hand rival: the same single
    // `indicator()` call, so this is the hand's home turf by construction.
    dm_n: Dm(p);
    di_n: Di(p);
    dx_n: Dx(p);
    adx_n: Adx(p);
    // THE ATR RUNG — added specifically to make `di_n`'s cost DECOMPOSABLE.
    // Measured floor (one isolated run): dm 6.09, dx 9.65, adx 11.63, DI 17.20
    // µs, where the hand runs DI in 8.48 — DI is the one shape 2x BEHIND, and it
    // also costs MORE than `adx`, whose arena contains dm + dx + atr PLUS its own
    // smoothing. That inversion cannot be explained from the four rows alone,
    // because DI = (the whole `dm` chain) + (the mean `Wilders(tr)` ATR chain) +
    // two `Mul`/`Max` glue nodes, and until now the mean ATR chain had never been
    // timed by itself anywhere in the registry. With this rung, `di_n − dm_n −
    // atr_n` is the glue's measured cost; without it any claim about WHY DI is
    // slow is a guess. `atr_natr_shared_p` pairs ATR with NATR, so it is not this
    // rung and does not answer the question.
    atr_n: Atr(p);
}
