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
//! Kind-by-kind status — either refused, or CLOSED with the evidence that
//! un-refused it (a closed item stays listed: silently un-refusing a kind is the
//! failure mode this table exists to prevent):
//! * `HtInPhase` / `HtQuadrature` — CLOSED by C6, but only after the export grew
//!   a second order: those consumers rebuild a `FixedRingBuffer` from a
//!   CHRONOLOGICAL (oldest-first) slice, so the decode reads `RingExport.chrono`,
//!   never `raw`. `RollSum`/`RollAge`/`Hma` (`mass`, `adxr`, `hma`) were the
//!   first: they rebuild a plain `Buffer<Warm>` from raw slots + `cursor / 8`,
//!   which is exactly the exported shape (their own length assertions are the
//!   guard). Anything still on this list is refused rather than answered with a
//!   rotated or partial state.
//! * `KvoFlow` — CLOSED by C5. The emitted arm stores `kvacc[k] = [hlc, cm, dm]`,
//!   which IS the next bar's `[prev_hlc, cm, prev_hl]`, with the trend bit in
//!   `st[k]` as 1.0/0.0; gated by `engine_carry_kvo_continues_bit_exact`.
//! * `Iir` / `VariableAlpha` — no `ChainState` variant. Their carry IS captured
//!   (`st[k]`, written back from the hoisted `{ident}_carry`), so the gap is the
//!   enum, not the kernel.
//! * `Prev` — CLOSED 2026-11-16: a `Prev` node's state is one last-value carry,
//!   the same shape `Mirror` means, so it decodes as `ChainState::Mirror`. This
//!   is what `recipes::fisher`'s own comment had always claimed; the arm was
//!   simply never written, and `fisher` refused its own tail (kinds `[28, 29]`).
//! * `Accum` — CLOSED for `adosc` the same way: the recipe already exports the
//!   accumulator through `Node::Mirror`, so no variant was needed.
//! * `NviRecur` / `PviRecur` — CLOSED 2026-11-16: the recipes export the carry,
//!   `close` and `volume` as three `Mirror`s and both resume bit-exactly
//!   (`engine_carry_nvi_pvi_continue_bit_exact`), so the old "the published row
//!   is a prefix product and cannot be expressed" verdict is retired — the row
//!   IS expressible as a carried scalar plus its two gate inputs.
//! * the remaining bespoke recurrences (`Psar`/`Sdft*`/`Cybercycle`/`Mama`/
//!   `Hd`/`InstantaneousTrendline`/`AdaptiveMSW`/`PeakDecay`/`MeanAbsDev`) — no
//!   `ChainState` variant per kind; `MeanAbsDev` is a rescan, so its consumers
//!   (`md`, `cci`) turned out to need the ROLL's export instead, not a new kind.
//! * `stochrsi` — CLOSED 2026-11-16 (C14). Its state is three separate carries
//!   and each now comes from the node that owns it: the grafted `rsi`'s channels
//!   reach the parent through `Grafted::tails`, the two rolls' `(value, trail)`
//!   ride `RollMax`/`RollMin`, and the ring of past RSI values — which this list
//!   claimed no shape could export — was ALREADY captured by `RingSet::capture`
//!   for every group and only needed handing over in the `K_ROLLMAX | K_ROLLMIN`
//!   arm. A `MirrorBuffer` rebuilds from the primary slots alone, because its
//!   mirror half is a duplicate by construction.
//!
//! That closes the request kinds: 93 `Planable` impls, 93 `Restorable` impls and
//! 93 `restorer_for` arms, with no gap in either direction (measured 2026-11-16).
//! The entries above that still say "no variant" are bespoke recurrences no
//! REQUEST recipe composes today, not open state work.
//!
//! `Mirror` used to be on this list and no longer is: its value is the one thing
//! the steady loop had to be TOLD to keep, so `emit_scalar`'s Mirror arm stores it
//! into `v[k]` — the array `exec`'s `K_MIRROR` arm already reads — and the decode
//! reads it back. That store is the whole cost, and it is per bar per mirror.

use crate::cse::recipe::ChainState;
use crate::cse::signature::{
    Carries, Signature, Values, K_ACCUM, K_CYBERCYCLE, K_EMA, K_HIGHPASS, K_HMA,
    K_HOMODYNEDISCRIMINATOR, K_HTINPHASE, K_HTQUADRATURE, K_IIRCLAMPED, K_IIRCLAMPEDPLAIN,
    K_INSTANTANEOUSTRENDLINE, K_KVOFLOW, K_LINREGFLOW, K_MAMA, K_MIRROR, K_PEAKDECAY, K_PREV,
    K_PSAR, K_ROLLAGE, K_ROLLMAX, K_ROLLMIN, K_ROLLSUM, K_SDFTROTATE, K_SEEDEDEMA, K_SMA, K_STDDEV,
    K_SUPERSMOOTHER, K_SUPERTREND, K_VARIABLEALPHA, K_WILDERS, K_WMA, MAX,
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
        K_ROLLSUM => {
            // Group = `Values::ring_of[k]`, the leader the planner published.
            // A shared group's capacity is the MAX period across its members, so
            // a member can legitimately have `period < capacity` (`ao`'s 5-bar
            // SMA sits in the 34-bar group; `ultosc` has 7/14 in 28). Reading the
            // raw slots for such a member would hand back a buffer of the wrong
            // length, so the export is split in two by shape:
            //
            // * `p == capacity` — the raw slots ARE the member's window, and the
            //   consumer's own `index = cursor / 8` rebuild (`mass.rs`) applies
            //   unchanged;
            // * `p < capacity` — the member's window is the LAST `p` values in
            //   CHRONOLOGICAL order (`e.chrono`), and `cursor` is exported as 0.
            //   That is a valid equivalent `Buffer` state: after exactly `p`
            //   pushes into a capacity-`p` ring the write slot has wrapped back to
            //   0, which is precisely where the next (oldest-evicting) push goes.
            //
            // Either way an UNFILLED group is refused, because a partially warm
            // ring cannot be distinguished from a full one by its slots alone.
            let g = vals.ring_of[k] as usize;
            let e = c.rings.get(g).and_then(|o| o.as_ref())?;
            let p = vals.periods[k] as usize;
            if p == 0 || e.count != e.capacity || e.raw.len() < p {
                return None;
            }
            if p == e.capacity as usize {
                ChainState::RollSumRing {
                    sum: c.st[k],
                    ring: e.raw.clone(),
                    cursor: e.cursor,
                    period: vals.periods[k],
                }
            } else {
                if e.chrono.len() < p {
                    return None;
                }
                ChainState::RollSumRing {
                    sum: c.st[k],
                    ring: e.chrono[e.chrono.len() - p..].to_vec(),
                    cursor: 0,
                    period: vals.periods[k],
                }
            }
        }
        K_ROLLAGE => {
            // `adxr` asserts `ring.len() == n` and rebuilds `Buffer<Warm>` from
            // `index = cursor / 8`, so `n` is the EXPORTED capacity — not the
            // node's own period, which for an age roll is the ring's span.
            let g = vals.ring_of[k] as usize;
            let e = c.rings.get(g).and_then(|o| o.as_ref())?;
            if e.capacity == 0 || e.raw.len() != e.capacity as usize || e.count != e.capacity {
                return None;
            }
            ChainState::RollAge {
                ring: e.raw.clone(),
                cursor: e.cursor,
                n: e.capacity,
            }
        }
        K_HMA => {
            // Hma owns its ring at its OWN slot (the emitter feeds
            // `rings.feed_take_at(k, …)`, unlike the roll kinds which address
            // `ring_of[k]`), and it is `psqrt` long — a runtime length the
            // variant carries rather than a fixed array, which is exactly why
            // `Carries` is `Clone` and not `Copy`.
            let e = c.rings.get(k).and_then(|o| o.as_ref())?;
            if e.capacity == 0 || e.raw.len() != e.capacity as usize || e.count != e.capacity {
                return None;
            }
            let mut accs = [0.0f64; 6];
            accs.copy_from_slice(&c.hacc[k][..6]);
            ChainState::Hma {
                accs,
                cursor: e.cursor,
                ring: e.raw.clone(),
                period: vals.periods[k],
                p2: vals.roll_lb[k],
                psqrt: e.capacity,
            }
        }
        // The HD pipeline's whole state is already carried in one 38-cell
        // (`hd_acc[k]`) — five stage rings plus six IIR/period scalars — so the
        // export is the cell itself, and the consumer rebuilds its typed rings via
        // `homodynediscriminator::hd_state_from_acc`. `kvacc[k]` rides along as
        // `extras` because the composite kinds (ITL, MAMA) keep their own feedback
        // scalars there, and `st[k]` as `st` because MAMA's emitter parks its live
        // `alpha` in that slot for its companion `Fama` node.
        K_HOMODYNEDISCRIMINATOR | K_INSTANTANEOUSTRENDLINE | K_MAMA => ChainState::HdPipeline {
            acc: c.hd_acc[k].to_vec(),
            extras: c.kvacc[k],
            st: c.st[k],
        },
        K_IIRCLAMPED => ChainState::IirClamped { value: c.st[k] },
        // `IirClampedPlain` is the same carry spelled with plain multiply+add so
        // it matches a hand that does not fuse (`ccfisher`'s `val1`); the HOISTED
        // `{ident}_carry` is scattered back into `st[k]` exactly as `IirClamped`'s
        // is, so the shape is identical and no new variant is needed.
        K_IIRCLAMPEDPLAIN => ChainState::IirClamped { value: c.st[k] },
        // The sliding-DFT rotation carry: the emitter keeps the coupled triple in
        // `kvacc[k]` (`rp`, `ip`, and the cached `lead_sine` its `SdftLead`
        // companion reads), so the pair the hand state needs is exported as one
        // shape rather than split across slots.
        K_SDFTROTATE => {
            let [rp, ip, lead_sine] = c.kvacc[k];
            ChainState::SdftRotate { rp, ip, lead_sine }
        }
        // `Node::Prev`'s whole state IS one last-value carry — the emitter hoists
        // it into `{ident}_carry` and `carry_back` scatters it back into `st[k]`,
        // exactly like `IirClamped` — and the consumer's field is a bare `f64`
        // (`fisher::State.fish`). So it decodes as `Mirror`, the shape that means
        // "this slot's value at the seam", rather than needing a new variant: the
        // `recipes::fisher` doc has always described this as reading "through the
        // existing `ChainState::Mirror` shape", but the arm was never written, so
        // `fisher` refused its own tail (measured 2026-11-16: slots [17, 24],
        // kinds [28, 29]).
        K_PREV => ChainState::Mirror { value: c.st[k] },
        // `Node::Accum` is the same shape one step on: its state is ONE running
        // value (`obv`'s cumulative volume flow, `wad`'s, `vwap`'s PV and volume
        // sums), the emitter hoists it into `{ident}_carry`, and `carry_back`
        // scatters it into `st[k]` exactly as `Prev`/`IirClamped` do. The
        // consumers already match `ChainState::Mirror` — the shape that means
        // "this slot's value at the seam" — so no new variant is needed, and the
        // three kinds that were refusing on this arm (`obv`, `wad`, `vwap`) are
        // answered by the one line.
        K_ACCUM => ChainState::Mirror { value: c.st[k] },
        // CyberCycle's IIR feedback pair. `emit_scalar`'s steady arm writes
        // `kvacc[k] = [cycle, cycle_prev, 0.0]` at the END of the step, so slot 0
        // already IS the next bar's `cycle_prev` — which is why the variant's
        // fields are named after the hand's, not after the timeline.
        K_CYBERCYCLE => ChainState::CyberCycle {
            cycle_prev: c.kvacc[k][0],
            cycle_prev2: c.kvacc[k][1],
        },
        // `PeakDecay` is one running latch (`peak = max(prev * decay, src)`) kept
        // in `st[k]`, i.e. the same "this slot's value at the seam" the `Prev` and
        // `Accum` arms already spell as `Mirror` — `trendmode`'s and `ccfisher`'s
        // `pk` field reads it straight back.
        K_PEAKDECAY => ChainState::Mirror { value: c.st[k] },
        // A `VariableAlpha` carry is ONE running value whose blend weight is a LIVE
        // node (`w = alpha * short_sd / long_sd` for `vidya`), so unlike `Ema` there
        // is no `mults`/`invms` pair to export — just the slot. It is hoisted as
        // `{ident}_carry` and scattered back into `st[k]` in the same list as
        // `Prev`/`Accum` (`emit_scalar.rs`'s roll-hoist loop), and `vidya::State`'s
        // `prev_vidya` IS that scalar, so `Mirror` is the honest shape and no new
        // variant is needed.
        K_VARIABLEALPHA => ChainState::Mirror { value: c.st[k] },
        // Parabolic SAR's whole state is four scalars, and the emitter already
        // writes all four: `emit_scalar`'s `Kind::Psar` ends every bar with
        // `kvacc[k] = [psar, extream, accel]` and `st[k] = if uptrend { 1.0 }
        // else { 0.0 }`, which `warmup.rs`'s `K_PSAR` arm spells identically, so
        // the warm→steady handoff is one rule rather than two. The `psar` and
        // `extream` in the cell are POST-flip values (a reversal rewrites both in
        // the same bar), which is exactly what the hand's `calc` expects to read
        // back — nothing here re-derives them from a window.
        K_PSAR => ChainState::Psar {
            psar: c.kvacc[k][0],
            extream: c.kvacc[k][1],
            accel: c.kvacc[k][2],
            uptrend: c.st[k] != 0.0,
        },
        K_KVOFLOW => {
            // C5. The emitted steady arm stores `kvacc[k] = [hlc, cm, dm]` after
            // the bar, i.e. exactly the NEXT bar's `[prev_hlc, cm, prev_hl]`, and
            // the trend flip-flop rides the node's own `st[k]` as 1.0/0.0 — both
            // conventions read from `emit_scalar.rs`'s `Kind::KvoFlow` arm and
            // `warmup.rs`'s `K_KVOFLOW` arm, which agree (`st[k] < 0.5` = down in
            // both). `kvo::restore_state` matches `[Ema, Ema, KvoFlow]` and
            // `recipes::kvo` declares `tail_inputs = [short_ema, long_ema, vf]`,
            // so this arm is the third slot the restorer already expects.
            ChainState::KvoFlow {
                cm: c.kvacc[k][1],
                prev_hlc: c.kvacc[k][0],
                prev_hl: c.kvacc[k][2],
                trend: c.st[k] >= 0.5,
            }
        }
        // The value a mirror exists to publish. `emit_scalar`'s `Kind::Mirror`
        // arm stores it into `v[k]` every steady bar precisely so this line can
        // read it — before that the copy was dead code and the chunk-exit state
        // for `supertrend`/`rsi`/`cmo`/`dm`-style shapes was unrecoverable.
        K_MIRROR => ChainState::Mirror { value: c.v[k] },
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
        K_ROLLMAX | K_ROLLMIN => {
            // A roll over a DERIVED source reads its window from a ring group, and
            // that group is already exported (every group is, by `RingSet::capture`
            // at both epilogue exits) — so the window rides along here. A roll over
            // a RAW input column reads the stream itself: no group exists, so it
            // exports an empty window rather than a fabricated one, and its value
            // and trail alone remain the whole state it carries.
            //
            // An UNFILLED group exports no window either: a partially warm ring
            // cannot be rebuilt into a full buffer, and `RollSumRing` refuses that
            // case outright.
            let g = vals.ring_of[k] as usize;
            let (ring, cursor) = match c.rings.get(g).and_then(|o| o.as_ref()) {
                Some(e) if e.count == e.capacity && e.raw.len() == e.capacity as usize => {
                    (e.raw.clone(), e.cursor)
                }
                _ => (Vec::new(), 0),
            };
            if sig.kinds[k] == K_ROLLMAX {
                ChainState::RollMax {
                    value: c.st[k],
                    trail: c.trail[k] as u64,
                    period,
                    look_back: vals.roll_lb[k],
                    ring,
                    cursor,
                }
            } else {
                ChainState::RollMin {
                    value: c.st[k],
                    trail: c.trail[k] as u64,
                    period,
                    look_back: vals.roll_lb[k],
                    ring,
                    cursor,
                }
            }
        }
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
        K_HTINPHASE | K_HTQUADRATURE => {
            // The shared 7-tap ring, exported CHRONOLOGICALLY (`chrono`) rather
            // than in slot order: `HilbertTransform::restore_state` rebuilds a
            // `FixedRingBuffer` by PUSHING values in order and that container
            // carries no cursor field, so a raw-order copy would rebuild a
            // rotated ring and print plausible-but-wrong taps behind a green
            // parity suite. The group is `ring_of[k]` (the leader the planner
            // published for the shared `roofing` source).
            let g = vals.ring_of[k] as usize;
            let e = c.rings.get(g).and_then(|o| o.as_ref())?;
            if e.chrono.len() != 7 || e.count != e.capacity {
                return None;
            }
            ChainState::HtRing {
                ring: e.chrono.clone(),
            }
        }
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
