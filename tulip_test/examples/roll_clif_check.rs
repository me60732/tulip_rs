//! E9 verification of the roll emitter, read off the EMITTED CLIF.
//!
//! Why this exists rather than a parity test: a redundant rescan is INVISIBLE to
//! bit-exactness — rescanning is idempotent for both value and trail (`trail =
//! look_back − idx` recomputes the same age), so an engine that walked every bar
//! would still match the hand perfectly. Only the shape of the emitted code shows
//! what a bar actually pays.
//!
//! What it reports per plan:
//!   · the planner's seed links (`roll_seeds()`),
//!   · every block's f64-load count. A per-bar body can read at most ONE price
//!     per stream, so `loads > n_streams` is unambiguously a walk. A walk
//!     SHORTER than that — a 1-slot gap — cannot be told apart from a merge or
//!     body by load count alone, so for those the evidence is the link list plus
//!     the emitter's own `for k in 0..(look_back − lb_n)` bound, not this table, and
//!   · the count of identity-constant float arithmetic, which is the only way to
//!     see that a state value is `Mirror`ed rather than re-derived by an
//!     `Ema{period: 1}` (the two print the same numbers, so no parity test can
//!     separate them — see the control row).
//!
//! Run: `cargo run -p tulip_test --example roll_clif_check`.

use std::collections::BTreeMap;

use tulip_rs::cse::{JitPlan, Planable, Recipe, Request, Sink};
use tulip_rs::indicators::aroon::Aroon;
use tulip_rs::indicators::cmo::Cmo;
use tulip_rs::indicators::donchianchannel::DonchianChannel;
use tulip_rs::indicators::ichimoku::Ichimoku;
use tulip_rs::indicators::max::Max;
use tulip_rs::indicators::rsi::Rsi;
use tulip_rs::indicators::willr::Willr;

/// (block label, f64 loads emitted into it), in CLIF order.
fn loads_per_block(clif: &str) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for line in clif.lines() {
        let t = line.trim();
        if t.starts_with("block") {
            out.push((t.split('(').next().unwrap_or(t).to_string(), 0));
        } else if let Some((_, n)) = out.last_mut() {
            if t.starts_with("vload") || t.contains("load.f64") {
                *n += 1;
            }
        }
    }
    out
}

/// Identity-constant float arithmetic: `fmul …, 1.0` and `fma …, 0.0, …`.
///
/// This is the fingerprint of an `Ema { period: 1 }` recurrence — α = 1 gives
/// `mult = 1/1 = 1.0` and `inv = 0/1 = 0.0`, so the EMA that "just returns its
/// input" still emits BOTH instructions per bar. A `Mirror` emits neither, and
/// because α = 1 makes the two numerically identical, no bit-exactness test can
/// tell them apart. Cranelift binds a float immediate to an SSA value and echoes
/// it in a trailing comment at each use:
///
/// ```text
/// v50 = fmul v47, v48       ; v48 = 0x1.0000000000000p0
/// v51 = fma  v8, v49, v50   ; v49 = 0.0
/// ```
fn identity_arith(clif: &str) -> usize {
    clif.lines()
        .filter(|l| match l.trim_start().split_once("= ") {
            Some((_, rhs)) => {
                (rhs.starts_with("fmul") && rhs.contains("= 0x1.0000000000000p0"))
                    || (rhs.starts_with("fma ") && rhs.contains("= 0.0"))
            }
            None => false,
        })
        .count()
}

fn report(label: &str, plan: &JitPlan) {
    let blocks = loads_per_block(plan.clif());
    let ns = plan.n_streams();
    println!("\n{label}");
    println!("  (ops, stores) = {:?}   streams = {ns}", plan.info(),);
    println!(
        "  seed links (wide op ← narrow seed op): {:?}",
        plan.roll_seeds()
    );
    let mut walks: Vec<(String, usize)> = Vec::new();
    let mut small: BTreeMap<usize, usize> = BTreeMap::new();
    for (name, n) in &blocks {
        if name == "block0" || *n == 0 {
            continue;
        }
        if *n > ns {
            walks.push((name.clone(), *n));
        } else {
            *small.entry(*n).or_default() += 1;
        }
    }
    println!(
        "  UNAMBIGUOUS walk blocks (> {ns} loads): {:?}",
        if walks.is_empty() {
            "none".to_string()
        } else {
            walks
                .iter()
                .map(|(b, n)| format!("{b}:{n}"))
                .collect::<Vec<_>>()
                .join(" ")
        }
    );
    println!(
        "  blocks with 1..={ns} loads (bodies + merges + any 1-slot gap walk): {:?}",
        small.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>()
    );
    // E11: the vector-walk mode must be VISIBLE, not assumed — `vany_true` only
    // exists in the chunked walk, so a zero here means the flag did not take.
    let clif = plan.clif();
    let nvec = clif.matches("vany_true").count();
    let nmax = clif.matches("fmax").count() + clif.matches("fmin").count();
    println!("  vector-walk evidence: vany_true × {nvec}, fmax/fmin × {nmax}");
    println!(
        "  identity-constant fmul/fma (`Ema{{period: 1}}` = 2 per chain): × {}",
        identity_arith(clif)
    );
    // E13: which GUARD form is in the build. On x64 a float `select` becomes a
    // JUMP (`XmmCmove`), while `bitselect` is a bitwise blend — so this says
    // whether the blend default is in effect (and that `CSE_BRANCHSELECT=1`
    // restored the jump), printing in EVERY mode so a zero can never be read as
    // "no guards" instead of "mode off". The splat count says whether the mask
    // was built by a vector `fcmp` (the shipped form) or crossed from a GPR.
    let nsel = clif
        .lines()
        .filter(|l| {
            l.trim_start()
                .split_once("= ")
                .map(|(_, r)| r.starts_with("select "))
                .unwrap_or(false)
        })
        .count();
    let nblend = clif
        .lines()
        .filter(|l| {
            l.trim_start()
                .split_once("= ")
                .map(|(_, r)| r.starts_with("bitselect "))
                .unwrap_or(false)
        })
        .count();
    let nsplat = clif
        .lines()
        .filter(|l| {
            l.trim_start()
                .split_once("= ")
                .map(|(_, r)| r.starts_with("splat"))
                .unwrap_or(false)
        })
        .count();
    println!(
        "  guard form: float `select` (jump) × {nsel}, `bitselect` (blend) × {nblend}, splatted (vector form) × {nsplat}"
    );
}

fn main() {
    // donchian(20) + max(50): rolls over the SAME high/low columns with
    // look_back 19 and 49, so the wide roll's walk is the 30-slot gap.
    report(
        "=== donchian(20) + max(50) — narrow(19) + wide(49) on the same columns ===",
        &Request::new()
            .add(DonchianChannel::recipe(&[20.0]), None)
            .add(Max::recipe(&[50.0]), None)
            .build(),
    );

    // aroon(14) + willr(14): same columns, DIFFERENT windows — aroon's rolls are
    // (period 14, look_back 14) = 15 bars, willr's are (14, 13) = 14 bars.
    // Interning gives nothing here; the chain gives the whole −30.7%.
    report(
        "=== aroon(14) + willr(14) — the row that flipped 1.12× loss → 1.30× win ===",
        &Request::new()
            .add(Aroon::recipe(&[14.0]), None)
            .add_all(Willr::recipe(&[14.0]))
            .build(),
    );

    // The densest case: three canonical windows per column (short 9, medium 26,
    // ultra 52), so each column chains twice and a rescan walks 9, 17 or 26
    // slots instead of 9, 26 or 52.
    report(
        "=== ichimoku(9, 26) — 6 rolls, 3 nested windows per column ===",
        &Request::new()
            .add(Ichimoku::recipe(&[9.0, 26.0]), None)
            .build(),
    );

    // Four SEPARATE indicator requests that merely happen to want overlapping
    // windows on the same columns (bench `nested/jit_4_requests_*`). This is the
    // cross-indicator case the chain exists for: none of these periods match, so
    // interning shares NOTHING between the requests, while the gap link chains
    // them into one walk per column per direction.
    report(
        "=== willr(14) + donchian(20) + max(50) + ichimoku(9,26) — 4 requests, one chain ===",
        &Request::new()
            .add(Willr::recipe(&[14.0]), None)
            .add(DonchianChannel::recipe(&[20.0]), None)
            .add(Max::recipe(&[50.0]), None)
            .add(Ichimoku::recipe(&[9.0, 26.0]), None)
            .build(),
    );

    // The momentum pair. Their two `Wilders`/two `RollSum` chains are real
    // recurrences, but each also reports hand-state prices (`rsi::prev_real`,
    // cmo's `prev`/`drop_real`) through `Mirror` nodes — copies whose last value
    // lands in a state word. Copies are numerically identical to the
    // `Ema{period: 1}` recurrences they replaced, so parity tests cannot see the
    // difference; the identity-constant count can, and it must read ZERO here.
    report(
        "=== rsi(14) + cmo(14) — mirrored state, no identity recurrence ===",
        &Request::new()
            .add(Rsi::recipe(&[14.0]), None)
            .add(Cmo::recipe(&[14.0]), None)
            .build(),
    );

    // POSITIVE CONTROL for the identity-constant matcher. A `Mirror` and an
    // `Ema{period: 1}` print the same numbers, so the zero above is only worth
    // something if this detector fires on the shape it replaced — and this is
    // literally that shape: a 1-period EMA over a price, feeding one sink.
    let mut ctl = Recipe::new();
    let price = ctl.add(tulip_rs::cse::Node::Input { stream: 0 });
    let ident = ctl.add(tulip_rs::cse::Node::Ema {
        src: price,
        period: 1,
    });
    ctl.kind = "identity-ema-control";
    ctl.outputs.push(Sink::iface(ident));
    report(
        "=== control: one `Ema{period: 1}` — the node rsi/cmo used to carry ===",
        &Request::new().add(ctl, None).build(),
    );
}
