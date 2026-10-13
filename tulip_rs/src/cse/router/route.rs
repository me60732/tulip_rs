//! The ROUTER: a runtime `Request` becomes a plan, a plan finds its kernel.
//!
//! Per request: `erase_request` merges + erases into the rev-2 pair — the
//! structure-only `Signature` (the dispatch key) and `Values` (every runtime
//! number) — then `try_run` compares it against the generated `SIG_*` consts
//! and calls the matched kernel. A `None` means no registered pattern serves
//! it (unregistered shape, unsupported node, over-cap sizes): the caller
//! falls to the hand path, never a runtime compile.
//!
//! DIRECTION (not built, per user 2026-10-12): SPLIT routing. A request the
//! registry cannot serve WHOLE may still decompose — e.g. request
//! `Apo(12,26) + Ppo(12,26) + Macd(12,26,9)` when only
//! `apo_ppo_shared_sl` is registered: serve the Apo+Ppo pair from that
//! kernel and route the Macd line to its handrolled indicator function,
//! instead of dropping the whole request to per-indicator fallback. That
//! needs kernel-vs-request cost accounting (the saved-work law says a split
//! only pays when the kernel's dedup beats the extra hand pass), so it lands
//! with the router milestones, not here.

use crate::cse::kernel::registry::{try_run, try_run_assets};
use crate::cse::merge::merge;
use crate::cse::router::query::{Request, RowRef};
use crate::cse::signature::{erase, SinkOwner};

/// Result rows in arena-sink order + the regrouping map.
///
/// A disabled optional sink produces a zero-length `RowRef` (`is_empty() == true`);
/// it never reads and its store is guarded by `sink_want`. The contract mirrors the
/// hand's `Vec::new()` rows for disabled optionals.
pub struct Outcome {
    pub rows: Vec<RowRef>,
    pub owners: Vec<SinkOwner>,
    /// Erased-shape hash of the pattern that served the request (telemetry).
    pub fp: u64,
}

/// Merge a request, apply its per-optional flags to the resolved sinks (the
/// JIT's `build` did the same; `merge` itself defaults all-on), and erase —
/// the router's front half, exported for benches that hoist the plan out of
/// their timed loops.
pub fn erase_request(
    req: &Request,
) -> Option<(
    crate::cse::Merged,
    crate::cse::Signature,
    crate::cse::Values,
    Vec<SinkOwner>,
)> {
    let (arena, mut resolved) = merge(&req.recipes);
    for (r, flags) in resolved.iter_mut().zip(req.opt_flags.iter()) {
        if flags.len() == r.optional_enabled.len() {
            r.optional_enabled = flags.clone();
        }
    }
    let leads = arena.leads();
    // erase now RECORDS optional flags as want bits instead of filtering sinks
    let (sig, vals, owners) = erase(&arena, &resolved, &leads)?;
    Some((arena, sig, vals, owners))
}

/// Attempt to serve `req` from the generated registry. `None` = no pattern
/// matches (unsupported nodes, unregistered shape, > cap sizes).
pub fn run_registered(req: &Request, streams: &[&[f64]]) -> Option<Outcome> {
    let (_arena, sig, vals, owners) = erase_request(req)?;
    let rows = try_run(&sig, &vals, streams)?;
    Some(Outcome {
        rows,
        owners,
        fp: sig.hash(),
    })
}

/// Result of an **asset-lane** request: one plan, L assets, per-asset
/// [`Outcome`]s (the [`run_registered`] shape stretched across N assets).
pub struct AssetsOutcome {
    /// Lane `a`'s fused result — same owned-row type as the scalar path
    /// because the lane kernel produces the same buffers, L-wide.
    pub per_asset: Vec<Outcome>,
}

/// Run a [`Request`] through the **lane kernels**: one `(Signature, Values)`
/// plan drives all `L` assets inside a single generated function.
/// `assets[a]` is that asset's stream tuple — the same layout
/// `Request::new` takes. Returns `None` if no registered pattern matches,
/// exactly like [`run_registered`].
///
/// This is the `indicator_by_assets` shape with the planner's dedup kept:
/// L assets × M indicators that share recurrences run as one fused, SIMD
/// pass — which per-indicator SIMD cannot do (it dedups nothing across
/// indicators). Resume/`get_states` is M1.
pub fn run_assets_registered<const L: usize>(
    req: &Request,
    assets: &[&[&[f64]]; L],
) -> Option<AssetsOutcome> {
    let (_arena, sig, vals, owners) = erase_request(req)?;
    let per_asset = try_run_assets::<L>(&sig, &vals, assets)?;
    let fp = sig.hash();
    Some(AssetsOutcome {
        per_asset: per_asset
            .into_iter()
            .map(|rows| Outcome {
                rows,
                owners: owners.clone(),
                fp,
            })
            .collect(),
    })
}
