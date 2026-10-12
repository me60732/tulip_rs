//! The template registry: `request!` compiles the patterns (CSE at macro
//! expansion, straight-line steady loops) + dispatch. A miss is `None` —
//! tier-2/3 territory by design, never a runtime compile.

use crate::cse::merge::merge;
use crate::cse::query::{Request, RowRef};
use crate::cse::signature::{erase, Signature, SinkOwner, Values};

// Generate the fused pattern registry at module level.
tulip_rs_macros::request! {
    // The classic sharing graph: apo+ppo+macd on one (s,l) pair, signal g.
    apo_ppo_macd_shared_sl: Apo(s, l), Ppo(s, l), Macd(s, l, g);
    //apo_ppo_macd_shared_sl_p: packed Apo(s, l), Ppo(s, l), Macd(s, l, g);
    // apo + ppo over the same pair.
    apo_ppo_shared_sl: Apo(s, l), Ppo(s, l);
    //apo_ppo_shared_sl_p: packed Apo(s, l), Ppo(s, l);
    // ATR family: atr = wilders(tr(p)), natr = (atr/close)*100 sharing tr+wilders
    atr_natr_shared_p: Atr(p), Natr(p);
    //natr_supertrend_shared_p: packed Atr(p), Natr(p);
    // NATR + SuperTrend: BOTH share the mean `Wilders(tr)` ATR node → dedup to one chain
    natr_supertrend_shared_p: Natr(p), Supertrend(p, 3.0);
    //natr_supertrend_shared_p_p: packed Natr(p), Supertrend(p, 3.0);

    // The 6-distinct-EMA shape: Macd(12,24,9), Apo(10,20), Ppo(6,14) erases to
    // SIX price-sourced Ema chains (no dedup) + 1 chained signal Ema + glue.
    ema_six: Macd(s1, l1, g), Apo(s2, l2), Ppo(s3, l3);
    ema_six_p: packed Macd(s1, l1, g), Apo(s2, l2), Ppo(s3, l3);

    // Rolling family, price-sourced: the stddev's optional sma mirror
    // interns onto the Sma node (one recurrence, two rows per request AND
    // across the two requests at a shared period).
    sma_stddev_shared_p: Sma(p), Stddev(p);
    // The two moving averages co-requested on one column: the standalone
    // Sma row interns onto the very Sma node wma mirrors (one arena node,
    // two sinks) — the hand pair pays two full passes over the series.
    sma_wma_shared_p: Sma(p), Wma(p);
    // Bollinger bands PAIRED with WMA on one period (the engine is a MERGE
    // engine — single-indicator shapes are the hand's home turf): one
    // `Sma(p)` node serves the bbands middle, the wma mirror, AND wma's
    // interned optional — three sinks, one recurrence, one pass.
    bbands_wma_shared_p: Bbands(p, m), Wma(p);

    // The 6-distinct-SMA shape (six periods, one column) — the rolling
    // analogue of `ema_six`: no dedup between the chains, and the packed
    // twin runs them as `Simd<f64,4>` + `Simd<f64,2>` lane groups.
    sma_six: Sma(p1), Sma(p2), Sma(p3), Sma(p4), Sma(p5), Sma(p6);
    sma_six_p: packed Sma(p1), Sma(p2), Sma(p3), Sma(p4), Sma(p5), Sma(p6);

    // Six distinct EMA CHAINS on one close series — the PURE multi-options
    // shape (rival: `Ema::indicator_by_options::<6>`); the packed twin
    // steps six recurrences as `Simd<f64,4>` + `Simd<f64,2>`.
    ema_chains: Ema(p1), Ema(p2), Ema(p3), Ema(p4), Ema(p5), Ema(p6);
    ema_chains_p: packed Ema(p1), Ema(p2), Ema(p3), Ema(p4), Ema(p5), Ema(p6);

    // Three ATRs at DIFFERENT periods on one (h,l,c): the tr subgraph
    // interns to ONE node, the three mean-`Wilders(tr)` chains are lanes of
    // one formula over a shared DERIVED source — the packing candidate the
    // ROADMAP named (rival: `Atr::indicator_by_options::<3>`).
    atr_multi: Atr(p1), Atr(p2), Atr(p3);
    atr_multi_p: packed Atr(p1), Atr(p2), Atr(p3);

    // ── trail-rescan hub ─────────────────────────────────────────────────
    // Max+Min on one period (over the SAME column they are the two faces of
    // one range; the anti-correlation rule only says their RESCANS don't
    // fuse into one walk — the arena still shares every upstream node).
    max_min_shared_p: Max(p), Min(p);
    // The aroon showcase: aroonosc costs ONE extra fsub + store on the
    // interned graph (the hand runs a second full pass over both columns).
    aroon_osc_shared_p: Aroon(p), AroonOsc(p);
    // willr + donchian: the same canonical (p, p−1) roll pair, four output
    // rows off two rolls, one Add/Abs/Div glue set.
    willr_donchian_shared_p: Willr(p), DonchianChannel(p);
    // The densest hub: six rolls (two columns × three windows), four
    // combines, one pass.
    ichimoku_s_m: Ichimoku(s, m);
}

/// Result rows in arena-sink order + the regrouping map.
///
/// A disabled optional sink produces a zero-length `RowRef` (`is_empty() == true`);
/// it never reads and its store is guarded by `sink_want`. The contract mirrors the
/// hand's `Vec::new()` rows for disabled optionals.
pub struct Outcome {
    pub rows: Vec<RowRef>,
    pub owners: Vec<SinkOwner>,
    /// Erased-shape hash of the pattern that served the request (telemetry).
    pub fp: u64,
}

/// Merge a request, apply its per-optional flags to the resolved sinks (the
/// JIT's `build` does the same; `merge` itself defaults all-on), and erase.
pub fn erase_request(
    req: &Request,
) -> Option<(crate::cse::Merged, Signature, Values, Vec<SinkOwner>)> {
    let (arena, mut resolved) = merge(&req.recipes);
    for (r, flags) in resolved.iter_mut().zip(req.opt_flags.iter()) {
        if flags.len() == r.optional_enabled.len() {
            r.optional_enabled = flags.clone();
        }
    }
    let leads = arena.leads();
    // erase now RECORDS optional flags as want bits instead of filtering sinks
    let (sig, vals, owners) = erase(&arena, &resolved, &leads)?;
    Some((arena, sig, vals, owners))
}

/// Attempt to serve `req` from the generated registry. `None` = no pattern
/// matches (unsupported nodes, unregistered shape, > cap sizes).
pub fn run_registered(req: &Request, streams: &[&[f64]]) -> Option<Outcome> {
    let (_arena, sig, vals, owners) = erase_request(req)?;
    let rows = try_run(&sig, &vals, streams)?;
    Some(Outcome {
        rows,
        owners,
        fp: sig.hash(),
    })
}

/// Result of an **asset-lane** request: one plan, L assets, per-asset
/// [`Outcome`]s (the [`run_registered`] shape stretched across N assets).
pub struct AssetsOutcome {
    /// Lane `a`'s fused result — same owned-row type as the scalar path
    /// because the lane kernel produces the same buffers, L-wide.
    pub per_asset: Vec<Outcome>,
}

/// Run a [`Request`] through the **lane kernels**: one `(Signature, Values)`
/// plan drives all `L` assets inside a single generated function.
/// `assets[a]` is that asset's stream tuple — the same layout
/// `Request::new` takes. Returns `None` if no registered pattern matches,
/// exactly like [`run_registered`].
///
/// This is the `indicator_by_assets` shape with the planner's dedup kept:
/// L assets × M indicators that share recurrences run as one fused, SIMD
/// pass — which per-indicator SIMD cannot do (it dedups nothing across
/// indicators). Resume/`get_states` is M1.
pub fn run_assets_registered<const L: usize>(
    req: &Request,
    assets: &[&[&[f64]]; L],
) -> Option<AssetsOutcome> {
    let (_arena, sig, vals, owners) = erase_request(req)?;
    let per_asset = try_run_assets::<L>(&sig, &vals, assets)?;
    let fp = sig.hash();
    Some(AssetsOutcome {
        per_asset: per_asset
            .into_iter()
            .map(|rows| Outcome {
                rows,
                owners: owners.clone(),
                fp,
            })
            .collect(),
    })
}
