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
    Fma,
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
            Kind::Fma => 14,
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
}

impl ANode {
    fn new_simple(kind: Kind, a: usize, b: usize) -> Self {
        Self {
            kind,
            a,
            b,
            c: None,
        }
    }

    fn new_three(kind: Kind, a: usize, b: usize, c: usize) -> Self {
        Self {
            kind,
            a,
            b,
            c: Some(c),
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
    /// Parallel to `sinks`: true = a recipe PRIMARY output (always stored),
    /// false = a declared OPTIONAL sink (store gated at runtime by
    /// `Values.sink_want`). Mirrors `Resolved` order: primaries then
    /// optionals, per recipe in request order.
    pub sink_is_primary: Vec<bool>,
    /// If true, same-source Ema chains run as SIMD lane groups (4-wide
    /// then 2-wide, remainder scalar). Chained EMAs and Wilders stay scalar.
    pub packed: bool,
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
            Node::Wilders { src, .. } => ANode::new_simple(Kind::Wilders, src as usize, 0),
            Node::Sub { a, b } => ANode::new_simple(Kind::Sub, a as usize, b as usize),
            Node::Add { a, b } => ANode::new_simple(Kind::Add, a as usize, b as usize),
            Node::Mul { a, b } => ANode::new_simple(Kind::Mul, a as usize, b as usize),
            Node::Div { a, b } => ANode::new_simple(Kind::Div, a as usize, b as usize),
            Node::Max { a, b } => ANode::new_simple(Kind::Max, a as usize, b as usize),
            Node::Abs { src } => ANode::new_simple(Kind::Abs, src as usize, 0),
            Node::Lag { src, .. } => ANode::new_simple(Kind::Lag, src as usize, 0),
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
            Node::Fma { a, b, c } => {
                ANode::new_three(Kind::Fma, a as usize, b as usize, c as usize)
            }
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

    Pattern {
        name: name.to_string(),
        nodes,
        names,
        sinks,
        sink_is_primary,
        packed,
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
