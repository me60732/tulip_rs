//! `kernel!` — the JIT's pipeline, emitting Rust instead of CLIF.
//!
//! ```text
//! tulip_rs_macros::kernel! {
//!     apo_ppo_macd_shared_sl: Apo(s, l), Ppo(s, l), Macd(s, l, g);
//!     apo_ppo_shared_sl: Apo(s, l), Ppo(s, l);
//!     atr_natr_shared_p: Atr(p), Natr(p);
//!     natr_supertrend_shared_p: Natr(p), Supertrend(p, 3.0);
//! }
//! ```
//!
//! expands (at compile time) to, per pattern:
//! * `pub const SIG_<name>: Signature` — the erased-shape const the runtime
//!   `erase_request` output is matched against (structural equality, same
//!   mechanism as the hand-written M0 consts — but generated, no drift),
//! * `pub fn run_<name>(&Values, &[&[f64]]) -> Vec<Vec<f64>>` — a
//!   straight-line steady loop: one op per arena slot, named locals,
//!   `Simd<f64,2>` pairing of role-aligned price chains (the pass LLVM SLP
//!   declined), runtime params read from `Values` and hoisted, zero
//!   dispatch, zero `match`,
//! * `pub fn try_run(...)` — the dispatch match.
//!
//! Warm-up bars run through the tier-2 interpreter's `exec::<SIG, true>`
//! (shared code, cold path, once per request); the steady loop is the
//! generated specialization. Params remain fully runtime (rev-2 law): same
//! name across recipes ⇒ same chain; the generated code reads
//! `vals.mults[k]`/`vals.sink_lead[s]`, never a baked number. Two patterns
//! whose arenas coincide collide loudly at expansion (unreachable match
//! arm), which is the intended way to discover duplicate shapes.

use proc_macro::TokenStream;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, Lit, Token,
};

mod codegen;
mod planner;

use planner::{Arg, Pattern};

struct RecipeIn {
    id: Ident,
    args: Vec<Arg>,
}

impl Parse for RecipeIn {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let id: Ident = input.parse()?;
        let content;
        syn::parenthesized!(content in input);
        let mut args = Vec::new();
        while !content.is_empty() {
            if content.peek(Lit) {
                match content.parse::<Lit>()? {
                    Lit::Float(f) => args.push(Arg::Lit(f.base10_parse()?)),
                    Lit::Int(i) => args.push(Arg::Lit(i.base10_parse::<u32>()? as f64)),
                    other => {
                        return Err(syn::Error::new(
                            other.span(),
                            "kernel!: args are names or numeric literals",
                        ))
                    }
                }
            } else {
                let n: Ident = content.parse()?;
                args.push(Arg::Name(n.to_string()));
            }
            if !content.is_empty() {
                let _: Token![,] = content.parse()?;
            }
        }
        Ok(RecipeIn { id, args })
    }
}

struct PatternIn {
    name: Ident,
    packed: bool,
    recipes: Vec<RecipeIn>,
}

impl Parse for PatternIn {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let _: Token![:] = input.parse()?;
        // Optional `packed` keyword before recipe list - check first token without consuming
        let packed = if input.peek(Ident) {
            // Get the next token as a string to check if it's "packed"
            let ident_str: String = input
                .fork()
                .parse::<Ident>()
                .ok()
                .map(|i| i.to_string())
                .unwrap_or_default();
            if ident_str == "packed" {
                // Consume it now from the main stream
                let _: Ident = input.parse()?;
                true
            } else {
                false
            }
        } else {
            false
        };
        let mut recipes = vec![input.parse::<RecipeIn>()?];
        while input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
            if input.is_empty() || input.peek(Token![;]) {
                break;
            }
            recipes.push(input.parse::<RecipeIn>()?);
        }
        Ok(PatternIn {
            name,
            packed,
            recipes,
        })
    }
}

struct KernelInput {
    patterns: Vec<PatternIn>,
}

impl Parse for KernelInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut patterns = Vec::new();
        while !input.is_empty() {
            let p = input.parse::<PatternIn>()?;
            let _semi: Token![;] = input.parse()?;
            patterns.push(p);
        }
        Ok(KernelInput { patterns })
    }
}

/// Entry body for [`crate::kernel`] (the root forwards here — proc-macro
/// attributes must sit at the crate root).
pub(crate) fn kernel_impl(input: TokenStream) -> TokenStream {
    let req = parse_macro_input!(input as KernelInput);
    let mut patterns: Vec<Pattern> = Vec::new();
    for p in &req.patterns {
        let recipes: Vec<(String, Vec<Arg>)> = p
            .recipes
            .iter()
            .map(|r| (r.id.to_string(), r.args.clone()))
            .collect();
        patterns.push(planner::expand(&p.name.to_string(), &recipes, p.packed));
    }
    let out = codegen::expand(&patterns);
    out.into()
}

#[cfg(test)]
mod tests {
    use super::planner::Kind;
    use super::*;

    fn build(recipes: &[(&str, Vec<Arg>)], packed: bool) -> Pattern {
        let r: Vec<(String, Vec<Arg>)> = recipes
            .iter()
            .map(|(n, a)| (n.to_string(), a.clone()))
            .collect();
        planner::expand("t", &r, packed)
    }

    fn n(s: &str) -> Arg {
        Arg::Name(s.into())
    }
    fn l(v: f64) -> Arg {
        Arg::Lit(v)
    }

    #[test]
    fn apo_ppo_macd_arena_mirrors_the_real_merge() {
        // dump-verified runtime arena with optional outputs: kinds [1,3,3,5,2,7,8,3,5];
        // a [0,0,0,1,0,3,5,3,3]; b [0,0,0,2,0,4,2,0,7]; sinks [d,es,el,q,es,el,d,sig,hist,es,el]
        let p = build(
            &[
                ("Apo", vec![n("s"), n("l")]),
                ("Ppo", vec![n("s"), n("l")]),
                ("Macd", vec![n("s"), n("l"), n("g")]),
            ],
            false,
        );
        assert_eq!(p.nodes.len(), 9);
        let kinds: Vec<u8> = p.nodes.iter().map(|x| x.kind.tag()).collect();
        assert_eq!(kinds, vec![1, 3, 3, 5, 2, 7, 8, 3, 5]);
        let a: Vec<usize> = p.nodes.iter().map(|x| x.a).collect();
        assert_eq!(a, vec![0, 0, 0, 1, 0, 3, 5, 3, 3]);
        let b: Vec<usize> = p.nodes.iter().map(|x| x.b).collect();
        assert_eq!(b, vec![0, 0, 0, 2, 0, 4, 2, 0, 7]);
        // Sinks ride the Resolved list: every DECLARED optional is present
        // (enable/disable is the runtime want-bit, never an arena difference).
        // Apo[d,es,el], Ppo[q,es,el], Macd[d,sig,hist,es,el] → [3,1,2,6,1,2,3,7,8,1,2]
        assert_eq!(p.sinks, vec![3, 1, 2, 6, 1, 2, 3, 7, 8, 1, 2]);
        // Verify names are present
        let names: Vec<Option<&str>> = p.names.iter().map(|o| o.as_deref()).collect();
        println!("Names: {:?}", names);
        assert!(names.len() == 9, "names should have same length as nodes");
        assert!(p.nodes.iter().all(|x| x.kind != Kind::Input || x.a == 0));
    }

    #[test]
    fn literal_equality_shares_like_names() {
        let p = build(
            &[
                ("Apo", vec![l(12.0), l(26.0)]),
                ("Ppo", vec![l(12.0), l(26.0)]),
            ],
            false,
        );
        assert_eq!(p.nodes.len(), 7, "shared pair");
        let q = build(
            &[
                ("Apo", vec![l(12.0), l(26.0)]),
                ("Ppo", vec![l(13.0), l(27.0)]),
            ],
            false,
        );
        assert_eq!(q.nodes.len(), 10, "distinct periods -> distinct chains");
    }

    /// CMO on the REAL merged arena: its gain and loss sums are independent
    /// columns with equal ages, so the planner puts them in ONE fused
    /// container — and the emitted kernel must actually build and feed it. A
    /// parity run cannot tell a working fold from an empty bundle list, so this
    /// pins the firing, not just the safety.
    #[test]
    fn cmo_fuses_its_gain_and_loss_sums() {
        let p = build(&[("Cmo", vec![l(9.0)])], false);
        assert_eq!(p.bundles.len(), 1, "one fused container");
        let b = &p.bundles[0];
        assert_eq!(b.groups.len(), 2, "two sum groups share it");
        assert_eq!(b.members.len(), 2, "one member each: ages = members");
        assert_eq!(
            b.call_site(),
            *b.members.iter().min().unwrap(),
            "the fused call sits at the first member"
        );
        for &m in &b.members {
            assert!(
                p.nodes[m].a < b.call_site(),
                "every bundled source ({}) precedes the fused call site ({})",
                p.nodes[m].a,
                b.call_site()
            );
        }
        let out = codegen::expand(&[p]).to_string();
        assert!(out.contains("BundleRings"), "the container is emitted");
        assert!(out.contains("bu0"), "and it is the one fed");
    }

    /// ULTOSC is the WIDE bundle: two independent pure-sum groups of THREE
    /// members each (`bp` and `tr` at short/medium/long), so the fused call
    /// carries six ages for two series — one push and ONE cursor advance for
    /// all six folds. cmo proved B=2/N=1; this is the shape that tests the
    /// `B × N` read cost the container choice is paid for, and the hand itself
    /// answers with `MultiBuffer<2>` (`ultosc.rs:9`).
    #[test]
    fn ultosc_bundles_two_groups_of_three_members() {
        let p = build(&[("Ultosc", vec![l(7.0), l(14.0), l(28.0)])], false);
        assert_eq!(p.bundles.len(), 1, "bp and tr share one container");
        let b = &p.bundles[0];
        assert_eq!(b.groups.len(), 2, "two series");
        assert_eq!(b.members.len(), 6, "three windows per series");
        assert_eq!(b.call_site(), *b.members.iter().min().unwrap());
        for &g in &b.groups {
            assert_ne!(b.series_of(g), None, "group {g} has a series");
        }
        for &m in &b.members {
            assert_ne!(b.rank_of(m), None, "member {m} has a rank");
            assert!(
                p.nodes[m].a < b.call_site(),
                "member {m}'s source {} must precede the fused call site {}",
                p.nodes[m].a,
                b.call_site()
            );
        }
        // The long member's age equals the container capacity, so its
        // pre-push read IS the eviction — no cap flag anywhere in the call.
        let out = codegen::expand(&[p]).to_string();
        assert!(out.contains("BundleRings"), "the fused container is built");
        assert!(out.contains("bu0"), "and named by the plan");
        assert!(
            !out.contains("feed_periods"),
            "a bundled arena emits no per-group fold at all"
        );
    }

    /// The hub's packed twin builds NO lane group, and that is the measured state
    /// of this shape, not an oversight: `classify_packed` buckets by
    /// `(kind, source)` and every `Wilders` in the hub now has its own source —
    /// `+DM`/`−DM` over their own mutual-exclusion nodes, the mean ATR over `tr`,
    /// the ADX smoothing over `dx`. Nothing shares a source, so there is no
    /// fan-out to pack. The hub's 4.6x is pure CSE DEDUP.
    ///
    /// RETRACTION this test now encodes: an earlier revision asserted the OPPOSITE
    /// (`!groups.is_empty()`), because `di`'s COMPOSITE ATR-sum and `adx`'s MEAN
    /// ATR were both `Wilders` over the same interned `tr`. That pair was not a
    /// capability — it was the emitter defect (the mean slot's published row came
    /// out as the raw SUM, measured at 15.2→40.4 where the hand prints ~6.1). With
    /// `di` moved onto the shared mean chain the shape is gone. If this test ever
    /// FAILS because a group appeared, that means a same-source mixed-form pair is
    /// back and the defect is live again — check the published row against the hand
    /// before believing the group is a win.
    #[test]
    fn hub_packed_twin_builds_no_lane_group() {
        let hub = || {
            vec![
                ("Dm", vec![l(14.0)]),
                ("Di", vec![l(14.0)]),
                ("Dx", vec![l(14.0)]),
                ("Adx", vec![l(14.0)]),
            ]
        };
        let packed = codegen::expand(&[build(&hub(), true)]).to_string();
        let groups: Vec<String> = (0..8)
            .map(|i| format!("g{i}_m"))
            .filter(|g| packed.contains(g.as_str()))
            .collect();
        assert!(
            groups.is_empty(),
            "packed hub built lane groups {groups:?} — a same-source Wilders pair is \
             back, which is the shape the emitter mishandles (see this test's doc)",
        );
    }

    /// CAN AN AD-LINE EMA SHARE A LANE GROUP WITH A PRICE EMA? Not today, and this
    /// is the gate that says so. `classify_packed` buckets packables by
    /// `(Kind, node.a)` — the SOURCE SLOT — and the group step multiplies the
    /// carry by `Simd::splat(src)`, i.e. one broadcast source read serves every
    /// lane. So lanes are N recurrences over ONE source, never N sources over one
    /// recurrence.
    ///
    /// The test uses the shape where the question bites: adosc AND apo AND ppo on
    /// the SAME (12,26), so the arena holds four `Ema` chains — two over the
    /// volume-driven `Accum` line, two over the interned close (apo's and ppo's
    /// pair interning to one). Two buckets of two → exactly two 2-wide groups and
    /// zero 4-wide, which is what the assertions below pin.
    ///
    /// WHY SOURCE-KEYED IS KEPT — MEASURED, not assumed. An `xpacked` arm WAS built
    /// (Ema buckets keyed on form alone; the group step reading a per-lane
    /// `from_array` of each source local instead of one splat) as a third row over the
    /// registry trio arena `Adosc(s,l), Apo(a1,b1), Ppo(a2,b2)` — SIX `Ema` chains
    /// across two sources (four close, two AD line). `CSE_FUSED_DEBUG`'s own bucket
    /// print showed the arms differ as predicted: `packed` gave (Ema,close) =
    /// [15,16,18,19] and (Ema,AD) = [12,13]; `xpacked` merged them into one bucket and
    /// emitted a 4-wide group [12,13,15,16] mixing the two sources. Gated byte-exact
    /// against the scalar kernel, then timed in ONE binary:
    ///   scalar 12.515 us | packed 12.031 us | xpacked 12.798 us | hand 17.712 us
    /// so cross-source cost +6.4% over same-source packing AND +2.3% over NOT packing
    /// at all. The `xpacked` arm was reverted; the SOURCE-KEYED twin was kept and
    /// re-measured as a WIN (scalar 12.582 -> `packed` 11.873 us, +6.0%; hand 17.768),
    /// and is registered as `adosc_apo_ppo_p`. This test pins the status quo on a
    /// FOUR-chain variant (apo and ppo sharing the same 12/26 literals, so their pairs
    /// intern to one close chain) — the mechanism is the same, the numbers are not.
    ///
    /// Why it loses: `xpacked` halves the GROUP count (two vector steps to one) but
    /// must first assemble `<4 x f64>` from two different locals every bar, and that
    /// per-bar operand movement costs more than the halved step. Width is not the
    /// limit — this emitter is rustc/LLVM on AVX2, so 256-bit is genuinely available,
    /// unlike the cranelift path whose 128-bit SSA ceiling could have excused its own
    /// loss (§6.34: classic -22%, dense -11%). Two backends, one conclusion: packing
    /// pays only where it REPLACES what the lane-group mechanism cannot express, and a
    /// broadcast source is precisely what it expresses for free.
    ///
    /// Standing rule from the user (2026-10-17), which this measurement confirms:
    /// prioritize same-source grouping, because a splat is faster than reading from
    /// different sources — even where cross-source packing is expressible. Per
    /// `simd-packing-decided-at-merge-by-shared-depth` the decision belongs in the
    /// planner, never as a per-kind rule in `codegen.rs`, which is why the preference
    /// is stated here rather than enforced by a kind check in the emitter.
    #[test]
    fn ad_line_and_price_emas_do_not_share_a_lane_group() {
        let matched = || {
            vec![
                ("Adosc", vec![l(12.0), l(26.0)]),
                ("Apo", vec![l(12.0), l(26.0)]),
                ("Ppo", vec![l(12.0), l(26.0)]),
            ]
        };
        let scalar = build(&matched(), false);
        // The node-level half: matched periods intern apo's pair and ppo's pair to
        // ONE close chain, so four Ema slots total — two reading the Accum, two
        // reading the close Input. Different sources, so two buckets.
        let emas: Vec<usize> = scalar
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind == Kind::Ema)
            .map(|(k, _)| k)
            .collect();
        assert_eq!(
            emas.len(),
            4,
            "2 over the AD line + 1 shared close pair (x2)"
        );
        let srcs: Vec<usize> = emas.iter().map(|k| scalar.nodes[*k].a).collect();
        let mut uniq = srcs.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(
            uniq.len(),
            2,
            "the four Ema slots must span exactly two sources (AD line, close) — \
             got slots {emas:?} reading {srcs:?}"
        );
        for s in &uniq {
            assert_eq!(
                srcs.iter().filter(|x| *x == s).count(),
                2,
                "each source bucket holds two chains (the 12 and the 26)"
            );
        }

        // The packing half. `TokenStream::to_string()` separates EVERY token with a
        // space (`Simd :: from_array ([st [15usize] , ...])`), so raw source-text
        // probes silently match nothing — two earlier versions of this test read
        // meaningless zeros that way. Canonicalise by stripping whitespace FIRST,
        // then assert, and keep the positive control so a zero is a finding.
        let adosc = canon(codegen::expand(&[build(&matched(), true)]).to_string());
        // Positive control, built from its OWN expansion so a 4-wide here can only
        // come from it: Macd(12,24,9)+Apo(10,20)+Ppo(6,14) are six EMAs over the
        // ONE interned close Input, so the bucket is six deep and chunking must
        // emit a 4-wide group.
        let control = canon(
            codegen::expand(&[build(
                &[
                    ("Macd", vec![l(12.0), l(24.0), l(9.0)]),
                    ("Apo", vec![l(10.0), l(20.0)]),
                    ("Ppo", vec![l(6.0), l(14.0)]),
                ],
                true,
            )])
            .to_string(),
        );
        // SELF-TEST of the probe: the control's six same-source EMAs must show up
        // as a 4-wide group in the SAME canonical text the claim below reads. If
        // this fails, the probe is broken and the negative proves nothing. (An
        // earlier version OR'ed this with a `contains` so it could never fail —
        // a self-test that cannot fail is not a self-test.)
        assert!(
            control.contains("Simd::<f64,4>::from_array([st["),
            "the control shape shows no 4-wide group in canonical text, so this probe \
             cannot see groups at all"
        );

        // THE CLAIM: the adosc shape never merges the two buckets into one 4-wide
        // group. Each source's pair gets its own 2-wide step, so the AD line and
        // the price line are NOT lanes of one vector op.
        assert!(
            !adosc.contains("Simd::<f64,4>::from_array([st["),
            "a 4-wide group appeared for the adosc+apo+ppo arena — cross-source packing \
             exists now, so the claim this test pins is superseded"
        );
        let wide2 = adosc.matches("Simd::from_array([st[").count();
        assert_eq!(
            wide2, 2,
            "expected exactly two 2-wide lane groups (one per source bucket), found {wide2}"
        );
        // WHY cross-source packing is impossible today: the group step broadcasts
        // ONE source value to every lane, so a group is N recurrences over one
        // source by construction.
        assert!(
            adosc.contains("Simd::splat("),
            "expected the group step to splat a single source"
        );
    }

    /// Strip whitespace so generated-source probes match at all: a `TokenStream`
    /// rendered with `to_string()` separates EVERY token, `Simd :: from_array (…`.
    fn canon(src: String) -> String {
        src.chars().filter(|c| !c.is_whitespace()).collect()
    }

    fn packed_group_count(src: &str) -> usize {
        (0..32)
            .filter(|i| src.contains(&format!("g{}_m", i)))
            .count()
    }

    /// Phase-3 discovery gate: under CURRENT classifier rules, the candidate
    /// standalone rows remain vacuous for lane grouping (no `gN_m` declarations).
    ///
    /// This is the conservative precondition check before any `_p` row additions:
    /// if every candidate yields zero groups, Phase 3 proceeds via the no-op path.
    #[test]
    fn discovery_phase3_candidates_nonvacuous_gate() {
        let cases: Vec<(&str, Vec<(&str, Vec<Arg>)>)> = vec![
            ("donchianchannel", vec![("DonchianChannel", vec![l(20.0)])]),
            ("willr", vec![("Willr", vec![l(20.0)])]),
            ("aroon", vec![("Aroon", vec![l(20.0)])]),
            ("aroon_osc", vec![("AroonOsc", vec![l(20.0)])]),
            ("stochrsi", vec![("Stochrsi", vec![l(14.0)])]),
        ];

        for (name, recipes) in cases {
            let packed_src = canon(codegen::expand(&[build(&recipes, true)]).to_string());
            let groups = packed_group_count(&packed_src);
            assert_eq!(
                groups, 0,
                "{name}: expected no lane groups under current classifier; found {groups}"
            );
        }
    }

    /// STOCH on the real arena: %D rolls the SAME BAR's %K output, so no fused
    /// push can carry both — the plan must refuse, and no fused container may
    /// appear in the kernel. This is the shape the user pointed at in
    /// `stoch.rs`: two single buffers there are correct, not unoptimised.
    #[test]
    fn stoch_refuses_to_fuse_its_two_sums() {
        let p = build(&[("Stoch", vec![l(9.0), l(3.0), l(3.0)])], false);
        assert!(
            p.bundles.is_empty(),
            "d_line reads k_line's output this bar: separate containers"
        );
        let out = codegen::expand(&[p]).to_string();
        assert!(
            !out.contains("BundleRings"),
            "a refused arena emits no fused container"
        );
    }
}
