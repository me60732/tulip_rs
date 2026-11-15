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

/// Emit ONE lane group over the first `n` slots of a bucket and return the
/// remainder. Only the LEADER carries the statement; every lane is registered in
/// `slot_group`, which is what the renderers read to derive the group's WIDTH —
/// so this is the single place a group's lane set is decided, for either
/// chunking policy.
fn emit_lane_group(
    c: &mut Classified,
    slots: &[usize],
    n: usize,
    src: usize,
    group_idx: usize,
) -> Vec<usize> {
    debug_assert!(n >= 2 && slots.len() >= n, "lane group needs >= 2 slots");
    let (head, rest) = slots.split_at(n);
    if std::env::var("CSE_FUSED_DEBUG").is_ok() {
        eprintln!(
            "classify_packed created {}-lane group {} with slots {:?}",
            n, group_idx, head
        );
    }
    c.groups.push((head[0], head[1], src));
    c.slot_group.insert(head[0], (group_idx, false));
    for &s in &head[1..] {
        c.slot_group.insert(s, (group_idx, true));
    }
    rest.to_vec()
}

/// Packed classify: same-source chains become lane groups, chunked 4-then-2 with
/// a remainder-1 scalar, for every packable form. Ema/Wilders/Iir pack over any
/// shared source node (price Input or a derived node like `tr`); `Sma` packs over
/// price Inputs only. The bucket-width alternative was built and measured out —
/// see the chunking comment below for the numbers and the single mechanism behind
/// both failures.
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
        // CHUNKING: 4-then-2, remainder 1 scalar — for EVERY form, including
        // chains. Bucket-sized groups were BUILT, measured, and REJECTED here on
        // 2026-11-15, so this line is a measured result, not a missed
        // simplification. What survived the experiment is the renderer: the
        // emitted arms are now width-generic (`Simd::<f64, N>` with an N-entry
        // slot list), so re-enabling a wide group is a chunker change only.
        //
        // WHY IT LOST — the same mechanism in both failing shapes: moving
        // operands across the vector boundary per bar, not instruction count.
        //   * SUM groups (`Sma`): the step is `g + (splat(price) - from_array
        //     [one leaving load PER LANE])`, so one 6-wide vector assembles from
        //     six independent memory operands every bar. Measured +21.3%
        //     (`sma_six_p` 13.69 → 16.60 µs, scalar twin +0.2%).
        //   * CHAIN groups WITH AN INTRA-BAR CONSUMER (`Ema` where a lane feeds a
        //     derived chain stepped in the same bar): `ema_six_p`'s arena has TWO
        //     group statements and TWO `to_array()` scatters (macd's line is built
        //     from group lanes, then the signal EMA consumes it), and one 6-wide
        //     group measured 13.08 µs vs 10.99 µs with the ladder = **+16.0%** for
        //     the wide form. Its own scalar twin was 11.03/10.96 — unchanged —
        //     while every control row sat inside ±2%, so the cause is the width.
        //   * The control case proves it is not width in general: `ema_chains_p`
        //     (six lanes, sinks only, ONE group statement and ONE scatter) was
        //     12.18 vs 12.28 µs — nothing. Same box, same passes.
        //
        // So the AVX-512 argument (8 lanes = 3 vector ops instead of 6, one
        // `zmm` step instead of two `ymm`) is real in the instruction stream and
        // still unmeasured here — but it cannot outweigh a loss of this size on a
        // shape we already ship, and it is unverifiable without the hardware.
        // The ceiling finding stands regardless: `Simd<f64, N>` accepts non-power
        // -of-two N and hard-errors above 64 elements (`E0080: … has more elements
        // than the limit 64`), which our own `MAX = 64` node cap already bounds.
        while slots.len() >= 4 {
            slots = emit_lane_group(&mut c, &slots, 4, src, group_idx);
            group_idx += 1;
        }
        if slots.len() >= 2 {
            slots = emit_lane_group(&mut c, &slots, 2, src, group_idx);
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
