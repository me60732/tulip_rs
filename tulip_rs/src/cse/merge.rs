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

use crate::cse::{Id, Node, Recipe, Sink, SinkSrc};
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
                | Node::Mul { a, b }
                | Node::Div { a, b }
                | Node::Max { a, b } => leads[a as usize].max(leads[b as usize]),
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

/// Where one input recipe landed in the merged arena: its tail wiring with
/// all ids resolved to arena globals.
#[derive(Debug, Clone)]
pub struct Resolved {
    /// Tail selector (from `Recipe::kind`).
    pub kind: &'static str,
    /// Options forwarded to the tail constructor.
    pub options: Vec<f64>,
    /// Global arena ids fed to the tail, in argument order. The request's
    /// tail starts running at `max(leads[inputs])` — the first bar every
    /// argument is reported.
    pub inputs: Vec<Id>,
    /// Primary sinks (global ids / tail slots), `Info::outputs` order.
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
        Mul { a, b } => Mul {
            a: l2g[a as usize],
            b: l2g[b as usize],
        },
        Div { a, b } => Div {
            a: l2g[a as usize],
            b: l2g[b as usize],
        },
        Max { a, b } => Max {
            a: l2g[a as usize],
            b: l2g[b as usize],
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
            SinkSrc::Tail(i) => SinkSrc::Tail(i),
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

    /// The three classic requests share every recurrence: `ema12`/`ema26`
    /// CSE to one arena node each, so the whole interface is 3 nodes
    /// (`Input` + 2 `Ema`) — the signal EMA is macd's *tail* state, not an
    /// interface node, and `Apo ≡ macd_line` is one tail subtraction each.
    #[test]
    fn merge_dedups_interface_to_three_nodes() {
        let (m, r) = merge(&[
            Apo::recipe(&[12.0, 26.0]),
            Ppo::recipe(&[12.0, 26.0]),
            Macd::recipe(&[12.0, 26.0, 9.0]),
        ]);
        assert_eq!(m.nodes.len(), 3, "Input + ema12 + ema26, shared");
        let n_ema = m
            .nodes
            .iter()
            .filter(|n| matches!(n, Node::Ema { .. }))
            .count();
        assert_eq!(n_ema, 2);

        // every request's tail reads the same two global ids
        for res in &r {
            assert_eq!(res.inputs.len(), 2);
        }
        assert_eq!(r[0].inputs, r[1].inputs);
        assert_eq!(r[1].inputs, r[2].inputs);

        // defined-from leads: ema12 → 11, ema26 → 25 (Input = 0)
        let leads = m.leads();
        assert_eq!(leads[r[0].inputs[0] as usize], 11);
        assert_eq!(leads[r[0].inputs[1] as usize], 25);

        // passthrough optionals are Interface mirrors of those same nodes
        assert!(matches!(
            r[0].optional_outputs[0].src,
            SinkSrc::Interface(id) if id == r[0].inputs[0]
        ));
        assert!(matches!(
            r[2].optional_outputs[1].src,
            SinkSrc::Interface(id) if id == r[2].inputs[1]
        ));
    }

    /// The dense dashboard's PARTIAL overlap: `Apo(6,9)`'s long EMA and
    /// `Ppo(9,25)`'s short EMA are both `ema9(close)` — one structural key,
    /// so the arena holds exactly ONE `Ema{period: 9}` node and the two
    /// requests receive the same global id (one advance per bar; the JIT
    /// emits one chain, both tails read it). All other periods in the set
    /// are distinct, so the interface is `Input` + 11 EMAs — not 12.
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
                .filter(|(_, n)| matches!(**n, Node::Ema { period, .. } if period == p))
                .map(|(i, _)| i as Id)
                .collect()
        };
        assert_eq!(
            m.nodes.len(),
            12,
            "Input + 11 distinct close EMAs (periods 5,6,8,9,10,11,12,23,24,25,26)"
        );
        assert_eq!(ema(9), vec![ema(9)[0]], "ema9 must exist exactly once");
        // Apo(6,9) = request 1: tail inputs [ema6, ema9];
        // Ppo(9,25) = request 2: tail inputs [ema9, ema25].
        assert_eq!(
            r[1].inputs[1], r[2].inputs[0],
            "Apo long(9) and Ppo short(9) must be the SAME arena node"
        );
        // and one Input for all six recipes:
        assert!(matches!(m.nodes[0], Node::Input { stream: 0 }));
        assert!(
            m.nodes[1..].iter().all(|n| matches!(
                n,
                Node::Ema { src, .. } if *src == 0
            )),
            "every EMA in this query reads the single close Input"
        );
    }
}
