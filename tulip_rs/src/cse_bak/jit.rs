//! The Cranelift emitter: compile a `Request`'s MERGED ARENA into ONE fused
//! machine loop per plan shape.
//!
//! Every interpreted executor that ran this arena (retired `fuse`/
//! `columnar`/`hybrid`, Handover §4) paid ~65µs of per-bar store→load
//! round-trips on the classic query because no compiler can fuse across the
//! per-request pass boundary. A JIT'd plan owns ALL math in one loop: every
//! arena chain (CSE'd EMA, stepped exactly once for all consumers), every
//! derived value (the tail arithmetic lifted to CLIF for the closed kind
//! set), every sink (lead-gated stores into the caller's output buffers).
//!
//! Emitted shape — PHASE-SPLIT, the hand `init_state`/sliced-`cycle`
//! discipline as codegen: the absolute bar line is cut at every gate the
//! plan has (sink leads + chained feed leads) and each segment gets its
//! own specialized loop, so inside a loop every gate is a compile-time
//! constant:
//!
//! ```text
//! setup (once per chunk): state loaded; entry seed from price[0]; out
//!   pointers; per-segment hi = min(end, boundary). i is the ABSOLUTE bar.
//! for seg [b, e) in [0,b0) [b0,b1) … [blast, ∞):        # heads chain,
//!   for i in max(start, b)..min(end, e):                # 1 cmp per skip
//!     price = load
//!     ops in topological order:
//!       Ema  : fma(prev, inv, src*mult)                 // ← `mul_add`, FMA
//!              chained: SKIPPED before its feed lead; the one segment
//!              where feeding starts keeps the seed select; later segments
//!              step unconditionally (seed flag proven 1)
//!       arith: fsub / fmul / fdiv / fmax over node operands — a `Sub`
//!              declared identically by APO, MACD and PPO is ONE node
//!              (merge interns it: one fsub, up to 3 consumers)
//!       Const  : materialized f64 (fmax's guard, the ×100)
//!     sinks with lead ≤ b: UNCONDITIONAL out[i - lead] stores — no icmp,
//!          no brif, no select on the address (branchless always-store was
//!          measured 18→24µs WORSE; the phase split removes the branches
//!          without ever issuing a dead write)
//! state stored once → ret
//! ```
//!
//! Bit-exactness doctrine: `ema::calc` is `prev.mul_add(inv, value*mult)` →
//! `fmul` + `fma` (never mul+add); the §5.3 lifecycle (seed on first valid
//! source bar, chained recurrences fed only past their source's publish
//! lead, `next_bar`-absolute gating) is the lifecycle the hand indicators
//! define — every parity test here is against HAND, bit-for-bit (`to_bits`),
//! mirroring the hand-derived leads and the hand `TState::calc` order.
//!
//! Scope: emittable node kinds `Input`/`Ema`/`Const`/`Sub`/`Mul`/`Div`/
//! `Max` over any merged recipe graph — the engine carries NO indicator
//! knowledge (not even a `match` on indicator): each recipe's `restorer`
//! fn pointer (defined in the indicator module) rebuilds its hand state.
//! Filter/HT node kinds panic at build — emitting them (and WMA/ring taps)
//! is the coverage roadmap (Handover §6 Step E). Sources are
//! assumed complete (producer-NaN gating would be one more select per node);
//! E1 lane packing is in: price-sourced EMA chains group in `f64x2` pairs
//! stepped by one vector `fma` (cranelift x64's 128-bit ceiling — see
//! `MAX_LANES`), chained recurrences stay scalar.

use cranelift::codegen::ir::{BlockArg, Function, MemFlagsData, UserFuncName};
use cranelift::codegen::isa::TargetFrontendConfig;
use cranelift::codegen::Context;
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};

use crate::cse::query::{IndicatorStateBox, Request, Results};
use crate::cse::{merge, Merged, Node, Sink, SinkSrc, StateRestorer};
use crate::indicators::ema::{multiplier, State as EmaState};
use crate::types::Warm;

/// Compiled entry point: advance `len` bars of `price`, storing to `outs`
/// (one `*mut f64` per sink, plan order), carrying `state` (raw u64 words,
/// layout owned by the plan) across chunk calls.
pub type JitFn =
    unsafe extern "C" fn(state: *mut u64, price: *const f64, len: usize, outs: *mut *mut f64);

/// Source reference: the price stream.
const PRICE: usize = usize::MAX;
/// Sink lead meaning "never print" (disabled optional).
const LEAD_NEVER: u64 = u64::MAX;

/// E1 lane cap. The x64 backend only classifies vectors ≤ 128 bits
/// (`rc_for_type`), so **f64x2 is the widest packable chain group** —
/// cranelift has no 256/512-bit vector support regardless of host AVX
/// flags (those enable VEX encodings for xmm ops). Groups are therefore
/// pairs; `avx512` does not lift this ceiling.
const MAX_LANES: usize = 2;

fn vec_ty(lanes: usize) -> Type {
    match lanes {
        8 => types::F64X8,
        4 => types::F64X4,
        _ => types::F64X2,
    }
}

/// One loop op: an EMA recurrence (state) or a pure derived value.
#[derive(Clone)]
struct Op {
    e: OpE,
    /// Absolute bar the value first exists (chain defined-from / tail's
    /// max-of-sources lead).
    lead: u64,
    /// State-word indices (EMA ops only).
    ema_word: usize,
    sd_word: usize,
}

#[derive(Clone)]
enum OpE {
    /// `prev*inv + src*mult` — the `mul_add` recurrence. `chained` = the
    /// source is another op (any node, incl. an arith node — MACD's signal
    /// is just an `Ema` over the line `Sub`): step/seed only once the
    /// source reports (`i >= feed`), otherwise advance every bar (price
    /// source is always there; the chunk's first bar seeds an unseeded
    /// chain in setup).
    Ema {
        src: usize,
        mult: f64,
        inv: f64,
        feed: u64,
        chained: bool,
    },
    /// A binary graph node (`Node::Sub/Mul/Div/Max`). With `vec == true`
    /// (role-aligned pair subtraction, E2-lite) `a`/`b` are GROUP ids whose
    /// lanes are the two paired expressions: ONE `fsub` computes both,
    /// lanes split only at consumers/stores. `vec == false`: `a`/`b` are op
    /// refs, scalarized on demand (grouped operands extract their lane,
    /// aliased operands read the pair-vector lane — see `alias`).
    Arith {
        op: ArithKind,
        a: usize,
        b: usize,
        vec: bool,
    },
    /// A `Node::Const` leaf — materialized, rematerialized by RA at use.
    Const(f64),
}

/// `alias`/lane sentinel: "the op's own scalar value".
const LANE_SELF: u8 = 0xFE;

#[derive(Clone, Copy)]
enum ArithKind {
    Sub,
    Mul,
    Div,
    /// CLIF `fmax` — IEEE maxNum, same NaN rule as `f64::max`.
    Max,
}

/// One compiled sink: value op + publish lead (clamped `max(declared,
/// value lead)`; `LEAD_NEVER` when disabled).
struct SinkSpec {
    val: usize,
    lead: u64,
}

/// Per-request metadata for `get_states` reconstitution — the request's
/// state chains (op + period, `TState` field order) and the INDICATOR's own
/// restorer fn. The engine holds no indicator identity at all: `restorer`
/// is an opaque pointer and the sink layout lives in `counts`.
struct ReqMeta {
    state: Vec<(usize, u32)>,
    restorer: Option<StateRestorer>,
}

/// E1: one lane-packed group of price-sourced EMA chains. All members read
/// the SAME price stream each bar, so the group steps with ONE vector
/// `fmul`+`fma` (`vfmadd*pd` per lane — lane-wise identical rounding to the
/// scalar `mul_add`, so bit-exactness is preserved by construction).
/// Consumers extract their lane (cached per op, exactly once per bar).
/// Chained recurrences never join a group: their source is a same-bar op
/// value (the §2 producer-consumer rule).
pub struct LaneGroup {
    /// Member chain ops, in LANE order.
    pub ops: Vec<usize>,
}

/// A compiled plan: the JIT module (keeps the code alive), the entry point,
/// and the metadata that makes `run`/`resume` return shaped `Results`.
pub struct JitPlan {
    module: JITModule,
    func: Option<JitFn>,
    arena: Merged,
    /// The verifier-cleaned CLIF text of the compiled loop, captured at
    /// compile time for inspection/debugging (`JitPlan::clif`).
    clif: String,
    ops: Vec<Op>,
    /// Lane groups (§7.1 greedy pow-2 buckets ≤ `MAX_LANES`).
    groups: Vec<LaneGroup>,
    /// op → (group, lane); `usize::MAX` group = stepped scalarly.
    lane_of: Vec<(usize, usize)>,
    /// E2-lite: absorbed pair-sub → (pair-vector sub op, lane 1).
    alias: Vec<(usize, u8)>,
    sinks: Vec<SinkSpec>,
    counts: Vec<(usize, usize)>,
    reqs: Vec<ReqMeta>,
    nwords: usize,
}

/// Per-asset resumable state: the plan's raw state words (header
/// `next_bar`/`seeded`, chained seed flags, chain values). Layout owned by
/// the emitter; opaque otherwise.
#[derive(Clone)]
pub struct JitState {
    pub words: Vec<u64>,
}

impl JitState {
    pub fn next_bar(&self) -> u64 {
        self.words[0]
    }
}

const W_NEXT: usize = 0;
const W_SEED: usize = 1;

/// Merge `Request` (CSE'd node-graph arena + leads + sink wiring) and
/// compile it into one fused loop. The walk is over NODE KINDS — the engine
/// knows no indicators; a recipe's graph carries the whole math (chains,
/// shared sub-expressions, constants) and the only per-indicator hook is
/// the recipe's own `restorer` fn for `get_states`. Node kinds outside the
/// emittable set (`HighPass`/`SuperSmoother`/HT taps) panic at build.
fn compile_request(req: &Request) -> JitPlan {
    let (arena, mut resolved) = merge(&req.recipes);
    // same switch `Request::build` applies — disabled optionals compile to
    // never-print sinks (EMPTY in position, values unchanged elsewhere).
    for (r, flags) in resolved.iter_mut().zip(&req.opt_flags) {
        r.optional_enabled = flags.clone();
    }
    let leads = arena.leads();

    // ---- pass 1: arena nodes → ops, SSA order (producers are lower ids) ----
    let mut ops: Vec<Op> = Vec::new();
    let mut node_op: Vec<usize> = vec![PRICE; arena.nodes.len()]; // Input → PRICE
    macro_rules! push {
        ($id:expr, $e:expr) => {{
            let id = $id as usize;
            ops.push(Op {
                e: $e,
                lead: leads[id] as u64,
                ema_word: 0,
                sd_word: 0,
            });
            node_op[id] = ops.len() - 1;
        }};
    }
    for (id, node) in arena.nodes.iter().enumerate() {
        match *node {
            Node::Input { .. } => {}
            Node::Const { bits } => push!(id, OpE::Const(f64::from_bits(bits))),
            Node::Ema { src, period } => {
                let (mult, inv) = multiplier(period as usize);
                let src_op = node_op[src as usize];
                push!(
                    id,
                    OpE::Ema {
                        src: src_op,
                        mult,
                        inv,
                        feed: leads[src as usize] as u64,
                        chained: src_op != PRICE,
                    }
                );
            }
            Node::Sub { a, b } => push!(
                id,
                OpE::Arith {
                    op: ArithKind::Sub,
                    a: node_op[a as usize],
                    b: node_op[b as usize],
                    vec: false,
                }
            ),
            Node::Mul { a, b } => push!(
                id,
                OpE::Arith {
                    op: ArithKind::Mul,
                    a: node_op[a as usize],
                    b: node_op[b as usize],
                    vec: false,
                }
            ),
            Node::Div { a, b } => push!(
                id,
                OpE::Arith {
                    op: ArithKind::Div,
                    a: node_op[a as usize],
                    b: node_op[b as usize],
                    vec: false,
                }
            ),
            Node::Max { a, b } => push!(
                id,
                OpE::Arith {
                    op: ArithKind::Max,
                    a: node_op[a as usize],
                    b: node_op[b as usize],
                    vec: false,
                }
            ),
            other => panic!("jit: node {other:?} is not emittable yet"),
        }
    }
    debug_assert_eq!(
        ops.len(),
        arena
            .nodes
            .iter()
            .filter(|n| !matches!(n, Node::Input { .. }))
            .count()
    );

    // ---- pass 2: sinks + state wiring, all by arena id ----
    let mut sinks: Vec<SinkSpec> = Vec::new();
    let mut counts = Vec::new();
    let mut reqs: Vec<ReqMeta> = Vec::new();

    for res in &resolved {
        let mut add = |list: &[Sink], enabled: Option<&[bool]>| {
            for (oi, s) in list.iter().enumerate() {
                if enabled.is_some_and(|f| !f.get(oi).copied().unwrap_or(true)) {
                    sinks.push(SinkSpec {
                        val: 0,
                        lead: LEAD_NEVER,
                    });
                    continue;
                }
                let SinkSrc::Interface(id) = s.src;
                let o = node_op[id as usize];
                assert!(
                    o != PRICE,
                    "jit: mirroring a bare Input needs sink support for price streams"
                );
                // hand invariant (slot_lengths/output_length): publish lead
                // = max(declared, value lead)
                sinks.push(SinkSpec {
                    val: o,
                    lead: (s.lead as u64).max(ops[o].lead),
                });
            }
        };
        add(&res.outputs, None);
        add(&res.optional_outputs, Some(&res.optional_enabled));

        counts.push((res.outputs.len(), res.optional_outputs.len()));
        let state = res
            .inputs
            .iter()
            .map(|&i| match arena.nodes[i as usize] {
                Node::Ema { period, .. } => (node_op[i as usize], period),
                _ => panic!("jit: `tail_inputs` id {i} must name an Ema chain"),
            })
            .collect();
        reqs.push(ReqMeta {
            state,
            restorer: res.restorer,
        });
    }

    // ---- E1 + E2-lite: lane grouping with role-aligned line-subs ----
    //
    // Price-sourced chain ops step as f64x2 lanes. E2-lite pairs `short −
    // long` sub-ops whose operands are both chains: ONE vector group holds
    // a pair's shorts, another its longs, and ONE fsub on the group vectors
    // produces both line values; the second absorbed sub aliases (pair-sub,
    // lane 1). Consumers split lanes lazily (`scalarize` in the emitter).
    // Pairs are chosen disjoint-chains-first (zero duplication); a chain
    // feeding several subs is cloned with its own state word — identical
    // stepping keeps every word bit-exact. Chains not covered by any pair
    // fall back to plain arena-order buckets.
    let base_ops = ops.len();
    let mut lane_of: Vec<(usize, usize)> = vec![(usize::MAX, 0); 2 * base_ops + 8];
    let mut placed: Vec<bool> = vec![false; base_ops];
    let mut groups: Vec<LaneGroup> = Vec::new();
    let mut alias: Vec<(usize, u8)> = vec![(usize::MAX, LANE_SELF); base_ops];

    // Snapshot candidate sub operands BEFORE any `ops` mutation.
    let is_chain = |o: &Op| matches!(o.e, OpE::Ema { src, chained: false, .. } if src == PRICE);
    let sub_operands: Vec<Option<(usize, usize)>> = (0..base_ops)
        .map(|j| match ops[j].e {
            OpE::Arith { op: ArithKind::Sub, a, b, vec: false }
                if is_chain(&ops[a]) && is_chain(&ops[b]) && a != b =>
            {
                Some((a, b))
            }
            _ => None,
        })
        .collect();

    // Pair candidates: a disjoint partner (zero clones) when one exists,
    // otherwise any partner (reuse absorbed by cloning).
    let mut cands: Vec<usize> = (0..base_ops).filter(|&j| sub_operands[j].is_some()).collect();
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    while !cands.is_empty() {
        let s1 = cands.remove(0);
        let (a1, b1) = sub_operands[s1].unwrap();
        let disjoint = cands.iter().position(|&s2| {
            let (a2, b2) = sub_operands[s2].unwrap();
            a2 != a1 && a2 != b1 && b2 != a1 && b2 != b1
        });
        let pick = disjoint.or(if cands.len() >= 2 { Some(1) } else { None });
        if let Some(k) = pick {
            let s2 = cands.remove(k);
            pairs.push((s1, s2));
        }
    }

    // Place each pair: S group (lane 0 = sub1's short, lane 1 = sub2's),
    // L group likewise. A chain already placed gets a CLONE op — same
    // stepping, its own state word, so the two consumers' words agree
    // bit-exactly.
    macro_rules! slot {
        ($chain:expr) => {{
            let chain = $chain;
            if !placed[chain] {
                placed[chain] = true;
                chain
            } else {
                let clone = ops.len();
                ops.push(ops[chain].clone());
                clone
            }
        }};
    }
    for &(s1, s2) in &pairs {
        let (a1, b1) = sub_operands[s1].unwrap();
        let (a2, b2) = sub_operands[s2].unwrap();
        let gs = groups.len();
        groups.push(LaneGroup { ops: Vec::new() });
        let gl = groups.len();
        groups.push(LaneGroup { ops: Vec::new() });
        for (chain, g, lane) in [
            (a1, gs, 0usize),
            (a2, gs, 1),
            (b1, gl, 0),
            (b2, gl, 1),
        ] {
            let slot = slot!(chain);
            lane_of[slot] = (g, lane);
            groups[g].ops.push(slot);
        }
        ops[s1].e = OpE::Arith {
            op: ArithKind::Sub,
            a: gs,
            b: gl,
            vec: true,
        };
        alias[s2] = (s1, 1);
    }
    debug_assert!(groups.iter().all(|g| g.ops.len() <= MAX_LANES));

    // Chains no pair covers (classic's two chains under its lone sub):
    // plain arena-order buckets.
    {
        let mut rest: Vec<usize> = (0..base_ops)
            .filter(|&j| is_chain(&ops[j]) && !placed[j])
            .collect();
        while rest.len() >= 2 {
            let cap = MAX_LANES.min(rest.len());
            let mut take = 1usize;
            while take * 2 <= cap {
                take *= 2;
            }
            let chunk: Vec<usize> = rest.drain(..take).collect();
            let g = groups.len();
            for (l, &o) in chunk.iter().enumerate() {
                lane_of[o] = (g, l);
            }
            groups.push(LaneGroup { ops: chunk });
        }
        // a lone remainder stays scalar (no group)
    }
    alias.resize(ops.len(), (usize::MAX, LANE_SELF));

    // ---- state layout: header, chained seed flags, op values ----
    let chained: Vec<usize> = (0..ops.len())
        .filter(|&j| matches!(ops[j].e, OpE::Ema { chained: true, .. }))
        .collect();
    let nsd = chained.len();
    let ema_base = 2 + nsd;
    for (i, &j) in chained.iter().enumerate() {
        ops[j].sd_word = 2 + i;
    }
    for (j, op) in ops.iter_mut().enumerate() {
        op.ema_word = ema_base + j;
    }
    let nwords = ema_base + ops.len();

    // ---- emit ----
    let mut flag_builder = settings::builder();
    flag_builder.set("is_pic", "false").unwrap();
    // E1 lane packing needs the host vector features (SSE2/AVX/FMA) in the
    // ISA flags — plain `builder()` leaves CPUID-gated flags off.
    let isa_builder = cranelift_native::builder_with_options(true).expect("host ISA builder");
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .expect("native ISA");
    let fc = isa.frontend_config();
    let module = JITModule::new(JITBuilder::with_isa(isa, default_libcall_names()));

    let mut plan = JitPlan {
        module,
        func: None,
        arena,
        clif: String::new(),
        ops,
        groups,
        lane_of,
        alias,
        sinks,
        counts,
        reqs,
        nwords,
    };
    emit(&mut plan, fc);
    plan
}

impl JitPlan {
    /// Compile entry used by `Request::build`.
    pub fn compile(req: &Request) -> Self {
        compile_request(req)
    }

    /// The CSE'd arena (introspection: node count proves dedup).
    pub fn arena(&self) -> &Merged {
        &self.arena
    }

    /// The compiled loop's CLIF text (debug/inspection).
    pub fn clif(&self) -> &str {
        &self.clif
    }

    /// (ops, sinks) — the compiled shape, for caching keys/bench black-boxing.
    pub fn info(&self) -> (usize, usize) {
        (self.ops.len(), self.sinks.len())
    }

    /// E1 lane-group widths, e.g. `[2]` (classic) or `[2, 2, 2, 2, 2, 2]`
    /// (dense, role-aligned pairs).
    pub fn lanes(&self) -> Vec<usize> {
        self.groups.iter().map(|g| g.ops.len()).collect()
    }

    /// E2-lite: number of `short − long` subtractions computed as ONE
    /// vector `fsub` over lane-aligned groups (dense dashboards pair their
    /// lines; classic's lone sub stays scalar).
    pub fn vec_subs(&self) -> usize {
        self.ops
            .iter()
            .filter(|op| matches!(op.e, OpE::Arith { op: ArithKind::Sub, vec: true, .. }))
            .count()
    }

    pub fn new_state(&self) -> JitState {
        JitState {
            words: vec![0u64; self.nwords],
        }
    }

    pub fn run(&self, series: &[&[f64]]) -> (Results, JitState) {
        let mut st = self.new_state();
        let res = self.advance(&mut st, series);
        (res, st)
    }

    pub fn resume(&self, mut st: JitState, series: &[&[f64]]) -> (Results, JitState) {
        let res = self.advance(&mut st, series);
        (res, st)
    }

    /// Advance one chunk: per-sink output rows `len − max(lead, start) +
    /// start` (empty for never-print), call the compiled loop, regroup
    /// `Results` request-major (disabled optionals keep their EMPTY slot).
    pub fn advance(&self, st: &mut JitState, series: &[&[f64]]) -> Results {
        let len = series.iter().map(|s| s.len()).max().unwrap_or(0);
        let start = st.words[W_NEXT] as usize;
        let price = series.first().copied().unwrap_or(&[]);

        let mut outs: Vec<Vec<f64>> = Vec::with_capacity(self.sinks.len());
        let mut ptrs: Vec<*mut f64> = Vec::with_capacity(self.sinks.len());
        for s in &self.sinks {
            let g0 = s.lead.saturating_sub(start as u64) as usize;
            let rows = len.saturating_sub(g0);
            let mut v = crate::uninit_vec!(f64, rows);
            ptrs.push(v.as_mut_ptr());
            outs.push(v);
        }
        if len > 0 {
            let f = self.func.expect("compiled plan has a function");
            unsafe {
                (f)(
                    st.words.as_mut_ptr(),
                    price.as_ptr(),
                    len,
                    ptrs.as_mut_ptr(),
                )
            };
        }
        st.words[W_NEXT] = (start + len) as u64;

        let mut outputs = Vec::with_capacity(self.counts.len());
        let mut optional = Vec::with_capacity(self.counts.len());
        for &(np, no) in &self.counts {
            outputs.push(outs.drain(..np).collect());
            optional.push(outs.drain(..no).collect());
        }
        Results { outputs, optional }
    }

    fn ema_of(&self, st: &JitState, op: usize) -> f64 {
        f64::from_bits(st.words[self.ops[op].ema_word])
    }

    /// Reconstitute hand states by feeding each request's chain slots to
    /// the INDICATOR's `restorer` (carried through `Recipe::restorer`) —
    /// the engine never names an indicator to rebuild its state.
    pub fn get_states(&self, st: &JitState) -> Vec<IndicatorStateBox> {
        self.reqs
            .iter()
            .map(|r| match r.restorer {
                Some(rest) => {
                    let states = r
                        .state
                        .iter()
                        .map(|&(op, period)| ema_state(self.ema_of(st, op), period as usize))
                        .collect();
                    rest(states)
                }
                None => IndicatorStateBox::Other,
            })
            .collect()
    }
}

fn ema_state(ema: f64, period: usize) -> EmaState<Warm> {
    EmaState::new(ema, period).into_warm()
}

// ---------------------------------------------------------------------------
// code generation
// ---------------------------------------------------------------------------

fn ba(vals: &[Value]) -> Vec<BlockArg> {
    vals.iter().copied().map(BlockArg::Value).collect()
}

fn add_params(fb: &mut FunctionBuilder, b: Block, tys: &[Type]) {
    for &t in tys {
        fb.append_block_param(b, t);
    }
}

fn ld(fb: &mut FunctionBuilder, base: Value, off: i64, ty: Type) -> Value {
    let offv = fb.ins().iconst(types::I64, off);
    let addr = fb.ins().iadd(base, offv);
    fb.ins().load(ty, MemFlagsData::new(), addr, 0)
}

fn st(fb: &mut FunctionBuilder, base: Value, off: i64, val: Value) {
    let offv = fb.ins().iconst(types::I64, off);
    let addr = fb.ins().iadd(base, offv);
    fb.ins().store(MemFlagsData::new(), val, addr, 0);
}

/// Build and define the plan's function; sets `plan.func`.
///
/// Phase-split (the hand `init_state`/sliced-`cycle` discipline as codegen):
/// the chunk's absolute bar line is cut at every gate point the plan has
/// (sink leads + chained feed leads), and ONE specialized loop is emitted
/// per segment. Inside a segment each gate is a compile-time constant:
/// sinks whose lead ≤ the segment start store UNCONDITIONALLY at
/// `i - base`; later sinks emit no gate at all; chained recurrences step
/// only in segments past their feed lead (the segment where feeding
/// begins keeps the one-time seed select). The steady (last) segment is
/// the hand shape: zero `icmp`, zero `brif` beyond the loop condition,
/// zero selects — branch cost paid once per boundary crossing, per
/// lifetime, not per bar.
///
/// E1 lane packing: the loop-carried VALUE SLOTS are `[group vectors..,
/// scalar op values..]` — each price-sourced EMA group (chains reading the
/// same stream, §7.1 pow-2 buckets) steps as ONE vector `fmul`+`fma`
/// (`vfmadd*pd`, lane-wise identical to scalar `mul_add`); consumers get
/// their lane via one cached `extractlane`. State words stay per-chain
/// (pack/unpack happens exactly once per chunk in setup/end, never in the
/// loop body's edges).
fn emit(plan: &mut JitPlan, fc: TargetFrontendConfig) {
    let module = &mut plan.module;
    let i64_t = types::I64;
    let f64_t = types::F64;

    let nops = plan.ops.len();
    let nsinks = plan.sinks.len();
    let ngroups = plan.groups.len();
    let chained: Vec<usize> = (0..nops)
        .filter(|&j| matches!(plan.ops[j].e, OpE::Ema { chained: true, .. }))
        .collect();
    let nsd = chained.len();

    // Value slots: [0..ngroups) = group vectors, then ungrouped ops in op
    // order. `cur[op]` remains the per-op scalar view for consumers.
    let mut slot_of_op = vec![usize::MAX; nops];
    let mut op_of_slot: Vec<usize> = Vec::with_capacity(nops);
    for (g, grp) in plan.groups.iter().enumerate() {
        for &o in &grp.ops {
            slot_of_op[o] = g;
        }
    }
    for j in 0..nops {
        if plan.lane_of[j].0 == usize::MAX && matches!(plan.ops[j].e, OpE::Ema { .. }) {
            slot_of_op[j] = ngroups + op_of_slot.len();
            op_of_slot.push(j);
        }
    }
    let nval = ngroups + op_of_slot.len();

    // Segment bounds: sorted distinct finite gate leads (sink + chained
    // feed); segments are [0,b0), [b0,b1), …, [blast, ∞) in absolute bars.
    let mut bounds: Vec<u64> = plan
        .sinks
        .iter()
        .map(|s| s.lead)
        .filter(|&l| l < LEAD_NEVER && l > 0)
        .chain(plan.ops.iter().filter_map(|op| match op.e {
            OpE::Ema {
                chained: true,
                feed,
                ..
            } if feed > 0 => Some(feed),
            _ => None,
        }))
        .collect();
    bounds.sort_unstable();
    bounds.dedup();
    let mut segs: Vec<(u64, Option<u64>)> = Vec::with_capacity(bounds.len() + 1);
    {
        let mut prev = 0u64;
        for &b in &bounds {
            segs.push((prev, Some(b)));
            prev = b;
        }
        segs.push((prev, None));
    }

    let mut sig = module.make_signature();
    for _ in 0..4 {
        sig.params.push(AbiParam::new(i64_t));
    }
    let id = module
        .declare_function("cse_jit_plan", Linkage::Export, &sig)
        .expect("declare");

    let mut ctx = Context::new();
    ctx.func = Function::with_name_signature(UserFuncName::user(0, 0), sig.clone());
    let mut fb_ctx = FunctionBuilderContext::new();
    let mut fb = FunctionBuilder::new(&mut ctx.func, &mut fb_ctx);

    // Loop-carried params (IDENTICAL on every segment loop so phase
    // transitions are pass-throughs): [ value slots.., chained seeds.., i ].
    // `i` is the ABSOLUTE bar counter; `sdpos`/`ipos` index into it.
    let mut pt: Vec<Type> = Vec::with_capacity(nval);
    for (g, grp) in plan.groups.iter().enumerate() {
        let _ = g;
        pt.push(vec_ty(grp.ops.len()));
    }
    pt.extend(std::iter::repeat(f64_t).take(op_of_slot.len()));
    pt.extend(std::iter::repeat(i64_t).take(nsd));
    pt.push(i64_t);
    let nparam = pt.len();
    let ipos = nparam - 1;
    let sdpos = nval;

    let entry = fb.create_block();
    let setup = fb.create_block();
    let end_bb = fb.create_block();
    let mut head_bb = Vec::with_capacity(segs.len());
    let mut body_bb = Vec::with_capacity(segs.len());
    for _ in 0..segs.len() {
        head_bb.push(fb.create_block());
        body_bb.push(fb.create_block());
    }

    fb.append_block_params_for_function_params(entry);
    for &h in &head_bb {
        add_params(&mut fb, h, &pt);
    }
    for &b in &body_bb {
        add_params(&mut fb, b, &pt);
    }
    // end: value slots + seeds + the global seeded flag
    let mut et = pt.clone();
    et.pop();
    et.push(i64_t);
    add_params(&mut fb, end_bb, &et);

    // assemble a group vector from per-lane scalar sources
    macro_rules! pack {
        ($g:expr, $src:expr) => {{
            let g = $g;
            let ty = vec_ty(plan.groups[g].ops.len());
            let z = fb.ins().f64const(0.0);
            let mut v = fb.ins().splat(ty, z);
            for (l, &o) in plan.groups[g].ops.iter().enumerate() {
                v = fb.ins().insertlane(v, $src[o], l as u8);
            }
            v
        }};
    }

    // ---- entry: loads, empty-chunk early-out ----
    fb.switch_to_block(entry);
    let ep: Vec<Value> = fb.block_params(entry).to_vec();
    let (state, price, len, outs) = (ep[0], ep[1], ep[2], ep[3]);
    let zero = fb.ins().iconst(i64_t, 0);
    let one = fb.ins().iconst(i64_t, 1);
    let eight = fb.ins().iconst(i64_t, 8);

    let next_bar = ld(&mut fb, state, 8 * W_NEXT as i64, i64_t);
    let seeded = ld(&mut fb, state, 8 * W_SEED as i64, i64_t);
    let mut loads: Vec<Value> = Vec::with_capacity(nops);
    for j in 0..nops {
        let w = plan.ops[j].ema_word as i64;
        loads.push(ld(&mut fb, state, 8 * w, f64_t));
    }
    let mut sdloads: Vec<Value> = Vec::with_capacity(nsd);
    for &j in &chained {
        let w = plan.ops[j].sd_word as i64;
        sdloads.push(ld(&mut fb, state, 8 * w, i64_t));
    }
    let empty = fb.ins().icmp(IntCC::Equal, len, zero);
    let mut end_empty: Vec<Value> = Vec::with_capacity(nval);
    for g in 0..ngroups {
        let v = pack!(g, loads);
        end_empty.push(v);
    }
    for &o in &op_of_slot {
        end_empty.push(loads[o]);
    }
    end_empty.extend(sdloads.iter().copied());
    end_empty.push(seeded);
    fb.ins()
        .brif(empty, end_bb, &ba(&end_empty), setup, &ba(&[]));

    // ---- setup: seed selects, constants, out pointers, segment bounds ----
    fb.switch_to_block(setup);
    let is_seeded = fb.ins().icmp(IntCC::NotEqual, seeded, zero);
    let p0 = ld(&mut fb, price, 0, f64_t);
    let end_abs = fb.ins().iadd(next_bar, len);
    let mut init: Vec<Value> = Vec::with_capacity(nops);
    for j in 0..nops {
        let v = match plan.ops[j].e {
            OpE::Ema { chained: false, .. } => fb.ins().select(is_seeded, loads[j], p0),
            OpE::Ema { .. } => loads[j],
            _ => fb.ins().f64const(0.0),
        };
        init.push(v);
    }
    // Lane constants (setup values — they dominate every loop).
    let mut gmvec: Vec<Value> = Vec::with_capacity(ngroups);
    let mut givec: Vec<Value> = Vec::with_capacity(ngroups);
    for g in 0..ngroups {
        let ty = vec_ty(plan.groups[g].ops.len());
        let z = fb.ins().f64const(0.0);
        let mut mv = fb.ins().splat(ty, z);
        let mut iv = fb.ins().splat(ty, z);
        for (l, &o) in plan.groups[g].ops.iter().enumerate() {
            let (mult, inv) = match plan.ops[o].e {
                OpE::Ema { mult, inv, .. } => (mult, inv),
                _ => unreachable!(),
            };
            let mc = fb.ins().f64const(mult);
            let ic = fb.ins().f64const(inv);
            mv = fb.ins().insertlane(mv, mc, l as u8);
            iv = fb.ins().insertlane(iv, ic, l as u8);
        }
        gmvec.push(mv);
        givec.push(iv);
    }
    // Cold chunk: bar `start` (== 0) was consumed by the entry seed ⇒ i0 = 1.
    let start1 = fb.ins().iadd(next_bar, one);
    let i0 = fb.ins().select(is_seeded, next_bar, start1);
    let mut optr: Vec<Value> = Vec::with_capacity(nsinks);
    for j in 0..nsinks {
        optr.push(ld(&mut fb, outs, 8 * j as i64, i64_t));
    }
    // Segment runtime upper bounds: hi_s = min(end, e_s). Segments are
    // entered in order and each head's `done` test passes the live set
    // straight through — skipping a fully-past segment costs one compare.
    let mut hi_v: Vec<Value> = Vec::with_capacity(segs.len());
    for (_b, e) in &segs {
        match e {
            Some(eb) => {
                let ec = fb.ins().iconst(i64_t, *eb as i64);
                let above = fb.ins().icmp(IntCC::SignedGreaterThan, end_abs, ec);
                hi_v.push(fb.ins().select(above, ec, end_abs));
            }
            None => hi_v.push(end_abs),
        }
    }
    // Per-sink store base = max(start, lead): `out_k[0]` corresponds to
    // absolute bar `base_k` (exactly `advance`'s row slicing), so the
    // in-segment index is `i - base_k` with `i` the absolute counter.
    let mut sbase: Vec<Value> = Vec::with_capacity(nsinks);
    for s in plan.sinks.iter() {
        if s.lead >= LEAD_NEVER {
            sbase.push(zero); // never stores; base unused
            continue;
        }
        let lc = fb.ins().iconst(i64_t, s.lead as i64);
        let below = fb.ins().icmp(IntCC::SignedLessThan, next_bar, lc);
        sbase.push(fb.ins().select(below, lc, next_bar));
    }
    let mut args: Vec<Value> = Vec::with_capacity(nparam);
    for g in 0..ngroups {
        let v = pack!(g, init);
        args.push(v);
    }
    for &o in &op_of_slot {
        args.push(init[o]);
    }
    args.extend(sdloads.iter().copied());
    args.push(i0);
    fb.ins().jump(head_bb[0], &ba(&args));

    // ---- segment loops ----
    for (s, (b, _e)) in segs.iter().enumerate() {
        let b = *b as i64;
        let is_last = s + 1 == segs.len();
        let succ = if is_last { end_bb } else { head_bb[s + 1] };

        fb.switch_to_block(head_bb[s]);
        let hp: Vec<Value> = fb.block_params(head_bb[s]).to_vec();
        let done = fb
            .ins()
            .icmp(IntCC::SignedGreaterThanOrEqual, hp[ipos], hi_v[s]);
        if is_last {
            let mut end_args = hp[..nval + nsd].to_vec();
            end_args.push(one);
            fb.ins()
                .brif(done, end_bb, &ba(&end_args), body_bb[s], &ba(&hp));
        } else {
            fb.ins().brif(done, succ, &ba(&hp), body_bb[s], &ba(&hp));
        }

        fb.switch_to_block(body_bb[s]);
        let bp: Vec<Value> = fb.block_params(body_bb[s]).to_vec();
        let ivar = bp[ipos];
        let roff = fb.ins().isub(ivar, next_bar);
        let poff = fb.ins().imul(roff, eight);
        let paddr = fb.ins().iadd(price, poff);
        let price_v = fb.ins().load(f64_t, MemFlagsData::new(), paddr, 0);

        // price broadcast, one per distinct vector width
        let mut pvs: Vec<(Type, Value)> = Vec::with_capacity(2);
        let mut splat_price = |ty: Type, fb: &mut FunctionBuilder| -> Value {
            if let Some((_, v)) = pvs.iter().find(|(t, _)| *t == ty) {
                return *v;
            }
            let v = fb.ins().splat(ty, price_v);
            pvs.push((ty, v));
            v
        };

        // packed groups: ONE vector step per group. Lane values are NOT
        // extracted here — `scalarize` materializes an extract only where
        // a scalar consumer (sink store, arith operand, chained seed)
        // actually needs one; vector subs consume whole group vectors.
        let mut stepv: Vec<Value> = Vec::with_capacity(ngroups);
        for g in 0..ngroups {
            let ty = vec_ty(plan.groups[g].ops.len());
            let pv = splat_price(ty, &mut fb);
            let prev = bp[g];
            let prod = fb.ins().fmul(pv, gmvec[g]);
            stepv.push(fb.ins().fma(prev, givec[g], prod));
        }
        // lazily-materialized scalar views (Eager values: ungrouped Ema /
        // arith / const; grouped chains + aliased subs start None)
        let mut cur: Vec<Option<Value>> = vec![None; nops];
        // pair-vector results (vec Arith ops): consumed by lane via `alias`
        let mut vvec: Vec<Option<Value>> = vec![None; nops];

        // Resolve any op id to its scalar value for this bar, extracting a
        // vector lane only now (E2-lite's lazy split). SSA order makes the
        // memoized fill safe: producers precede consumers.
        macro_rules! scalarize {
            ($o:expr) => {{
                let o = $o;
                if let Some(v) = cur[o] {
                    v
                } else {
                    let (tgt, lane) = plan.alias[o];
                    let (vecv, l) = if tgt != usize::MAX {
                        (vvec[tgt].expect("pair-vector produced first"), lane)
                    } else if plan.lane_of[o].0 != usize::MAX {
                        (stepv[plan.lane_of[o].0], plan.lane_of[o].1 as u8)
                    } else {
                        (vvec[o].expect("scalarize: value-producing op computed first"), 0)
                    };
                    let v = fb.ins().extractlane(vecv, l);
                    cur[o] = Some(v);
                    v
                }
            }};
        }
        // scalar ops in arena order (grouped members already computed)
        let mut sd: Vec<Value> = Vec::with_capacity(nsd);
        let mut ci = 0usize;
        for j in 0..nops {
            if plan.lane_of[j].0 != usize::MAX {
                continue;
            }
            if plan.alias[j].0 != usize::MAX {
                // absorbed pair-sub: its value IS a lane of another op's
                // pair-vector; `scalarize` extracts there — compute nothing
                // (and leave `cur[j]` None so consumers take the alias).
                continue;
            }
            let newv = match plan.ops[j].e {
                OpE::Ema {
                    src,
                    mult,
                    inv,
                    feed,
                    chained,
                } => {
                    let slot = slot_of_op[j];
                    let prev = bp[slot];
                    macro_rules! step_ema {
                        () => {{
                            let src_v = if src == PRICE { price_v } else { scalarize!(src) };
                            let m = fb.ins().f64const(mult);
                            let iv = fb.ins().f64const(inv);
                            let prod = fb.ins().fmul(src_v, m);
                            fb.ins().fma(prev, iv, prod)
                        }};
                    }
                    if !chained {
                        step_ema!()
                    } else {
                        let sdj = bp[sdpos + ci];
                        ci += 1;
                        if b < feed as i64 {
                            prev // source reports no bars of this segment
                        } else if b == feed as i64 {
                            // feeding starts here: first bar seeds
                            let stepped = step_ema!();
                            let is_sd = fb.ins().icmp(IntCC::NotEqual, sdj, zero);
                            let src_v = if src == PRICE { price_v } else { scalarize!(src) };
                            fb.ins().select(is_sd, stepped, src_v)
                        } else {
                            step_ema!() // sd proven 1: pure step
                        }
                    }
                }
                OpE::Arith { op, a, b, vec } => {
                    if vec {
                        // E2-lite pair-sub: a/b are GROUP ids; one fsub on
                        // THIS bar's stepped lane vectors yields both line
                        // values (`bp` holds the previous bar).
                        let v = fb.ins().fsub(stepv[a], stepv[b]);
                        vvec[j] = Some(v);
                        cur[j] = Some(fb.ins().extractlane(v, 0));
                        continue;
                    }
                    let av = scalarize!(a);
                    let bv = scalarize!(b);
                    match op {
                        ArithKind::Sub => fb.ins().fsub(av, bv),
                        ArithKind::Mul => fb.ins().fmul(av, bv),
                        ArithKind::Div => fb.ins().fdiv(av, bv),
                        // IEEE maxNum — Rust's `f64::max` exactly.
                        ArithKind::Max => fb.ins().fmax(av, bv),
                    }
                }
                OpE::Const(v) => fb.ins().f64const(v),
            };
            cur[j] = Some(newv);
        }
        // Seed flags for the next edge, in `chained` order.
        for (i, &cj) in chained.iter().enumerate() {
            let feed = match plan.ops[cj].e {
                OpE::Ema { feed, .. } => feed as i64,
                _ => unreachable!(),
            };
            if b < feed {
                sd.push(bp[sdpos + i]);
            } else {
                sd.push(one);
            }
        }

        for (k, sink) in plan.sinks.iter().enumerate() {
            if sink.lead >= LEAD_NEVER || sink.lead as i64 > b {
                continue; // not printing during this segment (compile-time)
            }
            let idx = fb.ins().isub(ivar, sbase[k]);
            let soff = fb.ins().imul(idx, eight);
            let saddr = fb.ins().iadd(optr[k], soff);
            let v = scalarize!(sink.val);
            fb.ins().store(MemFlagsData::new(), v, saddr, 0);
        }
        let i1 = fb.ins().iadd(ivar, one);
        let mut la: Vec<Value> = Vec::with_capacity(nparam);
        la.extend(stepv.iter().copied());
        for &o in &op_of_slot {
            la.push(cur[o].expect("state-carried op has a value"));
        }
        la.extend(sd.iter().copied());
        la.push(i1);
        fb.ins().jump(head_bb[s], &ba(&la));
    }

    // ---- end: store state once (group vectors extracted back per chain) ----
    fb.switch_to_block(end_bb);
    let endp: Vec<Value> = fb.block_params(end_bb).to_vec();
    for g in 0..ngroups {
        for (l, &o) in plan.groups[g].ops.iter().enumerate() {
            let v = fb.ins().extractlane(endp[g], l as u8);
            st(&mut fb, state, 8 * plan.ops[o].ema_word as i64, v);
        }
    }
    for (si, &o) in op_of_slot.iter().enumerate() {
        if matches!(plan.ops[o].e, OpE::Ema { .. }) {
            st(
                &mut fb,
                state,
                8 * plan.ops[o].ema_word as i64,
                endp[ngroups + si],
            );
        }
    }
    for (i, &j) in chained.iter().enumerate() {
        st(
            &mut fb,
            state,
            8 * plan.ops[j].sd_word as i64,
            endp[nval + i],
        );
    }
    st(&mut fb, state, 8 * W_SEED as i64, endp[nval + nsd]);
    let nb = fb.ins().iadd(next_bar, len);
    st(&mut fb, state, 8 * W_NEXT as i64, nb);
    fb.ins().return_(&[]);

    fb.seal_all_blocks();
    fb.finalize(fc);

    plan.clif = ctx.func.to_string();
    module
        .define_function(id, &mut ctx)
        .expect("define_function");
    module.clear_context(&mut ctx);
    module.finalize_definitions().expect("jit finalize");
    plan.func =
        Some(unsafe { std::mem::transmute::<*const u8, JitFn>(module.get_finalized_function(id)) });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cse::Planable;
    use crate::indicator_types::Indicator;
    use crate::indicators::{apo::Apo, ema::Ema, macd::Macd, ppo::Ppo};

    const APO: [f64; 2] = [12.0, 26.0];
    const MACD: [f64; 3] = [12.0, 26.0, 9.0];
    const PPO: [f64; 2] = [12.0, 26.0];

    fn series(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| ((i as f64 * 0.7).sin() * 10.0 + 100.0) * (1.0 + (i % 7) as f64 * 0.0003))
            .collect()
    }

    fn cmp_exact(a: &[f64], b: &[f64], what: &str) {
        assert_eq!(a.len(), b.len(), "{what}: len {} vs {}", a.len(), b.len());
        for i in 0..a.len() {
            assert_eq!(
                a[i].to_bits(),
                b[i].to_bits(),
                "{what}: mismatch at {i}: {:?} vs {:?}",
                a[i],
                b[i]
            );
        }
    }

    fn classic() -> Request {
        Request::new()
            .add_opt(Apo::recipe(&APO), None)
            .add_opt(Ppo::recipe(&PPO), None)
            .add_opt(Macd::recipe(&MACD), None)
    }

    /// The engine gate: the fused compiled loop over the CSE'd arena matches
    /// the three hand `::indicator()` calls bit-for-bit (primary lines, with
    /// the optional switch off — the benchmark shape).
    #[test]
    fn jit_classic_matches_hand() {
        let close = series(500);
        let s: [&[f64]; 1] = [close.as_slice()];
        let (res, _) = classic().build().run(&s);

        cmp_exact(
            res.request(0).output(0),
            &Apo::indicator(&s, &APO, None).unwrap().0[0],
            "apo_line",
        );
        cmp_exact(
            res.request(1).output(0),
            &Ppo::indicator(&s, &PPO, None).unwrap().0[0],
            "ppo_line",
        );
        let macd = Macd::indicator(&s, &MACD, None).unwrap().0;
        cmp_exact(res.request(2).output(0), &macd[0], "macd_line");
        cmp_exact(res.request(2).output(1), &macd[1], "macd_signal");
        cmp_exact(res.request(2).output(2), &macd[2], "macd_hist");
        for req in 0..3 {
            for o in 0..res.optional[req].len() {
                assert!(
                    res.request(req).optional(o).is_empty(),
                    "disabled opt printed"
                );
            }
        }
    }

    /// Streaming contract: `run ++ resume == run` for every split (seed
    /// flags, chained signal feeding, and `next_bar`-absolute gating all
    /// survive the chunk boundary), bit-exact on primaries.
    #[test]
    fn jit_classic_streaming_matches_full_run() {
        let close = series(500);
        let s_all: [&[f64]; 1] = [close.as_slice()];
        let plan = classic().build();
        let (full, _) = plan.run(&s_all);
        // Split points deliberately straddle every phase boundary
        // (leads 25/26/33): a chunk ending AT a boundary, one starting at
        // one, and one crossing two.
        for split in [90usize, 140, 20, 33, 25, 26, 27, 24, 34] {
            let s1: [&[f64]; 1] = [&close[..split]];
            let s2: [&[f64]; 1] = [&close[split..]];
            let (h1, qs) = plan.run(&s1);
            let (h2, _) = plan.resume(qs, &s2);
            for req in 0..full.outputs.len() {
                for out in 0..full.outputs[req].len() {
                    let mut c = h1.outputs[req][out].clone();
                    c.extend_from_slice(&h2.outputs[req][out]);
                    cmp_exact(
                        &c,
                        &full.outputs[req][out],
                        &format!("split{split} p{req}o{out}"),
                    );
                }
            }
        }
    }

    /// Every optional line and the standalone `Ema` requests: one merged
    /// plan computing ema12/ema26 ONCE for five consumers, matching all
    /// hand arrays bit-for-bit including lengths (ppo's optional long line
    /// is a raw EMA mirror with the hand's explicit lead, the ema lines are
    /// the hand `Ema` primary shape).
    #[test]
    fn jit_mirrors_and_ema_requests_match_hand() {
        let close = series(500);
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = Request::new()
            .add(Apo::recipe(&APO))
            .add(Ppo::recipe(&PPO))
            .add(Macd::recipe(&MACD))
            .add_opt(Ema::recipe(&[12.0]), None)
            .add_opt(Ema::recipe(&[26.0]), None)
            .build();
        let (res, st) = plan.run(&s);

        let flags = Some(&[true, true][..]);
        let apo = Apo::indicator(&s, &APO, flags).unwrap().0;
        cmp_exact(res.request(0).output(0), &apo[0], "apo_line");
        cmp_exact(res.request(0).optional(0), &apo[1], "apo_short");
        cmp_exact(res.request(0).optional(1), &apo[2], "apo_long");
        let ppo = Ppo::indicator(&s, &PPO, flags).unwrap().0;
        cmp_exact(res.request(1).output(0), &ppo[0], "ppo_line");
        cmp_exact(res.request(1).optional(0), &ppo[1], "ppo_short");
        cmp_exact(res.request(1).optional(1), &ppo[2], "ppo_long_mirror");
        let macd = Macd::indicator(&s, &MACD, flags).unwrap().0;
        for o in 0..3 {
            cmp_exact(res.request(2).output(o), &macd[o], &format!("macd{o}"));
        }
        cmp_exact(res.request(2).optional(0), &macd[3], "macd_short");
        cmp_exact(res.request(2).optional(1), &macd[4], "macd_long");
        cmp_exact(
            res.request(3).output(0),
            &Ema::indicator(&s, &[12.0], None).unwrap().0[0],
            "ema12",
        );
        cmp_exact(
            res.request(4).output(0),
            &Ema::indicator(&s, &[26.0], None).unwrap().0[0],
            "ema26",
        );

        // one authority per node: the standalone Ema line IS the mirror.
        cmp_exact(
            res.request(3).output(0),
            res.request(0).optional(0),
            "ema12_eq_mirror",
        );

        // get_states: chain slots reconstitute the hand states, matching the
        // published line ends.
        let states = plan.get_states(&st);
        let IndicatorStateBox::Ema(e12) = &states[3] else {
            panic!("ema state")
        };
        assert_eq!(
            e12.ema.to_bits(),
            res.request(3).output(0).last().unwrap().to_bits()
        );
        let IndicatorStateBox::Macd(mc) = &states[2] else {
            panic!("macd state")
        };
        assert_eq!(
            mc.signal_state.ema.to_bits(),
            res.request(2).output(1).last().unwrap().to_bits()
        );
        assert_eq!(
            mc.long_ema.ema.to_bits(),
            res.request(4).output(0).last().unwrap().to_bits()
        );
    }

    /// The dense dashboard (6 requests, 11 distinct EMAs + 2 chained
    /// signals — the `run_dense_dashboard` / `by_options::<2>` benchmark
    /// shape) matches the six separate hand calls, primaries, bit-for-bit.
    #[test]
    fn jit_dense_dashboard_matches_hand() {
        let close = series(500);
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = Request::new()
            .add_opt(Apo::recipe(&[5.0, 8.0]), None)
            .add_opt(Apo::recipe(&[6.0, 9.0]), None)
            .add_opt(Ppo::recipe(&[9.0, 25.0]), None)
            .add_opt(Ppo::recipe(&[10.0, 26.0]), None)
            .add_opt(Macd::recipe(&[11.0, 23.0, 17.0]), None)
            .add_opt(Macd::recipe(&[12.0, 24.0, 9.0]), None)
            .build();
        let (res, _) = plan.run(&s);

        let hand: Vec<Vec<Vec<f64>>> = vec![
            Apo::indicator(&s, &[5.0, 8.0], None).unwrap().0,
            Apo::indicator(&s, &[6.0, 9.0], None).unwrap().0,
            Ppo::indicator(&s, &[9.0, 25.0], None).unwrap().0,
            Ppo::indicator(&s, &[10.0, 26.0], None).unwrap().0,
            Macd::indicator(&s, &[11.0, 23.0, 17.0], None).unwrap().0,
            Macd::indicator(&s, &[12.0, 24.0, 9.0], None).unwrap().0,
        ];
        // hand returns declared EMPTY optional slots too — compare primaries
        // only (1,1,1,1,3,3 by Info::outputs).
        let np = [1, 1, 1, 1, 3, 3];
        for r in 0..6 {
            for o in 0..np[r] {
                cmp_exact(
                    res.request(r).output(o),
                    &hand[r][o],
                    &format!("dense r{r} o{o}"),
                );
            }
        }
    }

    /// Empty chunks are true no-ops (state words unchanged, no rows).
    #[test]
    fn jit_empty_chunk_is_noop() {
        let close = series(300);
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = classic().build();
        let (_full, st_a) = plan.run(&s);
        let empty: [&[f64]; 1] = [&[]];
        let (res, st_b) = plan.resume(st_a.clone(), &empty);
        assert_eq!(st_b.words, st_a.words, "empty chunk changed state");
        for v in &res.outputs {
            for line in v {
                assert!(line.is_empty(), "empty chunk printed rows");
            }
        }
    }

    /// Sub-epsilon prices: the one domain where "clamp guards only the
    /// divisor" (`(s−l)*100/max(l,ε)`) DISAGREES with the old
    /// "clamp feeds the subtraction" (`(s−max(l,ε))*100/max(l,ε)`) and the
    /// raw-vs-clamped `long_ema` optional. Bit-exact parity here pins the
    /// emitter to the hand `Ppo::State::calc` shape, not just to sane data.
    #[test]
    fn jit_ppo_matches_hand_below_epsilon() {
        let tiny: Vec<f64> = (0..300)
            .map(|i| ((i as f64 * 0.7).sin() * 0.5 + 0.5).mul_add(1e-18, 3e-19))
            .collect();
        let s: [&[f64]; 1] = [tiny.as_slice()];
        let plan = Request::new()
            .add_opt(Ppo::recipe(&PPO), Some(&[true, true]))
            .build();
        let (res, _) = plan.run(&s);
        let ppo = Ppo::indicator(&s, &PPO, Some(&[true, true])).unwrap().0;
        cmp_exact(res.request(0).output(0), &ppo[0], "ppo_line_tiny");
        cmp_exact(res.request(0).optional(0), &ppo[1], "ppo_short_tiny");
        cmp_exact(res.request(0).optional(1), &ppo[2], "ppo_long_raw_tiny");
    }

    /// `Apo ≡ macd_line ≡ ppo numerator`: on the classic query the three
    /// indicators' subtractions intern to exactly ONE `Sub` op (plus macd's
    /// histogram), and the optional EMA mirrors make `ema12`/`ema26` feed
    /// five consumers. Counting ops pins the sharing itself — parity of
    /// VALUES cannot catch a regressed duplicate.
    #[test]
    fn jit_classic_interns_one_sub_for_three_indicators() {
        let plan = Request::new()
            .add(Apo::recipe(&APO))
            .add(Ppo::recipe(&PPO))
            .add(Macd::recipe(&MACD))
            .build();
        let (nops, nsinks) = plan.info();
        // Classic arena as NODES: Input, ema12, ema26, ONE shared line Sub,
        // Const eps, Max (ppo divisor guard), Const 100, Mul, Div, chained
        // signal Ema, hist Sub = 11 nodes ⇒ 10 ops (no op for the Input).
        let arena = plan.arena();
        let n_ema = arena
            .nodes
            .iter()
            .filter(|n| matches!(n, Node::Ema { .. }))
            .count();
        let n_sub = arena
            .nodes
            .iter()
            .filter(|n| matches!(n, Node::Sub { .. }))
            .count();
        assert_eq!(n_ema, 3, "ema12, ema26, chained signal — nothing else");
        assert_eq!(
            n_sub, 2,
            "ONE Sub shared by apo line, macd line, ppo numerator — plus hist"
        );
        assert_eq!(nops, 10);
        assert_eq!(nsinks, 11);
    }

    /// E1/E2-lite shape: classic's two price chains pack into one `f64x2`
    /// group (its lone line sub has no partner to pair with, so it stays a
    /// scalar consumer of the lanes); dense's six line-subs form three
    /// role-aligned pairs → six f64x2 groups covering all 11 close EMAs
    /// plus ONE clone (ema9 feeds both an apo long and a ppo short) and
    /// three vector `fsub`s replacing six scalar ones. Chained signals
    /// never pack (producer in the same bar). Value parity across all
    /// suites above now runs THROUGH the packed stepping — lane-wise
    /// `fma` is the same `mul_add` per lane.
    #[test]
    fn jit_lanes_pack_price_chains() {
        assert_eq!(
            classic().build().lanes(),
            vec![2],
            "ema12+ema26 in one f64x2"
        );
        assert_eq!(classic().build().vec_subs(), 0, "lone sub stays scalar");
        let dense = Request::new()
            .add_opt(Apo::recipe(&[5.0, 8.0]), None)
            .add_opt(Apo::recipe(&[6.0, 9.0]), None)
            .add_opt(Ppo::recipe(&[9.0, 25.0]), None)
            .add_opt(Ppo::recipe(&[10.0, 26.0]), None)
            .add_opt(Macd::recipe(&[11.0, 23.0, 17.0]), None)
            .add_opt(Macd::recipe(&[12.0, 24.0, 9.0]), None)
            .build();
        assert_eq!(
            dense.lanes(),
            vec![2, 2, 2, 2, 2, 2],
            "three role-aligned sub pairs → six f64x2 groups"
        );
        assert_eq!(dense.vec_subs(), 3, "one vector fsub per pair");
        // 11 distinct close chains + 1 clone (shared ema9) = 12 lane slots
        let slots: usize = dense.lanes().iter().sum();
        assert_eq!(slots, 12, "clone absorbs the cross-request chain reuse");
    }
}
