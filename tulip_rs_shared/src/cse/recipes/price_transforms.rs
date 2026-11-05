//! Price transform recipes: medprice, typprice, wcprice, avgprice

use crate::cse::{Node, Recipe, Sink};

/// `MEDPRICE = 0.5 * (high + low)` -- stateless, every bar publishes
/// (`Sink::iface`, lead 0). Streams: 0 = high, 1 = low.
pub fn medprice(_options: &[f64]) -> Recipe {
    let mut r = Recipe::hl();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let sum = r.add(Node::Add { a: hi, b: lo });
    r.tag(sum, "hl_sum");
    let half = r.constant(0.5);
    r.tag(half, "half");
    let med = r.add(Node::Mul { a: half, b: sum });
    r.tag(med, "medprice");
    r.kind = "medprice";
    r.options = Vec::new();
    r.outputs.push(Sink::iface(med));
    r
}

/// `BOP` — Balance of Power: `(close - open) / (high - low).max(f64::EPSILON)`.
/// Streams: 0=open, 1=high, 2=low, 3=close. No options.
pub fn bop(_options: &[f64]) -> Recipe {
    let mut r = Recipe::ohlc();
    let open = r.add(Node::Input { stream: 0 });
    r.tag(open, "open");
    let high = r.add(Node::Input { stream: 1 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 2 });
    r.tag(low, "low");
    let close = r.add(Node::Input { stream: 3 });
    r.tag(close, "close");
    let hl_diff = r.add(Node::Sub { a: high, b: low });
    let eps = r.constant(f64::EPSILON);
    let den = r.add(Node::Max { a: hl_diff, b: eps });
    let num = r.add(Node::Sub { a: close, b: open });
    let line = r.add(Node::Div { a: num, b: den });
    r.tag(line, "bop");

    r.kind = "bop";
    r.options = Vec::new();
    r.outputs.push(Sink::iface(line));
    r
}

/// `TYPPRICE` = `(high + low + close) / 3` — stateless, every bar publishes
/// (`Sink::iface`, lead 0). Streams: 0=high, 1=low, 2=close. No options.
pub fn typprice(_options: &[f64]) -> Recipe {
    let mut r = Recipe::hlc();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let close = r.add(Node::Input { stream: 2 });
    r.tag(close, "close");
    let sum = r.add(Node::Add { a: high, b: low });
    let sum2 = r.add(Node::Add { a: sum, b: close });
    let div = r.constant(1.0 / 3.0);
    r.tag(div, "third");
    let line = r.add(Node::Mul { a: sum2, b: div });
    r.tag(line, "typprice");

    r.kind = "typprice";
    r.options = Vec::new();
    r.outputs.push(Sink::iface(line));
    r
}

/// `WCPRICE` — Weighted Close Price, hand `wcprice.rs::calc` =
/// `close.mul_add(2.0, high + low) * 0.25`. The `mul_add` is **one fused rounding**, so
/// this spells `Node::Fma` — a separate `Mul`+`Add` double-rounds and is not bit-exact.
/// Streams: 0=high, 1=low, 2=close. No options.
pub fn wcprice(_options: &[f64]) -> Recipe {
    let mut r = Recipe::hlc();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let close = r.add(Node::Input { stream: 2 });
    r.tag(close, "close");
    let sum_hl = r.add(Node::Add { a: high, b: low });
    let two = r.constant(2.0);
    r.tag(two, "two");
    let prod = r.add(Node::Fma {
        a: close,
        b: two,
        c: sum_hl,
    });
    r.tag(prod, "hl_2c");
    let quarter = r.constant(0.25);
    r.tag(quarter, "quarter");
    let line = r.add(Node::Mul {
        a: prod,
        b: quarter,
    });
    r.tag(line, "wcprice");

    r.kind = "wcprice";
    r.options = Vec::new();
    r.outputs.push(Sink::iface(line));
    r
}

/// `AVGPRICE` = `(open + high + low + close) * 0.25` — stateless, every bar
/// publishes (`Sink::iface`, lead 0). Streams: 0=open, 1=high, 2=low, 3=close.
pub fn avgprice(_options: &[f64]) -> Recipe {
    let mut r = Recipe::ohlc();
    let open = r.add(Node::Input { stream: 0 });
    r.tag(open, "open");
    let high = r.add(Node::Input { stream: 1 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 2 });
    r.tag(low, "low");
    let close = r.add(Node::Input { stream: 3 });
    r.tag(close, "close");
    let sum = r.add(Node::Add { a: open, b: high });
    r.tag(sum, "oh_sum");
    let sum2 = r.add(Node::Add { a: sum, b: low });
    r.tag(sum2, "ohl_sum");
    let line = r.add(Node::Add { a: sum2, b: close });
    r.tag(line, "ohlc_sum");
    let quarter = r.constant(0.25);
    r.tag(quarter, "quarter");
    let avg = r.add(Node::Mul {
        a: line,
        b: quarter,
    });
    r.tag(avg, "avgprice");

    r.kind = "avgprice";
    r.options = Vec::new();
    r.outputs.push(Sink::iface(avg));
    r
}
