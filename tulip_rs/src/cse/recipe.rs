//! The recipe *language*: what an indicator request **is** to the planner.
//!
//! A [`Recipe`] is not a program to interpret — it is a **cut-set** through the
//! indicator's math, exactly the split `Apo::calc_cse` made manual:
//!
//! * [`nodes`] — the **interface**: the recurrences the *planner* owns
//!   ([`Node::Input`] streams + [`Node::Ema`] chains). These are CSE-merged
//!   across every request in the [`crate::cse::Request`], so `ema26(close)`
//!   advanced for APO is the same state MACD and PPO read — one authority,
//!   one advance per bar (§5.2/§5.3 unchanged).
//! * [`kind`] + [`tail_inputs`] — the request's **tail**: the indicator's
//!   own math ([`crate::cse::tail::Tail`]) that receives the gated
//!   interface values and produces its outputs (owning any state the
//!   planner does *not* model — until the emitter covers it as a chained
//!   recurrence, which macd's signal now is). The JIT lifts exactly this
//!   arithmetic into the fused loop: the math below the interface never
//!   passes through a node dispatcher.
//! * [`outputs`] / [`optional_outputs`] — sinks: each reads either a tail
//!   output slot or an interface value directly (a passthrough EMA line is a
//!   *mirror*, not a copy through the tail), with an explicit publish lead.
//!
//! [`nodes`]: Recipe::nodes · [`kind`]: Recipe::kind · [`tail_inputs`]: Recipe::tail_inputs
//! [`outputs`]: Recipe::outputs · [`optional_outputs`]: Recipe::optional_outputs
//!
//! SSA discipline is unchanged: a node's inputs are lower ids, so the merged
//! arena is topologically ordered and `merge` remaps local→global ids.

/// Local index of a node inside a [`Recipe`]'s `nodes` vector (SSA order).
pub type Id = u32;

/// A primitive the **interface** can contain. v1 executes `Input` + `Ema`;
/// the arithmetic/filter kinds are vocabulary for future interface chains
/// (e.g. a WMA + shared delay-ring tap for HMA) and the merge/lead rules
/// already cover them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Node {
    /// A named source stream, by input position (`Info::inputs` order;
    /// `0` = close/real).
    Input { stream: Id },
    /// `ema(period)` applied to node `src` — the shareable recurrence.
    Ema { src: Id, period: Id },
    /// `a - b` (future interface glue; not executed by the v1 engine).
    Sub { a: Id, b: Id },
    /// `a * b`.
    Mul { a: Id, b: Id },
    /// `a / b`.
    Div { a: Id, b: Id },
    /// `a.max(b)` — the NaN-propagates-other `f64::max`.
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

/// Where a sink's value comes from each bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkSrc {
    /// Slot `u32` of the request tail's output array (primaries first, then
    /// any optional values the tail itself computes).
    Tail(u32),
    /// Mirror of an interface node's reported value (arena id after merge).
    /// This is how a passthrough optional (`short_ema`, `long_ema`, …)
    /// reaches the output without round-tripping through the tail.
    Interface(Id),
}

/// One output sink: its source, and the absolute bar it starts printing at.
/// `lead = 0` falls back to the source's defined-from lead; an interface
/// mirror's lead is additionally clamped up to the node's defined-from
/// (same rule as `finalize`'s clamp, so publish ≤ defined is safe to write).
#[derive(Debug, Clone, Copy)]
pub struct Sink {
    pub src: SinkSrc,
    pub lead: Id,
}

impl Sink {
    /// Tail-output sink with an explicit publish lead.
    pub fn tail(slot: u32, lead: Id) -> Self {
        Sink {
            src: SinkSrc::Tail(slot),
            lead,
        }
    }
    /// Interface mirror; reporting starts at the node's defined-from lead.
    pub fn iface(id: Id) -> Self {
        Sink {
            src: SinkSrc::Interface(id),
            lead: 0,
        }
    }
    /// Interface mirror with an explicit publish lead (clamped up to the
    /// node's defined-from by the engine).
    pub fn iface_lead(id: Id, lead: Id) -> Self {
        Sink {
            src: SinkSrc::Interface(id),
            lead,
        }
    }
}

/// A canonical request: the interface graph + tail wiring for one indicator
/// invocation, e.g. `Macd::recipe(&[12.0, 26.0, 9.0])`.
#[derive(Debug, Clone)]
pub struct Recipe {
    /// Interface nodes, SSA-ordered (inputs at lower ids). Input + Ema in v1.
    pub nodes: Vec<Node>,
    /// Concrete-state tag for [`crate::cse::JitPlan::get_states`]
    /// (e.g. `"macd"`). Also names the tail math the emitter inlines for
    /// this request (the closed set `"ema" | "apo" | "ppo" | "macd"`; the
    /// [`crate::cse::tail::Tail`] impl in the indicator module is the
    /// reference those CLIF ops mirror). Empty ⇒ interface-only (all sinks
    /// are mirrors; no derived values).
    pub kind: &'static str,
    /// The resolved option values (`[12.0, 26.0]` for `Apo(12, 26)`) — the
    /// tail constructor receives them (macd's signal period, …).
    pub options: Vec<f64>,
    /// Arena (post-merge: global) ids fed to the tail, **in tail argument
    /// order**. SSA order matches the hand `TState::calc` advance order, so
    /// the tail reads exactly what the hand loop did at that bar.
    pub tail_inputs: Vec<Id>,
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
}

impl Default for Recipe {
    fn default() -> Self {
        Recipe {
            nodes: Vec::new(),
            kind: "",
            options: Vec::new(),
            tail_inputs: Vec::new(),
            outputs: Vec::new(),
            optional_outputs: Vec::new(),
        }
    }
}
