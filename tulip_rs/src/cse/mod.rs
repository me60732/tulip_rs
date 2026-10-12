//! CSE: the planner for the cut-set execution model.
//!
//! An indicator request is split: the **interface** — the recurrences the
//! planner CSE-merges into one arena and advances once per bar through one
//! state authority — and the indicator's own derived math. The math
//! reference for every expression is the indicator's own hand `TState::calc`
//! in its module — the parity tests are against `::indicator()`, so the
//! hand loop IS the spec; no parallel body-duplicates survive here anymore.
//!
//! The engine is the macro-fused kernel registry ([`registry`](registry)):
//! `request!` patterns emit straight-line Rust steady loops, dispatched on
//! the erased `Signature` (design + measured history in
//! `proposals/Macro_Cse_Engine/`). The const-generic interpreter
//! ([`kernel`](kernel)) is the warm path of every generated kernel and the
//! cross-check oracle. Every pattern also emits a **lane kernel**
//! (`<name>_assets<const L>`, dispatch [`try_run_assets`]): L assets
//! through ONE (Signature, Values) — the `indicator_by_assets` shape, but
//! keeping the merged graph's dedup (which per-indicator SIMD cannot). The
//! retired Cranelift JIT (tier-2 reference through 2026-10) is archived in
//! the top-level `old_jit/` directory; its resume/get_states contracts
//! port to the registry as M1. The `safe_inputs` data-trust feature died
//! with the JIT (2026-10-12): the divisor floor it toggled for `Ppo` is
//! gone everywhere — a zero price is excluded by the data contract, and
//! the hand divide, the recipes, and the kernels now state one graph.
//!
//! PHASE 1 + 2 of "recipes as data" (2026-10-10/12): the vocabulary —
//! `Node`, `Recipe`, `Sink`, `Planable`, `merge()`/`leads()`,
//! `Signature`/`Values`/`erase` — AND the recipe GRAPHS
//! ([`recipes`] re-exported from the leaf) live in
//! `tulip_rs_shared::cse`, so runtime, build scripts, and the `request!`
//! generator (`tulip_rs_macros::cse`, a proc macro, which may not depend
//! on this crate) consume the SAME recipe data: one source of graphs, no
//! transcription. This module re-exports it, so every `crate::cse::*` path
//! is unchanged. Runtime-only pieces stay here: the restorer table
//! ([`query`]'s `restorer_for`, keyed by `Recipe::kind` — replacing the
//! old `Recipe.restorer` field, which the leaf cannot type), the
//! `ChainState` → hand-state extension ([`state_ext`]), `Request`/`Results`
//! ([`query`]), the interpreter, and the registry.

pub mod kernel;
mod query;
mod registry;
mod state_ext;

/// Graph/plan sizes the merge/interning canaries pin
/// (`tulip_test/tests/cse_merge_test.rs`). Exact-value assertions so an
/// accidental de-dup or an accidental extra node is a test failure, never a
/// `<=` shrug: the classic apo_ppo_macd graph is ONE shared line, three
/// EMA chains (e12/e26/signal), one `hundred` Const — divisor guards live
/// nowhere since `safe_inputs` retired with the JIT.
pub const CLASSIC_NODES: usize = 9;
/// [`CLASSIC_NODES`]'s fused-op count.
pub const CLASSIC_OPS: usize = 8;
/// The dense dashboard's merged node count (two `Ppo`s at different
/// periods ⇒ two guards, one shared `ε` — one `hundred` Const each, no
/// divisor guards post-`safe_inputs`).
pub const DENSE_NODES: usize = 27;

pub use query::{restorer_for, IndicatorStateBox, Request, Results, RowRef, StateRestorer};
pub use registry::erase_request;
/// Generated kernels, callable directly (bypasses `try_run` first-match):
/// the scalar/packed twins share one Signature each, so same-binary A/B of
/// the two codegen strategies needs direct entry points (`ema_six`,
/// `sma_six`, `ema_chains`, `atr_multi` families).
pub use registry::{atr_multi, atr_multi_p, ema_chains, ema_chains_p, ema_six, ema_six_p};
/// Asset-lane request facade (one plan, L assets, per-asset [`Outcome`]s).
pub use registry::{run_assets_registered, AssetsOutcome};
pub use registry::{run_registered, sma_six, sma_six_p, try_run, try_run_assets, Outcome};
pub use registry::{
    SIG_apo_ppo_macd_shared_sl, SIG_apo_ppo_shared_sl, SIG_aroon_osc_shared_p, SIG_atr_multi,
    SIG_atr_natr_shared_p, SIG_bbands_wma_shared_p, SIG_ema_chains, SIG_ema_six, SIG_ema_six_p,
    SIG_ichimoku_s_m, SIG_max_min_shared_p, SIG_natr_supertrend_shared_p, SIG_sma_six,
    SIG_sma_stddev_shared_p, SIG_sma_wma_shared_p, SIG_willr_donchian_shared_p,
};
pub use state_ext::ChainStateEma;

// ── Leaf vocabulary (Phase 1 + 2, "recipes as data") ─────────────────────
// Item paths AND module paths (`crate::cse::signature::K_*`, …) preserved.
pub use tulip_rs_shared::cse::{erase, Signature, Values, MAX, MAX_SINKS};
pub use tulip_rs_shared::cse::{merge, Merged, Resolved};
pub use tulip_rs_shared::cse::{multipliers, recipe, recipes, signature};
pub use tulip_rs_shared::cse::{ChainState, Grafted, Id, Node, Planable, Recipe, Sink, SinkSrc};
