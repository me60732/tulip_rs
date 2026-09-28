//! SIMD lane-packing primitives for the CSE planner (proposal §7).
//!
//! The planner lowers the CSE'd arena to a *physical plan* of recurrence
//! **groups**: several same-kind nodes (e.g. `ema12(close)` and `ema24(close)`)
//! step together as the lanes of one `Simd<f64, N>` recurrence, so one vector
//! op per bar advances N EMAs. A group's lanes need not share a source — the
//! *pack key is the canonical equation only* (§7.3), so `ema13(real)` and
//! `ema12(ad)` pack together; each lane gathers its own per-bar input.
//!
//! This module is the **foundation**: the width cap, the bucketing, and the two
//! horizontal-combine strategies a packed group needs (e.g. `macd_line =
//! lane0 − lane1`). The full executor that drives a physical plan sits on top of
//! this. It proves the load-bearing bit-exactness premise here too: a
//! `Simd<f64, N>` lane is bit-identical to the matching scalar recurrence, so
//! vectorizing the planner cannot change results — only speed.
//!
//! # The two horizontal-combine strategies (§7.4 experiment)
//! When two lanes of one group feed an arithmetic node, the scalar value has to
//! be extracted. Two ways, both bit-exact to the scalar `Sub` (asserted in
//! `sub_reduce_matches_sub_extract`):
//! * [`sub_extract`] — `to_array(); a[0] - a[1]` (the `adosc.rs` shape).
//! * [`sub_reduce`] — `(v * [1,-1]).reduce_sum()` (stays in the vector).
//! We benchmark both (`cse/simd_sub_*`) to pick the executor's lowering.

use crate::cse::{fuse, Id, Merged, Node, QueryState, Resolved, Results};
use crate::indicator_types::{TSimdState, TState};
use crate::indicators::ema::State as EmaState;
use crate::indicators::simd_indicators::ema_simd::{multiplier_simd, SimdState};
use crate::types::Warm;
use std::collections::HashMap;
use std::simd::{prelude::SimdFloat, Simd};

/// Compile-time width cap for a single lane-packed `Simd<f64, N>` recurrence.
///
/// AVX2's widest f64 vector is 4 lanes (256-bit); an AVX-512 host (8 lanes)
/// opts in via the `avx512` Cargo feature. `lane_buckets` splits a group into
/// power-of-two chunks each ≤ this cap, mirroring `road_train`'s width ladder.
pub const MAX_LANES: usize = if cfg!(feature = "avx512") { 8 } else { 4 };

/// Split a runtime lane count `n` into power-of-two chunks, each ≤ [`MAX_LANES`],
/// greedily largest-first — the §7.1 bucketing rule. `MAX_LANES == 4` ⇒ `6 →
/// [4, 2]`; `MAX_LANES == 8` ⇒ `6 → [4, 2]` too (6 is not a power of two), but a
/// group of 8 becomes a single `Simd<f64, 8>` instead of `[4, 4]`.
pub fn lane_buckets(n: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut left = n;
    while left > 0 {
        // Largest power of two ≤ min(left, MAX_LANES).
        let mut w = MAX_LANES.min(left);
        while !w.is_power_of_two() {
            w -= 1;
        }
        out.push(w);
        left -= w;
    }
    out
}

/// Horizontal `a - b` of the first two lanes, scalar path (`adosc.rs` style).
#[inline(always)]
pub fn sub_extract(v: Simd<f64, 2>) -> f64 {
    let a = v.to_array();
    a[0] - a[1]
}

/// Horizontal `lane0 - lane1` staying in the vector: dot with `[1, -1]`.
///
/// `reduce_sum` of two lanes is `e0 + e1` with `e = v * [1,-1]`, and `v0 + (−v1)`
/// is bit-identical to `v0 − v1` (IEEE negation is exact) — hence equal to
/// [`sub_extract`] on every input.
#[inline(always)]
pub fn sub_reduce(v: Simd<f64, 2>) -> f64 {
    (v * Simd::from_array([1.0, -1.0])).reduce_sum()
}

/// A 2-lane EMA group over one shared source — the executor's smallest unit and
/// the micro-bench's test bed. Both lanes advance as one vector recurrence; the
/// two combine strategies differ only in how `macd_line` is recovered.
pub struct EmaPair {
    multipliers: (Simd<f64, 2>, Simd<f64, 2>),
    state: Option<SimdState<2>>,
}

impl EmaPair {
    pub fn new([a, b]: [usize; 2]) -> Self {
        Self {
            multipliers: multiplier_simd([a, b]),
            state: None,
        }
    }

    /// Advance both lanes by one `x`, seeding from `x` on the first bar — the
    /// exact `advance_ema` lifecycle (`new(x)` reports `x`, later bars `calc`).
    #[inline(always)]
    fn advance(&mut self, x: f64) -> Simd<f64, 2> {
        match &mut self.state {
            Some(s) => s.calc(Simd::splat(x)),
            None => {
                let s = SimdState::new(Simd::splat(x), self.multipliers);
                let v = s.ema;
                self.state = Some(s);
                v
            }
        }
    }

    /// Advance, then recover `lane_a - lane_b` via the scalar-extract strategy.
    #[inline(always)]
    pub fn step_sub_extract(&mut self, x: f64) -> f64 {
        sub_extract(self.advance(x))
    }

    /// Advance, then recover `lane_a - lane_b` via the SIMD-reduce strategy.
    #[inline(always)]
    pub fn step_sub_reduce(&mut self, x: f64) -> f64 {
        sub_reduce(self.advance(x))
    }
}

// ── PackedPlan: the merged SIMD executor (recurrence axis) ───────────────────
//
// Lanes = different periods/sources of the SAME node kind on ONE asset. The
// planner packs same-`(level, init_bar)` `Ema` nodes into one `Simd<f64, N>`
// recurrence: one vector op per bar advances N EMAs. The `level` (longest path
// from the leaves) guarantees a group never consumes its own lanes — every
// source is at a strictly lower level, so at any bar the group's row values
// are already computed. The shared `init_bar` (= the sources' defined-from
// lead) gives the whole vector ONE seed bar: that is the recurrence axis'
// "wave" machinery — `ema12/ema26(close)` seed at bar 0 together, while a
// later wave (macd's `signal = ema9(macd_line)`, init 25) can never join the
// earlier vector because its `init_bar` differs → it stays a scalar node.
//
// `fuse.rs` remains the bit-exact oracle; this executor mirrors its
// prologue/main emit shape and must reproduce its `Results` for every recipe
// (asserted by the `packed_*` tests, and by every existing `CompiledPlan`
// test once `query.rs` wires `run`/`resume` here).

/// One reporting sink — the same shape/publish rule as `fuse::Sink`
/// (`max(declared, node_lead)`); kept local so the packed executor is
/// self-contained and `fuse` stays untouched.
#[derive(Debug, Clone, Copy)]
struct Sink {
    node: Id,
    lead: Id,
    /// Optional-output switch (see [`crate::cse::Resolved::optional_enabled`]).
    /// Disabled optional sinks keep their slot in `Results` but print an EMPTY
    /// array; primary sinks are always enabled.
    enabled: bool,
}

/// One lane of a packed EMA group.
#[derive(Debug, Clone, Copy)]
struct Member {
    /// Arena node id of the EMA (the row write target).
    id: Id,
    /// Arena id of its producer (the row read source).
    src: Id,
    /// EMA period (determines the lane's multiplier).
    period: Id,
    /// The node's own defined-from lead (the row gate).
    lead: Id,
}

/// A width-`N` lane group. All lanes share the same `level` and `init_bar`
/// (the §7.3 pack key), so they seed on the SAME bar and no lane feeds
/// another. The level only matters at lowering time (it places the group's
/// step in the level-major walk), so it is not stored here.
#[derive(Debug, Clone)]
struct Group<const N: usize> {
    init_bar: Id,
    members: [Member; N],
    multipliers: (Simd<f64, N>, Simd<f64, N>),
}

/// Build-time only: how a node participates in the packed plan. Consumed once
/// by [`PackedPlan::lower`] to lay out the per-level execution lists, then
/// dropped — the runtime walk is those lists, so scalar nodes cost exactly one
/// [`Node`] match per bar (the `fuse` shape) and groups call [`step_group`]
/// directly, with **no extra per-node dispatch tax**.
#[derive(Debug, Clone, Copy)]
enum NodeKind {
    Scalar,
    /// A non-head group member: its row is written when the head ran, so the
    /// node contributes nothing at runtime.
    GroupMember,
    /// The lowest-id member of group `(width, index)`: advances the whole
    /// vector op at its arena position.
    GroupHead(u8, u32),
}

/// One group to advance as a unit, with a width-tagged dispatch only at the
/// call site (one per group per bar, not per node).
#[derive(Debug, Clone, Copy)]
struct GroupStep {
    width: u8,
    index: u32,
}

/// One level's execution list, in arena-id order. Nodes of one level are
/// mutually independent (a dependency would put the consumer one level higher)
/// and all their sources live at lower levels, so executing level-major is a
/// valid topological order of the arena.
#[derive(Debug, Clone, Default)]
struct LevelPlan {
    scalars: Vec<Id>,
    groups: Vec<GroupStep>,
}

/// Per-asset resumable state for the packed executor. SIMD lane states live in
/// the group vectors (one `Option` per group); `scalar_ema` keeps the leftover
/// scalar EMAs, keyed by arena node id (packed node slots stay `None` — their
/// state lives in the lanes).
pub(crate) struct PackedState {
    groups2: Vec<Option<SimdState<2>>>,
    groups4: Vec<Option<SimdState<4>>>,
    groups8: Vec<Option<SimdState<8>>>,
    scalar_ema: Vec<Option<EmaState<Warm>>>,
    next_bar: Id,
}

/// The packed physical plan (`proposal.md` §6 Step A). Built once per
/// `Request` at compile time; data-length-independent.
pub(crate) struct PackedPlan {
    arena: Merged,
    /// Per-node defined-from lead (absolute bar), for the scalar steps' gate.
    leads: Vec<Id>,
    groups2: Vec<Group<2>>,
    groups4: Vec<Group<4>>,
    groups8: Vec<Group<8>>,
    /// Per-level execution lists (scalar ids + group steps), level-major.
    levels: Vec<LevelPlan>,
    primary: Vec<Sink>,
    optional: Vec<Sink>,
    max_publish_lead: Id,
    counts: Vec<(usize, usize)>,
}

impl PackedPlan {
    /// Lower the merged arena to a packed physical plan. Returns `None` when
    /// no ≥2-lane group forms — callers fall back to the scalar fused plan.
    pub(crate) fn lower(arena: &Merged, leads: &[Id], resolved: &[Resolved]) -> Option<Self> {
        let nodes = &arena.nodes;
        let n = nodes.len();

        // Level per node: 0 for Input/Const, else 1 + max(child levels).
        // SSA order ⇒ every child is processed before its parent. Levels are
        // both the pack key's first component AND the execution order: the
        // plan runs level-major, so no node reads a row its level hasn't
        // produced yet.
        let mut levels = vec![0u32; n];
        let mut max_level = 0u32;
        for (id, node) in nodes.iter().enumerate() {
            let l = match node {
                Node::Input { .. } | Node::Const { .. } => 0,
                Node::Ema { src, .. } => levels[*src as usize] + 1,
                Node::Sub { a, b }
                | Node::Mul { a, b }
                | Node::Div { a, b }
                | Node::Max { a, b } => levels[*a as usize].max(levels[*b as usize]) + 1,
                Node::HighPass { input, .. } => levels[*input as usize] + 1,
                Node::SuperSmoother { src, .. } => levels[*src as usize] + 1,
                Node::HtInPhase { src } | Node::HtQuadrature { src } => levels[*src as usize] + 1,
            };
            levels[id] = l;
            max_level = max_level.max(l);
        }

        // Pack key = (level, init_bar = the source's defined-from lead): same
        // level ⇒ no intra-group dependency; same init ⇒ one seed bar for the
        // whole vector (§7.3). Members are collected in arena-id order —
        // deterministic across runs, which keeps persisted `QueryState` ids
        // trustworthy.
        let mut buckets: HashMap<(u32, Id), Vec<Member>> = HashMap::new();
        for (id, node) in nodes.iter().enumerate() {
            if let Node::Ema { src, period } = *node {
                buckets
                    .entry((levels[id], leads[src as usize]))
                    .or_default()
                    .push(Member {
                        id: id as Id,
                        src,
                        period,
                        lead: leads[id],
                    });
            }
        }
        let mut keys: Vec<(u32, Id)> = buckets.keys().copied().collect();
        keys.sort_unstable();

        let mut groups2: Vec<Group<2>> = Vec::new();
        let mut groups4: Vec<Group<4>> = Vec::new();
        let mut groups8: Vec<Group<8>> = Vec::new();
        let mut kinds: Vec<NodeKind> = vec![NodeKind::Scalar; n];
        let mut any = false;

        for (level, init_bar) in keys {
            let members = &buckets[&(level, init_bar)];
            if members.len() < 2 {
                continue; // singleton waves stay scalar
            }
            // Greedy pow-2 buckets, largest first, ≤ MAX_LANES (§7.1); a
            // leftover width-1 bucket remains a scalar op.
            let mut off = 0;
            for w in lane_buckets(members.len()) {
                let chunk = &members[off..off + w];
                off += w;
                if w == 1 {
                    continue;
                }
                any = true;
                match w {
                    2 => push_group(&mut groups2, init_bar, chunk, &mut kinds),
                    4 => push_group(&mut groups4, init_bar, chunk, &mut kinds),
                    8 => push_group(&mut groups8, init_bar, chunk, &mut kinds),
                    _ => unreachable!("lane_buckets yields pow2 ≤ MAX_LANES ∈ {{4,8}}"),
                }
            }
        }
        if !any {
            return None;
        }

        // Execution lists, level-major: every scalar node id in arena order,
        // plus one group step per group at its members' level.
        let mut level_plans: Vec<LevelPlan> = vec![LevelPlan::default(); max_level as usize + 1];
        for (id, kind) in kinds.iter().enumerate() {
            match *kind {
                NodeKind::Scalar => level_plans[levels[id] as usize].scalars.push(id as Id),
                NodeKind::GroupMember => {} // its rows are written by the head step
                NodeKind::GroupHead(width, index) => level_plans[levels[id] as usize]
                    .groups
                    .push(GroupStep { width, index }),
            }
        }

        let mut primary: Vec<Sink> = Vec::new();
        let mut optional: Vec<Sink> = Vec::new();
        let mut counts: Vec<(usize, usize)> = Vec::with_capacity(resolved.len());
        let mut max_publish_lead = 0u32;
        for res in resolved {
            for (oi, &node) in res.outputs.iter().enumerate() {
                let node_lead = leads[node as usize];
                let declared = res.output_leads.get(oi).copied().unwrap_or(node_lead);
                let lead = node_lead.max(declared);
                primary.push(Sink {
                    node,
                    lead,
                    enabled: true,
                });
                max_publish_lead = max_publish_lead.max(lead);
            }
            for (oi, &node) in res.optional_outputs.iter().enumerate() {
                let node_lead = leads[node as usize];
                let declared = res
                    .optional_output_leads
                    .get(oi)
                    .copied()
                    .unwrap_or(node_lead);
                let lead = node_lead.max(declared);
                let enabled = res.optional_enabled.get(oi).copied().unwrap_or(true);
                optional.push(Sink {
                    node,
                    lead,
                    enabled,
                });
                if enabled {
                    max_publish_lead = max_publish_lead.max(lead);
                }
            }
            counts.push((res.outputs.len(), res.optional_outputs.len()));
        }

        Some(PackedPlan {
            arena: arena.clone(),
            leads: leads.to_vec(),
            groups2,
            groups4,
            groups8,
            levels: level_plans,
            primary,
            optional,
            max_publish_lead,
            counts,
        })
    }

    /// Recurrence-axis cost gate — whether this plan is worth running in
    /// place of the scalar fused interpreter.
    ///
    /// Measured (2026-09, this machine, packed-vs-scalar-fused, same binary):
    /// - 2 lanes of 12 nodes (`cse/run_one_asset`): 155µs vs 80µs.
    /// - 6 lanes of 7 EMAs (`Simd<4>+Simd<2>` dashboard): 172µs vs 88µs.
    /// - 10 lanes of 13 nodes (`cse/run_dense_dashboard`): 265µs vs 163µs.
    ///
    /// The vector op repays its gather/extract/branch overhead only if the
    /// interpreter machinery around it disappears; at every density the
    /// packed run loses because packing removes only the O(P) EMA dispatch
    /// while its own structure tax scales with arena size. The threshold
    /// below is therefore a TEMPORARY fence: it keeps realistic workloads on
    /// the scalar oracle path (no regression), lets a genuinely dense pack be
    /// built + A/B'd, and must be revisited the moment a winning shape
    /// exists (a typed fused kernel with no row/sink dispatch, or the
    /// cross-asset axis), which is where the recurrence-axis work converges.
    pub(crate) fn worth_it(&self) -> bool {
        let packed = self.groups2.len() * 2 + self.groups4.len() * 4 + self.groups8.len() * 8;
        let n_ema = self
            .arena
            .nodes
            .iter()
            .filter(|n| matches!(n, Node::Ema { .. }))
            .count();
        packed >= 8 && packed * 2 >= n_ema
    }

    /// A fresh per-asset execution state (`next_bar = 0`). Production enters
    /// through [`Self::state_from_query`] on the caller's `QueryState` (see
    /// `CompiledPlan::advance`); this is the test-facing shortcut.
    #[cfg(test)]
    pub(crate) fn new_state(&self) -> PackedState {
        PackedState {
            groups2: vec![None; self.groups2.len()],
            groups4: vec![None; self.groups4.len()],
            groups8: vec![None; self.groups8.len()],
            scalar_ema: vec![None; self.arena.nodes.len()],
            next_bar: 0,
        }
    }

    /// Fused lane-packed streaming interpreter: prologue (aligned, gated) +
    /// main (gate-free), mirroring `FusedPlan::advance`'s emit shape. A
    /// carried `next_bar` drives `inline`; groups whose waves already seeded
    /// keep their `SimdState`. Bit-identical to the scalar path for every
    /// packed recipe (asserted by the `packed_*` tests).
    pub(crate) fn advance(&self, state: &mut PackedState, series: &[&[f64]]) -> Results {
        let new_len = series.iter().map(|s| s.len()).max().unwrap_or(0);
        if new_len == 0 {
            return Results::default();
        }
        let new_len = new_len as Id;
        let start = state.next_bar;
        // Relative bar at which the last sink comes online; `[0, inline)` is the
        // prologue, `[inline, new_len)` the flat main loop (empty by
        // construction when the chunk ends before the last sink is online).
        let inline = self.max_publish_lead.saturating_sub(start).min(new_len);

        let mut p_out: Vec<Vec<f64>> = Vec::with_capacity(self.primary.len());
        let mut p_rel: Vec<Id> = Vec::with_capacity(self.primary.len());
        for s in &self.primary {
            let rel = s.lead.saturating_sub(start);
            p_out.push(crate::uninit_vec!(
                f64,
                new_len.saturating_sub(rel) as usize
            ));
            p_rel.push(rel);
        }
        let mut o_out: Vec<Vec<f64>> = Vec::with_capacity(self.optional.len());
        let mut o_rel: Vec<Id> = Vec::with_capacity(self.optional.len());
        for s in &self.optional {
            // Disabled optional sink: EMPTY array in declared position,
            // `rel = Id::MAX` so the gated prologue can never select it.
            if !s.enabled {
                o_out.push(Vec::new());
                o_rel.push(Id::MAX);
                continue;
            }
            let rel = s.lead.saturating_sub(start);
            o_out.push(crate::uninit_vec!(
                f64,
                new_len.saturating_sub(rel) as usize
            ));
            o_rel.push(rel);
        }

        let n = self.arena.nodes.len();
        let mut row = vec![f64::NAN; n];

        // ---- Prologue [0, inline): gated; sinks write once online. ----
        for rel in 0..inline {
            let bar = start + rel;
            // Level-major: scalar nodes evaluate exactly like `fuse` (one
            // `Node` match each); group heads advance the whole lane vector
            // in one op. Nodes of one level are mutually independent, so this
            // is a valid topological walk.
            for level in &self.levels {
                self.run_level(level, bar, rel, series, state, &mut row, true);
            }
            for (si, s) in self.primary.iter().enumerate() {
                if rel >= p_rel[si] {
                    unsafe {
                        *p_out[si].get_unchecked_mut((rel - p_rel[si]) as usize) =
                            row[s.node as usize];
                    }
                }
            }
            for (si, s) in self.optional.iter().enumerate() {
                if rel >= o_rel[si] {
                    unsafe {
                        *o_out[si].get_unchecked_mut((rel - o_rel[si]) as usize) =
                            row[s.node as usize];
                    }
                }
            }
        }

        // ---- Main loop [inline, new_len): every node defined, every sink
        // online (inline == max rel_lead ≥ every rel_lead), so write indices
        // are in-bounds and each slot is written exactly once. Branch-free,
        // index-stores only — the `cycle_macd` hot-loop shape. ----
        for rel in inline..new_len {
            let bar = start + rel;
            for level in &self.levels {
                self.run_level(level, bar, rel, series, state, &mut row, false);
            }
            for (si, s) in self.primary.iter().enumerate() {
                unsafe {
                    *p_out[si].get_unchecked_mut((rel - p_rel[si]) as usize) = row[s.node as usize];
                }
            }
            for (si, s) in self.optional.iter().enumerate() {
                // Disabled sinks printed an empty vec; `rel − o_rel` would
                // underflow against `Id::MAX`, so skip unconditionally.
                if !s.enabled {
                    continue;
                }
                unsafe {
                    *o_out[si].get_unchecked_mut((rel - o_rel[si]) as usize) = row[s.node as usize];
                }
            }
        }

        state.next_bar = start + new_len;

        // Regroup the flat sink vectors into `Results`' request-major shape.
        let mut outputs = Vec::with_capacity(self.counts.len());
        let mut optional = Vec::with_capacity(self.counts.len());
        for &(np, no) in &self.counts {
            outputs.push(p_out.drain(..np).collect());
            optional.push(o_out.drain(..no).collect());
        }
        Results { outputs, optional }
    }

    /// Execute one level of the walk: scalar evals first (one `Node` match per
    /// node, the `fuse` shape — gated only in the prologue), then each group
    /// head advances its whole lane vector in a direct call. No per-node
    /// dispatch beyond the scalar [`Node`] match itself.
    #[inline]
    fn run_level(
        &self,
        level: &LevelPlan,
        bar: Id,
        rel: Id,
        series: &[&[f64]],
        state: &mut PackedState,
        row: &mut [f64],
        gated: bool,
    ) {
        for &id in &level.scalars {
            let v = self.eval_scalar(id as usize, rel, series, state, row);
            row[id as usize] = if gated && bar < self.leads[id as usize] {
                f64::NAN
            } else {
                v
            };
        }
        for gs in &level.groups {
            match gs.width {
                2 => step_group(
                    &self.groups2[gs.index as usize],
                    &mut state.groups2[gs.index as usize],
                    bar,
                    row,
                ),
                4 => step_group(
                    &self.groups4[gs.index as usize],
                    &mut state.groups4[gs.index as usize],
                    bar,
                    row,
                ),
                8 => step_group(
                    &self.groups8[gs.index as usize],
                    &mut state.groups8[gs.index as usize],
                    bar,
                    row,
                ),
                _ => unreachable!("group widths are 2/4/8"),
            }
        }
    }

    /// Scalar (unpacked) node eval — the `fuse` gated/steady core. Packed Ema
    /// node ids never arrive here (they're absorbed into group steps).
    #[inline]
    fn eval_scalar(
        &self,
        id: usize,
        rel: Id,
        series: &[&[f64]],
        state: &mut PackedState,
        row: &[f64],
    ) -> f64 {
        match self.arena.nodes[id] {
            // Input read is chunk-relative (`rel`); lead/publish gates use
            // the absolute `bar` in the caller's loop — the §5.3 streaming
            // invariant.
            Node::Input { stream } => series
                .get(stream as usize)
                .and_then(|s| s.get(rel as usize))
                .copied()
                .unwrap_or(f64::NAN),
            Node::Const { bits } => f64::from_bits(bits),
            Node::Ema { src, period } => {
                let p = row[src as usize];
                if p.is_nan() {
                    return f64::NAN; // producer not yet reported
                }
                fuse::advance_ema(&mut state.scalar_ema[id], p, period)
            }
            Node::Sub { a, b } => row[a as usize] - row[b as usize],
            Node::Mul { a, b } => row[a as usize] * row[b as usize],
            Node::Div { a, b } => row[a as usize] / row[b as usize],
            // f64::max (NaN-propagates-other), matching ppo.rs `.max(EPSILON)`.
            Node::Max { a, b } => row[a as usize].max(row[b as usize]),
            // Filter/Hilbert kinds: not executed yet (no recipe uses them).
            _ => f64::NAN,
        }
    }

    /// Rebuild a [`PackedState`] from a saved [`QueryState`] — `resume`
    /// interop. Lane states are reconstructed from the per-node scalar states
    /// (multipliers included, so the vector math continues bit-identically).
    pub(crate) fn state_from_query(&self, qs: &QueryState) -> PackedState {
        PackedState {
            groups2: self.groups2.iter().map(|g| gather(g, &qs.ema)).collect(),
            groups4: self.groups4.iter().map(|g| gather(g, &qs.ema)).collect(),
            groups8: self.groups8.iter().map(|g| gather(g, &qs.ema)).collect(),
            scalar_ema: qs.ema.clone(),
            next_bar: qs.next_bar,
        }
    }

    /// Project a finished [`PackedState`] back to the caller-facing
    /// [`QueryState`] (arena-keyed, one `EmaState` per node). Packed lanes
    /// become `EmaState::new(lane_value, period)` — the identical state the
    /// scalar path would hold (lane bits are proven equal; multipliers derive
    /// from the period in both paths).
    pub(crate) fn state_to_query(&self, ps: PackedState) -> QueryState {
        let mut ema = ps.scalar_ema;
        project(&self.groups2, &ps.groups2, &mut ema);
        project(&self.groups4, &ps.groups4, &mut ema);
        project(&self.groups8, &ps.groups8, &mut ema);
        QueryState {
            ema,
            next_bar: ps.next_bar,
        }
    }
}

/// Insert one pow-2 bucket of `Member`s as a width-`N` group (the members
/// come from the bucket in arena-id order, so lanes are deterministically
/// ordered). The lowest-id member becomes the group head — the node whose
/// execution position advances the whole vector; the rest are marked as
/// members whose rows the head step writes.
fn push_group<const N: usize>(
    groups: &mut Vec<Group<N>>,
    init_bar: Id,
    chunk: &[Member],
    kinds: &mut [NodeKind],
) {
    let mut members = [Member {
        id: 0,
        src: 0,
        period: 0,
        lead: 0,
    }; N];
    members.copy_from_slice(chunk);
    let periods: [usize; N] = std::array::from_fn(|i| members[i].period as usize);
    let multipliers = multiplier_simd(periods);
    let index = groups.len() as u32;
    for (i, m) in members.iter().enumerate() {
        kinds[m.id as usize] = if i == 0 {
            NodeKind::GroupHead(N as u8, index)
        } else {
            NodeKind::GroupMember
        };
    }
    groups.push(Group {
        init_bar,
        members,
        multipliers,
    });
}

/// Advance one lane group at absolute bar `bar` — the wave/seed machinery.
///
/// - `bar < init_bar`: the wave hasn't started — lanes stay NaN and the state
///   is untouched (the seed fires exactly at `init_bar`).
/// - `bar == init_bar`: seed `SimdState::new(gathered_srcs, muls)` — each lane
///   seeds from its own source value, identical to N scalar `EmaState::new`s.
/// - `bar > init_bar`: one vector `calc`.
///
/// The per-lane node lead gates what lands in `row`: lanes with different
/// periods report from different bars (e.g. `ema12` at 11 vs `ema26` at 25),
/// matching scalar `fuse` where the state advances while gated-out.
#[inline]
fn step_group<const N: usize>(
    group: &Group<N>,
    state: &mut Option<SimdState<N>>,
    bar: Id,
    row: &mut [f64],
) {
    if bar < group.init_bar {
        for m in &group.members {
            row[m.id as usize] = f64::NAN;
        }
        return;
    }
    let mut xs = [0.0f64; N];
    for (i, m) in group.members.iter().enumerate() {
        xs[i] = row[m.src as usize];
    }
    // Packed sources stay valid from `init_bar` onward (equal-length input
    // streams). Ragged multi-stream data is out of scope for the packed
    // executor — the scalar path remains the oracle for it.
    debug_assert!(
        !xs.iter().any(|x| x.is_nan()),
        "packed group source NaN at bar {bar} (ragged multi-stream input?)"
    );
    let v = Simd::from_array(xs);
    let out = match state {
        Some(s) => s.calc(v),
        None => {
            let s = SimdState::new(v, group.multipliers);
            let o = s.ema;
            *state = Some(s);
            o
        }
    };
    let arr = out.to_array();
    for (i, m) in group.members.iter().enumerate() {
        row[m.id as usize] = if bar >= m.lead { arr[i] } else { f64::NAN };
    }
}

/// Gather one group's per-lane `EmaState`s out of the arena-keyed `QueryState`
/// into a `SimdState` (all lanes always seed together on `init_bar`, so the
/// states are all-`Some` or all-`None` — anything else is a desync bug).
fn gather<const N: usize>(g: &Group<N>, ema: &[Option<EmaState<Warm>>]) -> Option<SimdState<N>> {
    let mut owned: [Option<EmaState<Warm>>; N] = std::array::from_fn(|_| None);
    for (i, m) in g.members.iter().enumerate() {
        owned[i] = ema[m.id as usize].clone();
    }
    if owned.iter().any(Option::is_none) {
        return None;
    }
    let mut refs: Vec<&mut EmaState<Warm>> =
        owned.iter_mut().map(|o| o.as_mut().unwrap()).collect();
    Some(SimdState::from_states(&mut refs))
}

/// Scatter a finished group's lane values into the arena-keyed `ema` vec.
fn project<const N: usize>(
    groups: &[Group<N>],
    states: &[Option<SimdState<N>>],
    ema: &mut [Option<EmaState<Warm>>],
) {
    for (g, s) in groups.iter().zip(states) {
        if let Some(s) = s {
            let arr = s.ema.to_array();
            for (i, m) in g.members.iter().enumerate() {
                ema[m.id as usize] = Some(EmaState::new(arr[i], m.period as usize).into_warm());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cse::{exec, finalize, fuse::FusedPlan, merge, Planable, Recipe};
    use crate::indicator_types::Indicator;
    use crate::indicators::apo::Apo;
    use crate::indicators::ema::{calc, multiplier};
    use crate::indicators::macd::Macd;
    use crate::indicators::ppo::Ppo;

    fn series(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| (i as f64 * 0.7).sin() * 10.0 + 100.0)
            .collect()
    }

    /// `lane_buckets` is a power-of-two split under the cap (the §7.1 ladder),
    /// summing back to `n`.
    #[test]
    fn lane_buckets_are_pow2_within_cap() {
        for n in 0..=20usize {
            let b = lane_buckets(n);
            assert_eq!(
                b.iter().sum::<usize>(),
                n,
                "buckets don't sum: {n} -> {b:?}"
            );
            for &w in &b {
                assert!(w.is_power_of_two(), "non-pow2 width {w} for {n}");
                assert!(w <= MAX_LANES, "width {w} exceeds cap {MAX_LANES} for {n}");
            }
        }
        // The user's two concrete cases (cap 4 default):
        assert_eq!(lane_buckets(2), vec![2]);
        assert_eq!(lane_buckets(6), vec![4, 2]);
    }

    /// LOAD-BEARING PREMISE: a 2-lane `Simd` EMA is bit-identical to two scalar
    /// EMAs advanced in lockstep. The planner only vectorizes if this holds; if
    /// it ever breaks, `SimdState`'s formula drifted from `ema::calc`.
    #[test]
    fn simd_group_matches_scalar_ema_bitwise() {
        let (pa, pb) = (12usize, 26usize);
        let data = series(200);

        // Reference scalar recurrences: seed = first value, then `calc` each
        // bar — exactly the cse `advance_ema` lifecycle.
        let (ma, ia) = multiplier(pa);
        let (mb, ib) = multiplier(pb);
        let mut sa = vec![0.0f64; data.len()];
        let mut sb = vec![0.0f64; data.len()];
        sa[0] = data[0];
        sb[0] = data[0];
        for i in 1..data.len() {
            sa[i] = calc(data[i], sa[i - 1], ma, ia);
            sb[i] = calc(data[i], sb[i - 1], mb, ib);
        }

        // SIMD pair (both lanes on the same source) must match bit-for-bit.
        let mut pair = EmaPair::new([pa, pb]);
        for (i, &x) in data.iter().enumerate() {
            let lane = pair.advance(x).to_array();
            assert_eq!(lane[0].to_bits(), sa[i].to_bits(), "lane a @ bar {i}");
            assert_eq!(lane[1].to_bits(), sb[i].to_bits(), "lane b @ bar {i}");
        }
        // Keep the `Warm` state path honest: the SIMD lane mirrors the stateful
        // `into_warm` scalar `ema`, not a cold recompute.
        let _ = EmaState::new(data[0], pa).into_warm();
    }

    /// The two horizontal-subtract strategies are bit-identical to each other
    /// and to the scalar difference — so choosing between them is purely a
    /// *speed* decision for the executor, never a correctness one.
    #[test]
    fn sub_reduce_matches_sub_extract() {
        let data = series(300);
        let mut e = EmaPair::new([12, 26]);
        let mut r = EmaPair::new([12, 26]);
        for &x in &data {
            let ex = e.step_sub_extract(x);
            let rd = r.step_sub_reduce(x);
            assert_eq!(ex.to_bits(), rd.to_bits(), "extract vs reduce @ {x}");
        }
        // Direct: both horizontal helpers agree on arbitrary vectors.
        for k in 0..64 {
            let v = Simd::from_array([k as f64 * 1.3, 100.0 - k as f64]);
            assert_eq!(sub_extract(v).to_bits(), sub_reduce(v).to_bits());
        }
    }

    // ── PackedPlan (recurrence axis) ────────────────────────────────────────

    fn cmp_exact(a: &[f64], b: &[f64], what: &str) {
        assert_eq!(a.len(), b.len(), "{what}: len {} vs {}", a.len(), b.len());
        for i in 0..a.len() {
            assert_eq!(a[i].to_bits(), b[i].to_bits(), "{what}: mismatch at {i}");
        }
    }

    fn cmp_results(a: &Results, b: &Results, what: &str) {
        assert_eq!(a.outputs.len(), b.outputs.len(), "{what}: request count");
        for req in 0..a.outputs.len() {
            assert_eq!(
                a.outputs[req].len(),
                b.outputs[req].len(),
                "{what}: req {req} outputs"
            );
            for out in 0..a.outputs[req].len() {
                cmp_exact(&a.outputs[req][out], &b.outputs[req][out], what);
            }
            assert_eq!(
                a.optional[req].len(),
                b.optional[req].len(),
                "{what}: req {req} optional"
            );
            for out in 0..a.optional[req].len() {
                cmp_exact(&a.optional[req][out], &b.optional[req][out], what);
            }
        }
    }

    /// Merged arena helper: `Apo(12,26) + Ppo(12,26) + Macd(12,26,9)` — the
    /// recipe set every packed test keys off (resolved index 0 = Apo, 1 = Ppo,
    /// 2 = Macd).
    fn merged_plan() -> (Merged, Vec<Resolved>, Vec<Id>) {
        let (m, r) = merge(&[
            Apo::recipe(&[12.0, 26.0]),
            Ppo::recipe(&[12.0, 26.0]),
            Macd::recipe(&[12.0, 26.0, 9.0]),
        ]);
        let leads = m.leads();
        (m, r, leads)
    }

    fn long_series(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| ((i as f64 * 0.7).sin() * 10.0 + 100.0) * (1.0 + (i % 7) as f64 * 0.0003))
            .collect()
    }

    /// The §7.3 pack key in action: `ema12/ema26(close)` are one wave-0
    /// `Simd<2>` group; the chained `ema9(macd_line)` is a later wave
    /// (init 25 ≠ 0) so it can't ride the same vector and stays scalar.
    #[test]
    fn packed_lower_packs_shared_source_emas() {
        let (m, r, leads) = merged_plan();
        let p = PackedPlan::lower(&m, &leads, &r).expect("Apo+Ppo+Macd must pack");
        assert_eq!(p.groups2.len(), 1, "one Simd<2> wave-0 group");
        assert!(p.groups4.is_empty() && p.groups8.is_empty());
        let g = &p.groups2[0];
        assert_eq!(g.init_bar, 0, "close is defined from bar 0");
        assert_eq!(g.members.len(), 2);
        assert_eq!(g.members[0].period, 12);
        assert_eq!(g.members[1].period, 26);
        assert_eq!(
            g.members[0].src, g.members[1].src,
            "both lanes read the same producer"
        );
        // Defined-from leads gate each lane's row write separately.
        assert_eq!(g.members[0].lead, 11);
        assert_eq!(g.members[1].lead, 25);
        // macd signal (a chained EMA at level 3, init 25) must be scalar.
        let signal_id = r[2].outputs[1];
        assert!(matches!(m.nodes[signal_id as usize], Node::Ema { .. }));
        assert!(
            !g.members.iter().any(|m| m.id == signal_id),
            "signal EMA must not join the close wave"
        );
    }

    /// THE gate: the packed executor is bit-identical to the scalar fused
    /// path, to the `exec`+`finalize` oracle (primary AND optional), and to the
    /// hand indicators (macd's chained signal/histogram included) on a long
    /// run.
    #[test]
    fn packed_advance_matches_fused_and_oracle() {
        let close = long_series(5000);
        let s: [&[f64]; 1] = [close.as_slice()];

        let (m, r, leads) = merged_plan();
        let fused = FusedPlan::build(&m, &leads, &r);
        let packed = PackedPlan::lower(&m, &leads, &r).expect("groups");
        let mut qs = QueryState {
            ema: vec![None; m.nodes.len()],
            next_bar: 0,
        };
        let mut ps = packed.new_state();
        let a = fused.advance(&mut qs, &s);
        let b = packed.advance(&mut ps, &s);
        cmp_results(&a, &b, "packed_vs_fused");

        // Oracle: exec + finalize on the same arena.
        let t = exec(&m, &s);
        for (req, res) in r.iter().enumerate() {
            let want = finalize(&t, &res.outputs, &res.output_leads);
            for out in 0..res.outputs.len() {
                cmp_exact(&b.outputs[req][out], &want[out], "packed_vs_oracle");
            }
        }

        // Hand indicators — the CSE-soundness proof on the packed path.
        let apo = Apo::indicator(&s, &[12.0, 26.0], None).unwrap().0;
        let ppo = Ppo::indicator(&s, &[12.0, 26.0], None).unwrap().0;
        let macd = Macd::indicator(&s, &[12.0, 26.0, 9.0], None).unwrap().0;
        cmp_exact(&b.outputs[0][0], &apo[0], "packed_apo");
        cmp_exact(&b.outputs[1][0], &ppo[0], "packed_ppo");
        cmp_exact(&b.outputs[2][0], &macd[0], "packed_macd_line");
        cmp_exact(&b.outputs[2][1], &macd[1], "packed_macd_signal");
        cmp_exact(&b.outputs[2][2], &macd[2], "packed_macd_histogram");
    }

    /// Streaming contract on the packed executor: chunks split at ANY boundary
    /// re-concatenate to the single full run, bit-exact, with state
    /// round-tripping through the caller-facing `QueryState` between chunks
    /// (the Step-B projection). Covers the empty-prologue fast path (first
    /// split past the 33-bar lead) and the short-chunk prologue path (20 bars,
    /// where the macd sinks are not yet online).
    #[test]
    fn packed_resume_round_trips_query_state() {
        let close: Vec<f64> = (0..200)
            .map(|i| (i as f64 * 0.7).sin() * 10.0 + 100.0)
            .collect();
        let (m, r, leads) = merged_plan();
        let packed = PackedPlan::lower(&m, &leads, &r).expect("groups");
        let s_all: [&[f64]; 1] = [close.as_slice()];
        let mut full_ps = packed.new_state();
        let full = packed.advance(&mut full_ps, &s_all);

        // 90, 140 split (90 > max_publish_lead 33 → resumes 2–3 take the
        // empty-prologue fast path).
        let s1: [&[f64]; 1] = [&close[..90]];
        let s2: [&[f64]; 1] = [&close[90..140]];
        let s3: [&[f64]; 1] = [&close[140..]];

        let mut ps = packed.new_state();
        let head = packed.advance(&mut ps, &s1);
        let qs = packed.state_to_query(ps);
        let mut ps = packed.state_from_query(&qs);
        let mid = packed.advance(&mut ps, &s2);
        let qs = packed.state_to_query(ps);
        let mut ps = packed.state_from_query(&qs);
        let tail = packed.advance(&mut ps, &s3);

        for req in 0..full.outputs.len() {
            for out in 0..full.outputs[req].len() {
                let mut combined = head.outputs[req][out].clone();
                combined.extend_from_slice(&mid.outputs[req][out]);
                combined.extend_from_slice(&tail.outputs[req][out]);
                cmp_exact(&combined, &full.outputs[req][out], "split_vs_full_primary");
            }
            for out in 0..full.optional[req].len() {
                let mut combined = head.optional[req][out].clone();
                combined.extend_from_slice(&mid.optional[req][out]);
                combined.extend_from_slice(&tail.optional[req][out]);
                cmp_exact(
                    &combined,
                    &full.optional[req][out],
                    "split_vs_full_optional",
                );
            }
        }

        // Short chunk (20 < max_publish_lead 33): late sinks stay empty in the
        // head; the resume completes the arrays exactly.
        let s_h: [&[f64]; 1] = [&close[..20]];
        let s_rest: [&[f64]; 1] = [&close[20..]];
        let mut ps = packed.new_state();
        let head = packed.advance(&mut ps, &s_h);
        let qs = packed.state_to_query(ps);
        let mut ps = packed.state_from_query(&qs);
        let tail = packed.advance(&mut ps, &s_rest);

        for req in 0..full.outputs.len() {
            for out in 0..full.outputs[req].len() {
                assert_eq!(
                    head.outputs[req][out].len() + tail.outputs[req][out].len(),
                    full.outputs[req][out].len(),
                    "short-chunk len @ req {req} out {out}"
                );
                let mut combined = head.outputs[req][out].clone();
                combined.extend_from_slice(&tail.outputs[req][out]);
                cmp_exact(&combined, &full.outputs[req][out], "short_chunk_primary");
            }
            for out in 0..full.optional[req].len() {
                let mut combined = head.optional[req][out].clone();
                combined.extend_from_slice(&tail.optional[req][out]);
                cmp_exact(&combined, &full.optional[req][out], "short_chunk_optional");
            }
        }
    }

    /// Six distinct-period EMAs on `close` pack as `Simd<4> + Simd<2>` (also
    /// under `avx512` — 6 is not a power of two), the chained
    /// `ema9(macd_line)` stays scalar, and the whole run matches the fused
    /// scalar path bit-for-bit.
    #[test]
    fn packed_buckets_six_emas() {
        let (m, r) = merge(&[
            Apo::recipe(&[6.0, 9.0]),
            Ppo::recipe(&[10.0, 26.0]),
            Macd::recipe(&[12.0, 24.0, 9.0]),
        ]);
        let leads = m.leads();
        let p = PackedPlan::lower(&m, &leads, &r).expect("six-lane wave must pack");
        assert_eq!(p.groups4.len(), 1, "Simd<4> bucket");
        assert_eq!(p.groups2.len(), 1, "Simd<2> bucket");
        assert!(p.groups8.is_empty());

        let close = long_series(2000);
        let s: [&[f64]; 1] = [close.as_slice()];
        let fused = FusedPlan::build(&m, &leads, &r);
        let mut qs = QueryState {
            ema: vec![None; m.nodes.len()],
            next_bar: 0,
        };
        let mut ps = p.new_state();
        let a = fused.advance(&mut qs, &s);
        let b = p.advance(&mut ps, &s);
        cmp_results(&a, &b, "six_ema");
    }

    /// An arena without any ≥2-lane wave yields no packed plan — the scalar
    /// fused path remains the engine for it.
    #[test]
    fn packed_lower_none_without_groups() {
        let mut recipe = Recipe::new();
        let input = recipe.add(Node::Input { stream: 0 });
        let e1 = recipe.add(Node::Ema {
            src: input,
            period: 12,
        });
        let e2 = recipe.add(Node::Ema { src: e1, period: 9 });
        recipe.outputs = vec![e2];
        let (m, r) = merge(&[recipe]);
        let leads = m.leads();
        assert!(
            PackedPlan::lower(&m, &leads, &r).is_none(),
            "chained EMAs init at different bars → no packable bucket"
        );
    }
}
