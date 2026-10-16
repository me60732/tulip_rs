//! The recipe GRAPHS — Phase 2 of "recipes as data": the node graphs the
//! indicators describe, as plain leaf functions.
//!
//! One source of truth per graph: the runtime `Planable` impls in the
//! indicator modules are one-line forwarders here, and the `kernel!`
//! generator calls `merge()` over these SAME functions at expansion — the
//! macro-side transcribed planner mirror is deleted, so a recipe edit can
//! no longer drift into a wrong `Signature` (the drift class the old
//! `*_sig_derived` tests existed to catch).
//!
//! Params arrive as plain `f64`s in `options` exactly like `Planable::recipe`
//! — at macro expansion the `kernel!` pattern names become distinct
//! SENTINEL values (dedup by identity, never baked numbers; runtime periods
//! bind through `Values`). The role tags (`r.tag(…)`) are display-only
//! locals for the emitter: the first recipe to declare a node names it, as
//! the planner's arms used to.
//!
//! Add-order is load-bearing: arena slots follow it, so these spellings
//! reproduce the grafted `sub_at` orders the merged arena was pinned with
//! (`composing indicators add their source nodes in the order the old
//! `Recipe::sub`/`sub_at`/`sub_graph` calls produced them — helpers below
//! inline those grafts without reordering any node).

use crate::cse::{Id, Node, Recipe, Sink};

/// `Ema(p)` over `Input{0}`: one chain node mirroring its own line.
pub fn ema(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let e = r.add(Node::Ema { src: close, period });
    r.kind = "ema";
    r.options = options.to_vec();
    r.tail_inputs = vec![e];
    // Publish lead = `period`: hand `min_data = p + 1`, `output_length =
    // len − p` — report starts one bar after the node's defined-from (the
    // seed bar is not published).
    r.outputs.push(Sink::iface_lead(e, period));
    r
}

/// `Apo(s, l) ≡ ema_s(close) − ema_l(close)`: the two EMA chains plus the
/// line `Sub` — the §4 identity node shared (by CSE key) with MACD's line
/// and PPO's numerator. SSA order matches `TState::calc` (short before long).
pub fn apo(options: &[f64]) -> Recipe {
    let short = options[0] as u32;
    let long = options[1] as u32;
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let s = r.add(Node::Ema {
        src: close,
        period: short,
    });
    r.tag(s, "ema_short");
    let l = r.add(Node::Ema {
        src: close,
        period: long,
    });
    r.tag(l, "ema_long");
    let line = r.add(Node::Sub { a: s, b: l });
    r.tag(line, "line");

    r.kind = "apo";
    r.options = options.to_vec();
    r.tail_inputs = vec![s, l];
    r.outputs.push(Sink::iface_lead(line, long - 1));
    // optional_outputs in Info order: short_ema at `Ema::output_length`
    // capacity (lead = period), long reuses the primary capacity
    // (lead = long−1 = defined-from).
    r.optional_outputs.push(Sink::iface_lead(s, short));
    r.optional_outputs.push(Sink::iface(l));
    r
}

/// `Ppo(s, l) ≡ (ema_s − ema_l) * 100.0 / ema_l`, in the hand's exact
/// evaluation order `((s − l) * 100.0) / l` — reassociating would change the
/// bits. The `Sub` numerator is the SAME key apo/macd declare (one `fsub`).
pub fn ppo(options: &[f64]) -> Recipe {
    let short = options[0] as u32;
    let long = options[1] as u32;
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let s = r.add(Node::Ema {
        src: close,
        period: short,
    });
    let l = r.add(Node::Ema {
        src: close,
        period: long,
    });
    let line = r.add(Node::Sub { a: s, b: l });
    let c100 = r.constant(100.0);
    r.tag(c100, "hundred");
    let num = r.add(Node::Mul { a: line, b: c100 });
    r.tag(num, "scaled");
    let ratio = r.add(Node::Div { a: num, b: l });
    r.tag(ratio, "ppo");

    r.kind = "ppo";
    r.options = options.to_vec();
    r.tail_inputs = vec![s, l];
    // publish lead = min_data − 1 = options[1] (output_length = data_len −
    // min_data + 1, right-aligned); the ratio node is DEFINED at long-1 but
    // the hand capacity starts it one bar later.
    r.outputs.push(Sink::iface_lead(ratio, long));
    r.optional_outputs.push(Sink::iface_lead(s, short));
    r.optional_outputs.push(Sink::iface_lead(l, long));
    r
}

/// `Macd(s, l, g)`: the apo line (`Sub` of the close EMAs — shared key), an
/// `Ema` of THAT node as the signal (chained recurrence: fed the line's
/// reported bars), and the hist `Sub`. SSA order matches `TState::calc`.
pub fn macd(options: &[f64]) -> Recipe {
    let short = options[0] as u32;
    let long = options[1] as u32;
    let signal_period = options[2] as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "price");
    let s = r.add(Node::Ema {
        src: real,
        period: short,
    });
    let l = r.add(Node::Ema {
        src: real,
        period: long,
    });
    let line = r.add(Node::Sub { a: s, b: l });
    let sig = r.add(Node::Ema {
        src: line,
        period: signal_period,
    });
    r.tag(sig, "signal");
    let hist = r.add(Node::Sub { a: line, b: sig });
    r.tag(hist, "histogram");

    r.kind = "macd";
    r.options = options.to_vec();
    // The hand State's three chains, in TState field order.
    r.tail_inputs = vec![s, l, sig];
    // outputs in Info order: line, signal, histogram — publish leads = hand
    // slot_lengths offsets (25 / 33 / 33 for 12/26/9).
    r.outputs.push(Sink::iface_lead(line, long - 1));
    r.outputs
        .push(Sink::iface_lead(sig, long + signal_period - 2));
    r.outputs
        .push(Sink::iface_lead(hist, long + signal_period - 2));
    r.optional_outputs.push(Sink::iface_lead(s, short));
    r.optional_outputs.push(Sink::iface_lead(l, long));
    r
}

/// `WILDERS(p)` over `Input{0}` — one decayed-average node. Warm = fold the
/// first `p` into a mean seed; steady = `w·(p−1)/p + v·(1/p)`. Publish lead
/// = `period` (hand reports `len − p` rows, `min_data = p+1`).
pub fn wilders(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::new();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let w = r.add(Node::Wilders {
        src: close,
        period,
        composite: false,
        feed: false,
    });
    r.kind = "wilders";
    r.options = options.to_vec();
    r.tail_inputs = vec![w];
    r.outputs.push(Sink::iface_lead(w, period));
    r
}

/// The true-range subgraph over given (high, low, close) node ids:
/// `max(h−l, |h−pc|, |l−pc|)` with `pc = Lag(close, 1)` — the `Lag` clamp
/// makes bar 0 read `high[0] − low[0]` (hand `init_state`'s bar-0 term).
/// Node order is the canonical chain the ATR family has always interned.
fn tr_nodes(r: &mut Recipe, hi: Id, lo: Id, cl: Id) -> Id {
    let pc = r.add(Node::Lag { src: cl, n: 1 });
    r.tag(pc, "prev_close");
    let range = r.add(Node::Sub { a: hi, b: lo });
    r.tag(range, "hl_range");
    let h_pc = r.add(Node::Sub { a: hi, b: pc });
    r.tag(h_pc, "h_pc");
    let hc = r.add(Node::Abs { src: h_pc });
    r.tag(hc, "h_pc_abs");
    let l_pc = r.add(Node::Sub { a: lo, b: pc });
    r.tag(l_pc, "l_pc");
    let lc = r.add(Node::Abs { src: l_pc });
    r.tag(lc, "l_pc_abs");
    let m1 = r.add(Node::Max { a: range, b: hc });
    r.tag(m1, "max_hl_gap");
    let tr = r.add(Node::Max { a: m1, b: lc });
    r.tag(tr, "tr");
    tr
}

/// The mean-`Wilders(tr)` ATR chain: the tr subgraph plus one smoothing
/// node. Returns `(atr, tr)` — every consumer of "the ATR" means THIS
/// spelling (natr, supertrend, adx compose it identically, so CSE dedups
/// the whole chain across the family).
fn atr_nodes(r: &mut Recipe, hi: Id, lo: Id, cl: Id, period: u32) -> (Id, Id) {
    let tr = tr_nodes(r, hi, lo, cl);
    let w = r.add(Node::Wilders {
        src: tr,
        period,
        composite: false,
        feed: false,
    });
    r.tag(w, "atr");
    (w, tr)
}

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
    let (w, tr) = atr_nodes(&mut r, hi, lo, cl, period);
    r.kind = "atr";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(w, period));
    // Hand `Info::optional_outputs = ["tr"]` — the exposed tr node is where
    // every family member's `tr` mirror comes from, never a re-derived copy.
    r.optional_outputs.push(Sink::iface_lead(tr, 1));
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
    let (atr_id, tr) = atr_nodes(&mut r, hi, lo, close, period);
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
    let (atr_id, tr) = atr_nodes(&mut r, hi, lo, close, period);
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

    r.kind = "supertrend";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(st, period));
    // Hand optional_outputs order: [atr, tr, medprice].
    r.optional_outputs.push(Sink::iface_lead(atr_id, period));
    r.optional_outputs.push(Sink::iface_lead(tr, 1));
    r.optional_outputs.push(Sink::iface(med));
    r
}

/// `MEDPRICE = 0.5 · (high + low)` — stateless, every bar publishes
/// (`Sink::iface`, lead 0). Streams: 0 = high, 1 = low.
pub fn medprice(_options: &[f64]) -> Recipe {
    let mut r = Recipe::new();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let sum = r.add(Node::Add { a: hi, b: lo });
    r.tag(sum, "hl_sum");
    let half = r.constant(0.5);
    r.tag(half, "half");
    let med = r.add(Node::Mul { a: half, b: sum });
    r.tag(med, "medprice");
    r.kind = "medprice";
    r.options = Vec::new();
    r.outputs.push(Sink::iface(med));
    r
}

/// `AO(s, l)` = `mean RollSum(medprice, s) − mean RollSum(medprice, l)` —
/// the ring-backed pair over the ONE shared `medprice` node, spelled with
/// the same `Mul(half, Add)` shape as [`medprice`] so interning collapses
/// them (one computed medprice column per bar, one ring group of capacity
/// `max(s, l)`; leave = eviction at the cap, age read below it). Streams:
/// 0 = high, 1 = low; options `[short, long]` default to the hand module's
/// consts `(5, 34)`. Publish lead = `long` EXPLICIT — the hand's LONG row
/// begins one bar after the long roll's own defined-from (bar `long−1`)
/// because its `init_state` never emits the fold-completion mean; clamping
/// up there is exactly `leads`+explicit contract.
pub fn ao(options: &[f64]) -> Recipe {
    let short = options.first().copied().unwrap_or(5.0) as u32;
    let long = options.get(1).copied().unwrap_or(34.0) as u32;
    let mut r = Recipe::new();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let sum = r.add(Node::Add { a: hi, b: lo });
    r.tag(sum, "hl_sum");
    let half = r.constant(0.5);
    r.tag(half, "half");
    let med = r.add(Node::Mul { a: half, b: sum });
    r.tag(med, "medprice");
    let s_short = r.add(Node::RollSum {
        src: med,
        period: short,
        mean: true,
        skip: 0,
        steady_from: 0,
    });
    r.tag(s_short, "sma_short");
    let s_long = r.add(Node::RollSum {
        src: med,
        period: long,
        mean: true,
        skip: 0,
        steady_from: 0,
    });
    r.tag(s_long, "sma_long");
    let ao = r.add(Node::Sub {
        a: s_short,
        b: s_long,
    });
    r.tag(ao, "ao");
    r.kind = "ao";
    r.options = vec![short as f64, long as f64];
    r.outputs.push(Sink::iface_lead(ao, long));
    // Hand `Info::optional_outputs = ["short_sma", "long_sma", "medprice"]`
    // — declared order, leads = each node's defined-from. The `medprice`
    // mirror is the seam that lets AO fuse with supertrend/fisher: all read
    // the SAME interned node.
    r.optional_outputs.push(Sink::iface(s_short));
    r.optional_outputs.push(Sink::iface(s_long));
    r.optional_outputs.push(Sink::iface(med));
    r
}

/// `HMA(p)` — ONE bespoke rolling node (E3 stage 3): the two inner WMA
/// engines (p and p/2 with the hand's OFFSET init window), the diff series
/// and its √p ring all live inside the op. It is NOT a recipe of generic
/// Wmas — composition is not bit-exact (state2's fold weights restart at
/// the offset window; the tail divides `wss/ww` BEFORE its subtractions).
/// Publish lead `p + trunc(sqrt p)` = hand `init_state`'s returned start.
pub fn hma(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let psqrt = (period as f64).sqrt() as u32;
    let mut r = Recipe::new();
    let close = r.add(Node::Input { stream: 0 });
    let h = r.add(Node::Hma { src: close, period });
    r.kind = "hma";
    r.options = options.to_vec();
    r.tail_inputs = vec![h];
    r.outputs.push(Sink::iface_lead(h, period + psqrt));
    r
}

/// `FISHER(period)` — the fisher transform over the DERIVED medprice
/// column, mirroring `fisher.rs::calc_fisher` op for op: `med = 0.5·(high+low)`
/// (the SAME interned node AO/supertrend read — co-requesting fisher with AO
/// adds zero arithmetic nodes, pinned by the merge canary); a
/// `(period, period−1)` min/max pair over that derived column (the hand's
/// `MirrorBuffer` + min/max states — an extremum ring, the fourth ring
/// shape); `inner = (med−min)/max(max−min, 1e−3) − 0.5`; `val1` with the
/// hand's if/elif clamp folded INTO the carried value (`IirClamped`,
/// 0.66·inner + 0.67·prev); `fish = 0.5·(ln((1+val1)/(1−val1)) + prev)`
/// (`Iir`); `signal = prev fish` (`Prev`). Parity law: 1e-12 RELATIVE vs
/// the hand (`Node::Ln` is a host libm call — Handover §6.41).
pub fn fisher(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::new();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let sum = r.add(Node::Add { a: hi, b: lo });
    r.tag(sum, "hl_sum");
    let half = r.constant(0.5);
    let med = r.add(Node::Mul { a: half, b: sum });
    r.tag(med, "medprice");
    let mn = r.add(Node::RollMin {
        src: med,
        period,
        look_back: period - 1,
    });
    r.tag(mn, "med_min");
    let mx = r.add(Node::RollMax {
        src: med,
        period,
        look_back: period - 1,
    });
    r.tag(mx, "med_max");
    let eps = r.constant(0.001);
    let range = r.add(Node::Sub { a: mx, b: mn });
    let mm = r.add(Node::Max { a: range, b: eps });
    let diff = r.add(Node::Sub { a: med, b: mn });
    let ratio = r.add(Node::Div { a: diff, b: mm });
    let inner = r.add(Node::Sub { a: ratio, b: half });
    // clamp thresholds/values as Const children, BEFORE the node that uses
    // them (their lead 0 keeps the arena order trivial).
    let hi_t = r.constant(0.99);
    let hi_v = r.constant(0.999);
    let lo_t = r.constant(-0.99);
    let lo_v = r.constant(-0.999);
    let val1 = r.add(Node::IirClamped {
        src: inner,
        a_bits: 0.66f64.to_bits(),
        b_bits: 0.67f64.to_bits(),
        hi_t,
        hi_v,
        lo_t,
        lo_v,
    });
    r.tag(val1, "val1");
    let one = r.constant(1.0);
    let numer = r.add(Node::Add { a: one, b: val1 });
    let denom = r.add(Node::Sub { a: one, b: val1 });
    let ln_arg = r.add(Node::Div { a: numer, b: denom });
    let ln = r.add(Node::Ln { src: ln_arg });
    r.tag(ln, "ln");
    let fish = r.add(Node::Iir {
        src: ln,
        a_bits: 0.5f64.to_bits(),
        b_bits: 0.5f64.to_bits(),
    });
    r.tag(fish, "fisher");
    let signal = r.add(Node::Prev { src: fish });
    r.tag(signal, "fisher_signal");
    r.kind = "fisher";
    r.options = vec![period as f64];
    r.outputs.push(Sink::iface(fish));
    r.outputs.push(Sink::iface(signal));
    r
}

/// `STOCH` (slow stochastic) as a standalone request: three price streams,
/// the fast-%K division over the shared `(k, k−1)` high/low rolls, slow %K as
/// the mean of fast with `skip: 2`, and %D as the mean of the slow line. The
/// denominator is `Max(dif, ε)` — the hand FLOORS at epsilon (a flat bar
/// prints a huge fast %K; the engine must not "improve" that), the ε-guard
/// and left-assoc `Div(Mul(100, sub), den)` transcription are bit-load-bearing.
pub fn stoch(options: &[f64]) -> Recipe {
    let k_period = options[0] as u32;
    let k_slow = options[1] as u32;
    let d_period = options[2] as u32;
    let look_back = k_period.saturating_sub(1);

    let mut r = Recipe::new();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let close = r.add(Node::Input { stream: 2 });
    r.tag(close, "close");
    let mx = r.add(Node::RollMax {
        src: high,
        period: k_period,
        look_back,
    });
    let mn = r.add(Node::RollMin {
        src: low,
        period: k_period,
        look_back,
    });
    let dif = r.add(Node::Sub { a: mx, b: mn });
    let eps = r.constant(f64::EPSILON);
    let den = r.add(Node::Max { a: dif, b: eps });
    let mc = r.add(Node::Sub { a: close, b: mn });
    let c100 = r.constant(100.0);
    let num = r.add(Node::Mul { a: c100, b: mc });
    let kfast = r.add(Node::Div { a: num, b: den });
    // Slow %K: the hand discards fast %K's first two published values
    // (its roll seeding), hence `skip: 2`.
    let k_line = r.add(Node::RollSum {
        src: kfast,
        period: k_slow,
        mean: true,
        skip: 2,
        // The hand's init loop runs to bar `k + k_slow + d − 1`, past the
        // first full window, and inside it the eviction subtracts as its own
        // statement — so the steady `sum += v - old` only begins at
        // `k + k_slow + d`, exactly where %D starts printing.
        steady_from: k_period + k_slow + d_period,
    });
    // %D: a mean of the slow %K line, folding from that line's first
    // published bar — the hand's `prev_d` receives exactly those, and its
    // init loop never evicts, so the steady form applies throughout.
    let d_line = r.add(Node::RollSum {
        src: k_line,
        period: d_period,
        mean: true,
        skip: 0,
        steady_from: 0,
    });
    r.kind = "stoch";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface(k_line));
    r.outputs
        .push(Sink::iface_lead(d_line, k_period + k_slow + d_period));
    r
}

/// `RSI(p)` as a standalone request: the movement `diff = real − Lag{real,1}`
/// split through two `Max` gates into the hand's `up_down` arms, each fed to a
/// mean-`Wilders` chain (seed = mean of the first `p` gated values, then the
/// Wilders recurrence), and `line = 100·up_avg/(up_avg+down_avg)`. The second
/// arm is `Max(Lag − real, 0)` — the antisymmetric difference, exact to the
/// bit by IEEE — NOT the one-guard rewrite `up − diff`: on `diff = +∞` that
/// rewrite returns `NaN` where the hand writes `+0.0`, and a NaN entering a
/// Wilders chain poisons every later bar (`up_down_rewrite_proofs` swept
/// ~7,000 movement classes and found it). Publish lead `period + 1` (hand
/// `min_data = period + 2`); the state mirrors (`prev_real`) ride
/// `tail_inputs`, outside the sink graph.
pub fn rsi(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::new();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "price");
    let prev = r.add(Node::Lag { src: real, n: 1 });
    let diff = r.add(Node::Sub { a: real, b: prev });
    let zero = r.constant(0.0);
    let up = r.add(Node::Max { a: diff, b: zero });
    r.tag(up, "up");
    let neg = r.add(Node::Sub { a: prev, b: real });
    let down = r.add(Node::Max { a: neg, b: zero });
    r.tag(down, "down");
    let up_avg = r.add(Node::Wilders {
        src: up,
        period,
        composite: false,
        feed: true,
    });
    r.tag(up_avg, "up_avg");
    let down_avg = r.add(Node::Wilders {
        src: down,
        period,
        composite: false,
        feed: true,
    });
    r.tag(down_avg, "down_avg");
    let total = r.add(Node::Add {
        a: up_avg,
        b: down_avg,
    });
    let ratio = r.add(Node::Div {
        a: up_avg,
        b: total,
    });
    r.tag(ratio, "ratio");
    let hundred = r.constant(100.0);
    let line = r.add(Node::Mul {
        a: hundred,
        b: ratio,
    });
    r.tag(line, "rsi");
    // State mirror: the hand keeps the last processed close so the next
    // `calc` can difference against it. Identity copy, feeds NO sink — the
    // emitter's `Kind::Mirror` arm computes it dead in the steady loop and
    // `get_states` (M1) is the real consumer. Verbatim from the runtime
    // recipe: dropping it would break the `[Wilders, Wilders, Mirror]`
    // state-layout contract in `restore_state`.
    let prev_real = r.add(Node::Mirror { src: real });
    r.kind = "rsi";
    r.options = options.to_vec();
    // hand `State` field order: the up lane, the down lane, `prev_real`
    r.tail_inputs = vec![up_avg, down_avg, prev_real];
    r.outputs.push(Sink::iface_lead(line, period + 1));
    r
}

/// `STOCHRIS(p)` = `100·(rsi − min) / (max − min)` over the last `p` RSI
/// values, with the RSI line itself an optional output — ONE option driving
/// both the Wilders smoothing and the stochastic window, exactly as the hand
/// wires it. The RSI half is the `rsi` leaf graph GRAFTED (the runtime
/// recipe's `sub_at::<Rsi>` — same `Recipe::graft` call, so a co-requested
/// `Rsi(p)` interning the whole movement chain, both Wilders chains, the
/// division and the ×100 is the same stored stream as this recipe's optional
/// rsi row. The rolls are `(p, p−1)` over a DERIVED source — the extremum
/// ring machinery (proven on fisher's medprice rings).
pub fn stochrsi(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::new();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "price");
    // Exactly what the runtime recipe's `sub_at::<Rsi>(&[period], &[real])`
    // does — graft the leaf rsi graph with stream 0 remapped onto `real`;
    // `.primary` is the grafted rsi `line` node. The grafted `prev_real`
    // mirror rides the arena dead (state channel, feeds no sink here).
    let g = r.graft(rsi(options), &[real]);
    let rsi = g.primary;
    let mx = r.add(Node::RollMax {
        src: rsi,
        period,
        look_back: period.saturating_sub(1),
    });
    let mn = r.add(Node::RollMin {
        src: rsi,
        period,
        look_back: period.saturating_sub(1),
    });
    let kdif = r.add(Node::Sub { a: mx, b: mn });
    let sub = r.add(Node::Sub { a: rsi, b: mn });
    let c100 = r.constant(100.0);
    let num = r.add(Node::Mul { a: c100, b: sub });
    let q = r.add(Node::Div { a: num, b: kdif });
    let eps = r.constant(f64::EPSILON);
    // `if kdif < ε { 0.0 } else { q }` is `WhereLt` directly — the `<`-form
    // mirror of `WhereGe`, which is what the hand actually writes. The reason
    // it must be `<` and not `>=`: on a FLAT series the RSI line is exactly
    // `0/0 = NaN`, so `kdif = max − min` is NaN, and an IEEE compare is false in
    // BOTH directions there. The hand's `<` test therefore falls to the ELSE arm
    // (the division) and keeps the NaN, which is what it prints; `WhereGe`'s
    // false branch is the pinned `+0.0` and swallows it (the parity test failed
    // on `stochrsi flat p3` until this node replaced a `0 − maxNum(0 − kdif, −∞)`
    // mapping that cost three extra ops to buy the same NaN propagation).
    let line = r.add(Node::WhereLt {
        a: kdif,
        b: eps,
        t: q,
    });
    r.tag(line, "stochrsi");
    r.kind = "stochrsi";
    r.options = options.to_vec();
    // leads: the rsi node exists from bar `p + 1`, the roll's window is
    // complete at `2p − 1`, and the hand PRINTS from `2p` (`min_data =
    // 2p + 1`), hence the explicit primary lead; the optional rsi row keeps
    // rsi's own `p + 1` lead, matching `Rsi::output_length`.
    r.outputs.push(Sink::iface_lead(line, period * 2));
    r.optional_outputs = vec![Sink::iface_lead(rsi, period + 1)];
    r
}

/// `CMO(p)` = `100·(Σup − Σdown) / (Σup + Σdown)` over the movement of the
/// last `p` bars — the first consumer of `RollSum`, a rolling sum over a
/// DERIVED source: what it accumulates is `up`/`down`, not a price, so the
/// leaving term comes from a ring of that series' own recent values (two
/// ring groups, each single-member cap `p`). Hand facts encoded here, read
/// against `cycle_cmo(&real[1..], …, period, …)`:
/// · `up_down(v, prev) = [diff.max(0), (-diff).max(0)]` with `diff = v − prev`
///   and `prev = Lag{real, 1}` — the SAME nodes `rsi` builds (the negative
///   arm written `Max(Lag − real, 0)`, exact by IEEE's antisymmetric
///   subtraction), so an `rsi`+`cmo` query computes the movement ONCE; only
///   the aggregation differs: RSI Wilder-smooths these, CMO rolls them up.
/// · `init_state` folds bars `1..=p` (its `skip(1)`), so a full window
///   exists at bar `p`. `RollSum` folds from bar 0 instead, and bar 0's
///   movement is `[0.0, 0.0]` through the `Lag` clamp — adding a zero
///   changes neither value nor rounding, so the two agree bit-for-bit.
/// · `min_data = p + 2` while the first PRINTED bar is `p + 1`, hence the
///   explicit sink lead. The state carries `prev` (last close) and
///   `drop_real` (the close `p` bars back) as Mirrors — copies riding the
///   `Input` and the `Lag{real, period}` nodes, so reporting them adds no
///   arithmetic.
pub fn cmo(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::new();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "price");
    let l1 = r.add(Node::Lag { src: real, n: 1 });
    let diff = r.add(Node::Sub { a: real, b: l1 });
    let zero = r.constant(0.0);
    let up = r.add(Node::Max { a: diff, b: zero });
    r.tag(up, "up");
    let neg = r.add(Node::Sub { a: l1, b: real });
    let down = r.add(Node::Max { a: neg, b: zero });
    r.tag(down, "down");
    let up_sum = r.add(Node::RollSum {
        src: up,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(up_sum, "up_sum");
    let down_sum = r.add(Node::RollSum {
        src: down,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(down_sum, "down_sum");
    // `100.0 * (up_sum - down_sum) / (up_sum + down_sum)` parses left to
    // right, so the DIVIDE's numerator is the already-scaled difference.
    let hundred = r.constant(100.0);
    let num = r.add(Node::Sub {
        a: up_sum,
        b: down_sum,
    });
    let scaled = r.add(Node::Mul { a: hundred, b: num });
    let den = r.add(Node::Add {
        a: up_sum,
        b: down_sum,
    });
    let line = r.add(Node::Div { a: scaled, b: den });
    r.tag(line, "cmo");
    let lp = r.add(Node::Lag {
        src: real,
        n: period,
    });
    let drop_real = r.add(Node::Mirror { src: lp });
    let prev = r.add(Node::Mirror { src: real });
    r.kind = "cmo";
    r.options = options.to_vec();
    // hand `State` field order: up_sum, down_sum, prev, drop_real.
    r.tail_inputs = vec![up_sum, down_sum, prev, drop_real];
    r.outputs.push(Sink::iface_lead(line, period + 1));
    r
}

/// `ULTOSC(short, medium, long)` — Ultimate Oscillator: the weighted ratio of
/// three-window buying-pressure and true-range sums.
///
/// The arena mirrors `ultosc.rs` statement for statement. `true_low =
/// Min(low, prev)`, `true_high = Max(high, prev)`, `bp = close − true_low`,
/// `tr = true_high − true_low`, then SIX raw sums (mean: false — the hand
/// divides sums by sums, never scales a window) and the tail
/// `(4·bps/trs + 2·bps_m/trs_m + bps_l/trs_l) · (100.0/7.0)`, parenthesised in
/// that order because association is visible.
///
/// **Why `Lag` and not `Prev`:** the hand seeds `prev_close = close[0]`
/// (`State::new(long_period, close[0])`) and its warm-up loop is
/// `.skip(1).take(long_period)`, so bar 0 is never computed and never pushed.
/// `Node::Lag`'s clamp rule reads sample 0 for `t < n` — the same value the hand
/// holds at bar 0 — and it gives `prev` LEAD 1, so `bp`/`tr` inherit lead 1 and
/// the ring first feeds at bar 1. That propagation is why `skip` stays 0: the
/// feed start comes from the structure, not from a knob.
///
/// **The container this shape is the test for:** `bp` and `tr` are two
/// independent pure-sum groups of THREE members each (short, medium, long), all
/// three windows read through one capacity (`long`), and the long member's leave
/// is the eviction at `p == cap`. `lowering::bundles()` fuses the pair into ONE
/// `MultiBuffer<2>` — which is precisely the hand's own `Buffer<2, f64, S>` — so
/// one push and one cursor advance per bar serve all six folds.
pub fn ultosc(options: &[f64]) -> Recipe {
    let (short, medium, long) = (options[0] as u32, options[1] as u32, options[2] as u32);
    let mut r = Recipe::new();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let cl = r.add(Node::Input { stream: 2 });
    r.tag(cl, "close");

    let prev = r.add(Node::Lag { src: cl, n: 1 });
    r.tag(prev, "prev_close");
    let tl = r.add(Node::Min { a: lo, b: prev });
    r.tag(tl, "true_low");
    let th = r.add(Node::Max { a: hi, b: prev });
    r.tag(th, "true_high");
    let bp = r.add(Node::Sub { a: cl, b: tl });
    r.tag(bp, "bp");
    let tr = r.add(Node::Sub { a: th, b: tl });
    r.tag(tr, "tr");

    // Two groups of three members, all six slots AFTER both sources: that is
    // what lets `bundles()` place one fused call at the first sum node (its SSA
    // guard is max src < min member).
    let roll = |p: u32, src: Id| Node::RollSum {
        src,
        period: p,
        mean: false,
        skip: 0,
        steady_from: 0,
    };
    let bp_s = r.add(roll(short, bp));
    r.tag(bp_s, "bp_sum_s");
    let bp_m = r.add(roll(medium, bp));
    r.tag(bp_m, "bp_sum_m");
    let bp_l = r.add(roll(long, bp));
    r.tag(bp_l, "bp_sum_l");
    let tr_s = r.add(roll(short, tr));
    r.tag(tr_s, "tr_sum_s");
    let tr_m = r.add(roll(medium, tr));
    r.tag(tr_m, "tr_sum_m");
    let tr_l = r.add(roll(long, tr));
    r.tag(tr_l, "tr_sum_l");

    let four = r.constant(4.0);
    let two = r.constant(2.0);
    let div = r.constant(100.0 / 7.0);
    let bp_s4 = r.add(Node::Mul { a: four, b: bp_s });
    let w_s = r.add(Node::Div { a: bp_s4, b: tr_s });
    let bp_m2 = r.add(Node::Mul { a: two, b: bp_m });
    let w_m = r.add(Node::Div { a: bp_m2, b: tr_m });
    let third = r.add(Node::Div { a: bp_l, b: tr_l });
    // `reduce_sum()` over the hand's 2-lane vector is this add; then `+ third`,
    // then the scale — the same left-to-right order `ultosc.rs` writes.
    let pair = r.add(Node::Add { a: w_s, b: w_m });
    let wsum = r.add(Node::Add { a: pair, b: third });
    let line = r.add(Node::Mul { a: wsum, b: div });
    r.tag(line, "ultosc");

    r.kind = "ultosc";
    r.options = options.to_vec();
    // hand `State` field order: bp_sums_2x (short, medium), tr_sums_2x (short,
    // medium), bp_long_sum, tr_long_sum. All six are consumed by the published
    // ratio, so this list changes no liveness today; `get_states` for a
    // MultiBuffer-shaped hand state is M1 and not wired.
    r.tail_inputs = vec![bp_s, bp_m, tr_s, tr_m, bp_l, tr_l];
    r.outputs.push(Sink::iface_lead(line, long));
    // `tr` and `bp` are OPTIONAL, not primary: the hand allocates them with
    // `init_optional_outputs_eff!(optional_outputs, &[false, false], …)`, so a
    // primary-only request emits ONE row. (The `Info.outputs` table lists all
    // three; the allocation site is the authority — same rule as `atr`'s `tr`.)
    r.optional_outputs.push(Sink::iface_lead(tr, 1));
    r.optional_outputs.push(Sink::iface_lead(bp, 1));
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
    r.tail_inputs = vec![sum];
    r.outputs.push(Sink::iface_lead(sum, period + 15));
    r
}

/// `TR` as a standalone request: the true-range graph over the three price
/// streams, publish lead 1 (`output_length = len − 1`). Stateless —
/// `prevClose` is the `Lag` anchor, not a chain. The same nodes the ATR
/// family's chain feeds on, so a dashboard sharing TR with atr/natr/supertrend
/// rolls it once.
pub fn tr(_options: &[f64]) -> Recipe {
    let mut r = Recipe::new();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let cl = r.add(Node::Input { stream: 2 });
    r.tag(cl, "close");
    let tr = tr_nodes(&mut r, hi, lo, cl);
    r.kind = "tr";
    r.options = Vec::new();
    r.outputs.push(Sink::iface_lead(tr, 1));
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
    r
}

/// `DX(p)` = `(|DM+ − DM−| · 100 / (DM+ + DM−)).max(0)` — a PURE function of the
/// two COMPOSITE accumulators, so it grafts `dm`'s own graph (one source of
/// truth) and needs only high/low; the hand's third input, `close`, feeds only
/// its optional ATR/TR mirrors, which this recipe does not build. Exact op order
/// `(diff · 100) / sum` (left-assoc), then `Max(_,0)`; the divide emits the raw
/// sum (hand `calc_dx` skips the `÷atr`). Publish lead = `period`. `Adx` chains
/// a mean `Wilders` onto this node.
pub fn dx(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hl();
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
    let atr = r.sub(atr(&[period as f64]), &[0, 1, 2]);
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
    let dx = r.sub(dx(&[period as f64]), &[0, 1]);
    let adx = r.add(Node::Wilders {
        src: dx,
        period,
        composite: false,
        feed: true,
    });
    r.tag(adx, "adx");
    // atr/tr mirrors: compose the mean ATR chain (primary + exposed tr) — the
    // same nodes natr/atr use, so shared.
    let atr = r.sub_graph(atr(&[period as f64]), &[0, 1, 2]);
    let atr_id = atr.primary;
    let tr = atr.optional_at(0);
    r.kind = "adx";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(adx, 2 * period - 1));
    // Hand optional_outputs order: [dx, atr, tr].
    r.optional_outputs.push(Sink::iface_lead(dx, period));
    r.optional_outputs.push(Sink::iface_lead(atr_id, period));
    r.optional_outputs.push(Sink::iface_lead(tr, 1));
    r
}

/// `Hilbert(hp, ss)` = `ht(ss(hp(real)))` split into I and Q — the last
/// indicator-side graph, moved here because the graph is ENGINE data: when the
/// node vocabulary changes this file changes, not the indicator module.
///
/// SSA order mirrors the hand's `TState::calc`: the Roofing Filter advances first
/// (`HighPass` then `SuperSmoother`), then the 7-tap Hilbert kernel reads the
/// `roofing` stream and yields I (centre tap) and Q (FIR sum). The Roofing Filter
/// is DECOMPOSED into two nodes rather than one opaque leaf, so the `highpass`
/// optional has its own SSA id and can share with the standalone `highpass`
/// indicator.
///
/// **NOT REGISTERABLE YET, and the reason is measured, not assumed**: `HighPass`,
/// `SuperSmoother`, `HtInPhase` and `HtQuadrature` have zero references in
/// `signature.rs` (the erase vocabulary), `planner.rs` and `codegen.rs`, so
/// `erase` returns `None` for this arena and no `SIG_*` can ever be built from it.
/// It registers the day those four kinds land; the graph itself is final.
pub fn hilberttransform(options: &[f64]) -> Recipe {
    let ss_period = options[0] as Id;
    let hp_period = options[1] as Id;

    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    let highpass = r.add(Node::HighPass {
        input: real,
        period: hp_period,
    });
    r.tag(highpass, "highpass");
    let roofing = r.add(Node::SuperSmoother {
        src: highpass,
        period: ss_period,
    });
    r.tag(roofing, "roofing");
    let in_phase = r.add(Node::HtInPhase { src: roofing });
    r.tag(in_phase, "in_phase");
    let quadrature = r.add(Node::HtQuadrature { src: roofing });
    r.tag(quadrature, "quadrature");

    r.kind = "hilberttransform";
    r.options = options.to_vec();
    // outputs in Info::outputs order: in_phase, quadrature.
    // optional_outputs in Info::optional_outputs order: roofing, highpass.
    r.outputs.push(Sink::iface(in_phase));
    r.outputs.push(Sink::iface(quadrature));
    r.optional_outputs.push(Sink::iface(roofing));
    r.optional_outputs.push(Sink::iface(highpass));
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
    let period = options[0] as u32;
    let look_back = period.saturating_sub(1);

    // `real` IS the close column (`INPUTS = 1`), declared so a co-request with an
    // OHLC recipe interns onto Close instead of onto that recipe's `Input{0}`.
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
    let line = r.add(Node::Div { a: range, b: den });
    r.tag(line, "vhf");

    r.kind = "vhf";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface(line));
    r
}

/// `SMA(p)` — one price-sourced rolling-sum node. Publish lead = `period`:
/// the hand reports `len − period` rows (init folds bars `0..p` but the
/// seed bar is not published), matching the `Ema` mirror convention. Hand
/// state is the lone `sum` — the plan keeps the identical value in the
/// node's state word (see `sma::restore_state`).
pub fn sma(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::new();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let m = r.add(Node::Sma { src: close, period });
    r.tag(m, "sma");
    r.kind = "sma";
    r.options = options.to_vec();
    r.tail_inputs = vec![m];
    r.outputs.push(Sink::iface_lead(m, period));
    r
}

/// `WMA(p)` — the weighted-sum trick node (its own window `sum` carry, read
/// BEFORE its update exactly as the hand `calc` orders it, plus the
/// weighted line) and the shared `Sma` mirror node: any `Sma(p)` request
/// over the same column interns onto that mirror, so the pair's rows come
/// off one arena node while each rolling op keeps its own bit-exact update
/// chain (the hand's own structure). Lead = `period` on both rows.
pub fn wma(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::new();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let w = r.add(Node::Wma { src: close, period });
    r.tag(w, "wma");
    let m = r.add(Node::Sma { src: close, period });
    r.tag(m, "sma");
    r.kind = "wma";
    r.options = options.to_vec();
    r.tail_inputs = vec![w];
    r.outputs.push(Sink::iface_lead(w, period));
    r.optional_outputs.push(Sink::iface_lead(m, period));
    r
}

/// `STDDEV(p)` — the self-contained two-moment node (its own `sum` +
/// `sum_sq` carries, hand `calc` op for op); its optional SMA line is the
/// SAME `Sma` node the standalone request builds, so CSE gives ONE
/// recurrence with two mirrors. Lead = `period` for both rows.
pub fn stddev(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::new();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let sd = r.add(Node::Stddev { src: close, period });
    r.tag(sd, "stddev");
    let m = r.add(Node::Sma { src: close, period });
    r.tag(m, "sma");
    r.kind = "stddev";
    r.options = options.to_vec();
    r.tail_inputs = vec![sd];
    r.outputs.push(Sink::iface_lead(sd, period));
    r.optional_outputs.push(Sink::iface_lead(m, period));
    r
}

/// `BBANDS(p, k)` — the rolling core shared with the standalone `Sma` and
/// `Stddev` requests (middle IS the `Sma` node, spread comes from the
/// `Stddev` node), plus the hand's single-rounded fmas:
/// `upper = k·sd + sma`, `lower = (−k)·sd + sma` — the negated-factor form
/// `(k·−1)·sd + sma` is bit-identical (multiplying by −1 is exact). All
/// three rows print from bar `period` (hand `min_data = p + 1`).
pub fn bbands(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let range = options[1];
    let mut r = Recipe::new();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let m = r.add(Node::Sma { src: close, period });
    r.tag(m, "middle");
    let sd = r.add(Node::Stddev { src: close, period });
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
    let lead = period;
    // hand `Info::outputs` order: lower, middle, upper.
    r.outputs.push(Sink::iface_lead(lo, lead));
    r.outputs.push(Sink::iface_lead(m, lead));
    r.outputs.push(Sink::iface_lead(up, lead));
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
    let mut r = Recipe::new();
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
    let mut r = Recipe::new();
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
    let mut r = Recipe::new();
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
    let mut r = Recipe::new();
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
    let mut r = Recipe::new();
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

/// `DONCHIAN` = `(min, 0.5·(max+min), max)` over the canonical
/// `(period, period−1)` window — the SAME two roll nodes `willr` builds at
/// a matching period (`aroon` deliberately does not join them: its window
/// is `(p, p)`). All three rows print from `period−1` = defined-from.
pub fn donchianchannel(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let look_back = period - 1;
    let mut r = Recipe::new();
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
    let mut r = Recipe::new();
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

/// The AD line as a sub-graph: the guarded CLV times volume, accumulated into
/// ONE [`Node::Accum`] carry. Returns the accumulator's node id.
///
/// Streams are the hand's `TState::Inputs` order (`ad.rs`:
/// `(high, low, close, volume)`), so the caller binds `0=hi, 1=lo, 2=cl, 3=vol`.
///
/// The guard polarity is load-bearing. Hand `ad.rs::calc` is an EARLY RETURN —
/// `if range <= f64::EPSILON { return self.ad }` — so the compute branch is
/// `!(range <= eps)`:
/// * `WhereLt` (`a < b` → zero) puts the `range == eps` bar on the wrong arm.
/// * `WhereGt` (`a > b` → value) pins `0.0` when `range` is NaN, silently
///   UN-poisoning a carry the hand would poison.
/// [`Node::WhereLe`] is the only one whose arms match the hand both at the
/// boundary and under NaN.
///
/// `Ad` and `Adosc` share this helper so their AD sub-graphs intern onto the
/// same arena nodes in a co-request.
pub(crate) fn ad_nodes(r: &mut Recipe, hi: Id, lo: Id, cl: Id, vol: Id) -> Id {
    let range = r.add(Node::Sub { a: hi, b: lo });
    r.tag(range, "hl_range");
    // `(close - low - high + close)` spelled left-to-right exactly as the hand
    // writes it — association changes the last bits, so this is
    // `((cl - lo) - hi) + cl` and NOT a re-association.
    let cl_lo = r.add(Node::Sub { a: cl, b: lo });
    let minus_hi = r.add(Node::Sub { a: cl_lo, b: hi });
    let numer = r.add(Node::Add { a: minus_hi, b: cl });
    r.tag(numer, "clv_numer");
    let ratio = r.add(Node::Div { a: numer, b: range });
    r.tag(ratio, "clv");
    let eps = r.constant(f64::EPSILON);
    let gated = r.add(Node::WhereLe {
        a: range,
        b: eps,
        t: ratio,
    });
    r.tag(gated, "clv_gated");
    // ONE fused rounding: hand `ratio.mul_add(volume, self.ad)`.
    let line = r.add(Node::Accum { a: gated, b: vol });
    r.tag(line, "ad");
    line
}

/// `AD` — Accumulation/Distribution Line. The engine's first volume-consuming
/// recipe: four streams and a single running total, zero options (hand
/// `OPTIONS = 0`).
///
/// Sink lead is `0`: hand `min_data = 1`, `output_length(data_len) = data_len`,
/// so bar 0 already prints (the accumulator's zero init IS the hand's
/// `State::new(0.0)`).
pub fn ad(_options: &[f64]) -> Recipe {
    let mut r = Recipe::hlcvol();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let cl = r.add(Node::Input { stream: 2 });
    r.tag(cl, "close");
    let vol = r.add(Node::Input { stream: 3 });
    r.tag(vol, "volume");

    let line = ad_nodes(&mut r, hi, lo, cl, vol);

    r.kind = "ad";
    r.options = Vec::new();
    // hand `IndicatorState { ad }` — the one carry.
    r.tail_inputs = vec![line];
    r.outputs.push(Sink::iface(line));
    r
}

/// `ADOSC(s, l)` — the Chaikin oscillator: `ema_s(ad) − ema_l(ad)`, the two
/// EMAs chained onto the SAME [`ad_nodes`] accumulator.
///
/// Publish lead `= long − 1` (hand `min_data = options[1]`,
/// `output_length = len − long + 1`). The chain convention is already exact:
/// the AD line's lead is `0`, so each EMA's `feeds` is `0` and `exec` seeds at
/// `t == 0` with `ad[0]` and steps thereafter — statement-for-statement the
/// hand's `init_state` loop (`i == 0` assigns the seed, `i > 0` calls `calc`).
pub fn adosc(options: &[f64]) -> Recipe {
    let short = options[0] as u32;
    let long = options[1] as u32;
    let mut r = Recipe::hlcvol();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let cl = r.add(Node::Input { stream: 2 });
    r.tag(cl, "close");
    let vol = r.add(Node::Input { stream: 3 });
    r.tag(vol, "volume");

    let ad = ad_nodes(&mut r, hi, lo, cl, vol);
    let ema_short = r.add(Node::Ema {
        src: ad,
        period: short,
    });
    r.tag(ema_short, "ema_short");
    let ema_long = r.add(Node::Ema {
        src: ad,
        period: long,
    });
    r.tag(ema_long, "ema_long");
    let osc = r.add(Node::Sub {
        a: ema_short,
        b: ema_long,
    });
    r.tag(osc, "adosc");

    r.kind = "adosc";
    r.options = options.to_vec();
    // hand `State { ema_state: EmaSimdState<2>, ad_state }` — SSA order (short
    // before long, the AD line last as `TState::calc` computes it first).
    r.tail_inputs = vec![ema_short, ema_long, ad];
    r.outputs.push(Sink::iface_lead(osc, long - 1));
    // Hand `optional_outputs = ["short_ema", "long_ema", "ad"]`, in that order,
    // each at ITS OWN length: `short_ema_line` is allocated
    // `Ema::output_length(len, short)` = len − short (lead `short`, the seed bar
    // unpublished), `long_ema_line` reuses `adosc_capacity` = len − long + 1
    // (lead `long − 1`), and the AD line is `Ad::output_length(len)` = len
    // (lead 0).
    r.optional_outputs = vec![
        Sink::iface_lead(ema_short, short),
        Sink::iface_lead(ema_long, long - 1),
        Sink::iface(ad),
    ];
    r
}

/// `MOM(p) = real[i] − real[i − p]` — the engine's multi-bar `Lag` read.
///
/// This is the recipe that makes `lag_n` observable in a PUBLISHED row. The
/// hand's `cycle_mom` pairs bar `i` with bar `i − p`, and its lead is `p`
/// (`min_data = p + 1`, `output_length = len − p`), so a steady-loop read of
/// one bar back instead of `p` is a wrong VALUE, not a state-only artefact —
/// which is what `cmo`'s `Lag{src: real, n: period}` could never show, because
/// there the lag feeds only a `Mirror` (state, `get_states`, M1) and never a
/// sink.
///
/// `n` is a VALUE (`Values::lag_n`), never structure: the signature erases it,
/// and at macro expansion the parameter arrives as a sentinel. Same as `cmo`.
///
/// The hand's state is a raw window of the last `p` `real` values, not an
/// engine chain, so `tail_inputs` is empty — `get_states` has no chain to
/// restore here.
pub fn mom(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::new();
    let cl = r.add(Node::Input { stream: 0 });
    r.tag(cl, "price");
    let prev = r.add(Node::Lag { src: cl, n: period });
    r.tag(prev, "prev_period");
    let line = r.add(Node::Sub { a: cl, b: prev });
    r.tag(line, "mom");

    r.kind = "mom";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(line, period));
    r
}
