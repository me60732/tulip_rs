//! Perf-a compile-size / register-pressure canary (review §3.7 "pin CLIF size per
//! canonical workload"). Deterministic for a fixed cranelift version + target, so
//! it's a *blowup tripwire*: if a refactor quietly grows the fused loop (params,
//! stores, CLIF lines), these bounds fail and force a deliberate re-check. It also
//! records the measured relationship behind the fusion story: the fused-loop win
//! decays as live-xmm (packed chains) rises, which is why the lever is lane
//! packing + host-threading, not micro param removal (see Handover / perf notes).

use tulip_rs::cse::{Node, Planable, Request};
use tulip_rs::indicators::apo::Apo;
use tulip_rs::indicators::ema::Ema;
use tulip_rs::indicators::macd::Macd;
use tulip_rs::indicators::ppo::Ppo;

fn chains_and_lanes(p: &tulip_rs::cse::JitPlan) -> (usize, usize) {
    let chains = p
        .arena()
        .nodes
        .iter()
        .filter(|n| {
            matches!(
                n,
                Node::Ema { .. }
                    | Node::Wilders { .. }
                    | Node::Sma { .. }
                    | Node::Wma { .. }
                    | Node::Stddev { .. }
            )
        })
        .count();
    (chains, p.lanes().iter().sum::<usize>())
}

#[test]
fn classic_compile_size_is_bounded() {
    let p = Request::new()
        .add(Apo::recipe(&[12.0, 26.0]), None)
        .add(Ppo::recipe(&[12.0, 26.0]), None)
        .add(Macd::recipe(&[12.0, 26.0, 9.0]), None)
        .build();
    let (ops, stores) = p.info();
    let (chains, packed) = chains_and_lanes(&p);
    let lines = p.clif().lines().count();
    println!("classic: chains={chains} packed={packed} ops={ops} stores={stores} clif={lines}");
    // CSE'd shape is 11 nodes / 3 chains / one fused pair group — these are the
    // §4-identity invariants; if DCE/interning regress, op or store counts move.
    // `Ppo`'s divisor guard (`Max` + its `ε` `Const`) is not emitted when prices
    // are declared trustworthy, so the pinned counts are feature-keyed — the same
    // pair lives in `cse/mod.rs` as `CLASSIC_OPS`/`DENSE_NODES` for the unit
    // tests (an integration test can't reach crate-private items, hence the
    // duplication; change both together).
    assert_eq!(chains, 3, "ema12, ema26, macd signal");
    assert_eq!(
        ops,
        if cfg!(feature = "safe_inputs") { 8 } else { 10 },
        "classic fused op count (DCE-primed canary)"
    );
    assert!(lines < 400, "classic CLIF blew past its canary: {lines}");
}

#[test]
fn dense_dashboard_compile_size_is_bounded() {
    let p = Request::new()
        .add(Apo::recipe(&[5.0, 8.0]), None)
        .add(Apo::recipe(&[6.0, 9.0]), None)
        .add(Ppo::recipe(&[9.0, 25.0]), None)
        .add(Ppo::recipe(&[10.0, 26.0]), None)
        .add(Macd::recipe(&[11.0, 23.0, 17.0]), None)
        .add(Macd::recipe(&[12.0, 24.0, 9.0]), None)
        .add(Ema::recipe(&[200.0]), None)
        .build();
    let (ops, stores) = p.info();
    let (chains, packed) = chains_and_lanes(&p);
    let lines = p.clif().lines().count();
    println!("dense+1: chains={chains} packed={packed} ops={ops} stores={stores} clif={lines}");
    // 13 chains packed into ≤12 lanes (MAX_LANES=2) + a lone 200-ema; ~10 rows.
    assert!(chains >= 13, "dense has 13+ EMA chains, got {chains}");
    assert!(
        packed <= 2 * (chains.div_ceil(2)),
        "lane width exceeded MAX_LANES=2 packing"
    );
    assert!(lines < 1400, "dense CLIF blew past its canary: {lines}");
}
