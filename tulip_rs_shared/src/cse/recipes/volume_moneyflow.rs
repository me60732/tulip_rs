use crate::cse::{Id, Node, Recipe, Sink};

pub fn emv(_options: &[f64]) -> Recipe {
    let mut r = Recipe::hlvol();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let volume = r.add(Node::Input { stream: 2 });
    r.tag(volume, "volume");
    let sum = r.add(Node::Add { a: high, b: low });
    let half = r.constant(0.5);
    let med = r.add(Node::Mul { a: half, b: sum });
    r.tag(med, "medprice");
    let prev = r.add(Node::RollAge { src: med, n: 1 });
    r.tag(prev, "prev_medprice");
    let moved = r.add(Node::Sub { a: med, b: prev });
    let range = r.add(Node::Sub { a: high, b: low });
    let eps = r.constant(f64::EPSILON);
    let range = r.add(Node::Max { a: range, b: eps });
    let scale = r.constant(10000.0);
    let scaled = r.add(Node::Mul { a: moved, b: scale });
    let scaled = r.add(Node::Mul {
        a: scaled,
        b: range,
    });
    let volume = r.add(Node::Max { a: volume, b: eps });
    let line = r.add(Node::Div {
        a: scaled,
        b: volume,
    });
    r.tag(line, "emv");

    r.kind = "emv";
    r.options = Vec::new();
    r.tail_inputs = vec![prev];
    r.outputs.push(Sink::iface_lead(line, 1));
    r
}

pub(crate) fn ad_nodes(r: &mut Recipe, hi: Id, lo: Id, cl: Id, vol: Id) -> Id {
    let range = r.add(Node::Sub { a: hi, b: lo });
    r.tag(range, "hl_range");
    let cl_lo = r.add(Node::Sub { a: cl, b: lo });
    let minus_hi = r.add(Node::Sub { a: cl_lo, b: hi });
    let numer = r.add(Node::Add { a: minus_hi, b: cl });
    r.tag(numer, "clv_numer");
    let ratio = r.add(Node::Div { a: numer, b: range });
    r.tag(ratio, "clv");
    let eps = r.constant(f64::EPSILON);
    let gated = r.add(Node::WhereLe {
        a: range,
        b: eps,
        t: ratio,
    });
    r.tag(gated, "clv_gated");
    let line = r.add(Node::Accum { a: gated, b: vol });
    r.tag(line, "ad");
    line
}

pub fn ad(_options: &[f64]) -> Recipe {
    let mut r = Recipe::hlcvol();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let cl = r.add(Node::Input { stream: 2 });
    r.tag(cl, "close");
    let vol = r.add(Node::Input { stream: 3 });
    r.tag(vol, "volume");

    let line = ad_nodes(&mut r, hi, lo, cl, vol);

    r.kind = "ad";
    r.options = Vec::new();
    let mirror = r.add(Node::Mirror { src: line });
    r.tag(mirror, "ad_mirror");
    r.tail_inputs = vec![mirror];
    r.outputs.push(Sink::iface(line));
    r
}

pub fn adosc(options: &[f64]) -> Recipe {
    let short = options[0] as u32;
    let long = options[1] as u32;
    let mut r = Recipe::hlcvol();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let cl = r.add(Node::Input { stream: 2 });
    r.tag(cl, "close");
    let vol = r.add(Node::Input { stream: 3 });
    r.tag(vol, "volume");

    let ad = ad_nodes(&mut r, hi, lo, cl, vol);
    let ema_short = r.add(Node::Ema {
        src: ad,
        period: short,
    });
    r.tag(ema_short, "ema_short");
    let ema_long = r.add(Node::Ema {
        src: ad,
        period: long,
    });
    r.tag(ema_long, "ema_long");
    let osc = r.add(Node::Sub {
        a: ema_short,
        b: ema_long,
    });
    r.tag(osc, "adosc");

    r.kind = "adosc";
    r.options = options.to_vec();
    let ad_mirror = r.add(Node::Mirror { src: ad });
    r.tag(ad_mirror, "ad_mirror");
    r.tail_inputs = vec![ema_short, ema_long, ad_mirror];
    r.outputs.push(Sink::iface_lead(osc, long - 1));
    r.optional_outputs = vec![
        Sink::iface_lead(ema_short, short),
        Sink::iface_lead(ema_long, long - 1),
        Sink::iface(ad),
    ];
    r
}

pub fn marketfi(_options: &[f64]) -> Recipe {
    let mut r = Recipe::hlvol();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let vol = r.add(Node::Input { stream: 2 });
    r.tag(vol, "volume");

    let range = r.add(Node::Sub { a: hi, b: lo });
    r.tag(range, "hl_range");
    let eps = r.constant(f64::EPSILON);
    let den = r.add(Node::Max { a: vol, b: eps });
    r.tag(den, "vol_floor");
    let line = r.add(Node::Div { a: range, b: den });
    r.tag(line, "marketfi");

    r.kind = "marketfi";
    r.options = Vec::new();
    r.outputs.push(Sink::iface(line));
    r
}

pub fn vwma(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::cvol();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "close");
    let volume = r.add(Node::Input { stream: 1 });
    r.tag(volume, "volume");
    let close_vol = r.add(Node::Mul {
        a: close,
        b: volume,
    });
    r.tag(close_vol, "close_vol");
    let sum = r.add(Node::RollSum {
        src: close_vol,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(sum, "pv_sum");
    let vol_sum = r.add(Node::RollSum {
        src: volume,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(vol_sum, "vol_sum");
    let ratio = r.add(Node::Div { a: sum, b: vol_sum });
    r.tag(ratio, "vwma_ratio");
    let zero = r.constant(0.0);
    let mag = r.add(Node::Abs { src: vol_sum });
    r.tag(mag, "vol_sum_abs");
    let line = r.add(Node::WhereLe {
        a: mag,
        b: zero,
        t: ratio,
    });
    r.tag(line, "vwma");

    r.kind = "vwma";
    r.options = options.to_vec();
    r.tail_inputs = vec![sum, vol_sum];
    r.outputs.push(Sink::iface_lead(line, period));
    r
}

pub fn chaikinmf(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hlcvol();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let cl = r.add(Node::Input { stream: 2 });
    r.tag(cl, "close");
    let vol = r.add(Node::Input { stream: 3 });
    r.tag(vol, "volume");

    let range = r.add(Node::Sub { a: hi, b: lo });
    r.tag(range, "hl_range");
    let eps = r.constant(f64::EPSILON);
    let den = r.add(Node::Max { a: range, b: eps });
    r.tag(den, "hl_range_floored");
    let cl_lo = r.add(Node::Sub { a: cl, b: lo });
    let hi_cl = r.add(Node::Sub { a: hi, b: cl });
    let numer = r.add(Node::Sub { a: cl_lo, b: hi_cl });
    r.tag(numer, "mfv_numer");
    let clv = r.add(Node::Div { a: numer, b: den });
    r.tag(clv, "clv");
    let mfv = r.add(Node::Mul { a: clv, b: vol });
    r.tag(mfv, "mfv");

    let num_sum = r.add(Node::RollSum {
        src: mfv,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(num_sum, "mfv_sum");
    let vol_sum = r.add(Node::RollSum {
        src: vol,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(vol_sum, "vol_sum");

    let line = r.add(Node::Div {
        a: num_sum,
        b: vol_sum,
    });
    r.tag(line, "chaikinmf");

    r.kind = "chaikinmf";
    r.options = options.to_vec();
    r.tail_inputs = vec![num_sum, vol_sum];
    r.outputs.push(Sink::iface_lead(line, period));
    r
}

pub fn vwap(_options: &[f64]) -> Recipe {
    let mut r = Recipe::hlcvol();
    let hi = r.add(Node::Input { stream: 0 });
    r.tag(hi, "high");
    let lo = r.add(Node::Input { stream: 1 });
    r.tag(lo, "low");
    let cl = r.add(Node::Input { stream: 2 });
    r.tag(cl, "close");
    let vol = r.add(Node::Input { stream: 3 });
    r.tag(vol, "volume");

    let sum = r.add(Node::Add { a: hi, b: lo });
    let sum2 = r.add(Node::Add { a: sum, b: cl });
    let third = r.constant(1.0 / 3.0);
    r.tag(third, "third");
    let tp = r.add(Node::Mul { a: sum2, b: third });
    r.tag(tp, "typprice");

    let one = r.constant(1.0);
    let product = r.add(Node::Mul { a: tp, b: vol });
    r.tag(product, "tp_vol");
    let pv_sum = r.add(Node::Accum { a: product, b: one });
    r.tag(pv_sum, "pv_sum");
    let vol_sum = r.add(Node::Accum { a: vol, b: one });
    r.tag(vol_sum, "vol_sum");
    let line = r.add(Node::Div {
        a: pv_sum,
        b: vol_sum,
    });
    r.tag(line, "vwap");

    r.kind = "vwap";
    r.options = Vec::new();
    r.tail_inputs = vec![pv_sum, vol_sum];
    r.outputs.push(Sink::iface(line));
    r.optional_outputs.push(Sink::iface(tp));
    r
}

pub fn obv(_options: &[f64]) -> Recipe {
    let mut r = Recipe::cvol();
    let cl = r.add(Node::Input { stream: 0 });
    r.tag(cl, "close");
    let vol = r.add(Node::Input { stream: 1 });
    r.tag(vol, "volume");
    let prev = r.add(Node::Lag { src: cl, n: 1 });
    r.tag(prev, "prev_close");
    let up = r.add(Node::WhereGt {
        a: cl,
        b: prev,
        t: vol,
    });
    r.tag(up, "up_vol");
    let down = r.add(Node::WhereGt {
        a: prev,
        b: cl,
        t: vol,
    });
    r.tag(down, "down_vol");
    let delta = r.add(Node::Sub { a: up, b: down });
    r.tag(delta, "obv_delta");
    let one = r.constant(1.0);
    let line = r.add(Node::Accum { a: delta, b: one });
    r.tag(line, "obv");

    r.kind = "obv";
    r.options = Vec::new();
    r.tail_inputs = vec![line];
    r.outputs.push(Sink::iface(line));
    r
}

pub fn wad(_options: &[f64]) -> Recipe {
    let mut r = Recipe::hlc();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let cl = r.add(Node::Input { stream: 2 });
    r.tag(cl, "close");
    let prev = r.add(Node::Lag { src: cl, n: 1 });
    r.tag(prev, "prev_close");
    let lower = r.add(Node::Min { a: prev, b: low });
    let upper = r.add(Node::Max { a: prev, b: high });
    let up_move = r.add(Node::Sub { a: cl, b: lower });
    let down_move = r.add(Node::Sub { a: cl, b: upper });
    let up = r.add(Node::WhereGt {
        a: cl,
        b: prev,
        t: up_move,
    });
    r.tag(up, "up_move");
    let down = r.add(Node::WhereGt {
        a: prev,
        b: cl,
        t: down_move,
    });
    r.tag(down, "down_move");
    let delta = r.add(Node::Add { a: up, b: down });
    r.tag(delta, "wad_delta");
    let one = r.constant(1.0);
    let line = r.add(Node::Accum { a: delta, b: one });
    r.tag(line, "wad");

    r.kind = "wad";
    r.options = Vec::new();
    r.tail_inputs = vec![line];
    r.outputs.push(Sink::iface(line));
    r
}

pub fn mfi(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::hlcvol();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let close = r.add(Node::Input { stream: 2 });
    r.tag(close, "close");
    let volume = r.add(Node::Input { stream: 3 });
    r.tag(volume, "volume");
    let sum = r.add(Node::Add { a: high, b: low });
    let sum = r.add(Node::Add { a: sum, b: close });
    let third = r.constant(1.0 / 3.0);
    let typprice = r.add(Node::Mul { a: sum, b: third });
    r.tag(typprice, "typprice");
    let prev = r.add(Node::RollAge {
        src: typprice,
        n: 1,
    });
    r.tag(prev, "prev_typprice");
    let change = r.add(Node::Sub {
        a: typprice,
        b: prev,
    });
    let flow = r.add(Node::Mul {
        a: typprice,
        b: volume,
    });
    let zero = r.constant(0.0);
    let pos = r.add(Node::WhereGt {
        a: change,
        b: zero,
        t: flow,
    });
    r.tag(pos, "pos_flow");
    let neg = r.add(Node::WhereGt {
        a: zero,
        b: change,
        t: flow,
    });
    r.tag(neg, "neg_flow");
    let pos_sum = r.add(Node::RollSum {
        src: pos,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(pos_sum, "pos_sum");
    let neg_sum = r.add(Node::RollSum {
        src: neg,
        period,
        mean: false,
        skip: 0,
        steady_from: 0,
    });
    r.tag(neg_sum, "neg_sum");
    let total = r.add(Node::Add {
        a: pos_sum,
        b: neg_sum,
    });
    let ratio = r.add(Node::Div {
        a: pos_sum,
        b: total,
    });
    let hundred = r.constant(100.0);
    let scaled = r.add(Node::Mul {
        a: ratio,
        b: hundred,
    });
    let eps = r.constant(1e-14);
    let line = r.add(Node::WhereLt {
        a: total,
        b: eps,
        t: scaled,
    });
    r.tag(line, "mfi");

    r.kind = "mfi";
    r.options = options.to_vec();
    r.tail_inputs = vec![prev, pos_sum, neg_sum];
    r.outputs.push(Sink::iface_lead(line, period));
    r
}

pub fn vosc(options: &[f64]) -> Recipe {
    let short_period = options[0] as u32;
    let long_period = options[1] as u32;
    let mut r = Recipe::vol();
    let volume = r.add(Node::Input { stream: 0 });
    r.tag(volume, "volume");
    let fast_sma = r.add(Node::RollSum {
        src: volume,
        period: short_period,
        mean: true,
        skip: 0,
        steady_from: 0,
    });
    r.tag(fast_sma, "short_sma");
    let slow_sma = r.add(Node::RollSum {
        src: volume,
        period: long_period,
        mean: true,
        skip: 0,
        steady_from: 0,
    });
    r.tag(slow_sma, "long_sma");
    let diff = r.add(Node::Sub {
        a: fast_sma,
        b: slow_sma,
    });
    r.tag(diff, "diff");
    let hundred = r.constant(100.0);
    r.tag(hundred, "hundred");
    let scaled = r.add(Node::Mul {
        a: diff,
        b: hundred,
    });
    r.tag(scaled, "scaled");
    let ratio = r.add(Node::Div {
        a: scaled,
        b: slow_sma,
    });
    r.tag(ratio, "vosc_ratio");
    let zero = r.constant(0.0);
    let mag = r.add(Node::Abs { src: slow_sma });
    r.tag(mag, "slow_abs");
    let line = r.add(Node::WhereLe {
        a: mag,
        b: zero,
        t: ratio,
    });
    r.tag(line, "vosc");

    r.kind = "vosc";
    r.options = options.to_vec();
    r.tail_inputs = vec![fast_sma, slow_sma];
    r.outputs.push(Sink::iface_lead(line, long_period));
    r.optional_outputs
        .push(Sink::iface_lead(fast_sma, short_period));
    r.optional_outputs
        .push(Sink::iface_lead(slow_sma, long_period));
    r
}

pub fn nvi(_options: &[f64]) -> Recipe {
    let mut r = Recipe::cvol();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "close");
    let volume = r.add(Node::Input { stream: 1 });
    r.tag(volume, "volume");
    let prev_close = r.add(Node::Lag { src: close, n: 1 });
    r.tag(prev_close, "prev_close");
    let prev_volume = r.add(Node::Lag { src: volume, n: 1 });
    r.tag(prev_volume, "prev_volume");
    let line = r.add(Node::NviRecur {
        src: close,
        prev_close,
        volume,
        prev_volume,
    });
    r.tag(line, "nvi");

    r.kind = "nvi";
    r.options = Vec::new();
    let nvi_mirror = r.add(Node::Mirror { src: line });
    r.tag(nvi_mirror, "nvi_mirror");
    let close_mirror = r.add(Node::Mirror { src: close });
    r.tag(close_mirror, "close_mirror");
    let volume_mirror = r.add(Node::Mirror { src: volume });
    r.tag(volume_mirror, "volume_mirror");
    r.tail_inputs = vec![nvi_mirror, close_mirror, volume_mirror];
    r.outputs.push(Sink::iface(line));
    r
}

pub fn pvi(_options: &[f64]) -> Recipe {
    let mut r = Recipe::cvol();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "close");
    let volume = r.add(Node::Input { stream: 1 });
    r.tag(volume, "volume");
    let prev_close = r.add(Node::Lag { src: close, n: 1 });
    r.tag(prev_close, "prev_close");
    let prev_volume = r.add(Node::Lag { src: volume, n: 1 });
    r.tag(prev_volume, "prev_volume");
    let line = r.add(Node::PviRecur {
        src: close,
        prev_close,
        volume,
        prev_volume,
    });
    r.tag(line, "pvi");

    r.kind = "pvi";
    r.options = Vec::new();
    let pvi_mirror = r.add(Node::Mirror { src: line });
    r.tag(pvi_mirror, "pvi_mirror");
    let close_mirror = r.add(Node::Mirror { src: close });
    r.tag(close_mirror, "close_mirror");
    let volume_mirror = r.add(Node::Mirror { src: volume });
    r.tag(volume_mirror, "volume_mirror");
    r.tail_inputs = vec![pvi_mirror, close_mirror, volume_mirror];
    r.outputs.push(Sink::iface(line));
    r
}

pub fn kvo(options: &[f64]) -> Recipe {
    let short_period = options[0] as u32;
    let long_period = options[1] as u32;
    let mut r = Recipe::hlcvol();
    let high = r.add(Node::Input { stream: 0 });
    r.tag(high, "high");
    let low = r.add(Node::Input { stream: 1 });
    r.tag(low, "low");
    let close = r.add(Node::Input { stream: 2 });
    r.tag(close, "close");
    let volume = r.add(Node::Input { stream: 3 });
    r.tag(volume, "volume");
    let vf = r.add(Node::KvoFlow {
        high,
        low,
        close,
        volume,
    });
    r.tag(vf, "vf");
    let short_ema = r.add(Node::Ema {
        src: vf,
        period: short_period,
    });
    r.tag(short_ema, "short_ema");
    let long_ema = r.add(Node::Ema {
        src: vf,
        period: long_period,
    });
    r.tag(long_ema, "long_ema");
    let kvo = r.add(Node::Sub {
        a: short_ema,
        b: long_ema,
    });
    r.tag(kvo, "kvo");
    r.kind = "kvo";
    r.options = options.to_vec();
    r.tail_inputs = vec![short_ema, long_ema, vf];
    r.outputs.push(Sink::iface(kvo));
    r
}
