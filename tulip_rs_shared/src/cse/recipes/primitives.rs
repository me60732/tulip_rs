use crate::cse::{Node, Recipe, Sink};

/// `Ema(p)` over `Input{0}`: one chain node mirroring its own line.
pub fn ema(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let e = r.add(Node::Ema { src: close, period });
    r.kind = "ema";
    r.options = options.to_vec();
    r.tail_inputs = vec![e];
    // Publish lead = `period`: hand `min_data = p + 1`, `output_length =
    // len − p` — report starts one bar after the node's defined-from (the
    // seed bar is not published).
    r.outputs.push(Sink::iface_lead(e, period));
    r
}

/// `WILDERS(p)` over `Input{0}` — one decayed-average node. Warm = fold the
/// first `p` into a mean seed; steady = `w·(p−1)/p + v·(1/p)`. Publish lead
/// = `period` (hand reports `len − p` rows, `min_data = p+1`).
pub fn wilders(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let w = r.add(Node::Wilders {
        src: close,
        period,
        composite: false,
        feed: false,
    });
    r.kind = "wilders";
    r.options = options.to_vec();
    r.tail_inputs = vec![w];
    r.outputs.push(Sink::iface_lead(w, period));
    r
}

/// `SMA(p)` — one price-sourced rolling-sum node. Publish lead = `period`:
/// the hand reports `len − period` rows (init folds bars `0..p` but the
/// seed bar is not published), matching the `Ema` mirror convention. Hand
/// state is the lone `sum` — the plan keeps the identical value in the
/// node's state word (see `sma::restore_state`).
pub fn sma(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let m = r.add(Node::Sma { src: close, period });
    r.tag(m, "sma");
    r.kind = "sma";
    r.options = options.to_vec();
    r.tail_inputs = vec![m];
    r.outputs.push(Sink::iface_lead(m, period));
    r
}

/// `WMA(p)` — the weighted-sum trick node (its own window `sum` carry, read
/// BEFORE its update exactly as the hand `calc` orders it, plus the
/// weighted line) and the shared `Sma` mirror node: any `Sma(p)` request
/// over the same column interns onto that mirror, so the pair's rows come
/// off one arena node while each rolling op keeps its own bit-exact update
/// chain (the hand's own structure). Lead = `period` on both rows.
pub fn wma(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let w = r.add(Node::Wma { src: close, period });
    r.tag(w, "wma");
    let m = r.add(Node::Sma { src: close, period });
    r.tag(m, "sma");
    r.kind = "wma";
    r.options = options.to_vec();
    r.tail_inputs = vec![w];
    r.outputs.push(Sink::iface_lead(w, period));
    r.optional_outputs.push(Sink::iface_lead(m, period));
    r
}

/// `STDDEV(p)` — the self-contained two-moment node (its own `sum` +
/// `sum_sq` carries, hand `calc` op for op); its optional SMA line is the
/// SAME `Sma` node the standalone request builds, so CSE gives ONE
/// recurrence with two mirrors. Lead = `period` for both rows.
/// Shared builder behind `stddev()` and `vidya()`'s two internal subgraphs --
/// `feed`, when set, overrides the `Stddev` node's early-publish bar (see
/// `Node::Stddev`'s doc); `stddev()` itself always passes `None`.
pub(crate) fn stddev_recipe(period: u32, feed: Option<u32>, switch: Option<u32>) -> Recipe {
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let sd = r.add(Node::Stddev {
        src: close,
        period,
        feed,
        switch,
    });
    r.tag(sd, "stddev");
    let m = r.add(Node::Sma { src: close, period });
    r.tag(m, "sma");
    r.kind = "stddev";
    r.tail_inputs = vec![sd];
    r.outputs.push(Sink::iface_lead(sd, feed.unwrap_or(period)));
    r.optional_outputs.push(Sink::iface_lead(m, period));
    r
}

pub fn stddev(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = stddev_recipe(period, None, None);
    r.options = options.to_vec();
    r
}
