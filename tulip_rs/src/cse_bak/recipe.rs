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

/// Rebuilds the concrete hand `IndicatorState` from the chains named by
/// [`Recipe::tail_inputs`], in that order. Defined in the INDICATOR module —
/// the only per-indicator hook the executor calls without knowing what the
/// indicator is (mirrored into [`crate::cse::merge::Resolved`]).
pub type StateRestorer =
    fn(Vec<crate::indicators::ema::State<crate::types::Warm>>) -> crate::cse::IndicatorStateBox;

/// A primitive the node graph can contain. The v1 executor emits `Input`,
/// `Ema`, `Sub`, `Mul`, `Div`, `Max`, `Const`; the filter/HT kinds are
/// vocabulary for future chains (e.g. a WMA + shared delay-ring tap for HMA)
/// and the merge/lead rules already cover them.
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
    /// `a * b`.
    Mul { a: Id, b: Id },
    /// `a / b`.
    Div { a: Id, b: Id },
    /// `a.max(b)` — the NaN-propagates-other `f64::max` (CLIF `fmax`,
    /// same IEEE maxNum semantics).
    Max { a: Id, b: Id },
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
