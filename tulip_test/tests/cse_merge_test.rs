//! Merge/interning canaries: CSE dedup rules, lead math, sharing
//! groups, lane regions. Moved from `tulip_rs/src/cse/merge_tests.rs`.
//! The size canaries stay DEFINED in `tulip_rs` (`cse::{CLASSIC_NODES,
//! CLASSIC_OPS, DENSE_NODES}`) as plain constants (9/8/27).

#[cfg(test)]
mod tests {
    use tulip_rs::cse::Planable;
    use tulip_rs::cse::{merge, *};
    use tulip_rs::cse::{Id, Merged, Node, Recipe};
    use tulip_rs::indicators::adosc::Adosc;
    use tulip_rs::indicators::adx::Adx;
    use tulip_rs::indicators::apo::Apo;
    use tulip_rs::indicators::ema::Ema;
    use tulip_rs::indicators::macd::Macd;
    use tulip_rs::indicators::natr::Natr;
    use tulip_rs::indicators::ppo::Ppo;
    use tulip_rs::indicators::supertrend::SuperTrend;

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
            CLASSIC_NODES,
            "one shared graph, no duplicates"
        );
        let n = |p: fn(&Node) -> bool| m.nodes.iter().filter(|n| p(n)).count();
        assert_eq!(n(|x| matches!(x, Node::Ema { .. })), 3, "e12, e26, sig");
        assert_eq!(n(|x| matches!(x, Node::Sub { .. })), 2, "line + hist");
        // ppo's divisor guard (`Max` + `ε` Const) removed after safe_inputs feature removal.
        // Counted separately from the graph size so a lost node is still attributable.
        assert_eq!(
            n(|x| matches!(x, Node::Const { .. })),
            1,
            "only hundred Const (eps guard removed)"
        );
        assert_eq!(
            n(|x| matches!(x, Node::Max { .. })),
            0,
            "divisor guard Max removed"
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
        assert_eq!(m.nodes.len(), DENSE_NODES);
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
        use tulip_rs::indicators::cmo::Cmo;
        use tulip_rs::indicators::di::Di;
        use tulip_rs::indicators::rsi::Rsi;
        use tulip_rs::indicators::willr::Willr;
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
                    tulip_rs::indicators::wilders::Wilders::recipe(&[9.0]),
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
        use tulip_rs::indicators::wilders::Wilders;
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
        use tulip_rs::indicators::rsi::Rsi;
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
        use tulip_rs::indicators::willr::Willr;
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
        use tulip_rs::indicators::medprice::Medprice;
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

    /// Ring-rolls canary: AO's two mean `RollSum`s sit on ONE interned
    /// `medprice` node (same shape `recipes::medprice` builds — a co-request
    /// adds ZERO new arithmetic nodes), erase binds them to a single ring
    /// GROUP (leader = first roll slot, capacity = max period), and the
    /// publish lead is the LONG window's defined-from bar (33).
    #[test]
    fn ao_rings_intern_share_and_erase() {
        use tulip_rs::cse::signature::K_ROLLSUM;

        let (arena, resolved) = merge(&[recipes::ao(&[]), recipes::medprice(&[])]);
        assert_eq!(
            arena.nodes.len(),
            8,
            "ao = hi, lo, sum, half, med, 2 rolls, sub; medprice adds ZERO new \
             nodes when interned: {:?}",
            arena.nodes.len()
        );
        let rolls: Vec<usize> = (0..arena.nodes.len())
            .filter(|&k| matches!(arena.nodes[k], Node::RollSum { .. }))
            .collect();
        assert_eq!(rolls.len(), 2, "two ring rolls");
        let Node::RollSum { src: s0, .. } = arena.nodes[rolls[0]] else {
            unreachable!()
        };
        let Node::RollSum { src: s1, .. } = arena.nodes[rolls[1]] else {
            unreachable!()
        };
        assert_eq!(s0, s1, "one shared medprice src ⇒ one ring group");

        let leads = arena.leads();
        let (sig, vals, _o) = erase(&arena, &resolved, &leads).expect("ao erases");
        assert_eq!(sig.kinds[rolls[0]], K_ROLLSUM);
        assert_eq!(vals.ring_of[rolls[0]], rolls[0] as u32, "leader is first");
        assert_eq!(vals.ring_of[rolls[1]], rolls[0] as u32);
        assert_eq!(vals.ring_cap[rolls[0]], 34, "capacity = group max");
        assert_eq!(vals.mults[rolls[0]], 1.0 / 5.0);
        assert_eq!(vals.mults[rolls[1]], 1.0 / 34.0);
        assert_eq!(vals.feeds[rolls[0]], 0, "medprice is live from bar 0");
        assert_eq!(
            vals.sink_lead[0], 34,
            "ao publishes at the hand's LONG row head"
        );
    }

    /// THE medprice-sharing showcase (2026-10-13): AO and Fisher
    /// co-requested intern ONE medprice and ALL FOUR rolls (2 sums + 2
    /// extrema) ride ONE ring group — the arena where the old JIT re-ran a
    /// full medprice pass and lost ~3.5x. Node math: ao's 8 + fisher's 25
    /// minus the 5 shared (hi, lo, sum, half, med) = 28, exact.
    #[test]
    fn ao_fisher_intern_med_and_share_one_ring() {
        let (arena, resolved) = merge(&[recipes::ao(&[]), recipes::fisher(&[9.0])]);
        assert_eq!(
            arena.nodes.len(),
            28,
            "co-request must add ZERO medprice-arithmetic nodes: {}",
            arena.nodes.len()
        );
        let rolls: Vec<usize> = (0..arena.nodes.len())
            .filter(|&k| {
                matches!(
                    arena.nodes[k],
                    Node::RollSum { .. } | Node::RollMin { .. } | Node::RollMax { .. }
                )
            })
            .collect();
        assert_eq!(rolls.len(), 4, "2 sum rolls + 2 extremum rolls");
        let leads = arena.leads();
        let (_sig, vals, _o) = erase(&arena, &resolved, &leads).expect("ao+fisher erases");
        let leader = vals.ring_of[rolls[0]];
        assert_ne!(leader, u32::MAX, "the first roll heads the group");
        for &k in &rolls[1..] {
            assert_eq!(
                vals.ring_of[k], leader,
                "one ring group over the shared med"
            );
        }
        assert_eq!(vals.ring_cap[leader as usize], 34, "cap = group MAX period");
        // the extremum rolls' own window facts:
        for k in 0..arena.nodes.len() {
            if matches!(arena.nodes[k], Node::RollMin { .. } | Node::RollMax { .. }) {
                assert_eq!(vals.periods[k], 9);
                assert_eq!(vals.roll_lb[k], 8, "fisher window = (9, 8)");
                assert_eq!(vals.feeds[k], 0, "medprice lives from bar 0");
            }
        }
        // sinks: ao primary, 3 ao optionals, fisher fish, fisher signal.
        assert_eq!(vals.sink_lead[0], 34, "ao keeps its LONG head");
        assert_eq!(vals.sink_lead[4], 8, "fish lead = period-1");
        assert_eq!(vals.sink_lead[5], 8, "signal shares it");
    }

    /// The stream-index law: columns keep the hand's `Open, High, Low, Close,
    /// Volume` order and a subset just SHIFTS LEFT within it, so a caller binds
    /// the vectors straight out of `let [open, high, low, close] = *inputs`.
    ///
    /// This is what separates column interning from the old positional behavior:
    /// `Adosc + Apo` used to land apo's close on adosc's HIGH (both wrote
    /// `Input{0}`) with every registry and asset gate green — the parity suites
    /// cannot make this test pass for the wrong reason.
    #[test]
    fn streams_shift_left_in_the_hands_column_order() {
        let adosc = || Adosc::recipe(&[3.0, 10.0]);
        let apo = || Apo::recipe(&[12.0, 26.0]);
        // The arena's stream slots, ascending — one per distinct column.
        let streams = |m: &Merged| -> Vec<Id> {
            let mut v: Vec<Id> = m
                .nodes
                .iter()
                .filter_map(|n| match *n {
                    Node::Input { stream } => Some(stream),
                    _ => None,
                })
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        };

        // A recipe alone already sits in the shifted-left order.
        let (m, _) = merge(&[adosc()]);
        assert_eq!(streams(&m), vec![0, 1, 2, 3], "high, low, close, volume");
        let (m, _) = merge(&[apo()]);
        assert_eq!(streams(&m), vec![0], "close alone shifts to stream 0");

        // The cross-convention co-request: union is high/low/close/volume, so
        // apo's close must resolve to stream 2 — NOT 0, which is adosc's high.
        let (m, r) = merge(&[adosc(), apo()]);
        assert_eq!(
            streams(&m),
            vec![0, 1, 2, 3],
            "one slot per column, the shared close deduped"
        );
        let apo_close = match m.nodes[r[1].inputs[0] as usize] {
            Node::Ema { src, .. } => src,
            ref n => panic!("apo tail[0] is not an Ema: {n:?}"),
        };
        let bound = match m.nodes[apo_close as usize] {
            Node::Input { stream } => stream,
            ref n => panic!("apo's ema source is not an Input: {n:?}"),
        };
        assert_eq!(bound, 2, "apo reads CLOSE (2), not adosc's high (0)");

        // Request order moves nothing — rank is canonical, not first-seen. If it
        // were insertion order these arenas differ and the registry splits.
        let (m_rev, _) = merge(&[apo(), adosc()]);
        assert_eq!(streams(&m_rev), vec![0, 1, 2, 3]);
        assert_eq!(m_rev.nodes.len(), m.nodes.len(), "order-independent arena");
    }

    /// The graft path's DERIVE branch — the one that translates a child's slots
    /// from its declared COLUMNS when the caller names no index at all. It had
    /// never executed: every graft site in the tree passes explicit indices, so
    /// only the CHECK branch was covered and the translation the user asked for
    /// ("the planner need to translate the idx's") was untested code.
    ///
    /// The discriminating case is a price child (`[Close]`) grafted into a
    /// three-column parent (`[High, Low, Close]`). Correct derivation binds the
    /// child's slot 0 to parent slot **2**; an identity binding — or a
    /// positional one, which is the bug this whole mechanism replaces — puts it
    /// on parent slot 0, i.e. HIGH. So the test asserts equality with the right
    /// answer AND inequality with the positional answer: it cannot pass by both
    /// branches agreeing on a wrong index.
    #[test]
    fn graft_without_indices_derives_the_columns_not_the_positions() {
        // Derived: the caller names nothing, the planner translates.
        let mut derived = Recipe::hlc();
        let _g1 = derived.sub_graph(recipes::ema(&[9.0]), &[]);

        // The same graph spelled with the CORRECT explicit index (Close = 2).
        let mut hand_written = Recipe::hlc();
        let _g2 = hand_written.sub_graph(recipes::ema(&[9.0]), &[2]);

        assert_eq!(
            derived.nodes, hand_written.nodes,
            "derivation must reproduce the correct column binding exactly"
        );

        // The child's own EMA must read arena slot 2, not 0.
        let src = match derived
            .nodes
            .iter()
            .copied()
            .find(|n| matches!(n, Node::Ema { .. }))
            .expect("the grafted child contributes an Ema")
        {
            Node::Ema { src, .. } => src,
            _ => unreachable!(),
        };
        match derived.nodes[src as usize] {
            Node::Input { stream } => assert_eq!(stream, 2, "child reads Close"),
            ref n => panic!("ema source is not an Input: {n:?}"),
        }
    }

    /// When BOTH parent and child declare columns, binding a child's `Close` onto
    /// a parent slot that is `High` is **unconstructible**: `Recipe::bind_columns`
    /// asserts in its CHECK branch and panics, with the message quoted below.
    /// So a wrong arena cannot be built, interned, or compared — the column
    /// conflation is refused at authoring time, not discovered at parity time.
    /// This is why the derive test asserts equality against the CORRECT index
    /// only: there is no wrong-index graph to compare against.
    #[test]
    #[should_panic(
        expected = "positional binding onto the wrong price column (parent columns [High, Low, Close])"
    )]
    fn graft_onto_the_wrong_column_is_refused_at_authoring_time() {
        let mut p = Recipe::hlc();
        // `ema` declares [Close]; parent slot 0 is High. Positional thinking.
        let _g = p.sub_graph(recipes::ema(&[9.0]), &[0]);
    }
}
