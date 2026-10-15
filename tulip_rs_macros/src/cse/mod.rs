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
