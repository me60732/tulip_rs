//! # cse — the recipe vocabulary (leaf-side)
//!
//! Home of the CSE engine's **data**: node/recipe/sink types, the
//! merge/interning rules, and the indicator recipe GRAPHS. It exists so the
//! proc-macro kernel generator (`tulip_rs_macros::cse`) and the runtime
//! planner can consume the SAME recipe data — a proc macro may not depend
//! on `tulip_rs`, and this leaf is what it (and the runtime crate, which
//! re-exports from here) can depend on instead.
//!
//! The move is phased (design + executed status:
//! `proposals/Macro_Cse_Engine/ROADMAP.md`, "recipes as data" R1):
//!
//! 1. DONE (2026-10-10) vocabulary: `Node`, `Recipe`, `Sink`, `Planable` from
//!    `tulip_rs/src/cse/recipe.rs`; `merge()`/`leads()` from `merge.rs`;
//!    `Signature`/`Values`/`erase` from `signature.rs` — `tulip_rs::cse`
//!    re-exports so every existing path keeps compiling.
//! 2. DONE (2026-10-12) graphs: the `impl Planable` recipe bodies live here
//!    as [`recipes`] functions (`recipes::apo(options) -> Recipe`); the
//!    indicator modules keep the marker types and a one-line forwarder plus
//!    everything else they own (hand compute, `TState`, restorers, `Info`).
//!    Role tags ride the data: `Recipe::tag` (first writer wins) feeds the
//!    emitter's local names through `Merged::tags` — never an intern key.
//! 3. DONE (2026-10-12) the macro generator (`tulip_rs_macros::cse`) calls
//!    the real `merge()` with sentinel period values at expansion — the
//!    transcribed planner graphs are deleted; the pinned arena test passes
//!    unchanged, proving slot identity.
//!
//! Constraints: no dependency on other workspace crates, no I/O, no
//! generics-heavy machinery — build scripts and proc-macro crates link this
//! on the HOST target, so keep it portable Rust (no `#[target_feature]`,
//! no runtime detection here).

// Phase 1 landed the vocabulary types here (`Node`/`Recipe`/`Sink`/
// `Planable`, `merge`, `Signature`/`Values`/`erase`); see ROADMAP R1 for
// the graph migration (Phase 2).

pub mod merge;
pub mod multipliers;
pub mod recipe;
pub mod recipes;
pub mod signature;

// Root re-exports: the moved files import each other through `crate::cse::…`
// paths (exactly as they did inside `tulip_rs`), and consumers in BOTH crates
// keep working through the same names — zero path edits in the move.
pub use merge::*;
pub use recipe::*;
pub use signature::*;

// The Planable trait lives in `recipe.rs` HERE (Phase 1) — the recipe
// builders it calls are plain graph data. Runtime's cse/mod.rs re-exports
// it alongside the rest of the vocabulary.
