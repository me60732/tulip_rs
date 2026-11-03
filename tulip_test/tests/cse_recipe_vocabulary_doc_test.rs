//! The Node table in `tulip_rs_shared/src/cse/RECIPE_VOCABULARY.md` is the vocabulary an
//! author copies names from. Two ways it goes wrong, both seen for real:
//!
//! * a row naming a variant that does not exist (`Neg` was written twice — once by a
//!   delegated recipe author, once by me in the prose) — the author writes a name that
//!   never compiles, or worse, invents the node instead;
//! * a row set that has gone stale, so a live variant (`HighPass`, `SuperSmoother`,
//!   `HtInPhase`, `HtQuadrature`) is invisible and an author concludes the family is
//!   blocked when it is not.
//!
//! Checking the table against `pub enum Node` in both directions costs a file read and no
//! compilation, so it runs with the rest of the suite. It only compares NAMES: it cannot
//! tell you a description is wrong, which is why every claim in the doc still needs the
//! grep that produced it.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_file(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(rel)
}

fn read(rel: &str) -> String {
    let p = repo_file(rel);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
}

/// Variant names of `pub enum Node { ... }`, taken from lines at the enum's own brace
/// depth, so field names, doc comments and attributes never enter the set.
fn node_variants(src: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut inside = false;
    let mut depth = 0usize;
    for raw in src.lines() {
        let line = raw.trim();
        if !inside {
            if line.starts_with("pub enum Node {") {
                inside = true;
                depth = 1;
            }
            continue;
        }
        // Count braces only on code lines: a doc comment can carry `{ 0.0 }` inside a
        // prose quote of the hand's guard, and that is not structure.
        let comment_or_attr = line.starts_with("//") || line.starts_with('#');
        let opens = if comment_or_attr {
            0
        } else {
            line.matches('{').count()
        };
        let closes = if comment_or_attr {
            0
        } else {
            line.matches('}').count()
        };
        if depth == 1 && !comment_or_attr {
            let head: String = line
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if head.starts_with(|c: char| c.is_ascii_uppercase()) {
                out.insert(head);
            }
        }
        depth += opens;
        depth -= closes;
        if depth == 0 {
            return out;
        }
    }
    panic!("reached end of recipe.rs without closing `pub enum Node`");
}

/// Identifiers in the first column of §5's Node table, with `Name{fields}` reduced to
/// `Name`. Stops at the next `##` heading so §5's prose cannot feed the set.
fn documented_names(md: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut in_section = false;
    let mut in_table = false;
    for raw in md.lines() {
        let line = raw.trim();
        if line.starts_with("## ") {
            if in_section {
                break;
            }
            in_section = line.starts_with("## 5.");
            continue;
        }
        if !in_section {
            continue;
        }
        if line.starts_with("| Node |") {
            in_table = true;
            continue;
        }
        if !in_table {
            continue;
        }
        if !line.starts_with('|') {
            break;
        }
        if line
            .chars()
            .all(|c| c == '|' || c == '-' || c.is_whitespace())
        {
            continue;
        }
        let first = line.trim_start_matches('|').split('|').next().unwrap_or("");
        for tok in first.split_whitespace() {
            let name: String = tok
                .trim_matches('`')
                .split('{')
                .next()
                .unwrap_or("")
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if name.starts_with(|c: char| c.is_ascii_uppercase()) {
                out.insert(name);
            }
        }
    }
    assert!(!out.is_empty(), "found no rows in §5's Node table");
    out
}

const DOC: &str = "tulip_rs_shared/src/cse/RECIPE_VOCABULARY.md";
const RECIPE: &str = "tulip_rs_shared/src/cse/recipe.rs";
const REGISTRY: &str = "tulip_rs/src/cse/kernel/registry.rs";

/// Guard against a vacuous parse: if the enum reader silently returned an empty or wrong
/// set, both tests above would pass for the wrong reason. These are the three facts the
/// table was corrected against, asserted where they cannot be confused with the table.
#[test]
fn node_variant_reader_sees_the_enum_that_caught_the_errors() {
    let variants = node_variants(&read(RECIPE));
    assert!(
        variants.len() > 20,
        "parsed only {} variants",
        variants.len()
    );
    // `Neg` is the phantom this whole check exists for: negation is `Mul(x, -1.0)`.
    assert!(!variants.contains("Neg"), "recipe.rs grew a Node::Neg");
    // The four HT kinds were missing from the table entirely.
    for live in ["HighPass", "SuperSmoother", "HtInPhase", "HtQuadrature"] {
        assert!(variants.contains(live), "{live} vanished from the enum");
    }
}

#[test]
fn vocabulary_doc_names_only_real_node_variants() {
    let variants = node_variants(&read(RECIPE));
    let documented = documented_names(&read(DOC));
    let invented: Vec<&String> = documented
        .iter()
        .filter(|n| !variants.contains(*n))
        .collect();
    assert!(
        invented.is_empty(),
        "RECIPE_VOCABULARY.md §5 names {invented:?} which are not variants of `pub enum Node` \
         in recipe.rs — the table is the author's source of names, so a phantom row is worse \
         than no row"
    );
}

#[test]
fn vocabulary_doc_covers_every_node_variant() {
    let variants = node_variants(&read(RECIPE));
    let documented = documented_names(&read(DOC));
    let missing: Vec<&String> = variants
        .iter()
        .filter(|v| !documented.contains(*v))
        .collect();
    assert!(
        missing.is_empty(),
        "`pub enum Node` has variants {missing:?} that §5's table does not list — an omitted \
         node reads as a missing capability (the HT family was left out this way); add the row, \
         and if it cannot be erased yet, say so"
    );
}

/// What is registered must be gated. This is a MENTION floor, not a proof of parity:
/// it asks only whether each pattern in a `kernel!` row appears anywhere in the engine
/// gate files. It is deliberately weak because a precise check cannot be written — the
/// row token and the Rust type name are allowed to differ (`Stddev(p)` in a row versus
/// `struct StdDev`), so deriving `Stddev::indicator` from the row would fail on correct
/// code. A weak check that cannot false-fail is worth having, because the hole it found
/// was real: `bop_n: Bop();` was registered and reachable with `Bop` appearing nowhere
/// in any engine gate — every hit was in `bop_test.rs`, which is hand-vs-tulip-C. Read
/// §1 of the doc: a registered row without a gate is WORSE than an unregistered recipe,
/// because it can ship wrong values to a caller. Only the hand-parity test itself proves
/// a row correct; this test only proves it was not skipped entirely.
#[test]
fn every_registered_pattern_is_named_in_an_engine_gate() {
    let patterns = registered_patterns(&read(REGISTRY));
    assert!(
        patterns.len() > 40,
        "parsed only {} pattern names from registry.rs — the row scanner is broken, \
         not the registry",
        patterns.len()
    );

    let mut gated = String::new();
    for f in [
        "tulip_test/tests/cse_kernel_registry_test.rs",
        "tulip_test/tests/cse_merge_test.rs",
        "tulip_test/tests/cse_assets_test.rs",
        // B4a probe: a synthetic, non-indicator recipe with no hand oracle
        // (ENGINE_CHECKLIST.md). Gated against an independent reference
        // recurrence instead of a hand indicator — see the file's own doc.
        "tulip_test/tests/wilders_mixed_probe_test.rs",
    ] {
        gated.push_str(&read(f));
    }
    let gated = normalize(&gated);

    let ungated: Vec<&String> = patterns
        .iter()
        .filter(|p| !gated.contains(&normalize(p)))
        .collect();
    assert!(
        ungated.is_empty(),
        "registered patterns {ungated:?} are never named in an engine gate file — add the \
         bit-exact hand-parity test, or unregister the row"
    );
}

/// `pub enum Node` names live in registry.rs as `kernel!` rows: `name: A(p), B();`
/// possibly wrapped, with `packed` allowed and `///` comments interleaved. Returns the
/// pattern identifiers (the `A` in `A(p)`), deduplicated.
fn registered_patterns(src: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut in_row = false;
    for raw in src.lines() {
        let line = raw.trim_end();
        let t = line.trim_start();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        if !in_row {
            // a row begins at column 4 with `lowercase_name: `
            in_row = line.starts_with("    ")
                && t.contains(": ")
                && t[..t.find(':').unwrap()]
                    .chars()
                    .all(|c| c.is_lowercase() || c == '_' || c.is_ascii_digit())
                && !t.starts_with("let ")
                && !t.starts_with("pub ");
            if !in_row {
                continue;
            }
        }
        let chars: Vec<char> = t.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i].is_ascii_uppercase() {
                let mut j = i;
                while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
                let name: String = chars[i..j].iter().collect();
                let mut k = j;
                while k < chars.len() && chars[k] == ' ' {
                    k += 1;
                }
                if k < chars.len() && chars[k] == '(' {
                    out.insert(name);
                }
                i = j;
            } else {
                i += 1;
            }
        }
        if t.ends_with(';') {
            in_row = false;
        }
    }
    out
}

fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '_')
        .flat_map(|c| c.to_lowercase())
        .collect()
}
