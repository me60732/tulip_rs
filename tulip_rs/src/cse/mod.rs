//! CSE: the planner for the cut-set execution model.
//!
//! An indicator request is split: the **interface** — the recurrences the
//! planner CSE-merges into one arena and advances once per bar through one
//! state authority — and the indicator's own derived math. The math
//! reference for every expression is the indicator's own hand `TState::calc`
//! in its module — the parity tests are against `::indicator()`, so the
//! hand loop IS the spec; no parallel body-duplicates survive here anymore.
//!
//! The crate is split by the two halves of that sentence, so the naming can
//! not blur again (2026-10-12):
//!
//! * [`kernel`] — GENERATION. [`kernel::registry`](kernel::registry) is the
//!   one `kernel!` call site (patterns → compile-time fused kernels + the
//!   `try_run`/`try_run_assets` dispatchers); [`kernel::warmup`](kernel::warmup)
//!   is the const-generic machine every generated kernel CALLS for its warm
//!   bars `[0, gate)` (its full-series `run` wrapper was retired the same
//!   day — the parity gate is generated-kernel-vs-hand, nothing interprets
//!   a series at runtime). The retired Cranelift JIT (tier-2 reference
//!   through 2026-10) is archived in the top-level `old_jit/` directory;
//!   its resume/get_states contracts port to the registry as M1.
//! * [`router`] — ROUTING. [`router::query`](router::query) is the runtime
//!   `Request`/`Results`/`RowRef` data model + restorer table;
//!   [`router::route`](router::route) erases a request to the rev-2 pair
//!   (structure-only `Signature` = dispatch key, `Values` = runtime numbers)
//!   and matches it against the generated kernels; misses return `None` and
//!   fall to the hand path, never a runtime compile. Split routing
//!   (registered part from a kernel, remainder to hand) is documented there,
//!   not built.
//!
//! The `safe_inputs` data-trust feature died with the JIT (2026-10-12): the
//! divisor floor it toggled for `Ppo` is gone everywhere — a zero price is
//! excluded by the data contract, and the hand divide, the recipes, and the
//! kernels now state one graph.
//!
//! PHASE 1 + 2 of "recipes as data" (2026-10-10/12): the vocabulary —
//! `Node`, `Recipe`, `Sink`, `Planable`, `merge()`/`leads()`,
//! `Signature`/`Values`/`erase` — AND the recipe GRAPHS
//! ([`recipes`] re-exported from the leaf) live in
//! `tulip_rs_shared::cse`, so runtime, build scripts, and the `kernel!`
//! generator (`tulip_rs_macros::cse`, a proc macro, which may not depend
//! on this crate) consume the SAME recipe data: one source of graphs, no
//! transcription. This module re-exports it, so every `crate::cse::*` path
//! is unchanged — as are the `crate::cse::query::*` and
//! `crate::cse::kernel::exec` paths the macro EMITS into generated code.

pub mod kernel;
mod router;

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

// ── Routing facade (the runtime ask) ─────────────────────────────────────
// Module alias: generated kernels reference `crate::cse::query::RowRef`.
pub use router::query;
pub use router::{
    erase_request, restorer_for, run_assets_registered, run_registered, AssetsOutcome,
    ChainStateEma, IndicatorStateBox, Outcome, Request, Restorable, Results, RowRef, StateRestorer,
};

// ── Kernel facade (the generated engines + the warm/oracle machine) ─────
/// Generated kernels, callable directly (bypasses `try_run` first-match):
/// the scalar/packed twins share one Signature each, so same-binary A/B of
/// the two codegen strategies needs direct entry points (`ema_six`,
/// `sma_six`, `ema_chains`, `atr_multi` families). Every other generated
/// kernel is reachable at `kernel::registry::<name>`.
pub use kernel::{atr_multi, atr_multi_p, ema_chains, ema_chains_p, ema_six, ema_six_p};
pub use kernel::{exec, try_run, try_run_assets};
pub use kernel::{sma_six, sma_six_p};
pub use kernel::{
    SIG_ao_fisher_shared_med, SIG_ao_s_m, SIG_apo_ppo_macd_shared_sl, SIG_apo_ppo_shared_sl,
    SIG_aroon_osc_shared_p, SIG_atr_multi, SIG_atr_natr_shared_p, SIG_bbands_wma_shared_p,
    SIG_cmo_n, SIG_ema_chains, SIG_ema_six, SIG_ema_six_p, SIG_fisher_n, SIG_hma_n,
    SIG_ichimoku_s_m, SIG_max_min_shared_p, SIG_natr_supertrend_shared_p, SIG_rsi_cmo_shared_p,
    SIG_rsi_n, SIG_rsi_stochrsi_shared_p, SIG_sma_six, SIG_sma_stddev_shared_p,
    SIG_sma_wma_shared_p, SIG_stoch_kd, SIG_stoch_willr_donchian_shared_p,
    SIG_willr_donchian_shared_p, SIG_kama_n, SIG_vidya_n,
};

// ── Leaf vocabulary (Phase 1 + 2, "recipes as data") ─────────────────────
// Item paths AND module paths (`crate::cse::signature::K_*`, …) preserved.
pub use tulip_rs_shared::cse::{erase, Signature, Values, MAX, MAX_SINKS};
pub use tulip_rs_shared::cse::{merge, Merged, Resolved};
pub use tulip_rs_shared::cse::{multipliers, recipe, recipes, signature};
pub use tulip_rs_shared::cse::{ChainState, Grafted, Id, Node, Planable, Recipe, Sink, SinkSrc};
