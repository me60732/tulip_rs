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
    // THE PACKED TWIN OF THE CROSS-CONVENTION ARENA — same-source packing, and it
    // WON. `classify_packed` buckets on `(Kind, source slot)`, so this arena's SIX
    // `Ema` chains form exactly TWO groups: `(Ema, close)` = [15,16,18,19] → one
    // 4-wide, and `(Ema, AD line)` = [12,13] → one 2-wide. Every lane of a group
    // reads ONE broadcast source (`Simd::splat`), which is the only packing form
    // this engine ships.
    //
    // MEASURED twice, one binary each, `runonly`, 6705 bars, medians:
    //   run 1  scalar 12.515 µs → packed 12.031 µs (+4.0%), hand 17.712 µs
    //   run 2  scalar 12.582 µs → packed **11.873 µs** (+6.0%), hand 17.768 µs
    // The win reproduced, and the twin is 1.50× over the hand. Budget per bar:
    // scalar is 6 chains × (mul + fma) = 12 ops; packed is 2 groups × (mul + fma) = 4
    // vector ops, every lane reading ONE broadcast source.
    //
    // CROSS-SOURCE PACKING WAS BUILT AND REJECTED: a third arm that merged the two
    // buckets into one 4-wide group reading a PER-LANE source vector was emitted,
    // gated byte-exact, and measured 12.798 µs — +6.4% over this twin AND +2.3%
    // over not packing at all. Cause: halving the group count still costs the
    // per-bar assembly of `<4 x f64>` from two different locals, which exceeds the
    // halved step; width was not the limit (rustc/LLVM on AVX2, real 256-bit). The
    // standing rule from that measurement: prioritize same-source grouping, because
    // a splat is faster than reading from different sources. See
    // `ad_line_and_price_emas_do_not_share_a_lane_group` in the macro crate.
    adosc_apo_ppo_p: packed Adosc(s, l), Apo(a1, b1), Ppo(a2, b2);
    // MOM: the pattern that makes `Values::lag_n` observable IN A ROW — the
    // hand pairs bar i with bar i−p, so a steady-loop depth other than `p` is a
    // wrong value, not a state-only artefact. `cmo` carries the same
    // `Lag{…, period}` but feeds it only to a `Mirror`, which is why the depth
    // bug stayed masked.
    mom_n: Mom(p);
    // MASS + THE HUB, four REAL co-requested recipes on one arena. This used to
    // be a hand-composed recipe (`MassApoPpoMacd`, now deleted) because mass is
    // HL-shaped while apo/ppo/macd are price-shaped, and positional slots made
    // that combination unsafe: the hub's `Input{0}=close` would have interned
    // onto mass's HIGH column. `mass` now declares `hl()` and apo/ppo/macd
    // declare `price()`, so `merge` ranks the union {High:0, Low:1, Close:2} and
    // remaps each recipe into it — the hub reads stream 2 by DERIVATION.
    // The hub's three requests still share ONE `ema_s`/`ema_l`/`line` triple, so
    // four indicators cost three EMA chains + one ring group; mass's own chain is
    // independent and is merged for the PASS, not for the math.
    mass_apo_ppo_macd: Mass(p), Apo(s, l), Ppo(s, l), Macd(s, l, g);
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
    // the whole request interns by MEANING; an undeclared recipe would now be
    // rejected by `merge` outright rather than silently binding positionally.
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
    // ── THE CALCULATED-INPUT AND RATE-OF-CHANGE HUBS ─────────────────────
    // MOM + ROC + ROCR on ONE period. The hand runs three passes and recomputes
    // `real[i-p]` in every one. Here the arena holds ONE `Input`, ONE `Lag{close,p}`,
    // the ONE `Sub{close,prev}` that IS `mom`, and ONE `Max{prev,eps}` floor shared
    // by roc's and rocr's denominators — so three published indicators cost two new
    // `Div`s and nothing else. The identity that makes it intern: `roc`'s optional row
    // (hand `Info.optional_outputs: ["mom"]`) IS the standalone `Mom` request's node.
    mom_roc_rocr_shared_p: Mom(p), Roc(p), Rocr(p);
    // MEDPRICE + TYPPRICE + WCPPRICE: the three calculated-input composites all build
    // the SAME `Add{high, low}` — medprice scales it by 0.5, wcprice feeds it to the
    // `Fma` as the `c` operand, typprice adds close and scales by `1.0/3.0`. One add,
    // three outputs. This is the `E8` calculated-inputs family, where co-requesting is
    // the cheapest way to stop paying for the same composite three times.
    medprice_typprice_wcprice: Medprice(), Typprice(), WcPrice();
    // THE LONE FLOORS for the same family — DIAGNOSTICS, not shipping options (the
    // engine's unit is the merge, so these are what the two hubs above are measured
    // AGAINST). `bop_n` doubles as something new: it is the registry's first pattern
    // whose arena contains an OPEN column, so it is the live proof that `ohlc()`'s
    // canonical ranking binds open=0/high=1/low=2/close=3 instead of silently reading
    // a neighbouring column — the exact hazard the column-declared remap closed.
    // THAT PROOF IS NOW LIVE, and was not before: `Bop` appeared in no engine gate
    // anywhere (every `bop` hit was in `bop_test.rs`, hand-vs-tulip-C), so the row was
    // registered, reachable, and unverified. `bop_lone_floor_bit_exact` closes it with
    // four DISTINCT columns per slot, so a ranking slip moves every bar, plus a
    // flat-bar block that forces `high == low` into the `Max(_, EPSILON)` floor.
    bop_n: Bop();
    roc_n: Roc(p);
    rocr_n: Rocr(p);
    typprice_n: Typprice();
    wcprice_n: WcPrice();
    // ── THE SECOND CALCULATED-INPUT / OSCILLATOR TRANCHE ─────────────────
    // Sma(p) + Dpo(p) on ONE period is the reason dpo's roll is spelled
    // `Node::Sma` and not `RollSum{mean:true}`: the two are DIFFERENT signature
    // kinds (K_SMA=16 vs K_ROLLSUM=24), so as a ring roll the arena would carry the
    // same rolling mean of `real` TWICE and the hub would save nothing. As written,
    // dpo's roll and the standalone Sma request are one interned node -- which
    // `sma_dpo_shared_p_interns_one_sma_and_is_bit_exact` measures rather than
    // asserts in prose.
    sma_dpo_shared_p: Sma(p), Dpo(p);
    // Lone floors for the same tranche (diagnostics, not shipping options): the
    // arena-declared arities are the hand's own -- AvgPrice reads open/high/low/
    // close, QStick open/close (`oc()`), Vwma close/volume (`cvol()`), Vosc volume
    // only (`vol()`), so each row also pins that its column set is exactly what the
    // hand's Info declares.
    avgprice_n: AvgPrice();
    qstick_n: QStick(p);
    dpo_n: Dpo(p);
    vwma_n: Vwma(p);
    vosc_s_l: Vosc(s, l);
    // CHAIKIN MONEY FLOW, registered as a HUB with `Ad` rather than as another lone
    // floor, because the two share their column set AND their arithmetic: both are
    // `hlcvol()`, and `chaikinmf.rs::calc_mfv` opens with the same `(high - low)` the AD
    // chain builds, plus the same `f64::EPSILON` constant. So the pair's arena should
    // carry the range `Sub` ONCE. What is deliberately NOT shared is the money-flow
    // numerator: the hand writes `(close - low) - (high - close)` while `ad_nodes` writes
    // `((close - low) - high) + close`. Different association, therefore not bit-equal in
    // f64, so interning them would be a fabrication -- the gate asserts the split, not a
    // tidier-looking count.
    // NOTE the hand's final divide is UNGUARDED (`sums[0] / sums[1]`), unlike vwma/vosc:
    // an all-zero-volume window must print the hand's NaN, not a invented 0.0.
    ad_chaikinmf_shared: Ad(), ChaikinMf(p);
    chaikinmf_n: ChaikinMf(p);
    // VHF IS THE FIRST ROW ADDED PURELY TO ROUTE AN EXISTING RECIPE. `recipes::vhf` was
    // complete and `impl Planable for Vhf` existed (`vhf.rs:240`), but with no planner arm
    // and no row the graph could never reach a kernel -- a `Planable` impl that silently
    // promises more than it delivers.
    //
    // `max_min_vhf_shared_p` is the reason to register it as a hub rather than a floor:
    // `recipes::max` and `recipes::min` each build `RollMax{src: real, period,
    // look_back: period-1}` / `RollMin{..}` over `price()`, and `vhf` builds the two
    // IDENTICAL nodes itself. `intern` keys on the whole `Node`, so three consumers share
    // TWO ring rescans instead of paying four -- the merge win, measured by the arena-count
    // gate rather than asserted here.
    vhf_n: Vhf(p);
    max_min_vhf_shared_p: Max(p), Min(p), Vhf(p);
    // VWAP IS THE CALCULATED-INPUTS HUB THAT ARRIVES WITHOUT ANY NEW MACHINERY. The hand
    // `vwap.rs:44` computes `tp` by CALLING `typprice::calc`, and `recipes::vwap` spells
    // that subgraph node-for-node the way `recipes::typprice` does, so `intern` (keyed on
    // the whole `Node`) merges them: vwap's OPTIONAL `typprice` row and Typprice's PRIMARY
    // row become the same arena node. If the subgraph failed to intern, `K_ADD` would
    // count 4 instead of 2 -- the gate asserts the counts, so a silent un-shared graph
    // cannot pass as a hub. This is the `E8` payoff measured, not planned.
    vwap_n: Vwap();
    vwap_typprice_shared: Vwap(), Typprice();
    // SMA ENVELOPE is a calculated-composite hub in the same sense as
    // `vwap_typpice_shared`: its middle band is the same `Node::Sma{src: Input{0}, period}`
    // that `recipes::sma` builds, so `Sma(p) + SmaEnvelope(p, m)` interns to ONE roll. The
    // distinguishing count is K_SMA -- 1 shared versus 2 duplicated (arena length goes 6
    // vs 7, but `K_SMA` is the claim). It publishes THREE primaries (`Info.outputs =
    // ["lower","middle","upper"]`, no optionals), so the hub's sink table is 4 rows from 2
    // consumers, and the gate asserts that count as well as the bit-exactness.
    smaenvelope_n: SmaEnvelope(p, m);
    sma_smaenvelope_shared_p: Sma(p), SmaEnvelope(p, m);
    // OBV IS THE FIRST HUB THAT PROVES THE COLUMN LAW IN THE POSITIVE DIRECTION. `tr`
    // reads close as stream 2 (`hlc()`), `obv` reads close as stream 0 (`cvol()`) -- two
    // recipes that DISAGREE about the index and agree about the column. The merge remaps by
    // column before interning, so `Tr() + Obv()` must carry ONE `Lag{close, 1}`: K_LAG 1
    // (17 nodes) if the remap works, 2 (18) if it does not. Every previous test pinned only
    // the rejection half (the merge panicking when a recipe declared no columns); this is
    // the acceptance half, and it is the half that has to keep working for pairings like
    // `adosc + apo` to be legal at all.
    obv_n: Obv();
    tr_obv_shared: Tr(), Obv();
    // THE CHAINED-EMA TRANCHE. NOT the registry's first chained EMA -- `mass`
    // already chains twice (`Ema{src: hl}` then `Ema{src: ema}`) and `macd`'s signal
    // chains once off a `Sub`. What is new here is DEPTH: `tema` is the first
    // TRIPLY-chained arena (ema1 -> ema2 -> ema3, `Ema{src: ema2}`), which puts three
    // dependent recurrences through the warm-up `[0, gate)` prologue back to back,
    // each consuming the previous chain's OUTPUT as its input.
    // dema = ema1*2 - ema2 (ONE `Fma`, matching the hand's single `mul_add`),
    // tema = ema1*3 + (ema2*-3 + ema3) (TWO nested `Fma`s).
    //
    // ZLEMA IS NOT REGISTERED, AND IT IS NOT A LEAD PROBLEM -- MEASURED. Two
    // separate discrepancies, one cause:
    //  · LENGTH: the arena computes lead 12 for p=9 (its `adjusted` source is
    //    defined-from at lag=4, and `Ema(9)` over it composes to 4 + 8 = 12), where
    //    the hand publishes from bar `lag` = 4. That is 8 rows, explainable.
    //  · VALUES: aligning the two rows bar-for-bar still differs from the FIRST
    //    aligned bar -- 137 mismatched bars of 688 on `wiggle()`, 132 of 668 on a
    //    ramp, and ZERO on a flat 100.0 series. Length alone cannot do that.
    // The cause is the START STATE, not the lead: `zlema.rs:42` seeds the EMA with
    // `real[lag - 1]`, while the engine warms the same recurrence through
    // `[0, gate)`. An EMA carries its seed forever -- the error never decays to zero
    // in f64 -- so a differing start means every later bar differs, and a flat series
    // hides it precisely because there the seed equals the warmed value.
    // So a gate on the aligned overlap would NOT be a fair test: it would compare an
    // engine with the right recurrence and the wrong initial state against a hand
    // with the right initial state, and call the residue "close enough". Registering
    // it needs a seeded-start mechanism (an Ema whose carry is initialized from a
    // node), which is a vocabulary decision, not a recipe fix.
    dema_n: Dema(p);
    tema_n: Tema(p);
    // THE ATR ABLATION RUNGS -- see the `atr_n` note above for why they exist.
    // `tr_n` is the pure 3-term Max/Abs subgraph over (high, low, close) with a
    // Lag{close,1}, NO recurrence; `wilders_n` is the mean-Wilders recurrence alone
    // over close, NO tr subgraph. `atr_n` is both. So
    //     atr_n − tr_n − wilders_n
    // is the measured cost of fusing them (register pressure, extra live carries),
    // and each rung's own engine/hand ratio says WHICH half of atr's 1.60x deficit
    // belongs to the tr walk and which to the smoothing. Gated in
    // `atr_ablation_rungs_bit_exact` before any number was read off them.
    tr_n: Tr();
    wilders_n: Wilders(p);
    // tema OWNS a DemaState, so its ema1/ema2 AND its `dema` output node are the
    // very nodes the standalone dema request builds -- co-requesting them should
    // intern the whole two-chain subgraph and one dema Fma, with tema's `dema`
    // optional row publishing the same stored stream as dema's primary.
    dema_tema_shared_p: Dema(p), Tema(p);
}
