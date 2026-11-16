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
    let (arena, sig, vals, owners, _tails) = erase_parts(req)?;
    Some((arena, sig, vals, owners))
}

/// The same front half, additionally handing back each request's TAIL SLOTS —
/// the global arena ids of `Recipe::tail_inputs` as `merge` remapped them
/// (`Resolved::inputs`). Those are the slots a run must export for this
/// request's `Restorable`, and `carries_to_chains` is the only consumer, which
/// is why they ride a separate entry point instead of `erase_request`'s
/// 4-tuple (its callers are benches and ~every registry gate).
pub fn erase_request_tails(
    req: &Request,
) -> Option<(
    crate::cse::Signature,
    crate::cse::Values,
    Vec<SinkOwner>,
    Vec<Vec<crate::cse::Id>>,
)> {
    let (_arena, sig, vals, owners, tails) = erase_parts(req)?;
    Some((sig, vals, owners, tails))
}

fn erase_parts(
    req: &Request,
) -> Option<(
    crate::cse::Merged,
    crate::cse::Signature,
    crate::cse::Values,
    Vec<SinkOwner>,
    Vec<Vec<crate::cse::Id>>,
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
    let tails = resolved.iter().map(|r| r.inputs.clone()).collect();
    Some((arena, sig, vals, owners, tails))
}

/// A registered run WITH the state guarantee attached.
pub struct RunWithStates {
    /// Rows in arena-sink order, exactly what [`run_registered`] returns.
    pub outcome: Outcome,
    /// Which form ran, so a caller can attribute the number it is looking at.
    pub form: crate::cse::Form,
    /// `Some(states)` — one hand `IndicatorState` per request, in request order,
    /// every one of them rebuilt from THIS run's captured carries: resuming from
    /// it is bit-exact by the gates in `cse_state_capture_test.rs`
    /// (`engine_carry_*_continues_bit_exact`), which is what makes the promise
    /// mean something.
    ///
    /// `None` — the rows are correct and complete, but at least one request's
    /// tail could not be decoded (`carries_to_chains` refused) or its restorer
    /// declined the chains. There is deliberately NO partial variant: a
    /// half-populated state list is how a resumed run silently becomes a
    /// different machine (see §R8 and the "must not shrink" rule in
    /// `proposals/Macro_Cse_Engine/state_capture_checklist.md`).
    pub states: Option<Vec<crate::cse::IndicatorStateBox>>,
}

/// Run a request through the CAPTURED arm set and, when every request's chains
/// decode and every restorer accepts them, hand back the continuable states.
///
/// `None` means no registered pattern serves the request at all (the same
/// contract as [`run_registered`]) — the caller then takes the hand path, which
/// is where its own states come from anyway.
///
/// The windows passed to each restorer are the FULL caller columns, not a
/// trimmed tail: every `IndicatorState::new` in the library tail-slices what it
/// needs (`real[real.len() - period + 1..]`), so passing more is harmless while
/// guessing a shorter window is exactly the class of bug that made the replay
/// restorers wrong (a `period`-sized window has underflowed before). Column
/// order is the request's own `columns` order — the user's law, `open, high, low,
/// close, volume` shifted left — which is also what the arena's stream slots
/// mean, so no translation happens here.
pub fn run_registered_with_states(req: &Request, streams: &[&[f64]]) -> Option<RunWithStates> {
    let (_arena, sig, vals, owners, tails) = erase_parts(req)?;
    let form = super::form_for(&sig, req);
    let mut car = crate::cse::Carries::default();
    let rows = crate::cse::kernel::registry::try_run_captured(&sig, &vals, streams, &mut car)?;
    let fp = sig.hash();

    let mut states = Vec::with_capacity(tails.len());
    let mut promised = true;
    for (r, slots) in tails.iter().enumerate() {
        // Three ways to lose the promise and all three mean the same thing: the
        // tail does not decode, the kind has no restorer at all, or the restorer
        // declined the chains it was handed. The rows stand either way — that is
        // why this returns a run with an optional guarantee rather than an
        // `Option`-shaped failure.
        let Some(chains) = crate::cse::carries_to_chains(&sig, &vals, &car, slots) else {
            promised = false;
            break;
        };
        let recipe = &req.recipes[r];
        // The recipe's own options vector goes to the restorer. Every trait-path
        // arm forwards it to `Restorable::restore_state_with`, whose default
        // ignores it, so this is behaviour-neutral today and is the hook that
        // lets a kind whose parameter is NOT recoverable from a window length
        // (`mom`, `medprice`, `fosc`) restore honestly instead of inferring.
        let options = recipe.options.as_slice();
        let restored = match crate::cse::restorer_for_with_options(&recipe.kind, options) {
            Some(f) => f(chains, streams, options),
            None => match crate::cse::restorer_for(&recipe.kind) {
                Some(f) => f(chains, streams, options),
                None => {
                    promised = false;
                    break;
                }
            },
        };
        match restored {
            crate::cse::IndicatorStateBox::Other => {
                promised = false;
                break;
            }
            other => states.push(other),
        }
    }
    Some(RunWithStates {
        outcome: Outcome { rows, owners, fp },
        form,
        states: if promised { Some(states) } else { None },
    })
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
