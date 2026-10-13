//! The ROUTER side of the engine: what a caller asks for, and how the ask
//! reaches a kernel (or the hand).
//!
//! * [`query`](query) — the runtime data model: `Request` (recipe list +
//!   per-optional flags, hand-`::indicator`-parity builder), `Results` /
//!   `RequestResults` row views, `RowRef` (a window into the shared output
//!   arena), and the `get_states`/resume restorer table.
//! * [`route`](route) — `erase_request` (merge + the rev-2 structure/values
//!   split) then the `try_run` dispatch against the generated `SIG_*`
//!   consts: [`run_registered`], [`run_assets_registered`] and their
//!   [`Outcome`]/[`AssetsOutcome`] shapes. A registry miss is `None` — the
//!   hand path serves; and the documented-not-built direction is SPLIT
//!   routing (serve the registered part, hand-route the rest).
//! * [`state_ext`](state_ext) — the `ChainState` → hand-state adapter the
//!   restorers use to hand back concrete indicator states.

pub mod query;
mod route;
mod state_ext;

pub use query::{restorer_for, IndicatorStateBox, Request, Results, RowRef, StateRestorer};
pub use route::{erase_request, run_assets_registered, run_registered, AssetsOutcome, Outcome};
pub use state_ext::ChainStateEma;
