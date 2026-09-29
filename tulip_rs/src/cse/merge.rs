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
                Node::Abs { src } => leads[src as usize],
                // A delayed read of a stream first has a value once the
                // stream is `n` bars old.
                Node::Lag { src, n } => leads[src as usize] + n,
                Node::Sma { src, period }
                | Node::Stddev { src, period }
                | Node::Wma { src, period } => leads[src as usize] + period.saturating_sub(1),
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
                Node::Sqrt { src } => leads[src as usize],
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
    /// defaults all-`true`, `Request::add_opt` overrides per request.
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

/// Rewrite a recipe-local node into its globally-shaped form.
///
/// **Refs (→ `l2g[...]`):** `Ema{src}`, `Sub/Mul/Div/Max{a,b}`,
/// `HighPass{input}`, `SuperSmoother{src}`, `HtInPhase/HtQuadrature{src}`.
/// **Params (copy):** `Input{stream}`, `Const{bits}`, every `period`.
fn remap(node: &Node, l2g: &[Id]) -> Node {
    use Node::*;
    match *node {
        Input { stream } => Input { stream },
        Const { bits } => Const { bits },
        Ema { src, period } => Ema {
            src: l2g[src as usize],
            period,
        },
        Sub { a, b } => Sub {
            a: l2g[a as usize],
            b: l2g[b as usize],
        },
        Add { a, b } => Add {
            a: l2g[a as usize],
            b: l2g[b as usize],
        },
        Mul { a, b } => Mul {
            a: l2g[a as usize],
            b: l2g[b as usize],
        },
        Div { a, b } => Div {
            a: l2g[a as usize],
            b: l2g[b as usize],
        },
        WhereGe { a, b, t } => WhereGe {
            a: l2g[a as usize],
            b: l2g[b as usize],
            t: l2g[t as usize],
        },
        Abs { src } => Abs {
            src: l2g[src as usize],
        },
        Lag { src, n } => Lag {
            src: l2g[src as usize],
            n,
        },
        Max { a, b } => Max {
            a: l2g[a as usize],
            b: l2g[b as usize],
        },
        Hma { src, period } => Hma {
            src: l2g[src as usize],
            period,
        },
        Wilders {
            src,
            period,
            composite,
            feed,
        } => Wilders {
            src: l2g[src as usize],
            period,
            composite,
            feed,
        },
        Wma { src, period } => Wma {
            src: l2g[src as usize],
            period,
        },
        Sma { src, period } => Sma {
            src: l2g[src as usize],
            period,
        },
        Stddev { src, period } => Stddev {
            src: l2g[src as usize],
            period,
        },
        Sqrt { src } => Sqrt {
            src: l2g[src as usize],
        },
        Fma { a, b, c } => Fma {
            a: l2g[a as usize],
            b: l2g[b as usize],
            c: l2g[c as usize],
        },
        HighPass { input, period } => HighPass {
            input: l2g[input as usize],
            period,
        },
        SuperSmoother { src, period } => SuperSmoother {
            src: l2g[src as usize],
            period,
        },
        HtInPhase { src } => HtInPhase {
            src: l2g[src as usize],
        },
        HtQuadrature { src } => HtQuadrature {
            src: l2g[src as usize],
        },
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
    use crate::indicators::apo::Apo;
    use crate::indicators::macd::Macd;
    use crate::indicators::ppo::Ppo;

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
        assert_eq!(m.nodes.len(), 11, "one shared graph, no duplicates");
        let n = |p: fn(&Node) -> bool| m.nodes.iter().filter(|n| p(n)).count();
        assert_eq!(n(|x| matches!(x, Node::Ema { .. })), 3, "e12, e26, sig");
        assert_eq!(n(|x| matches!(x, Node::Sub { .. })), 2, "line + hist");
        assert_eq!(n(|x| matches!(x, Node::Const { .. })), 2, "eps, 100");
        assert_eq!(n(|x| matches!(x, Node::Max { .. })), 1);
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
        assert_eq!(m.nodes.len(), 30);
    }
}
