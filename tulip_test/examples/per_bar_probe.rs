//! Read the fused loop's PER-BAR instruction mix, off the emitted CLIF.
//!
//! Why this exists: the standalone jit-vs-hand gap (rsi 1.72×, cmo 1.8×, vhf 1.7×
//! of hand) is NOT the roll rescan — `rsi` has no rescan at all and shows the same
//! gap. So the deficit lives in what every bar of the steady loop pays, and only
//! the emitted code shows that. Bit-exactness cannot: a redundant per-bar
//! instruction is invisible to a parity test (the same lesson as `roll_clif_check`,
//! one level down — there it was a redundant RESCAN, here it is redundant
//! BOOKKEEPING).
//!
//! Prints, per block: instruction count and the mix of loads / stores / float ops
//! / int ops / branches, and dumps the full CLIF to `/tmp/<plan>.clif` to be read
//! by hand.
//!
//! Run: `cargo run -p tulip_test --example per_bar_probe`

use tulip_rs::cse::{JitPlan, Planable, Request};
use tulip_rs::indicators::cmo::Cmo;
use tulip_rs::indicators::rsi::Rsi;
use tulip_rs::indicators::vhf::Vhf;

/// (label, instructions, loads, stores, float ops, int ops, branches)
fn block_stats(clif: &str) -> Vec<(String, usize, usize, usize, usize, usize, usize)> {
    let mut out: Vec<(String, usize, usize, usize, usize, usize, usize)> = Vec::new();
    for raw in clif.lines() {
        let line = raw.trim();
        if line.ends_with(':') && line.contains("block") {
            out.push((line.trim_end_matches(':').to_string(), 0, 0, 0, 0, 0, 0));
            continue;
        }
        let Some(last) = out.last_mut() else { continue };
        // An instruction line is `vNN = op ...`.
        let Some(eq) = line.find("= ") else { continue };
        let Some(body) = line.get(eq + 2..) else {
            continue;
        };
        let op = body.split([' ', '(']).next().unwrap_or("");
        if op.is_empty() {
            continue;
        }
        last.1 += 1;
        if op.starts_with("load") {
            last.2 += 1;
        } else if op.starts_with("store") {
            last.3 += 1;
        } else if op.starts_with('f') && !op.starts_with("fcvt") {
            last.4 += 1;
        } else if op.starts_with('i') && !op.starts_with("iconst") {
            last.5 += 1;
        }
        if matches!(op, "brif" | "jump" | "fallthrough") {
            last.6 += 1;
        }
    }
    out
}

fn report(name: &str, plan: &JitPlan) {
    let clif = plan.clif();
    let _ = std::fs::write(format!("/tmp/{name}.clif"), clif);
    let (ops, stores) = plan.info();
    println!(
        "\n=== {name}: ops={ops} stores={stores} clif_lines={} ===",
        clif.lines().count()
    );
    println!("block                  insts  ld  st  flt  int   br");
    for (label, n, ld, st, flt, iv, br) in block_stats(clif) {
        // Only the substantial blocks matter; the tiny ones are trampoline edges.
        if n < 4 {
            continue;
        }
        println!("{label:<22} {n:>5} {ld:>3} {st:>3} {flt:>4} {iv:>4} {br:>3}");
    }
}

fn main() {
    let p = 14.0f64;
    report("rsi", &Request::new().add(Rsi::recipe(&[p]), None).build());
    report("cmo", &Request::new().add(Cmo::recipe(&[p]), None).build());
    report("vhf", &Request::new().add(Vhf::recipe(&[p]), None).build());
    report(
        "rsi_plus_cmo",
        &Request::new()
            .add(Rsi::recipe(&[p]), None)
            .add(Cmo::recipe(&[p]), None)
            .build(),
    );
    println!("\nfull CLIF written to /tmp/<name>.clif");
}
