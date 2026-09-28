//! The CSE merge pass: combine several indicator [`Recipe`]s into one arena and
//! deduplicate identical nodes by *canonical structural key* — the planner's
//! common-subexpression elimination (`EXECUTION_PLANNER.md` §5.2).
//!
//! # The one hazard
//! [`Node`] stores **child references** and **plain params** (period, `Const`
//! bits, input stream) under the *same* [`Id`] type. That makes the recipes
//! terse, but it means the merge MUST know which `Id` fields are references —
//! those get remapped local→global; the rest are copied verbatim. See [`remap`].
//! When you add a [`Node`] variant with a new child, add it to `remap` or CSE
//! silently keys on a stale local index (the test below is the guard).
//!
//! # Why interning is order-independent
//! Every recipe is SSA-ordered (a node's inputs are lower indices), so when we
//! intern node `i` of a recipe, all of `i`'s children are *already* global — their
//! `local→global` entries are set. Thus two structurally-identical subtrees, no
//! matter which recipe or local indices they came from, rewrite to the *same*
//! globally-shaped `Node` and hit the same cache entry. That's exactly what makes
//! `Apo`'s `sub(ema12,ema26)` and `Macd`'s `macd_line` merge into one node — the
//! `Apo ≡ Macd.macd_line` identity (`MATHEMATICAL_ENGINE.md` §4) falls out of CSE
//! for free, with no special rewrite rule.

use crate::cse::{Id, Node, Recipe};
use std::collections::HashMap;

/// Merged, node-deduplicated graph. Child `Id`s in `nodes` are *global* arena
/// indices (not per-recipe SSA ids any more).
#[derive(Debug, Clone)]
pub struct Merged {
    pub nodes: Vec<Node>,
}

impl Merged {
    /// Canonical per-node `defined-from` lead: the earliest *absolute bar at which
    /// the node has a valid value* (NOT the hand `slot_lengths` report offset —
    /// that is the per-sink publish-lead, §5.3). Rules:
    /// - `Ema{src, period} = lead(src) + (period - 1)` (`ema.rs::init_state` seeds
    ///   bar 0 then advances `period - 1`).
    /// - arithmetic (`Sub/Mul/Div/Max`) = `max` of child leads.
    /// - `Input`/`Const` = 0.
    ///
    /// This is the *defined-from* half of §5.3; the *publish* half is
    /// `data_len - slot_lengths/output_length` per sink (the indicator's capacity),
    /// tracked as an open item (§10 decision U, §11). It is what makes
    /// `Apo`/`Ppo` (which report an EMA at bar `period`) coexist with `Macd`
    /// (which reports `macd_line` at `long - 1` and seeds `signal` from that
    /// early value). Right-aligned `to_bits` already pass on defined-from alone
    /// because EMA seed differences decay to sub-ulp; capacity-publish-lead will
    /// upgrade them to exact full-array alignment + turn on the chained test.
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
                // 7-tap kernel: needs `src` valid then 6 more taps (see ht_kernel ring buffer).
                Node::HtInPhase { src } | Node::HtQuadrature { src } => leads[src as usize] + 6,
            };
            leads.push(id);
        }
        leads
    }
}

/// Where one input recipe's outputs landed in the merged arena, plus the
/// per-sink publish lead (§5.3) so `exec`/`finalize` can emit arrays whose
/// length matches that indicator's own `slot_lengths`/`min_data`.
#[derive(Debug, Clone, Default)]
pub struct Resolved {
    /// Global ids of the request's primary outputs (`Info::outputs` order).
    pub outputs: Vec<Id>,
    /// Global ids of the request's optional outputs (`Info::optional_outputs` order).
    pub optional_outputs: Vec<Id>,
    /// Publish lead per primary output (parallel to `outputs`).
    pub output_leads: Vec<Id>,
    /// Publish lead per optional output (parallel to `optional_outputs`).
    pub optional_output_leads: Vec<Id>,
    /// Per-optional-output enable flags (parallel to `optional_outputs`);
    /// `merge` defaults all-`true`, `Request::add_opt` overrides per request.
    /// A disabled optional sink prints an EMPTY array in declared position
    /// (hand `::indicator(None)` parity) and is excluded from the plan's
    /// `max_publish_lead`. The producing nodes are still computed when they
    /// feed enabled sinks — flags prune emission, not (yet) liveness.
    pub optional_enabled: Vec<bool>,
    /// Arena node ids of this request's EMA nodes, in recipe order (== the
    /// concrete `State`'s field order), for `get_states` projection.
    pub ema_arena_ids: Vec<Id>,
}

/// Intern `node` (child ids already global) into `arena`, returning its global id.
/// Equal structure ⇒ same id ⇒ shared computation (the CSE key).
fn intern(arena: &mut Vec<Node>, cache: &mut HashMap<Node, Id>, node: Node) -> Id {
    if let Some(&existing) = cache.get(&node) {
        return existing;
    }
    let id = arena.len() as Id;
    arena.push(node.clone());
    cache.insert(node, id);
    id
}

/// Rewrite a recipe-local node into its globally-shaped form.
///
/// **Which fields are child refs (→ `l2g[...]`) vs params (→ copy):**
/// - `Input{stream}` · `Const{bits}` · every `period` — **params** (a value, not a ref).
/// - `Ema{src}` · `Sub/Mul/Div/Max{a,b}` · `HighPass{input}` · `SuperSmoother{src}` ·
///   `HtInPhase{src}` · `HtQuadrature{src}` — **refs** (translate through `l2g`).
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

/// Merge `recipes` into one arena with shared nodes deduplicated.
///
/// Returns the merged graph and, per input recipe (same order), where its
/// `outputs` / `optional_outputs` now live in the arena (plus the per-sink
/// publish leads copied through from the recipe — §5.3).
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
            outputs: recipe.outputs.iter().map(|&o| l2g[o as usize]).collect(),
            optional_outputs: recipe
                .optional_outputs
                .iter()
                .map(|&o| l2g[o as usize])
                .collect(),
            // leads are sink-local (bar counts), so they pass through unchanged.
            output_leads: recipe.output_leads.clone(),
            optional_output_leads: recipe.optional_output_leads.clone(),
            // Default: every declared optional emits (full `Results`).
            // `Request::add_opt` overrides per request before `build`.
            optional_enabled: vec![true; recipe.optional_outputs.len()],
            // EMA nodes in recipe order → their (interned) arena ids.
            ema_arena_ids: recipe
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| matches!(n, Node::Ema { .. }))
                .map(|(l, _)| l2g[l])
                .collect(),
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

    #[test]
    fn apo_macd_ppo_merge_dedups_shared_emas() {
        let apo = Apo::recipe(&[12.0, 26.0]); // 4 nodes: in, ema12, ema26, sub
        let macd = Macd::recipe(&[12.0, 26.0, 9.0]); // 6 nodes: in, ema12, ema26, sub, ema9, sub
        let ppo = Ppo::recipe(&[12.0, 26.0]); // 9 nodes: in, ema12, ema26, eps, max, sub, c100, mul, div
        let before = apo.nodes.len() + macd.nodes.len() + ppo.nodes.len();

        let (merged, resolved) = merge(&[apo, macd, ppo]);

        // 19 inputs collapse to 12 unique nodes.
        assert_eq!(before, 19);
        assert_eq!(merged.nodes.len(), 12);

        // The three shared Ema recurrences (ema12, ema26 across all 3; + macd's ema9
        // signal) → only 3 Ema nodes total, not 6.
        let n_ema = merged
            .nodes
            .iter()
            .filter(|n| matches!(n, Node::Ema { .. }))
            .count();
        assert_eq!(n_ema, 3);
        let n_sub = merged
            .nodes
            .iter()
            .filter(|n| matches!(n, Node::Sub { .. }))
            .count();
        assert_eq!(n_sub, 3); // apo/macd_line (shared), macd histogram, ppo diff

        // `Apo`'s value and `Macd`'s `macd_line` are literally the same global node.
        assert_eq!(resolved[0].outputs[0], resolved[1].outputs[0]);

        // (PPO's `diff` Sub is the third Sub counted above: its right operand is
        //  the clamped `max` node, not raw ema26, so CSE correctly keeps it a
        //  distinct node rather than merging it with apo/macd's `sub`.)

        // every resolved id points inside the arena
        for r in &resolved {
            for &id in r.outputs.iter().chain(r.optional_outputs.iter()) {
                assert!((id as usize) < merged.nodes.len());
            }
        }
    }
}
