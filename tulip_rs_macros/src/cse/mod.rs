//! `request!` — the JIT's pipeline, emitting Rust instead of CLIF.
//!
//! ```text
//! tulip_rs_macros::request! {
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
                            "request!: args are names or numeric literals",
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

struct RequestInput {
    patterns: Vec<PatternIn>,
}

impl Parse for RequestInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut patterns = Vec::new();
        while !input.is_empty() {
            let p = input.parse::<PatternIn>()?;
            let _semi: Token![;] = input.parse()?;
            patterns.push(p);
        }
        Ok(RequestInput { patterns })
    }
}

/// Entry body for [`crate::request`] (the root forwards here — proc-macro
/// attributes must sit at the crate root).
pub(crate) fn request_impl(input: TokenStream) -> TokenStream {
    let req = parse_macro_input!(input as RequestInput);
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
}
