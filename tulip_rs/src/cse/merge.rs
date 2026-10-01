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
/// `Abs/Lag/Sqrt{src}`, `Sma/Stddev/Wma/Hma{src,period}`, `Wilders{..}`,
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
        Sqrt { src } => Sqrt { src: g(src) },
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
        | Lag { src, .. }
        | Hma { src, .. }
        | Wma { src, .. }
        | Sma { src, .. }
        | Stddev { src, .. }
        | RollMax { src, .. }
        | RollMin { src, .. }
        | Trail { src }
        | Wilders { src, .. }
        | SuperSmoother { src, .. }
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
        Fma { a, b, c } => {
            visit(*a);
            visit(*b);
            visit(*c);
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
