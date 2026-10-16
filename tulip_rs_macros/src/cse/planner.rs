//! The macro-side expansion (Phase 2 of "recipes as data"): `kernel!`
//! pattern args → the REAL leaf recipe graphs
//! (`tulip_rs_shared::cse::recipes`) → the SAME `merge()` the runtime
//! planner runs → the arena converted into the emitter's [`Pattern`].
//!
//! The old hand-transcribed per-recipe graph templates are GONE: this
//! module consumed copies of the graphs and could drift from the runtime
//! recipes (the `*_sig_derived` tests existed to catch exactly that class);
//! now a recipe edit lands in the generated kernels by construction, one
//! source of truth.
//!
//! Recipe args arrive as [`Arg`]s: `Lit` behaves exactly like the runtime
//! `Planable::recipe` call it reproduces; `Name` symbols become distinct
//! SENTINEL values in the `SENTINEL_BASE + i` band, so dedup-by-identity
//! holds — same name across recipes ⇒ same node ⇒ one shared chain;
//! different names ⇒ distinct chains whatever the runtime values turn out
//! to be (a request binding equal values to two names dedups at runtime,
//! misses the `Signature`, and falls to tier 2 — correctness by fallback,
//! never by luck). Real periods live far below the band; a `Lit` reaching
//! into it is a loud expansion panic, never a silent collision.
//!
//! Generated code never bakes a number: period values, leads, and the
//! optional want-bits all bind at runtime through `Values`.

use std::collections::HashMap;
use tulip_rs_shared::cse::merge::merge;
use tulip_rs_shared::cse::recipe::Node;
use tulip_rs_shared::cse::recipes;
use tulip_rs_shared::cse::Recipe;
use tulip_rs_shared::cse::SinkSrc;
use tulip_rs_shared::cse::{bundles, Bundle, Plan, MAX};

/// A `kernel!` pattern argument: a NAME symbol (dedup identity only) or a
/// literal value (participates in dedup by equality, like the runtime).
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Name(String),
    Lit(f64),
}

/// The sentinel band base for `Arg::Name` values. Named args never carry a
/// value (params stay runtime); they only need DISTINCT f64s that survive
/// the recipes' `as u32` period casts and stay far above any real period.
const SENTINEL_BASE: f64 = 1_000_000.0;

/// The node kinds the fused emitter can lower today. `Kind::tag()` MUST
/// equal the runtime `signature::K_*` tag for the same node kind — the
/// `SIG_*` consts are built with these tags and matched against erased
/// runtime signatures bar-for-signature.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Kind {
    Input,
    Const,
    Ema,
    Sub,
    Add,
    Mul,
    Div,
    Max,
    /// IEEE minNum — the exact mirror of `Max`, both ways (scalar `f64::min`,
    /// lane `simd_min`), which is the hand's own split in `ultosc.rs` /
    /// `ultosc_simd.rs`. NOT a `WhereLt` select: that pins `0.0` on one arm.
    Min,
    Lag,
    Abs,
    Wilders,
    SuperTrend,
    Sma,
    Stddev,
    Wma,
    RollMax,
    RollMin,
    Trail,
    WhereGe,
    WhereLt,
    /// Strict select, value on `>` — `K_WHEREGT` (31).
    WhereGt,
    /// Guard select, zero on `<=` — `K_WHERELE` (32). The polarity an
    /// early-return guard like `ad`'s `range <= eps` needs (see `Node::WhereLe`).
    WhereLe,
    /// Single-rounding running total `st = a.mul_add(b, st)` — `K_ACCUM` (33);
    /// `a` is the node's own carry, so no existing kind can spell it.
    Accum,
    Fma,
    /// `ln(src)` host call — `K_LN`.
    Ln,
    /// Explicit-weight IIR line — `K_IIR`.
    Iir,
    /// IIR + the hand's clamp folded into the carry — `K_IIRCLAMPED`; the
    /// five refs ride `a`=src, `b`=hi_t, `c`=hi_v, `d`=lo_t, `e`=lo_v.
    IirClamped,
    /// The chain's carried-IN value (delayed copy in `st[k]`) — `K_PREV`.
    Prev,
    /// Ring-backed rolling sum over any source (derived included) — the
    /// emitter's `K_ROLLSUM` arm; group/capacity ride `Values` at runtime.
    RollSum,
    /// The bespoke Hull MA machine (two inner WMA engines + diff ring) —
    /// `K_HMA`; everything it needs rides `Values` per the erase arm.
    Hma,
    /// Zero-op identity copy of its source — `K_MIRROR` (12). The hand's
    /// state mirrors (rsi's `prev_real`, cmo's `prev`/`drop_real`): the
    /// value exists for the chunk-exit state store (`get_states`, M1), so
    /// the steady loop computes it and stores nothing.
    Mirror,
}

impl Kind {
    /// the runtime `signature::K_*` tag for this kind (must match!).
    pub const fn tag(self) -> u8 {
        match self {
            Kind::Input => 1,
            Kind::Const => 2,
            Kind::Ema => 3,
            Kind::Sub => 5,
            Kind::Add => 6,
            Kind::Mul => 7,
            Kind::Div => 8,
            Kind::Max => 9,
            Kind::Min => 30,
            Kind::Abs => 10,
            Kind::Lag => 11,
            Kind::Wilders => 4,
            Kind::SuperTrend => 15,
            Kind::Sma => 16,
            Kind::Stddev => 17,
            Kind::Wma => 18,
            Kind::RollMax => 19,
            Kind::RollMin => 20,
            Kind::Trail => 21,
            Kind::WhereGe => 22,
            Kind::WhereLt => 23,
            Kind::WhereGt => 31,
            Kind::WhereLe => 32,
            Kind::Accum => 33,
            Kind::Fma => 14,
            Kind::Ln => 26,
            Kind::Iir => 27,
            Kind::IirClamped => 28,
            Kind::Prev => 29,
            Kind::RollSum => 24,
            Kind::Hma => 25,
            Kind::Mirror => 12,
        }
    }
}

/// One emitter node: kind + up to three arena-slot operands (values live
/// in `Values` at runtime, never here).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ANode {
    pub kind: Kind,
    pub a: usize,
    pub b: usize,
    /// Optional third operand (SuperTrend: close stream).
    pub c: Option<usize>,
    /// 5th/6th arena refs for K_IIRCLAMPED (lo_t/lo_v); 0 elsewhere —
    /// mirrors `Signature::d/e` (erase writes the same pairs).
    pub d: usize,
    pub e: usize,
}

impl ANode {
    fn new_simple(kind: Kind, a: usize, b: usize) -> Self {
        Self {
            kind,
            a,
            b,
            c: None,
            d: 0,
            e: 0,
        }
    }

    fn new_three(kind: Kind, a: usize, b: usize, c: usize) -> Self {
        Self {
            kind,
            a,
            b,
            c: Some(c),
            d: 0,
            e: 0,
        }
    }
}

/// A compiled pattern: merged arena + sink wiring, ready for the emitter.
#[derive(Clone, Debug)]
pub struct Pattern {
    pub name: String,
    pub nodes: Vec<ANode>,
    /// Display role names, from the recipe graphs' tags (first recipe to
    /// declare a shared node owns its name).
    pub names: Vec<Option<String>>,
    pub sinks: Vec<usize>,
    /// If true, same-source Ema chains run as SIMD lane groups (4-wide
    /// then 2-wide, remainder scalar). Chained EMAs and Wilders stay scalar.
    pub packed: bool,
    /// The planner's lowering plan: ring-group partition and class, plus the
    /// `Price`/`Chain` stream mode — the layout facts the renderers READ
    /// instead of re-deriving from arena kinds. Derived from this arena's own
    /// kind/edge arrays through `lowering::Plan::compute`, the same function
    /// `Signature::build` (the `SIG_*` const) and runtime `erase` publish
    /// through, so expansion and runtime cannot disagree — the sig-consistency
    /// tests compare these fields as part of full struct equality.
    pub plan: Plan,
    /// The container plan: which pure-sum ring groups share ONE fused
    /// multi-series buffer (see [`Bundle`]). A group that appears in no bundle
    /// keeps the plain per-group vector buffer. The planner owns this choice —
    /// the emitter renders the containers this lists and never picks one.
    pub bundles: Vec<Bundle>,
    /// The sink partition: which sinks share ONE physical output row. Two
    /// sinks share a row iff they store the same arena node (the dedup that
    /// makes a mirror free — the trio's apo line and macd line are one row,
    /// one store per bar). Group `i`'s `members[0]` is its leader; `group_of`
    /// maps a sink index to its GROUP index (never the leader's sink index —
    /// the two diverge as soon as a duplicate sink precedes a later new
    /// group, and reading one as the other indexes out of range).
    pub sink_plan: SinkPlan,
}

/// The planner's output-row partition, read by BOTH renderers in place of
/// each building its own. Derived from `sinks`/`sink_is_primary` alone, so
/// the scalar and lane kernels cannot disagree about how many rows exist,
/// who leads them, or which are optional-only.
#[derive(Clone, Debug)]
pub struct SinkPlan {
    /// Sink indices per group, in first-appearance order.
    pub groups: Vec<Vec<usize>>,
    /// Sink index → group index.
    pub group_of: Vec<usize>,
    /// Per group: true = every member is a declared OPTIONAL sink, so the
    /// row's store is want-guarded; false = a PRIMARY member stores
    /// unconditionally.
    pub optional: Vec<bool>,
}

/// Partition sinks by the arena node they store to, in first-appearance
/// order. The one implementation of the row-dedup rule; `expand` attaches the
/// result to the pattern and both renderers read it.
pub fn partition_sinks(sinks: &[usize], sink_is_primary: &[bool]) -> SinkPlan {
    let mut by_node: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut group_of: Vec<usize> = Vec::with_capacity(sinks.len());
    for (sidx, &node_idx) in sinks.iter().enumerate() {
        let g = match by_node.get(&node_idx) {
            Some(&g) => g,
            None => {
                let g = groups.len();
                by_node.insert(node_idx, g);
                groups.push(Vec::new());
                g
            }
        };
        groups[g].push(sidx);
        group_of.push(g);
    }
    let optional = groups
        .iter()
        .map(|members| members.iter().all(|s| !sink_is_primary[*s]))
        .collect();
    SinkPlan {
        groups,
        group_of,
        optional,
    }
}

/// Build one pattern from `(recipe-id, args)` in request order: construct
/// the real leaf recipes (named args → sentinels), run the runtime `merge`,
/// and convert the arena for the emitter.
pub fn expand(name: &str, items: &[(String, Vec<Arg>)], packed: bool) -> Pattern {
    let mut names: HashMap<String, f64> = HashMap::new();
    let mut built: Vec<Recipe> = Vec::with_capacity(items.len());
    for (rid, args) in items {
        // The SuperTrend step is a per-pattern literal, not a runtime name
        // (the op node keys on its bits — a Name would silently fake an
        // identity): reject it loudly, as the old planner template did.
        if rid == "Supertrend" {
            if let Some(Arg::Name(step)) = args.get(1) {
                panic!("kernel!: Supertrend step must be a literal, got name `{step}`");
            }
        }
        let opts: Vec<f64> = args
            .iter()
            .map(|a| match a {
                Arg::Lit(v) => {
                    assert!(
                        v.abs() < SENTINEL_BASE,
                        "kernel!: literal {v} reaches the name-sentinel band"
                    );
                    *v
                }
                Arg::Name(n) => {
                    let next = SENTINEL_BASE + names.len() as f64;
                    *names.entry(n.clone()).or_insert(next)
                }
            })
            .collect();
        let r = match rid.as_str() {
            "Apo" => recipes::apo(&opts),
            "Ppo" => recipes::ppo(&opts),
            "Macd" => recipes::macd(&opts),
            "Ema" => recipes::ema(&opts),
            "Atr" => recipes::atr(&opts),
            "Natr" => recipes::natr(&opts),
            "Medprice" => recipes::medprice(&opts),
            "Supertrend" => recipes::supertrend(&opts),
            "Sma" => recipes::sma(&opts),
            "Stddev" => recipes::stddev(&opts),
            "Wma" => recipes::wma(&opts),
            "Bbands" => recipes::bbands(&opts),
            "Max" => recipes::max(&opts),
            "Min" => recipes::min(&opts),
            "Aroon" => recipes::aroon(&opts),
            "AroonOsc" => recipes::aroonosc(&opts),
            "Willr" => recipes::willr(&opts),
            "DonchianChannel" => recipes::donchianchannel(&opts),
            "Ichimoku" => recipes::ichimoku(&opts),
            "Ao" => recipes::ao(&opts),
            "Hma" => recipes::hma(&opts),
            "Fisher" => recipes::fisher(&opts),
            "Stoch" => recipes::stoch(&opts),
            "Rsi" => recipes::rsi(&opts),
            "Stochrsi" => recipes::stochrsi(&opts),
            "Cmo" => recipes::cmo(&opts),
            "Ultosc" => recipes::ultosc(&opts),
            "Mass" => recipes::mass(&opts),
            "Ad" => recipes::ad(&opts),
            "Adosc" => recipes::adosc(&opts),
            "Dm" => recipes::dm(&opts),
            "Di" => recipes::di(&opts),
            "Dx" => recipes::dx(&opts),
            "Adx" => recipes::adx(&opts),
            "Mom" => recipes::mom(&opts),
            other => panic!("kernel!: unknown recipe {other}"),
        };
        built.push(r);
    }

    // THE SAME merge the runtime planner runs — dedup, interning, and arena
    // order are identical to the erased signature by construction.
    let (arena, resolved) = merge(&built);

    let mut nodes = Vec::with_capacity(arena.nodes.len());
    for node in &arena.nodes {
        // Node kind → emitter kind + operand slots. Values (periods, bits,
        // stream ids beyond Input) stay in the node for runtime erase; the
        // emitter only reads structure. Anything outside the covered set is
        // the emitter's next coverage, never a silent fallback.
        let a = match *node {
            Node::Input { stream } => ANode::new_simple(Kind::Input, stream as usize, 0),
            Node::Const { .. } => ANode::new_simple(Kind::Const, 0, 0),
            Node::Ema { src, .. } => ANode::new_simple(Kind::Ema, src as usize, 0),
            // COMPOSITE rides the SAME slot `Signature::b` uses for it at
            // runtime (`erase` writes `(K_WILDERS, src, composite, 0)`), and
            // `sig_const` copies `x.b` straight into the generated `SIG_*`.
            // Hard-coding 0 here — the old spelling — made the macro-side
            // arena disagree with runtime `erase` for every composite node, so
            // the `*_sig_derived` gates could never pass a `dm`/`di` shape.
            Node::Wilders { src, composite, .. } => {
                ANode::new_simple(Kind::Wilders, src as usize, composite as usize)
            }
            Node::Sub { a, b } => ANode::new_simple(Kind::Sub, a as usize, b as usize),
            Node::Add { a, b } => ANode::new_simple(Kind::Add, a as usize, b as usize),
            Node::Mul { a, b } => ANode::new_simple(Kind::Mul, a as usize, b as usize),
            Node::Div { a, b } => ANode::new_simple(Kind::Div, a as usize, b as usize),
            Node::Max { a, b } => ANode::new_simple(Kind::Max, a as usize, b as usize),
            Node::Min { a, b } => ANode::new_simple(Kind::Min, a as usize, b as usize),
            Node::Abs { src } => ANode::new_simple(Kind::Abs, src as usize, 0),
            Node::Lag { src, .. } => ANode::new_simple(Kind::Lag, src as usize, 0),
            Node::Mirror { src } => ANode::new_simple(Kind::Mirror, src as usize, 0),
            Node::Sma { src, .. } => ANode::new_simple(Kind::Sma, src as usize, 0),
            Node::Stddev { src, .. } => ANode::new_simple(Kind::Stddev, src as usize, 0),
            Node::Wma { src, .. } => ANode::new_simple(Kind::Wma, src as usize, 0),
            Node::RollMax { src, .. } => ANode::new_simple(Kind::RollMax, src as usize, 0),
            Node::RollMin { src, .. } => ANode::new_simple(Kind::RollMin, src as usize, 0),
            Node::Trail { src } => ANode::new_simple(Kind::Trail, src as usize, 0),
            Node::WhereGe { a, b, t } => {
                ANode::new_three(Kind::WhereGe, a as usize, b as usize, t as usize)
            }
            Node::WhereLt { a, b, t } => {
                ANode::new_three(Kind::WhereLt, a as usize, b as usize, t as usize)
            }
            Node::WhereGt { a, b, t } => {
                ANode::new_three(Kind::WhereGt, a as usize, b as usize, t as usize)
            }
            Node::WhereLe { a, b, t } => {
                ANode::new_three(Kind::WhereLe, a as usize, b as usize, t as usize)
            }
            Node::Accum { a, b } => ANode::new_simple(Kind::Accum, a as usize, b as usize),
            Node::Fma { a, b, c } => {
                ANode::new_three(Kind::Fma, a as usize, b as usize, c as usize)
            }
            Node::RollSum { src, .. } => {
                // skip/steady_from ride in VALUES (erase stores them at the
                // slot); the structure is one RollSum kind — the warm
                // fold/seam/init/steady regions read the two numbers per
                // bar, so nothing remains to refuse here.
                ANode::new_simple(Kind::RollSum, src as usize, 0)
            }
            Node::Hma { src, .. } => ANode::new_simple(Kind::Hma, src as usize, 0),
            Node::Ln { src } => ANode::new_simple(Kind::Ln, src as usize, 0),
            Node::Iir { src, .. } => ANode::new_simple(Kind::Iir, src as usize, 0),
            Node::Prev { src } => ANode::new_simple(Kind::Prev, src as usize, 0),
            Node::IirClamped {
                src,
                hi_t,
                hi_v,
                lo_t,
                lo_v,
                ..
            } => ANode {
                kind: Kind::IirClamped,
                a: src as usize,
                b: hi_t as usize,
                c: Some(hi_v as usize),
                d: lo_t as usize,
                e: lo_v as usize,
            },
            Node::SuperTrend {
                atr, med, close, ..
            } => ANode::new_three(Kind::SuperTrend, atr as usize, med as usize, close as usize),
            other => {
                panic!("kernel!: arena node `{other:?}` is not covered by the fused emitter yet")
            }
        };
        nodes.push(a);
    }

    let names: Vec<Option<String>> = arena.tags.iter().map(|t| t.map(String::from)).collect();

    // Sinks in Resolved order: per recipe, primaries then optionals. Every
    // DECLARED optional stays in the list — enable/disable is the runtime
    // `sink_want` bit, never an arena difference.
    let mut sinks = Vec::new();
    let mut sink_is_primary = Vec::new();
    for r in &resolved {
        for s in &r.outputs {
            let SinkSrc::Interface(id) = s.src;
            sinks.push(id as usize);
            sink_is_primary.push(true);
        }
        for s in &r.optional_outputs {
            let SinkSrc::Interface(id) = s.src;
            sinks.push(id as usize);
            sink_is_primary.push(false);
        }
    }

    // The lowering plan: the ring partition/class and the `Price`/`Chain`
    // stream mode, derived from THIS arena's own kind/edge arrays through the
    // one function `Signature::build` runs when it emits the `SIG_*` const and
    // `erase` runs at runtime. Expansion, key, and runtime binding are then
    // one fact seen three times — the renderers read `plan` instead of
    // re-scanning arena kinds for a layout decision.
    assert!(
        nodes.len() <= MAX,
        "kernel!: arena of {} nodes exceeds MAX ({})",
        nodes.len(),
        MAX
    );
    let mut kinds = [0u8; MAX];
    let mut a_slots = [0u32; MAX];
    let mut b_slots = [0u32; MAX];
    let mut c_slots = [0u32; MAX];
    let mut d_slots = [0u32; MAX];
    let mut e_slots = [0u32; MAX];
    for (k, node) in nodes.iter().enumerate() {
        kinds[k] = node.kind.tag();
        a_slots[k] = node.a as u32;
        b_slots[k] = node.b as u32;
        c_slots[k] = node.c.unwrap_or(0) as u32;
        d_slots[k] = node.d as u32;
        e_slots[k] = node.e as u32;
    }
    let plan = Plan::compute(&kinds, &a_slots, nodes.len());
    // The container plan: which pure-sum ring groups share ONE fused buffer.
    // Value-independence is dataflow over the arena's operand edges, so it is
    // derived here from the same arrays the key is built from — the emitter
    // never decides a container, it emits the containers this lists.
    let bundles = bundles(
        &kinds,
        &a_slots,
        &b_slots,
        &c_slots,
        &d_slots,
        &e_slots,
        nodes.len(),
    );

    // The output-row partition (which sinks share one physical row), computed
    // here so both renderers read ONE structure instead of each building its
    // own — the scalar renderer used to rebuild it and then re-derive group
    // membership twice more by scanning every group's member list.
    let sink_plan = partition_sinks(&sinks, &sink_is_primary);

    Pattern {
        name: name.to_string(),
        nodes,
        names,
        sinks,
        packed,
        plan,
        bundles,
        sink_plan,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atr_arena_has_the_three_price_streams() {
        let pattern = expand("atr", &[("Atr".to_string(), vec![Arg::Lit(14.0)])], false);

        // Find all Input nodes and their stream indices
        let mut max_stream = 0;
        for node in &pattern.nodes {
            if matches!(node.kind, Kind::Input) {
                max_stream = max_stream.max(node.a);
            }
        }
        assert_eq!(max_stream, 2, "ATR should use streams 0,1,2");

        // role tags arrive from the recipe data (the emitter's idents)
        let tagged: Vec<&str> = pattern
            .names
            .iter()
            .enumerate()
            .filter_map(|(i, n)| n.as_deref().map(|n| (i, n)))
            .map(|(_, n)| n)
            .collect();
        assert!(
            tagged.contains(&"tr") && tagged.contains(&"atr") && tagged.contains(&"prev_close"),
            "recipe tags should name the atr family nodes: {tagged:?}"
        );
    }

    /// Group EMA slots by source Input and chunk into 4/2 lane groups.
    /// Returns `(width, Vec<slot_indices>)` in bucket-first-appearance order.
    /// - Same-source Ema slots (a targets Kind::Input) are grouped
    /// - Chained Ema (a targets non-Input) and Wilders: scalar only
    /// - Grouping strategy: greedily chunk `[4,4,...,2,2,r]` (prefer 4s)
    pub fn packed_groups(p: &Pattern) -> Vec<(usize, Vec<usize>)> {
        if !p.packed {
            return vec![];
        }

        // Bucket by source Input slot
        let mut buckets: std::collections::HashMap<usize, Vec<usize>> =
            std::collections::HashMap::new();
        for (k, node) in p.nodes.iter().enumerate() {
            if node.kind == Kind::Ema && p.nodes[node.a].kind == Kind::Input {
                // Group by source stream
                let src_stream = p.nodes[node.a].a;
                buckets.entry(src_stream).or_default().push(k);
            }
        }

        // Sort buckets by first appearance order (source stream index)
        let mut streams: Vec<usize> = buckets.keys().cloned().collect();
        streams.sort();

        let mut result = Vec::new();
        for src in streams {
            let slots = buckets.remove(&src).unwrap_or_default();
            if slots.is_empty() {
                continue;
            }
            // Chunk into 4/2 lane groups
            let mut remaining = slots;
            while !remaining.is_empty() {
                if remaining.len() >= 4 {
                    result.push((4, remaining.drain(0..4).collect()));
                } else if remaining.len() >= 2 {
                    result.push((2, remaining.drain(0..2).collect()));
                } else {
                    // Remainder 1 → scalar (not packed)
                    break;
                }
            }
        }

        result
    }

    /// packed grouping over the six-distinct-EMA shape: buckets of 4 then 2.
    #[test]
    fn packed_groups_chunks_4_then_2() {
        let p = expand(
            "ema_six_p",
            &[
                (
                    "Macd".to_string(),
                    vec![
                        Arg::Name("s1".into()),
                        Arg::Name("l1".into()),
                        Arg::Name("g".into()),
                    ],
                ),
                (
                    "Apo".to_string(),
                    vec![Arg::Name("s2".into()), Arg::Name("l2".into())],
                ),
                (
                    "Ppo".to_string(),
                    vec![Arg::Name("s3".into()), Arg::Name("l3".into())],
                ),
            ],
            true,
        );
        let groups = packed_groups(&p);
        assert_eq!(
            groups.len(),
            2,
            "six same-source chains: one 4-group + one 2-group"
        );
        assert_eq!(groups[0].0, 4);
        assert_eq!(groups[1].0, 2);
    }
}
