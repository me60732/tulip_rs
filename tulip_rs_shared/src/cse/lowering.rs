//! The lowering plan — the planner-owned ring-layout facts the arena's
//! structure determines and NOTHING else. No recipe writes any of it: like
//! [`super::merge`]'s `sharing_groups`, it is a pure function of the merged
//! arena's edges, so a new shape changes the plan only by changing the arena,
//! and a new parameterization of a known shape changes nothing.
//!
//! One implementation, three readers ([`ring_facts`], a `const fn` over the
//! key's own arrays):
//! * `Signature::build` derives the published fields at compile time, so a
//!   `SIG_*` literal cannot be built whose plan disagrees with its arena;
//! * `erase` derives the same fields for a runtime arena, and binds
//!   `Values::ring_of` from the same leader table — the key and the runtime
//!   binding are then one fact seen twice, never two derivations;
//! * the emitters read the published fields at expansion instead of scanning
//!   node kinds for a layout decision.
//!
//! The hard invariant follows directly: two arenas that erase to the same
//! `Signature` lower identically, because the layout fact IS part of the key
//! and there is no second implementation to drift.
//!
//! Capacity deliberately does NOT live here: capacity = max member period is
//! a NUMBER (rev-2 law: values never enter the pattern), and it rides
//! `Values::ring_cap` at the group's leader slot. This module owns the
//! partition, the class, and the laws that follow from the class.

use super::signature::{K_CONST, K_HMA, K_INPUT, K_ROLLMAX, K_ROLLMIN, K_ROLLSUM, MAX};

/// The slot is not ring-backed (not a roll at all, or a price-sourced
/// extremum, whose window is a stream slice — the hand's `max.rs` pointer
/// form, no ring, no group).
pub const RCLASS_NONE: u8 = 0;
/// Every member is a rolling sum: lanes advance in lockstep (same bar, same
/// age, same cap), so the leave is ONE vector read and the buffer is NOT a
/// mirror — sums never rescan, and the mirror's second store would be pure
/// tax. Per group per steady bar, this class costs one store.
pub const RCLASS_SUM: u8 = 1;
/// The group carries a `RollMax`/`RollMin` over a derived source: its rescan
/// triggers are per-lane and fire on different bars (disjoint in time), so
/// the lane-major layout with a per-lane contiguous mirror window is required
/// and lockstep vector reads are illegal. Costs two stores per bar (the
/// mirror) and pays only when a lane's trail expires.
pub const RCLASS_RESCAN: u8 = 2;
/// The Hull MA machine's own `√p` diff ring: a group of one, fed once per bar
/// by the node whose tail consumes it. Batching an of-one window measured
/// +48% slower, so this class stays out of the batched fold.
pub const RCLASS_BESPOKE: u8 = 3;

/// The source slot `k`'s ring feeds, or `usize::MAX` when `k` is not
/// ring-backed. A price-sourced extremum is NOT ring-backed; the `Hma`
/// machine keys its diff ring on itself. The one membership rule the
/// partition, the published classes, the replay leaders and `erase`'s group
/// table all share.
const fn ring_src(kinds: &[u8; MAX], a: &[u32; MAX], k: usize) -> usize {
    match kinds[k] {
        K_ROLLSUM => a[k] as usize,
        K_ROLLMAX | K_ROLLMIN => {
            if kinds[a[k] as usize] == K_INPUT {
                usize::MAX
            } else {
                a[k] as usize
            }
        }
        K_HMA => k,
        _ => usize::MAX,
    }
}

/// A member's own class contribution: a rescan-capable extremum lifts its
/// whole group; the HMA machine is bespoke (and alone in its group).
const fn member_class(kinds: &[u8; MAX], k: usize) -> u8 {
    match kinds[k] {
        K_ROLLMAX | K_ROLLMIN => RCLASS_RESCAN,
        K_HMA => RCLASS_BESPOKE,
        _ => RCLASS_SUM,
    }
}

/// The published ring plan, per arena slot:
/// * `class[k]` — the class of `k`'s group (`RCLASS_NONE` where `k` is not
///   ring-backed). A group containing any rescan member is rescan-class for
///   EVERY member: the shared ring's layout is one choice per group.
/// * `leader[k]` — `k`'s group's leader slot, the first ring member over the
///   same source in arena order (the slot `Values::ring_cap` sizes and
///   `Values::ring_of` points members back to). `0` where `k` is not
///   ring-backed.
///
/// A `const fn` over the key's own arrays — expansion time (`build`, host
/// reads) and runtime (`erase`) compute the identical fact through this one
/// function.
pub const fn ring_facts(kinds: &[u8; MAX], a: &[u32; MAX], len: usize) -> ([u8; MAX], [u32; MAX]) {
    let mut class = [RCLASS_NONE; MAX];
    let mut leader = [0u32; MAX];
    let mut k = 0;
    while k < len {
        let src = ring_src(kinds, a, k);
        if src != usize::MAX {
            // Group leader = the earliest ring member over this source.
            let mut ld = k;
            let mut j = 0;
            while j < k {
                if ring_src(kinds, a, j) == src {
                    ld = j;
                    break;
                }
                j += 1;
            }
            let mc = member_class(kinds, k);
            class[ld] = promote_class(class[ld], mc);
            class[k] = mc; // patched to the group class in the second pass
            leader[k] = ld as u32;
        }
        k += 1;
    }
    // Second pass: every member carries its GROUP's class, not its own.
    let mut k = 0;
    while k < len {
        if class[k] != RCLASS_NONE {
            let ld = leader[k] as usize;
            class[k] = class[ld];
        }
        k += 1;
    }
    (class, leader)
}

/// The ring plan as a named value: what the planner attaches to a pattern and
/// both renderers read, in place of scanning node kinds for a layout decision.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Plan {
    pub class: [u8; MAX],
    pub leader: [u32; MAX],
    /// Slot reads its `a` operand straight off a PRICE stream (a chain, `Lag`
    /// or roll whose source is `K_INPUT`) — the `Price`/`Chain` stream mode the
    /// lane classifier computed today, published for both renderers: it
    /// decides raw-stream hoists, lane-group splat sources, and the per-slot
    /// seed read (`t − p` off the stream vs a chain local).
    pub price_src: [bool; MAX],
}

impl Plan {
    /// The one derivation, from the same arrays `Signature::build` and `erase`
    /// publish through.
    pub const fn compute(kinds: &[u8; MAX], a: &[u32; MAX], len: usize) -> Plan {
        let (class, leader) = ring_facts(kinds, a, len);
        let mut price_src = [false; MAX];
        let mut k = 0;
        while k < len {
            // A node reads a price stream iff its `a` operand is an Input
            // slot. Leaves (`Input`/`Const`) have no read operand.
            price_src[k] =
                kinds[k] != K_INPUT && kinds[k] != K_CONST && kinds[a[k] as usize] == K_INPUT;
            k += 1;
        }
        Plan {
            class,
            leader,
            price_src,
        }
    }
    /// Is this slot ring-backed at all (the renderer's `derived_roll` /
    /// pointer-vs-ring layout question, now a read instead of a scan)?
    pub fn is_ring(&self, k: usize) -> bool {
        self.class[k] != RCLASS_NONE
    }
    /// Does this ring-backed slot's group carry a rescan (lane-major + mirror
    /// authority; the vector-element layout is illegal for it)?
    pub fn is_rescan(&self, k: usize) -> bool {
        self.class[k] == RCLASS_RESCAN
    }
    /// Does this ring-backed slot's group take the batched engine (everything
    /// but the HMA machine's of-one window, which measured slower batched)?
    pub fn is_batched(&self, k: usize) -> bool {
        self.class[k] != RCLASS_NONE && self.class[k] != RCLASS_BESPOKE
    }
    /// LOCKSTEP law: every lane advances on the same bar, age and capacity, so
    /// the leave is one vector read — true exactly for pure-sum groups. A
    /// rescan group's triggers fire on different bars per lane, which is what
    /// makes the vector layout illegal there.
    pub fn is_lockstep(&self, k: usize) -> bool {
        self.class[k] == RCLASS_SUM
    }
    /// MIRROR law: only min/max buffers mirror (a rescan walks one lane's
    /// contiguous chronological window); sums never rescan, so the mirror's
    /// second store would be pure tax — one store per group per bar for them.
    pub fn takes_mirror(&self, k: usize) -> bool {
        self.class[k] == RCLASS_RESCAN
    }
    /// The distinct group leaders in arena order — the replay/skip-list walk
    /// both renderers do over the ring engine's groups.
    pub fn leaders(&self, len: usize) -> Vec<usize> {
        let mut out = Vec::new();
        for k in 0..len {
            if self.class[k] != RCLASS_NONE && self.leader[k] as usize == k {
                out.push(k);
            }
        }
        out
    }
    /// Does slot `k` read its source straight off a price stream?
    pub fn is_price_src(&self, k: usize) -> bool {
        self.price_src[k]
    }
    /// The group led by `leader`, as member slots in arena order — which IS
    /// the fused call's lane order: member `j` reads `leaves[j]`. Reads the
    /// published leader table; it decides nothing.
    pub fn members_of(&self, leader: usize, len: usize) -> Vec<usize> {
        (0..len)
            .filter(|&j| self.class[j] != RCLASS_NONE && self.leader[j] as usize == leader)
            .collect()
    }
    /// Is `k` its group's leader — the one slot allowed to push the ring this
    /// bar (arena order guarantees it precedes every sibling)?
    pub fn is_leader(&self, k: usize) -> bool {
        self.class[k] != RCLASS_NONE && self.leader[k] as usize == k
    }
}

/// Group-class merge: a rescan member makes the whole group rescan-class;
/// `RCLASS_NONE` at a leader means "this is the first member". The `Hma`
/// machine never shares a group (it keys on itself), so its class is final.
const fn promote_class(cur: u8, mc: u8) -> u8 {
    if cur == RCLASS_NONE {
        mc
    } else if cur == RCLASS_RESCAN || mc == RCLASS_RESCAN {
        RCLASS_RESCAN
    } else {
        cur
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cse::signature::{Signature, K_ABS, K_ADD, K_DIV, K_MAX, K_SUB, K_TRAIL, K_UNUSED};

    /// `Signature`-shaped arena for the plan tests: `kinds` beyond its head
    /// is `K_UNUSED`, every edge array is 0 — the plan reads only `len`,
    /// `kinds` and `a`.
    fn facts(kinds: &[u8], a: &[u32]) -> ([u8; MAX], [u32; MAX]) {
        let mut k = [K_UNUSED; MAX];
        let mut aa = [0u32; MAX];
        for (i, &x) in kinds.iter().enumerate() {
            k[i] = x;
            aa[i] = a[i];
        }
        ring_facts(&k, &aa, kinds.len())
    }

    /// A price-sourced extremum (the hand's `max.rs` shape) is not a ring:
    /// the plan has no group for it — only the derived rolls do.
    #[test]
    fn pointer_rolls_get_no_group() {
        // 0=Input(high), 1=RollMax over 0, 2=RollSum over 3, 3=Add(derived col)
        let (class, leader) = facts(&[K_INPUT, K_ROLLMAX, K_ROLLSUM, K_ADD], &[0, 0, 3, 0]);
        assert_eq!(
            class[1], RCLASS_NONE,
            "price-sourced extremum: pointer form"
        );
        assert_eq!(class[2], RCLASS_SUM);
        assert_eq!(leader[2], 2, "it leads its own group");
        assert_eq!(leader[1], 0);
    }

    /// AO's two means over the ONE interned medprice column: two members,
    /// one group, leader = first in arena order, lockstep (vector layout
    /// legal) and non-mirror (sums never rescan).
    #[test]
    fn two_sums_share_one_group() {
        // 0=Input, 1=Add, 2=Div, 3=RollSum(long), 4=RollSum(short), 5=Sub
        let (class, leader) = facts(
            &[K_INPUT, K_ADD, K_DIV, K_ROLLSUM, K_ROLLSUM, K_SUB],
            &[0, 0, 1, 2, 2, 4],
        );
        assert_eq!(class[3], RCLASS_SUM);
        assert_eq!(class[4], RCLASS_SUM);
        assert_eq!(leader[3], 3);
        assert_eq!(leader[4], 3, "the sibling points at the leader");
    }

    /// Fisher's extrema pair over the derived medprice column: rescan class —
    /// lane-major layout and mirror required, lockstep illegal.
    #[test]
    fn derived_extrema_make_the_group_rescan() {
        let (class, leader) = facts(
            &[K_INPUT, K_ADD, K_DIV, K_ROLLMAX, K_ROLLMIN],
            &[0, 0, 1, 2, 2],
        );
        assert_eq!(class[3], RCLASS_RESCAN);
        assert_eq!(class[4], RCLASS_RESCAN);
        assert_eq!(leader[4], 3);
    }

    /// The ao+fisher co-request showcase: sums AND extrema intern onto the
    /// SAME medprice source, so ONE group of four members serves all four
    /// rolls — one ring feed per bar, and the group class (not the per-member
    /// kinds) is what forces the mirror layout on the sums too.
    #[test]
    fn sums_and_extrema_over_one_source_share_one_group() {
        let (class, leader) = facts(
            &[
                K_INPUT, K_ADD, K_DIV, K_ROLLSUM, K_ROLLSUM, K_ROLLMAX, K_ROLLMIN,
            ],
            &[0, 0, 1, 2, 2, 2, 2],
        );
        assert_eq!(leader[5], 3, "the first roll over the source leads");
        assert_eq!(leader[6], 3);
        assert_eq!(
            class[3], RCLASS_RESCAN,
            "one rescan member makes the whole group rescan"
        );
        assert_eq!(
            class[4], RCLASS_RESCAN,
            "the sum sibling takes the group class"
        );
    }

    /// CMO's two raw sums over the DISTINCT gain/loss columns: two groups,
    /// each lockstep and pure-sum — the shape whose lane layout is the
    /// vector-element one.
    #[test]
    fn distinct_columns_open_distinct_groups() {
        let (class, leader) = facts(
            &[K_INPUT, K_SUB, K_MAX, K_MAX, K_ROLLSUM, K_ROLLSUM],
            &[0, 0, 1, 1, 2, 3],
        );
        assert_eq!(class[4], RCLASS_SUM);
        assert_eq!(class[5], RCLASS_SUM);
        assert_eq!(leader[4], 4, "own leader");
        assert_eq!(leader[5], 5, "own leader");
    }

    /// The HMA machine keys its `√p` diff ring on ITSELF — a group of one,
    /// bespoke class, out of the batched fold (measured slower there).
    #[test]
    fn hma_diff_ring_is_a_bespoke_group_of_one() {
        let (class, leader) = facts(&[K_INPUT, K_ADD, K_HMA], &[0, 0, 1]);
        assert_eq!(class[2], RCLASS_BESPOKE);
        assert_eq!(leader[2], 2, "keyed on the HMA node itself");
    }

    /// The trail-rescan hub (ichimoku-shaped): every roll reads a price
    /// column directly, so the arena carries NO ring group at all.
    #[test]
    fn pointer_hub_has_no_groups() {
        let (class, leader) = facts(
            &[
                K_INPUT, K_ROLLMAX, K_TRAIL, K_ROLLMIN, K_TRAIL, K_ABS, K_SUB,
            ],
            &[0, 0, 1, 0, 3, 4, 2],
        );
        assert!(class.iter().all(|&c| c == RCLASS_NONE));
        assert!(leader.iter().all(|&l| l == 0));
    }

    /// The publication law: a class is only defined at ring-backed slots, and
    /// the leader slot's value equals the group class.
    #[test]
    fn classes_are_written_at_group_slots_only() {
        let (class, leader) = facts(
            &[
                K_INPUT, K_ADD, K_DIV, K_ROLLSUM, K_ROLLSUM, K_ROLLMAX, K_ROLLMIN, K_HMA,
            ],
            &[0, 0, 1, 2, 2, 2, 2, 1],
        );
        assert_eq!(class[3], RCLASS_RESCAN, "sums+extrema share leader 3");
        assert_eq!(class[0], RCLASS_NONE, "a price node is not a group");
        assert_eq!(class[1], RCLASS_NONE);
        assert_eq!(class[7], RCLASS_BESPOKE);
        assert_eq!(leader[3], 3);
        assert_eq!(leader[7], 7);
    }

    /// The invariant, stated as a test: the plan is a function of the key
    /// alone, so re-deriving from the same `Signature` cannot disagree — the
    /// check a second (hidden) implementation of the layout decision would
    /// defeat. `Signature::build` fills the published arrays through this
    /// same function, which is what makes the generated const and the erased
    /// signature one fact.
    #[test]
    fn plan_is_a_pure_function_of_the_key() {
        let kinds = [K_INPUT, K_ADD, K_DIV, K_ROLLSUM, K_ROLLMAX];
        let a = [0u32, 0, 1, 2, 2];
        let (c1, l1) = facts(&kinds, &a);
        let (c2, l2) = facts(&kinds, &a);
        assert_eq!(c1, c2);
        assert_eq!(l1, l2);
        // Through the real key constructor: `build` publishes the same arrays.
        let mut ka = [K_UNUSED; MAX];
        let mut aa = [0u32; MAX];
        for i in 0..kinds.len() {
            ka[i] = kinds[i];
            aa[i] = a[i];
        }
        let sig = Signature::build(&ka, &aa, &aa, &aa, &aa, &aa, &[0; 1], 1);
        assert_eq!(sig.ring_class, c1);
        assert_eq!(sig.ring_leader, l1);
    }
}
