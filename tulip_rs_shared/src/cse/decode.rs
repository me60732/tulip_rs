//! Decode a kernel's `Carries` into the `ChainState` values a `Restorable`
//! consumes — the middle hop between Phase A (capture) and the hand state.
//!
//! The kernel speaks in ARENA SLOTS (`st[k]`, `sumsq[k]`, …); the restorers speak
//! in `ChainState` variants named after node kinds. This module is the only place
//! those two vocabularies meet, and it is deliberately conservative: a slot whose
//! kind is not listed below returns `None`, which the router must read as "no
//! states for this request" — never as a partial chain list. That is the
//! "must not shrink" rule of
//! `proposals/Macro_Cse_Engine/state_capture_checklist.md`, enforced at the type:
//! a missing arm cannot be mistaken for a zero, because the value is absent.
//!
//! What is refused, and why:
//! * `RollSum` / `RollAge` / `Hma` / `HtInPhase` / `HtQuadrature` — their window
//!   lives in `RingSet`, which `Carries` does not export yet (Phase C). `Hma`
//!   additionally needs its variable-length diff ring (`√period`), so even the
//!   `hacc` accumulators alone are not a continuable state.
//! * `Iir` / `Prev` / `Accum` / `VariableAlpha` — `ChainState` has no variant for
//!   them. Their carry IS captured (`st[k]`, written back from the hoisted
//!   `{ident}_carry`), so the gap is the enum, not the kernel.
//! * `Mirror` — needs the node's last PUBLISHED value (`v[k]`), which is per-bar
//!   scratch, not state. The hand restorers that want a mirror read it off the
//!   caller's stream instead.
//! * `KvoFlow` — the emitted `kvacc[k]` triple is `[hlc, cm, dm]`, which does not
//!   line up field-for-field with `ChainState::KvoFlow { cm, prev_hlc, prev_hl,
//!   trend }`. Mapping it needs its own verified slice, not a guess here.
//! * the bespoke scalar recurrences (`Nvi`/`Pvi`/`Psar`/`Sdft*`/`Cybercycle`/
//!   `Mama`/`Hd`/`InstantaneousTrendline`/`AdaptiveMsw`/`PeakDecay`/
//!   `MeanAbsDev`/`SuperTrend`-consumers) — no `ChainState` variant per kind.

use crate::cse::recipe::ChainState;
use crate::cse::signature::{
    Carries, Signature, Values, K_EMA, K_HIGHPASS, K_IIRCLAMPED, K_LINREGFLOW, K_ROLLMAX,
    K_ROLLMIN, K_SEEDEDEMA, K_SMA, K_STDDEV, K_SUPERSMOOTHER, K_SUPERTREND, K_WILDERS, K_WMA, MAX,
};
use crate::cse::Id;

/// One slot's decoded chain, or `None` when its kind is outside the table.
///
/// The array each kind reads is the one its EMITTER writes: chains fold through
/// a hoisted `{ident}_carry` local that `carry_back` scatters into `st[k]`
/// (`emit_scalar.rs`), while `Stddev`/`Wma`/`SuperTrend`/`HighPass`/
/// `SuperSmoother` keep their extra fields in the family arrays the epilogue
/// memcpy exports. A grouped (packed) `Sma` is scattered from the lane register
/// — the roll loop skips it so exactly one write owns the slot.
pub fn chain_for_slot(sig: &Signature, vals: &Values, c: &Carries, k: usize) -> Option<ChainState> {
    if k >= MAX || k >= sig.len as usize {
        return None;
    }
    let period = vals.periods[k];
    let v = match sig.kinds[k] {
        K_EMA | K_SEEDEDEMA => ChainState::Ema {
            value: c.st[k],
            period,
        },
        K_WILDERS => ChainState::Wilders {
            value: c.st[k],
            period,
        },
        K_IIRCLAMPED => ChainState::IirClamped { value: c.st[k] },
        K_SMA => ChainState::Sma {
            sum: c.st[k],
            period,
        },
        K_STDDEV => ChainState::Stddev {
            sum: c.st[k],
            sum_sq: c.sumsq[k],
            period,
        },
        K_WMA => ChainState::Wma {
            sum: c.st[k],
            weighted: c.wsum[k],
            period,
        },
        K_ROLLMAX => ChainState::RollMax {
            value: c.st[k],
            trail: c.trail[k] as u64,
            period,
            look_back: vals.roll_lb[k],
        },
        K_ROLLMIN => ChainState::RollMin {
            value: c.st[k],
            trail: c.trail[k] as u64,
            period,
            look_back: vals.roll_lb[k],
        },
        K_SUPERTREND => ChainState::Supertrend {
            prev_st: c.stb[k],
            prev_ub: c.stc_ub[k],
            prev_lb: c.stlb[k],
            // The steady loop carries the trend bit as 1.0/0.0 in the node's own
            // `st[k]` slot — the same convention `kernel::exec`'s `K_SUPERTREND`
            // arm uses, which is what makes the warm→steady handoff exact.
            trend: c.st[k] != 0.0,
            step: vals.st_step[k],
        },
        K_HIGHPASS => ChainState::HighPass {
            y1: c.hp_acc[k][0],
            prev_real: c.hp_acc[k][1],
            period,
        },
        K_SUPERSMOOTHER => ChainState::SuperSmoother {
            y1: c.ss_acc[k][0],
            y2: c.ss_acc[k][1],
            prev_real: c.ss_acc[k][2],
            period,
        },
        // Handled by `carries_to_chains`, which pairs it — a lone moment node is
        // not a machine and decoding one in isolation would hand back a lie.
        K_LINREGFLOW => return None,
        _ => return None,
    };
    Some(v)
}

/// Decode the chains of ONE request, in its `tail_inputs` order.
///
/// `slots` are GLOBAL arena ids — `Resolved::inputs`, which `merge` already
/// remapped from the recipe's local `tail_inputs`. All-or-nothing: any slot
/// outside the table refuses the whole request.
///
/// `LinregFlow` is the one machine that spans two nodes, so the pair collapses to
/// a single entry (which is exactly what `Linreg::restore_state` matches:
/// `[ChainState::LinregFlow { .. }]`). Which half is which is read from VALUES,
/// not guessed: `erase` writes `mults[k] = 1.0` for the plain `sum_y` and
/// `mults[k] = period as f64` for the weighted `sum_xy` — the same numbers the
/// fold uses — so the test is exact float equality against a value the encoder
/// itself stored, never a comparison of computed results. `n` is the hand's phase
/// counter, which `State::init_state` sets to `period as f64` and never mutates
/// (verified: `linreg.rs` writes `n` only in `init_state`), so it is derived, not
/// carried.
pub fn carries_to_chains(
    sig: &Signature,
    vals: &Values,
    c: &Carries,
    slots: &[Id],
) -> Option<Vec<ChainState>> {
    let mut out = Vec::with_capacity(slots.len());
    let mut i = 0;
    while i < slots.len() {
        let k = slots[i] as usize;
        if k >= MAX || k >= sig.len as usize {
            return None;
        }
        if sig.kinds[k] == K_LINREGFLOW {
            let j = i + 1;
            if j >= slots.len() {
                return None;
            }
            let k2 = slots[j] as usize;
            if k2 >= MAX || k2 >= sig.len as usize || sig.kinds[k2] != K_LINREGFLOW {
                return None;
            }
            let period = vals.periods[k];
            if vals.periods[k2] != period || period < 2 {
                // Two different machines (or the degenerate period-1 case, where
                // the plain and weighted weights are the SAME number 1.0 and the
                // two halves cannot be told apart) — refuse rather than guess.
                return None;
            }
            let nf = period as f64;
            let (a, b) = (vals.mults[k], vals.mults[k2]);
            let (sum_y, sum_xy) = if a == 1.0 && b == nf {
                (c.st[k], c.st[k2])
            } else if b == 1.0 && a == nf {
                (c.st[k2], c.st[k])
            } else {
                return None;
            };
            out.push(ChainState::LinregFlow {
                sum_y,
                sum_xy,
                n: nf,
                period,
            });
            i += 2;
            continue;
        }
        out.push(chain_for_slot(sig, vals, c, k)?);
        i += 1;
    }
    Some(out)
}
