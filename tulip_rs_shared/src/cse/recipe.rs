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
}

/// A primitive the node graph can contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Node {
    Input {
        stream: Id,
    },
    Ema {
        src: Id,
        period: Id,
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
    Prev {
        src: Id,
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
    HtInPhase {
        src: Id,
    },
    HtQuadrature {
        src: Id,
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
