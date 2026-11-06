use super::*;

pub(super) struct Classified {
    pub(super) groups: Vec<(usize, usize, usize)>, // (k0, k1, stream) pair groups
    pub(super) slot_group: HashMap<usize, (usize, bool)>, // slot -> (group, is_lane1)
    pub(super) scalars: HashMap<usize, Scalar>,    // Ema slots kept scalar
}

pub(super) enum Scalar {
    Price(usize),
    Chain(usize),
}
#[cfg(debug_assertions)]
impl std::fmt::Debug for Scalar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scalar::Price(s) => write!(f, "Price({})", s),
            Scalar::Chain(a) => write!(f, "Chain({})", a),
        }
    }
}

pub(super) fn classify(p: &Pattern) -> Classified {
    let mut c = Classified {
        groups: vec![],
        slot_group: HashMap::new(),
        scalars: HashMap::new(),
    };
    let mut k = 0;
    while k + 1 < p.nodes.len() {
        let (x, y) = (&p.nodes[k], &p.nodes[k + 1]);
        if x.kind == Kind::Ema
            && y.kind == Kind::Ema
            && p.plan.is_price_src(k)
            && p.plan.is_price_src(k + 1)
            && x.a == y.a
        {
            c.groups.push((k, k + 1, x.a)); // SOURCE SLOT id, as in classify_packed
            c.slot_group.insert(k, (c.groups.len() - 1, false));
            c.slot_group.insert(k + 1, (c.groups.len() - 1, true));
            k += 2;
            continue;
        }
        k += 1;
    }
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::Ema && !c.slot_group.contains_key(&k) {
            c.scalars.insert(
                k,
                if p.plan.is_price_src(k) {
                    Scalar::Price(p.nodes[node.a].a)
                } else {
                    Scalar::Chain(node.a)
                },
            );
        }
        // SeededEma is never groupable (excluded from `chain_step_form`):
        // the fold SOURCE is `node.b`, not `node.a` (`node.a` is the SEED,
        // only read at the one seed bar) -- a shared-source lane group keyed
        // on `node.a` would bucket unrelated seeds together. Always a scalar
        // `Chain(node.b)`: the fold source is a derived node (`tr`), never a
        // literal price stream, for every registered use today.
        if node.kind == Kind::SeededEma {
            c.scalars.insert(k, Scalar::Chain(node.b));
        }
    }
    c
}

/// Packed classify: group same-SOURCE chains into 4/2 lane groups — Ema
/// and Wilders over any shared source node (price Input or a derived node
/// like `tr`), Sma over price Inputs only. Remainder single slots stay
/// scalar (the measured rule: +6.6% packed on `sma_six`, per-shape).
pub(super) fn classify_packed(p: &Pattern) -> Classified {
    let mut c = Classified {
        groups: vec![],
        slot_group: HashMap::new(),
        scalars: HashMap::new(),
    };

    // Bucket by (kind, SOURCE SLOT). Ema/Wilders pack over any source node
    // (price Input or a shared derived node like `tr` — lanes of one
    // formula reading ONE src value, e.g. atr_multi's three Wilders over
    // the interned tr node); Sma packs only price-sourced (its step reads
    // the stream `t − p` back, derived sources would need the host anchor).
    let mut buckets: std::collections::HashMap<(Kind, usize), Vec<usize>> =
        std::collections::HashMap::new();
    for (k, node) in p.nodes.iter().enumerate() {
        let packable = match node.kind {
            k if chain_step_form(k) => true,
            Kind::Sma => p.plan.is_price_src(k),
            _ => false,
        };
        if packable {
            // Buckets stay per-KIND even though the LAW above is per-form. The reason
            // is NOT what commit 15b238a's message claimed ("the scalar fallback is
            // written for `Kind::Ema` only" -- false: there is a second, near-identical
            // `Kind::Wilders` fallback loop below this function, which is why
            // `atr_multi_p`'s leftover third Wilders survives today).
            //
            // The real blocker is the ARRANGEMENT. `Ema` lowers to
            // `carry*invms + src*mults`; `Wilders` lowers to the SWAPPED
            // `carry*mults + src*invms`. That is why the group renderer carries TWO
            // statements -- one emitting `#g.mul_add(#gi_v, splat(src) * #gm)` and one
            // emitting `#g.mul_add(#gm, splat(src) * #gi_v)`. A group has exactly ONE
            // statement, so an Ema lane and a Wilders lane over one source cannot share
            // it. Cross-form buckets therefore need the GROUP HOIST to normalize each
            // lane into one arrangement -- carry-coefficient into `#gi_v`,
            // source-coefficient into `#gm`, per lane, chosen from that lane's kind --
            // which is a hoist change, not a bucket-key change.
            //
            // Separately: `Iir` has NO fallback loop at all (only Ema and Wilders do),
            // so making it groupable before its fallback exists would let an unmatched
            // `Iir` fall through with neither a group nor a scalar hoist.
            buckets.entry((node.kind, node.a)).or_default().push(k);
        }
    }

    // DEBUG: print bucket contents for packed patterns
    if std::env::var("CSE_FUSED_DEBUG").is_ok() && p.packed {
        eprintln!("classify_packed buckets:");
        for (key, slots) in &buckets {
            eprintln!("  bucket {:?}: {:?}", key, slots);
        }
        eprintln!(
            "  total EMA slots: {}",
            buckets.values().map(|v| v.len()).sum::<usize>()
        );
    }

    // Process buckets in kind-then-stream order, chunk into 4/2 groups
    let mut bucket_keys: Vec<(Kind, usize)> = buckets.keys().copied().collect();
    bucket_keys.sort_by_key(|(kind, stream)| (*kind as u8, *stream));

    let mut group_idx = 0usize;
    for key in bucket_keys {
        let src = key.1;
        let mut slots = buckets.remove(&key).unwrap_or_default();
        if slots.is_empty() {
            continue;
        }
        // DEBUG: print slots before chunking
        if std::env::var("CSE_FUSED_DEBUG").is_ok() && p.packed {
            eprintln!(
                "classify_packed stream {} slots before chunking: {:?}",
                src, slots
            );
        }
        // Greedy chunking: prefer 4, then 2, remainder scalar
        while slots.len() >= 4 {
            let k0 = slots[0];
            let k1 = slots[1];
            let k2 = slots[2];
            let k3 = slots[3];
            if std::env::var("CSE_FUSED_DEBUG").is_ok() && p.packed {
                eprintln!(
                    "classify_packed created 4-lane group {} with slots [{},{},{},{}]",
                    group_idx, k0, k1, k2, k3
                );
            }
            c.groups.push((k0, k1, src));
            // All 4 slots are part of this group; only k0 triggers the statement
            c.slot_group.insert(k0, (group_idx, false));
            c.slot_group.insert(k1, (group_idx, true));
            c.slot_group.insert(k2, (group_idx, true));
            c.slot_group.insert(k3, (group_idx, true));
            slots = slots[4..].to_vec();
            group_idx += 1;
        }
        while slots.len() >= 2 {
            let k0 = slots[0];
            let k1 = slots[1];
            if std::env::var("CSE_FUSED_DEBUG").is_ok() && p.packed {
                eprintln!(
                    "classify_packed created 2-lane group {} with slots [{},{}]",
                    group_idx, k0, k1
                );
            }
            c.groups.push((k0, k1, src));
            c.slot_group.insert(k0, (group_idx, false));
            c.slot_group.insert(k1, (group_idx, true));
            slots = slots[2..].to_vec();
            group_idx += 1;
        }
        // Remaining single slots go to scalars (Ema chains only — a
        // leftover Sma slot takes its scalar ROLL path from `roll_hoists`,
        // never a chain scalar hoist).
        for k in &slots {
            if p.nodes[*k].kind == Kind::Ema {
                c.scalars.insert(
                    *k,
                    if p.plan.is_price_src(*k) {
                        Scalar::Price(p.nodes[p.nodes[*k].a].a)
                    } else {
                        Scalar::Chain(p.nodes[*k].a)
                    },
                );
            }
        }
    }

    // Add any remaining EMA nodes that weren't grouped (chained EMAs)
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::Ema && !c.slot_group.contains_key(&k) && !c.scalars.contains_key(&k) {
            c.scalars.insert(
                k,
                if p.plan.is_price_src(k) {
                    Scalar::Price(p.nodes[node.a].a)
                } else {
                    Scalar::Chain(node.a)
                },
            );
        }
    }

    // Add Wilders to scalars (never packed in this task)
    for (k, node) in p.nodes.iter().enumerate() {
        if node.kind == Kind::Wilders
            && !c.slot_group.contains_key(&k)
            && !c.scalars.contains_key(&k)
        {
            c.scalars.insert(
                k,
                if p.plan.is_price_src(k) {
                    Scalar::Price(p.nodes[node.a].a)
                } else {
                    Scalar::Chain(node.a)
                },
            );
        }
    }

    c
}
