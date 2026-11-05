use crate::cse::{Id, Node, Recipe, Sink};

/// `Elderray(p)` = `(high − ema(close,p), low − ema(close,p))` — the two
/// divergence lines over close's EMA. Publish lead = `period`.
pub fn elderray(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hlc();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let close = r.add(Node::Input { stream: 2 });
    r.tag(close, "close");
    let ema = r.add(Node::Ema { src: close, period });
    r.tag(ema, "ema");
    let bull = r.add(Node::Sub { a: high, b: ema });
    r.tag(bull, "bull");
    let bear = r.add(Node::Sub { a: low, b: ema });
    r.tag(bear, "bear");
    r.kind = "elderray";
    r.options = options.to_vec();
    r.tail_inputs = vec![ema];
    r.outputs.push(Sink::iface_lead(bull, period));
    r.outputs.push(Sink::iface_lead(bear, period));
    r.optional_outputs.push(Sink::iface_lead(ema, period));
    r
}

/// `Apo(s, l) ≡ ema_s(close) − ema_l(close)`: the two EMA chains plus the
/// line `Sub` — the §4 identity node shared (by CSE key) with MACD's line
/// and PPO's numerator. SSA order matches `TState::calc` (short before long).
pub fn apo(options: &[f64]) -> Recipe {
    let short = options[0] as u32;
    let long = options[1] as u32;
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let s = r.add(Node::Ema {
        src: close,
        period: short,
    });
    r.tag(s, "ema_short");
    let l = r.add(Node::Ema {
        src: close,
        period: long,
    });
    r.tag(l, "ema_long");
    let line = r.add(Node::Sub { a: s, b: l });
    r.tag(line, "line");

    r.kind = "apo";
    r.options = options.to_vec();
    r.tail_inputs = vec![s, l];
    r.outputs.push(Sink::iface_lead(line, long - 1));
    // optional_outputs in Info order: short_ema at `Ema::output_length`
    // capacity (lead = period), long reuses the primary capacity
    // (lead = long−1 = defined-from).
    r.optional_outputs.push(Sink::iface_lead(s, short));
    r.optional_outputs.push(Sink::iface(l));
    r
}

/// `Ppo(s, l) ≡ (ema_s − ema_l) * 100.0 / ema_l`, in the hand's exact
/// evaluation order `((s − l) * 100.0) / l` — reassociating would change the
/// bits. The `Sub` numerator is the SAME key apo/macd declare (one `fsub`).
pub fn ppo(options: &[f64]) -> Recipe {
    let short = options[0] as u32;
    let long = options[1] as u32;
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let s = r.add(Node::Ema {
        src: close,
        period: short,
    });
    let l = r.add(Node::Ema {
        src: close,
        period: long,
    });
    let line = r.add(Node::Sub { a: s, b: l });
    let c100 = r.constant(100.0);
    r.tag(c100, "hundred");
    let num = r.add(Node::Mul { a: line, b: c100 });
    r.tag(num, "scaled");
    let ratio = r.add(Node::Div { a: num, b: l });
    r.tag(ratio, "ppo");

    r.kind = "ppo";
    r.options = options.to_vec();
    r.tail_inputs = vec![s, l];
    // publish lead = min_data − 1 = options[1] (output_length = data_len −
    // min_data + 1, right-aligned); the ratio node is DEFINED at long-1 but
    // the hand capacity starts it one bar later.
    r.outputs.push(Sink::iface_lead(ratio, long));
    r.optional_outputs.push(Sink::iface_lead(s, short));
    r.optional_outputs.push(Sink::iface_lead(l, long));
    r
}

/// `Macd(s, l, g)`: the apo line (`Sub` of the close EMAs — shared key), an
/// `Ema` of THAT node as the signal (chained recurrence: fed the line's
/// reported bars), and the hist `Sub`. SSA order matches `TState::calc`.
pub fn macd(options: &[f64]) -> Recipe {
    let short = options[0] as u32;
    let long = options[1] as u32;
    let signal_period = options[2] as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "price");
    let s = r.add(Node::Ema {
        src: real,
        period: short,
    });
    let l = r.add(Node::Ema {
        src: real,
        period: long,
    });
    let line = r.add(Node::Sub { a: s, b: l });
    let sig = r.add(Node::Ema {
        src: line,
        period: signal_period,
    });
    r.tag(sig, "signal");
    let hist = r.add(Node::Sub { a: line, b: sig });
    r.tag(hist, "histogram");

    r.kind = "macd";
    r.options = options.to_vec();
    // The hand State's three chains, in TState field order.
    r.tail_inputs = vec![s, l, sig];
    // outputs in Info order: line, signal, histogram — publish leads = hand
    // slot_lengths offsets (25 / 33 / 33 for 12/26/9).
    r.outputs.push(Sink::iface_lead(line, long - 1));
    r.outputs
        .push(Sink::iface_lead(sig, long + signal_period - 2));
    r.outputs
        .push(Sink::iface_lead(hist, long + signal_period - 2));
    r.optional_outputs.push(Sink::iface_lead(s, short));
    r.optional_outputs.push(Sink::iface_lead(l, long));
    r
}

/// `AO(s, l)` = `mean RollSum(medprice, s) − mean RollSum(medprice, l)` —
/// the ring-backed pair over the ONE shared `medprice` node, spelled with
/// the same `Mul(half, Add)` shape as [`super::medprice`] so interning collapses
/// them (one computed medprice column per bar, one ring group of capacity
/// `max(s, l)`; leave = eviction at the cap, age read below it). Streams:
/// 0 = high, 1 = low; options `[short, long]` default to the hand module's
/// consts `(5, 34)`. Publish lead = `long` EXPLICIT — the hand's LONG row
/// begins one bar after the long roll's own defined-from (bar `long−1`)
/// because its `init_state` never emits the fold-completion mean; clamping
/// up there is exactly `leads`+explicit contract.
pub fn ao(options: &[f64]) -> Recipe {
    let short = options.first().copied().unwrap_or(5.0) as u32;
    let long = options.get(1).copied().unwrap_or(34.0) as u32;
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
    let s_short = r.add(Node::RollSum {
        src: med,
        period: short,
        mean: true,
        skip: 0,
        steady_from: 0,
    });
    r.tag(s_short, "sma_short");
    let s_long = r.add(Node::RollSum {
        src: med,
        period: long,
        mean: true,
        skip: 0,
        steady_from: 0,
    });
    r.tag(s_long, "sma_long");
    let ao = r.add(Node::Sub {
        a: s_short,
        b: s_long,
    });
    r.tag(ao, "ao");
    r.kind = "ao";
    r.options = vec![short as f64, long as f64];
    r.outputs.push(Sink::iface_lead(ao, long));
    // Hand `Info::optional_outputs = ["short_sma", "long_sma", "medprice"]`
    // — declared order, leads = each node's defined-from. The `medprice`
    // mirror is the seam that lets AO fuse with supertrend/fisher: all read
    // the SAME interned node.
    r.optional_outputs.push(Sink::iface(s_short));
    r.optional_outputs.push(Sink::iface(s_long));
    r.optional_outputs.push(Sink::iface(med));
    r
}

/// `HMA(p)` — ONE bespoke rolling node (E3 stage 3): the two inner WMA
/// engines (p and p/2 with the hand's OFFSET init window), the diff series
/// and its √p ring all live inside the op. It is NOT a recipe of generic
/// Wmas — composition is not bit-exact (state2's fold weights restart at
/// the offset window; the tail divides `wss/ww` BEFORE its subtractions).
/// Publish lead `p + trunc(sqrt p)` = hand `init_state`'s returned start.
pub fn hma(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let psqrt = (period as f64).sqrt() as u32;
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    let h = r.add(Node::Hma { src: close, period });
    r.kind = "hma";
    r.options = options.to_vec();
    r.tail_inputs = vec![h];
    r.outputs.push(Sink::iface_lead(h, period + psqrt));
    r
}

/// `FISHER(period)` — the fisher transform over the DERIVED medprice
/// column, mirroring `fisher.rs::calc_fisher` op for op: `med = 0.5·(high+low)`
/// (the SAME interned node AO/supertrend read — co-requesting fisher with AO
/// adds zero arithmetic nodes, pinned by the merge canary); a
/// `(period, period−1)` min/max pair over that derived column (the hand's
/// `MirrorBuffer` + min/max states — an extremum ring, the fourth ring
/// shape); `inner = (med−min)/max(max−min, 1e−3) − 0.5`; `val1` with the
/// hand's if/elif clamp folded INTO the carried value (`IirClamped`,
/// 0.66·inner + 0.67·prev); `fish = 0.5·(ln((1+val1)/(1−val1)) + prev)`
/// (`Iir`); `signal = prev fish` (`Prev`). Parity law: 1e-12 RELATIVE vs
/// the hand (`Node::Ln` is a host libm call — Handover §6.41).
pub fn fisher(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hl();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let sum = r.add(Node::Add { a: hi, b: lo });
    r.tag(sum, "hl_sum");
    let half = r.constant(0.5);
    let med = r.add(Node::Mul { a: half, b: sum });
    r.tag(med, "medprice");
    let mn = r.add(Node::RollMin {
        src: med,
        period,
        look_back: period - 1,
    });
    r.tag(mn, "med_min");
    let mx = r.add(Node::RollMax {
        src: med,
        period,
        look_back: period - 1,
    });
    r.tag(mx, "med_max");
    let eps = r.constant(0.001);
    let range = r.add(Node::Sub { a: mx, b: mn });
    let mm = r.add(Node::Max { a: range, b: eps });
    let diff = r.add(Node::Sub { a: med, b: mn });
    let ratio = r.add(Node::Div { a: diff, b: mm });
    let inner = r.add(Node::Sub { a: ratio, b: half });
    // clamp thresholds/values as Const children, BEFORE the node that uses
    // them (their lead 0 keeps the arena order trivial).
    let hi_t = r.constant(0.99);
    let hi_v = r.constant(0.999);
    let lo_t = r.constant(-0.99);
    let lo_v = r.constant(-0.999);
    let val1 = r.add(Node::IirClamped {
        src: inner,
        a_bits: 0.66f64.to_bits(),
        b_bits: 0.67f64.to_bits(),
        hi_t,
        hi_v,
        lo_t,
        lo_v,
    });
    r.tag(val1, "val1");
    let one = r.constant(1.0);
    let numer = r.add(Node::Add { a: one, b: val1 });
    let denom = r.add(Node::Sub { a: one, b: val1 });
    let ln_arg = r.add(Node::Div { a: numer, b: denom });
    let ln = r.add(Node::Ln { src: ln_arg });
    r.tag(ln, "ln");
    let fish = r.add(Node::Iir {
        src: ln,
        a_bits: 0.5f64.to_bits(),
        b_bits: 0.5f64.to_bits(),
    });
    r.tag(fish, "fisher");
    let signal = r.add(Node::Prev { src: fish });
    r.tag(signal, "fisher_signal");
    r.kind = "fisher";
    r.options = vec![period as f64];
    // hand `State` field order: `val1` (the `IirClamped` node's own hidden
    // carry), then `fish` (the `Iir` node's own hidden carry). `fish` has no
    // dedicated export of its own: `signal` (`Node::Prev{src: fish}`) already
    // publishes exactly fish's last-bar value every bar, so reading the
    // already-live `signal` node through the existing `ChainState::Mirror`
    // shape recovers `fish` with zero new vocabulary -- `Node::Prev`'s state
    // IS a bare last-value carry, the same shape `Node::Mirror` publishes.
    // `buffer`/`min_state`/`max_state` are NOT tail_inputs: they're a
    // `period`-wide window of `medprice`, a pointwise function of raw
    // `high`/`low`, fully recomputable from a raw tail (the `cci`/`stochrsi`/
    // `md` technique) -- no chain export needed for them.
    r.tail_inputs = vec![val1, signal];
    r.outputs.push(Sink::iface(fish));
    r.outputs.push(Sink::iface(signal));
    r
}

/// `STOCH` (slow stochastic) as a standalone request: three price streams,
/// the fast-%K division over the shared `(k, k−1)` high/low rolls, slow %K as
/// the mean of fast with `skip: 2`, and %D as the mean of the slow line. The
/// denominator is `Max(dif, ε)` — the hand FLOORS at epsilon (a flat bar
/// prints a huge fast %K; the engine must not "improve" that), the ε-guard
/// and left-assoc `Div(Mul(100, sub), den)` transcription are bit-load-bearing.
pub fn stoch(options: &[f64]) -> Recipe {
    let k_period = options[0] as u32;
    let k_slow = options[1] as u32;
    let d_period = options[2] as u32;
    let look_back = k_period.saturating_sub(1);

    let mut r = Recipe::hlc();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let close = r.add(Node::Input { stream: 2 });
    r.tag(close, "close");
    let mx = r.add(Node::RollMax {
        src: high,
        period: k_period,
        look_back,
    });
    let mn = r.add(Node::RollMin {
        src: low,
        period: k_period,
        look_back,
    });
    let dif = r.add(Node::Sub { a: mx, b: mn });
    let eps = r.constant(f64::EPSILON);
    let den = r.add(Node::Max { a: dif, b: eps });
    let mc = r.add(Node::Sub { a: close, b: mn });
    let c100 = r.constant(100.0);
    let num = r.add(Node::Mul { a: c100, b: mc });
    let kfast = r.add(Node::Div { a: num, b: den });
    // Slow %K: the hand discards fast %K's first two published values
    // (its roll seeding), hence `skip: 2`.
    let k_line = r.add(Node::RollSum {
        src: kfast,
        period: k_slow,
        mean: true,
        skip: 2,
        // The hand's init loop runs to bar `k + k_slow + d − 1`, past the
        // first full window, and inside it the eviction subtracts as its own
        // statement — so the steady `sum += v - old` only begins at
        // `k + k_slow + d`, exactly where %D starts printing.
        steady_from: k_period + k_slow + d_period,
    });
    // %D: a mean of the slow %K line, folding from that line's first
    // published bar — the hand's `prev_d` receives exactly those, and its
    // init loop never evicts, so the steady form applies throughout.
    let d_line = r.add(Node::RollSum {
        src: k_line,
        period: d_period,
        mean: true,
        skip: 0,
        steady_from: 0,
    });
    r.tail_inputs = vec![mx, k_line, d_line];
    r.kind = "stoch";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface(k_line));
    r.outputs
        .push(Sink::iface_lead(d_line, k_period + k_slow + d_period));
    r
}

/// `RSI(p)` as a standalone request: the movement `diff = real − Lag{real,1}`
/// split through two `Max` gates into the hand's `up_down` arms, each fed to a
/// mean-`Wilders` chain (seed = mean of the first `p` gated values, then the
/// Wilders recurrence), and `line = 100·up_avg/(up_avg+down_avg)`. The second
/// arm is `Max(Lag − real, 0)` — the antisymmetric difference, exact to the
/// bit by IEEE — NOT the one-guard rewrite `up − diff`: on `diff = +∞` that
/// rewrite returns `NaN` where the hand writes `+0.0`, and a NaN entering a
/// Wilders chain poisons every later bar (`up_down_rewrite_proofs` swept
/// ~7,000 movement classes and found it). Publish lead `period + 1` (hand
/// `min_data = period + 2`); the state mirrors (`prev_real`) ride
/// `tail_inputs`, outside the sink graph.
pub fn rsi(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "price");
    let prev = r.add(Node::Lag { src: real, n: 1 });
    let diff = r.add(Node::Sub { a: real, b: prev });
    let zero = r.constant(0.0);
    let up = r.add(Node::Max { a: diff, b: zero });
    r.tag(up, "up");
    let neg = r.add(Node::Sub { a: prev, b: real });
    let down = r.add(Node::Max { a: neg, b: zero });
    r.tag(down, "down");
    let up_avg = r.add(Node::Wilders {
        src: up,
        period,
        composite: false,
        feed: true,
    });
    r.tag(up_avg, "up_avg");
    let down_avg = r.add(Node::Wilders {
        src: down,
        period,
        composite: false,
        feed: true,
    });
    r.tag(down_avg, "down_avg");
    let total = r.add(Node::Add {
        a: up_avg,
        b: down_avg,
    });
    let ratio = r.add(Node::Div {
        a: up_avg,
        b: total,
    });
    r.tag(ratio, "ratio");
    let hundred = r.constant(100.0);
    let line = r.add(Node::Mul {
        a: hundred,
        b: ratio,
    });
    r.tag(line, "rsi");
    // State mirror: the hand keeps the last processed close so the next
    // `calc` can difference against it. Identity copy, feeds NO sink — the
    // emitter's `Kind::Mirror` arm computes it dead in the steady loop and
    // `get_states` (M1) is the real consumer. Verbatim from the runtime
    // recipe: dropping it would break the `[Wilders, Wilders, Mirror]`
    // state-layout contract in `restore_state`.
    let prev_real = r.add(Node::Mirror { src: real });
    r.kind = "rsi";
    r.options = options.to_vec();
    // hand `State` field order: the up lane, the down lane, `prev_real`
    r.tail_inputs = vec![up_avg, down_avg, prev_real];
    r.outputs.push(Sink::iface_lead(line, period + 1));
    r
}

/// `STOCHRIS(p)` = `100·(rsi − min) / (max − min)` over the last `p` RSI
/// values, with the RSI line itself an optional output — ONE option driving
/// both the Wilders smoothing and the stochastic window, exactly as the hand
/// wires it. The RSI half is the `rsi` leaf graph GRAFTED (the runtime
/// recipe's `sub_at::<Rsi>` — same `Recipe::graft` call, so a co-requested
/// `Rsi(p)` interning the whole movement chain, both Wilders chains, the
/// division and the ×100 is the same stored stream as this recipe's optional
/// rsi row. The rolls are `(p, p−1)` over a DERIVED source — the extremum
/// ring machinery (proven on fisher's medprice rings).
pub fn stochrsi(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "price");
    // Exactly what the runtime recipe's `sub_at::<Rsi>(&[period], &[real])`
    // does — graft the leaf rsi graph with stream 0 remapped onto `real`;
    // `.primary` is the grafted rsi `line` node. The grafted `prev_real`
    // mirror rides the arena dead (state channel, feeds no sink here).
    let g = r.graft(rsi(options), &[real]);
    let rsi = g.primary;
    let mx = r.add(Node::RollMax {
        src: rsi,
        period,
        look_back: period.saturating_sub(1),
    });
    let mn = r.add(Node::RollMin {
        src: rsi,
        period,
        look_back: period.saturating_sub(1),
    });
    let kdif = r.add(Node::Sub { a: mx, b: mn });
    let sub = r.add(Node::Sub { a: rsi, b: mn });
    let c100 = r.constant(100.0);
    let num = r.add(Node::Mul { a: c100, b: sub });
    let q = r.add(Node::Div { a: num, b: kdif });
    let eps = r.constant(f64::EPSILON);
    // `if kdif < ε { 0.0 } else { q }` is `WhereLt` directly — the `<`-form
    // mirror of `WhereGe`, which is what the hand actually writes. The reason
    // it must be `<` and not `>=`: on a FLAT series the RSI line is exactly
    // `0/0 = NaN`, so `kdif = max − min` is NaN, and an IEEE compare is false in
    // BOTH directions there. The hand's `<` test therefore falls to the ELSE arm
    // (the division) and keeps the NaN, which is what it prints; `WhereGe`'s
    // false branch is the pinned `+0.0` and swallows it (the parity test failed
    // on `stochrsi flat p3` until this node replaced a `0 − maxNum(0 − kdif, −∞)`
    // mapping that cost three extra ops to buy the same NaN propagation).
    let line = r.add(Node::WhereLt {
        a: kdif,
        b: eps,
        t: q,
    });
    r.tag(line, "stochrsi");
    r.kind = "stochrsi";
    r.options = options.to_vec();
    // leads: the rsi node exists from bar `p + 1`, the roll's window is
    // complete at `2p − 1`, and the hand PRINTS from `2p` (`min_data =
    // 2p + 1`), hence the explicit primary lead; the optional rsi row keeps
    // rsi's own `p + 1` lead, matching `Rsi::output_length`.
    r.outputs.push(Sink::iface_lead(line, period * 2));
    r.optional_outputs = vec![Sink::iface_lead(rsi, period + 1)];
    r.tail_inputs = vec![mn];
    r
}

/// `CMO(p)` = `100·(Σup − Σdown) / (Σup + Σdown)` over the movement of the
/// last `p` bars — the first consumer of `RollSum`, a rolling sum over a
/// DERIVED source: what it accumulates is `up`/`down`, not a price, so the
/// leaving term comes from a ring of that series' own recent values (two
/// ring groups, each single-member cap `p`). Hand facts encoded here, read
/// against `cycle_cmo(&real[1..], …, period, …)`:
/// · `up_down(v, prev) = [diff.max(0), (-diff).max(0)]` with `diff = v − prev`
///   and `prev = Lag{real, 1}` — the SAME nodes `rsi` builds (the negative
///   arm written `Max(Lag − real, 0)`, exact by IEEE's antisymmetric
///   subtraction), so an `rsi`+`cmo` query computes the movement ONCE; only
///   the aggregation differs: RSI Wilder-smooths these, CMO rolls them up.
/// · `init_state` folds bars `1..=p` (its `skip(1)`), so a full window
///   exists at bar `p`. `RollSum` folds from bar 0 instead, and bar 0's
///   movement is `[0.0, 0.0]` through the `Lag` clamp — adding a zero
///   changes neither value nor rounding, so the two agree bit-for-bit.
/// · `min_data = p + 2` while the first PRINTED bar is `p + 1`, hence the
///   explicit sink lead. The state carries `prev` (last close) and
///   `drop_real` (the close `p` bars back) as Mirrors — copies riding the
///   `Input` and the `Lag{real, period}` nodes, so reporting them adds no
///   arithmetic.
pub fn cmo(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "price");
    let l1 = r.add(Node::Lag { src: real, n: 1 });
    let diff = r.add(Node::Sub { a: real, b: l1 });
    let zero = r.constant(0.0);
    let up = r.add(Node::Max { a: diff, b: zero });
    r.tag(up, "up");
    let neg = r.add(Node::Sub { a: l1, b: real });
    let down = r.add(Node::Max { a: neg, b: zero });
    r.tag(down, "down");
    let up_sum = r.add(Node::RollSum {
        src: up,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(up_sum, "up_sum");
    let down_sum = r.add(Node::RollSum {
        src: down,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(down_sum, "down_sum");
    // `100.0 * (up_sum - down_sum) / (up_sum + down_sum)` parses left to
    // right, so the DIVIDE's numerator is the already-scaled difference.
    let hundred = r.constant(100.0);
    let num = r.add(Node::Sub {
        a: up_sum,
        b: down_sum,
    });
    let scaled = r.add(Node::Mul { a: hundred, b: num });
    let den = r.add(Node::Add {
        a: up_sum,
        b: down_sum,
    });
    let line = r.add(Node::Div { a: scaled, b: den });
    r.tag(line, "cmo");
    let lp = r.add(Node::Lag {
        src: real,
        n: period,
    });
    let drop_real = r.add(Node::Mirror { src: lp });
    let prev = r.add(Node::Mirror { src: real });
    r.kind = "cmo";
    r.options = options.to_vec();
    // hand `State` field order: up_sum, down_sum, prev, drop_real.
    r.tail_inputs = vec![up_sum, down_sum, prev, drop_real];
    r.outputs.push(Sink::iface_lead(line, period + 1));
    r
}

/// `ULTOSC(short, medium, long)` — Ultimate Oscillator: the weighted ratio of
/// three-window buying-pressure and true-range sums.
///
/// The arena mirrors `ultosc.rs` statement for statement. `true_low =
/// Min(low, prev)`, `true_high = Max(high, prev)`, `bp = close − true_low`,
/// `tr = true_high − true_low`, then SIX raw sums (mean: false — the hand
/// divides sums by sums, never scales a window) and the tail
/// `(4·bps/trs + 2·bps_m/trs_m + bps_l/trs_l) · (100.0/7.0)`, parenthesised in
/// that order because association is visible.
///
/// **Why `Lag` and not `Prev`:** the hand seeds `prev_close = close[0]`
/// (`State::new(long_period, close[0])`) and its warm-up loop is
/// `.skip(1).take(long_period)`, so bar 0 is never computed and never pushed.
/// `Node::Lag`'s clamp rule reads sample 0 for `t < n` — the same value the hand
/// holds at bar 0 — and it gives `prev` LEAD 1, so `bp`/`tr` inherit lead 1 and
/// the ring first feeds at bar 1. That propagation is why `skip` stays 0: the
/// feed start comes from the structure, not from a knob.
///
/// **The container this shape is the test for:** `bp` and `tr` are two
/// independent pure-sum groups of THREE members each (short, medium, long), all
/// three windows read through one capacity (`long`), and the long member's leave
/// is the eviction at `p == cap`. `lowering::bundles()` fuses the pair into ONE
/// `MultiBuffer<2>` — which is precisely the hand's own `Buffer<2, f64, S>` — so
/// one push and one cursor advance per bar serve all six folds.
pub fn ultosc(options: &[f64]) -> Recipe {
    let (short, medium, long) = (options[0] as u32, options[1] as u32, options[2] as u32);
    let mut r = Recipe::hlc();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let cl = r.add(Node::Input { stream: 2 });
    r.tag(cl, "close");

    let prev = r.add(Node::Lag { src: cl, n: 1 });
    r.tag(prev, "prev_close");
    let tl = r.add(Node::Min { a: lo, b: prev });
    r.tag(tl, "true_low");
    let th = r.add(Node::Max { a: hi, b: prev });
    r.tag(th, "true_high");
    let bp = r.add(Node::Sub { a: cl, b: tl });
    r.tag(bp, "bp");
    let tr = r.add(Node::Sub { a: th, b: tl });
    r.tag(tr, "tr");

    // Two groups of three members, all six slots AFTER both sources: that is
    // what lets `bundles()` place one fused call at the first sum node (its SSA
    // guard is max src < min member).
    let roll = |p: u32, src: Id| Node::RollSum {
        src,
        period: p,
        mean: false,
        skip: 0,
        steady_from: 0,
    };
    let bp_s = r.add(roll(short, bp));
    r.tag(bp_s, "bp_sum_s");
    let bp_m = r.add(roll(medium, bp));
    r.tag(bp_m, "bp_sum_m");
    let bp_l = r.add(roll(long, bp));
    r.tag(bp_l, "bp_sum_l");
    let tr_s = r.add(roll(short, tr));
    r.tag(tr_s, "tr_sum_s");
    let tr_m = r.add(roll(medium, tr));
    r.tag(tr_m, "tr_sum_m");
    let tr_l = r.add(roll(long, tr));
    r.tag(tr_l, "tr_sum_l");

    let four = r.constant(4.0);
    let two = r.constant(2.0);
    let div = r.constant(100.0 / 7.0);
    let bp_s4 = r.add(Node::Mul { a: four, b: bp_s });
    let w_s = r.add(Node::Div { a: bp_s4, b: tr_s });
    let bp_m2 = r.add(Node::Mul { a: two, b: bp_m });
    let w_m = r.add(Node::Div { a: bp_m2, b: tr_m });
    let third = r.add(Node::Div { a: bp_l, b: tr_l });
    // `reduce_sum()` over the hand's 2-lane vector is this add; then `+ third`,
    // then the scale — the same left-to-right order `ultosc.rs` writes.
    let pair = r.add(Node::Add { a: w_s, b: w_m });
    let wsum = r.add(Node::Add { a: pair, b: third });
    let line = r.add(Node::Mul { a: wsum, b: div });
    r.tag(line, "ultosc");

    r.kind = "ultosc";
    r.options = options.to_vec();
    // Full-recompute restore (same technique as `stoch`/`cci`/`md`): the hand
    // `State` is a `MultiBuffer`-shaped carry that isn't worth exporting field
    // by field, but `State::init_state` rebuilds it exactly from a raw
    // `(high, low, close)` tail of `long_period + 1` bars. We only need the
    // three period VALUES back, and the three `RollSum{mean: false}` groups
    // report them for free via `ChainState::Sma{period, ..}` (RollSum groups
    // share that variant with true SMA regardless of `mean`) -- `bp_l`
    // (the long-period group) is already a live ancestor of the output, so
    // this adds only two more liveness roots (`bp_s`, `bp_m`), not new ones.
    r.tail_inputs = vec![bp_s, bp_m, bp_l];
    r.outputs.push(Sink::iface_lead(line, long));
    // `tr` and `bp` are OPTIONAL, not primary: the hand allocates them with
    // `init_optional_outputs_eff!(optional_outputs, &[false, false], …)`, so a
    // primary-only request emits ONE row. (The `Info.outputs` table lists all
    // three; the allocation site is the authority — same rule as `atr`'s `tr`.)
    r.optional_outputs.push(Sink::iface_lead(tr, 1));
    r.optional_outputs.push(Sink::iface_lead(bp, short));
    r
}
/// `MOM(p) = real[i] − real[i − p]` — the engine's multi-bar `Lag` read.
///
/// This is the recipe that makes `lag_n` observable in a PUBLISHED row. The
/// hand's `cycle_mom` pairs bar `i` with bar `i − p`, and its lead is `p`
/// (`min_data = p + 1`, `output_length = len − p`), so a steady-loop read of
/// one bar back instead of `p` is a wrong VALUE, not a state-only artefact —
/// which is what `cmo`'s `Lag{src: real, n: period}` could never show, because
/// there the lag feeds only a `Mirror` (state, `get_states`, M1) and never a
/// sink.
///
/// `n` is a VALUE (`Values::lag_n`), never structure: the signature erases it,
/// and at macro expansion the parameter arrives as a sentinel. Same as `cmo`.
///
/// The hand's state is a raw window of the last `p` `real` values, not an
/// engine chain, so `tail_inputs` is empty — `get_states` has no chain to
/// restore here.
pub fn mom(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let cl = r.add(Node::Input { stream: 0 });
    r.tag(cl, "price");
    let prev = r.add(Node::Lag { src: cl, n: period });
    r.tag(prev, "prev_period");
    let line = r.add(Node::Sub { a: cl, b: prev });
    r.tag(line, "mom");

    r.kind = "mom";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(line, period));
    r
}

/// `ROC(p)` — Rate of Change, hand `roc.rs::calc` = `calc_rocr(mom, prev_real)` where
/// `calc_mom = real - prev_real` and `calc_rocr(a, b) = a / b.max(f64::EPSILON)`.
/// So ROC is `(real − real[p]) / real[p].max(eps)` — **no ×100**, the hand does not scale
/// it — and it publishes `mom` as its optional row (`Info.optional_outputs: ["mom"]`).
/// One option (period). Streams: 0=real. Publish lead = period.
pub fn roc(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let prev = r.add(Node::Lag {
        src: close,
        n: period,
    });
    r.tag(prev, "prev_period");
    let mom = r.add(Node::Sub { a: close, b: prev });
    r.tag(mom, "mom");
    let eps = r.constant(f64::EPSILON);
    let den = r.add(Node::Max { a: prev, b: eps });
    let line = r.add(Node::Div { a: mom, b: den });
    r.tag(line, "roc");

    r.kind = "roc";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(line, period));
    r.optional_outputs.push(Sink::iface_lead(mom, period));
    r
}

/// `ROCR(p)` — Rate of Change Ratio, hand `rocr.rs::calc` =
/// `real / prev_real.max(f64::EPSILON)`. One option (period). Streams: 0=real.
/// Publish lead = period.
pub fn rocr(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let prev = r.add(Node::Lag {
        src: close,
        n: period,
    });
    r.tag(prev, "prev_period");
    let eps = r.constant(f64::EPSILON);
    let den = r.add(Node::Max { a: prev, b: eps });
    let line = r.add(Node::Div { a: close, b: den });
    r.tag(line, "rocr");

    r.kind = "rocr";
    r.options = options.to_vec();
    r.outputs.push(Sink::iface_lead(line, period));
    r
}

/// `QSTICK(p)` — the hand's `State(SmaState)` fed `(close - open, prev_close -
/// prev_open)`, and `SmaState::calc((incoming, outgoing))` is
/// `sum += incoming - outgoing; sum * (1/p)`. So the WINDOW SERIES is `close − open`
/// and the second tuple element is the bar leaving the ring, NOT a second operand to
/// difference — the ring supplies it, so no `Lag` node is needed. Hand `INPUTS` is
/// `["open","close"]`, so the arena declares exactly those two columns: `oc()` →
/// open=0, close=1. Streams: 0=open, 1=close. Options `[period]`. Publish lead =
/// `period` (hand `cycle_qstick` runs `i` from `period`; `optional_outputs` is empty).
pub fn qstick(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::oc();
    let open = r.add(Node::Input { stream: 0 });
    r.tag(open, "open");
    let close = r.add(Node::Input { stream: 1 });
    r.tag(close, "close");
    let diff = r.add(Node::Sub { a: close, b: open });
    r.tag(diff, "co_diff");
    let sma = r.add(Node::RollSum {
        src: diff,
        period,
        mean: true,
        skip: 0,
        steady_from: 0,
    });
    r.tag(sma, "qstick");

    r.kind = "qstick";
    r.options = options.to_vec();
    // The carried state is SmaState's `sum`, which the ring owns.
    r.tail_inputs = vec![sma];
    r.outputs.push(Sink::iface_lead(sma, period));
    r
}

/// `DPO(p)` = `real[i − (p/2 + 1)] − SMA(real, p)`. The hand's
/// `calc((value, prev_value, dpo_price))` calls `SmaState::calc((value, prev_value))`
/// with `value = real[i]` and `prev_value = real[i − p]` — the INCOMING and OUTGOING
/// bars of the SAME series — so the window is over `real`, not over the difference
/// series (that was an earlier misreading of this recipe). `dpo_period = p/2 + 1` is
/// the hand's own derivation (`dpo.rs`).
///
/// THE ROLL IS `Node::Sma`, NOT `RollSum{mean:true}`: its source is a raw price
/// Input, which is precisely what `Node::Sma` (which reads `stream[t-p]` directly,
/// the form `sma()` uses) supports. The two are DIFFERENT signature kinds
/// (`K_SMA=16` vs `K_ROLLSUM=24`), so spelling it as a ring roll would make a
/// co-requested `Sma(p) + Dpo(p)` compute the same rolling mean TWICE instead of
/// interning one node — see the `sma_dpo_shared_p` gate. Streams: 0=real. Options
/// `[period]`. Primary lead = `period`; `Info.optional_outputs = ["sma"]`, lead
/// `period`.
pub fn dpo(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let dpo_period = (period / 2 + 1) as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    let sma = r.add(Node::Sma { src: real, period });
    r.tag(sma, "sma");
    let dpo_price = r.add(Node::Lag {
        src: real,
        n: dpo_period,
    });
    r.tag(dpo_price, "dpo_price");
    let line = r.add(Node::Sub {
        a: dpo_price,
        b: sma,
    });
    r.tag(line, "dpo");

    r.kind = "dpo";
    r.options = options.to_vec();
    r.tail_inputs = vec![sma];
    r.outputs.push(Sink::iface_lead(line, period));
    r.optional_outputs.push(Sink::iface_lead(sma, period));
    r
}

/// `DEMA(p)` — hand `dema.rs::calc`: `ema1 = Ema(real)`, `ema2 = Ema(ema1)`, output
/// `ema1.mul_add(2.0, -ema2)`. ONE fused op, so this spells `Node::Fma` — `Mul` then
/// `Sub` would double-round. The negation is `Mul(ema2, -1.0)`, which is EXACT and
/// preserves the sign of zero (a `Sub{0.0, ema2}` would turn `-0.0` into `+0.0`).
///
/// Publish lead `2p-2` is NOT a pattern copied from `ema()`: it is
/// `min_data - 1` with `min_data = 2p-1` (`dema.rs:147`) under the trait's default
/// `output_length = len - min_data + 1` (`indicator_types.rs:28`), which `dema` does
/// not override. It is also exactly the defined-from of the SECOND chained EMA
/// (`ema1` at `p-1`, `ema2` at `2(p-1)`), so the stacked lead is structural, not a
/// magic number. The optional `ema` row keeps the hand's own `Ema::output_length`
/// lead (`p`), matching `dema.rs`'s separate `ema_capacity`.
pub fn dema(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    let ema1 = r.add(Node::Ema { src: real, period });
    r.tag(ema1, "ema");
    let ema2 = r.add(Node::Ema { src: ema1, period });
    r.tag(ema2, "ema2");
    let two = r.constant(2.0);
    let neg_one = r.constant(-1.0);
    let neg_ema2 = r.add(Node::Mul {
        a: ema2,
        b: neg_one,
    });
    let line = r.add(Node::Fma {
        a: ema1,
        b: two,
        c: neg_ema2,
    });
    r.tag(line, "dema");

    r.kind = "dema";
    r.options = options.to_vec();
    r.tail_inputs = vec![ema1, ema2];
    r.outputs.push(Sink::iface_lead(line, 2 * period - 2));
    // Info optional order: ["ema"]
    r.optional_outputs.push(Sink::iface_lead(ema1, period));
    r
}

/// `TEMA(p)` — hand `tema.rs::calc`: it OWNS a `DemaState` (so `ema1`, `ema2` are the
/// DEMA chains) plus `ema3 = Ema(ema2)`, and outputs
/// `ema1.mul_add(3.0, ema2.mul_add(-3.0, ema3))` — TWO nested single-rounding fmas.
/// Publish lead `3p-3` = `min_data - 1` (`min_data = 3p-2`, `tema.rs:174`) and the
/// defined-from of the third chain. Optionals are `["dema", "ema"]` in Info order,
/// the `dema` row at the DEMA lead and the `ema` row at the Ema lead.
///
/// THIS IS THE SHARING CASE: because tema's `ema1`/`ema2` are the very nodes `dema`
/// builds, a co-requested `Dema(p) + Tema(p)` interns the whole two-chain subgraph --
/// and tema's `dema` optional row is the same stored stream as dema's primary.
pub fn tema(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    let ema1 = r.add(Node::Ema { src: real, period });
    let ema2 = r.add(Node::Ema { src: ema1, period });
    let ema3 = r.add(Node::Ema { src: ema2, period });
    r.tag(ema3, "ema3");
    let three = r.constant(3.0);
    let neg_three = r.constant(-3.0);
    let inner = r.add(Node::Fma {
        a: ema2,
        b: neg_three,
        c: ema3,
    });
    let line = r.add(Node::Fma {
        a: ema1,
        b: three,
        c: inner,
    });
    r.tag(line, "tema");
    let dema_two = r.constant(2.0);
    let neg_one = r.constant(-1.0);
    let neg_ema2 = r.add(Node::Mul {
        a: ema2,
        b: neg_one,
    });
    let dema_line = r.add(Node::Fma {
        a: ema1,
        b: dema_two,
        c: neg_ema2,
    });
    r.tag(dema_line, "dema");

    r.kind = "tema";
    r.options = options.to_vec();
    r.tail_inputs = vec![ema1, ema2, ema3];
    r.outputs.push(Sink::iface_lead(line, 3 * period - 3));
    r.optional_outputs
        .push(Sink::iface_lead(dema_line, 2 * period - 2));
    r.optional_outputs.push(Sink::iface_lead(ema1, period));
    r
}

/// `ZLEMA(p)` — hand `zlema.rs::calc((current, lagged))`:
/// `adjusted = current + (current - lagged)`, then `zlema.mul_add(per, adjusted *
/// multiplier)` -- which IS the EMA recurrence, so this is one `Node::Ema` over a
/// DERIVED source (`macd`'s signal EMA is the same shape). `lagged = real[i - lag]`
/// with `lag = ((p-1)/2).max(1)` (`zlema.rs:132`), a Lag depth other than 1 or p,
/// read by a published sink.
///
/// Publish lead = `lag`: `min_data = (p-1)/2 + 1` under the default `output_length`,
/// and the hand's `cycle` runs `i` from `lag`.
///

/// `ZLEMA(p)` seeds its EMA from `real[lag-1]` (`zlema.rs:42`, `lag = ((p-1)/2).max(1)`),
/// which the engine spells as `Node::SeededEma{seed: real, src: adjusted, period, feed: Some(lag-1)}`
/// — `seed = real` (the raw input, read via the explicit `feed` override at bar `lag-1`
/// instead of its natural lead 0), `src = adjusted` (`2*real[i] - real[i-lag]`, the fold
/// value from bar `lag` on). Publish lead = `lag` (`Sink::iface_lead(line, lag)`),
/// matching the hand's `cycle` loop starting at `i = lag`.
pub fn zlema(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let lag = (((period as usize).saturating_sub(1)) / 2).max(1) as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    let lagged = r.add(Node::Lag { src: real, n: lag });
    r.tag(lagged, "lagged");
    let diff = r.add(Node::Sub { a: real, b: lagged });
    let adjusted = r.add(Node::Add { a: real, b: diff });
    r.tag(adjusted, "adjusted");
    let line = r.add(Node::SeededEma {
        seed: real,
        src: adjusted,
        period,
        feed: Some(lag.saturating_sub(1)),
    });
    r.tag(line, "zlema");

    r.kind = "zlema";
    r.options = options.to_vec();
    r.tail_inputs = vec![line];
    r.outputs.push(Sink::iface_lead(line, lag));
    r
}

/// `TRIX(p)` -- hand `trix.rs`: it owns a `TemaState` (the same `ema1, ema2,
/// ema3` chain `tema` builds) and outputs
/// `100.0 * (self.ema3 - prev_ema3) / self.ema3`, where `prev_ema3` is the
/// chain's OWN value from one bar back -- `Lag{ema3, 1}` -- and `self.ema3` is
/// the just-updated value. Rust's same-precedence left-to-right evaluation
/// makes this `(100.0 * (ema3 - prev_ema3)) / ema3`, NOT a reassociated form.
/// Streams: 0=real. Options `[period]`. Publish lead = `3p-2`
/// (`min_data = (p-1)*3+2`, `trix.rs:173`), one bar past ema3's own lead
/// `3(p-1)` -- the `Lag{ema3,1}` dominates. Optionals in Info order
/// `["tema","dema","ema"]`, the SAME nodes `tema`/`dema` themselves declare
/// (shared by CSE key in a co-requested merge).
pub fn trix(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    let ema1 = r.add(Node::Ema { src: real, period });
    let ema2 = r.add(Node::Ema { src: ema1, period });
    let ema3 = r.add(Node::Ema { src: ema2, period });
    r.tag(ema3, "ema3");
    let prev_ema3 = r.add(Node::RollAge { src: ema3, n: 1 });
    r.tag(prev_ema3, "prev_ema3");
    let diff = r.add(Node::Sub {
        a: ema3,
        b: prev_ema3,
    });
    r.tag(diff, "ema3_diff");
    let hundred = r.constant(100.0);
    r.tag(hundred, "hundred");
    let scaled = r.add(Node::Mul {
        a: hundred,
        b: diff,
    });
    r.tag(scaled, "scaled");
    let line = r.add(Node::Div { a: scaled, b: ema3 });
    r.tag(line, "trix");

    let dema_two = r.constant(2.0);
    let neg_one = r.constant(-1.0);
    let neg_ema2 = r.add(Node::Mul {
        a: ema2,
        b: neg_one,
    });
    let dema_line = r.add(Node::Fma {
        a: ema1,
        b: dema_two,
        c: neg_ema2,
    });
    r.tag(dema_line, "dema");
    let three = r.constant(3.0);
    let neg_three = r.constant(-3.0);
    let inner = r.add(Node::Fma {
        a: ema2,
        b: neg_three,
        c: ema3,
    });
    let tema_line = r.add(Node::Fma {
        a: ema1,
        b: three,
        c: inner,
    });
    r.tag(tema_line, "tema");

    r.kind = "trix";
    r.options = options.to_vec();
    r.tail_inputs = vec![ema1, ema2, ema3];
    r.outputs.push(Sink::iface_lead(line, 3 * period - 2));
    // Info optional order: ["tema", "dema", "ema"]
    r.optional_outputs
        .push(Sink::iface_lead(tema_line, 3 * period - 3));
    r.optional_outputs
        .push(Sink::iface_lead(dema_line, 2 * period - 2));
    r.optional_outputs.push(Sink::iface_lead(ema1, period));
    r
}

pub fn kama(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");

    // Reuse the already-landed `recipes::ef` subgraph for efficiency ratio.
    let ef_graph = r.sub_graph(ef(&[period as f64]), &[0]);
    let ef_id = ef_graph.outputs[0];

    // fast_ema = ema_multiplier(2).0, slow_ema = ema_multiplier(30).0 -- computed at recipe-build time
    let (fast_ema, slow_ema) = super::kama_multiplier();
    let fast_ema_node = r.constant(fast_ema);
    let slow_ema_node = r.constant(slow_ema);

    // w = smoothing_constant.powi(2) where smoothing_constant = (fast_ema - slow_ema).mul_add(ef, slow_ema)
    // First: fast_ema - slow_ema
    let diff = r.add(Node::Sub {
        a: fast_ema_node,
        b: slow_ema_node,
    });
    r.tag(diff, "diff");
    // Then: diff * ef + slow_ema (Fma = a.mul_add(b, c) = a*b + c)
    let smoothing_constant = r.add(Node::Fma {
        a: diff,
        b: ef_id,
        c: slow_ema_node,
    });
    r.tag(smoothing_constant, "smoothing_constant");
    // Then: smoothing_constant^2 (multiply by itself)
    let w_squared = r.add(Node::Mul {
        a: smoothing_constant,
        b: smoothing_constant,
    });
    r.tag(w_squared, "w_squared");

    // kama_node = VariableAlpha with Ema shape
    // seed: real (read at feed bar), src: real (folded in each bar after), mult: w_squared
    // feed: Some(period - 1) (kama seeds from real[period-1])
    let kama_node = r.add(Node::VariableAlpha {
        seed: real,
        src: real,
        mult: w_squared,
        feed: Some(period - 1),
        shape: super::AlphaShape::Ema,
    });
    r.tag(kama_node, "kama");

    r.kind = "kama";
    r.options = options.to_vec();
    // Sink with lead == period (output starts at bar `period`)
    // The VariableAlpha leads() formula gives base+1 where base=period-1, so lead=period
    r.outputs.push(Sink::iface_lead(kama_node, period));

    // Optionally expose ef's own output as a secondary sink
    let ef_out = ef_graph.primary;
    r.optional_outputs.push(Sink::iface(ef_out));

    r
}

pub fn vidya(options: &[f64]) -> Recipe {
    let short_period = options[0] as u32;
    let long_period = options[1] as u32;
    let alpha = options[2];
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");

    // Build two Node::Stddev subgraphs (short_period, long_period) over `real`.
    // Copy the exact pattern from bbands recipe for stddev construction.
    let short_stddev = super::stddev_recipe(short_period, None, Some(long_period));
    let short_graph = r.sub_graph(short_stddev, &[0]);
    let short_sd_id = short_graph.outputs[0];

    let long_stddev = super::stddev_recipe(long_period, Some(long_period - 1), None);
    let long_graph = r.sub_graph(long_stddev, &[0]);
    let long_sd_id = long_graph.outputs[0];

    // w = (short_stddev / long_stddev) * alpha, guarded the same way the
    // hand does: `long_stddev.abs() < f64::EPSILON` (stddev is always >= 0,
    // so abs() is a no-op) selects 0.0 instead of dividing -- WhereLt{a:
    // long_sd, b: eps, t: ratio} is `if a < b {0.0} else {t}` (K_WHERELT),
    // the exact polarity of the hand's `if long_stddev.abs() < EPS {0.0}
    // else {ratio}`.
    let ratio = r.add(Node::Div {
        a: short_sd_id,
        b: long_sd_id,
    });
    r.tag(ratio, "ratio");
    let eps = r.constant(f64::EPSILON);
    let guarded_ratio = r.add(Node::WhereLt {
        a: long_sd_id,
        b: eps,
        t: ratio,
    });
    r.tag(guarded_ratio, "guarded_ratio");
    let alpha_node = r.constant(alpha);
    let w = r.add(Node::Mul {
        a: guarded_ratio,
        b: alpha_node,
    });
    r.tag(w, "w");

    // vidya_node = VariableAlpha with Vidya shape
    // seed: real (read at feed bar), src: real (folded in each bar after), mult: w
    // feed: Some(long_period - 2) (vidya seeds from real[long_period-2])
    let vidya_node = r.add(Node::VariableAlpha {
        seed: real,
        src: real,
        mult: w,
        feed: Some(long_period - 2),
        shape: super::AlphaShape::Vidya,
    });
    r.tag(vidya_node, "vidya");

    r.kind = "vidya";
    r.options = options.to_vec();
    // Sink with lead == long_period - 1 (output starts at bar `long_period-1`)
    // The VariableAlpha leads() formula gives base+1 where base=long_period-2, so lead=long_period-1
    r.outputs
        .push(Sink::iface_lead(vidya_node, long_period - 1));

    // Optional outputs, in the hand's row order (`batch_indicator`'s
    // `Ok(vec![vidya_line, short_sma_line, long_sma_line, short_sd_line,
    // long_sd_line])`): short_sma, long_sma, short_stddev, long_stddev.
    let short_sma = short_graph.optional_at(0);
    let long_sma = long_graph.optional_at(0);
    // Explicit `iface_lead` (not plain `iface`, whose lead=0 is a sentinel
    // meaning "use the node's bare structural lead") -- the stddev recipe
    // deliberately declares lead=period (one past the structural minimum,
    // `StdDev::min_data == period+1` in the hand), and `iface()` would silently
    // discard that +1 convention, publishing one bar too early.
    r.optional_outputs
        .push(Sink::iface_lead(short_sma, short_period));
    r.optional_outputs
        .push(Sink::iface_lead(long_sma, long_period));
    r.optional_outputs
        .push(Sink::iface_lead(short_sd_id, short_period));
    r.optional_outputs
        .push(Sink::iface_lead(long_sd_id, long_period));

    r
}

/// `EF(p)` -- Efficiency Ratio, hand `ef.rs`: `|real[i]-real[i-p]| / sum`,
/// where `sum` is a ring-backed rolling sum of `|real[k]-real[k-1]|` over the
/// last `p` terms. `sum` is a sum of absolute values so it can never be
/// negative, which is what licenses `WhereLe{ sum <= 0 }` to stand in for the
/// hand's literal `sum != 0.0` guard with no NaN-semantics gap (same
/// reasoning as `cci`'s `md`-guard, which is also a non-negative magnitude).
/// Hand `min_data = p+1`, first published row at index `p` (0-indexed) --
/// `init_state` computes `ef_line[0]` straight from `real[0..=p]`, no extra
/// lead beyond the rolling-sum window itself.
pub fn ef(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "price");
    let prev = r.add(Node::Lag { src: real, n: 1 });
    let step = r.add(Node::Sub { a: real, b: prev });
    let step_abs = r.add(Node::Abs { src: step });
    r.tag(step_abs, "step_abs");
    let sum = r.add(Node::RollSum {
        src: step_abs,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(sum, "sum");
    let old = r.add(Node::Lag {
        src: real,
        n: period,
    });
    let num_raw = r.add(Node::Sub { a: real, b: old });
    let num = r.add(Node::Abs { src: num_raw });
    let ratio = r.add(Node::Div { a: num, b: sum });
    let zero = r.constant(0.0);
    let ef = r.add(Node::WhereLe {
        a: sum,
        b: zero,
        t: ratio,
    });
    r.tag(ef, "ef");
    r.kind = "ef";
    r.options = options.to_vec();
    // `ef::State<S>` is `{sum, prev, drop}` — `sum` is the `RollSum` node
    // above (already `ChainState::Sma`); `prev`/`drop` are the hand's two
    // scalar retentions (the newest and the about-to-drop `real` values),
    // captured the same way `atr()`'s `Mirror{src: close}` captures
    // `tr_state`'s `prev_close`: wrap the existing `prev`/`old` `Lag` nodes
    // in a fresh `Node::Mirror`, no new `ChainState` variant needed.
    let prev_mirror = r.add(Node::Mirror { src: prev });
    r.tag(prev_mirror, "prev_mirror");
    let old_mirror = r.add(Node::Mirror { src: old });
    r.tag(old_mirror, "old_mirror");
    r.tail_inputs = vec![sum, prev_mirror, old_mirror];
    r.outputs.push(Sink::iface_lead(ef, period));
    r
}
