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

use super::signature::{K_CONST, K_HMA, K_INPUT, K_ROLLMAX, K_ROLLMIN, K_ROLLSUM, K_UNUSED, MAX};

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

/// How many groups one fused container may carry. The fused read costs
/// `B × N` ages (one shared age list applied to every series, because
/// `push_with_info_periods` takes ONE period array), while fusion saves one
/// cursor advance per extra series: `B` series cost `B−1` advances saved and
/// `(B−1) × N` loads added. Past a handful of series the loads win, so the
/// partition stops here instead of bundling the whole arena.
pub const MAX_BUNDLE_SERIES: usize = 4;

/// One fused container: the ring groups that share a single multi-series
/// buffer, and therefore one push and ONE cursor advance per bar.
///
/// A `MultiBuffer<B, T, S>` holds `B` value vectors under a single
/// `index`/`capacity` and its push takes all `B` values up front, so a bundle
/// is legal exactly when
///
/// * every group is [`RCLASS_SUM`] — LOCKSTEP: same bar, same age layout. A
///   rescan group's triggers fire on different bars per lane, and only the
///   per-lane contiguous mirror window can serve them;
/// * the groups are mutually value-independent — no group's roll reads another
///   group's roll output *in the same bar*. Stoch's `d_line = RollSum{k_line}`
///   is the serial case: `k_line`'s value for this bar does not exist yet when
///   the fused push would have to be issued, which is why the hand kernel uses
///   two single buffers and why that is correct, not unoptimised;
/// * every source slot precedes every member slot (`max src < min member`):
///   the fused call is emitted at the bundle's FIRST member, so all `B` values
///   must already be bound there.
///
/// Capacity is not a condition and does not appear: the container is sized by
/// the max member period over the whole bundle (a VALUE, on `Values`), and a
/// fused read is bit-exact at any capacity ≥ the age asked for because the age
/// index is absolute from the write cursor — a series whose own group is
/// smaller simply has older slots it never reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bundle {
    /// Group leader slots, arena order. A group's SERIES index in the fused
    /// container is its position here.
    pub groups: Vec<usize>,
    /// Every member slot of every group, arena order — this order IS the fused
    /// call's age list, so member `m` reads `results[series][rank]` with
    /// `rank` its position here.
    pub members: Vec<usize>,
}

impl Bundle {
    /// The fused call site: the bundle's first member slot.
    pub fn call_site(&self) -> usize {
        self.members[0]
    }
    /// Which series of the container `group`'s values are pushed into.
    pub fn series_of(&self, group: usize) -> Option<usize> {
        self.groups.iter().position(|&g| g == group)
    }
    /// Where `member`'s own period sits in the fused age list.
    pub fn rank_of(&self, member: usize) -> Option<usize> {
        self.members.iter().position(|&m| m == member)
    }
}

/// The container plan: the bundles of TWO OR MORE groups only. A group left
/// out keeps the plain single-series vector buffer — the fused form buys
/// nothing for one series, so it must not be paid for.
///
/// A pure function of the same arena arrays [`ring_facts`] reads, so it is
/// determined by the key exactly like the class table: two arenas that erase
/// to one `Signature` get one identical bundle list.
pub fn bundles(
    kinds: &[u8; MAX],
    a: &[u32; MAX],
    b: &[u32; MAX],
    c: &[u32; MAX],
    d: &[u32; MAX],
    e: &[u32; MAX],
    len: usize,
) -> Vec<Bundle> {
    let (class, leader) = ring_facts(kinds, a, len);

    // Pure-sum group leaders, arena order — the only class that may fuse.
    let mut sums: Vec<usize> = Vec::new();
    let mut members: Vec<Vec<usize>> = Vec::new();
    for k in 0..len {
        if class[k] == RCLASS_SUM && leader[k] as usize == k {
            let mut ms = Vec::new();
            for j in 0..len {
                if class[j] == RCLASS_SUM && leader[j] as usize == k {
                    ms.push(j);
                }
            }
            sums.push(k);
            members.push(ms);
        }
    }

    // Dependencies: a group reads another group's output iff some member's
    // source has a member of that group among its ancestors.
    let deps: Vec<Vec<usize>> = sums
        .iter()
        .zip(members.iter())
        .map(|(&g, ms)| {
            let mut out = Vec::new();
            for &m in ms {
                for h in 0..sums.len() {
                    if sums[h] == g || out.contains(&sums[h]) {
                        continue;
                    }
                    if reaches(kinds, a, b, c, d, e, len, a[m] as usize, &members[h]) {
                        out.push(sums[h]);
                    }
                }
            }
            out
        })
        .collect();

    let mut out: Vec<Bundle> = Vec::new();
    let mut src_max: Vec<usize> = Vec::new(); // max source slot per bundle
    for gi in 0..sums.len() {
        let g = sums[gi];
        let lo = members[gi].iter().copied().min().unwrap_or(g);
        let hi = members[gi]
            .iter()
            .map(|&m| a[m] as usize)
            .max()
            .unwrap_or(usize::MAX);
        let mut placed = false;
        for bi in 0..out.len() {
            if out[bi].groups.len() >= MAX_BUNDLE_SERIES {
                continue;
            }
            // Neither direction may read the other's roll output.
            let conflict = out[bi].groups.iter().any(|&h| {
                deps[gi].contains(&h)
                    || deps[sums.iter().position(|&s| s == h).unwrap()].contains(&g)
            });
            // The shared call site is the bundle's first member, so every
            // source in the merged bundle must precede it.
            let ssa = hi < out[bi].call_site() && src_max[bi] < lo;
            if !conflict && ssa {
                let ms = members[gi].clone();
                out[bi].groups.push(g);
                out[bi].members.extend(ms);
                out[bi].members.sort();
                src_max[bi] = src_max[bi].max(hi);
                placed = true;
                break;
            }
        }
        if !placed {
            out.push(Bundle {
                groups: vec![g],
                members: members[gi].clone(),
            });
            src_max.push(hi);
        }
    }
    // Only real fusions: a one-group bundle stays on the single-series buffer.
    out.retain(|b| b.groups.len() > 1);
    out
}

/// Does `start` reach any of `targets` through the arena's operand edges?
/// A `0` edge reads as "no edge": slot 0 can never be a ring member (a member
/// always comes after its own source), so skipping it cannot hide a
/// dependency between two groups.
fn reaches(
    kinds: &[u8; MAX],
    a: &[u32; MAX],
    b: &[u32; MAX],
    c: &[u32; MAX],
    d: &[u32; MAX],
    e: &[u32; MAX],
    len: usize,
    start: usize,
    targets: &[usize],
) -> bool {
    let mut seen = [false; MAX];
    let mut stack = vec![start];
    while let Some(k) = stack.pop() {
        if targets.contains(&k) {
            return true;
        }
        if seen[k] || kinds[k] == K_UNUSED {
            continue;
        }
        seen[k] = true;
        for &edge in &[a[k], b[k], c[k], d[k], e[k]] {
            let j = edge as usize;
            if j != 0 && j < len && !seen[j] {
                stack.push(j);
            }
        }
    }
    false
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

    /// The same CMO arena on the container axis: two independent sums whose
    /// sources both precede both members — ONE fused container, and the fused
    /// age list is the member list in arena order.
    #[test]
    fn independent_sums_share_one_container() {
        let bs = bundled(
            &[K_INPUT, K_SUB, K_MAX, K_MAX, K_ROLLSUM, K_ROLLSUM],
            &[0, 0, 1, 1, 2, 3],
        );
        assert_eq!(bs.len(), 1, "the gain and loss sums fuse");
        assert_eq!(bs[0].groups, vec![4, 5], "series order is arena order");
        assert_eq!(bs[0].members, vec![4, 5]);
        assert_eq!(bs[0].call_site(), 4, "one call, at the first member");
        assert_eq!(bs[0].series_of(5), Some(1));
        assert_eq!(bs[0].rank_of(5), Some(1));
    }

    /// STOCH's law, stated as the refusal: `d_line = RollSum{k_line}` reads
    /// `k_line`'s output for the SAME bar, so no fused push can carry both —
    /// which is why the hand kernel keeps two single buffers. The plan must
    /// publish that refusal, not leave it to the emitter's judgement.
    #[test]
    fn a_serial_roll_chain_refuses_to_bundle() {
        let bs = bundled(&[K_INPUT, K_ROLLSUM, K_ROLLSUM], &[0, 0, 1]);
        assert!(
            bs.is_empty(),
            "k_line -> d_line is a serial dependency: two containers"
        );
    }

    /// The SSA guard: a second group whose source is computed AFTER the first
    /// member sits cannot join its container — the fused push would have to
    /// read a value that does not exist yet at the call site.
    #[test]
    fn a_late_source_blocks_the_container() {
        // 0=Input, 1=RollSum over 0, 2=Add over 1, 3=RollSum over 2
        let bs = bundled(&[K_INPUT, K_ROLLSUM, K_ADD, K_ROLLSUM], &[0, 0, 1, 2]);
        assert!(bs.is_empty(), "slot 3's source is slot 2, after slot 1");
    }

    /// Two sums over a shared derived column are ONE group (AO), so the
    /// container plan says nothing about them — the group fold already fuses.
    #[test]
    fn one_group_is_not_a_bundle() {
        let bs = bundled(
            &[K_INPUT, K_ADD, K_DIV, K_ROLLSUM, K_ROLLSUM],
            &[0, 0, 1, 2, 2],
        );
        assert!(bs.is_empty(), "one group is already one container");
    }

    /// The HMA machine's bespoke ring and a rescan group are both outside the
    /// container plan: only pure-sum lockstep groups may fuse.
    #[test]
    fn only_pure_sum_groups_are_bundled() {
        let bs = bundled(
            &[K_INPUT, K_ADD, K_ROLLSUM, K_ROLLMAX, K_HMA],
            &[0, 0, 2, 2, 1],
        );
        assert!(bs.is_empty(), "one sum only: nothing to fuse with");
    }

    /// The container plan is a function of the key alone, like the class
    /// table: the same arena always yields the same bundles.
    #[test]
    fn bundles_are_a_pure_function_of_the_key() {
        let kinds = [K_INPUT, K_SUB, K_MAX, K_MAX, K_ROLLSUM, K_ROLLSUM];
        let a = [0u32, 0, 1, 1, 2, 3];
        assert_eq!(bundled(&kinds, &a), bundled(&kinds, &a));
    }

    /// Container plan over a `kinds`/`a` arena (every other edge array is
    /// zero — the shapes under test carry no multi-operand nodes beyond `a`).
    fn bundled(kinds: &[u8], a: &[u32]) -> Vec<Bundle> {
        let mut k = [K_UNUSED; MAX];
        let mut aa = [0u32; MAX];
        for (i, &x) in kinds.iter().enumerate() {
            k[i] = x;
            aa[i] = a[i];
        }
        bundles(&k, &aa, &aa, &aa, &aa, &aa, kinds.len())
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
