//! The CSE merge pass: intern several indicator [`Recipe`]s' **interface**
//! graphs into one arena, deduplicating identical nodes by canonical
//! structural key (§5.2). The tail wiring (inputs, sinks) is remapped
//! local→global alongside the nodes.
//!
//! # The one hazard
//! [`Node`] stores child references and plain params (period, `Const` bits,
//! input stream) under the same [`Id`] type — `remap` is the single place that
//! knows which is which. A new `Node` variant with a child must be added to
//! `remap` **and** [`Merged::leads`], or CSE keys on a stale local index.
//!
//! # Why interning is order-independent
//! Every recipe is SSA-ordered, so when node `i` is interned all of `i`'s
//! children are already global. Structurally-identical subtrees from any
//! recipe rewrite to the same global node and hit the same cache entry — that
//! is what makes `Apo`'s `ema12/ema26` and `Macd`'s literally the same arena
//! nodes, the §4 identity falling out of CSE for free.

use crate::cse::{AlphaShape, Column, Id, Node, Recipe, Sink, SinkSrc};
use std::collections::HashMap;

/// Merged, node-deduplicated interface graph. Child `Id`s in `nodes` are
/// *global* arena indices.
#[derive(Debug, Clone)]
pub struct Merged {
    pub nodes: Vec<Node>,
    /// Display-only role tags parallel to `nodes` (from `Recipe::tags`,
    /// first writer across recipes wins). NEVER read by the planner's value
    /// paths — the `kernel!` emitter uses them to name locals.
    pub tags: Vec<Option<&'static str>>,
}

impl Merged {
    /// Canonical per-node `defined-from` lead: the earliest absolute bar at
    /// which the node has a valid value (§5.3). `Ema = lead(src) + period-1`,
    /// arithmetic = `max(children)`, `Input`/`Const` = 0, filter/HT per the
    /// catalog rules (not executed in v1, but the lead math is defined).
    pub fn leads(&self) -> Vec<Id> {
        let mut leads: Vec<Id> = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let id = match *node {
                Node::Input { .. } | Node::Const { .. } => 0,
                Node::Ema { src, period } => leads[src as usize] + period.saturating_sub(1),
                // Recurrence starts at the SEED's lead or explicit feed override.
                // For zlema, `feed` specifies the seed bar (lag-1), and output
                // starts at bar `base + 1 = lag`. For trvi, `feed` is leads[seed]
                // (0 for range), but the recipe sets sink lead to override.
                Node::SeededEma {
                    seed, period, feed, ..
                } => {
                    let base = feed.unwrap_or(leads[seed as usize]);
                    // The output starts at bar `base + 1` (where we start folding
                    // the src node), not at bar `base` (where we seed). The recipe's
                    // sink lead override can push the effective start later if needed.
                    base + 1
                }
                // Same seed/feed shape as SeededEma -- the mult stream is a
                // separate per-bar read inside the fold body, not a lead input.
                Node::VariableAlpha { seed, feed, .. } => {
                    let base = feed.unwrap_or(leads[seed as usize]);
                    base + 1
                }
                Node::Sub { a, b }
                | Node::Add { a, b }
                | Node::Mul { a, b }
                | Node::Div { a, b }
                | Node::Max { a, b }
                | Node::Min { a, b } => leads[a as usize].max(leads[b as usize]),
                // A compare-select: defined once all three inputs are.
                Node::WhereGe { a, b, t } => leads[a as usize]
                    .max(leads[b as usize])
                    .max(leads[t as usize]),
                Node::WhereLt { a, b, t } => leads[a as usize]
                    .max(leads[b as usize])
                    .max(leads[t as usize]),
                Node::WhereGt { a, b, t } => leads[a as usize]
                    .max(leads[b as usize])
                    .max(leads[t as usize]),
                Node::WhereLe { a, b, t } => leads[a as usize]
                    .max(leads[b as usize])
                    .max(leads[t as usize]),
                // Ring-backed age roll: capacity n, fed every bar from the
                // src lead onward. MirrorBuffer Cold push_with_info only
                // returns Some once count == capacity BEFORE the push, i.e.
                // the (n+1)-th push, at bar lead(src) + n (not + n - 1: the
                // n-th push just reaches full, nothing has evicted yet).
                Node::RollAge { src, n } => leads[src as usize] + n,
                // A running total is defined once both operands are — the same
                // rule as the product it fuses into one rounding.
                Node::Accum { a, b } => leads[a as usize].max(leads[b as usize]),
                Node::Abs { src } => leads[src as usize],
                // A delayed read of a stream first has a value once the
                // stream is `n` bars old.
                Node::Lag { src, n } => leads[src as usize] + n,
                // Supertrend's first bar is the atr seam (the single seed
                // `calc_st`), so its defined-from = the atr's lead (max of its
                // three node inputs; the atr chain dominates).
                Node::SuperTrend {
                    atr, med, close, ..
                } => leads[atr as usize]
                    .max(leads[med as usize])
                    .max(leads[close as usize]),
                Node::Sma { src, period } | Node::Wma { src, period } => {
                    leads[src as usize] + period.saturating_sub(1)
                }
                // `feed`, when set, is an explicit absolute-bar early-publish
                // override (same convention as `SeededEma`'s `feed`) -- `None`
                // reproduces the plain `Sma`/`Wma` formula exactly.
                Node::Stddev {
                    src, period, feed, ..
                } => feed.unwrap_or(leads[src as usize] + period.saturating_sub(1)),
                // E12: same boundary — the window is `period` wide, so the first
                // full one ends at `lead(src) + period − 1`; E13's `skip` pushes
                // it out further by the values the hand discards before folding.
                Node::RollSum {
                    src, period, skip, ..
                } => leads[src as usize] + skip + period.saturating_sub(1),
                // E9 trail-rescan roll: the carried extremum is meaningful
                // from bar 0, but the WINDOW the hand's rescan walks needs
                // `period` bars to exist, so the warm boundary is the same
                // `period − 1` a rolling sum uses. (`look_back` does not move
                // the boundary: `aroon`'s wider window still first prints at
                // bar `period` via its explicit sink lead.)
                Node::RollMax { src, period, .. } | Node::RollMin { src, period, .. } => {
                    leads[src as usize] + period.saturating_sub(1)
                }
                // E9: a trail projection exists exactly when its roll does.
                Node::Trail { src } => leads[src as usize],
                // E3 stage 3: the hand fills `√P` (truncated) diff slots
                // AFTER the two-engine warm, so the first value exists at
                // bar `P + psqrt − 1` (publish lead = that + 1, hand
                // `init_state`'s returned start).
                Node::Hma { src, period } => {
                    let psqrt = (period as f64).sqrt() as Id;
                    leads[src as usize] + (period + psqrt).saturating_sub(1)
                }
                // Wilder's smoothing: the fold is `period` long; MEAN mode
                // divides at the last fold bar (hand `init_state` seed),
                // COMPOSITE mode keeps the raw sum (`partial_calc`). The fold
                // START bar is bar 0 unless `feed` (then it starts at the
                // source's publish lead, like a chained `Ema` — `adx` over
                // `dx`). So the seam = `feed ? lead(src) : 0 + period-1`.
                Node::Wilders {
                    src, period, feed, ..
                } => {
                    let f = if feed { leads[src as usize] } else { 0 };
                    f + period.saturating_sub(1)
                }
                Node::Sqrt { src } | Node::Ln { src } | Node::Mirror { src } => leads[src as usize],
                // E15: recurrence defined once its source is (seam lead(src));
                // IirClamped's Const children are lead 0; Prev shares its chain.
                Node::Iir { src, .. } | Node::IirClamped { src, .. } | Node::Prev { src } => {
                    leads[src as usize]
                }
                Node::Fma { a, b, c } => leads[a as usize]
                    .max(leads[b as usize])
                    .max(leads[c as usize]),
                // HighPass's own `State::init_state` warms for exactly
                // `period` bars (discarded) then publishes — unlike `Ema` it
                // does NOT reuse its last warmup bar, so the standalone lead
                // is `period`, not `period - 1`. When chained (RoofingFilter)
                // the hand runs HighPass and SuperSmoother in LOCKSTEP from
                // bar 0 and warms for `max(hp_period, ss_period)` bars total
                // (not the sum) before publishing — so composition is `max`,
                // not a chained sum.
                Node::HighPass { input, period } => leads[input as usize].max(period),
                Node::SuperSmoother { src, period } => leads[src as usize].max(period),
                // The hand's `HilbertTransform::State::init_state` fills the
                // 7-slot ring with 7 pushes (bars `lead(roofing)..lead(roofing)+6`)
                // and publishes starting ONE bar after the ring becomes full,
                // i.e. `lead(roofing) + 7` — matching `RollAge`'s "(n+1)-th
                // push" convention, not the FIR's own tap count minus one.
                Node::HtInPhase { src } | Node::HtQuadrature { src } => leads[src as usize] + 7,
                // Gated multiplicative prefix product: defined once every
                // operand is (same rule as Accum's two-node recurrence).
                Node::NviRecur {
                    src,
                    prev_close,
                    volume,
                    prev_volume,
                }
                | Node::PviRecur {
                    src,
                    prev_close,
                    volume,
                    prev_volume,
                } => leads[src as usize]
                    .max(leads[prev_close as usize])
                    .max(leads[volume as usize])
                    .max(leads[prev_volume as usize]),
                // The shared `sma` operand (a RollSum{mean:true}) is itself
                // valid from `leads[src] + period - 1`; the hand's own
                // rescan loop starts ONE bar later than that (`i = period`,
                // not `period - 1`), so MeanAbsDev's lead is `leads[sma] + 1`
                // -- do NOT `.max()` with `leads[src]`, that under-counts by
                // one bar (leads[sma] always >= leads[src], so the max just
                // resolves to leads[sma], one bar too early).
                Node::MeanAbsDev { sma, .. } => leads[sma as usize] + 1,
                // linreg's moment carry. `src` is a price column, so
                // `leads[src]` is 0 and the first valid bar is `period` — the
                // hand's own first published bar (it works on `real[1..]` and
                // seeds from `real[1..period]`). The two instances agree:
                // `sum_y`'s `sub` is a `Lag` one bar *inside* the window
                // (lead `period - 1`), `sum_xy`'s is the `sum_y` node itself
                // (lead `period`), so the `max` never moves the answer.
                Node::LinregFlow {
                    src, sub, period, ..
                } => (leads[src as usize] + period).max(leads[sub as usize]),
                // Bespoke 4-state recurrence: hand's `init_state` loop
                // starts at i=1, never calling `calc_vf` at i=0 (needs a
                // previous bar to compare against) -- first valid vf bar is
                // one past the inputs' own lead (same `+1` as erase()'s feed).
                Node::KvoFlow {
                    high,
                    low,
                    close,
                    volume,
                } => {
                    leads[high as usize]
                        .max(leads[low as usize])
                        .max(leads[close as usize])
                        .max(leads[volume as usize])
                        + 1
                }
            };
            leads.push(id);
        }
        leads
    }

    /// Partition request indices into **sharing groups**: two requests join a
    /// group iff their sub-graphs reach a common *computed* node — a chain,
    /// arith, `Abs`/`Lag`, anything that is not a raw `Input` price stream or a
    /// `Const`. Inputs are shared by every indicator, so counting them would
    /// collapse all requests into one group and defeat the purpose; only
    /// genuine shared *computation* links. Roots are the same liveness roots the
    /// executor uses (enabled sinks ∪ `tail_inputs`), so a disabled optional
    /// pulls in no sharing.
    ///
    /// This is the primitive behind "don't fuse disjoint indicators into one
    /// loop" (measured: `natr`+`adx` and the disjoint `assets` lanes *lose* —
    /// zero dedup, monolithic-plan cost). Groups map to independent sub-plans:
    /// fuse *within* a cluster (shared chains computed once), run *across*
    /// clusters as separate loops (bounded register footprint; parallelizable
    /// on bulk `run`, sequential on `tick`). Returns `Vec<Vec<request_idx>>` in
    /// first-appearance order.
    pub fn sharing_groups(&self, resolved: &[Resolved]) -> Vec<Vec<usize>> {
        let n = resolved.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut Vec<usize>, x: usize) -> usize {
            if p[x] == x {
                return x;
            }
            let r = find(p, p[x]);
            p[x] = r;
            r
        }
        // arena node id -> first request whose fanin includes it (computed nodes
        // only); a second distinct request that reaches it unions the two.
        let mut owner: HashMap<Id, usize> = HashMap::new();
        let nnodes = self.nodes.len();
        let mut mark = vec![usize::MAX; nnodes]; // per-request visited stamp (reused)
        for (r, res) in resolved.iter().enumerate() {
            let mut stack: Vec<Id> = Vec::new();
            let mut roots = |sinks: &[Sink]| {
                for s in sinks {
                    let SinkSrc::Interface(id) = s.src;
                    stack.push(id);
                }
            };
            roots(&res.outputs);
            for (oi, s) in res.optional_outputs.iter().enumerate() {
                if res.optional_enabled.get(oi).copied().unwrap_or(true) {
                    stack.push(match s.src {
                        SinkSrc::Interface(id) => id,
                    });
                }
            }
            stack.extend(res.inputs.iter().copied());
            while let Some(id) = stack.pop() {
                if mark[id as usize] == r {
                    continue;
                }
                mark[id as usize] = r;
                let node = &self.nodes[id as usize];
                if !matches!(node, Node::Input { .. } | Node::Const { .. }) {
                    match owner.get(&id) {
                        Some(&o) => {
                            let (ro, rr) = (find(&mut parent, o), find(&mut parent, r));
                            if ro != rr {
                                parent[rr] = ro;
                            }
                        }
                        None => {
                            owner.insert(id, r);
                        }
                    }
                }
                node_children(node, &mut |c| stack.push(c));
            }
        }
        let mut order: Vec<usize> = Vec::new();
        let mut buckets: HashMap<usize, Vec<usize>> = HashMap::new();
        for r in 0..n {
            let root = find(&mut parent, r);
            let e = buckets.entry(root).or_default();
            if e.is_empty() {
                order.push(root);
            }
            e.push(r);
        }
        order
            .into_iter()
            .map(|root| buckets[&root].clone())
            .collect()
    }

    /// # Lane regions — the STRUCTURAL packing question
    ///
    /// Packing is decided here, on the merged graph, by ONE question: **which
    /// nodes do the same arithmetic?** No indicator identity is consulted, and
    /// there is no per-kind rule to maintain — an `Ema` and a `Wilders` are not
    /// two packing rules, they are node shapes that answer that question the
    /// same way for their own reasons.
    ///
    /// Matching shapes is necessary but not sufficient, and the difference
    /// decides whether packing pays:
    ///
    /// · Same arithmetic AND same inputs ⇒ `intern` already merged them into
    ///   ONE node. There is nothing to pack — the work is already done once.
    /// · Same arithmetic, DIFFERENT inputs, ONE operation deep (`a − b` and
    ///   nothing else shared) ⇒ packing **loses**. The lanes must be assembled
    ///   with `insertlane` and split again with `extractlane` every bar to buy
    ///   one vector op; two scalar ops were cheaper.
    /// · Same arithmetic, DIFFERENT inputs, and the shape sits in a TALLER run
    ///   of aligned shapes ⇒ packing **wins, even though the inputs come from
    ///   different sources**, because the value crosses the lane boundary once
    ///   for the whole run instead of once per level.
    ///
    /// So the run counts INSTRUCTIONS, not levels — [`LaneRegion::shared_ops`].
    /// A shape with NO children (`Input`/`Const`) is a BOUNDARY and never
    /// a region: two input streams are two lanes of *reading*, not two lanes of
    /// work.
    /// That is why `Apo(12, 26)` — one arithmetic LEVEL, two different periods —
    /// is worth packing while a lone shared subtraction is not: the level itself is
    /// a GROUP of two instructions (`x · mult`, then `fma(prev, inv, ·)`), so the
    /// run already holds ≥ 2 shared instructions. Counting LEVELS would call that
    /// unpaklable, which is why the run is measured in instructions.
    ///
    /// So the criterion is ONE count, not two conditions:
    ///
    /// ```text
    /// shared_ops(shape) = own_ops(shape) + max{ shared_ops(child) : child is itself shared }
    /// packable          = shared_ops >= 2
    /// ```
    ///
    /// A child whose shape is UNIQUE stops the run: it is either a deduped shared
    /// node (better than packing — the work is already done once) or a boundary
    /// (`Input`/`Const`), and a region must COMPUTE something, which is exactly
    /// `own_ops > 0`.
    ///
    /// Magnitudes (every `period`, `Lag n`, the input `stream`, `Const` bits) are
    /// deliberately EXCLUDED from the shape: differing magnitudes are legal per
    /// lane, since the emitter builds one constant vector holding both. What the
    /// formula key DOES carry is the instruction sequence — see [`lane_formula`].
    ///
    /// A candidate report only: the emitter consumes nothing from this yet, and
    /// [`LaneRegion::uniform_warmup`] reports the legality an emission must still
    /// check for itself (one segment structure has to serve every lane) without
    /// pretending the shape analysis decided it.
    pub fn lane_regions(&self) -> Vec<LaneRegion> {
        let leads = self.leads();
        // Shape ids are discovered bottom-up (arena order ⇒ children first), so a
        // parent's id always exceeds its children's and the run pass can walk ids
        // in increasing order.
        let mut ids: HashMap<(u8, Vec<u32>), u32> = HashMap::new();
        let mut sig = Vec::with_capacity(self.nodes.len());
        let mut kids: Vec<Vec<u32>> = Vec::with_capacity(self.nodes.len());
        let mut formula_of: Vec<u8> = Vec::with_capacity(self.nodes.len());
        let mut ops_of: Vec<u8> = Vec::with_capacity(self.nodes.len());
        let mut warm_of: Vec<u8> = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let (formula, ops) = lane_formula(node);
            let warm = lane_warmup(node);
            let mut ch: Vec<u32> = Vec::new();
            node_children(node, &mut |c| ch.push(sig[c as usize]));
            // Sorted ⇒ operand ORDER is not part of the shape. `Sub(real, lag)`
            // and `Sub(lag, real)` are the same arithmetic over swapped inputs,
            // and the swap costs nothing to realize: it is absorbed into which
            // value goes into lane 0 of each operand vector.
            ch.sort_unstable();
            let id = match ids.get(&(formula, ch.clone())) {
                Some(&v) => v,
                None => {
                    let v = ids.len() as u32;
                    ids.insert((formula, ch.clone()), v);
                    v
                }
            };
            sig.push(id);
            kids.push(ch);
            // Recorded per NODE (all lanes of a shape agree by construction — the
            // key is derived from these values).
            formula_of.push(formula);
            ops_of.push(ops);
            warm_of.push(warm);
        }

        // Lane lists per shape, in arena order (deterministic: a Vec, and the
        // scan above is ordered).
        let mut lanes: HashMap<u32, Vec<Id>> = HashMap::new();
        for (i, &s) in sig.iter().enumerate() {
            lanes.entry(s).or_default().push(i as Id);
        }
        // A region must do arithmetic: `own_ops > 0` excludes both the boundary
        // leaves and the zero-cost copies/projections (`Mirror`, `Lag`).
        let shared: Vec<u32> = {
            let mut v: Vec<u32> = ids
                .values()
                .copied()
                .filter(|&s| {
                    let l = &lanes[&s];
                    l.len() >= 2 && ops_of[l[0] as usize] > 0
                })
                .collect();
            v.sort_unstable();
            v
        };

        // Instructions per bar across the whole aligned run.
        let mut run: HashMap<u32, usize> = HashMap::new();
        for &s in &shared {
            let first = lanes[&s][0] as usize;
            let mut best = 0usize;
            for &c in &kids[first] {
                if shared.binary_search(&c).is_ok() {
                    best = best.max(run.get(&c).copied().unwrap_or(0));
                }
            }
            run.insert(s, best + ops_of[first] as usize);
        }

        let mut out: Vec<LaneRegion> = shared
            .iter()
            .filter(|&&s| run[&s] >= 2)
            .map(|&s| {
                let l = &lanes[&s];
                let first = l[0] as usize;
                // LEGALITY, reported not enforced: every lane must be servable by
                // ONE segment structure — same warm-up policy, and (where that
                // policy has a fold phase) the same defining bar.
                let lead = leads[first];
                let warm = warm_of[first];
                let uniform_warmup = l.iter().all(|&i| {
                    // `warm == 0` ⇒ no fold phase ⇒ lanes may publish on different
                    // bars and still run one loop.
                    warm_of[i as usize] == warm && (warm == 0 || leads[i as usize] == lead)
                });
                LaneRegion {
                    shape: s,
                    formula: formula_of[first],
                    ops: ops_of[first],
                    lanes: l.clone(),
                    shared_ops: run[&s],
                    uniform_warmup,
                }
            })
            .collect();
        out.sort_by_key(|r| (r.shape, r.lanes[0]));
        out
    }
}

/// A set of arena nodes doing the SAME arithmetic over DIFFERENT sources, shared
/// across at least two per-bar instructions — the unit the lane packer would
/// claim. Produced by [`Merged::lane_regions`], which returns only the packable
/// ones, so every value here is a candidate and nothing is a rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneRegion {
    /// Stable id of the shape within this graph (NOT an arena id).
    pub shape: u32,
    /// Formula id from [`lane_formula`] — the instruction sequence, NOT a kind.
    /// Two different `Node` variants with the same id are the same math (that is
    /// the point: `Ema` and a mean `Wilders` share one).
    pub formula: u8,
    /// Instructions this shape performs per bar on its own.
    pub ops: u8,
    /// The nodes sharing the arithmetic, in arena order ⇒ lane 0, lane 1, …
    /// A run longer than the backend's lane ceiling splits into pairs in this
    /// order, deterministically.
    pub lanes: Vec<Id>,
    /// Instructions per bar across the whole aligned run — the count that pays
    /// for one pack/unpack at each end. Always ≥ 2 in a returned region.
    pub shared_ops: usize,
    /// Every lane can be served by ONE segment structure: same warm-up policy, and
    /// same defining bar where that policy has a fold phase (see [`lane_warmup`]).
    /// A region can be worth packing and still not be legal AS IS — an `Ema` lane
    /// beside a `Wilders` lane computes the same steady `fma` but warms up
    /// differently. This is the first gate the emission has to clear.
    pub uniform_warmup: bool,
}

/// The node's ARITHMETIC shape: `(formula id, steady-state instructions per bar)`.
///
/// **The planner and optimizer see the math, never the indicator.** A `Node`
/// variant is shorthand for a formula the recipe expanded into the graph, so the
/// shape key must be the expansion, not the spelling — otherwise this pass is
/// just the old per-kind rule with a new hat. The motivating case:
///
/// ```text
/// ema::calc      prev_ema.mul_add(inv_multiplier,  value * multiplier)
/// wilders::calc  wilders  .mul_add(     multiplier, value * inv_multiplier)
/// ```
///
/// Both ARE `fma(state, c0, mul(x, c1))`: one `fmul`, one `fma`, in the same
/// order, differing only in WHICH NUMBER lands in which slot. Magnitudes are not
/// part of a shape (the emitter builds one constant vector holding both lanes),
/// so `Ema` and a mean `Wilders` are the SAME formula and pack as two lanes of
/// one instruction sequence — invisible to any key that names the kind.
///
/// The instruction count is what makes the rule one rule instead of two: a chain
/// is itself a GROUP of nodes, so a pair of chains already holds ≥ 2 shared
/// instructions and earns an assembly, while a pair of lone `Sub`s holds one and
/// does not.
///
/// Splitting rules — a formula differs when the INSTRUCTION SEQUENCE differs, not
/// when a number does:
/// · `Wilders::composite` folds `fma(state, c0, x)` with no multiply → 1 instr.
/// · `RollSum{mean:true}` and `Sma` both roll the sum then scale it → SAME
///   formula, two spellings, exactly as Ema/Wilders.
/// · `RollMax`/`RollMin` need different per-lane predicates, so they cannot share
///   one vector compare → separate formulas.
/// · `Fma` is one fused instruction, so it is NOT the Ema pair (which is two).
/// · `Lag`/`Mirror` are a load and a copy: no shared arithmetic at all, count 0.
/// · `Wilders::feed` decides WHEN folding starts. That is warm-up policy, not
///   steady math, so it does not split the shape — it is folded into
///   [`LaneRegion::uniform_warmup`] for the legality check instead.
///
/// Exhaustive with NO wildcard: a new `Node` variant fails to compile here, in
/// the same way `remap_node`/`node_children`/`leads` already do, so it cannot
/// silently become unpaklable. Counts for the bespoke multi-value ops (Hma,
/// SuperTrend, the filters) are steady-state instruction counts read off their
/// `codegen.rs` arms; they only ever need to clear the ≥ 2 threshold, so exactness
/// there is not load-bearing.
fn lane_formula(node: &Node) -> (u8, u8) {
    use Node::*;
    match *node {
        // Leaves: boundaries, not work. Same formula whatever value or stream
        // they name, so a run may permute between lanes over them.
        Input { .. } => (1, 0),
        Const { .. } => (2, 0),
        // `fma(state, c0, mul(x, c1))` — the Ema/Wilders identity, one formula.
        Ema { .. } => (3, 2),
        // Same fma identity as `Ema` -- seed bar aside, the fold step is
        // identical (`st.mul_add(invm, src*mult)`); the seed-vs-fold source
        // split lives in `node_children`/`erase`, not the lane shape.
        SeededEma { .. } => (3, 2),
        Wilders {
            composite: false, ..
        } => (3, 2),
        Wilders {
            composite: true, ..
        } => (4, 1),
        Sub { .. } => (5, 1),
        Add { .. } => (6, 1),
        Mul { .. } => (7, 1),
        Div { .. } => (8, 1),
        Abs { .. } => (9, 1),
        // An anchor read: a load, and not vectorizable (two lanes come from
        // different pointers), so it never earns an assembly on its own.
        Lag { .. } => (10, 0),
        Max { .. } => (11, 1),
        // minNum is its own instruction (`minpd`/`minsd`), so it is NOT the
        // `Max` formula id — one id means one instruction sequence, and
        // sharing them would let the packer treat max and min as one region.
        Min { .. } => (34, 1),
        WhereGe { .. } => (12, 2),
        WhereLt { .. } => (13, 2),
        // The two new selects are their OWN instructions-per-guard pair, but a
        // distinct shape id from 12/13: one id means one instruction sequence,
        // and `cmpgt`/`cmple` are not `cmpgt`-or-`cmpge` of the existing arms.
        WhereGt { .. } => (35, 2),
        WhereLe { .. } => (36, 2),
        // `sum += x − old` then an optional scale: the Sma/RollSum identity.
        RollSum { mean: false, .. } => (14, 2),
        RollSum { mean: true, .. } => (15, 3),
        Sma { .. } => (15, 3),
        RollMax { .. } => (16, 4),
        RollMin { .. } => (17, 4),
        // Projections/copies: one conversion, or none.
        Trail { .. } => (18, 1),
        Mirror { .. } => (19, 0),
        // Ring-backed age roll: no arithmetic, just read from ring.
        RollAge { .. } => (40, 0),
        Ln { .. } => (20, 1),
        Sqrt { .. } => (21, 1),
        // One fused instruction — deliberately NOT formula 3.
        Fma { .. } => (22, 1),
        Hma { .. } => (23, 8),
        Stddev { .. } => (24, 6),
        Wma { .. } => (25, 5),
        HighPass { .. } => (26, 4),
        SuperSmoother { .. } => (27, 5),
        HtInPhase { .. } => (28, 4),
        HtQuadrature { .. } => (29, 4),
        SuperTrend { .. } => (30, 6),
        // E15: Iir = Ema/mean-Wilders 2-op shape (id 3); IirClamped = +clamp (5);
        // Prev = slot read (0).
        Iir { .. } => (3, 2),
        IirClamped { .. } => (33, 5),
        Prev { .. } => (32, 0),
        // A running total is ONE fused instruction (`a.mul_add(b, st)`) — it is
        // not formula 3, whose second operand is a scaled `src`.
        Accum { .. } => (37, 1),
        // Scalar-only gated multiplicative prefix product: the per-bar
        // branch on volume-vs-prev-volume cannot be lane-packed, so there
        // is no SIMD form -- same as the hand.
        NviRecur { .. } => (0, 0),
        PviRecur { .. } => (0, 0),
        // Window rescan over the shared sma operand: one state value, no
        // arithmetic shared with RollSum's own formula (its `sum` is NOT
        // what this walks -- it rescans |x - sma| fresh every bar).
        MeanAbsDev { .. } => (38, 0),
        // linreg's moment carry: scalar-only. Its `sub` is a slot read (the
        // `sum_y` node, or a per-lane `Lag` load) and the recurrence owns a
        // `st` cell, so it never shares an assembly with a neighbour — the
        // same standing as `NviRecur`/`KvoFlow`.
        LinregFlow { .. } => (0, 0),
        // Bespoke 4-state recurrence, branchy like Nvi/Pvi -- scalar-only.
        KvoFlow { .. } => (0, 0),
        // Per-bar stream-read blend weight: scalar-only, same standing as
        // NviRecur/LinregFlow/KvoFlow.
        VariableAlpha { .. } => (0, 0),
    }
}

/// Warm-up POLICY bits — whether a node's per-bar ARITHMETIC changes before it is
/// warm. Not part of the shape (it is not steady-state math); it feeds
/// [`LaneRegion::uniform_warmup`], the legality gate an emission must clear.
///
/// The distinction that matters is between a node that merely PUBLISHES later and
/// one that COMPUTES differently while warming up:
/// · a price-sourced `Ema` steps the same `fma` from its seed on every bar, so two
///   of different periods have different `lead`s but IDENTICAL per-bar arithmetic
///   — and since store gating is per SINK, they still share one loop. That is
///   `Apo(12, 26)`, which the emitter packs today and measured a win: requiring
///   equal leads there would be simply wrong.
/// · `Wilders`, `Sma`/`Stddev`/`Wma`/`RollSum` FOLD a running sum until their seam
///   and then switch step, and the trail rolls guard a rescan that is unreachable
///   before bar `period`. Those need the segment cut at a different bar, so lanes
///   must also agree on `lead` (checked by the caller, which knows the bit is the
///   "has a fold phase" kind).
/// · `Wilders::feed` additionally ignores its source until the source publishes,
///   which is a different predicate in the body, so it splits the policy.
fn lane_warmup(node: &Node) -> u8 {
    use Node::*;
    match *node {
        Ema { .. } => 0,
        SeededEma { .. } => 0,
        Wilders { feed, .. } => 1 | ((feed as u8) << 1),
        Sma { .. } | Stddev { .. } | Wma { .. } | RollSum { .. } => 2,
        RollMax { .. } | RollMin { .. } => 3,
        _ => 0,
    }
}

/// Where one input recipe landed in the merged arena: its state wiring and
/// sinks with all ids resolved to arena globals.
#[derive(Debug, Clone)]
pub struct Resolved {
    /// Debug/label tag (from `Recipe::kind`) — the executor does not read it.
    pub kind: &'static str,
    /// Options (labels; the graph carries what matters).
    pub options: Vec<f64>,
    /// Global arena ids of the `Ema` chains the hand `State` holds, in
    /// `TState` field order — runtime uses table lookup keyed by kind.
    pub inputs: Vec<Id>,
    /// Phase 1: restorer removed from leaf's Recipe; runtime uses table.
    pub outputs: Vec<Sink>,
    /// Optional sinks, `Info::optional_outputs` order.
    pub optional_outputs: Vec<Sink>,
    /// Per-optional enable flags (parallel to `optional_outputs`); `merge`
    /// defaults all-`true`, [`crate::cse::Request::add`] overrides per request.
    /// A disabled optional sink prints an EMPTY array in declared position
    /// (hand `::indicator(None)` parity) and is excluded from emission.
    pub optional_enabled: Vec<bool>,
}

/// Intern `node` (child ids already global) into `arena`, returning its global
/// id. Equal structure ⇒ same id ⇒ shared computation (the CSE key). The
/// display tag is NOT part of the key — two equal nodes with different tags
/// still share ONE arena slot (first writer keeps the name).
fn intern(
    arena: &mut Vec<Node>,
    tags: &mut Vec<Option<&'static str>>,
    cache: &mut HashMap<Node, Id>,
    node: Node,
) -> Id {
    if let Some(&existing) = cache.get(&node) {
        return existing;
    }
    let id = arena.len() as Id;
    arena.push(node);
    tags.push(None);
    cache.insert(node, id);
    id
}

/// Rewrite a recipe-local node into its globally-shaped form, mapping every
/// child ref through `g` (slice wrapper for the merge pass).
fn remap(node: &Node, l2g: &[Id]) -> Node {
    remap_node(node, &|id| l2g[id as usize])
}

/// The one child-id-remapping truth, shared by merge (local→global) and
/// [`Recipe::sub`](crate::cse::Recipe::sub) (sub-local→parent-local graft).
/// **Refs (→ `g`):** `Ema{src}`, `Sub/Add/Mul/Div/Max{a,b}`, `WhereGe{a,b,t}`,
/// `WhereLt{a,b,t}`, `Abs/Lag/Sqrt{src}`, `Sma/Stddev/Wma/Hma{src,period}`, `Wilders{..}`,
/// `SuperTrend{atr,med,close}`, `Fma{a,b,c}`, `HighPass/SuperSmoother`,
/// `Ht*{src}`. **Params (copy):** `Input{stream}`, `Const{bits}`, every
/// `period`, `Lag{n}`, `SuperTrend{step_bits}`.
pub(crate) fn remap_node(node: &Node, g: &dyn Fn(Id) -> Id) -> Node {
    use Node::*;
    match *node {
        Input { stream } => Input { stream },
        Const { bits } => Const { bits },
        Ema { src, period } => Ema {
            src: g(src),
            period,
        },
        SeededEma {
            seed,
            src,
            period,
            feed,
        } => SeededEma {
            seed: g(seed),
            src: g(src),
            period,
            feed,
        },
        VariableAlpha {
            seed,
            src,
            mult,
            feed,
            shape,
        } => VariableAlpha {
            seed: g(seed),
            src: g(src),
            mult: g(mult),
            feed,
            shape,
        },
        Sub { a, b } => Sub { a: g(a), b: g(b) },
        Add { a, b } => Add { a: g(a), b: g(b) },
        Mul { a, b } => Mul { a: g(a), b: g(b) },
        Div { a, b } => Div { a: g(a), b: g(b) },
        WhereGe { a, b, t } => WhereGe {
            a: g(a),
            b: g(b),
            t: g(t),
        },
        WhereLt { a, b, t } => WhereLt {
            a: g(a),
            b: g(b),
            t: g(t),
        },
        WhereGt { a, b, t } => WhereGt {
            a: g(a),
            b: g(b),
            t: g(t),
        },
        WhereLe { a, b, t } => WhereLe {
            a: g(a),
            b: g(b),
            t: g(t),
        },
        Accum { a, b } => Accum { a: g(a), b: g(b) },
        Abs { src } => Abs { src: g(src) },
        Lag { src, n } => Lag { src: g(src), n },
        Max { a, b } => Max { a: g(a), b: g(b) },
        Min { a, b } => Min { a: g(a), b: g(b) },
        Hma { src, period } => Hma {
            src: g(src),
            period,
        },
        Wilders {
            src,
            period,
            composite,
            feed,
        } => Wilders {
            src: g(src),
            period,
            composite,
            feed,
        },
        SuperTrend {
            atr,
            med,
            close,
            period,
            step_bits,
        } => SuperTrend {
            atr: g(atr),
            med: g(med),
            close: g(close),
            period,
            step_bits,
        },
        Wma { src, period } => Wma {
            src: g(src),
            period,
        },
        Sma { src, period } => Sma {
            src: g(src),
            period,
        },
        RollSum {
            src,
            period,
            mean,
            skip,
            steady_from,
        } => RollSum {
            src: g(src),
            period,
            mean,
            skip,
            steady_from,
        },
        Stddev {
            src,
            period,
            feed,
            switch,
        } => Stddev {
            src: g(src),
            period,
            feed,
            switch,
        },
        RollMax {
            src,
            period,
            look_back,
        } => RollMax {
            src: g(src),
            period,
            look_back,
        },
        RollMin {
            src,
            period,
            look_back,
        } => RollMin {
            src: g(src),
            period,
            look_back,
        },
        Trail { src } => Trail { src: g(src) },
        // Ring-backed age roll: remap src through the graph; n is a plain
        // capacity VALUE (not an arena id -- node_children doesn't visit it
        // either), so it must NOT be run through the index-remapping g().
        RollAge { src, n } => RollAge { src: g(src), n },
        Mirror { src } => Mirror { src: g(src) },
        Sqrt { src } => Sqrt { src: g(src) },
        Ln { src } => Ln { src: g(src) },
        Iir {
            src,
            a_bits,
            b_bits,
        } => Iir {
            src: g(src),
            a_bits,
            b_bits,
        },
        IirClamped {
            src,
            a_bits,
            b_bits,
            hi_t,
            hi_v,
            lo_t,
            lo_v,
        } => IirClamped {
            src: g(src),
            a_bits,
            b_bits,
            hi_t: g(hi_t),
            hi_v: g(hi_v),
            lo_t: g(lo_t),
            lo_v: g(lo_v),
        },
        Prev { src } => Prev { src: g(src) },
        Fma { a, b, c } => Fma {
            a: g(a),
            b: g(b),
            c: g(c),
        },
        HighPass { input, period } => HighPass {
            input: g(input),
            period,
        },
        SuperSmoother { src, period } => SuperSmoother {
            src: g(src),
            period,
        },
        HtInPhase { src } => HtInPhase { src: g(src) },
        HtQuadrature { src } => HtQuadrature { src: g(src) },
        NviRecur {
            src,
            prev_close,
            volume,
            prev_volume,
        } => NviRecur {
            src: g(src),
            prev_close: g(prev_close),
            volume: g(volume),
            prev_volume: g(prev_volume),
        },
        PviRecur {
            src,
            prev_close,
            volume,
            prev_volume,
        } => PviRecur {
            src: g(src),
            prev_close: g(prev_close),
            volume: g(volume),
            prev_volume: g(prev_volume),
        },
        MeanAbsDev { src, period, sma } => MeanAbsDev {
            src: g(src),
            period,
            sma: g(sma),
        },
        LinregFlow {
            src,
            sub,
            period,
            weighted,
        } => LinregFlow {
            src: g(src),
            sub: g(sub),
            period,
            weighted,
        },
        KvoFlow {
            high,
            low,
            close,
            volume,
        } => KvoFlow {
            high: g(high),
            low: g(low),
            close: g(close),
            volume: g(volume),
        },
    }
}

/// Visit every **child ref** of `node` (params — `period`, `n`, `stream`,
/// `bits`, `step_bits` — are not graph edges and are skipped). SSA order
/// guarantees each child id `< node` id, so the executor's dead-node pass
/// propagates liveness in one descending scan. A new `Node` variant with a
/// child must be added here **and** `remap_node` **and** `Merged::leads`.
pub fn node_children(node: &Node, visit: &mut dyn FnMut(Id)) {
    use Node::*;
    match node {
        Input { .. } | Const { .. } => {}
        Ema { src, .. }
        | Abs { src }
        | Sqrt { src }
        | Ln { src }
        | Lag { src, .. }
        | Hma { src, .. }
        | Wma { src, .. }
        | Sma { src, .. }
        | RollSum { src, .. }
        | Stddev { src, .. }
        | RollMax { src, .. }
        | RollMin { src, .. }
        | Trail { src }
        | Mirror { src }
        | RollAge { src, .. }
        | Wilders { src, .. }
        | SuperSmoother { src, .. }
        | Iir { src, .. }
        | Prev { src }
        | HtInPhase { src }
        | HtQuadrature { src } => visit(*src),
        SeededEma { seed, src, .. } => {
            visit(*seed);
            visit(*src);
        }
        Sub { a, b } | Add { a, b } | Mul { a, b } | Div { a, b } | Max { a, b } | Min { a, b } => {
            visit(*a);
            visit(*b);
        }
        // A running total reads a node AND a column (volume), both graph edges.
        Accum { a, b } => {
            visit(*a);
            visit(*b);
        }
        WhereGe { a, b, t } => {
            visit(*a);
            visit(*b);
            visit(*t);
        }
        WhereLt { a, b, t } => {
            visit(*a);
            visit(*b);
            visit(*t);
        }
        WhereGt { a, b, t } => {
            visit(*a);
            visit(*b);
            visit(*t);
        }
        WhereLe { a, b, t } => {
            visit(*a);
            visit(*b);
            visit(*t);
        }
        Fma { a, b, c } => {
            visit(*a);
            visit(*b);
            visit(*c);
        }
        // src plus the four threshold/value Const children (all graph edges).
        IirClamped {
            src,
            hi_t,
            hi_v,
            lo_t,
            lo_v,
            ..
        } => {
            visit(*src);
            visit(*hi_t);
            visit(*hi_v);
            visit(*lo_t);
            visit(*lo_v);
        }
        HighPass { input, .. } => visit(*input),
        SuperTrend {
            atr, med, close, ..
        } => {
            visit(*atr);
            visit(*med);
            visit(*close);
        }
        NviRecur {
            src,
            prev_close,
            volume,
            prev_volume,
        }
        | PviRecur {
            src,
            prev_close,
            volume,
            prev_volume,
        } => {
            visit(*src);
            visit(*prev_close);
            visit(*volume);
            visit(*prev_volume);
        }
        MeanAbsDev { src, sma, .. } => {
            visit(*src);
            visit(*sma);
        }
        LinregFlow { src, sub, .. } => {
            visit(*src);
            visit(*sub);
        }
        KvoFlow {
            high,
            low,
            close,
            volume,
        } => {
            visit(*high);
            visit(*low);
            visit(*close);
            visit(*volume);
        }
        VariableAlpha {
            seed,
            src,
            mult,
            shape: _,
            ..
        } => {
            visit(*seed);
            visit(*src);
            visit(*mult);
        }
    }
}

fn remap_sink(sink: &Sink, l2g: &[Id]) -> Sink {
    Sink {
        src: match sink.src {
            SinkSrc::Interface(local) => SinkSrc::Interface(l2g[local as usize]),
        },
        lead: sink.lead,
    }
}

/// Merge `recipes` into one interface arena with shared nodes deduplicated.
/// Returns the arena and, per input recipe (same order), its resolved wiring.
///
/// # Input interning: by column, not by position
/// A recipe that declares [`Recipe::columns`] has its `Node::Input` slots
/// renumbered into the request's COLUMN UNION before interning, so two recipes
/// that both read `Close` share one arena node and a price recipe's `Input{0}`
/// can never be interned onto an OHLC recipe's `Input{0}` (which silently
/// computed on the wrong column while every `SIG_*` equality test stayed green).
///
/// Interning is **mandatory, not opportunistic**: a recipe that binds any
/// [`Node::Input`] without declaring `columns` is rejected at the top of
/// [`merge`], because the silent alternative was the whole hazard — positional
/// interning makes `Input{0}` mean "close" to a price recipe and "high" to an
/// OHLC one inside the same arena, and every `SIG_*` equality test stays green
/// while the VALUES are wrong. There is deliberately no positional path left to
/// fall back to: the migration is finished, so the fallback would only ever
/// conceal a new recipe that forgot to declare.
pub fn merge(recipes: &[Recipe]) -> (Merged, Vec<Resolved>) {
    let mut arena: Vec<Node> = Vec::new();
    let mut tags: Vec<Option<&'static str>> = Vec::new();
    let mut cache: HashMap<Node, Id> = HashMap::new();
    let mut resolved = Vec::with_capacity(recipes.len());

    // A recipe that binds inputs must say what each slot MEANS. Checked before
    // anything is interned, so the failure names the offender instead of
    // producing an arena that merely looks well-formed.
    for r in recipes {
        let binds_input = r.nodes.iter().any(|n| matches!(n, Node::Input { .. }));
        if binds_input && r.columns.is_empty() {
            panic!(
                "cse: recipe {:?} binds inputs but declares no columns -- every recipe must \
                 state its slots via Recipe::price()/hl()/hlc()/hlcvol()",
                r.kind
            );
        }
    }

    // The column union, ranked in `Column`'s DECLARED order (canonical): the
    // stream index a column gets depends only on WHICH columns are present,
    // never on the order recipes were requested — so `Apo + Atr` and `Atr + Apo`
    // erase to the SAME `Signature` and hit one registry entry.
    let mut col_of_slot: Vec<Column> = Vec::new();
    for r in recipes {
        for &c in &r.columns {
            if !col_of_slot.contains(&c) {
                col_of_slot.push(c);
            }
        }
    }
    col_of_slot.sort();
    // recipe-local slot -> arena stream index, per recipe.
    let slot_maps: Vec<Vec<Id>> = recipes
        .iter()
        .map(|r| {
            r.columns
                .iter()
                .map(|c| col_of_slot.iter().position(|u| u == c).unwrap() as Id)
                .collect()
        })
        .collect();

    for (ri, recipe) in recipes.iter().enumerate() {
        // local → global id per recipe; filled in SSA order.
        let mut l2g = vec![0u32; recipe.nodes.len()];
        for (local, node) in recipe.nodes.iter().enumerate() {
            let global = match *node {
                Node::Input { stream } => {
                    let Some(&slot) = slot_maps[ri].get(stream as usize) else {
                        panic!(
                            "cse: recipe {:?} binds Input{{stream: {stream}}} but declares only {} column(s) {:?}",
                            recipe.kind,
                            recipe.columns.len(),
                            recipe.columns
                        );
                    };
                    Node::Input { stream: slot }
                }
                other => remap(&other, &l2g),
            };
            let id = intern(&mut arena, &mut tags, &mut cache, global);
            if let Some(Some(t)) = recipe.tags.get(local) {
                if tags[id as usize].is_none() {
                    tags[id as usize] = Some(*t);
                }
            }
            l2g[local] = id;
        }
        resolved.push(Resolved {
            kind: recipe.kind,
            options: recipe.options.clone(),
            inputs: recipe
                .tail_inputs
                .iter()
                .map(|&i| l2g[i as usize])
                .collect(),
            outputs: recipe.outputs.iter().map(|s| remap_sink(s, &l2g)).collect(),
            optional_outputs: recipe
                .optional_outputs
                .iter()
                .map(|s| remap_sink(s, &l2g))
                .collect(),
            optional_enabled: vec![true; recipe.optional_outputs.len()],
        });
    }

    (Merged { nodes: arena, tags }, resolved)
}
