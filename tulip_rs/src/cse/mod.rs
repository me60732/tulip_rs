//! CSE: the planner for the cut-set execution model, run by ONE engine —
//! the Cranelift JIT.
//!
//! An indicator request is split: the **interface** — the recurrences the
//! planner CSE-merges into one arena and advances once per bar through one
//! state authority — and the **tail**, the indicator's own math. The JIT
//! ([`jit`](jit)) consumes the merged arena directly and compiles the whole
//! query into ONE fused machine loop: chains stepped once for all consumers,
//! the closed tail set (`ema`/`apo`/`ppo`/`macd`, mirrors) lifted to CLIF
//! bit-exactly, lead-gated stores per sink. The tail *bodies* in the
//! indicator modules remain the reference the emitter mirrors (and the
//! macd seeding rule's standalone gate); the wiring language is in
//! [`recipe`](recipe), the surface in [`query`](query).
//!
//! Retired engines: `fuse`/`columnar`/`simd`/`plan` (`src/cse_bak/`, not
//! compiled) lost to hand code by 2.9–6.2×; `hybrid.rs` (the interpreted
//! cut-set executor) was 3.9–4.4× behind the JIT on every bulk workload
//! (81µs vs 18.4µs classic) and was removed once the emitter matched the
//! hand indicators bit-exact on all wired shapes — `jit.rs` tests are that
//! proof, re-run against hand. Measured rationale: proposals/Handover.md
//! §4, and §5.2/§5.3 of proposals/EXECUTION_PLANNER.md (CSE key, warm-up
//! trap — both still exact).

mod merge;
mod query;
mod recipe;
mod tail;

pub mod jit;

pub use jit::{JitPlan, JitState};
pub use merge::{merge, Merged, Resolved};
pub use query::{IndicatorStateBox, Request, Results};
pub use recipe::{Id, Node, Recipe, Sink, SinkSrc};
pub use tail::Tail;

/// An indicator that can describe itself as a canonical recipe graph.
///
/// `options` are the resolved runtime option values (e.g. `[12.0, 26.0]` for
/// `Apo(12, 26)`), exactly as passed to `Indicator::indicator` — the recipe
/// bakes them into the interface params (so `Ema{period: 12}` in two
/// indicators merges) and into `Recipe::options` for the tail constructors.
pub trait Planable {
    fn recipe(options: &[f64]) -> Recipe;
}
