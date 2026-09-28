//! CSE: the planner-side vocabulary — the recipe *language* (what an indicator
//! *is*, as a mathematical graph) and the trait that exposes it.
//!
//! This module is deliberately additive and inert: nothing in the runtime
//! indicator path consumes it yet. Each indicator gains a [`Planable::recipe`]
//! describing its decomposition into [`Node`]s; a later planner (more modules
//! here, e.g. `arena`/`plan`) ingests those graphs, hash-conses identical nodes
//! across indicators, and lowers the merged graph into a shared execution plan.
//!
//! See `proposals/EXECUTION_PLANNER.md` §5.1 (recipes as data) and §6 (the
//! `PlanNode` vocabulary this [`Node`] mirrors).

mod columnar;
mod fuse;
mod merge;
mod plan;
mod query;
mod recipe;
mod simd;

pub use merge::{merge, Merged, Resolved};
pub use plan::{exec, finalize, NodeOut};
pub use query::{CompiledPlan, IndicatorStateBox, QueryState, Request, Results};
pub use recipe::{Id, Node, Recipe};
pub use simd::{lane_buckets, sub_extract, sub_reduce, EmaPair, MAX_LANES};

/// An indicator that can describe itself as a canonical recipe graph.
///
/// `options` are the resolved runtime option values (e.g. `[12.0, 26.0]` for
/// `Apo(12, 26)`), exactly as passed to `Indicator::indicator`. The recipe is
/// therefore a *function of the request*, with concrete periods baked into the
/// [`Node`] params (so `Ema{period: 12}` in two different indicators shares).
pub trait Planable {
    fn recipe(options: &[f64]) -> Recipe;
}
