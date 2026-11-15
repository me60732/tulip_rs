//! The router's DECISION layer: which generated FORM serves an erased shape,
//! and what the caller must do when none does.
//!
//! The registry is already the filter that matters: a row is declared only
//! while it beats the hand on the axis it serves (the per-axis census,
//! `proposals/Macro_Cse_Engine/CSE_PER_AXIS_MATRIX_2026-11-15.md`), so the
//! routing rule needs no prediction table — **a kernel that exists is
//! preferred over the hand, and the only real question is WHICH kernel.**
//! This module answers that for the scalar axis; the asset axis never asks,
//! because it is a different API endpoint (`try_run_assets`, per-asset rows).
//!
//! Two shapes reach the same `Signature` slot and must be told apart:
//!
//! * `Apo(6,12) + Ppo(8,14) + Macd(9,18)` — SIX distinct EMA chains, nothing
//!   dedups. The redundancy is per-LANE, so the arena wants the packed kernel
//!   (`ema_chains_p`-style: one mul + one fma for a whole bucket of chains over
//!   one broadcast source). No packed kernel ⇒ the hand's own answer for this
//!   shape is `indicator_by_options::<N>`, not N scalar calls.
//! * `Apo(6,12) + Ppo(6,12) + Macd(6,12)` — the chains ARE the sharing. CSE has
//!   already collapsed them, so the scalar kernel is the right form and the
//!   fallback is per-indicator `indicator()`.
//!
//! The discriminator is therefore structural, computed on the erased arena:
//! the biggest group of packable chains that share ONE source node. ≥ 2 means
//! the request asked for lane-able work; whether it is *worth* packing is not
//! decided here — a `packed` row that loses is undeclared, and this layer then
//! routes to the scalar kernel instead of to the hand.

use crate::cse::kernel::registry::{try_run, try_run_packed};
use crate::cse::query::RowRef;
use crate::cse::router::route::{erase_request, Outcome};
use crate::cse::signature::{Signature, K_EMA, K_IIR, K_WILDERS};

/// The node kinds whose steady step lowers to `chain_step` — one broadcast
/// source, two per-slot weights from `Values`, one carry in `st[k]`, whatever
/// recipe built it. This is the runtime mirror of the compile-time predicate
/// `tulip_rs_macros::cse::codegen::shape::chain_step_form` (`Ema | Wilders |
/// Iir`); `packable_kinds_match_the_codegen_predicate` pins the two lists equal,
/// because a group the emitter cannot vectorise must never be routed to.
///
/// Excluded on the same evidence as the emitter: `IirClamped` carries four clamp
/// thresholds through `b/c/d/e` and a group statement hoists only two per-slot
/// vectors; `Accum`'s multiplier is a NODE (volume), not a value, so it hoists
/// nothing; `Sma`/`RollSum`/`Stddev`/`Wma`/rings are a window fold, grouped by
/// the ring machinery instead.
pub const PACKABLE_KINDS: [u8; 3] = [K_EMA, K_WILDERS, K_IIR];

/// Packable chains the arena still holds, whatever their grouping.
pub fn packable_slots(sig: &Signature) -> usize {
    (0..sig.len as usize)
        .filter(|i| PACKABLE_KINDS.contains(&sig.kinds[*i]))
        .count()
}

/// Packable chains the REQUEST asked for, summed over the recipes' own interface
/// graphs, before interning. `Request.recipes` is `pub(crate)` and
/// `Recipe::nodes` is the same `Vec<Node>` the merge consumes, so this is the
/// merge's own input counted a second time — not a parallel implementation.
///
/// The two numbers TOGETHER are the discriminator:
/// `requested > arena` means CSE collapsed chains, which is the engine's win
/// and the scalar kernel's shape; `requested == arena` means nothing deduped,
/// so the only remaining redundancy is per-lane and packing is what the shape
/// asks for. Group size alone cannot tell them apart — `Apo(6,12) + Ppo(6,12)`
/// leaves TWO live EMA chains over one source, structurally identical to two of
/// `ema_chains`' six, and only the request says which case it is.
pub fn packable_requested(req: &crate::cse::Request) -> usize {
    req.recipes
        .iter()
        .flat_map(|r| r.nodes.iter())
        .filter(|n| {
            matches!(
                n,
                crate::cse::Node::Ema { .. }
                    | crate::cse::Node::Wilders { .. }
                    | crate::cse::Node::Iir { .. }
            )
        })
        .count()
}

/// The largest group of packable chains reading ONE source node.
///
/// `(kind, a)` is the bucket key because `erase` writes a chain's source NODE
/// index into `a` (`Node::Ema { src, period }` → `(K_EMA, src, 0, 0)`,
/// `signature.rs`'s Ema arm) while the period goes to `Values::periods` — so
/// two chains of one kind over one interned source collide in the key by
/// construction, which is exactly the `classify_packed` bucketing rule. Chains
/// over DIFFERENT sources are not a group: cross-source packing was built,
/// gated byte-exact and measured SLOWER than not packing (`xpacked`, +6.4% over
/// the same-source twin and +2.3% over scalar), so the splat-only law holds.
///
/// Returns 1 for an arena with no multi-chain group, 0 when it holds none.
pub fn packable_group(sig: &Signature) -> usize {
    let n = sig.len as usize;
    let mut best = 0usize;
    for k in PACKABLE_KINDS {
        for i in 0..n {
            if sig.kinds[i] != k {
                continue;
            }
            // Count only FORWARD from `i` so one bucket is sized once, and a
            // chain that is itself the leader of its group is never skipped.
            let mut same = 0usize;
            for j in i..n {
                if sig.kinds[j] == k && sig.a[j] == sig.a[i] {
                    same += 1;
                }
            }
            best = best.max(same);
        }
    }
    best
}

/// Which generated form an erased shape wants. `Packed` is a CANDIDACY claim,
/// not a promise that such a kernel was declared — [`route_registered`] probes
/// `try_run_packed` and falls through to the scalar kernel when it is absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Packed,
    Scalar,
}

/// The form rule, computed from the erased arena AND the request that produced
/// it. `Packed` is a CANDIDACY claim, not a promise that such a kernel was
/// declared — [`route_registered`] probes `try_run_packed` and falls through to
/// the scalar kernel when it is absent, which is the right outcome for the
/// shapes whose packed twin measured no better than their scalar one
/// (`ema_chains` 12.36 vs 12.31 µs, `sma_six` 13.69 vs 13.51).
///
/// Packed requires BOTH:
/// * a group of ≥2 packable chains over ONE source (the lane-able work), and
/// * nothing collapsed — `packable_slots == packable_requested`, so the
///   redundancy that remains is per-lane rather than per-request.
///
/// `Kind::Sma` is deliberately absent, unlike `classify_packed`, which buckets
/// price-sourced `Sma` separately. The runtime mirror stays with the
/// `chain_step` forms because that is where the arrangement law lives; the
/// `sma_six` packed row then simply falls through to the scalar kernel, which
/// measured 1.3% ahead of it here. NOTE the disagreement on record: this
/// module's own comment in `codegen/classify.rs` cites "+6.6% packed on
/// `sma_six`", while the 2026-11-15 same-binary pass measured packed BEHIND
/// scalar. That difference is unresolved and is stated rather than papered over.
pub fn form_for(sig: &Signature, req: &crate::cse::Request) -> Form {
    let nothing_collapsed = packable_slots(sig) == packable_requested(req);
    if nothing_collapsed && packable_group(sig) >= 2 {
        Form::Packed
    } else {
        Form::Scalar
    }
}

/// What the caller must do with the request.
///
/// Not `Debug`, and that is forced, not a choice: the `Kernel` arm holds an
/// `Outcome` whose rows are `Vec<RowRef>`, and `RowRef` derives only `Clone`
/// (`query.rs`, the row is a window into the arena with a borrowed slice) — so
/// `Outcome` cannot derive `Debug` either. Tests match on the variant instead.
pub enum Served {
    /// A generated kernel ran it, with its rows. `form` records WHICH one, so a
    /// bench or a telemetry path can attribute the number it is looking at.
    Kernel { outcome: Outcome, form: Form },
    /// A packing-candidate shape with no packed kernel: the hand answers this
    /// request with one SIMD-over-option-sets call per indicator
    /// (`indicator_by_options::<N>`, bucketed 4/2 — the driver is only valid at
    /// 2/4/8 widths), NOT with N scalar passes.
    HandOptions,
    /// Nothing registered serves it: per-indicator scalar `indicator()` calls.
    HandScalar,
}

/// The scalar-axis route: erase, pick the form the arena implies, prefer a
/// kernel over the hand.
///
/// Order matters and is the whole policy: for a packing candidate the packed
/// dispatcher is probed FIRST, then the scalar one, so a shape whose packed row
/// was never declared (or was deleted for losing) still gets its scalar kernel
/// instead of falling to the hand. A non-candidate goes straight to scalar.
pub fn route_registered(req: &crate::cse::Request, streams: &[&[f64]]) -> Served {
    let (_arena, sig, vals, owners) = match erase_request(req) {
        Some(x) => x,
        // The arena could not be erased at all (unsupported node, over-cap
        // sizes): there is no shape to route, so the hand's scalar path is the
        // only defined answer.
        None => return Served::HandScalar,
    };
    let form = form_for(&sig, req);
    let rows = match form {
        Form::Packed => {
            try_run_packed(&sig, &vals, streams).or_else(|| try_run(&sig, &vals, streams))
        }
        Form::Scalar => try_run(&sig, &vals, streams),
    };
    match rows {
        Some(rows) => Served::Kernel {
            outcome: Outcome {
                rows,
                owners,
                fp: sig.hash(),
            },
            form,
        },
        None => match form {
            Form::Packed => Served::HandOptions,
            Form::Scalar => Served::HandScalar,
        },
    }
}

/// Rows of a `Served::Kernel`, empty for the hand routes — the shape the
/// benches and parity gates already use so a route change reads as a value
/// difference, never as a compile break.
impl Served {
    pub fn rows(&self) -> &[RowRef] {
        match self {
            Served::Kernel { outcome, .. } => &outcome.rows,
            Served::HandOptions | Served::HandScalar => &[],
        }
    }
}
