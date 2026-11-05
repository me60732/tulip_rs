use crate::cse::{Id, Node, Recipe, Sink};

/// `TRVI(p)` -- True Range Volatility Indicator, hand `trvi.rs`. CLOSED
/// 2026-10-26 via the new `Node::SeededEma { seed, src, period }` primitive.
/// The hand seeds its `ema` from a bar-0-only special case (`high[0] -
/// low[0]`, ignoring close), then folds the REAL `tr` formula (with
/// `Lag(close, 1)`, structural lead 1) from bar 1 onward -- a plain
/// `Ema{src: tr, ..}` would seed from `tr[1]` (a real, close-aware TR value),
/// not the hand's bar-0 range, so `SeededEma{seed: range, src: tr, period}`
/// is used instead: `feeds[k] = leads[seed] = 0`, publishes `range[0]` at the
/// seed bar, folds `tr[1], tr[2], ..` from the next bar on. `leads()` for
/// `SeededEma` is `leads[seed] + period - 1`, same shape as plain `Ema`
/// (confirmed against `cvi`'s identical `Ema`+`RollAge` chain: the ring
/// must start pushing at `period - 1`, not bar 0, for the eviction math to
/// line up with the hand's buffer-capacity semantics -- traced concretely,
/// not assumed). The `ema` optional output's REPORTED lead, however, is
/// `period + 1` (two bars later than the `period - 1` structural minimum):
/// `Ema::min_data` defaults to `period + 1` (trait default), so the hand's
/// `ema_line` capacity is `tr_capacity - period` where `tr_capacity =
/// len - 1`, i.e. offset `period + 1` against the full series. A
/// capacity-bookkeeping artifact of the hand, not a correctness
/// requirement (the value IS valid two bars earlier), but `Sink`'s
/// `put_sink` can only push an override LATER than structural, never
/// earlier, so `Sink::iface_lead(ema, period + 1)` is used to match row
/// lengths exactly. `tr`'s own optional output keeps its ordinary lead of
/// 1 (`tr_nodes()`'s `Lag`).
pub fn trvi(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hlc();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let close = r.add(Node::Input { stream: 2 });
    r.tag(close, "close");
    let range = r.add(Node::Sub { a: high, b: low });
    r.tag(range, "hl_range");
    let tr = super::tr_nodes(&mut r, high, low, close);
    let ema = r.add(Node::SeededEma {
        seed: range,
        src: tr,
        period,
        feed: None,
    });
    r.tag(ema, "ema");
    let old_ema = r.add(Node::RollAge {
        src: ema,
        n: period,
    });
    r.tag(old_ema, "old_ema");
    let diff = r.add(Node::Sub { a: ema, b: old_ema });
    let ratio = r.add(Node::Div {
        a: diff,
        b: old_ema,
    });
    let hundred = r.constant(100.0);
    let trvi = r.add(Node::Mul {
        a: ratio,
        b: hundred,
    });
    r.tag(trvi, "trvi");
    r.kind = "trvi";
    r.options = options.to_vec();
    r.tail_inputs = vec![old_ema, ema];
    r.outputs.push(Sink::iface_lead(trvi, 2 * period - 1));
    r.optional_outputs.push(Sink::iface_lead(tr, 1));
    // Structurally valid from `period - 1` (same as cvi's internal RollAge
    // math relies on), but Ema::min_data defaults to `period + 1` (the
    // trait's default `options[OP-1] + 1`), so the hand's own `ema_line`
    // capacity is `tr_capacity - (period + 1) + 1 = tr_capacity - period`,
    // i.e. offset `period + 1` against the full series -- TWO bars later
    // than the structural minimum. A capacity-bookkeeping artifact of the
    // hand (the value IS valid earlier), but `put_sink` can only push an
    // override LATER than structural, never earlier, so
    // `Sink::iface_lead(ema, period + 1)` is used to match row lengths
    // exactly.
    r.optional_outputs.push(Sink::iface_lead(ema, period + 1));
    r
}

/// ENGINE-CORRECTNESS PROBE for B4a -- NOT a real indicator, no hand oracle.
/// Exercises the one shape the checklist flagged as untested: a MEAN `Wilders`
/// and a COMPOSITE `Wilders` sharing the SAME source node (here, `tr`) in one
/// arena, so `classify_packed` (`tulip_rs_macros/src/cse/codegen.rs`) buckets
/// them into ONE lane group (`(Kind::Wilders, tr)`). `erase()` already
/// publishes the per-node magnitude correctly (`invms[k] = 1.0` for
/// composite, `1/period` for mean -- `signature.rs`'s `Node::Wilders` arm),
/// and the packed-group hoist already reads `vals.invms[k]/vals.mults[k]`
/// PER SLOT (`codegen.rs`'s `Simd::from_array([vals.mults[k0], vals.mults[k1]]`
/// in the group hoist, and `vals.invms[k0], vals.invms[k1]` likewise) -- so
/// this is a confirmation probe, not new code: B4a's own notes already traced
/// this as "value-correct by construction", reachable by no shipped recipe.
/// Two primary outputs, `mean` and `composite`, both fed from bar 0 (same
/// convention as `atr`'s own `w` node) so the bit-exact test can walk the
/// full series against an independently-written reference recurrence.
pub fn wilders_mixed_probe(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hlc();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let close = r.add(Node::Input { stream: 2 });
    r.tag(close, "close");
    let tr = super::tr_nodes(&mut r, high, low, close);
    // `feed: false` + `Sink::iface_lead(_, period)` — the proven convention
    // from `atr_nodes()` (mean) and `dm()`'s `dmup`/`dmdown` (composite), not
    // guessed: both already publish a Wilders(tr)-shaped node this exact way.
    let mean = r.add(Node::Wilders {
        src: tr,
        period,
        composite: false,
        feed: false,
    });
    r.tag(mean, "mean");
    let composite = r.add(Node::Wilders {
        src: tr,
        period,
        composite: true,
        feed: false,
    });
    r.tag(composite, "composite");
    r.kind = "wilders_mixed_probe";
    r.options = options.to_vec();
    r.tail_inputs = vec![mean, composite];
    r.outputs.push(Sink::iface_lead(mean, period));
    r.outputs.push(Sink::iface_lead(composite, period));
    r
}

/// `MASS(period)` — Mass Index: the raw window sum of `ema9(hl) / ema9(ema9(hl))`.
///
/// Mirrors `mass.rs`: `hl = Max(Sub(hi,lo), Const(EPSILON))` (`:112`), two
/// chained period-9 `Ema`s, the ratio (`:116`, unclamped in BOTH phases since
/// 2026-10-15 — the warm-up used to keep a `.max(0.0)` the steady loop had
/// commented out), then `sum += mass − evicted`, a raw `RollSum` of capacity
/// `period`.
///
/// The two magic `16`s of the hand fall out of the arena instead of being
/// spelled: `lead(Ema) = lead(src) + p − 1` gives `ema1` lead 8 and `ema2`
/// lead 16, and a derived roll feeds at `feeds[k] = lead(src)`
/// (`signature.rs:401`) — so the ring first feeds at bar 16, exactly the hand's
/// `if i >= 16` push gate, and the publish lead is `period + 15`
/// (`min_data − 1`, the row-0 value the fill loop writes at bar `period + 15`).
/// The `9`s are the hand's hardcoded EMA periods (it never reads `options` for
/// them); `options[0]` sizes only the ring.
pub fn mass(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hl();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let range = r.add(Node::Sub { a: hi, b: lo });
    r.tag(range, "hl_range");
    let eps = r.constant(f64::EPSILON);
    let hl = r.add(Node::Max { a: range, b: eps });
    r.tag(hl, "hl_diff");
    let ema = r.add(Node::Ema { src: hl, period: 9 });
    r.tag(ema, "ema");
    let ema_signal = r.add(Node::Ema {
        src: ema,
        period: 9,
    });
    r.tag(ema_signal, "ema_signal");
    let ratio = r.add(Node::Div {
        a: ema,
        b: ema_signal,
    });
    r.tag(ratio, "mass");
    let sum = r.add(Node::RollSum {
        src: ratio,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(sum, "mass_idx");

    r.kind = "mass";
    r.options = options.to_vec();
    // hand `State` field order: `ema_state` (the `ema` node's own `Ema`
    // chain), `ema_signal` (the `ema_signal` node's own `Ema` chain -- a bare
    // f64 is read off its `value`, not a whole `EmaState`), then `sum` (the
    // `RollSum` node's running total PLUS its physical ring -- see
    // `ChainState::RollSumRing`'s doc comment for why a ring export is
    // possible here, unlike the generic `Sma` shape). `buffer` itself is not
    // a separate tail_input: it's fully implied by `RollSumRing`'s own
    // `ring`/`cursor` pair.
    r.tail_inputs = vec![ema, ema_signal, sum];
    r.outputs.push(Sink::iface_lead(sum, period + 15));
    r
}

/// `TR` as a standalone request: the true-range graph over the three price
/// streams, publish lead 1 (`output_length = len − 1`). Stateless —
/// `prevClose` is the `Lag` anchor, not a chain. The same nodes the ATR
/// family's chain feeds on, so a dashboard sharing TR with atr/natr/supertrend
/// rolls it once.
pub fn tr(_options: &[f64]) -> Recipe {
    let mut r = Recipe::hlc();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let cl = r.add(Node::Input { stream: 2 });
    r.tag(cl, "close");
    let tr = super::tr_nodes(&mut r, hi, lo, cl);
    r.kind = "tr";
    r.options = Vec::new();
    r.outputs.push(Sink::iface_lead(tr, 1));
    // `tr::State.prev_close` is the last published `close` — a plain scalar
    // retention, so `Node::Mirror{src: cl}` captures it with no new
    // `ChainState` variant.
    let prev_close = r.add(Node::Mirror { src: cl });
    r.tag(prev_close, "prev_close");
    r.tail_inputs = vec![prev_close];
    r
}

/// `DM(p)` = Wilders **sum** smoothing (COMPOSITE `Wilders`) over the raw
/// directional movement, inputs (high, low). The raw movement is the mutual
/// exclusion `up = h − Lag(h,1)`, `down = Lag(l,1) − l`, then
/// `+DM = where(up >= down, max(up,0), 0)` and the mirror for `−DM` — a
/// `WhereGe` select plus `Max`, NOT a `×0.0` mask (a mask can yield `-0.0`/NaN
/// where the hand writes a literal `0.0`). The accumulators fold from bar 0
/// because the `Lag` clamp makes `up[0] = down[0] = 0`, which is exactly the
/// hand's `skip(1)` warmup adding nothing at bar 0. Publish lead = `period`.
///
/// `Di`/`Dx`/`Adx` reuse these exact nodes, so a directional dashboard computes
/// directional movement ONCE — the hub this family is registered for.
pub fn dm(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hl();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let ph = r.add(Node::Lag { src: high, n: 1 });
    r.tag(ph, "prev_high");
    let pl = r.add(Node::Lag { src: low, n: 1 });
    r.tag(pl, "prev_low");
    let up = r.add(Node::Sub { a: high, b: ph });
    r.tag(up, "up_move");
    let down = r.add(Node::Sub { a: pl, b: low });
    r.tag(down, "down_move");
    let zero = r.constant(0.0);
    let up_pos = r.add(Node::Max { a: up, b: zero });
    let down_pos = r.add(Node::Max { a: down, b: zero });
    let dp = r.add(Node::WhereGe {
        a: up,
        b: down,
        t: up_pos,
    });
    r.tag(dp, "dm_plus_raw");
    let dm = r.add(Node::WhereGe {
        a: down,
        b: up,
        t: down_pos,
    });
    r.tag(dm, "dm_minus_raw");
    let dmup = r.add(Node::Wilders {
        src: dp,
        period,
        composite: true,
        feed: false,
    });
    r.tag(dmup, "dm_plus");
    let dmdown = r.add(Node::Wilders {
        src: dm,
        period,
        composite: true,
        feed: false,
    });
    r.tag(dmdown, "dm_minus");
    r.kind = "dm";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(dmup, period));
    r.outputs.push(Sink::iface_lead(dmdown, period));
    // `dm::State` holds the two COMPOSITE Wilders accumulators plus the
    // previous bar's raw high/low. Captured via dedicated `Node::Mirror`
    // nodes (NOT the `ph`/`pl` `Lag` nodes already feeding the movement
    // calc above) so tail-capture never depends on how a future `Lag`
    // state-resolution might be defined.
    let mirror_high = r.add(Node::Mirror { src: high });
    r.tag(mirror_high, "prev_high_mirror");
    let mirror_low = r.add(Node::Mirror { src: low });
    r.tag(mirror_low, "prev_low_mirror");
    r.tail_inputs = vec![dmup, dmdown, mirror_high, mirror_low];
    r
}

/// `DX(p)` = `(|DM+ − DM−| · 100 / (DM+ + DM−)).max(0)` — a PURE function of the
/// two COMPOSITE accumulators, so it grafts `dm`'s own graph (one source of
/// truth); the VALUE math needs only high/low. `close` (and the mean atr
/// chain grafted below) exist purely so `dx::State` — a transparent wrapper
/// around the FULL `di::State`, atr_state included — can be restored; see the
/// `tail_inputs` comment below for the cost note. Exact op order
/// `(diff · 100) / sum` (left-assoc), then `Max(_,0)`; the divide emits the raw
/// sum (hand `calc_dx` skips the `÷atr`). Publish lead = `period`. `Adx` chains
/// a mean `Wilders` onto this node.
pub fn dx(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hlc();
    let dm = r.sub_graph(dm(&[period as f64]), &[0, 1]);
    let (dmup, dmdown) = (dm.outputs[0], dm.outputs[1]);
    let zero = r.constant(0.0);
    let dm_sub = r.add(Node::Sub { a: dmup, b: dmdown });
    r.tag(dm_sub, "dm_diff_raw");
    let dm_diff = r.add(Node::Abs { src: dm_sub });
    r.tag(dm_diff, "dm_diff");
    let hundred = r.constant(100.0);
    let numer = r.add(Node::Mul {
        a: dm_diff,
        b: hundred,
    });
    r.tag(numer, "dx_numer");
    let denom = r.add(Node::Add { a: dmup, b: dmdown });
    r.tag(denom, "dx_denom");
    let quot = r.add(Node::Div { a: numer, b: denom });
    let dx = r.add(Node::Max { a: quot, b: zero });
    r.tag(dx, "dx");
    r.kind = "dx";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(dx, period));
    // `dx::State` is a transparent wrapper around the FULL `di::State`
    // (`di_state` + `atr_state`), so restorability needs the same chains
    // `di` captures, even though `dx`'s own VALUE never reads them: the
    // shared mean `Wilders(tr)` atr chain + `close`/`high`/`low` mirrors.
    // Real cost note: this adds the atr/tr chain to a bare standalone `dx`
    // request that previously never built it — free when co-requested with
    // `atr`/`di`/`natr`/`supertrend`/`adx` (interning dedups it), a genuine
    // (if narrow) extra cost otherwise.
    let close = r.add(Node::Input { stream: 2 });
    let atr_id = r.sub(super::atr(&[period as f64]), &[0, 1, 2]);
    let high = r.add(Node::Input { stream: 0 });
    let low = r.add(Node::Input { stream: 1 });
    let prev_high = r.add(Node::Mirror { src: high });
    r.tag(prev_high, "prev_high_mirror");
    let prev_low = r.add(Node::Mirror { src: low });
    r.tag(prev_low, "prev_low_mirror");
    let prev_close = r.add(Node::Mirror { src: close });
    r.tag(prev_close, "prev_close_mirror");
    r.tail_inputs = vec![dmup, dmdown, prev_high, prev_low, atr_id, prev_close];
    r
}

/// `DI(p)` = `(±DM_smoothed · ((100 / period) / ATR_mean)).max(0)`, inputs
/// (high, low, close). The `+DM`/`−DM` graph and its two COMPOSITE accumulators
/// are the SAME nodes `dm` builds. The denominator is the shared **MEAN**
/// `Wilders(tr)` chain — the very node `atr`/`natr`/`supertrend`/`adx` build —
/// not a second raw-sum `Wilders(tr)`. That is legal because the hand's
/// denominator is that same chain scaled: `AtrState::init_state(…, true)` keeps
/// the raw sum and `atr.rs:60-62` divides by `period` only when `!composite`,
/// while the mean step is the sum step scaled by `1/p` (`st*mult + s*(1/p) ==
/// (st*mult + s)/p`), so `mean == sum/p` holds EVERY bar and
/// `100/sum == (100/p)/mean` — the period cancels into the CONSTANT.
///
/// The second chain it replaces was not merely redundant, it was BROKEN: a mean
/// slot and a composite slot over the SAME interned `tr` made the emitter publish
/// the raw SUM into the mean slot's row (measured 2026-10-16: adx's `atr` mirror
/// grew 15.2 → 40.4 where the hand prints ~6.1, while `erase` was proven correct
/// with `invms` 1.0 and 1/14 on the two slots). One form per source, so that shape
/// cannot arise.
///
/// Exact op order: `atr_inv = (100/p)/atr` shared by both, `pdi =
/// max(dmup·atr_inv, 0)`, `mdi = max(dmdown·atr_inv, 0)`. `Max` lowers to
/// `f64::max` (IEEE maxNum), reproducing the hand's NaN→0 when `atr = 0` makes
/// `0·inf`; CLIF `fmax` would return NaN here, so it is deliberately not used.
/// Publish lead = `period`.
///
/// PARITY COST, stated because it is real: `dm`/`dx`/`adx` rows stay bit-exact,
/// but DI's two rows are RELATIVE (<=1e-12 measured) against the hand —
/// `(100/p)/mean` rounds differently from `100/sum`. Gate:
/// `dm_di_dx_adx_shared_p_bit_exact` compares them with `assert_rel_1e12`.
pub fn di(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hlc();
    let zero = r.constant(0.0);
    let dm = r.sub_graph(dm(&[period as f64]), &[0, 1]);
    let (dmup, dmdown) = (dm.outputs[0], dm.outputs[1]);
    // THE SHARED MEAN-ATR CHAIN — the very node `atr`/`natr`/`supertrend` build,
    // so one `Wilders(tr)` serves the whole directional AND volatility family.
    //
    // DI's hand denominator is the running SUM (`AtrState::init_state(…, true)` +
    // `partial_calc`), and the mean chain is that sum divided by `period` at the
    // seam (hand `atr.rs`: `if !composite { atr /= period }`), with the steady step
    // scaled identically: with `inv = 1/p`, `st_mean*mult + s*inv` == `(st_sum*mult
    // + s)/p`, so `mean == sum/p` holds EVERY bar, not just at the seam.
    // Therefore `100/sum == (100/p)/mean` and the period cancels into the
    // CONSTANT — which is what lets DI read the shared chain instead of building a
    // second `Wilders(tr)` of its own.
    //
    // The second chain was not merely redundant, it was BROKEN: a mean slot and a
    // composite slot over the SAME interned `tr` made the emitter publish the raw
    // SUM into the mean slot's row (measured 2026-10-16: adx's `atr` mirror grew
    // 15.2 → 40.4 where the hand prints ~6.1, while `erase` was proven correct with
    // `invms` 1.0 and 1/14 on the two slots). One form per source, so that shape
    // cannot arise.
    //
    // PARITY COST, stated because it is real: `dm`/`dx`/`adx` rows stay
    // bit-exact, but these two rows are now RELATIVE (<=1e-12 measured) against the
    // hand — `(100/p)/mean` rounds differently from `100/sum`. Gate:
    // `dm_di_dx_adx_shared_p_bit_exact` compares them with `assert_rel_1e12`.
    let atr = r.sub(super::atr(&[period as f64]), &[0, 1, 2]);
    // `100.0 / period` — the factor the SUM-vs-MEAN scale cancels into.
    let scale = r.constant(100.0 / period as f64);
    let atr_inv = r.add(Node::Div { a: scale, b: atr });
    r.tag(atr_inv, "atr_inv");
    let up_di = r.add(Node::Mul {
        a: dmup,
        b: atr_inv,
    });
    let pdi = r.add(Node::Max { a: up_di, b: zero });
    r.tag(pdi, "plus_di");
    let dn_di = r.add(Node::Mul {
        a: dmdown,
        b: atr_inv,
    });
    let mdi = r.add(Node::Max { a: dn_di, b: zero });
    r.tag(mdi, "minus_di");
    r.kind = "di";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(pdi, period));
    r.outputs.push(Sink::iface_lead(mdi, period));
    // `di::State` holds `{di_state: dm::State, atr_state: atr::State}`.
    // `di_state` needs dm's two COMPOSITE Wilders accumulators (already
    // shared via the `dm` sub-graph above) plus prev_high/prev_low — captured
    // via FRESH top-level `Mirror` nodes (not reaching into `dm`'s own
    // internal mirrors, which `sub_graph` does not expose); these intern-
    // dedup with dm's identical mirrors at merge time since `Node::Mirror`
    // equality is structural. `atr_state` needs the COMPOSITE (raw-sum) atr,
    // but this recipe deliberately reads the shared MEAN chain (`atr`, see
    // the doc comment above) — the restorer recovers the sum by scaling the
    // captured mean back by `period` (`mean == sum/period` holds every bar).
    let high = r.add(Node::Input { stream: 0 });
    let low = r.add(Node::Input { stream: 1 });
    let close = r.add(Node::Input { stream: 2 });
    let prev_high = r.add(Node::Mirror { src: high });
    r.tag(prev_high, "prev_high_mirror");
    let prev_low = r.add(Node::Mirror { src: low });
    r.tag(prev_low, "prev_low_mirror");
    let prev_close = r.add(Node::Mirror { src: close });
    r.tag(prev_close, "prev_close_mirror");
    r.tail_inputs = vec![dmup, dmdown, prev_high, prev_low, atr, prev_close];
    r
}

/// `ADX(p)` = a MEAN `Wilders` over the `dx` node, but **feed-gated**: hand
/// `init_state` folds the FIRST `period` *published* `dx` values, divides by
/// `period`, then steady-decays — so the fold window starts at `dx`'s publish
/// lead, not bar 0 (the `feed: true` axis), because `dx` is only meaningful
/// after its own warmup. The `dx` chain is composed from `dx` rather than
/// re-derived, so the whole `+DM`/`−DM` → `dx` subgraph has one source of truth
/// and dedups with any co-requested `dm`/`di`/`dx`. `adx` never reads the ATR
/// for its value, so the `atr`/`tr` mirrors compose `atr` (mean chain + exposed
/// tr), shared with natr/atr. Publish lead = `2·period − 1`.
pub fn adx(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hlc();
    let dx = r.sub(dx(&[period as f64]), &[0, 1, 2]);
    let adx = r.add(Node::Wilders {
        src: dx,
        period,
        composite: false,
        feed: true,
    });
    r.tag(adx, "adx");
    // atr/tr mirrors: compose the mean ATR chain (primary + exposed tr) — the
    // same nodes natr/atr use, so shared.
    let atr = r.sub_graph(super::atr(&[period as f64]), &[0, 1, 2]);
    let atr_id = atr.primary;
    let tr = atr.optional_at(0);
    r.kind = "adx";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(adx, 2 * period - 1));
    // Hand optional_outputs order: [dx, atr, tr].
    r.optional_outputs.push(Sink::iface_lead(dx, period));
    r.optional_outputs.push(Sink::iface_lead(atr_id, period));
    r.optional_outputs.push(Sink::iface_lead(tr, 1));
    // `adx::State` is `{dx_state: dx::State (= FULL di::State), wilders_state}`.
    // `dx_state` needs the same 6 chains `di`/`dx` capture: `r.sub(dx(...))`
    // only returns dx's PRIMARY (the final node), discarding its internal
    // dm/mirror ids, so this recipe builds its OWN `dm` subgraph (free: it
    // interns to the same nodes dx's internal one builds) plus fresh
    // prev_high/prev_low mirrors (the atr chain + prev_close are already
    // built above for the atr/tr optional mirrors). `wilders_state` is this
    // node's own `adx` Wilders node.
    let dm = r.sub_graph(dm(&[period as f64]), &[0, 1]);
    let (dmup, dmdown) = (dm.outputs[0], dm.outputs[1]);
    let high = r.add(Node::Input { stream: 0 });
    let low = r.add(Node::Input { stream: 1 });
    let close = r.add(Node::Input { stream: 2 });
    let prev_high = r.add(Node::Mirror { src: high });
    r.tag(prev_high, "prev_high_mirror");
    let prev_low = r.add(Node::Mirror { src: low });
    r.tag(prev_low, "prev_low_mirror");
    let prev_close = r.add(Node::Mirror { src: close });
    r.tag(prev_close, "prev_close_mirror");
    r.tail_inputs = vec![dmup, dmdown, prev_high, prev_low, atr_id, prev_close, adx];
    r
}

/// `VHF(p)` = `(max − min) / max(Σ|Δclose|, ε)` over a `p`-bar window — the ratio
/// of the range the price covered to the distance it travelled, so a straight
/// trend scores high and a choppy one low. Hand facts, read off
/// `State::init_state`/`calc`:
/// · the numerator is the SAME `(p, p−1)` `RollMax`/`RollMin` pair `willr`
///   declares over one stream, so `vhf` + `willr` in one query walks each window
///   once;
/// · the denominator is a plain SUM, not a mean (`RollSum{mean: false}`, like
///   `cmo` — the hand maintains `sum += new − old`);
/// · its source is `|real − prev_real|`, a DERIVED node: the leaving term cannot
///   come from the host price anchor, so the ring holds the absolute bar's `Abs`
///   value and drops it `p` bars later;
/// · **no `skip`/`steady_from`, unlike `stoch`.** The hand's init sums `|Δ|` for
///   bars `1..=period` with a buffer exactly `period` wide, so the window is
///   complete at the bar it first publishes and the loop never subtracts during
///   seeding — both recurrences coincide and the defaults hold;
/// · the divisor floor is `(sum).max(f64::EPSILON)`, a `Max` against a positive
///   constant, NOT `willr`'s `WhereLt`: an unchanging series divides by ε instead
///   of printing `0.0`;
/// · `init_state` writes `indicator_line[0]` at bar `period` and `cycle` runs from
///   `period + 1`, so the first printed bar is `period` — already the `Div`'s
///   computed lead — so the sink needs no override.
///
/// `get_states` is DEFERRED (same shape as `stoch`'s): the hand state's `buffer`
/// is this `RollSum`'s own ring, so the export is the `ChainState::Hma{ring,
/// cursor}` precedent, and `prev_real` is a `Mirror` once that lands. Running and
/// resuming are bit-exact without it.
pub fn vhf(options: &[f64]) -> Recipe {
    let period = options[0] as Id;
    let look_back = period - 1;

    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    let mx = r.add(Node::RollMax {
        src: real,
        period,
        look_back,
    });
    r.tag(mx, "highest");
    let mn = r.add(Node::RollMin {
        src: real,
        period,
        look_back,
    });
    r.tag(mn, "lowest");
    let prev = r.add(Node::Lag { src: real, n: 1 });
    r.tag(prev, "prev_real");
    let diff = r.add(Node::Sub { a: real, b: prev });
    let travel = r.add(Node::Abs { src: diff });
    r.tag(travel, "abs_move");
    let sum = r.add(Node::RollSum {
        src: travel,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(sum, "travel_sum");
    let range = r.add(Node::Sub { a: mx, b: mn });
    r.tag(range, "range");
    let eps = r.constant(f64::EPSILON);
    let den = r.add(Node::Max { a: sum, b: eps });
    r.tag(den, "travel_floor");
    r.tail_inputs = vec![mx];
    let line = r.add(Node::Div { a: range, b: den });
    r.tag(line, "vhf");

    r.kind = "vhf";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface(line));
    r
}

/// `Volatility(p)` — the hand's volatility indicator: rate-of-change values
/// with std dev scaled by annual multiplier (252.sqrt() ≈ 15.8745).
///
/// Graph structure:
///   close → lag → sub → div → roc (rate of change)
///   roc → RollSum(p) → mean (mean of ROC values)
///   roc² → RollSum(p) → mean_sq (mean of squared ROC values)
///   variance = mean_sq - mean²
///   stddev = sqrt(variance.max(EPSILON))
///   volatility = stddev * annual
///
/// Note: We cannot use Stddev node directly because it requires a price source.
/// Instead, we compute std dev manually from two rolling sums over the derived
/// ROC values. This matches the hand's semantics while being CSE-emittable.
pub fn volatility(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "close");

    // Lag by 1 bar to get prev_real for ROC computation and state restoration
    let lag = r.add(Node::Lag { src: close, n: 1 });
    r.tag(lag, "lag");

    // Compute rate of change: (close - prev_close) / prev_close
    // This is equivalent to close/prev_close - 1 but more numerically stable
    let diff = r.add(Node::Sub { a: close, b: lag });
    r.tag(diff, "diff");

    let roc = r.add(Node::Div { a: diff, b: lag });
    r.tag(roc, "rate_of_change");

    // Compute ROC squared for variance calculation
    let roc_sq = r.add(Node::Mul { a: roc, b: roc });
    r.tag(roc_sq, "roc_squared");

    // Use RollSum nodes on derived ROC values - these work via rings and can handle
    // derived sources. The hand's stddev computes variance as mean_of_squares - mean^2.
    // The hand's init_state folds bars 1..=period (ROC values from close[1]/close[0]-1 etc).
    // To match, we skip bar 0 where ROC=0 (lag clamps), then fold period bars starting
    // at bar 1. RollSum with skip=1 and steady_from=1 matches hand's init loop exactly.
    let roc_sum = r.add(Node::RollSum {
        src: roc,
        period,
        mean: true,     // This makes it a rolling MEAN (sum * 1/period)
        skip: 1,        // Skip bar 0 where ROC=0 (lag clamps to same value)
        steady_from: 1, // Start computing from bar 1 (after the skipped bar)
    });
    r.tag(roc_sum, "roc_mean");

    let roc_sq_sum = r.add(Node::RollSum {
        src: roc_sq,
        period,
        mean: true,
        skip: 1,
        steady_from: 1,
    });
    r.tag(roc_sq_sum, "roc_sq_mean");

    // Compute variance = mean_of_squares - mean^2 (same as hand's stddev calc)
    let mean_sq = r.add(Node::Mul {
        a: roc_sum,
        b: roc_sum,
    });
    r.tag(mean_sq, "mean_squared");
    let variance = r.add(Node::Sub {
        a: roc_sq_sum,
        b: mean_sq,
    });
    r.tag(variance, "variance");

    // Floor variance at EPSILON for numerical stability (hand does max() on sqrt result)
    let eps = r.constant(f64::EPSILON);
    r.tag(eps, "epsilon");
    let var_floor = r.add(Node::Max {
        a: variance,
        b: eps,
    });
    r.tag(var_floor, "variance_floored");

    // Compute stddev = sqrt(variance)
    let sd = r.add(Node::Sqrt { src: var_floor });
    r.tag(sd, "stddev");

    // Annual multiplier constant (252.sqrt() = 15.874507866387544)
    let annual = r.constant(15.874507866387544);
    r.tag(annual, "annual");

    // Scale stddev by annual factor (hand: sd * ANNUAL)
    let vol = r.add(Node::Mul { a: sd, b: annual });
    r.tag(vol, "volatility");

    r.kind = "volatility";
    r.options = options.to_vec();

    // Hand State holds: buffer (for stddev's prev_value), stddev_state, prev_real
    // The RollSum nodes capture sum/sum_sq via Values periods; we need close_mirror
    // for prev_real restoration. Tail inputs match the Restorable implementation.
    let close_mirror = r.add(Node::Mirror { src: close });
    r.tag(close_mirror, "close_mirror");

    r.tail_inputs = vec![roc_sum, roc_sq_sum, close_mirror];

    // Lead is period + 1 - hand `init_state` consumes bars 0..=period (period+1 bars),
    // computes sum/sum_sq over ROC[1..=period], then `cycle` starts at bar period+1.
    // CSE must match: RollSum seams at bar period+1 (fold=skip=1, seam=1+p-1=p+1).
    r.outputs.push(Sink::iface_lead(vol, period + 1));

    r
}

// ── the trail-rescan hub ──────────────────────────────────────────────────
//
// `RollMax`/`RollMin` carry `{value, trail}` and step the hand's trail rule
// exactly: `trail += 1`; when `period <= trail` RESCAN the
// `look_back + 1`-bar window (hand `find_max_simd::<4>` /
// `find_min_simd::<4>` — the tie rules included, the engine calls THOSE
// functions); else fold the entering bar (`>= max` / `<= min` takes it).
// `Trail` projects the carried trail as a value. Windows are the whole
// game: `max/min/willr/donchian/ichimoku` are CANONICAL `(p, p−1)`;
// `aroon` is `(p, p)` — its window is `p + 1` bars, which is the user's
// `Aroon(9) ≡ max(10)` window rule: same WINDOW, different trigger, values
// (and trails) identical either way, so interning on the full
// `(src, period, look_back)` key stays correct and two spellings of the
// same window share only when both fields agree.

/// `MAX(p)` — the trail-rescan rolling maximum over one stream. Hand facts:
/// window `(p, p−1)`; first printed bar `p−1` = the node's defined-from, so
/// the sink needs no explicit lead.
pub fn max(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "price");
    let m = r.add(Node::RollMax {
        src: real,
        period,
        look_back: period.saturating_sub(1),
    });
    r.tag(m, "roll_max");
    r.kind = "max";
    r.options = options.to_vec();
    r.tail_inputs = vec![m];
    r.outputs.push(Sink::iface(m));
    r
}

/// `MIN(p)` — the mirror of [`max`] (the hand's init-bar asymmetry is
/// value-invisible; the engine uses the uniform rule, pinned by parity).
pub fn min(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "price");
    let m = r.add(Node::RollMin {
        src: real,
        period,
        look_back: period.saturating_sub(1),
    });
    r.tag(m, "roll_min");
    r.kind = "min";
    r.options = options.to_vec();
    r.tail_inputs = vec![m];
    r.outputs.push(Sink::iface(m));
    r
}

/// The aroon node graph (both consumers build through it so every node is
/// byte-identical: co-requesting aroon + aroonosc interns the WHOLE shape —
/// one roll pair, one trail pair, one scaled-line pair, the oscillator
/// costing exactly one `fsub` + one store). Returns
/// `{down, up, min_roll, max_roll}`.
pub struct AroonLines {
    pub down: Id,
    pub up: Id,
    pub min_roll: Id,
    pub max_roll: Id,
}

pub fn aroon_graph(r: &mut Recipe, period: u32) -> AroonLines {
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    // AROON's window is `period + 1` bars: `look_back` EQUAL to period,
    // unlike the canonical rolls — the `Aroon(9) ≡ max(10)` rule.
    let max_roll = r.add(Node::RollMax {
        src: high,
        period,
        look_back: period,
    });
    r.tag(max_roll, "roll_max");
    let min_roll = r.add(Node::RollMin {
        src: low,
        period,
        look_back: period,
    });
    r.tag(min_roll, "roll_min");
    let tmax = r.add(Node::Trail { src: max_roll });
    r.tag(tmax, "trail_max");
    let tmin = r.add(Node::Trail { src: min_roll });
    r.tag(tmin, "trail_min");
    let p = r.constant(period as f64);
    r.tag(p, "period_f");
    let mult = r.constant(100.0 / period as f64);
    r.tag(mult, "mult");
    // hand `calc_aroon`: `(period - trail) as f64 * multiplier` — integer
    // subtract, one conversion; float over the projected trail is
    // bit-identical (exact integers < 2^53).
    let up_gap = r.add(Node::Sub { a: p, b: tmax });
    let up = r.add(Node::Mul { a: up_gap, b: mult });
    r.tag(up, "aroon_up");
    let down_gap = r.add(Node::Sub { a: p, b: tmin });
    let down = r.add(Node::Mul {
        a: down_gap,
        b: mult,
    });
    r.tag(down, "aroon_down");
    AroonLines {
        down,
        up,
        min_roll,
        max_roll,
    }
}

/// `AROON` — the two trail-scaled lines over the roll pair. Hand facts:
/// both lines publish from bar `period` (the rolls' defined-from is one
/// bar earlier), hence the explicit sink leads.
pub fn aroon(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hl();
    let g = aroon_graph(&mut r, period);
    r.kind = "aroon";
    r.options = options.to_vec();
    // hand `State` field order: min_state THEN max_state
    r.tail_inputs = vec![g.min_roll, g.max_roll];
    r.outputs = vec![
        Sink::iface_lead(g.down, period),
        Sink::iface_lead(g.up, period),
    ];
    r
}

/// `AROONOSC` = `aroon_up − aroon_down` over the SAME interned aroon graph.
pub fn aroonosc(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hl();
    let g = aroon_graph(&mut r, period);
    let osc = r.add(Node::Sub { a: g.up, b: g.down });
    r.tag(osc, "aroonosc");
    r.kind = "aroonosc";
    r.options = options.to_vec();
    r.tail_inputs = vec![g.min_roll, g.max_roll];
    r.outputs = vec![Sink::iface_lead(osc, period)];
    // hand `Info::optional_outputs` order: [aroon_down, aroon_up].
    r.optional_outputs = vec![
        Sink::iface_lead(g.down, period),
        Sink::iface_lead(g.up, period),
    ];
    r
}

/// `WILLR` = `100·(max − close) / (max − min)`, zeroed on a degenerate
/// range. `WhereLt{|range|, ε, ratio}` = `if |range| < ε {0} else {ratio}`
/// — the hand's guard form (a NaN compare is false BOTH ways, so NaN
/// propagates through the division exactly as the hand does). Rolls are
/// canonical `(p, p−1)`: max-of-high / min-of-low intern with
/// `donchianchannel`/`stoch` at the same period.
pub fn willr(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let look_back = period - 1;
    let mut r = Recipe::hlc();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let close = r.add(Node::Input { stream: 2 });
    r.tag(close, "close");
    let mx = r.add(Node::RollMax {
        src: high,
        period,
        look_back,
    });
    r.tag(mx, "roll_max");
    let mn = r.add(Node::RollMin {
        src: low,
        period,
        look_back,
    });
    r.tag(mn, "roll_min");
    let dif = r.add(Node::Sub { a: mx, b: mn });
    r.tag(dif, "roll_range");
    let adif = r.add(Node::Abs { src: dif });
    r.tag(adif, "range_abs");
    let mc = r.add(Node::Sub { a: mx, b: close });
    r.tag(mc, "max_close");
    let c100 = r.constant(100.0);
    r.tag(c100, "hundred");
    let num = r.add(Node::Mul { a: c100, b: mc });
    r.tag(num, "numerator");
    let q = r.add(Node::Div { a: num, b: dif });
    r.tag(q, "ratio");
    let eps = r.constant(f64::EPSILON);
    r.tag(eps, "eps");
    let wr = r.add(Node::WhereLt {
        a: adif,
        b: eps,
        t: q,
    });
    r.tag(wr, "willr");
    r.kind = "willr";
    r.options = options.to_vec();
    r.tail_inputs = vec![mn, mx];
    // primary prints from bar `period`; the min/max optionals print ONE BAR
    // EARLIER (hand init_state stores them at `look_back`), so they take the
    // nodes' own defined-from lead.
    r.outputs = vec![Sink::iface_lead(wr, period)];
    r.optional_outputs = vec![Sink::iface(mn), Sink::iface(mx)];
    r
}

/// `VORTEX(p)` — Vortex Indicators, hand `vortex.rs`. Two bands that share ONE true-range
/// walk and one window:
/// `vi_up = sum(|high - low[1]|, p) / sum(tr, p)`, `vi_down = sum(|low - high[1]|, p) /
/// sum(tr, p)`, with `tr` as the optional third row.
///
/// The hand keeps `prev_low_high: Simd<f64,2>` updated to `Simd::from_array([high,low]).
/// reverse()`, i.e. `[low[i-1], high[i-1]]`, so `vm_simd = ([high,low] - prev_low_high).abs()`
/// is exactly `|high - low[1]|` and `|low - high[1]|`. Those are lags over COLUMNS, which
/// `Kind::Lag` can read -- so unlike adxr this needs no new node kind.
///
/// EVICTION, NOT PUBLISHED LAG: `tr_sum += tr - old_tr` and `vm_sums += vm_simd - old_vm`
/// are a window's leaving terms, so three ring `RollSum`s express it (the `chaikinmf`
/// precedent). The hand's warm pass accumulates WITHOUT eviction (`tr_sum += tr`) while the
/// buffer fills, and its published windows therefore start at bar 1; the engine's ring folds
/// bar 0 too, but bar 0's `tr`/`vm` are evicted before any published window opens, so the
/// published values agree. That agreement is the gate's job, not this comment's.
///
/// Lead = `period + 1`, which is NOT `min_data - 1` here: `vortex.rs:218` overrides
/// `output_length` as `data_len - Self::min_data(options)` with the `+ 1` commented out, so
/// rows = `len - period - 1` (the hand test's `expected_main_len = n - period - 1`). Using
/// the trait-default lead would print one bar early -- the zlema/stoch class of error.
/// The `tr` mirror keeps the `Tr` indicator's own lead 1 (`tr_line` is sized
/// `Tr::output_length`).
pub fn vortex(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hlc();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let cl = r.add(Node::Input { stream: 2 });
    r.tag(cl, "close");

    // The shared true-range walk -- the same nodes `tr`/`atr`/`adx` build.
    let tr = super::tr_nodes(&mut r, hi, lo, cl);

    let lag_hi = r.add(Node::Lag { src: hi, n: 1 });
    r.tag(lag_hi, "prev_high");
    let lag_lo = r.add(Node::Lag { src: lo, n: 1 });
    r.tag(lag_lo, "prev_low");
    let vm_up_src = r.add(Node::Sub { a: hi, b: lag_lo });
    r.tag(vm_up_src, "h_minus_prev_l");
    let vm_up = r.add(Node::Abs { src: vm_up_src });
    r.tag(vm_up, "vm_up");
    let vm_dn_src = r.add(Node::Sub { a: lo, b: lag_hi });
    r.tag(vm_dn_src, "l_minus_prev_h");
    let vm_dn = r.add(Node::Abs { src: vm_dn_src });
    r.tag(vm_dn, "vm_dn");

    let sum_up = r.add(Node::RollSum {
        src: vm_up,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(sum_up, "vm_up_sum");
    let sum_dn = r.add(Node::RollSum {
        src: vm_dn,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(sum_dn, "vm_dn_sum");
    let sum_tr = r.add(Node::RollSum {
        src: tr,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(sum_tr, "tr_sum");

    let vi_up = r.add(Node::Div {
        a: sum_up,
        b: sum_tr,
    });
    r.tag(vi_up, "vi_up");
    let vi_dn = r.add(Node::Div {
        a: sum_dn,
        b: sum_tr,
    });
    r.tag(vi_dn, "vi_down");

    r.kind = "vortex";
    r.options = options.to_vec();
    r.tail_inputs = vec![sum_up, sum_dn, sum_tr];
    r.outputs.push(Sink::iface_lead(vi_up, period + 1));
    r.outputs.push(Sink::iface_lead(vi_dn, period + 1));
    // Info.optional_outputs = ["tr"] -- the same arena node `tr_n` publishes.
    r.optional_outputs.push(Sink::iface_lead(tr, 1));
    r
}

/// The arena ids of the linreg sub-chain, so `linreg` and `tsf` can build it
/// through ONE function and have their nodes intern together by construction
/// rather than by two hand-maintained copies agreeing by luck.
struct LinregChain {
    real: Id,
    sum_y: Id,
    sum_xy: Id,
    slope: Id,
    intercept: Id,
    line: Id,
}

/// The linreg sub-chain. Hand `linreg.rs`; the engine ALWAYS runs the hand's `calc`
/// path (never `partial_calc`, user decision 2026-10-31), which is what makes
/// the primary row bit-identical to `tsf`'s re-published linreg row and lets
/// the two requests intern one chain.
///
/// The two rolling moments are the SAME machine — `publish = st + src·mult`,
/// then `st = publish − sub` — so they are two instances of one
/// `Node::LinregFlow`, differing only in `mult` (1.0 vs `period`, i.e.
/// `Values::mults`) and in what `sub` names:
/// · `sum_y`  evicts `Lag(real, period − 1)`, the bar the hand calls
///   `prev_value`: it works on `&real[1..]`, so at published bar `t` that is
///   `real[t − period + 1]` — a column lag, which is the one lag `Kind::Lag`
///   can spell.
/// · `sum_xy` evicts the `sum_y` node PUBLISHED THE SAME BAR, because the
///   hand's `sum_xy -= self.sum_y` runs after `sum_y += value` and before
///   `sum_y -= prev_value`. A reused roll cannot stand in for it: the hand
///   publishes BEFORE it evicts, and `RollSum` folds the leave into its
///   published value either way round. Measured: this shape is bit-exact on
///   every bar of 24 series×period cells; the roll window breaks 697/698.
///
/// The downstream arithmetic is `calc` statement for statement, spelled with
/// NO negation node: the hand writes `-(sum_x * sum_y)` and `(-slope).mul_add(
/// sum_x, sum_y)`, and IEEE takes a product's sign independently of its
/// magnitude rounding, so `(-sum_x)·y` is bit-identical to `-(sum_x·y)` —
/// including at signed zero, which `zero − z` would get wrong. One
/// `Const(-sum_x)` therefore serves both the slope and the intercept fma.
fn linreg_chain(r: &mut Recipe, period: u32) -> LinregChain {
    // `init_state`'s two loops, verbatim (linreg.rs:85-96). The values are
    // integers well under 2^53 so the float and integer spellings agree, but
    // the shape is copied so a reader can diff against the hand.
    let mut sum_x = 0.0f64;
    let mut sum_xx = 0.0f64;
    for i in 0..period.saturating_sub(1) {
        sum_x += (i + 1) as f64;
        sum_xx += ((i + 1) as f64).powi(2);
    }
    let nf = period as f64;
    sum_x += nf;
    // The hand writes `(period * period) as f64` on a `usize`, so its integer
    // square cannot overflow; `period` here is `u32`, and the SAME spelling
    // panicked the `kernel!` macro's const evaluation on a large registry
    // period. Widening before the multiply is bit-identical for every period
    // whose square fits — it just doesn't abort the build when it doesn't.
    let pp = period as u64;
    sum_xx += (pp * pp) as f64;
    // `multiplier(period, sum_x, sum_xx)` — and note the hand's own integer
    // square cast, not `nf * nf`.
    let per = 1.0 / (nf * sum_xx - sum_x.powi(2));
    let inv_n = 1.0 / nf;

    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "price");
    let prev = r.add(Node::Lag {
        src: real,
        n: period.saturating_sub(1),
    });
    r.tag(prev, "prev");
    let sum_y = r.add(Node::LinregFlow {
        src: real,
        sub: prev,
        period,
        weighted: false,
    });
    r.tag(sum_y, "sum_y");
    let sum_xy = r.add(Node::LinregFlow {
        src: real,
        sub: sum_y,
        period,
        weighted: true,
    });
    r.tag(sum_xy, "sum_xy");

    let nf_c = r.constant(nf);
    let neg_sum_x_c = r.constant(-sum_x);
    let per_c = r.constant(per);
    let inv_n_c = r.constant(inv_n);

    // slope = (n·sum_xy − sum_x·sum_y) · per
    let sx_sy = r.add(Node::Mul {
        a: neg_sum_x_c,
        b: sum_y,
    });
    let s_num = r.add(Node::Fma {
        a: nf_c,
        b: sum_xy,
        c: sx_sy,
    });
    let slope = r.add(Node::Mul { a: s_num, b: per_c });
    r.tag(slope, "linregslope");
    // intercept = (sum_y − slope·sum_x) · inv_n — SAME negated constant, so
    // slope and intercept share one Const node.
    let i_num = r.add(Node::Fma {
        a: slope,
        b: neg_sum_x_c,
        c: sum_y,
    });
    let intercept = r.add(Node::Mul {
        a: i_num,
        b: inv_n_c,
    });
    r.tag(intercept, "linregintercept");
    // linreg = slope·n + intercept
    let line = r.add(Node::Fma {
        a: nf_c,
        b: slope,
        c: intercept,
    });
    r.tag(line, "linreg");

    LinregChain {
        real,
        sum_y,
        sum_xy,
        slope,
        intercept,
        line,
    }
}

/// `LINREG(p)` — the linreg sub-chain plus the hand's three published rows.
/// Design rationale (why a bespoke carry, why `calc` always) is on
/// [`linreg_chain`]; this wrapper only owns sinks.
pub fn linreg(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let c = linreg_chain(&mut r, period);
    r.kind = "linreg";
    r.options = options.to_vec();
    r.tail_inputs = vec![c.sum_y, c.sum_xy];
    // Lead = `period`: the hand sets no `min_data`, so the default
    // `options[OP-1] + 1` applies and it prints `len − period` rows.
    let lead = period;
    r.outputs.push(Sink::iface_lead(c.line, lead));
    r.optional_outputs.push(Sink::iface_lead(c.slope, lead));
    r.optional_outputs.push(Sink::iface_lead(c.intercept, lead));
    r
}

/// The one `Fma` that turns the linreg chain into tsf's forecast row.
///
/// `tsf.rs`'s `State<Warm>::calc` spells
/// `slope.mul_add(self.linreg_state.n + 1.0, intercept)`
/// with `n = period as f64` — one fused step, spelled exactly as the hand
/// spells it. The commented-out line above it (`intercept + slope *
/// (period + 1) as f64`) is the algebraic cousin the `trix` lesson warns
/// against: same value, different association, different bits.
///
/// Shared by `tsf` AND `fosc` so their nodes intern rather than duplicate.
fn tsf_row(r: &mut Recipe, period: u32, c: &LinregChain) -> Id {
    let n_plus_1 = r.constant(period as f64 + 1.0);
    let tsf = r.add(Node::Fma {
        a: c.slope,
        b: n_plus_1,
        c: c.intercept,
    });
    r.tag(tsf, "tsf");
    tsf
}

/// `TSF(p)` — hand `tsf.rs`: the SAME sub-chain, plus ONE `Fma`.
///
/// The sharing is structural, not a coincidence: `tsf.rs`'s `State::init_state`
/// delegates to `LinregState::init_state`, `cycle_tsf` keeps the identical
/// `(j, period − 1 + j)` pairing over `real[1..]` (so the same
/// `Lag(price, period−1)` evict and the same lead), and it calls
/// `TState::calc` UNCONDITIONALLY — the following `if has_optional` gates only
/// the STORES. So the `linreg` row tsf re-publishes is the same value `linreg`
/// publishes, which is what lets `linreg + tsf` intern one chain instead of two.
pub fn tsf(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let c = linreg_chain(&mut r, period);
    let tsf = tsf_row(&mut r, period, &c);
    r.kind = "tsf";
    r.options = options.to_vec();
    r.tail_inputs = vec![c.sum_y, c.sum_xy];
    let lead = period;
    r.outputs.push(Sink::iface_lead(tsf, lead));
    // `Info` order: outputs ["tsf"], optional ["linreg","linregslope",
    // "linregintercept"] — the hand returns `vec![tsf, linreg, slope, intercept]`.
    r.optional_outputs.push(Sink::iface_lead(c.line, lead));
    r.optional_outputs.push(Sink::iface_lead(c.slope, lead));
    r.optional_outputs.push(Sink::iface_lead(c.intercept, lead));
    r
}

/// `FOSC(p)` — hand `fosc.rs`: the tsf chain, plus a ONE-BAR PUBLISHED LAG of
/// the forecast and three arithmetic nodes.
///
/// `fosc.rs`'s `State<Warm>::calc` computes `fosc` from `self.tsf` BEFORE the
/// `self.tsf = tsf` at the end of that method overwrites it, so the value used
/// at bar `t` is the forecast PUBLISHED AT BAR `t−1`. That is the published-lag
/// class `Lag` cannot spell (`Kind::Lag` resolves its source's stream slot, so
/// it lags a COLUMN, not a node — see `trix`'s `RollAge{ema3,1}` for the working
/// precedent), and it is NOT an eviction either: the ring exists to hold a
/// printed value, not a leaving term. So `Node::RollAge { src: tsf, n: 1 }`.
///
/// Lead = `period + 1`, from the hand's OWN `Fosc::min_data` override
/// (`options[0] + 2`, unlike linreg/tsf which take the default) — and
/// `merge.rs`'s `RollAge` lead rule (`leads[src] + n`) gives the age node
/// `period + 1` for the same reason, so the row and its input agree. The four
/// re-published regression rows keep lead `period`, exactly as `tsf` publishes
/// them; the hand slices them apart with `slice_outputs_start!` in
/// `Fosc::indicator`.
///
/// `fosc = 100.0 * (value − tsf_prev) / value`, associated as
/// `(100·diff)/value` — Mul then Div, never `100·(diff/value)`. The divide is
/// UNGUARDED: the hand's `.max(f64::EPSILON)` is commented out, so no guard
/// node is added here (transcribe the line above, not the one that should be).
pub fn fosc(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let c = linreg_chain(&mut r, period);
    let tsf = tsf_row(&mut r, period, &c);
    let real = c.real;
    let prev_tsf = r.add(Node::RollAge { src: tsf, n: 1 });
    r.tag(prev_tsf, "prev_tsf");
    let hundred = r.constant(100.0);
    let diff = r.add(Node::Sub {
        a: real,
        b: prev_tsf,
    });
    let scaled = r.add(Node::Mul {
        a: hundred,
        b: diff,
    });
    let line = r.add(Node::Div { a: scaled, b: real });
    r.tag(line, "fosc");

    r.kind = "fosc";
    r.options = options.to_vec();
    r.tail_inputs = vec![c.sum_y, c.sum_xy];
    r.outputs.push(Sink::iface_lead(line, period + 1));
    // `Info` order: outputs ["fosc"], optional ["tsf","linreg","linregslope",
    // "linregintercept"]; the hand returns all five in that order.
    let lead = period;
    r.optional_outputs.push(Sink::iface_lead(tsf, lead));
    r.optional_outputs.push(Sink::iface_lead(c.line, lead));
    r.optional_outputs.push(Sink::iface_lead(c.slope, lead));
    r.optional_outputs.push(Sink::iface_lead(c.intercept, lead));
    r
}

/// `TRIMA(p)` -- Triangular Moving Average, hand `trima.rs`: the SMA-of-SMA
/// formulation. `m1 = (p+1)/2`, `m2 = p - m1 + 1`; `trima = SMA_m2(SMA_m1(real))`.
/// The inner SMA is price-sourced (`Node::Sma` only accepts a raw `Input`), but
/// the outer SMA's source is the inner SMA's OWN output -- a derived src, which
/// `Node::Sma`'s `erase()` guard rejects -- so the outer roll is spelled
/// `RollSum{mean:true}`, the derived-source form the hand's own telescoping
/// `sum2 += s1 - evicted` matches exactly (same incoming/outgoing-bar shape as
/// `vwma`/`vosc`/`qstick`'s derived rolls). Streams: 0=real. Options `[period]`.
/// Publish lead = `period - 1` (`min_data = period`, `trima.rs:233`), which is
/// exactly `lead(s1) + m2 - 1 = (m1-1) + (m2-1) = m1+m2-2 = p-1`.
pub fn trima(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let m1 = (period + 1) / 2;
    let m2 = period - m1 + 1;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    let s1 = r.add(Node::RollSum {
        src: real,
        period: m1,
        mean: true,
        skip: 0,
        steady_from: 0,
    });
    r.tag(s1, "sma1");
    let s2 = r.add(Node::RollSum {
        src: s1,
        period: m2,
        mean: true,
        skip: 0,
        steady_from: 0,
    });
    r.tag(s2, "trima");

    r.kind = "trima";
    r.options = options.to_vec();
    r.tail_inputs = vec![s1, s2];
    r.outputs.push(Sink::iface_lead(s2, period - 1));
    r
}

/// `ADXR(p)` = `0.5 * (adx + ring_delay(adx, p - 1))` -- hand `adxr.rs`: a
/// ring-backed age roll over the derived ADX line, capacity `period - 1`.
/// The `adx` subgraph is shared with dm/di/dx (grafted via `r.sub`), so one
/// request computes directional movement ONCE.
pub fn adxr(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hlc();
    let dx = r.sub(dx(&[period as f64]), &[0, 1, 2]);
    let adx = r.add(Node::Wilders {
        src: dx,
        period,
        composite: false,
        feed: true,
    });
    r.tag(adx, "adx");
    let aged = r.add(Node::RollAge {
        src: adx,
        n: period - 1,
    });
    r.tag(aged, "aged_adx");
    let sum = r.add(Node::Add { a: adx, b: aged });
    let half = r.constant(0.5);
    let adxr = r.add(Node::Mul { a: half, b: sum });
    r.tag(adxr, "adxr");
    r.kind = "adxr";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(adxr, 3 * (period - 1)));
    // `adxr::State` is `{adx_state: adx::State, buffer: Buffer<Warm, f64>}`.
    // `adx_state` needs the SAME 7 ids `adx()`'s own recipe exposes via its
    // `tail_inputs` -- going through `r.sub(dx(...))`/`r.sub(adx(...))` only
    // returns the PRIMARY id, discarding the internal dm/mirror/atr ids the
    // restorer needs (same reasoning as `adx()`'s own recipe comment above),
    // so this recipe builds its OWN `dm`/`atr` subgraphs directly (free:
    // they intern-dedup with the identical nodes `dx`'s internal subgraph
    // above already built). `buffer` is captured via the `aged`
    // `Node::RollAge` node itself (the ring `adxr`'s hand buffer rolls).
    let dm = r.sub_graph(dm(&[period as f64]), &[0, 1]);
    let (dmup, dmdown) = (dm.outputs[0], dm.outputs[1]);
    let high = r.add(Node::Input { stream: 0 });
    let low = r.add(Node::Input { stream: 1 });
    let close = r.add(Node::Input { stream: 2 });
    let prev_high = r.add(Node::Mirror { src: high });
    r.tag(prev_high, "prev_high_mirror");
    let prev_low = r.add(Node::Mirror { src: low });
    r.tag(prev_low, "prev_low_mirror");
    let prev_close = r.add(Node::Mirror { src: close });
    r.tag(prev_close, "prev_close_mirror");
    let atr = r.sub_graph(super::atr(&[period as f64]), &[0, 1, 2]);
    let atr_id = atr.primary;
    r.tail_inputs = vec![
        dmup, dmdown, prev_high, prev_low, atr_id, prev_close, adx, aged,
    ];
    r
}

/// `MD(p)` -- Mean Absolute Deviation over one stream. Hand `md.rs`
/// `calc_md`/`calc_md_simd`: `sum(|x - sma|) / period`, NOT incrementally
/// updatable (unlike `RollSum`) because `|x - mean|` isn't linear -- every
/// bar needs a full rescan of the window with the CURRENT bar's sma. `sma`
/// is an explicit shared `RollSum{mean:true}` operand.
pub fn md(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    let sma = r.add(Node::RollSum {
        src: real,
        period,
        mean: true,
        skip: 0,
        steady_from: 0,
    });
    r.tag(sma, "sma");
    let md = r.add(Node::MeanAbsDev {
        src: real,
        period,
        sma,
    });
    r.tag(md, "md");
    r.kind = "md";
    r.options = options.to_vec();
    // `md::State<S>` is `#[repr(transparent)] State(pub SmaState<S>)` — the
    // `sma` node (already `ChainState::Sma{sum, period}`) IS the whole
    // continuable state; the `md` output node itself is a derived, non-
    // incremental rescan value, never a chain to restore from.
    r.tail_inputs = vec![sma];
    r.outputs.push(Sink::iface(md));
    r.optional_outputs.push(Sink::iface(sma));
    r
}

/// `CCI(p)` -- Commodity Channel Index over HLC:
/// `(typprice - sma) / (0.015 * md)` with guard `if md == 0.0 { return 0.0 }`.
/// The `md` node is `MeanAbsDev` over `typprice` (derived source from the
/// inline typprice subgraph -- the `/3` divide matters, see [`super::typprice`]).
pub fn cci(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hlc();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let close = r.add(Node::Input { stream: 2 });
    r.tag(close, "close");
    let sum = r.add(Node::Add { a: high, b: low });
    let sum2 = r.add(Node::Add { a: sum, b: close });
    let third = r.constant(1.0 / 3.0);
    let typprice = r.add(Node::Mul { a: sum2, b: third });
    r.tag(typprice, "typprice");
    let sma = r.add(Node::RollSum {
        src: typprice,
        period,
        mean: true,
        skip: 0,
        steady_from: 0,
    });
    r.tag(sma, "sma");
    let md = r.add(Node::MeanAbsDev {
        src: typprice,
        period,
        sma,
    });
    r.tag(md, "md");
    let diff = r.add(Node::Sub {
        a: typprice,
        b: sma,
    });
    let scale = r.constant(0.015);
    let denom = r.add(Node::Mul { a: scale, b: md });
    let cci_raw = r.add(Node::Div { a: diff, b: denom });
    let zero = r.constant(0.0);
    let cci = r.add(Node::WhereLe {
        a: md,
        b: zero,
        t: cci_raw,
    });
    r.tag(cci, "cci");
    r.kind = "cci";
    r.options = options.to_vec();
    r.tail_inputs = vec![sma];
    r.outputs.push(Sink::iface_lead(cci, 2 * period - 2));
    r.optional_outputs.push(Sink::iface_lead(sma, period));
    r.optional_outputs.push(Sink::iface_lead(md, period));
    r.optional_outputs.push(Sink::iface(typprice));
    r
}

/// `CVI(p)` -- Chaikin Volatility: `(ema - ring_delay(ema, p)) / ring_delay(ema, p) * 100`,
/// hand `cvi.rs`: an EMA of `(high - low).max(EPSILON)`, ring-backed age roll
/// of capacity `period` (same `RollAge` machine as [`adxr`], `n = period`
/// directly instead of `period - 1`).
pub fn cvi(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hl();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let range = r.add(Node::Sub { a: high, b: low });
    r.tag(range, "hl_range");
    let eps = r.constant(f64::EPSILON);
    let range_clamped = r.add(Node::Max { a: range, b: eps });
    r.tag(range_clamped, "hl_range_clamped");
    let ema = r.add(Node::Ema {
        src: range_clamped,
        period,
    });
    r.tag(ema, "ema");
    let old_ema = r.add(Node::RollAge {
        src: ema,
        n: period,
    });
    r.tag(old_ema, "old_ema");
    let diff = r.add(Node::Sub { a: ema, b: old_ema });
    let ratio = r.add(Node::Div {
        a: diff,
        b: old_ema,
    });
    let hundred = r.constant(100.0);
    let cvi = r.add(Node::Mul {
        a: ratio,
        b: hundred,
    });
    r.tag(cvi, "cvi");
    r.kind = "cvi";
    r.options = options.to_vec();
    // hand `State` field order: `ema_state` (the `ema` node's own carry), then
    // `buffer` (the `old_ema` ring `RollAge` keeps) -- the output `cvi` node
    // itself is a derived ratio, never a chain to restore from.
    r.tail_inputs = vec![ema, old_ema];
    r.outputs.push(Sink::iface_lead(cvi, 2 * period - 1));
    r
}
