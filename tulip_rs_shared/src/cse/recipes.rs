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
//! `Recipe::sub`/`sub_at`/`sub_graph` calls produced them — helpers in this
//! module and its family modules inline those grafts without reordering nodes).

use crate::cse::multipliers::kama_multiplier;
use crate::cse::AlphaShape;
use crate::cse::{Id, Node, Recipe};

mod cycle_ehlers;
mod momentum_oscillators;
mod price_transforms;
mod primitives;
mod range_roll_stats;
mod trend_channels;
mod volume_moneyflow;

use primitives::stddev_recipe;

pub use momentum_oscillators::{
    ao, apo, cmo, dema, dpo, ef, fisher, hma, kama, macd, mom, ppo, qstick, roc, rocr, rsi, stoch,
    stochrsi, tema, trix, ultosc, vidya, zlema,
};
pub use range_roll_stats::{
    adx, adxr, aroon, aroon_graph, aroonosc, cci, cvi, di, dm, dx, fosc, linreg, mass, max, md,
    min, tr, trima, tsf, vhf, vortex, willr, AroonLines,
};

/// `Ema(p)` over `Input{0}`: one chain node mirroring its own line.
pub fn ema(options: &[f64]) -> Recipe {
    primitives::ema(options)
}

/// `WILDERS(p)` over `Input{0}` — one decayed-average node.
pub fn wilders(options: &[f64]) -> Recipe {
    primitives::wilders(options)
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
    trend_channels::atr(options)
}

/// `NATR(p)` = `(atr / close) * 100` wrapping the SAME mean-`Wilders(tr)`
/// node `atr` builds. Close arrives FIRST (stream 2 before 0/1) — the arena
/// order the family's shared chains were pinned with. Exact op order
/// `(atr / close) * 100.0` (hand `calc`).
pub fn natr(options: &[f64]) -> Recipe {
    trend_channels::natr(options)
}

/// `SUPERTREND(p, step)` — the multi-value feedback band over THREE shared
/// nodes: the mean-`Wilders(tr)` atr (dedups with natr/atr), the
/// `0.5·(high+low)` medprice, and close. `st` is the bespoke op (4 carried
/// states mirroring the hand `State`); publish lead = `period`, the op
/// seeds one `calc_st` at the atr seam. Close FIRST (like natr's order).
pub fn supertrend(options: &[f64]) -> Recipe {
    trend_channels::supertrend(options)
}

pub fn psar(options: &[f64]) -> Recipe {
    trend_channels::psar(options)
}

/// `msw` = MESA Sine Wave.
pub fn msw(options: &[f64]) -> Recipe {
    cycle_ehlers::msw(options)
}

/// Ehlers Homodyne Discriminator -- a single bespoke carry node.
pub fn homodynediscriminator(options: &[f64]) -> Recipe {
    cycle_ehlers::homodynediscriminator(options)
}

/// `InstantaneousTrendline()` — Ehlers 2-pole IIR with adaptive α from embedded HD.
/// Streams: 0=real. Options: none (fully adaptive). Fixed 23-bar warmup (bars 0-21),
/// lead 22. Uses the same HD pipeline as `homodynediscriminator`, but applies a
/// separate 2-pole IIR recurrence on top of the DC period.
/// Graph: hd = HomodyneDiscriminator(real), it = InstantaneousTrendline(hd).
/// First published bar is 22 (min_data=23).
pub fn instantaneoustrendline(options: &[f64]) -> Recipe {
    cycle_ehlers::instantaneoustrendline(options)
}

pub fn mama(options: &[f64]) -> Recipe {
    cycle_ehlers::mama(options)
}

/// `MEDPRICE = 0.5 * (high + low)` -- stateless, every bar publishes
/// (`Sink::iface`, lead 0). Streams: 0 = high, 1 = low.
pub fn medprice(_options: &[f64]) -> Recipe {
    price_transforms::medprice(_options)
}

/// `EMV` — the median-price movement, scaled by the guarded range and volume.
/// The derived median price is delayed with `RollAge`, not a raw-input `Lag`.
pub fn emv(options: &[f64]) -> Recipe {
    volume_moneyflow::emv(options)
}

/// `Hilbert(hp, ss)` = `ht(ss(hp(real)))` split into I and Q.
pub fn hilberttransform(options: &[f64]) -> Recipe {
    cycle_ehlers::hilberttransform(options)
}

/// `HighPass(period)` — standalone forwarder onto `Node::HighPass`.
pub fn highpass(options: &[f64]) -> Recipe {
    cycle_ehlers::highpass(options)
}

/// `SuperSmoother(period)` — standalone forwarder onto `Node::SuperSmoother`.
pub fn supersmoother(options: &[f64]) -> Recipe {
    cycle_ehlers::supersmoother(options)
}

/// `RoofingFilter(ss_period, hp_period)` — cascades HighPass into SuperSmoother.
pub fn roofingfilter(options: &[f64]) -> Recipe {
    cycle_ehlers::roofingfilter(options)
}

/// `SMA(p)` — one price-sourced rolling-sum node.
pub fn sma(options: &[f64]) -> Recipe {
    primitives::sma(options)
}

/// `WMA(p)` — weighted moving average recipe.
pub fn wma(options: &[f64]) -> Recipe {
    primitives::wma(options)
}

pub fn stddev(options: &[f64]) -> Recipe {
    primitives::stddev(options)
}

/// `BBANDS(p, k)` — the rolling core shared with the standalone `Sma` and
/// `Stddev` requests (middle IS the `Sma` node, spread comes from the
/// `Stddev` node), plus the hand's single-rounded fmas:
/// `upper = k·sd + sma`, `lower = (−k)·sd + sma` — the negated-factor form
/// `(k·−1)·sd + sma` is bit-identical (multiplying by −1 is exact). All
/// three rows print from bar `period` (hand `min_data = p + 1`).
/// `BBANDS(p, range)` — Bollinger Bands: middle SMA, stddev band width,
/// upper/middle/lower via FMA. Streams: 0=close. Options `[period, range]`.
pub fn bbands(options: &[f64]) -> Recipe {
    trend_channels::bbands(options)
}

/// `DONCHIAN` = `(min, 0.5·(max+min), max)` over the canonical
/// `(period, period−1)` window — the SAME two roll nodes `willr` builds at
/// a matching period (`aroon` deliberately does not join them: its window
/// is `(p, p)`). All three rows print from `period−1` = defined-from.
pub fn donchianchannel(options: &[f64]) -> Recipe {
    trend_channels::donchianchannel(options)
}

/// `ICHIMOKU(s, m)` — six canonical rolls (short/medium/ultra × max/min)
/// over high and low plus the `0.5·(x+y)` combines, `l = 2m`. The densest
/// hub case: three windows per column, one arena, one pass. Narrow → wide
/// add-order (matches the interned arena); leads from `slot_lengths`
/// (conversion `s−1`, base `m−1`, both spans `2m−1` — the hand withholds
/// span A there on purpose). `lagging_span` is a verbatim close clone the
/// engine has no business copying — not declared (same fact as runtime).
pub fn ichimoku(options: &[f64]) -> Recipe {
    trend_channels::ichimoku(options)
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
#[allow(dead_code)]
pub(crate) fn ad_nodes(r: &mut Recipe, hi: Id, lo: Id, cl: Id, vol: Id) -> Id {
    volume_moneyflow::ad_nodes(r, hi, lo, cl, vol)
}

/// `AD` — Accumulation/Distribution Line. The engine's first volume-consuming
/// recipe: four streams and a single running total, zero options (hand
/// `OPTIONS = 0`).
///
/// Sink lead is `0`: hand `min_data = 1`, `output_length(data_len) = data_len`,
/// so bar 0 already prints (the accumulator's zero init IS the hand's
/// `State::new(0.0)`).
pub fn ad(options: &[f64]) -> Recipe {
    volume_moneyflow::ad(options)
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
    volume_moneyflow::adosc(options)
}

/// `BOP` — Balance of Power: `(close - open) / (high - low).max(f64::EPSILON)`.
/// Streams: 0=open, 1=high, 2=low, 3=close. No options.
pub fn bop(_options: &[f64]) -> Recipe {
    price_transforms::bop(_options)
}

/// `MARKETFFI` (Market Facilitation Index) — hand `marketfi.rs::calc` is
/// `(high - low) / volume.max(f64::EPSILON)`. Stateless: `Info.options = []`,
/// `min_data = 1`, `output_length = data_len`, so lead 0 and `Sink::iface`.
///
/// Columns are `hlvol()` = the hand's `Info.inputs = ["high","low","volume"]`. This is the
/// first consumer of that constructor, and the reason it exists rather than a hand-typed
/// column list: `with_columns` is private precisely so a recipe cannot name a column set
/// the canonical rank order would silently shift.
///
/// SHARES ARITHMETIC WITH `BOP`, not a recurrence: `bop` builds `Sub{high, low}` and an
/// `EPSILON` `Const`, and `marketfi` needs exactly those two nodes -- its denominator is
/// `Max{volume, EPSILON}` (a DIFFERENT `Max`, over volume rather than the range), and its
/// numerator IS bop's `hl_diff`. So `Bop() + Marketfi()` interns the range `Sub` and the
/// constant and adds only two nodes. There is no carry here to share, which is why the
/// gate counts nodes rather than claiming any fused recurrence.
pub fn marketfi(options: &[f64]) -> Recipe {
    volume_moneyflow::marketfi(options)
}

/// `TYPPRICE` — Typical Price: `(high + low + close) * (1.0 / 3.0)`.
/// Streams: 0=high, 1=low, 2=close. No options.
pub fn typprice(_options: &[f64]) -> Recipe {
    price_transforms::typprice(_options)
}

/// `WCPRICE` — Weighted Close Price, hand `wcprice.rs::calc` =
/// `close.mul_add(2.0, high + low) * 0.25`. The `mul_add` is **one fused rounding**, so
/// this spells `Node::Fma` — a separate `Mul`+`Add` double-rounds and is not bit-exact.
/// Streams: 0=high, 1=low, 2=close. No options.
pub fn wcprice(_options: &[f64]) -> Recipe {
    price_transforms::wcprice(_options)
}

/// `AVGPRICE` = `(open + high + low + close) * 0.25` — stateless, every bar
/// publishes (`Sink::iface`, lead 0). Streams: 0=open, 1=high, 2=low, 3=close.
pub fn avgprice(_options: &[f64]) -> Recipe {
    price_transforms::avgprice(_options)
}

/// `VWMA(p)` — the volume-weighted moving average: the ratio of two WINDOW SUMS,
/// `Σ(close·volume)` over `p` bars divided by `Σ volume` over the same `p` bars. The
/// hand writes this as the telescoping `sum += close*volume − prev_close*prev_volume`
/// / `vol_sum += volume − prev_volume`, which is exactly what a raw `RollSum` does
/// internally (incoming minus outgoing) — NOT an unbounded `Accum`, which would never
/// drop a bar. Hand `if self.vol_sum == 0.0 { return 0.0 }` guards the DIVISION
/// RESULT, so it is spelled `WhereLe{ Abs(vol_sum) <= 0 } → 0 : ratio`: the take arm
/// is `!(a <= b)`, which holds for every non-zero finite value AND for NaN (where the
/// hand also falls through to `sum/NaN`), and pins 0 only at ±0.0. Streams: 0=close,
/// 1=volume (`cvol()`). Options `[period]`. Publish lead = `period`.
pub fn vwma(options: &[f64]) -> Recipe {
    volume_moneyflow::vwma(options)
}

/// `CHAIKINMF(p)` — Chaikin Money Flow, hand `chaikinmf.rs`. Two rolling sums over
/// the same window, then their ratio:
/// `sum(mfv, p) / sum(volume, p)` where
/// `mfv = ((close - low) - (high - close)) / (high - low).max(EPSILON) * volume`.
///
/// Columns are `hlcvol()` because `Info.inputs = ["high", "low", "close", "volume"]`
/// — the SAME column set as `ad`, so a co-requested `Ad + ChaikinMf` interns all four
/// `Input` slots, the `Sub{high, low}` range node, and the `EPSILON` `Const` (interned
/// by value). It does NOT intern the money-flow numerator: the hand writes
/// `(close - low) - (high - close)`, while `ad_nodes` writes the left-to-right
/// `((close - low) - high) + close`. Those associate differently and are not bit-equal
/// in f64, so sharing them would be a fabrication — the two graphs keep separate
/// numerators on purpose.
///
/// THE FINAL DIVIDE IS UNGUARDED. `calc` is literally `self.sums[0] / self.sums[1]`
/// with no `if denom == 0`, unlike `vwma`/`vosc` which DO guard their ratio. Adding a
/// `WhereLe` here to match those neighbours would be inventing a guard the hand does not
/// have: on an all-zero-volume window the hand prints `0.0 / 0.0` = NaN, and so must the
/// kernel. The `Max(range, EPSILON)` floor belongs only to the per-bar `mfv`, NOT to the
/// window ratio — reading it as the guard for both is the easy wrong transcription.
///
/// Lead = `period`: the hand overrides neither `min_data` (default `options[OP-1] + 1`)
/// nor `output_length` (default `len - min_data + 1`), and its steady loop starts at
/// `&high[period..]` — all three agree on `period`.
pub fn chaikinmf(options: &[f64]) -> Recipe {
    volume_moneyflow::chaikinmf(options)
}

/// `VWAP` — Volume Weighted Average Price, hand `vwap.rs`. CUMULATIVE, not a window:
/// `pv_sum += tp * volume; vol_sum += volume; (pv_sum / vol_sum, tp)`, zero options — so
/// both lanes are `Node::Accum` running totals (the `ad` shape) rather than rolls.
///
/// `tp` IS `typprice::calc` = `(high + low + close) * (1.0 / 3.0)`, spelled here
/// node-for-node as [`typprice`] spells it (`Add{h,l}`, `Add{_,c}`, `Const(1/3)`,
/// `Mul{_,third}`). `intern` keys on the whole `Node`, so a co-requested
/// `Vwap + Typprice` shares the ENTIRE typical-price subgraph — and vwap's OPTIONAL
/// `typprice` row is literally Typprice's PRIMARY row: one arena node, two published
/// rows. That is the `E8` calculated-inputs payoff arriving by ordinary interning, with
/// no `Node::CalcInput` provision switch built.
///
/// THE ACCUMULATOR IS NOT `tp.mul_add(volume, carry)`. The hand writes
/// `self.pv_sum += tp * volume` — a multiply THAT a separate add, two roundings — while
/// `Accum{a, b}` is `carry = a.mul_add(b, carry)`, ONE fused rounding. `ad` legitimately
/// uses the fused form because its hand literally writes `ratio.mul_add(volume, self.ad)`;
/// copying that spelling here would change the bits. So the product is its own `Mul` node
/// and each lane is `Accum{a: <something>, b: Const(1.0)}`: `fma(m, 1.0, carry)` is
/// `RN(m + carry)`, bit-identical to a plain add, because multiplying by 1.0 is exact —
/// including signed zero and NaN. The volume lane needs the same 1.0, and the `Const`
/// interns with the price lane's (its bits are the key), so one constant serves both.
///
/// THE DIVIDE IS UNGUARDED, exactly like [`chaikinmf`]: `pv_sum / vol_sum` with no zero
/// test, so an all-zero-volume series must print the hand's `0.0 / 0.0` = NaN. The two
/// neighbouring volume ratios disagree on this (`vwma` and `vosc` DO guard), which is why
/// the guard is read from THIS hand rather than borrowed.
///
/// Lead = 0: `min_data`/`output_length` both DELEGATE to `Typprice` (`1` and `data_len`),
/// so bar 0 prints — `Sink::iface` (node-derived lead) rather than an explicit one.
/// Columns are `hlcvol()`, the hand's `Info.inputs = ["high","low","close","volume"]`.
pub fn vwap(options: &[f64]) -> Recipe {
    volume_moneyflow::vwap(options)
}

/// `SMAENVELOPE(p, b)` — hand `smaenvelope.rs`: the SMA line plus `sma * (b/100)` added
/// to and subtracted from it. `Info.outputs = ["lower","middle","upper"]` with NO
/// optional outputs, so all three are PRIMARIES, in that order.
///
/// The middle band is the SAME `Node::Sma{src: Input{0}, period}` that [`sma`] builds over
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
/// the same explicit `+1` [`sma`] needs for the identical reason. `Sink::iface` here would
/// publish one bar early.
pub fn smaenvelope(options: &[f64]) -> Recipe {
    trend_channels::smaenvelope(options)
}

/// `OBV` — On-Balance Volume, hand `obv.rs`: `if close > prev_close { obv += volume } else
/// if close < prev_close { obv -= volume }` — and NOTE THERE IS NO `else`: on an equal
/// close the carry is left EXACTLY as it was. Columns are `cvol()`, the hand's
/// `Info.inputs = ["close", "volume"]`.
///
/// Spelled as the difference of two `WhereGt` selects, NOT as a guard:
/// `delta = WhereGt{close, prev, vol} - WhereGt{prev, close, vol}` gives `+vol`, `-vol`,
/// `0.0` for the three cases, and both compares are false under NaN so a NaN close yields
/// `0.0` -- which is what the hand does, since `NaN > x` and `NaN < x` are both false.
/// A single `Where*` cannot express three-way selection, and `WhereLt` would be wrong on
/// its face: its value arm is `>=` (`if a < b { 0.0 } else { t }`, codegen.rs:1455-1464),
/// so it takes the value on the `a == b` boundary the hand leaves alone.
///
/// THE ACCUMULATOR IS `Accum{a: delta, b: Const(1.0)}`, and the `1.0` is load-bearing for
/// bit-exactness, not filler: `Accum` is `carry = a.mul_add(b, carry)`, so with `b = 1.0`
/// the product is exact and the step reduces to `RN(delta + carry)` -- the hand's plain
/// `+=`/`-=`. Multiplying by `+1.0`/`-1.0` is exact in IEEE, so `delta` is `+vol` or
/// `-vol` with no rounding of its own, and `carry` is never poisoned by the multiply.
///
/// Publish lead = 1: `min_data = 2` (`obv.rs:89`), default `output_length`, and `indicator`
/// seeds `prev_close = close[0]` then cycles `&inputs[0][1..]` -- bar 0 is consumed by the
/// seed and never printed. The `Lag{close, 1}` node composes to lead 1 on its own, so the
/// sink is `Sink::iface` with no explicit lead.
///
/// SHARED WITH THE `tr` FAMILY: `tr_nodes` builds `Lag{src: close, n: 1}` over `hlc()`,
/// this builds the same node over `cvol()`. Because `intern` keys on the whole `Node` AFTER
/// the column-declared remap (`merge.rs:864-874`), a co-requested `Tr + Obv` has ONE
/// prev-close lag despite the two recipes disagreeing about which stream index `close` is
/// (2 versus 0). That is the column law working, gated in the POSITIVE direction for the
/// first time -- every prior test pinned only the rejection half.
pub fn obv(options: &[f64]) -> Recipe {
    volume_moneyflow::obv(options)
}

/// `WAD` — Williams Accumulation/Distribution. The two signed close-movement
/// branches sum before the same unit-multiplier accumulator used by `obv`.
pub fn wad(options: &[f64]) -> Recipe {
    volume_moneyflow::wad(options)
}

/// `MFI(p)` — signed typical-price money flow over two plain rolling sums.
/// The hand seeds the typical price on bar 0 and begins publishing at bar p.
pub fn mfi(options: &[f64]) -> Recipe {
    volume_moneyflow::mfi(options)
}

/// `KELTNERCHANNEL(p, m)` — hand `keltnerchannel.rs`: `AtrState` and `EmaState` over the
/// SAME period, then `per = atr * step`, `(ema - per, ema, ema + per)` plus the `atr`/`tr`
/// mirrors. Columns `hlc()` from `Info.inputs = ["high","low","close"]`; lead `period`
/// (`min_data = options[0] + 1` at `keltnerchannel.rs:206`).
///
/// COMPOSES THE ATR, DOES NOT RESTATE IT: `atr_nodes` is documented as the ONE spelling
/// every family member means, so the bands hang off the same `Wilders{composite:false}` and
/// the same tr walk. That this is legal and not a silent re-scaling is checked, not
/// assumed: `keltnerchannel.rs:157-160` stores `atr_line => atr` with
/// `store_optional_outputs!` -- UNCORRECTED. Compare `adxr.rs`, which must use
/// `store_optional_outputs_corrected!` because ITS ATR is reached through `DxState` and
/// runs in composite (sum) mode. Keltner calls `AtrState::init_state` directly, i.e. mean
/// mode, which is exactly the node `Atr` publishes bit-exactly today.
///
/// THREE PRIMARIES IN INFO ORDER (`lower, middle, upper`) and TWO OPTIONALS (`atr, tr`),
/// the `tr` mirror at lead 1 like `atr`'s own (`tr_line` is sized
/// `Tr::output_length(len)`, so it starts one bar later than the bands).
///
/// THE HUB PAYS HERE MORE THAN ANYWHERE ELSE IN THE REGISTRY. `Ema(p) + Atr(p) +
/// KeltnerChannel(p, m)` is a real dashboard combo, and the ATR ablation measured the tr
/// walk as the single biggest component of `atr_n`'s 1.59x deficit -- so computing that
/// walk ONCE for three consumers is the merge win the engine exists for.
pub fn keltnerchannel(options: &[f64]) -> Recipe {
    trend_channels::keltnerchannel(options)
}

/// `VOSC(s, l)` — the hand's `calc((current, short, long))` calls
/// `short_state.calc((current, short))` and `long_state.calc((current, long))`, i.e.
/// TWO `SmaState` windows over the SAME series (`volume`) with different sizes: `s`
/// and `l`. `Info.inputs = ["volume"]` with `INPUTS = 1`, so the arena is `vol()` —
/// declaring `price()` here would silently bind CLOSE under a "volume" tag, the exact
/// positional hazard the column-declared remap exists to prevent.
/// `if slow_sma == 0.0 → 0` guards the result, spelled `WhereLe{ Abs(slow) <= 0 }` for
/// the same NaN/-0.0 reasons as `vwma`. Streams: 0=volume. Options `[short, long]`.
/// Primary lead = `long`; `Info.optional_outputs = ["short_sma","long_sma"]` in that
/// order, each with ITS OWN lead (declared-order rule).
pub fn vosc(options: &[f64]) -> Recipe {
    volume_moneyflow::vosc(options)
}

/// `NVI` -- Negative Volume Index, hand `nvi.rs`: a gated multiplicative
/// prefix product seeded 1000.0, `nvi = close/prev_close * nvi` only when
/// `volume < prev_volume`, held flat otherwise. Columns are `cvol()`.
pub fn nvi(options: &[f64]) -> Recipe {
    volume_moneyflow::nvi(options)
}

/// `PVI` -- Positive Volume Index, the mirror gate (`volume > prev_volume`).
/// See [`nvi`].
pub fn pvi(options: &[f64]) -> Recipe {
    volume_moneyflow::pvi(options)
}

/// `CHANDELIEREXIT(period, step)` -- hand `chandelierexit.rs`: `long = atr.mul_add(-step,
/// max)`, `short = atr.mul_add(step, min)`, where `max`/`min` are the SAME `(period,
/// period-1)` `RollMax`/`RollMin` pair `willr`/`donchianchannel` build over high/low, and
/// `atr` is the SAME mean-`Wilders(tr)` chain `atr()` builds. No new `Node` kind: this is
/// pure composition of three already-erasable machines plus two `Fma` nodes. `step` is
/// folded to a literal at recipe-build time (`constant(-step)`), matching the hand's
/// `mul_add(-self.step, ..)` bit-for-bit -- negating a literal is exact, never a runtime op.
pub fn chandelierexit(options: &[f64]) -> Recipe {
    trend_channels::chandelierexit(options)
}

/// `KVO(short, long)` -- Klinger Volume Oscillator: a bespoke 4-state
/// recurrence (`KvoFlow`, see its doc) producing the signed volume-force
/// line `vf`, then two EMA chains and their difference -- hand `kvo.rs`.
pub fn kvo(options: &[f64]) -> Recipe {
    volume_moneyflow::kvo(options)
}

/// `ELDERRAY(p)` -- hand `elderray.rs`: `bull = high - ema(close,p)`,
/// `bear = low - ema`. No new `Node` kind -- the SAME `ema()` chain, just
/// with two `Sub` taps instead of one. Publish lead = `period`, same +1
/// bump over the node's own structural lead that `ema()`'s own `e` sink
/// applies (the seed bar is not published).
pub fn elderray(options: &[f64]) -> Recipe {
    momentum_oscillators::elderray(options)
}

/// `TRVI(p)` -- True Range Volatility Indicator, hand `trvi.rs`. CLOSED
/// 2026-10-26 via the new `Node::SeededEma { seed, src, period }` primitive.
pub fn trvi(options: &[f64]) -> Recipe {
    range_roll_stats::trvi(options)
}

/// `Volatility(p)` — the hand's volatility indicator: rate-of-change values
/// over a rolling stddev. The hand computes std dev manually from two rolling sums
/// over the derived ROC values, so we do the same here to match semantics.
pub fn volatility(options: &[f64]) -> Recipe {
    range_roll_stats::volatility(options)
}

/// ENGINE-CORRECTNESS PROBE for B4a -- NOT a real indicator, no hand oracle.
/// Exercises the one shape the checklist flagged as untested: a MEAN `Wilders`
/// and a COMPOSITE `Wilders` sharing the SAME source node (here, `tr`) in one
/// arena, so `classify_packed` (`tulip_rs_macros/src/cse/codegen.rs`) buckets
/// them into ONE lane group (`(Kind::Wilders, tr)`). `erase()` already
/// publishes the per-node magnitude correctly (`invms[k] = 1.0` for
/// composite, `1/period` for mean -- `signature.rs`'s `Node::Wilders` arm),
/// and the packed-group hoist already reads `vals.invms[k]`/`vals.mults[k]`
/// PER SLOT (`codegen.rs`'s `Simd::from_array([vals.mults[k0], vals.mults[k1]])`
/// in the group hoist, and `vals.invms[k0], vals.invms[k1]` likewise) -- so
/// this is a confirmation probe, not new code: B4a's own notes already traced
/// this as "value-correct by construction", reachable by no shipped recipe.
pub fn wilders_mixed_probe(options: &[f64]) -> Recipe {
    range_roll_stats::wilders_mixed_probe(options)
}

/// `Cybercycle(alpha)` — standalone forwarder onto `Node::Cybercycle`.
pub fn cybercycle(options: &[f64]) -> Recipe {
    cycle_ehlers::cybercycle(options)
}

/// `TrendMode(alpha)` — uses cybercycle + peak decay envelope + threshold logic.
pub fn trendmode(options: &[f64]) -> Recipe {
    cycle_ehlers::trendmode(options)
}

/// `CcFisher(alpha)` — uses cybercycle + peak decay envelope + Fisher transform.
pub fn ccfisher(options: &[f64]) -> Recipe {
    cycle_ehlers::ccfisher(options)
}
