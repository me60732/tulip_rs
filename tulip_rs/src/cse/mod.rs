//! CSE: the planner for the cut-set execution model, run by ONE engine —
//! the Cranelift JIT.
//!
//! An indicator request is split: the **interface** — the recurrences the
//! planner CSE-merges into one arena and advances once per bar through one
//! state authority — and the indicator's own derived math. The JIT
//! ([`jit`](jit)) consumes the merged arena directly and compiles the whole
//! query into ONE fused machine loop: chains stepped once for all
//! consumers, the derived expressions (apo/macd/ppo subtractions, the ppo
//! clamp+ratio, macd's chained signal) emitted as interned CLIF ops,
//! lead-gated stores per sink. The math reference for every expression is
//! the indicator's own hand `TState::calc` in its module — the parity tests
//! are against `::indicator()`, so the hand loop IS the spec; no parallel
//! body-duplicates survive here anymore.
//!
//! Retired engines: `fuse`/`columnar`/`simd`/`plan` (`src/cse_bak/`, not
//! compiled) lost to hand code by 2.9–6.2×; `hybrid.rs` (the interpreted
//! cut-set executor) and `tail.rs` (its `Tail`-trait dispatch contract)
//! were removed once the JIT matched the hand suite — 81µs vs 9.4µs
//! classic made the decision for us. Measured rationale: proposals/
//! Handover.md §4, and §5.2/§5.3 of proposals/EXECUTION_PLANNER.md (CSE
//! key, warm-up trap — both still exact).

mod cache;
mod merge;
mod query;
mod recipe;

pub mod jit;

pub use cache::cached_plan;
pub use jit::{JitPlan, JitState};
pub use merge::{merge, Merged, Resolved};
pub use query::{IndicatorStateBox, Request, Results, RowRef};
pub use recipe::{ChainState, Id, Node, Recipe, Sink, SinkSrc, StateRestorer};

/// An indicator that can describe itself as a canonical recipe graph.
///
/// `options` are the resolved runtime option values (e.g. `[12.0, 26.0]` for
/// `Apo(12, 26)`), exactly as passed to `Indicator::indicator` — the recipe
/// bakes them into the interface params (so `Ema{period: 12}` in two
/// indicators merges) and into `Recipe::options` (macd's signal period, …).
pub trait Planable {
    fn recipe(options: &[f64]) -> Recipe;
}
