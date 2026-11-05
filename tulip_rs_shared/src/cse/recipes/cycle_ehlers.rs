use crate::cse::{Id, Node, Recipe, Sink};

/// `msw` = MESA Sine Wave. Single input stream (close/real), one
/// `Node::SdftRotate` node owning the sliding-DFT carry (publishes `sine`),
/// one `Node::SdftLead` companion node reading the cached `lead_sine`. Hand
/// `Info::outputs` order is `["msw_sine", "msw_lead"]`.
pub fn msw(options: &[f64]) -> Recipe {
    let period = options[0] as u32;
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let rot = r.add(Node::SdftRotate { src: close, period });
    r.tag(rot, "sdft_rotate");
    let lead = r.add(Node::SdftLead { owner: rot });
    r.tag(lead, "sdft_lead");
    r.kind = "msw";
    r.options = options.to_vec();
    // The SdftRotate node owns the only real carry state (rp, ip, cached
    // lead_sine); SdftLead has no carry of its own.
    r.tail_inputs = vec![rot];
    let output_lead = period;
    r.outputs.push(Sink::iface_lead(rot, output_lead));
    r.outputs.push(Sink::iface_lead(lead, output_lead));
    r
}

/// Ehlers Homodyne Discriminator -- a single bespoke carry node
/// (`Node::HomodyneDiscriminator`, see its doc for the full per-bar
/// algorithm). `OPTIONS=0`, nothing to key off; the lead is the fixed
/// constant 22 (`leads[close] + 22`, `leads[close] == 0` for a raw input).
pub fn homodynediscriminator(options: &[f64]) -> Recipe {
    let mut r = Recipe::price();
    let close = r.add(Node::Input { stream: 0 });
    r.tag(close, "price");
    let hd = r.add(Node::HomodyneDiscriminator { src: close });
    r.tag(hd, "dc_period");
    r.kind = "homodynediscriminator";
    r.options = options.to_vec();
    r.tail_inputs = vec![hd];
    r.outputs.push(Sink::iface_lead(hd, 22));
    r
}

pub fn instantaneoustrendline(options: &[f64]) -> Recipe {
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    // IT uses the embedded HomodyneDiscriminator pipeline internally.
    // The node is bespoke (K_INSTANTANEOUSTRENDLINE), so it carries its own
    // 3-field `[it_prev, it_prev2, smooth[2]]` in kvacc[k].
    let it = r.add(Node::InstantaneousTrendline { src: real });
    r.tag(it, "trendline");

    r.kind = "instantaneoustrendline";
    r.options = options.to_vec();
    r.tail_inputs = vec![it];
    // First published bar is 22 (min_data=23), so lead = 22.
    r.outputs.push(Sink::iface_lead(it, 22));
    r
}

pub fn mama(options: &[f64]) -> Recipe {
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    // MAMA uses the embedded HomodyneDiscriminator pipeline internally,
    // then applies phase-driven adaptive α to EMA MAMA and FAMA.
    // The node is bespoke (K_MAMA), carrying `[mama_prev, fama_prev, prev_phase]`
    // in kvacc[k]; alpha is written to st[k] for FAMA to consume.
    let fast_limit = options.get(0).copied().unwrap_or(0.5);
    let slow_limit = options.get(1).copied().unwrap_or(0.05);
    let mama_node = r.add(Node::Mama {
        src: real,
        fast_limit_bits: fast_limit.to_bits(),
        slow_limit_bits: slow_limit.to_bits(),
    });
    r.tag(mama_node, "mama");

    // FAMA node reads from Mama's output and alpha stored in Mama's st slot.
    let fama_node = r.add(Node::Fama { src: mama_node });
    r.tag(fama_node, "fama");

    r.kind = "mama";
    r.options = options.to_vec();
    r.tail_inputs = vec![mama_node];
    // First published bar is 22 (min_data=23), so lead = 22.
    // Outputs: mama (primary) and fama (secondary node).
    r.outputs.push(Sink::iface_lead(mama_node, 22));
    r.outputs.push(Sink::iface_lead(fama_node, 22));
    r
}

/// `Hilbert(hp, ss)` = `ht(ss(hp(real)))` split into I and Q — the last
/// indicator-side graph, moved here because the graph is ENGINE data: when the
/// node vocabulary changes this file changes, not the indicator module.
///
/// SSA order mirrors the hand's `TState::calc`: the Roofing Filter advances first
/// (`HighPass` then `SuperSmoother`), then the 7-tap Hilbert kernel reads the
/// `roofing` stream and yields I (centre tap) and Q (FIR sum). The Roofing Filter
/// is DECOMPOSED into two nodes rather than one opaque leaf, so the `highpass`
/// optional has its own SSA id and can share with the standalone `highpass`
/// indicator.
///
/// **REGISTERABLE**: `HighPass`, `SuperSmoother`, `HtInPhase` and
/// `HtQuadrature` all have `erase` arms (`signature.rs`), planner `Kind`
/// mappings (`planner.rs`) and kernel/codegen arms (`warmup.rs::exec`,
/// `codegen.rs`'s `run_fn`/`run_assets_fn`) — see `hilbert_n` in
/// `tulip_rs/src/cse/kernel/registry.rs` and `hilberttransform_bit_exact`
/// in `cse_kernel_registry_test.rs`.
pub fn hilberttransform(options: &[f64]) -> Recipe {
    let ss_period = options[0] as Id;
    let hp_period = options[1] as Id;

    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    let highpass = r.add(Node::HighPass {
        input: real,
        period: hp_period,
    });
    r.tag(highpass, "highpass");
    let roofing = r.add(Node::SuperSmoother {
        src: highpass,
        period: ss_period,
    });
    r.tag(roofing, "roofing");
    let in_phase = r.add(Node::HtInPhase { src: roofing });
    r.tag(in_phase, "in_phase");
    let quadrature = r.add(Node::HtQuadrature { src: roofing });
    r.tag(quadrature, "quadrature");

    r.kind = "hilberttransform";
    r.options = options.to_vec();
    // outputs in Info::outputs order: in_phase, quadrature.
    // optional_outputs in Info::optional_outputs order: roofing, highpass.
    r.outputs.push(Sink::iface(in_phase));
    r.outputs.push(Sink::iface(quadrature));
    r.optional_outputs.push(Sink::iface(roofing));
    r.optional_outputs.push(Sink::iface(highpass));
    r
}

/// `HighPass(period)` — standalone forwarder onto `Node::HighPass`, the same
/// node `hilberttransform` already chains (and gates bit-exact). Streams:
/// 0=real. Options `[period]`. The hand's `State::init_state` runs the
/// ordinary recurrence from bar 0 with a zero carry for `period` bars
/// (discarded), then publishes — `merge.rs::leads`'s `HighPass{input,
/// period} => leads[input].max(period)` already yields `period` for a
/// standalone `Input` source, so the sink needs no override.
pub fn highpass(options: &[f64]) -> Recipe {
    let period = options[0] as Id;

    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    let hp = r.add(Node::HighPass {
        input: real,
        period,
    });
    r.tag(hp, "highpass");

    r.kind = "highpass";
    r.options = options.to_vec();
    r.tail_inputs = vec![hp];
    r.outputs.push(Sink::iface_lead(hp, period));
    r
}

/// `SuperSmoother(period)` — standalone forwarder onto `Node::SuperSmoother`,
/// same node/precedent as `highpass` above. Streams: 0=real. Options
/// `[period]`. Same zero-carry/`period`-bar-warmup shape as `HighPass`, and
/// the same `leads[src].max(period)` rule gives lead = `period` standalone.
pub fn supersmoother(options: &[f64]) -> Recipe {
    let period = options[0] as Id;

    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    let ss = r.add(Node::SuperSmoother { src: real, period });
    r.tag(ss, "supersmoother");

    r.kind = "supersmoother";
    r.options = options.to_vec();
    r.tail_inputs = vec![ss];
    r.outputs.push(Sink::iface_lead(ss, period));
    r
}

/// `RoofingFilter(ss_period, hp_period)` — cascades `Node::HighPass(hp_period)`
/// into `Node::SuperSmoother(ss_period)`, the same two-node chain
/// `hilberttransform` uses for its own first stage. Streams: 0=real. Options
/// `[ss_period, hp_period]` (hand's `Info::options` order). Primary output =
/// `roofing`; optional output = the intermediate `highpass`. The hand warms
/// both filters in lockstep for `max(ss_period, hp_period)` bars, exactly the
/// `SuperSmoother` node's own `leads[src].max(ss_period)` where `src`'s lead is
/// already `hp_period` — no manual override needed for the primary sink.
pub fn roofingfilter(options: &[f64]) -> Recipe {
    let ss_period = options[0] as Id;
    let hp_period = options[1] as Id;

    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    let highpass = r.add(Node::HighPass {
        input: real,
        period: hp_period,
    });
    r.tag(highpass, "highpass");
    let roofing = r.add(Node::SuperSmoother {
        src: highpass,
        period: ss_period,
    });
    r.tag(roofing, "roofing");

    r.kind = "roofingfilter";
    r.options = options.to_vec();
    r.tail_inputs = vec![roofing, highpass];
    // primary lead = max(ss_period, hp_period); the standalone HighPass node's
    // OWN lead is already hp_period (computed by merge.rs::leads), so no manual
    // max() is needed here -- just report each sink at its own node's lead.
    r.outputs
        .push(Sink::iface_lead(roofing, ss_period.max(hp_period)));
    r.optional_outputs
        .push(Sink::iface_lead(highpass, hp_period));
    r
}

/// `Cybercycle(alpha)` — standalone forwarder onto `Node::Cybercycle`.
/// Streams: 0=real. Options `[alpha]`. First published bar is 6 (`min_data=7`).
pub fn cybercycle(options: &[f64]) -> Recipe {
    let alpha_bits = options[0].to_bits();
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");
    let cc = r.add(Node::Cybercycle {
        src: real,
        alpha_bits,
    });
    r.tag(cc, "cybercycle");

    r.kind = "cybercycle";
    r.options = options.to_vec();
    r.tail_inputs = vec![cc];
    // First published bar is 6 (min_data=7), so lead = 6.
    r.outputs.push(Sink::iface_lead(cc, 6));
    r
}

/// `TrendMode(alpha)` — uses cybercycle + peak decay envelope + threshold logic.
/// Streams: 0=real. Options `[alpha]`. Fixed alpha only (>0).
/// Graph: cycle = Cybercycle(real, alpha), abs_cycle = Abs(cycle),
///        peak = PeakDecay(abs_cycle, 0.991)
///        trend = WhereLe(Abs(cycle), Mul(peak, 0.2)) & WhereGt(peak, 0).
/// First published bar is 55 (`min_data=56`).
pub fn trendmode(options: &[f64]) -> Recipe {
    let alpha_bits = options[0].to_bits();
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");

    // Cybercycle with fixed alpha (alpha > 0)
    let cc = r.add(Node::Cybercycle {
        src: real,
        alpha_bits,
    });
    r.tag(cc, "cybercycle");

    // Absolute cycle
    let abs_cycle = r.add(Node::Abs { src: cc });
    r.tag(abs_cycle, "abs_cycle");

    // Peak decay envelope with fixed decay 0.991
    let decay_bits = 0.991_f64.to_bits();
    let peak = r.add(Node::PeakDecay {
        src: abs_cycle,
        decay_bits,
    });
    r.tag(peak, "peak");

    // Threshold logic: trend = 1 if peak > 0 AND abs_cycle < 0.2*peak else 0
    let threshold = r.constant(0.2);
    let scaled_peak = r.add(Node::Mul {
        a: threshold,
        b: peak,
    });
    r.tag(scaled_peak, "scaled_peak");
    // Zero const for comparison
    let zero_const = r.constant(0.0);
    r.tag(zero_const, "zero_const");
    // WhereGt(peak, 0, 1) → 1 if peak > 0, else 0
    let one_const = r.constant(1.0);
    let gt_peak = r.add(Node::WhereGt {
        a: peak,
        b: zero_const,
        t: one_const,
    });
    r.tag(gt_peak, "gt_peak");
    // WhereGt(scaled_peak, abs_cycle, 1) → 1 if scaled_peak > abs_cycle (i.e. abs_cycle < scaled_peak), else 0
    let gt_scaled = r.add(Node::WhereGt {
        a: scaled_peak,
        b: abs_cycle,
        t: one_const,
    });
    r.tag(gt_scaled, "gt_scaled");
    // AND both conditions (Mul works as logical AND for 0/1 values)
    let trend = r.add(Node::Mul {
        a: gt_peak,
        b: gt_scaled,
    });
    r.tag(trend, "trend");

    r.kind = "trendmode";
    r.options = options.to_vec();
    r.tail_inputs = vec![cc];
    // First published bar is 55 (min_data=56), so lead = 55.
    // Primary: trend with lead 55
    r.outputs.push(Sink::iface_lead(trend, 55));
    // Optional outputs: cycle and peak with lead 55
    r.optional_outputs.push(Sink::iface_lead(cc, 55));
    r.optional_outputs.push(Sink::iface_lead(peak, 55));
    r
}

/// `CcFisher(alpha)` — uses cybercycle + peak decay envelope + Fisher transform.
/// Streams: 0=real. Options `[alpha]`. Fixed alpha only (>0).
/// Graph: cycle = Cybercycle(real, alpha), abs_cycle = Abs(cycle),
///        peak = PeakDecay(abs_cycle, 0.991)
///        value = IfThenElse(peak > 0, cycle / peak, 0)
///        val1 = IirClamped(value, a=0.35, b=0.65, clamp ±0.999)
///        fisher = 0.5 * ln((1+val1)/(1-val1))
///        signal = Prev(fisher)
/// First published bar is 55 (`min_data=56`).
pub fn ccfisher(options: &[f64]) -> Recipe {
    let alpha_bits = options[0].to_bits();
    let mut r = Recipe::price();
    let real = r.add(Node::Input { stream: 0 });
    r.tag(real, "real");

    // Cybercycle with fixed alpha (alpha > 0)
    let cc = r.add(Node::Cybercycle {
        src: real,
        alpha_bits,
    });
    r.tag(cc, "cybercycle");

    // Absolute cycle
    let abs_cycle = r.add(Node::Abs { src: cc });
    r.tag(abs_cycle, "abs_cycle");

    // Peak decay envelope with fixed decay 0.991
    let decay_bits = 0.991_f64.to_bits();
    let peak = r.add(Node::PeakDecay {
        src: abs_cycle,
        decay_bits,
    });
    r.tag(peak, "peak");

    // value = if peak > 0 then cycle / peak else 0
    let zero_const = r.constant(0.0);
    r.tag(zero_const, "zero_const");
    let ratio = r.add(Node::Div { a: cc, b: peak });
    r.tag(ratio, "ratio");
    let value = r.add(Node::WhereGt {
        a: peak,
        b: zero_const,
        t: ratio,
    });
    r.tag(value, "value");

    // IirClamped: val1 = 0.65*val1 + 0.35*value, clamp ±0.999
    // Using K_IIRCLAMPED with a=0.35, b=0.65 and clamps ±0.999
    let hi_t = r.constant(0.999); // threshold (unused for IirClamped)
    let hi_v = r.constant(0.999); // high clamp value
    let lo_t = r.constant(-0.999); // threshold (unused for IirClamped)
    let lo_v = r.constant(-0.999); // low clamp value
    let val1 = r.add(Node::IirClampedPlain {
        src: value,
        a_bits: 0.35_f64.to_bits(),
        b_bits: 0.65_f64.to_bits(),
        hi_t,
        hi_v,
        lo_t,
        lo_v,
    });
    r.tag(val1, "val1");

    // Fisher transform: fisher = 0.5 * ln((1+val1)/(1-val1))
    let one = r.constant(1.0);
    let half = r.constant(0.5);
    let one_plus_val1 = r.add(Node::Add { a: one, b: val1 });
    r.tag(one_plus_val1, "one_plus_val1");
    let one_minus_val1 = r.add(Node::Sub { a: one, b: val1 });
    r.tag(one_minus_val1, "one_minus_val1");
    let ratio_fisher = r.add(Node::Div {
        a: one_plus_val1,
        b: one_minus_val1,
    });
    r.tag(ratio_fisher, "ratio_fisher");
    let fisher = r.add(Node::Ln { src: ratio_fisher });
    r.tag(fisher, "fisher");
    let fisher_scaled = r.add(Node::Mul { a: half, b: fisher });
    r.tag(fisher_scaled, "fisher");

    // Signal = Prev(fisher)
    let signal = r.add(Node::Prev { src: fisher_scaled });
    r.tag(signal, "signal");

    r.kind = "ccfisher";
    r.options = options.to_vec();
    r.tail_inputs = vec![cc];
    // First published bar is 55 (min_data=56), so lead = 55.
    // Primary outputs: fisher and signal with lead 55
    r.outputs.push(Sink::iface_lead(fisher_scaled, 55));
    r.outputs.push(Sink::iface_lead(signal, 55));
    // Optional outputs: trendmode (computed from same nodes), cycle, peak with lead 55
    // Reuse the threshold logic for trendmode optional output
    let threshold = r.constant(0.2);
    let scaled_peak_opt = r.add(Node::Mul {
        a: threshold,
        b: peak,
    });
    r.tag(scaled_peak_opt, "scaled_peak");
    // Zero const for comparison
    let zero_const_opt = r.constant(0.0);
    r.tag(zero_const_opt, "zero_const");
    let one_const_opt = r.constant(1.0);
    let gt_peak_opt = r.add(Node::WhereGt {
        a: peak,
        b: zero_const_opt,
        t: one_const_opt,
    });
    r.tag(gt_peak_opt, "gt_peak");
    // WhereGt(scaled_peak, abs_cycle, 1) → 1 if scaled_peak > abs_cycle (i.e. abs_cycle < scaled_peak), else 0
    let gt_scaled_opt = r.add(Node::WhereGt {
        a: scaled_peak_opt,
        b: abs_cycle,
        t: one_const_opt,
    });
    r.tag(gt_scaled_opt, "gt_scaled");
    let trendmode_opt = r.add(Node::Mul {
        a: gt_peak_opt,
        b: gt_scaled_opt,
    });
    r.tag(trendmode_opt, "trendmode");
    r.optional_outputs.push(Sink::iface_lead(trendmode_opt, 55));
    r.optional_outputs.push(Sink::iface_lead(cc, 55));
    r.optional_outputs.push(Sink::iface_lead(peak, 55));
    r
}
