//! Trend channel recipes: atr, natr, supertrend, psar, bbands, donchianchannel, ichimoku, keltnerchannel, chandelierexit

use crate::cse::{Node, Recipe, Sink};

/// `ATR(p)` = `Wilders(tr(high, low, close))` — the mean-Wilders chain,
/// literally composed: streams (0,1,2) = (high, low, close), the tr graph,
/// then the smoothing node whose mean seed makes `init_state` bit-exact.
pub fn atr(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hlc();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let cl = r.add(Node::Input { stream: 2 });
    r.tag(cl, "close");
    let (w, tr) = super::atr_nodes(&mut r, hi, lo, cl, period);
    r.kind = "atr";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(w, period));
    // Hand `Info::optional_outputs = ["tr"]` — the exposed tr node is where
    // every family member's `tr` mirror comes from, never a re-derived copy.
    r.optional_outputs.push(Sink::iface_lead(tr, 1));
    // `atr::State` holds the mean `Wilders(tr)` chain plus `tr_state`'s one
    // scalar retention (`prev_close`, last close fed before the warm loop) —
    // `Node::Mirror{src: cl}` captures the latter, no new variant needed.
    let prev_close = r.add(Node::Mirror { src: cl });
    r.tag(prev_close, "prev_close");
    r.tail_inputs = vec![w, prev_close];
    r
}

/// `NATR(p)` = `(atr / close) * 100` wrapping the SAME mean-`Wilders(tr)`
/// node `atr` builds. Close arrives FIRST (stream 2 before 0/1) — the arena
/// order the family's shared chains were pinned with. Exact op order
/// `(atr / close) * 100.0` (hand `calc`).
pub fn natr(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hlc();
    let close = r.add(Node::Input { stream: 2 });
    r.tag(close, "close");
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let (atr_id, tr) = super::atr_nodes(&mut r, hi, lo, close, period);
    let ratio = r.add(Node::Div {
        a: atr_id,
        b: close,
    });
    r.tag(ratio, "atr_ratio");
    let hundred = r.constant(100.0);
    r.tag(hundred, "hundred");
    let natr = r.add(Node::Mul {
        a: ratio,
        b: hundred,
    });
    r.tag(natr, "natr");

    r.kind = "natr";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(natr, period));
    // Hand optional_outputs order: [atr, tr] — identical arena nodes to
    // standalone Atr/adx, so fully shared.
    r.optional_outputs.push(Sink::iface_lead(atr_id, period));
    r.optional_outputs.push(Sink::iface_lead(tr, 1));
    // `natr::State` is a transparent wrapper around `atr::State<Warm>` —
    // same tail shape as `atr`'s own recipe: the mean chain + prev_close.
    let prev_close = r.add(Node::Mirror { src: close });
    r.tag(prev_close, "prev_close");
    r.tail_inputs = vec![atr_id, prev_close];
    r
}

/// `SUPERTREND(p, step)` — the multi-value feedback band over THREE shared
/// nodes: the mean-`Wilders(tr)` atr (dedups with natr/atr), the
/// `0.5·(high+low)` medprice, and close. `st` is the bespoke op (4 carried
/// states mirroring the hand `State`); publish lead = `period`, the op
/// seeds one `calc_st` at the atr seam. Close FIRST (like natr's order).
pub fn supertrend(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let step = options[1];
    let mut r = Recipe::hlc();
    let close = r.add(Node::Input { stream: 2 });
    r.tag(close, "close");
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let (atr_id, tr) = super::atr_nodes(&mut r, hi, lo, close, period);
    // The medprice subgraph, inline (same nodes/add-order the grafted
    // `sub::<Medprice>` produced: sum, half, product).
    let sum = r.add(Node::Add { a: hi, b: lo });
    r.tag(sum, "hl_sum");
    let half = r.constant(0.5);
    r.tag(half, "half");
    let med = r.add(Node::Mul { a: half, b: sum });
    r.tag(med, "medprice");
    let st = r.add(Node::SuperTrend {
        atr: atr_id,
        med,
        close,
        period,
        step_bits: step.to_bits(),
    });
    r.tag(st, "supertrend");
    let prev_close = r.add(Node::Mirror { src: close });
    r.tag(prev_close, "prev_close");

    r.kind = "supertrend";
    r.options = options.to_vec();
    // hand State field order: atr_state (Wilders(tr) + prev_close mirror),
    // then the SuperTrend node's own 4-field feedback carry.
    r.tail_inputs = vec![atr_id, prev_close, st];
    r.outputs.push(Sink::iface_lead(st, period));
    // Hand optional_outputs order: [atr, tr, medprice].
    r.optional_outputs.push(Sink::iface_lead(atr_id, period));
    r.optional_outputs.push(Sink::iface_lead(tr, 1));
    r.optional_outputs.push(Sink::iface(med));
    r
}

pub fn psar(options: &[f64]) -> Recipe {
    let af_step = options[0];
    let max_af = options[1];
    let mut r = Recipe::hl();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let p = r.add(Node::Psar {
        high: hi,
        low: lo,
        af_step_bits: af_step.to_bits(),
        max_af_bits: max_af.to_bits(),
    });
    r.tag(p, "psar");

    r.kind = "psar";
    r.options = options.to_vec();
    // hand State field order: psar/extream/accel/af_step/max_af/uptrend --
    // the whole carry rides this one node's hidden [psar, extream, accel]
    // + trend bool, nothing else to capture.
    r.tail_inputs = vec![p];
    r.outputs.push(Sink::iface_lead(p, 1));
    r
}

/// `BBANDS(p, range)` — Bollinger Bands: middle SMA, stddev band width,
/// upper/middle/lower via FMA. Streams: 0=close. Options `[period, range]`.
pub fn bbands(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let range = options[1];
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let m = r.add(Node::Sma { src: close, period });
    r.tag(m, "middle");
    let sd = r.add(Node::Stddev {
        src: close,
        period,
        feed: None,
        switch: None,
    });
    r.tag(sd, "stddev");
    let k = r.constant(range);
    r.tag(k, "range");
    let n1 = r.constant(-1.0);
    r.tag(n1, "neg_one");
    let nk = r.add(Node::Mul { a: k, b: n1 });
    r.tag(nk, "neg_range");
    let up = r.add(Node::Fma { a: k, b: sd, c: m });
    r.tag(up, "upper");
    let lo = r.add(Node::Fma { a: nk, b: sd, c: m });
    r.tag(lo, "lower");
    r.kind = "bbands";
    r.options = options.to_vec();
    // `bbands::State<S>` is `{stddev_state, std_dev}` — `stddev_state` is the
    // `Stddev` node above (already `ChainState::Stddev{sum, sum_sq, period}`);
    // `std_dev` is a recipe CONSTANT (the `k` node), not carried state, same
    // as `supertrend`'s `step` — but unlike `supertrend` there is no bespoke
    // node whose own feedback struct already carries it, so it is recovered
    // the same way `ef`'s `prev`/`drop` are: a fresh `Node::Mirror` wrapping
    // the existing constant node, read back as `ChainState::Mirror{value}`.
    // No new `ChainState` variant needed.
    let std_dev_mirror = r.add(Node::Mirror { src: k });
    r.tag(std_dev_mirror, "std_dev_mirror");
    r.tail_inputs = vec![sd, std_dev_mirror];
    let lead = period;
    // hand `Info::outputs` order: lower, middle, upper.
    r.outputs.push(Sink::iface_lead(lo, lead));
    r.outputs.push(Sink::iface_lead(m, lead));
    r.outputs.push(Sink::iface_lead(up, lead));
    r
}

/// `DONCHIANCHANNEL(p)` — Donchian Channel: upper=RollMax(high,p), lower=RollMin(low,p),
/// middle=(upper+lower)/2. Streams: 0=high, 1=low. Options `[period]`.
pub fn donchianchannel(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let look_back = period - 1;
    let mut r = Recipe::hl();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
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
    let sum = r.add(Node::Add { a: mx, b: mn });
    r.tag(sum, "roll_mid_sum");
    let half = r.constant(0.5);
    r.tag(half, "half");
    let mid = r.add(Node::Mul { a: half, b: sum });
    r.tag(mid, "channel_mid");
    r.kind = "donchianchannel";
    r.options = options.to_vec();
    // hand `State` field order: min_state, max_state; outputs: lower,
    // middle, upper.
    r.tail_inputs = vec![mn, mx];
    r.outputs = vec![Sink::iface(mn), Sink::iface(mid), Sink::iface(mx)];
    r
}

/// `ICHIMOKU(s, m)` — six canonical rolls (short/medium/ultra × max/min)
/// over high and low plus the `0.5·(x+y)` combines, `l = 2m`. The densest
/// hub case: three windows per column, one arena, one pass. Narrow → wide
/// add-order (matches the interned arena); leads from `slot_lengths`
/// (conversion `s−1`, base `m−1`, both spans `2m−1` — the hand withholds
/// span A there on purpose). `lagging_span` is a verbatim close clone the
/// engine has no business copying — not declared (same fact as runtime).
pub fn ichimoku(options: &[f64]) -> Recipe {
    let short = options[0] as u32;
    let medium = options[1] as u32;
    let long = medium * 2;
    let mut r = Recipe::hl();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let s_min = r.add(Node::RollMin {
        src: low,
        period: short,
        look_back: short - 1,
    });
    r.tag(s_min, "roll_min_s");
    let s_max = r.add(Node::RollMax {
        src: high,
        period: short,
        look_back: short - 1,
    });
    r.tag(s_max, "roll_max_s");
    let m_min = r.add(Node::RollMin {
        src: low,
        period: medium,
        look_back: medium - 1,
    });
    r.tag(m_min, "roll_min_m");
    let m_max = r.add(Node::RollMax {
        src: high,
        period: medium,
        look_back: medium - 1,
    });
    r.tag(m_max, "roll_max_m");
    let l_min = r.add(Node::RollMin {
        src: low,
        period: long,
        look_back: long - 1,
    });
    r.tag(l_min, "roll_min_l");
    let l_max = r.add(Node::RollMax {
        src: high,
        period: long,
        look_back: long - 1,
    });
    r.tag(l_max, "roll_max_l");

    let half = r.constant(0.5);
    r.tag(half, "half");
    let conv_sum = r.add(Node::Add { a: s_min, b: s_max });
    let conversion = r.add(Node::Mul {
        a: half,
        b: conv_sum,
    });
    r.tag(conversion, "conversion");
    let base_sum = r.add(Node::Add { a: m_min, b: m_max });
    let base = r.add(Node::Mul {
        a: half,
        b: base_sum,
    });
    r.tag(base, "base_line");
    let span_a_sum = r.add(Node::Add {
        a: conversion,
        b: base,
    });
    let span_a = r.add(Node::Mul {
        a: half,
        b: span_a_sum,
    });
    r.tag(span_a, "span_a");
    let span_b_sum = r.add(Node::Add { a: l_min, b: l_max });
    let span_b = r.add(Node::Mul {
        a: half,
        b: span_b_sum,
    });
    r.tag(span_b, "span_b");

    r.kind = "ichimoku";
    r.options = options.to_vec();
    // hand `State` field order (short/min,max · medium · long).
    r.tail_inputs = vec![s_min, s_max, m_min, m_max, l_min, l_max];
    r.outputs = vec![
        Sink::iface_lead(conversion, short - 1),
        Sink::iface_lead(base, medium - 1),
        Sink::iface_lead(span_a, long - 1),
        Sink::iface_lead(span_b, long - 1),
    ];
    r
}

/// `KELTNERCHANNEL(p, step)` — Keltner Channel: ATR-based band around EMA(close,p).
/// Streams: 0=high, 1=low, 2=close. Options `[period, step]`.
pub fn keltnerchannel(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hlc();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let cl = r.add(Node::Input { stream: 2 });
    r.tag(cl, "close");

    let (atr_id, tr) = super::atr_nodes(&mut r, hi, lo, cl, period);
    let ema = r.add(Node::Ema { src: cl, period });
    r.tag(ema, "middle");
    let step = r.constant(options[1]);
    r.tag(step, "step");
    let per = r.add(Node::Mul { a: atr_id, b: step });
    r.tag(per, "band_half");
    let lower = r.add(Node::Sub { a: ema, b: per });
    r.tag(lower, "lower");
    let upper = r.add(Node::Add { a: ema, b: per });
    r.tag(upper, "upper");

    r.kind = "keltnerchannel";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(lower, period));
    r.outputs.push(Sink::iface_lead(ema, period));
    r.outputs.push(Sink::iface_lead(upper, period));
    // Info optional order: [atr, tr] -- both the SAME arena nodes `atr` publishes.
    r.optional_outputs.push(Sink::iface_lead(atr_id, period));
    r.optional_outputs.push(Sink::iface_lead(tr, 1));
    r
}

/// `CHANDLERIEREXIT(p, step)` — Chandelier Exit: ATR-based trailing stop. Long=RollMax(high,p) - atr*step,
/// short=RollMin(low,p) + atr*step. Streams: 0=high, 1=low, 2=close. Options `[period, step]`.
pub fn chandelierexit(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let look_back = period - 1;
    let step = options[1];
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
    let (atr, tr) = super::atr_nodes(&mut r, high, low, close, period);
    let neg_step = r.constant(-step);
    r.tag(neg_step, "neg_step");
    let pos_step = r.constant(step);
    r.tag(pos_step, "step");
    let long = r.add(Node::Fma {
        a: atr,
        b: neg_step,
        c: mx,
    });
    r.tag(long, "long");
    let short = r.add(Node::Fma {
        a: atr,
        b: pos_step,
        c: mn,
    });
    r.tag(short, "short");
    r.kind = "chandelierexit";
    r.options = options.to_vec();
    // `chandelierexit::State<S>` is `{atr_state, min_state, max_state, step}`.
    // `atr_state` needs `atr()`'s own 2-entry `[Wilders, Mirror{src: close}]`
    // shape (not built by `atr_nodes()`, which only returns the Wilders/tr
    // ids), so a fresh `Mirror{src: close}` is added here, same as `atr()`'s
    // own recipe. `min_state`/`max_state` are the `mn`/`mx` roll nodes above
    // (already `ChainState::RollMin`/`RollMax`). `step` is a recipe CONSTANT
    // (the `pos_step` node), recovered the same Mirror-of-Const way `bbands`'
    // `std_dev` is — no new `ChainState` variant needed.
    let prev_close = r.add(Node::Mirror { src: close });
    r.tag(prev_close, "prev_close_mirror");
    let step_mirror = r.add(Node::Mirror { src: pos_step });
    r.tag(step_mirror, "step_mirror");
    r.tail_inputs = vec![atr, prev_close, mn, mx, step_mirror];
    // `long`/`short`'s own node-natural lead is `period-1` (atr's Wilders is
    // `feed:false`, so its lead is `period-1` structurally), but the hand
    // publishes starting at `periods.0 = period` -- ONE bar later, the same
    // +1 `atr()`'s own recipe applies to its `w` sink. Match it explicitly.
    r.outputs.push(Sink::iface_lead(long, period));
    r.outputs.push(Sink::iface_lead(short, period));
    // Hand `Info::optional_outputs = ["atr", "tr", "min", "max"]` -- atr/tr take the SAME
    // leads `atr()` publishes them at; min/max take their own node lead (`period-1`), same
    // as `willr`'s `Sink::iface(mn)`/`Sink::iface(mx)` pair.
    r.optional_outputs.push(Sink::iface_lead(atr, period));
    r.optional_outputs.push(Sink::iface_lead(tr, 1));
    r.optional_outputs.push(Sink::iface(mn));
    r.optional_outputs.push(Sink::iface(mx));
    r
}

/// `SMAENVELOPE(p, b)` — hand `smaenvelope.rs`: the SMA line plus `sma * (b/100)` added
/// to and subtracted from it. `Info.outputs = ["lower","middle","upper"]` with NO
/// optional outputs, so all three are PRIMARIES, in that order.
///
/// The middle band is the SAME `Node::Sma{src: Input{0}, period}` that [`super::sma`] builds over
/// the same close column, so `Sma(p) + SmaEnvelope(p, b)` interns to ONE roll — the arena
/// carries six nodes, not seven. `intern` keys on the whole `Node` (merge.rs:576), so the
/// two rolls merge only because the source, period and kind are all identical; and the
/// envelope's `middle` row is then literally Sma's primary row, not a copy.
///
/// The band scale is a `Const`, not a parameter: the hand folds `percentage / 100.0` into
/// the STATE at `State::new`/`init_state` and then multiplies, so the recipe carries
/// `constant(options[1] / 100.0)` — one division at plan time, then a plain `Mul` per bar,
/// matching `let step = sma * self.percentage`. `Const{bits}` is part of the intern key but
/// its VALUE is not in the Signature (`(K_CONST, 0, 0, 0)`, signature.rs:351-353; the value
/// rides `vals.consts[k]`), which is what lets two different band widths share a kernel
/// while still occupying distinct arena slots.
///
/// Lead = `period` on ALL THREE rows. `Node::Sma`'s node lead is `period - 1`
/// (`merge.rs:79-81`, "the first full window ends at lead(src) + period − 1"), but the hand
/// publishes from bar `period` (`min_data = options[0] + 1`, default `output_length`, and
/// `cycle` starts at `period..len`). So each sink spells `Sink::iface_lead(_, period)` —
/// the same explicit `+1` [`super::sma`] needs for the identical reason.
pub fn smaenvelope(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "price");
    let m = r.add(Node::Sma { src: real, period });
    r.tag(m, "middle");
    let pct = r.constant(options[1] / 100.0);
    r.tag(pct, "band_pct");
    let step = r.add(Node::Mul { a: m, b: pct });
    r.tag(step, "step");
    let lower = r.add(Node::Sub { a: m, b: step });
    r.tag(lower, "lower");
    let upper = r.add(Node::Add { a: m, b: step });
    r.tag(upper, "upper");

    r.kind = "smaenvelope";
    r.options = options.to_vec();
    r.tail_inputs = vec![m];
    // Info order: lower, middle, upper.
    r.outputs.push(Sink::iface_lead(lower, period));
    r.outputs.push(Sink::iface_lead(m, period));
    r.outputs.push(Sink::iface_lead(upper, period));
    r
}
