//! The recipe *language*: what an indicator request **is** to the planner.
//!
//! A [`Recipe`] is not a program to interpret — it is the indicator's math
//! written as a **node graph**: everything downstream of the raw price
//! streams, including the derived values an indicator used to hide behind a
//! "tail" label. The planner/executor split is by node kind, not indicator:
//!
//! * [`nodes`] — the graph, SSA-ordered: [`Node::Input`] streams,
//!   [`Node::Ema`] recurrences, [`Node::Sub`]/[`Mul`]/[`Div`]/[`Max`] glue and
//!   [`Node::Const`] leaves. All of it is CSE-merged across requests, so
//!   `ema26(close)` advanced for APO is the same state MACD and PPO read —
//!   and the `ema12 − ema26` line APO, MACD and PPO all consume is ONE node.
//! * [`tail_inputs`] — the arena ids of the chains the indicator's hand state
//!   holds, in `TState` field order; a runtime-side restorer rebuilds the
//!   concrete hand `IndicatorState` from them (`get_states`). Both are the ONLY
//!   places an indicator's identity appears outside its own module — the
//!   executor emits node kinds, never indicator names.
//! * [`outputs`] / [`optional_outputs`] — sinks: node references with an
//!   explicit publish lead (hand `slot_lengths`/`output_length` facts; a
//!   passthrough EMA line and a derived oscillator line are the same thing —
//!   a mirror of a node).
//!
//! [`nodes`]: Recipe::nodes · [`tail_inputs`]: Recipe::tail_inputs ·
//! [`outputs`]: Recipe::outputs · [`optional_outputs`]: Recipe::optional_outputs
//!
//! SSA discipline: a node's inputs are lower ids, so the merged arena is
//! topologically ordered and `merge` remaps local→global ids.

/// Local index of a node inside a [`Recipe`]'s `nodes` vector (SSA order).
pub type Id = u32;

/// A resolved state chain handed to a runtime restorer: one entry per
/// [`Recipe::tail_inputs`] id, in that order. The engine builds these from
/// its OWN `OpE` vocabulary (never indicator identity), so this is not a
/// blindness violation — it is the machine's instruction set surfaced as
/// data. The indicator module pattern-matches the variants back into its
/// concrete hand `State<Warm>` (including a `Buffer` for HMA's diff ring).
#[derive(Debug, Clone)]
pub enum ChainState {
    /// `Node::Mirror` — the last value the node published.
    Mirror { value: f64 },
    /// A price- or chain-sourced EMA recurrence — one carried value.
    Ema { value: f64, period: u32 },
    /// A rolling-sum SMA — the group `sum`.
    Sma { sum: f64, period: u32 },
    /// Wilder's smoothing — the carried decayed-average `value`.
    Wilders { value: f64, period: u32 },
    /// Rolling `sum` + `sum_sq` (the two recurrences hand stddev carries).
    Stddev { sum: f64, sum_sq: f64, period: u32 },
    /// Rolling maximum (E9) — the carried extremum + the `trail`.
    RollMax {
        value: f64,
        trail: u64,
        period: u32,
        look_back: u32,
    },
    /// Rolling minimum (E9) — the mirror of [`ChainState::RollMax`].
    RollMin {
        value: f64,
        trail: u64,
        period: u32,
        look_back: u32,
    },
    /// Rolling raw `sum` + the weighted-sum trick line.
    Wma {
        sum: f64,
        weighted: f64,
        period: u32,
    },
    /// HMA's self-contained state: `[sum1, ws1, sum2, ws2, wss, ss]`.
    Hma {
        accs: [f64; 6],
        cursor: u64,
        ring: Vec<f64>,
        period: u32,
        p2: u32,
        psqrt: u32,
    },
    /// `Node::SuperTrend`'s own 4-field feedback carry (`stb`/`stc_ub`/
    /// `stlb`/`st` in the kernel's per-node state arrays) — mirrors hand
    /// `supertrend::State`'s `prev_st`/`prev_ub`/`prev_lb`/`trend`. `step` is
    /// not carried state — it's the node's own `step_bits` literal, copied
    /// through the same way `Ema`/`Wilders` carry their recipe-constant
    /// `period` alongside the live value.
    Supertrend {
        prev_st: f64,
        prev_ub: f64,
        prev_lb: f64,
        trend: bool,
        step: f64,
    },
    /// `Node::RollAge` — a ring-backed published-lag roll (e.g. `adxr`'s ring of
    /// past ADX values): the raw ring contents + cursor, read back the same way
    /// `ChainState::Hma`'s `ring`/`cursor` pair is (see `Hma` restorer in
    /// `hma.rs` for the reconstruction pattern).
    RollAge { ring: Vec<f64>, cursor: u64, n: u32 },
    /// `Node::KvoFlow`'s own hidden 4-field feedback carry (`cm`/`prev_hlc`/
    /// `prev_hl`/`trend` in the kernel's per-node state arrays) — mirrors hand
    /// `kvo::State`'s own `cm`/`prev_hlc`/`prev_hl`/`trend` fields. `KvoFlow`
    /// only *publishes* `vf` downstream, so this hidden state needs its own
    /// dedicated variant, the same way `ChainState::Supertrend` does for
    /// `Node::SuperTrend`'s hidden band-feedback fields.
    KvoFlow {
        cm: f64,
        prev_hlc: f64,
        prev_hl: f64,
        trend: bool,
    },
    /// `Node::IirClamped`'s own hidden carried value (`fisher`'s `val1`) —
    /// a single `st[k]` scalar, same shape as [`ChainState::Mirror`], but
    /// tagged distinctly because it originates from a dedicated node KIND
    /// (`Node::IirClamped`, not `Node::Mirror`), matching this enum's
    /// one-variant-per-node-kind convention (`Ema`/`Wilders`/`Supertrend`/
    /// `KvoFlow` are all named after their originating `Node` kind the same
    /// way). No `period` field: `IirClamped`'s `a_bits`/`b_bits`/clamp
    /// thresholds are recipe-fixed constants, not per-period data, so the
    /// node carries nothing else worth restoring.
    IirClamped { value: f64 },
    /// `Node::RollSum`'s own ring, when the hand state needs the raw WINDOW
    /// content (not just the running `sum`) — e.g. `mass`'s `buffer: Buffer`
    /// of past `mass` ratios, which the generic `ChainState::Sma` shape
    /// (`{sum, period}`, no ring) cannot express. The physical per-group ring
    /// storage (`rings.feed`/`rings.at`/`rings.window` in the generated
    /// kernel, `tulip_rs_macros::cse::codegen`) already exists for every
    /// `RollSum` group regardless of its `RCLASS_SUM`/`RCLASS_RESCAN` class
    /// — `lowering.rs`'s "the buffer is NOT a mirror" note is about skipping a
    /// SECOND lane-major copy for the rescan-disjoint-trigger case, not about
    /// the primary ring not existing — so this is a per-node EXPORT CHOICE a
    /// populator makes at a specific `RollSum` node (read its own ring
    /// instead of collapsing to the generic `Sma` shape), the same way
    /// `Hma`/`RollAge` already get bespoke ring shapes instead of a generic
    /// default for their own nodes. Reconstruction mirrors
    /// `ChainState::Hma`'s `ring`/`cursor` pair (see the `Hma`/`Adxr`
    /// restorers).
    RollSumRing {
        sum: f64,
        ring: Vec<f64>,
        cursor: u64,
        period: u32,
    },
    /// `Node::HighPass`'s own hidden 2-field carry (`y1`, `prev_real`) —
    /// mirrors hand `highpass::State`'s own fields. `period` rides alongside
    /// (the `Ema`/`Wilders` convention) so a restorer can recompute `(a1, a2)`
    /// via `highpass::multiplier`/`multipliers::highpass_multiplier` without
    /// a second storage path for the coefficients.
    HighPass {
        y1: f64,
        prev_real: f64,
        period: u32,
    },
    /// `Node::SuperSmoother`'s own hidden 3-field carry (`y1`, `y2`,
    /// `prev_real`) — mirrors hand `supersmoother::State`'s own fields, same
    /// `period`-alongside convention as [`ChainState::HighPass`].
    SuperSmoother {
        y1: f64,
        y2: f64,
        prev_real: f64,
        period: u32,
    },
    /// The shared 7-tap ring `Node::HtInPhase`/`Node::HtQuadrature` co-own
    /// over one `src` — chronological order (oldest first), exactly 7
    /// elements; rebuild via `FixedRingBuffer::new()` + 7 sequential
    /// `.push()` calls + `.into_full()` (mirrors `ChainState::Hma`'s
    /// `ring`/`cursor` reconstruction idiom, minus the cursor: the hand's
    /// `FixedRingBuffer` carries no separate cursor field).
    HtRing { ring: Vec<f64> },
}

/// A primitive the node graph can contain.
/// Which fold spelling a `Node::VariableAlpha` uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlphaShape {
    /// kama-style
    Ema,
    /// vidya-style
    Vidya,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Node {
    Input {
        stream: Id,
    },
    Ema {
        src: Id,
        period: Id,
    },
    /// EMA recurrence whose SEED and FOLD source differ: bar `feed`
    /// (leads[seed] or explicit override) seeds straight from `seed`'s own
    /// value, every later bar folds `src`'s value with the ordinary EMA step.
    /// `trvi`'s hand seeds from `high[0]-low[0]` (the bar-0 range) then folds
    /// the real true-range `tr` from bar 1 on -- `tr` itself is only valid
    /// from its own structural lead (the `Lag{close,1}` inside it), one bar
    /// LATER than the plain range's lead 0, so a bare `Ema{src: tr, ..}`
    /// would seed from `tr[1]` (wrong) instead of the hand's bar-0 range.
    /// `zlema` seeds from `real[lag-1]`, where `lag-1` is known at recipe-build
    /// time but the raw `Input{stream: 0}` node's own lead is 0, so it needs an
    /// explicit feed-bar override. Ordinary `Ema{src,period}` is the special case
    /// `seed == src`.
    SeededEma {
        seed: Id,
        src: Id,
        period: Id,
        /// When `Some(bar)`, overrides the seed bar to an explicit literal bar
        /// index instead of `leads[seed]` — needed when the seed must be read at a
        /// bar LATER than the seed node's own natural lead (e.g. `zlema` seeds
        /// `real[lag-1]` where `lag-1` is known at recipe-build time but `real`'s
        /// own lead is 0).
        feed: Option<u32>,
    },
    /// Variable alpha recurrence: carry' = carry*(1-w) + value*w, with w computed live per bar.
    /// Two spellings are supported via the `shape` field:
    /// - Ema shape (kama-style): carry.mul_add(1.0 - w, value * w)
    ///   where kama computes w = smoothing_constant.powi(2) with
    ///   smoothing_constant = (fast_ema - slow_ema).mul_add(ef, slow_ema);
    /// - Vidya shape: (value - carry).mul_add(w, carry)
    ///   where vidya computes w = (short_stddev / long_stddev) * alpha.
    VariableAlpha {
        seed: Id,
        src: Id,  // the "value" input
        mult: Id, // node whose live value each bar is blend weight w
        feed: Option<u32>,
        shape: AlphaShape,
    },
    Sub {
        a: Id,
        b: Id,
    },
    Add {
        a: Id,
        b: Id,
    },
    Mul {
        a: Id,
        b: Id,
    },
    Div {
        a: Id,
        b: Id,
    },
    Abs {
        src: Id,
    },
    Lag {
        src: Id,
        n: u32,
    },
    Max {
        a: Id,
        b: Id,
    },
    /// IEEE minNum, the exact mirror of [`Node::Max`] — it returns the OTHER
    /// operand when one is NaN. The hand's `low.min(prev_close)` (ultosc's
    /// true-low) is this and NOT a select: `WhereLt` pins `0.0` on one arm, so
    /// the two disagree on NaN and on the `-0.0`/`+0.0` tie.
    Min {
        a: Id,
        b: Id,
    },
    WhereGe {
        a: Id,
        b: Id,
        t: Id,
    },
    WhereLt {
        a: Id,
        b: Id,
        t: Id,
    },
    /// `if a > b { t } else { 0.0 }` — the strict sibling of [`Node::WhereGe`].
    WhereGt {
        a: Id,
        b: Id,
        t: Id,
    },
    /// `if a <= b { 0.0 } else { t }` — the [`Node::WhereLt`] sibling that keeps
    /// the value on the STRICT inverse of the guard. `ad`'s hand guard is
    /// `if range <= eps { return self.ad }` — an EARLY RETURN, so the compute
    /// branch is `!(a <= b)`: a NaN `range` makes `<=` false, the value arm
    /// runs and poisons the carry. `WhereLt` cannot say this (its value arm is
    /// `>=`, and the `a == b` boundary lands on the wrong side) and `WhereGt`
    /// gets the polarity backwards — it pins `0.0` on a NaN compare, silently
    /// UN-poisoning. This is the polarity the hand actually has.
    WhereLe {
        a: Id,
        b: Id,
        t: Id,
    },
    /// A single-rounding running total: `st[k] = a.mul_add(b, st[k])`, zero
    /// init, feeding from `feeds[k]` like the other recurrences, with NO
    /// weights. `Iir { a: 1.0, b: 1.0 }` over a `Mul` node is NOT the same math
    /// — it rounds the product before adding it to the carry, and an unbounded
    /// running total never washes the difference out. `Fma` cannot substitute
    /// either: its `c` names a node, not this node's own carry.
    /// Hand `ad.rs`: `ratio.mul_add(volume, self.ad)`.
    Accum {
        a: Id,
        b: Id,
    },
    Hma {
        src: Id,
        period: Id,
    },

    Wilders {
        src: Id,
        period: Id,
        composite: bool,
        feed: bool,
    },
    Wma {
        src: Id,
        period: Id,
    },
    SuperTrend {
        atr: Id,
        med: Id,
        close: Id,
        period: Id,
        step_bits: u64,
    },
    Sma {
        src: Id,
        period: Id,
    },
    Stddev {
        src: Id,
        period: Id,
        /// When `Some(bar)`, the node ALSO publishes a one-time "naive" two-moment
        /// stat at that absolute bar index, computed from the accumulated
        /// `sum`/`sum_sq` BEFORE any roll (evict/add) step has run -- used by
        /// `vidya`, whose first published value needs the hand's naive stddev
        /// over `real[0..=long_period-1]` one bar earlier than the ordinary
        /// roll would otherwise become valid. `None` (every other caller)
        /// preserves the original behavior exactly: 0.0 until bar `period`.
        feed: Option<u32>,
        /// When `Some(bar)`, bars in `[period, bar)` use the hand's PLAIN
        /// (non-`mul_add`, no epsilon floor) roll arithmetic instead of the
        /// ordinary `StddevState::calc` association -- `vidya`'s `short_stddev`
        /// stays in this "naive roll" shape from `short_period` through
        /// `long_period - 1` because `vidya.rs::init_state`'s own loop (not
        /// `StddevState::calc`) computes it, and only switches to the real
        /// rolling association once `cycle()` takes over at `bar`. `None`
        /// preserves the original behavior exactly (always `mul_add` + ε).
        switch: Option<u32>,
    },
    RollSum {
        src: Id,
        period: u32,
        mean: bool,
        skip: u32,
        steady_from: u32,
    },
    RollMax {
        src: Id,
        period: u32,
        look_back: u32,
    },
    RollMin {
        src: Id,
        period: u32,
        look_back: u32,
    },
    Trail {
        src: Id,
    },
    Mirror {
        src: Id,
    },
    Fma {
        a: Id,
        b: Id,
        c: Id,
    },
    Sqrt {
        src: Id,
    },
    Ln {
        src: Id,
    },
    Iir {
        src: Id,
        a_bits: u64,
        b_bits: u64,
    },
    IirClamped {
        src: Id,
        a_bits: u64,
        b_bits: u64,
        hi_t: Id,
        hi_v: Id,
        lo_t: Id,
        lo_v: Id,
    },
    /// Plain IIR clamped recurrence: `nv = a * src + b * prev` (separate mul/add,
    /// NO fused mul_add), then clamp with the same thresholds and values as
    /// [`Node::IirClamped`]. Used by ccfisher val1 to match hand's plain math.
    IirClampedPlain {
        src: Id,
        a_bits: u64,
        b_bits: u64,
        hi_t: Id,
        hi_v: Id,
        lo_t: Id,
        lo_v: Id,
    },
    Prev {
        src: Id,
    },
    /// Ring-backed age roll (publish src delayed by n bars, backed by a ring
    /// of capacity n). The published value is the raw evicted/age-read from
    /// the ring. Same group protocol as RollSum.
    RollAge {
        src: Id,
        n: Id,
    },
    /// Negative Volume Index recurrence: a gated multiplicative prefix
    /// product seeded 1000.0, `nvi = close/prev_close * nvi` only when
    /// `volume < prev_volume`, held flat otherwise. The gate and the
    /// multiplier are both NODES, so it hoists like `Prev`/`Accum` and stops
    /// there -- scalar-only, no SIMD form.
    NviRecur {
        src: Id,
        prev_close: Id,
        volume: Id,
        prev_volume: Id,
    },
    /// Positive Volume Index recurrence -- the mirror gate
    /// (`volume > prev_volume`). See [`Node::NviRecur`].
    PviRecur {
        src: Id,
        prev_close: Id,
        volume: Id,
        prev_volume: Id,
    },
    /// Mean Absolute Deviation rescan: `sum(|x - sma|) / period` over the
    /// window ending at this bar. NOT incrementally updatable (unlike
    /// RollSum) because `|x - mean|` isn't linear -- every bar needs a full
    /// rescan with the CURRENT bar's sma. `sma` is an explicit shared
    /// `RollSum{mean:true}` operand (not an internally recomputed copy), so
    /// `md`/`cci`'s mean-of-window math is read off ONE carried sum.
    MeanAbsDev {
        src: Id,
        period: u32,
        sma: Id,
    },
    /// `linreg`'s sliding-moment carry. Steady shape, one arm for both of the
    /// hand's sums: `publish = st + src·mult`, then `st = publish − sub`, with
    /// `mult` 1.0 for `sum_y` and `period` for `sum_xy`. `weighted` picks which
    /// and is a VALUES fact (it becomes `Values::mults`, like any other
    /// period): both spellings emit the identical `st + src·m`, and the two
    /// instances already differ in `b`, so it is NOT structure.
    ///
    /// It exists because the hand publishes BEFORE it evicts:
    /// `sum_y += value` … publish … `sum_y -= prev_value`. `RollSum` cannot
    /// spell that — its steady region is `st += x − leave` and its init region
    /// `(st + x) − leave`, both of which fold the leave into the PUBLISHED
    /// value. Measured against the hand module: this shape is bit-exact on
    /// every bar of 24 series×period cells (ramp/flat/wiggle/noise, p 2..60);
    /// a `RollSum` window breaks 697/698, 685/686 and 672/673 of them. Flat
    /// series hide that entirely, which is why the gate must not be flat-only.
    ///
    /// The `w = min(t, mult)` fold also replaces the seam rescan this node was
    /// going to need: the hand seeds `sum_xy += (i+1)·real[1+i]`, and `i+1` IS
    /// that value's absolute bar index, so bars below the lead accumulate with
    /// a bar-index weight capped at `mult`, and `min(t, mult) == mult` once
    /// `t >= period` — so the steady arm needs no clamp and no branch. Bar 0
    /// contributes `src · 0.0`, which is the hand never reading `real[0]` (it
    /// works on `real[1..]`) — SOUND ONLY WHILE BAR 0 IS FINITE, the parity
    /// matrices' contract (`inf·0.0`/`NaN·0.0` would poison the carry where the
    /// hand stays clean). No hand-module helper, no second region, one `st`
    /// cell per instance.
    ///
    /// The two instances chain: `sum_y`'s `sub` is `Lag(real, period − 1)` (the
    /// evicting bar — a column, so `Lag` applies), while `sum_xy`'s `sub` is
    /// the `sum_y` NODE published the same bar. Scalar-only: `sub` is a slot
    /// read and the fold carries `st`, so there is no lane form (cf.
    /// `NviRecur`/`KvoFlow`).
    LinregFlow {
        src: Id,
        sub: Id,
        period: u32,
        weighted: bool,
    },
    /// Bespoke KVO flow machine: a 4-field-state recurrence (trend flip-flop,
    /// cm accumulator, prev hlc, prev hl) that cannot be expressed with
    /// existing node kinds. Output is the signed volume-force line `vf`;
    /// the two EMAs and their difference are separate nodes downstream.
    KvoFlow {
        high: Id,
        low: Id,
        close: Id,
        volume: Id,
    },
    Const {
        bits: u64,
    },
    HighPass {
        input: Id,
        period: Id,
    },
    SuperSmoother {
        src: Id,
        period: Id,
    },
    /// Cybercycle recurrence (`K_CYBERCYCLE`): a two-pole high-pass IIR over
    /// a 6-tap weighted smooth. The hand's `State::init_state` seeds bars 0–5
    /// with the second-difference formula for cycle_prev/cycle_prev2, and the
    /// steady recurrence is:
    ///   cycle = coef*(smooth - 2*smooth[1] + smooth[2]) + d1*cycle_prev - d2*cycle_prev2
    /// where `coef,d1,d2` come from `multiplier(alpha)`.
    ///
    /// Warmup: bars 0–5 follow the hand's init_state path exactly (seeded cycle).
    /// First published bar is 6 (`min_data=7`).
    ///
    /// Scalar-only: no SIMD form exists in the hand module.
    Cybercycle {
        src: Id,
        alpha_bits: u64,
    },
    /// Peak decay envelope for trendmode/ccfisher: `peak = max(prev_peak * decay, src)` with zero init.
    /// `decay` is encoded as bits from f64 0.991 (fixed). Scalar-only, a=src (raw stream), carry in st[k].
    PeakDecay {
        src: Id,
        decay_bits: u64,
    },
    HtInPhase {
        src: Id,
    },
    HtQuadrature {
        src: Id,
    },
    /// Parabolic SAR (`K_PSAR`): a bespoke 3-field carry (`psar`/`extream`/
    /// `accel`) plus a trend flip-flop bool, keyed off two raw price streams
    /// (`high`/`low`) with NO intermediate nodes -- unlike `SuperTrend`, which
    /// grafts an ATR chain, `psar` reads `high`/`low` directly every bar. The
    /// seed bar (absolute bar 1) uses a DIFFERENT formula than every bar after
    /// it (the hand's `State::new`/`calc` at i=1 skip the lookback-2 clamp that
    /// `calc_unchecked` applies from bar 2 on) -- see `warmup.rs`'s `K_PSAR` arm
    /// for the full transcription. `af_step_bits`/`max_af_bits` are bits-encoded
    /// f64 options, the same convention `SuperTrend::step_bits` uses.
    Psar {
        high: Id,
        low: Id,
        af_step_bits: u64,
        max_af_bits: u64,
    },
    /// Sliding DFT rotation carry for `msw`'s MESA Sine Wave (`K_SDFTROTATE`):
    /// a coupled 2-state carry `[rp, ip]` in `kvacc[k]`, seeded by a weighted
    /// dot-product over the first `period` bars of `src` against the hand's
    /// own precomputed twiddle tables -- the seed calls
    /// `crate::indicators::msw::calc_rp_ip` VERBATIM (same function the hand
    /// itself calls), so the seed is bit-identical to the hand's
    /// `cycle_msw_sdft` by construction, not by re-derivation. Every bar after
    /// the seed advances by the hand's coupled rotation:
    /// `rp' = wr*rp - wi*ip + (new - old)`, `ip' = wr*ip + wi*rp` (using the
    /// OLD `rp`, a simultaneous-update fold, not sequential) where
    /// `new = src[t]`, `old = src[t - period]` (a raw-stream lag, `src` MUST
    /// be a raw `Node::Input`). `wr`/`wi` = cos/sin(2*PI/period), recipe-time
    /// constants riding `mults`/`invms` (same convention `Psar`'s
    /// `af_step`/`max_af` use). Publishes `sine` (via
    /// `crate::indicators::msw::phase_from_rp_ip`, called verbatim for the
    /// same bit-identical reason as the seed); `lead_sine` is cached in
    /// `kvacc[k][2]` every bar for the companion `Node::SdftLead` to read.
    /// Scalar-only (same standing as `NviRecur`/`Psar` -- the hand's SDFT
    /// state has no SIMD lane form).
    SdftRotate {
        src: Id,
        period: u32,
    },
    /// Companion read of a `Node::SdftRotate`'s cached `lead_sine` at
    /// `kvacc[owner_k][2]`. This is a REAL graph edge to `owner` (not an
    /// aliased arena slot), so topological order guarantees `owner` always
    /// executes first in the same bar's node loop -- `owner`'s `kvacc[owner_k]`
    /// is always freshly written before this reads it. Has no carry of its
    /// own and performs no arithmetic.
    SdftLead {
        owner: Id,
    },
    /// Ehlers Homodyne Discriminator (`homodynediscriminator.rs`), bespoke
    /// non-shared carry (Option 1: dedicated node kind, no reuse of
    /// `HtInPhase`/`HtQuadrature`'s shared-ring codegen — those always run
    /// `gain=1.0`, this needs an adaptive per-bar gain fed back from its own
    /// previous output). `src` must be a raw `Node::Input` stream.
    ///
    /// Per-bar algorithm (hardcoded thresholds, this indicator has
    /// `OPTIONS=0`, nothing to key off):
    /// - `t in [0, 3)`: push `src` into a private 4-cap price ring. Publish 0.
    /// - `t in [3, 9)`: price ring full. `gain = 0.075*period + 0.54`
    ///   (`period` starts at 0.0). `smooth = Hann(price ring ages 0..3)`
    ///   (`(4*age0 + 3*age1 + 2*age2 + age3) * 0.1`, exact FMA grouping:
    ///   `ab = 4.mul_add(age0, 3*age1); cd = 2.mul_add(age2, age3);
    ///   smooth = (ab+cd)*0.1`). Push `smooth` into a private 7-cap smooth
    ///   ring. Publish 0.
    /// - `t in [9, 15)`: smooth ring full. `detrender = kernel(smooth ring,
    ///   gain)` (Q-tap only, the hand discards the center tap here). Push
    ///   into a private 7-cap detrender ring. Publish 0.
    /// - `t in [15, 21)`: detrender ring full. `i1 = detrender ring age 3`
    ///   (direct read, no gain). `q1 = kernel(detrender ring, gain)`. Push
    ///   `i1`/`q1` each into their own private 7-cap ring. Publish 0.
    /// - `t >= 21`: i1/q1 rings full. `j_i = kernel(i1 ring, gain)`,
    ///   `j_q = kernel(q1 ring, gain)`. Run the homodyne discriminator (see
    ///   `hd_apply_discriminator` in `homodynediscriminator.rs`): phase
    ///   rotation (`i2_raw = i1 - j_q`, `q2_raw = q1 + j_i`), two IIR smooths
    ///   (α=0.2) on i2/q2, a Re/Im dot/cross against the PRE-update i2/q2
    ///   carry, two more IIR smooths (α=0.2) on Re/Im against the PRE-update
    ///   Re/Im carry, then a period estimate (`TAU / atan_fast(im/re)`,
    ///   held at the prior period if `im==0.0 || re==0.0`), a ±50%-per-bar
    ///   rate-of-change clamp, an absolute `[6,50]` clamp, one more IIR
    ///   smooth (α=0.2) producing `period`, and a final IIR (weight 0.33)
    ///   producing `smooth_period` — the published value. **This full update
    ///   runs starting at `t=21`, but its result is only PUBLISHED from
    ///   `t=22` on** (`feeds[k] = leads[src] + 22`) — at `t=21` the hand's own
    ///   `State::init_state` warmup loop computes and then discards it, yet
    ///   the six scalar carries it updates still feed `t=22`'s computation,
    ///   so the engine must run the SAME update at `t=21` and simply not
    ///   expose it.
    ///
    /// The kernel formula (`C0=0.0962, C1=0.5769, C2=-0.5769, C3=-0.0962`,
    /// `(C0.mul_add(at0, C1*at2) + C2.mul_add(at4, C3*at6)) * gain`) is
    /// `hilberttransform.rs`'s own `ht_kernel` Q-tap, reused verbatim on this
    /// node's own private rings (never the shared `HtInPhase`/`HtQuadrature`
    /// ring group — those are always `gain=1.0`, structurally different).
    ///
    /// Carry storage is a bespoke, non-shared flat `[f64; 38]` scratch cell
    /// per node (`hdacc[k]`, threaded through `exec`/codegen the same way
    /// `kvacc`/`hacc` are) — NOT the generic `RingSet`/`ring_of` group
    /// mechanism, because this node owns five independent private rings
    /// plus six scalars that are never shared with any other node. See
    /// `homodynediscriminator.rs`'s `hd_ring_push`/`hd_ring_at`/`hd_kernel`/
    /// `hd_apply_discriminator`/`hd_step` for the exact shared primitives
    /// both `warmup.rs`'s interpreter and `codegen.rs`'s steady loop call.
    ///
    /// Scalar-only: `lane_formula` is `(0, 0)`, same standing as
    /// `NviRecur`/`KvoFlow`/`Psar`/`SdftRotate` — no SIMD lane form.
    HomodyneDiscriminator {
        src: Id,
    },
    /// Ehlers InstantaneousTrendline recurrence -- bespoke 2-pole IIR with
    /// adaptive α computed live per bar from the embedded HomodyneDiscriminator's
    /// SmoothPeriod (DC). `alpha_bits` encodes the coefficients `(c0, c1, c2, d1, d2)`
    /// as f64 bit patterns. Own 3-field carry `[it_prev, it_prev2, smooth[2]]`
    /// in `kvacc[k]`. See `instantaneoustrendline.rs`'s `cycle` and `State::calc`
    /// for the exact recurrence.
    InstantaneousTrendline {
        src: Id,
    },
    /// Adaptive MESA Sine Wave DFT (`adaptivemsw`): computes one bar's
    /// `(sine, lead_sine)` from the raw source stream and a live DC-period
    /// input (`dc`, usually `Node::HomodyneDiscriminator`).
    ///
    /// Per bar at absolute index `t`:
    /// `period = clamp(round(dc), 6, min(50, t+1))`,
    /// `window = src[t+1-period ..= t]` (oldest first),
    /// then DFT dot-products against `msw::twiddles_for_period(period)` and
    /// `msw::dot_product_simd::<8>`, finally
    /// `msw::phase_from_rp_ip(rp, ip)`.
    ///
    /// Publishes `sine` and caches `lead_sine` in `kvacc[k][2]` for
    /// `Node::SdftLead { owner: this }`.
    ///
    /// Scalar-only (same standing as `SdftRotate`/`HomodyneDiscriminator`):
    /// each bar reads a variable-length raw stream window.
    AdaptiveMsw {
        src: Id,
        dc: Id,
    },
    Mama {
        src: Id,
        fast_limit_bits: u64,
        slow_limit_bits: u64,
    },
    Fama {
        src: Id, // Points to Mama's slot
    },
}

/// Where a sink's value comes from each bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkSrc {
    Interface(Id),
}

/// One output sink: its source, and the absolute bar it starts printing at.
#[derive(Debug, Clone, Copy)]
pub struct Sink {
    pub src: SinkSrc,
    pub lead: Id,
}

impl Sink {
    pub fn iface(id: Id) -> Self {
        Sink {
            src: SinkSrc::Interface(id),
            lead: 0,
        }
    }
    pub fn iface_lead(id: Id, lead: Id) -> Self {
        Sink {
            src: SinkSrc::Interface(id),
            lead,
        }
    }
}

/// Handle to a sub-recipe's nodes after grafting.
#[derive(Debug, Clone)]
pub struct Grafted {
    pub primary: Id,
    pub outputs: Vec<Id>,
    pub optional: Vec<Id>,
}

impl Grafted {
    pub fn optional_at(&self, i: usize) -> Id {
        self.optional
            .get(i)
            .copied()
            .unwrap_or_else(|| panic!("cse: sub-recipe has no optional output #{i}"))
    }
}

/// What a recipe's input slot CONSUMES, so `merge` can intern `Node::Input`
/// by meaning instead of by position. Two recipes that both read close then
/// share one arena node (and one per-bar load); a price recipe's `Input{0}`
/// can never be interned with an OHLC recipe's `Input{0}`, which is exactly
/// the silent wrong-column bug this replaces.
///
/// Declared in CANONICAL INTERN ORDER. A column's global stream index is its
/// rank among the columns PRESENT in the request, so the layout does not depend
/// on recipe order: `Apo + Atr` and `Atr + Apo` erase to the SAME `Signature`.
/// Order-dependent (insertion-based) indices would split the registry cache on
/// request order and silently double-compile plans.
///
/// This order also reproduces every existing pattern's layout exactly: a price
/// recipe (`[Close]`) still gets stream 0, `Atr`'s `[High, Low, Close]` still
/// gets 0/1/2, and `Ad`'s `[…, Volume]` still gets 3.
///
/// **The variants are declared in the hand's own `INPUTS` order** —
/// `open, high, low, close, volume` — so a caller can bind the vectors it
/// destructures straight from its input array (`let [open, high, low, close] =
/// *inputs;`) and have them land on the slots the kernel reads. Canonical rank
/// that disagrees with the hand's convention would force every OHLC caller to
/// re-order its vectors to match the engine, which is backwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Column {
    Open,
    High,
    Low,
    Close,
    Volume,
}

/// A canonical request: the indicator's node graph + sink wiring for one invocation.
#[derive(Debug, Clone)]
pub struct Recipe {
    pub nodes: Vec<Node>,
    /// Display-only role tags, parallel to `nodes` (first non-`None` writer
    /// wins, see [`Recipe::tag`]). NEVER part of an intern key, the merged
    /// graph identity, or any `Signature` — they exist so the `kernel!`
    /// emitter can name generated locals (`ema_short`, `signal`, `tr`) from
    /// the recipe data itself instead of a transcribed table.
    pub tags: Vec<Option<&'static str>>,
    /// What each input slot consumes, indexed by the `stream` field of
    /// [`Node::Input`] — so `columns[0]` is what `Input { stream: 0 }` reads.
    /// REQUIRED on any recipe that binds an input: [`crate::cse::merge`] rejects
    /// one that does not, because positional interning would let a price
    /// recipe's `Input{0}` (= close) intern onto an OHLC recipe's `Input{0}`
    /// (= high) — wrong values behind a green `SIG_*` test. Build with
    /// [`Recipe::price`]/[`Recipe::hl`]/[`Recipe::hlc`]/[`Recipe::hlcvol`] so the
    /// compiler supplies it. It MUST cover every slot the recipe binds.
    pub columns: Vec<Column>,
    pub kind: &'static str,
    pub options: Vec<f64>,
    pub tail_inputs: Vec<Id>,
    /// The restorer field is removed in Phase 1; runtime-side table lookup replaces it.
    pub outputs: Vec<Sink>,
    pub optional_outputs: Vec<Sink>,
}

impl Recipe {
    /// Price-sourced recipe: its only input slot is `Input{0}` = **close**.
    pub fn price() -> Self {
        Self::with_columns(vec![Column::Close])
    }
    /// High/low recipe (`medprice`, `ao`, `fisher`, `mass`, `aroon`, …).
    pub fn hl() -> Self {
        Self::with_columns(vec![Column::High, Column::Low])
    }
    /// OHLC recipe (`atr`, `stoch`, `willr`, `ultosc`, `tr`, …).
    pub fn hlc() -> Self {
        Self::with_columns(vec![Column::High, Column::Low, Column::Close])
    }
    /// OHLC + volume recipe (`ad`, `adosc`).
    pub fn hlcvol() -> Self {
        Self::with_columns(vec![
            Column::High,
            Column::Low,
            Column::Close,
            Column::Volume,
        ])
    }
    /// High/low + volume recipe (`marketfi`, `emv`) — the volume-ratio indicators that
    /// read neither open nor close. Canonical rank gives high=0, low=1, volume=2.
    pub fn hlvol() -> Self {
        Self::with_columns(vec![Column::High, Column::Low, Column::Volume])
    }
    /// Close + volume recipe (`vwma`, `vwap`) — the volume-weighted price averages
    /// that read neither high nor low.
    pub fn cvol() -> Self {
        Self::with_columns(vec![Column::Close, Column::Volume])
    }
    /// Volume-only recipe (`vosc`, `obv`, `nvi`, `pvi`, `mfi`, `wad`, `cmf`) — the
    /// indicators whose single input is volume, with NO price column at all.
    pub fn vol() -> Self {
        Self::with_columns(vec![Column::Volume])
    }
    /// Open + close recipe (`qstick`) — the two-column pair that reads neither high
    /// nor low. Canonical rank gives open=0, close=1.
    pub fn oc() -> Self {
        Self::with_columns(vec![Column::Open, Column::Close])
    }
    /// Full OHLC recipe (`bop` — the only indicator reading OPEN). Slot order is
    /// the canonical `Column` rank, so open=0, high=1, low=2, close=3.
    pub fn ohlc() -> Self {
        Self::with_columns(vec![Column::Open, Column::High, Column::Low, Column::Close])
    }
    fn with_columns(columns: Vec<Column>) -> Self {
        Recipe {
            columns,
            ..Recipe::default()
        }
    }

    pub fn add(&mut self, node: Node) -> Id {
        self.nodes.push(node);
        self.tags.push(None);
        (self.nodes.len() - 1) as Id
    }

    /// Tag node `id` with a display role name if it has none yet — exactly
    /// the `get_or_none` semantics the old macro-side planner used, so the
    /// first recipe to declare a shared node owns its readable name.
    pub fn tag(&mut self, id: Id, name: &'static str) {
        if let Some(slot) = self.tags.get_mut(id as usize) {
            if slot.is_none() {
                *slot = Some(name);
            }
        }
    }

    pub fn constant(&mut self, v: f64) -> Id {
        let bits = v.to_bits();
        if let Some(i) = self
            .nodes
            .iter()
            .position(|n| matches!(n, Node::Const { bits: b } if *b == bits))
        {
            return i as Id;
        }
        self.add(Node::Const { bits })
    }

    pub(crate) fn graft(&mut self, sub: Recipe, in_nodes: &[Id]) -> Grafted {
        let mut map: Vec<Id> = vec![0; sub.nodes.len()];
        for (l, node) in sub.nodes.iter().enumerate() {
            map[l] = match *node {
                Node::Input { stream } => in_nodes
                    .get(stream as usize)
                    .copied()
                    .unwrap_or_else(|| self.add(Node::Input { stream })),
                ref other => {
                    let remapped = crate::cse::merge::remap_node(other, &|c: Id| map[c as usize]);
                    let id = self.add(remapped);
                    // role tags ride the graft: a composed sub-recipe's node
                    // keeps its display name in the parent.
                    if let Some(Some(t)) = sub.tags.get(l) {
                        self.tag(id, t);
                    }
                    id
                }
            };
        }
        let iface = |s: &Sink| match s.src {
            SinkSrc::Interface(id) => map[id as usize],
        };
        Grafted {
            primary: sub
                .outputs
                .first()
                .map(iface)
                .unwrap_or_else(|| *map.last().unwrap_or(&0)),
            outputs: sub.outputs.iter().map(iface).collect(),
            optional: sub.optional_outputs.iter().map(iface).collect(),
        }
    }

    /// Bind parent stream slots for a sub-recipe graft. Passing an EMPTY slice
    /// in fully-declared mode DERIVES the binding from the columns (see
    /// [`Recipe::bind_columns`]) — the caller then names no index at all.
    fn bind_streams(&mut self, in_streams: &[u32]) -> Vec<Id> {
        in_streams
            .iter()
            .map(|&s| self.add(Node::Input { stream: s }))
            .collect()
    }

    /// The column contract between a parent and a grafted sub-recipe.
    ///
    /// A child's `Input{stream: l}` is indexed by the CHILD's own slot order and
    /// a parent's stream index by the PARENT's, so `in_streams[l]` is a translation
    /// between two independent index spaces — the same conflation `merge` now
    /// refuses for co-requested recipes, reachable inside one recipe here.
    ///
    /// Both sides now ALWAYS declare columns — `merge` rejects an input-binding
    /// recipe that does not — so the early return below is defence in depth, not
    /// a supported mode: it can only be reached by a recipe with no inputs at all.
    /// - `Some(derived)` — the caller passed no indices: bind each child slot to
    ///   the parent slot whose column matches, so `Dm{High, Low}` grafted into
    ///   `{Open, High, Low, Close, Volume}` lands on parent streams 1 and 2.
    /// - `None` — the caller's indices are checked: a child slot bound to a
    ///   parent slot of a DIFFERENT column panics instead of silently reading
    ///   the wrong price vector.
    fn bind_columns(&self, sub: &Recipe, in_streams: &[u32]) -> Option<Vec<u32>> {
        if self.columns.is_empty() || sub.columns.is_empty() {
            return None;
        }
        if in_streams.is_empty() {
            return Some(
                sub.columns
                    .iter()
                    .map(|c| {
                        self.columns
                            .iter()
                            .position(|pc| pc == c)
                            .unwrap_or_else(|| {
                                panic!(
                                    "cse: graft {:?} needs column {:?} but the parent recipe {:?} \
                                 declares only {:?}",
                                    sub.kind, c, self.kind, self.columns
                                )
                            }) as u32
                    })
                    .collect(),
            );
        }
        for (l, col) in sub.columns.iter().enumerate() {
            let Some(&s) = in_streams.get(l) else {
                panic!(
                    "cse: graft {:?} declares column {col:?} for slot {l} but the parent \
                     recipe {:?} passed only {} index(es)",
                    sub.kind,
                    self.kind,
                    in_streams.len()
                );
            };
            let Some(pc) = self.columns.get(s as usize) else {
                panic!(
                    "cse: graft {:?} slot {l} binds parent stream {s}, outside the parent \
                     recipe {:?}'s declared columns {:?}",
                    sub.kind, self.kind, self.columns
                );
            };
            assert_eq!(
                pc, col,
                "cse: graft {:?} slot {l} reads {col:?} but parent stream {s} is {pc:?} — \
                 positional binding onto the wrong price column (parent columns {:?})",
                sub.kind, self.columns
            );
        }
        None
    }

    /// Translate-and-bind: the shared front end of every grafting entry point.
    fn bind_for_graft(&mut self, sub: &Recipe, in_streams: &[u32]) -> Vec<Id> {
        let streams = self.bind_columns(sub, in_streams);
        self.bind_streams(streams.as_deref().unwrap_or(in_streams))
    }

    /// Graft a built sub-recipe, binding its input slots to parent STREAMS.
    ///
    /// This takes a `Recipe`, not an indicator type: `kernel!` expands inside
    /// `tulip_rs_shared`, which cannot name the `Dm`/`Dx`/`Atr` types living in
    /// `tulip_rs`. An `R: Planable` bound therefore made every composing recipe
    /// un-registerable by construction and pinned its graph to the indicator
    /// module. The composed graphs live in `cse::recipes` now and reach each
    /// other as plain sibling calls.
    pub fn sub(&mut self, sub: Recipe, in_streams: &[u32]) -> Id {
        let nodes = self.bind_for_graft(&sub, in_streams);
        self.graft(sub, &nodes).primary
    }

    pub fn sub_graph(&mut self, sub: Recipe, in_streams: &[u32]) -> Grafted {
        let nodes = self.bind_for_graft(&sub, in_streams);
        self.graft(sub, &nodes)
    }

    /// Graft a built sub-recipe whose inputs are already ARENA NODES (not
    /// stream slots) — the composing-recipe form, where the sub-graph consumes
    /// nodes the parent has already built.
    pub fn sub_at(&mut self, sub: Recipe, in_nodes: &[Id]) -> Id {
        self.graft(sub, in_nodes).primary
    }
}

impl Default for Recipe {
    fn default() -> Self {
        Recipe {
            nodes: Vec::new(),
            tags: Vec::new(),
            columns: Vec::new(),
            kind: "",
            options: Vec::new(),
            tail_inputs: Vec::new(),
            outputs: Vec::new(),
            optional_outputs: Vec::new(),
        }
    }
}

/// An indicator that can describe itself as a canonical recipe graph.
pub trait Planable {
    fn recipe(options: &[f64]) -> Recipe;
}
