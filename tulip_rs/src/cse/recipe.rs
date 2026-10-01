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
//!   and the `ema12 − ema26` line APO, MACD and PPO all consume is ONE node
//!   (§5.2/§5.3 unchanged; MACD's signal is an `Ema` over the shared line
//!   node, fed per §5.3 by the same machinery any chained `Ema` uses).
//! * [`tail_inputs`] — the arena ids of the chains the indicator's hand state
//!   holds, in `TState` field order; [`restorer`] rebuilds the concrete hand
//!   `IndicatorState` from them (`get_states`). Both are the ONLY places an
//!   indicator's identity appears outside its own module — the executor emits
//!   node kinds, never indicator names.
//! * [`outputs`] / [`optional_outputs`] — sinks: node references with an
//!   explicit publish lead (hand `slot_lengths`/`output_length` facts; a
//!   passthrough EMA line and a derived oscillator line are the same thing —
//!   a mirror of a node).
//!
//! [`nodes`]: Recipe::nodes · [`tail_inputs`]: Recipe::tail_inputs ·
//! [`restorer`]: Recipe::restorer ·
//! [`outputs`]: Recipe::outputs · [`optional_outputs`]: Recipe::optional_outputs
//!
//! SSA discipline: a node's inputs are lower ids, so the merged arena is
//! topologically ordered and `merge` remaps local→global ids.

/// Local index of a node inside a [`Recipe`]'s `nodes` vector (SSA order).
pub type Id = u32;

/// A resolved state chain handed to a [`StateRestorer`]: one entry per
/// [`Recipe::tail_inputs`] id, in that order. The engine builds these from
/// its OWN `OpE` vocabulary (never indicator identity), so this is not a
/// blindness violation — it is the machine's instruction set surfaced as
/// data. The indicator module pattern-matches the variants back into its
/// concrete hand `State<Warm>` (including a `Buffer` for HMA's diff ring).
///
/// The EMA value / rolling sum lives in the group leader's state word;
/// `sum` here is already resolved through `roll_leader`, so a follower
/// `Wma` sees the same sum its leader rolled.
#[derive(Debug, Clone)]
pub enum ChainState {
    /// A price- or chain-sourced EMA recurrence — one carried value.
    Ema { value: f64, period: u32 },
    /// A rolling-sum SMA — the group `sum` (output = `sum / period`).
    Sma { sum: f64, period: u32 },
    /// Wilder's smoothing — the carried decayed-average `value` (the chain
    /// is `Ema`-shaped; the warm mean is baked in by the time it is warm).
    Wilders { value: f64, period: u32 },
    /// Rolling `sum` + `sum_sq` (the two recurrences hand stddev carries).
    Stddev { sum: f64, sum_sq: f64, period: u32 },
    /// Rolling maximum (E9) — the carried extremum + the `trail` (bars since
    /// its NEWEST occurrence), exactly the hand `max::State{max, trail}`. The
    /// rescan window is NOT state: it is the host anchor (`JitState.window`),
    /// same as a rolling sum's price tail.
    RollMax {
        value: f64,
        trail: u64,
        period: u32,
        look_back: u32,
    },
    /// Rolling minimum (E9) — the mirror of [`ChainState::RollMax`] (hand
    /// `min::State{min, trail}`).
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
    /// HMA's self-contained state: `[sum1, ws1, sum2, ws2, wss, ss]`, the
    /// ring cursor as a BYTE OFFSET (`/8` = slot index), the `psqrt`-slot
    /// DIFF ring in slot order (slot 0 = the fill's `first_diff`), and the
    /// periods needed to recompute the hand's derived constants.
    Hma {
        accs: [f64; 6],
        cursor: u64,
        ring: Vec<f64>,
        period: u32,
        p2: u32,
        psqrt: u32,
    },
}

impl ChainState {
    /// The Ema-family view — rebuilds the hand `ema::State<Warm>` from an
    /// `Ema` chain (used by the ema/apo/ppo/macd restorers). Panics on a
    /// non-`Ema` variant; those indicators' `tail_inputs` only name Emas.
    pub fn as_ema(&self) -> crate::indicators::ema::State<crate::types::Warm> {
        match self {
            ChainState::Ema { value, period } => {
                crate::indicators::ema::State::new(*value, *period as usize).into_warm()
            }
            other => panic!("cse: restorer expected an Ema chain, got {other:?}"),
        }
    }
}

/// Rebuilds the concrete hand `IndicatorState` from the chains named by
/// [`Recipe::tail_inputs`], in that order. Defined in the INDICATOR module —
/// the only per-indicator hook the executor calls without knowing what the
/// indicator is (mirrored into [`crate::cse::merge::Resolved`]).
pub type StateRestorer = fn(Vec<ChainState>) -> crate::cse::IndicatorStateBox;

/// A primitive the node graph can contain. The executor emits `Input`,
/// `Ema`, `Sub`/`Add`/`Mul`/`Div`/`Max`, `Abs`, `WhereGe` (a `fcmp.ge`+select),
/// `Const`, the raw `Lag` (a stream delayed `n` bars off the anchor), the
/// rolling kinds `Sma`/`Stddev`/`Wma` (price-sourced, grouped by
/// `(src, period)`), the trail-rescan `RollMax`/`RollMin` (E9 — price-sourced,
/// carrying a value AND a bar-count trail), the `Wilders` chain (price leaf OR chained over a derived
/// node — `atr = Wilders(tr)`; `composite` mean/sum, `feed` fold-from-head vs
/// fold-from-src-lead), the bespoke rolling `Hma` (E3 stage 3 — its own
/// two-engine + diff-ring state), the bespoke multi-value feedback `SuperTrend`
/// (E7 — 3 f64 + a boolean carried like HMA's run), with `Fma`/`Sqrt` graph
/// glue; the filter/HT kinds stay vocabulary for future chains with
/// merge/lead rules already covering them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Node {
    /// A named source stream, by input position (`Info::inputs` order;
    /// `0` = close/real).
    Input { stream: Id },
    /// `ema(period)` applied to node `src` — the shareable recurrence
    /// (chained over any node; a chained `Ema` feeds on its source's
    /// REPORTED bars, §5.3).
    Ema { src: Id, period: Id },
    /// `a - b`.
    Sub { a: Id, b: Id },
    /// `a + b`.
    Add { a: Id, b: Id },
    /// `a * b`.
    Mul { a: Id, b: Id },
    /// `a / b`.
    Div { a: Id, b: Id },
    /// `|a|` — `f64::abs` (CLIF `fabs`).
    Abs { src: Id },
    /// `src` delayed by `n` bars (raw price stream only — `n >= 1`, source
    /// asserted price-sourced at build). Reads through the same anchor as
    /// the rolling prev-reads (`abases + (pmax − n) + poff`); bars `< n`
    /// CLAMP to the stream's first sample `price[0]` (anchor read at
    /// `lag_base + 8·(n − next_bar)`, correct cold AND after a roll) — NOT
    /// `0.0`. A consumer publishing at lead `>= n` never stores that bar, but
    /// `atr` FOLDS `tr[0] = high[0] − low[0]` into its Wilders seed (since
    /// `low[0] <= close[0] <= high[0]`, the clamp makes `max(h−l, |h−c0|,
    /// |l−c0|) = h0 − l0`, the hand's bar-0 term). E6: the stateless
    /// `tr = max(h−l, |h−pc|, |l−pc|)` (`Lag` = prev close) + atr/dx.
    Lag { src: Id, n: u32 },
    /// `a.max(b)` — the hand's Rust `f64::max` (IEEE **maxNum**: a NaN operand
    /// yields the *other*). Lowered by `codegen::max_num`, NOT CLIF `fmax` —
    /// Cranelift's `fmax` follows the WebAssembly rules (propagates NaN), the
    /// opposite of Rust, which broke `di`/`dx`'s `.max(0.0)` guard on a zero ATR
    /// (review §2.1 / D6).
    Max { a: Id, b: Id },
    /// `if a >= b { t } else { 0.0 }` — an ordered float compare feeding a
    /// CLIF `select`, the else arm pinned to `+0.0`. This is the DM+ / DM−
    /// mutual exclusion (`where(dp0>=dm0, max(dp0,0), 0)`) expressed with no
    /// `-0.0`/NaN hazard: the hand sets the losing arm to a literal `0.0`, so
    /// masking with `×0.0` would risk `-0.0` (finite negative × `+0.0`) or NaN
    /// and break bit-exactness; a real `select` cannot. NaN operands make the
    /// ordered `>=` false → `0.0` (Rust `>=` is identical, and prices are
    /// finite so it never fires).
    WhereGe { a: Id, b: Id, t: Id },
    /// `hma(period)` over node `src` — hand `hma.rs` as ONE bespoke rolling
    /// op (E3 stage 3): two private WMA engines (P and P/2 — the second's
    /// init window is OFFSET `[P−P/2 .. P−1]`, so it cannot share a
    /// standalone Wma group), the `2·fast − slow` diff series, and a ring
    /// of the last `√P` (truncated) DIFF values — the engine's only
    /// legitimately STORED calculation buffer (hand `prev_diff: Buffer`).
    /// Output `weighted_sumsqrt / (psqrt·(psqrt+1)/2)` as true division,
    /// the hand's exact order. Price-sourced (asserted at build).
    Hma { src: Id, period: Id },
    /// `wilders(period)` over node `src` — Wilder's smoothing:
    /// seed = **mean of the first `period` values** (`Σ/p`, a true divide),
    /// then the steady IIR `w = w·(p−1)/p + v·(1/p)`. One carried chain
    /// value (like `Ema`), NO history/anchor read (only the current `v`).
    /// `src` may be a PRICE stream (E6 leaf: standalone `wilders`) OR a
    /// derived node — `atr = Wilders(tr)` chains over the tr subgraph. A
    /// chained `Wilders` folds the source's first `period` NODE-VALUES from
    /// bar 0 (lead = `period−1` regardless of `lead(src)`; unlike a chained
    /// `Ema`, which feeds on its source's REPORTED bars), so `atr`'s bar-0
    /// seed `tr[0] = h0 − l0` (the `Lag` clamp) is included bit-exactly.
    /// `composite` switches to the hand's `partial_calc` path used by the
    /// DI/DX/DM chains: NO `÷period` at the seam (the seed is the raw sum
    /// `Σ`) and the steady step is `w = w·(p−1)/p + v` (no `·(1/p)`), so the
    /// accumulator grows like a running SUM, not a decayed mean. `feed` gates
    /// WHEN folding starts: `false` folds the source's node-values from bar 0
    /// (the head — correct for the price leaf and every chain whose source has
    /// a defined bar-0 value, incl. `atr`'s clamped `tr[0]`); `true` starts
    /// folding only at the source's PUBLISH lead (feeds on its REPORTED values,
    /// like a chained `Ema`) — this is `adx = Wilders(dx)`, whose `dx` is
    /// meaningless until its own warmup ends at bar `period−1`.
    Wilders {
        src: Id,
        period: Id,
        composite: bool,
        feed: bool,
    },
    /// `wma(period)` over node `src` — the hand's two-line trick
    /// (`ws −= sum_old; sum rolls; ws += src·P; out = ws · 2/(P(P+1))`,
    /// reciprocal multiply as the hand computes it). Warm bars fold
    /// `ws += src·(i+1)` exactly like `init_state`; the raw sum — not the
    /// SMA line — is what the trick needs, so no `×1/P` multiply happens
    /// unless an `Sma`/`Stddev` sibling demands it.
    Wma { src: Id, period: Id },
    /// `supertrend(period, step)` — the hand's stateful feedback band
    /// (E7), the engine's first MULTI-VALUE chain (mirrors hand
    /// `State` field types exactly: `prev_st`/`prev_ub`/`prev_lb: f64` +
    /// `trend: bool`). Reads three upstream NODES — `atr` (the shared mean
    /// `Wilders(tr)`), `med` (the shared `0.5·(high+low)` medprice), `close`
    /// — so a `natr`+`supertrend` query dedups the WHOLE atr chain (one
    /// level up from `tr`), and every H/L chart dedups `med` for free. Per
    /// bar (band half-width `w = step·atr`, `med±w`):
    /// `trend = (close>prev_st) | (trend & !(close<prev_st))`;
    /// `st = trend ? max(prev_lb, med−w) : min(prev_ub, med+w)`; the two
    /// non-chosen bands are still re-seeded raw (hand's `ub`/`lb`), so all
    /// three f64 + the latch persist across bars. `period`/`step` are
    /// literals (like `Wilders.period`), `step` carried as raw bits (like
    /// `Const`) so `Node` stays `Hash + Eq`. Defined-from bar `period−1` (the
    /// atr seam = the single seed `calc_st`), publish `period` (hand
    /// `min_data = period+1`).
    SuperTrend {
        atr: Id,
        med: Id,
        close: Id,
        period: Id,
        step_bits: u64,
    },
    /// `sma(period)` over node `src` — the rolling-sum recurrence
    /// (`sum += src[i] − src[i−P]`, output `sum/P`). The engine keeps a
    /// ring of the last `P` src values in the state image (for a
    /// price-sourced SMA the ring mirrors the hand `IndicatorState`'s
    /// price tail); emittable today for `src` = a price stream.
    Sma { src: Id, period: Id },
    /// `stddev(period)` over node `src` — hand `Stddev::calc` exactly:
    /// rolling `sum` and `sum_sq` (`fma` fold, like the hand's init),
    /// `sd = sqrt(sum_sq/P − sma·sma).max(f64::EPSILON)`.
    Stddev { src: Id, period: Id },
    /// Rolling **maximum** over node `src` — hand `max.rs` (E9), the
    /// trail-rescan recurrence (a windowed `Max` node, not a fold). Carries
    /// TWO values: the current max (f64) and the `trail` — bars since the
    /// NEWEST occurrence of that max (i64). Per bar: `trail += 1`; if
    /// `period <= trail` the window `[i − look_back ..= i]` is rescanned
    /// (newest-occurrence index ⇒ `trail = look_back − idx`); else the
    /// carried value is kept, reset to this bar with `trail = 0` when
    /// `src[i] >= max`. Tie-breaking is part of the contract: `aroon`
    /// consumes the *trail*, so the index must be the newest occurrence
    /// (`rescan_scalar_and_simd_forms_agree_on_extremum_and_index` proves the
    /// hand's SIMD and scalar forms agree, which is what lets the emitter pick
    /// either).
    ///
    /// `look_back` is a SECOND, independent parameter because the family
    /// genuinely needs both shapes: standalone `max`/`min` use `(P, P−1)` (a
    /// P-bar window) while `aroon` uses `(P, P)` (P+1 bars). Deriving one
    /// from the other would silently merge two different windows into one
    /// arena node.
    ///
    /// The rescan reads the host anchor (`abases[src] + 8·(pmax − k) + poff`),
    /// so `src` must be a PRICE stream today — a derived src needs E8's
    /// shift-eval (`Handover` §6.5). No ring words: the window lives in the
    /// same anchor the rolling prev-reads use, and `pmax` widens to
    /// `look_back`. Warm-up mirrors the hand's `init_state`: bar 0 seeds the
    /// value (`trail = 0`), bars `1..lead` fold the compare-and-reset branch
    /// only (no rescan is reachable before bar `period`). Defined-from =
    /// `lead(src) + period − 1`; standalone `max`/`min` publish at that lead,
    /// `aroon` publishes at `period` (the hand's `cycle_*` start bars).
    /// `RollMin` is the mirror image with `<=`.
    RollMax {
        src: Id,
        period: u32,
        look_back: u32,
    },
    /// Rolling **minimum** over node `src` — the mirror of
    /// [`Node::RollMax`] (hand `min.rs`: `trail += 1`, rescan when
    /// `period <= trail`, else reset when `src[i] <= min`). NOTE the hand
    /// asymmetry the engine deliberately does NOT copy: `Min::indicator`
    /// seeds with `period` bars while `Max::indicator` uses `period − 1`, so
    /// the min side re-processes its first printed bar and its trail runs one
    /// long. That is invisible in the printed value (an oversized trail only
    /// makes the rescan fire a bar early, and the rescan recomputes the exact
    /// window extremum), so the planner uses the uniform rule for both and
    /// `jit_min_matches_hand` pins the equivalence.
    RollMin {
        src: Id,
        period: u32,
        look_back: u32,
    },
    /// The [`RollMax`]/[`RollMin`] `src`'s carried **trail** — bars since the
    /// NEWEST occurrence of the extremum — projected as an f64 (CLIF
    /// `fcvt_from_uint`, the hand's `usize as f64`). This is what makes the
    /// trail a *graph value* instead of a hidden second output: `aroon` is
    /// `(period − trail) · (100/period)`, so its two lines are ordinary
    /// `Sub`/`Mul` nodes over a projection, and two requests asking for aroon
    /// and aroonosc share the rolls, the trails AND the scaled lines exactly
    /// like any other identical node. Asserted to reference a roll kind at
    /// build; defined-from = its roll's (`lead(src)`).
    ///
    /// [`RollMax`]: Node::RollMax · [`RollMin`]: Node::RollMin
    Trail { src: Id },
    /// `a * b + c` in ONE rounding — `f64::mul_add` as the hand writes it
    /// (bbands' `std_dev.mul_add(sd, sma)`; never an un-fused mul+add).
    Fma { a: Id, b: Id, c: Id },
    /// `sqrt(a)` — IEEE exact square root, `f64::sqrt`.
    Sqrt { src: Id },
    /// A floating-point constant leaf, stored as `f64::to_bits()` so `Node`
    /// stays `Eq + Hash` (the CSE key).
    Const { bits: u64 },
    /// Ehlers 2-pole high-pass filter (not executed by the v1 engine; the
    /// Hilbert recipe carries it as data).
    HighPass { input: Id, period: Id },
    /// Ehlers 2-pole super smoother (not executed by the v1 engine).
    SuperSmoother { src: Id, period: Id },
    /// 7-tap Hilbert in-phase tap (not executed by the v1 engine).
    HtInPhase { src: Id },
    /// 7-tap Hilbert quadrature tap (not executed by the v1 engine).
    HtQuadrature { src: Id },
}

/// Where a sink's value comes from each bar: a node reference (after merge:
/// the global arena id). Passthrough EMA lines and derived oscillator lines
/// are the same thing — mirrors of nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkSrc {
    /// Mirror of a node's reported value.
    Interface(Id),
}

/// One output sink: its source, and the absolute bar it starts printing at.
/// `lead = 0` falls back to the source's defined-from lead; the lead is
/// clamped up to the node's defined-from by the engine (publish ≤ defined
/// is safe to write).
#[derive(Debug, Clone, Copy)]
pub struct Sink {
    pub src: SinkSrc,
    pub lead: Id,
}

impl Sink {
    /// Mirror; reporting starts at the node's defined-from lead.
    pub fn iface(id: Id) -> Self {
        Sink {
            src: SinkSrc::Interface(id),
            lead: 0,
        }
    }
    /// Mirror with an explicit publish lead (clamped up to the node's
    /// defined-from by the engine — hand capacities are facts, e.g. the
    /// `Ema`-shaped optionals of apo/ppo/macd publish at `period`).
    pub fn iface_lead(id: Id, lead: Id) -> Self {
        Sink {
            src: SinkSrc::Interface(id),
            lead,
        }
    }
}

/// Handle to a sub-recipe's nodes after [`Recipe::graft`]/`sub_graph`, with
/// every output id already remapped into the PARENT recipe's id space. Only
/// the sub's PRIMARY is meant for direct chaining; the rest are for the rare
/// caller that reuses a sub's optional mirror too (e.g. grafting `atr` and
/// also mirroring its `tr`). `*_locals` are the ORIGINAL sub-local ids (0 for
/// primary, 1 for its tr mirror …), so a caller can address a mirror by a
/// stable per-subscriber convention rather than a positional index.
#[derive(Debug, Clone)]
pub struct Grafted {
    pub primary: Id,
    pub outputs: Vec<Id>,
    pub optional: Vec<Id>,
}

impl Grafted {
    /// The grafted optional at the sub-recipe's `optional_outputs[i]` position
    /// (a sub exposes exactly its own declared optionals, e.g. `atr`'s `[tr]`
    /// → `optional(0)`). Panics if the sub declares no such optional.
    pub fn optional_at(&self, i: usize) -> Id {
        self.optional
            .get(i)
            .copied()
            .unwrap_or_else(|| panic!("cse: sub-recipe has no optional output #{i}"))
    }
}

/// A canonical request: the indicator's node graph + sink wiring for one
/// invocation, e.g. `Macd::recipe(&[12.0, 26.0, 9.0])`.
#[derive(Debug, Clone)]
pub struct Recipe {
    /// The graph, SSA-ordered (inputs at lower ids).
    pub nodes: Vec<Node>,
    /// Debug/label tag for the request (e.g. `"macd"`) — carried through
    /// `merge`/`Resolved` for messages and future plan-cache keys, but the
    /// executor NEVER matches on it: everything the engine needs is in the
    /// nodes, the sinks, and `restorer`.
    pub kind: &'static str,
    /// The resolved option values (`[12.0, 26.0]` for `Apo(12, 26)`) —
    /// labels the request; the graph already carries the values that matter
    /// (periods, constants).
    pub options: Vec<f64>,
    /// Arena (post-merge: global) ids of the `Ema` chains this indicator's
    /// hand `State` holds, **in `TState` field order** — fed verbatim to
    /// [`restorer`] by `get_states`. (Formerly "tail inputs".)
    pub tail_inputs: Vec<Id>,
    /// Rebuilds the concrete hand `IndicatorState` from the chains named by
    /// [`tail_inputs`] — defined in the indicator module. `None` ⇒
    /// [`IndicatorStateBox::Other`].
    ///
    /// [`tail_inputs`]: Recipe::tail_inputs · [`IndicatorStateBox::Other`]: crate::cse::IndicatorStateBox
    pub restorer: Option<StateRestorer>,
    /// Primary sinks, `Info::outputs` order.
    pub outputs: Vec<Sink>,
    /// Optional sinks, `Info::optional_outputs` order (enabled/disabled is a
    /// per-request runtime switch — see [`crate::cse::merge::Resolved`]).
    /// A sub-recipe's optionals are ALSO what it exposes to consumers that
    /// compose it via [`sub_graph`](Self::sub_graph) — a sub-indicator exposes
    /// exactly its own declared optional outputs (e.g. `atr` exposes `tr`),
    /// no separate channel.
    pub optional_outputs: Vec<Sink>,
}

impl Recipe {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an interface node, returning its SSA-local id.
    pub fn add(&mut self, node: Node) -> Id {
        self.nodes.push(node);
        (self.nodes.len() - 1) as Id
    }

    /// Intern-friendly helper: append (or reuse) a `Const` leaf, returning
    /// its local id. Recipes dedup identical constants within themselves;
    /// `merge` dedups them across requests.
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

    /// Graft `sub`'s node graph into `self`, returning the sub's PRIMARY
    /// output as a parent-local id (plus every output id, in `outputs` order).
    ///
    /// `in_streams[k]` is the parent **stream number** the sub's `Input{k}`
    /// (its k-th declared price input) binds to; the sub's own `outputs` /
    /// `optional_outputs` / `tail_inputs` / `restorer` are IGNORED — only the
    /// shared math (nodes) composes; the top-level indicator keeps its own
    /// sinks and state restoration. Structural interning in `merge` then
    /// dedups the grafted nodes against anything already in the arena, so
    /// "if the math matches it is shared" holds automatically — a sub-recipe
    /// is only a shorthand for that sub-indicator's node graph, nothing more.
    pub(crate) fn graft(&mut self, sub: Recipe, in_nodes: &[Id]) -> Grafted {
        // sub-local id -> parent id, computed in SSA order. A sub `Input{k}`
        // binds to `in_nodes[k]` — a parent PRICE stream OR any computed node
        // (e.g. `tr`), so `Atr = Wilders(tr)` composes by binding wilders'
        // input to the tr node. An unbound input falls back to its own stream.
        let mut map: Vec<Id> = vec![0; sub.nodes.len()];
        for (l, node) in sub.nodes.iter().enumerate() {
            map[l] = match *node {
                Node::Input { stream } => in_nodes
                    .get(stream as usize)
                    .copied()
                    .unwrap_or_else(|| self.add(Node::Input { stream })),
                ref other => {
                    let remapped = crate::cse::merge::remap_node(other, &|c: Id| map[c as usize]);
                    self.add(remapped)
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

    /// Bind a sub-recipe's logical inputs to the parent's price streams
    /// `in_streams[k]` (creates one `Input` node each). This is the common
    /// sugar; see [`graft`](Self::graft)/[`sub_at`](Self::sub_at) for binding to
    /// an arbitrary node.
    fn bind_streams(&mut self, in_streams: &[u32]) -> Vec<Id> {
        in_streams
            .iter()
            .map(|&s| self.add(Node::Input { stream: s }))
            .collect()
    }

    /// Compose another [`Planable`]'s math as a sub-recipe over parent price
    /// streams, returning its PRIMARY output id for chaining. See
    /// [`sub_graph`](Self::sub_graph) / [`sub_at`](Self::sub_at) for optionals
    /// and node-bound inputs.
    pub fn sub<R: crate::cse::Planable>(&mut self, options: &[f64], in_streams: &[u32]) -> Id {
        let nodes = self.bind_streams(in_streams);
        self.graft(R::recipe(options), &nodes).primary
    }

    /// Like [`sub`](Self::sub) but returns the full [`Grafted`] handle (the
    /// sub's own `outputs` + `optional_outputs`, remapped).
    pub fn sub_graph<R: crate::cse::Planable>(
        &mut self,
        options: &[f64],
        in_streams: &[u32],
    ) -> Grafted {
        let nodes = self.bind_streams(in_streams);
        self.graft(R::recipe(options), &nodes)
    }

    /// Like [`sub`](Self::sub) but binds the sub's logical input `k` to an
    /// EXISTING parent node `in_nodes[k]` (a stream node, or any computed node
    /// such as `tr`). This is what makes `Atr = Wilders(tr)` a composition, not
    /// a re-emission: bind wilders' price-input to `tr`.
    pub fn sub_at<R: crate::cse::Planable>(&mut self, options: &[f64], in_nodes: &[Id]) -> Id {
        self.graft(R::recipe(options), in_nodes).primary
    }
}

impl Default for Recipe {
    fn default() -> Self {
        Recipe {
            nodes: Vec::new(),
            kind: "",
            options: Vec::new(),
            tail_inputs: Vec::new(),
            restorer: None,
            outputs: Vec::new(),
            optional_outputs: Vec::new(),
        }
    }
}
