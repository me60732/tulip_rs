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
    let mut r = Recipe::new();
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
    let mut r = Recipe::new();
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
    let mut r = Recipe::new();
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
    let mut r = Recipe::new();
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
    let mut r = Recipe::new();
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
    let mut r = Recipe::new();
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
    let mut r = Recipe::new();
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
