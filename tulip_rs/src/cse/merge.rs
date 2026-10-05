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

use crate::cse::{Id, Node, Recipe, Sink, SinkSrc, StateRestorer};
use std::collections::HashMap;

/// Merged, node-deduplicated interface graph. Child `Id`s in `nodes` are
/// *global* arena indices.
#[derive(Debug, Clone)]
pub struct Merged {
    pub nodes: Vec<Node>,
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
                Node::Sub { a, b }
                | Node::Add { a, b }
                | Node::Mul { a, b }
                | Node::Div { a, b }
                | Node::Max { a, b } => leads[a as usize].max(leads[b as usize]),
                // A compare-select: defined once all three inputs are.
                Node::WhereGe { a, b, t } => leads[a as usize]
                    .max(leads[b as usize])
                    .max(leads[t as usize]),
                Node::WhereLt { a, b, t } => leads[a as usize]
                    .max(leads[b as usize])
                    .max(leads[t as usize]),
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
                Node::Sma { src, period }
                | Node::Stddev { src, period }
                | Node::Wma { src, period } => leads[src as usize] + period.saturating_sub(1),
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
                Node::HighPass { input, period } => {
                    leads[input as usize] + period.saturating_sub(1)
                }
                Node::SuperSmoother { src, period } => {
                    leads[src as usize] + period.saturating_sub(1)
                }
                Node::HtInPhase { src } | Node::HtQuadrature { src } => leads[src as usize] + 6,
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
        WhereGe { .. } => (12, 2),
        WhereLt { .. } => (13, 2),
        // `sum += x − old` then an optional scale: the Sma/RollSum identity.
        RollSum { mean: false, .. } => (14, 2),
        RollSum { mean: true, .. } => (15, 3),
        Sma { .. } => (15, 3),
        RollMax { .. } => (16, 4),
        RollMin { .. } => (17, 4),
        // Projections/copies: one conversion, or none.
        Trail { .. } => (18, 1),
        Mirror { .. } => (19, 0),
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
    /// `TState` field order — `get_states` feeds their live slot values to
    /// `restorer`, in this order.
    pub inputs: Vec<Id>,
    /// Rebuilds the hand state (from the indicator module). `None` ⇒ Other.
    pub restorer: Option<StateRestorer>,
    /// Primary sinks (global node ids), `Info::outputs` order.
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
/// id. Equal structure ⇒ same id ⇒ shared computation (the CSE key).
fn intern(arena: &mut Vec<Node>, cache: &mut HashMap<Node, Id>, node: Node) -> Id {
    if let Some(&existing) = cache.get(&node) {
        return existing;
    }
    let id = arena.len() as Id;
    arena.push(node);
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
        Abs { src } => Abs { src: g(src) },
        Lag { src, n } => Lag { src: g(src), n },
        Max { a, b } => Max { a: g(a), b: g(b) },
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
        Stddev { src, period } => Stddev {
            src: g(src),
            period,
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
    }
}

/// Visit every **child ref** of `node` (params — `period`, `n`, `stream`,
/// `bits`, `step_bits` — are not graph edges and are skipped). SSA order
/// guarantees each child id `< node` id, so the executor's dead-node pass
/// propagates liveness in one descending scan. A new `Node` variant with a
/// child must be added here **and** `remap_node` **and** `Merged::leads`.
pub(crate) fn node_children(node: &Node, visit: &mut dyn FnMut(Id)) {
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
        | Wilders { src, .. }
        | SuperSmoother { src, .. }
        | Iir { src, .. }
        | Prev { src }
        | HtInPhase { src }
        | HtQuadrature { src } => visit(*src),
        Sub { a, b } | Add { a, b } | Mul { a, b } | Div { a, b } | Max { a, b } => {
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
pub fn merge(recipes: &[Recipe]) -> (Merged, Vec<Resolved>) {
    let mut arena: Vec<Node> = Vec::new();
    let mut cache: HashMap<Node, Id> = HashMap::new();
    let mut resolved = Vec::with_capacity(recipes.len());

    for recipe in recipes {
        // local → global id per recipe; filled in SSA order.
        let mut l2g = vec![0u32; recipe.nodes.len()];
        for (local, node) in recipe.nodes.iter().enumerate() {
            let global = remap(node, &l2g);
            l2g[local] = intern(&mut arena, &mut cache, global);
        }
        resolved.push(Resolved {
            kind: recipe.kind,
            options: recipe.options.clone(),
            inputs: recipe
                .tail_inputs
                .iter()
                .map(|&i| l2g[i as usize])
                .collect(),
            restorer: recipe.restorer,
            outputs: recipe.outputs.iter().map(|s| remap_sink(s, &l2g)).collect(),
            optional_outputs: recipe
                .optional_outputs
                .iter()
                .map(|s| remap_sink(s, &l2g))
                .collect(),
            optional_enabled: vec![true; recipe.optional_outputs.len()],
        });
    }

    (Merged { nodes: arena }, resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cse::Planable;
    use crate::indicators::adx::Adx;
    use crate::indicators::apo::Apo;
    use crate::indicators::ema::Ema;
    use crate::indicators::macd::Macd;
    use crate::indicators::natr::Natr;
    use crate::indicators::ppo::Ppo;
    use crate::indicators::supertrend::SuperTrend;

    /// Group requests and return each group's request indices, sorted, so the
    /// result is order-stable for assertion. `opt[i]` is request `i`'s optional
    /// enable mask (mirrors what `Request::build` applies before compiling).
    fn groups_of(recipes: &[Recipe], opt: &[Vec<bool>]) -> Vec<Vec<usize>> {
        let (arena, mut resolved) = merge(recipes);
        for (r, f) in resolved.iter_mut().zip(opt) {
            r.optional_enabled = f.clone();
        }
        let mut g = arena.sharing_groups(&resolved);
        for grp in &mut g {
            grp.sort_unstable();
        }
        g.sort();
        g
    }

    /// Partition primitive: `apo`+`macd` share `ema12/ema24`; `natr`+`supertrend`
    /// share the mean-`Wilders(tr)`/`tr` chain; `adx` (its `dx`/`dm` chain over
    /// high/low) shares no *computed* node with the others when its optional
    /// `atr`/`tr` mirrors are OFF → three disjoint groups → three sub-plans.
    #[test]
    fn sharing_groups_splits_disjoint_panels() {
        let recipes = [
            Apo::recipe(&[12.0, 24.0]),
            Macd::recipe(&[12.0, 24.0, 9.0]),
            Adx::recipe(&[12.0]),
            Natr::recipe(&[12.0]),
            SuperTrend::recipe(&[12.0, 3.0]),
        ];
        // primary-only: adx's atr/tr mirrors disabled ⇒ disjoint.
        let primary = groups_of(
            &recipes,
            &[
                vec![],
                vec![false, false],
                vec![false, false, false],
                vec![false, false],
                vec![false, false, false],
            ],
        );
        assert_eq!(
            primary,
            vec![vec![0, 1], vec![2], vec![3, 4]],
            "ema-pair / adx / mean-ATR pair are three sharing groups"
        );
        // With adx's optional atr/tr mirror ON, it now reaches the shared
        // mean-`Wilders(tr)` node → joins the natr/supertrend group (2 groups).
        let with_atr = groups_of(
            &recipes,
            &[
                vec![],
                vec![false, false],
                vec![true, true, true],
                vec![false, false],
                vec![false, false, false],
            ],
        );
        assert_eq!(
            with_atr,
            vec![vec![0, 1], vec![2, 3, 4]],
            "adx sharing the ATR mirror merges into the mean-ATR group"
        );
    }

    /// The three classic requests share EVERYTHING shareable as one graph:
    /// `Input` + `ema12` + `ema26` (one chain each), ONE `Sub` line node
    /// (apo's output, macd's line, ppo's numerator — the §4 identity is now
    /// a single arena node), the ppo guard chain (`Const ε`, `Max`,
    /// `Const 100`, `Mul`, `Div`), the chained signal `Ema` over the line,
    /// and the hist `Sub` = 11 nodes. Per-request state wiring stays just
    /// the EMA chains (`tail_inputs`), macd's naming its signal chain too.
    #[test]
    fn merge_dedups_classic_graph() {
        let (m, r) = merge(&[
            Apo::recipe(&[12.0, 26.0]),
            Ppo::recipe(&[12.0, 26.0]),
            Macd::recipe(&[12.0, 26.0, 9.0]),
        ]);
        assert_eq!(
            m.nodes.len(),
            crate::cse::CLASSIC_NODES,
            "one shared graph, no duplicates"
        );
        let n = |p: fn(&Node) -> bool| m.nodes.iter().filter(|n| p(n)).count();
        assert_eq!(n(|x| matches!(x, Node::Ema { .. })), 3, "e12, e26, sig");
        assert_eq!(n(|x| matches!(x, Node::Sub { .. })), 2, "line + hist");
        // ppo's divisor guard: `Const ε` + `Max`, both absent under
        // `safe_inputs` (see `cse/mod.rs`). Counted separately from the graph
        // size so a lost node is still attributable.
        assert_eq!(
            n(|x| matches!(x, Node::Const { .. })),
            if cfg!(feature = "safe_inputs") { 1 } else { 2 },
            "100, plus eps when the guard exists"
        );
        assert_eq!(
            n(|x| matches!(x, Node::Max { .. })),
            if cfg!(feature = "safe_inputs") { 0 } else { 1 }
        );
        assert_eq!(n(|x| matches!(x, Node::Mul { .. })), 1);
        assert_eq!(n(|x| matches!(x, Node::Div { .. })), 1);

        // state wiring: apo/ppo name the same two chains; macd adds the
        // signal chain (an arena Ema over the shared line).
        assert_eq!(r[0].inputs, vec![1, 2]);
        assert_eq!(r[1].inputs, vec![1, 2]);
        assert_eq!(r[2].inputs.len(), 3);
        assert_eq!(&r[2].inputs[..2], &[1, 2]);

        // defined-from leads: ema12 → 11, ema26 → 25, line → 25, sig → 33
        let leads = m.leads();
        assert_eq!(leads[1], 11);
        assert_eq!(leads[2], 25);
        assert_eq!(leads[r[2].inputs[2] as usize], 33, "signal = 25 + 9 - 1");

        // every sink is a node mirror (no tail slots survive)
        assert!(matches!(
            r[0].optional_outputs[0].src,
            SinkSrc::Interface(id) if id == 1
        ));
        assert!(matches!(r[1].outputs[0].src, SinkSrc::Interface(_)));
        assert!(matches!(
            r[2].optional_outputs[1].src,
            SinkSrc::Interface(id) if id == 2
        ));
    }

    /// The dense dashboard's PARTIAL overlap: `Apo(6,9)`'s long EMA and
    /// `Ppo(9,25)`'s short EMA are both `ema9(close)` — one structural key,
    /// so the arena holds exactly ONE price-sourced `Ema{period: 9}` node
    /// and the two requests receive the same global id (one advance per bar;
    /// both consumers read the same chain). `Macd(12,24,9)`'s signal is ALSO
    /// period 9 but over the line node — a different key, correctly NOT
    /// deduped. All other periods are distinct: 11 close EMAs + 2 signal
    /// chains + the per-request derived graph = 30 nodes.
    #[test]
    fn merge_dedups_dense_shared_ema9() {
        let (m, r) = merge(&[
            Apo::recipe(&[5.0, 8.0]),
            Apo::recipe(&[6.0, 9.0]),
            Ppo::recipe(&[9.0, 25.0]),
            Ppo::recipe(&[10.0, 26.0]),
            Macd::recipe(&[11.0, 23.0, 17.0]),
            Macd::recipe(&[12.0, 24.0, 9.0]),
        ]);
        let ema = |p: u32| -> Vec<Id> {
            m.nodes
                .iter()
                .enumerate()
                .filter(
                    |(_, n)| matches!(**n, Node::Ema { src, period } if period == p && src == 0),
                )
                .map(|(i, _)| i as Id)
                .collect()
        };
        assert_eq!(
            ema(9).len(),
            1,
            "ema9(close) exists exactly once (the period-9 SIGNAL chain is over the line, not close)"
        );
        // Apo(6,9) = request 1: state chains [ema6, ema9];
        // Ppo(9,25) = request 2: state chains [ema9, ema25].
        assert_eq!(
            r[1].inputs[1], r[2].inputs[0],
            "Apo long(9) and Ppo short(9) must be the SAME arena node"
        );
        let n_price_emas = m
            .nodes
            .iter()
            .filter(|n| matches!(n, Node::Ema { src, .. } if *src == 0))
            .count();
        assert_eq!(
            n_price_emas, 11,
            "periods 5,6,8,9,10,11,12,23,24,25,26 over the single close Input"
        );
        assert_eq!(m.nodes.len(), crate::cse::DENSE_NODES);
    }

    /// `lane_formula` id → a name, for report lines only. `IIR2` is the
    /// `Ema`/mean-`Wilders` identity, `Roll3` the `Sma`/`RollSum{mean}` one.
    fn formula_name(f: u8) -> &'static str {
        [
            "?",
            "Input",
            "Const",
            "IIR2",
            "IIR1",
            "Sub",
            "Add",
            "Mul",
            "Div",
            "Abs",
            "Lag",
            "Max",
            "WhereGe",
            "WhereLt",
            "Roll2",
            "Roll3",
            "RollMax",
            "RollMin",
            "Trail",
            "Mirror",
            "Ln",
            "Sqrt",
            "Fma",
            "Hma",
            "Stddev",
            "Wma",
            "HighPass",
            "SuperSmoother",
            "HtInPhase",
            "HtQuadrature",
            "SuperTrend",
        ][f as usize]
    }

    fn lane_report(m: &Merged) -> String {
        let mut out = String::new();
        let mut rs = m.lane_regions();
        rs.sort_by_key(|r| (r.shared_ops, r.formula));
        for r in rs.iter().rev() {
            out.push_str(&format!(
                "    {:<12} lanes={} own_ops={} run_ops={}{}\n",
                formula_name(r.formula),
                r.lanes.len(),
                r.ops,
                r.shared_ops,
                if r.uniform_warmup {
                    ""
                } else {
                    "  (warm-up differs: not packable AS IS)"
                },
            ));
        }
        out
    }

    /// The rule the user set: packing is a property of the MERGED GRAPH, decided
    /// by shared arithmetic and how many instructions that sharing runs over —
    /// never by node kind. This prints what the pass finds across the families
    /// that currently lose to the hand, so the theory can be reviewed against the
    /// graph before any emission consumes it
    /// (`cargo test -p tulip_rs cse::merge -- --nocapture`).
    #[test]
    fn lane_regions_report_the_shared_shapes() {
        use crate::indicators::cmo::Cmo;
        use crate::indicators::di::Di;
        use crate::indicators::rsi::Rsi;
        use crate::indicators::willr::Willr;
        let cases: Vec<(String, Vec<Recipe>)> = vec![
            ("rsi(14)".into(), vec![Rsi::recipe(&[14.0])]),
            ("cmo(14)".into(), vec![Cmo::recipe(&[14.0])]),
            ("di(14)".into(), vec![Di::recipe(&[14.0])]),
            ("willr(14)".into(), vec![Willr::recipe(&[14.0])]),
            (
                "rsi+cmo".into(),
                vec![Rsi::recipe(&[14.0]), Cmo::recipe(&[14.0])],
            ),
            ("apo(12,26)".into(), vec![Apo::recipe(&[12.0, 26.0])]),
            ("adx(14)".into(), vec![Adx::recipe(&[14.0])]),
            (
                "ema(9)+wilders(9)".into(),
                vec![
                    Ema::recipe(&[9.0]),
                    crate::indicators::wilders::Wilders::recipe(&[9.0]),
                ],
            ),
        ];
        for (name, recipes) in cases {
            let (m, _) = merge(&recipes);
            print!("=== {name} ({} nodes)\n{}", m.nodes.len(), lane_report(&m));
        }
    }

    /// THE point of keying on the expansion instead of the `Node` variant:
    /// `ema::calc` and `wilders::calc` are one instruction sequence
    /// (`fma(state, c0, mul(x, c1))`) with the two magnitudes in swapped slots,
    /// and magnitudes are not part of a shape. So a chain spelled `Ema` and a
    /// chain spelled `Wilders` are TWO LANES OF ONE FORMULA — a share no
    /// kind-keyed rule can see, and the reason "ema packs / wilders packs" was
    /// the wrong decomposition.
    #[test]
    fn lane_regions_see_ema_and_wilders_as_one_formula() {
        use crate::indicators::wilders::Wilders;
        let (m, _) = merge(&[Ema::recipe(&[9.0]), Wilders::recipe(&[9.0])]);
        let iir = m
            .lane_regions()
            .into_iter()
            .find(|r| r.formula == 3)
            .expect("an Ema and a mean Wilders over the same source are ONE shape");
        assert_eq!(iir.lanes.len(), 2, "one region, two lanes");
        assert_eq!(iir.ops, 2, "fmul + fma per bar, per lane");
        assert_eq!(iir.shared_ops, 2, "the chain alone amortizes one assembly");
        assert!(
            iir.lanes
                .iter()
                .all(|&i| matches!(m.nodes[i as usize], Node::Ema { .. } | Node::Wilders { .. })),
            "the lanes are the two spellings: {:?}",
            iir.lanes
        );
        let kinds: Vec<&str> = iir
            .lanes
            .iter()
            .map(|&i| match m.nodes[i as usize] {
                Node::Ema { .. } => "Ema",
                Node::Wilders { .. } => "Wilders",
                _ => "?",
            })
            .collect();
        assert_eq!(
            kinds,
            vec!["Ema", "Wilders"],
            "one lane of each spelling — cross-kind packing, found structurally"
        );
        // They earn the pack, but they do NOT yet share a loop: `Ema` steps from
        // its seed while a price-sourced `Wilders` FOLDS a running sum to the
        // seam, so the warm-up policy differs and one segment structure cannot
        // serve both as it stands. Reported, not hidden — this is the gate the
        // emission has to clear before claiming this region.
        assert!(
            !iir.uniform_warmup,
            "Ema and Wilders agree in steady math but differ in warm-up policy"
        );
    }

    /// `rsi`'s gain/loss pair, and the case that makes the instruction count mean
    /// something: the shared movement subtraction is ONE instruction with nothing
    /// else shared under it, so it is not offered at all.
    #[test]
    fn lane_regions_find_the_rsi_up_down_run() {
        use crate::indicators::rsi::Rsi;
        let (m, _) = merge(&[Rsi::recipe(&[14.0])]);
        let at = |f: u8| m.lane_regions().into_iter().find(|r| r.formula == f);
        // `diff = real − lag` and `neg = lag − real`: SAME arithmetic, operands
        // swapped, different sources — so they share a shape DESPITE the swap
        // (operand order is not part of a shape). But one `fsub` does not pay for
        // an assembly, so the pass must NOT offer it.
        assert!(
            at(5).is_none(),
            "the lone shared Sub (1 instruction) must not be offered for packing"
        );
        // Both guards: one `fcmp`+`select` pair of their own, sitting on the
        // shared Sub, so the run is 2 instructions wide.
        let max = at(11).expect("the two Max guards share a shape");
        assert_eq!(max.lanes.len(), 2);
        assert_eq!(max.ops, 1, "the guard itself is one vector op");
        assert_eq!(max.shared_ops, 2, "its own op + the shared Sub's");
        // Both smoothers, spelled `Wilders`: two instructions each, on top of the
        // guard run ⇒ a 4-instruction run for one pack/unpack.
        let iir = at(3).expect("the two Wilders share a shape");
        assert_eq!(iir.lanes.len(), 2);
        assert_eq!(iir.ops, 2, "fmul + fma");
        assert_eq!(iir.shared_ops, 4, "Wilders(2) + Max(1) + Sub(1)");
        assert!(iir.uniform_warmup, "both arms: same period, same feed");
    }

    /// `willr` is the control on the pass: its two rolls go over DIFFERENT
    /// streams and need opposite predicates, so they are not one formula, and
    /// nothing else in it is done twice. A pass that invented regions here would
    /// be pattern-matching indicators, not reading math.
    #[test]
    fn lane_regions_find_nothing_to_pack_in_willr() {
        use crate::indicators::willr::Willr;
        let (m, _) = merge(&[Willr::recipe(&[14.0])]);
        assert!(
            m.lane_regions().is_empty(),
            "willr rolls max-of-high and min-of-low: different formulas, and no \
             other shape appears twice — got {:?}",
            m.lane_regions()
        );
    }

    /// A query with no parallel ARITHMETIC proposes nothing: every shape is
    /// either unique (so `intern` already did the sharing) or a boundary slot
    /// with nothing to compute — two input streams are two lanes of *reading*,
    /// not two lanes of work.
    #[test]
    fn lane_regions_report_nothing_when_interning_already_did_the_work() {
        use crate::indicators::medprice::Medprice;
        let (m, _) = merge(&[Medprice::recipe(&[])]);
        assert!(
            m.lane_regions().is_empty(),
            "medprice is 0.5·(high+low) over two DIFFERENT streams — one node \
             per arithmetic shape, and its two `Input`s are boundaries, not a \
             region: {:?}",
            m.lane_regions()
        );
    }

    /// Periods are magnitudes, not shape: `Apo(12, 26)` is TWO instances of the
    /// same formula over the same source. It earns its pack on the chain's OWN
    /// two instructions — no second rule about carrying state is needed, which is
    /// what made the single criterion possible.
    #[test]
    fn lane_regions_pack_a_chain_pair_on_its_own_instruction_count() {
        let (m, _) = merge(&[Apo::recipe(&[12.0, 26.0])]);
        let ema = m
            .lane_regions()
            .into_iter()
            .find(|r| r.formula == 3)
            .expect("ema12 and ema26 are one formula with different periods");
        assert_eq!(ema.lanes.len(), 2);
        assert_eq!(ema.ops, 2, "the step is a group: fmul then fma");
        assert_eq!(
            ema.shared_ops, 2,
            "the run stops at the chain — `close` is one deduped node"
        );
        assert!(ema.uniform_warmup, "both are price-sourced Emas");
    }
}
