//! The recipe *language*: the primitive operator [`Node`]s an indicator lowers
//! into, plus a topologically-ordered [`Recipe`] graph.
//!
//! A node is the unit the planner's CSE pass merges on: two nodes are "the same
//! computation" when their [`Node`] variants (kind + params) are equal *and*
//! their input node ids resolve to the same merged nodes. So [`Node`] derives
//! `Eq + Hash` and forms the CSE cache key (see `EXECUTION_PLANNER.md` §5.2).
//!
//! Node indices ([`Id`]) are **local to a single [`Recipe`]** (SSA order: a
//! node's inputs always appear before it). Cross-indicator sharing happens when
//! the planner re-interns local ids into a global arena — the recipe itself
//! stays simple and per-indicator.
//!
//! # Execution parity note (the bit-safety doctrine)
//! The recipe encodes *what* the math is (the graph shape); it does not itself
//! execute. Reproducing the existing hand-written `::indicator()` output
//! bit-for-bit when a merged recipe is lowered is the planner's job, plus the
//! golden-parity harness (see `MATHEMATICAL_ENGINE.md` §5). Two consequences to
//! respect when extending:
//! - `Ema` must lower to the same `ema::calc` recurrence (its `mul_add` +
//!   `multiplier`/`inv_multiplier` form), not a re-associated variant.
//! - SSA node order is load-bearing: it mirrors the per-bar advance order of the
//!   indicator's existing `TState::calc` (e.g. `sub(short_ema, long_ema)` reads
//!   left-to-right, so `short_ema` is advanced first).

/// Local index of a node inside a [`Recipe`]'s `nodes` vector (SSA order).
pub type Id = u32;

/// A single primitive operator in a canonical recipe graph.
///
/// Mirrors `PlanNode` in `EXECUTION_PLANNER.md` §6 / the operator algebra in
/// `PRIMITIVE_CATALOG.md`. Extend this enum as the planner gains operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Node {
    /// A named source stream, by input position (matches `Info::inputs` order,
    /// e.g. `0` = close/real). The leaves of every recipe graph.
    Input { stream: Id },
    /// P-EMA primitive: `ema(period)` applied to the stream/node `src`.
    Ema { src: Id, period: Id },
    /// `a - b`.
    Sub { a: Id, b: Id },
    /// `a * b`.
    Mul { a: Id, b: Id },
    /// `a / b`.
    Div { a: Id, b: Id },
    /// `a.max(b)` — the NaN-propagates-other `f64::max` (NOT an `if a > b`;
    /// see the interpreter's lowering rule). Used by PPO's `.max(EPSILON)`.
    Max { a: Id, b: Id },
    /// A floating-point constant leaf, stored as its raw `f64::to_bits()` so
    /// `Node` remains `Eq + Hash` (the CSE key). Two `Const`s merge iff same
    /// bits — exactly the bit-identity `Exact` requires (§5). PPO's `eps`/
    /// `100.0`, and `k`-scaled `Mul`/`Div`, all lower through this node.
    Const { bits: u64 },
    /// Ehlers 2-pole high-pass filter. The Roofing Filter's stage 1; modelled as
    /// its own primitive so `roofing = ss( hp( close ) )` is explicit in the
    /// graph, AND so the `highpass` optional output has its own SSA `Id`.
    HighPass { input: Id, period: Id },
    /// Ehlers 2-pole super smoother. The Roofing Filter's stage 2; together with
    /// [`Node::HighPass`] it forms the canonical `P-RF` composite (PRIMITIVE_CATALOG
    /// lists `P-HP`/`P-SS` as their own kinds).
    SuperSmoother { src: Id, period: Id },
    /// 7-tap Hilbert in-phase (centre `buf[3]`) tap of `src`, unit gain. The
    /// `P-HT` coefficients (`C0..C3`) are intrinsic to the kind (Decision P).
    HtInPhase { src: Id },
    /// 7-tap Hilbert quadrature FIR of `src`, unit gain. Shares `src`'s 7-tap
    /// ring buffer with [`Node::HtInPhase`] at execution time.
    HtQuadrature { src: Id },
}

/// A canonical, topologically-ordered graph for one indicator request.
#[derive(Debug, Clone, Default)]
pub struct Recipe {
    /// SSA-ordered nodes; a node's inputs are always at a lower index.
    pub nodes: Vec<Node>,
    /// Node ids exposed as primary outputs, in `Info::outputs` order.
    pub outputs: Vec<Id>,
    /// Node ids exposed as optional outputs, in `Info::optional_outputs` order.
    pub optional_outputs: Vec<Id>,
    /// Per-*sink* publish lead: the absolute bar each output starts printing at
    /// (= number of leading bars it drops = `data_len - capacity`). This is the
    /// per-indicator warm-up **nuance** the arena cannot infer from the graph
    /// (§5.3): a shared node is one stream, but APO may publish it from bar 26
    /// while MACD publishes the same node from bar 25. Empty entry (or short
    /// vec) ⇒ fall back to the node's *defined-from* lead. This scales to every
    /// indicator — window/ring-buffer kinds will declare their own here.
    pub output_leads: Vec<Id>,
    /// Same, for optional outputs, in `optional_outputs` order.
    pub optional_output_leads: Vec<Id>,
    /// Concrete-state tag for [`crate::cse::query::CompiledPlan::get_states`]
    /// (e.g. "macd"). Empty ⇒ `IndicatorStateBox::Other` (no projection yet).
    pub kind: &'static str,
}

impl Recipe {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a node and return its local id (the SSA [`Id`] other nodes use as
    /// input).
    pub fn add(&mut self, node: Node) -> Id {
        self.nodes.push(node);
        (self.nodes.len() - 1) as Id
    }
}
