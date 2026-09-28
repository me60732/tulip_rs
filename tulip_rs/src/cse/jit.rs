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
//! Emitted shape:
//!
//! ```text
//! setup (once per chunk): state loaded; seeded/first-valid selects; per-sink
//!   first-row offsets g0 = sat_sub(lead, next_bar) and out pointers — all
//!   setup values DOMINATE the loop, so loop-carried params are just
//!   [op values.., chained seed flags.., i]:
//! for i in 0..len:
//!   price = load
//!   ops in topological order:
//!     Ema  : stepped = fma(prev, inv, src*mult)      // ← `mul_add`, ONE FMA
//!            chained (e.g. macd signal): step/seed only once src reports
//!     Sub  : a - b                                    // apo/line/histogram
//!     Clamp: max(b, EPSILON)                           // ppo
//!     Ppo  : (a - clamp)*100.0 / clamp                // op-for-op hand order
//!   sinks: if i >= g0: out[i - g0] = op value          // lead-gated stores
//!          (gate/store block chain; always-store-with-selected-address
//!           measured 18→24µs WORSE — dead writes tax the store buffer)
//! state stored once → ret
//! ```
//!
//! Bit-exactness doctrine: `ema::calc` is `prev.mul_add(inv, value*mult)` →
//! `fmul` + `fma` (never mul+add); the §5.3 lifecycle (seed on first valid
//! source bar, chained recurrences fed only past their source's publish
//! lead, `next_bar`-absolute gating) is the lifecycle the hand indicators
//! define — every parity test here is against HAND, bit-for-bit (`to_bits`),
//! mirroring the hand-derived leads and the `calc_cse` bodies.
//!
//! Scope: closed kind set `"ema" | "apo" | "ppo" | "macd" | ""` (mirror-only)
//! over `Input`/`Ema` arena nodes. Unknown kinds/nodes panic at build —
//! emitting the reserved arith/filter `Node` kinds is the coverage roadmap
//! (Handover §6 Step E). Sources are
//! assumed complete (producer-NaN gating would be one more select per node);
//! lane packing (`f64x2` vector `fma` over same-source EMAs) is the next
//! axis on top of this structure.

use std::collections::HashMap;

use cranelift::codegen::ir::{BlockArg, Function, MemFlagsData, UserFuncName};
use cranelift::codegen::isa::TargetFrontendConfig;
use cranelift::codegen::Context;
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};

use crate::cse::query::{IndicatorStateBox, Request, Results};
use crate::cse::{merge, Merged, Node, Sink, SinkSrc};
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

/// One loop op: an EMA recurrence (state) or a pure derived value.
struct Op {
    e: OpE,
    /// Absolute bar the value first exists (chain defined-from / tail's
    /// max-of-sources lead).
    lead: u64,
    /// State-word indices (EMA ops only).
    ema_word: usize,
    sd_word: usize,
}

enum OpE {
    /// `prev*inv + src*mult` — the `mul_add` recurrence. `chained` = the
    /// source is another op: step/seed only once the source reports
    /// (`i >= feed`), otherwise advance every bar (price source is always
    /// there; the chunk's first bar seeds an unseeded chain in setup).
    Ema {
        src: usize,
        mult: f64,
        inv: f64,
        feed: u64,
        chained: bool,
    },
    /// `a - b` (apo line, macd line, histogram).
    Sub { a: usize, b: usize },
    /// `max(b, EPSILON)` — ppo's clamped long_ema slot.
    Clamp { b: usize },
    /// `(a - max(b, EPSILON)) * 100.0 / max(b, EPSILON)` — `PpoTail`, op-for-op.
    Ppo { a: usize, b: usize },
}

/// One compiled sink: value op + publish lead (clamped `max(declared,
/// value lead)`; `LEAD_NEVER` when disabled).
struct SinkSpec {
    val: usize,
    lead: u64,
}

/// Per-request metadata for `get_states` reconstitution.
struct ReqMeta {
    kind: &'static str,
    options: Vec<f64>,
    /// Op index per `Resolved::inputs` entry (tail inputs in order).
    input_ops: Vec<usize>,
    /// The `Ema` mirror request's chain op.
    mirror_op: Option<usize>,
    /// macd's signal-chain op.
    sig_op: Option<usize>,
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

/// Merge `Request` (CSE arena + leads + sink wiring) and compile it into one
/// fused loop. Panics on shapes outside the emitter's scope.
fn compile_request(req: &Request) -> JitPlan {
    let (arena, mut resolved) = merge(&req.recipes);
    // same switch `Request::build` applies — disabled optionals compile to
    // never-print sinks (EMPTY in position, values unchanged elsewhere).
    for (r, flags) in resolved.iter_mut().zip(&req.opt_flags) {
        r.optional_enabled = flags.clone();
    }
    let leads = arena.leads();

    // ---- pass 1: arena EMA chains, SSA order (producers are lower ids) ----
    let mut ops: Vec<Op> = Vec::new();
    let mut node_op: Vec<usize> = vec![PRICE; arena.nodes.len()]; // Input → PRICE
    for (id, node) in arena.nodes.iter().enumerate() {
        if let Node::Ema { src, period } = *node {
            let (mult, inv) = multiplier(period as usize);
            let src_op = node_op[src as usize];
            let chained = src_op != PRICE;
            let feed = leads[src as usize] as u64;
            ops.push(Op {
                e: OpE::Ema {
                    src: src_op,
                    mult,
                    inv,
                    feed,
                    chained,
                },
                lead: leads[id] as u64,
                ema_word: 0,
                sd_word: 0,
            });
            node_op[id] = ops.len() - 1;
        }
    }

    // ---- pass 2: per-request tail ops, mirrors, sinks ----
    let mut sinks: Vec<SinkSpec> = Vec::new();
    let mut counts = Vec::new();
    let mut reqs: Vec<ReqMeta> = Vec::new();
    let mut opcache: HashMap<(u8, usize, usize), usize> = HashMap::new();

    for res in &resolved {
        let input_ops: Vec<usize> = res
            .inputs
            .iter()
            .map(|&i| {
                let o = node_op[i as usize];
                assert!(o != PRICE, "jit: tail input {i} is not an EMA chain");
                o
            })
            .collect();
        let first_bar: u64 = input_ops.iter().map(|&o| ops[o].lead).max().unwrap_or(0);

        // Tail value ops for this request, indexed by `SinkSrc::Tail(slot)`.
        let mut tvs: Vec<usize> = Vec::new();
        let mut mirror_op = None;
        let mut sig_op = None;

        match res.kind {
            "" => {
                assert!(
                    res.inputs.is_empty(),
                    "jit: mirror-only request must have no tail inputs"
                );
            }
            "ema" => match res.outputs[0].src {
                SinkSrc::Interface(id) => mirror_op = Some(node_op[id as usize]),
                _ => panic!("jit: `ema` request must mirror an interface node"),
            },
            "apo" => {
                // slot 0: line = short − long.
                tvs.push(intern_op(
                    &mut ops,
                    &mut opcache,
                    OpE::Sub {
                        a: input_ops[0],
                        b: input_ops[1],
                    },
                    first_bar,
                ));
            }
            "ppo" => {
                // slot 0 = scaled, slot 1 = clamped long (hand `PpoTail`).
                let clamp = intern_op(
                    &mut ops,
                    &mut opcache,
                    OpE::Clamp { b: input_ops[1] },
                    first_bar,
                );
                let scaled = intern_op(
                    &mut ops,
                    &mut opcache,
                    OpE::Ppo {
                        a: input_ops[0],
                        b: clamp,
                    },
                    first_bar,
                );
                tvs.push(scaled);
                tvs.push(clamp);
            }
            "macd" => {
                let sp = res.options[2] as usize;
                let (mult, inv) = multiplier(sp);
                // slot 0 = line; signal = chained EMA over the line; slot 2 =
                // hist = line − signal. The line `Sub` interns against APO's
                // identical subtraction — `Apo ≡ macd_line` is ONE fsub.
                let line = intern_op(
                    &mut ops,
                    &mut opcache,
                    OpE::Sub {
                        a: input_ops[0],
                        b: input_ops[1],
                    },
                    first_bar,
                );
                let k = intern_op(
                    &mut ops,
                    &mut opcache,
                    OpE::Ema {
                        src: line,
                        mult,
                        inv,
                        feed: first_bar,
                        chained: true,
                    },
                    first_bar + sp as u64 - 1,
                );
                sig_op = Some(k);
                let hist = intern_op(
                    &mut ops,
                    &mut opcache,
                    OpE::Sub { a: line, b: k },
                    first_bar + sp as u64 - 1,
                );
                tvs.push(line);
                tvs.push(k);
                tvs.push(hist);
            }
            other => panic!("jit: kind {other:?} not in the closed emittable set"),
        }

        // Sinks: primaries then ALL declared optionals, in order — disabled
        // ones keep their position as never-print slots (`Results` shape).
        let mut add = |list: &[Sink], enabled: Option<&[bool]>| {
            for (oi, s) in list.iter().enumerate() {
                if enabled.is_some_and(|f| !f.get(oi).copied().unwrap_or(true)) {
                    sinks.push(SinkSpec {
                        val: 0,
                        lead: LEAD_NEVER,
                    });
                    continue;
                }
                let (val, val_lead) = match s.src {
                    SinkSrc::Tail(slot) => {
                        let o = tvs[slot as usize];
                        (o, ops[o].lead)
                    }
                    SinkSrc::Interface(id) => {
                        let o = node_op[id as usize];
                        assert!(o != PRICE, "jit: mirror of a non-EMA node");
                        (o, ops[o].lead)
                    }
                };
                // hand invariant (slot_lengths/output_length): publish lead
                // = max(declared, value lead)
                sinks.push(SinkSpec {
                    val,
                    lead: (s.lead as u64).max(val_lead),
                });
            }
        };
        add(&res.outputs, None);
        add(&res.optional_outputs, Some(&res.optional_enabled));

        counts.push((res.outputs.len(), res.optional_outputs.len()));
        reqs.push(ReqMeta {
            kind: res.kind,
            options: res.options.clone(),
            input_ops,
            mirror_op,
            sig_op,
        });
    }

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
    let isa_builder = cranelift_native::builder().expect("host ISA builder");
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
        sinks,
        counts,
        reqs,
        nwords,
    };
    emit(&mut plan, fc);
    plan
}

/// Structural key for tail-op interning: identical expression on identical
/// value sources ⇒ ONE emitted instruction shared by every consumer
/// (`Apo ≡ macd_line` dedups inside the loop, not just across requests).
fn op_key(e: &OpE) -> (u8, usize, usize) {
    match e {
        // mult bits carry the period: same src + same period = same chain.
        OpE::Ema { src, mult, .. } => (1, *src, mult.to_bits() as usize),
        OpE::Sub { a, b } => (2, *a, *b),
        OpE::Clamp { b } => (3, *b, 0),
        OpE::Ppo { a, b } => (4, *a, *b),
    }
}

fn intern_op(
    ops: &mut Vec<Op>,
    cache: &mut HashMap<(u8, usize, usize), usize>,
    e: OpE,
    lead: u64,
) -> usize {
    let key = op_key(&e);
    if let Some(&j) = cache.get(&key) {
        debug_assert_eq!(ops[j].lead, lead, "interned op with divergent lead");
        return j;
    }
    ops.push(Op {
        e,
        lead,
        ema_word: 0,
        sd_word: 0,
    });
    let j = ops.len() - 1;
    cache.insert(key, j);
    j
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

    /// Reconstitute hand states from the chain slots — one authority per
    /// arena chain, matching the published lines exactly.
    pub fn get_states(&self, st: &JitState) -> Vec<IndicatorStateBox> {
        use crate::indicators::{apo, macd, ppo};
        self.reqs
            .iter()
            .map(|r| match r.kind {
                "ema" => match r.mirror_op {
                    Some(o) => {
                        IndicatorStateBox::Ema(ema_state(self.ema_of(st, o), r.options[0] as usize))
                    }
                    None => IndicatorStateBox::Other,
                },
                "apo" if r.input_ops.len() == 2 => IndicatorStateBox::Apo(apo::State {
                    short_ema: ema_state(self.ema_of(st, r.input_ops[0]), r.options[0] as usize),
                    long_ema: ema_state(self.ema_of(st, r.input_ops[1]), r.options[1] as usize),
                }),
                "ppo" if r.input_ops.len() == 2 => IndicatorStateBox::Ppo(ppo::State {
                    short_ema: ema_state(self.ema_of(st, r.input_ops[0]), r.options[0] as usize),
                    long_ema: ema_state(self.ema_of(st, r.input_ops[1]), r.options[1] as usize),
                }),
                "macd" if r.input_ops.len() == 2 && r.sig_op.is_some() => {
                    IndicatorStateBox::Macd(macd::State {
                        short_ema: ema_state(
                            self.ema_of(st, r.input_ops[0]),
                            r.options[0] as usize,
                        ),
                        long_ema: ema_state(self.ema_of(st, r.input_ops[1]), r.options[1] as usize),
                        signal_state: ema_state(
                            self.ema_of(st, r.sig_op.unwrap()),
                            r.options[2] as usize,
                        ),
                    })
                }
                _ => IndicatorStateBox::Other,
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

/// `sat_sub(lead, start)` computed once in setup (dominates the loop).
fn satsub(fb: &mut FunctionBuilder, lead_c: Value, start: Value, zero: Value) -> Value {
    let d = fb.ins().isub(lead_c, start);
    let neg = fb.ins().icmp(IntCC::SignedLessThan, d, zero);
    fb.ins().select(neg, zero, d)
}

/// Build and define the plan's function; sets `plan.func`.
fn emit(plan: &mut JitPlan, fc: TargetFrontendConfig) {
    let module = &mut plan.module;
    let i64_t = types::I64;
    let f64_t = types::F64;

    let nops = plan.ops.len();
    let nsinks = plan.sinks.len();
    let chained: Vec<usize> = (0..nops)
        .filter(|&j| matches!(plan.ops[j].e, OpE::Ema { chained: true, .. }))
        .collect();
    let nsd = chained.len();

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

    // Loop-carried params: [ op values.., chained seeds.., i ]
    let mut pt: Vec<Type> = vec![f64_t; nops];
    pt.extend(std::iter::repeat(i64_t).take(nsd));
    pt.push(i64_t);
    let nparam = pt.len();
    let ipos = nparam - 1;
    let sdpos = nops;

    let entry = fb.create_block();
    let setup = fb.create_block();
    let loop_bb = fb.create_block();
    let body_bb = fb.create_block();
    let after_bb = fb.create_block();
    let end_bb = fb.create_block();
    let mut gate_bb = Vec::with_capacity(nsinks);
    let mut store_bb = Vec::with_capacity(nsinks);
    for _ in 0..nsinks {
        gate_bb.push(fb.create_block());
        store_bb.push(fb.create_block());
    }

    fb.append_block_params_for_function_params(entry);
    for b in [loop_bb, body_bb, after_bb] {
        add_params(&mut fb, b, &pt);
    }
    for &g in gate_bb.iter().chain(store_bb.iter()) {
        add_params(&mut fb, g, &pt);
    }
    // end: values + seeds + the (unchanged-or-set) global seeded flag
    let mut et = vec![f64_t; nops];
    et.extend(std::iter::repeat(i64_t).take(nsd));
    et.push(i64_t);
    add_params(&mut fb, end_bb, &et);
    // (blocks are sealed once at the end — predecessors arrive as edges are
    // emitted, several of them late)

    // ---- entry ----
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
    let mut end_empty = loads.clone();
    end_empty.extend(sdloads.iter().copied());
    end_empty.push(seeded);
    fb.ins()
        .brif(empty, end_bb, &ba(&end_empty), setup, &ba(&[]));

    // ---- setup ----
    fb.switch_to_block(setup);
    let is_seeded = fb.ins().icmp(IntCC::NotEqual, seeded, zero);
    let p0 = ld(&mut fb, price, 0, f64_t);
    // per-op initial value + gates (entry values dominate the whole CFG)
    let mut init: Vec<Value> = Vec::with_capacity(nops);
    let mut feed_g: Vec<Value> = Vec::with_capacity(nops);
    for j in 0..nops {
        // feed_g[j] aligns with ops[j] (only chained EMAs read it).
        let (v, fg) = match plan.ops[j].e {
            OpE::Ema { chained: false, .. } => (fb.ins().select(is_seeded, loads[j], p0), zero),
            OpE::Ema {
                feed,
                chained: true,
                ..
            } => {
                let lc = fb.ins().iconst(i64_t, feed as i64);
                (loads[j], satsub(&mut fb, lc, next_bar, zero))
            }
            _ => (fb.ins().f64const(0.0), zero),
        };
        init.push(v);
        feed_g.push(fg);
    }
    let i_init = fb.ins().select(is_seeded, zero, one);
    let mut g0: Vec<Value> = Vec::with_capacity(nsinks);
    let mut optr: Vec<Value> = Vec::with_capacity(nsinks);
    for (j, s) in plan.sinks.iter().enumerate() {
        let lead_i: i64 = if s.lead > (i64::MAX as u64) / 2 {
            i64::MAX
        } else {
            s.lead as i64
        };
        let lc = fb.ins().iconst(i64_t, lead_i);
        g0.push(satsub(&mut fb, lc, next_bar, zero));
        optr.push(ld(&mut fb, outs, 8 * j as i64, i64_t));
    }
    let mut args = init;
    args.extend(sdloads.iter().copied());
    args.push(i_init);
    fb.ins().jump(loop_bb, &ba(&args));

    // ---- loop head ----
    fb.switch_to_block(loop_bb);
    let lp: Vec<Value> = fb.block_params(loop_bb).to_vec();
    let ivar = lp[ipos];
    let done = fb.ins().icmp(IntCC::SignedGreaterThanOrEqual, ivar, len);
    let mut end_done: Vec<Value> = lp[..nops + nsd].to_vec();
    end_done.push(one);
    fb.ins()
        .brif(done, end_bb, &ba(&end_done), body_bb, &ba(&lp));

    // ---- body: ops in topological order ----
    fb.switch_to_block(body_bb);
    let bp: Vec<Value> = fb.block_params(body_bb).to_vec();
    let poff = fb.ins().imul(bp[ipos], eight);
    let paddr = fb.ins().iadd(price, poff);
    let price_v = fb.ins().load(f64_t, MemFlagsData::new(), paddr, 0);

    let mut cur = bp[..nops].to_vec();
    let mut sd: Vec<Value> = Vec::with_capacity(nsd);
    let mut ci = 0usize;
    for j in 0..nops {
        let prev = bp[j];
        let newv = match plan.ops[j].e {
            OpE::Ema {
                src,
                mult,
                inv,
                chained,
                ..
            } => {
                let src_v = if src == PRICE { price_v } else { cur[src] };
                let m = fb.ins().f64const(mult);
                let iv = fb.ins().f64const(inv);
                let prod = fb.ins().fmul(src_v, m);
                let stepped = fb.ins().fma(prev, iv, prod);
                if !chained {
                    stepped
                } else {
                    let sdj = bp[sdpos + ci];
                    ci += 1;
                    let g = feed_g[j];
                    let fed_c = fb.ins().icmp(IntCC::SignedGreaterThanOrEqual, bp[ipos], g);
                    let is_sd = fb.ins().icmp(IntCC::NotEqual, sdj, zero);
                    let fed = fb.ins().select(is_sd, stepped, src_v);
                    let nv = fb.ins().select(fed_c, fed, prev);
                    let sd_next = fb.ins().select(fed_c, one, sdj);
                    sd.push(sd_next);
                    nv
                }
            }
            OpE::Sub { a, b } => {
                let av = cur[a];
                let bv = cur[b];
                fb.ins().fsub(av, bv)
            }
            OpE::Clamp { b } => {
                let bv = cur[b];
                let eps = fb.ins().f64const(f64::EPSILON);
                fb.ins().fmax(bv, eps)
            }
            OpE::Ppo { a, b } => {
                // `b` IS the Clamp op's clamped value (pass 2 wires it that
                // way) — no second fmax needed: (a - b) * 100 / b, the hand
                // `PpoTail` expression exactly.
                let av = cur[a];
                let bv = cur[b];
                let d = fb.ins().fsub(av, bv);
                let c100 = fb.ins().f64const(100.0);
                let num = fb.ins().fmul(d, c100);
                fb.ins().fdiv(num, bv)
            }
        };
        cur[j] = newv;
    }
    debug_assert_eq!(sd.len(), nsd);
    debug_assert_eq!(ci, nsd);

    // Sinks: gate → store block chain. The always-store alternative (select
    // the address, write a scratch word when gated off) was MEASURED
    // 18→24µs on classic: two of five sinks are predicated-off every bar,
    // and pushing those dead writes through the store buffer costs more
    // than the near-free not-taken branches — the same reason hand splits
    // its cycles into gated window loops instead of one branchless loop.
    let mut chain_args = cur.clone();
    chain_args.extend(sd.iter().copied());
    chain_args.push(bp[ipos]);
    if nsinks == 0 {
        fb.ins().jump(after_bb, &ba(&chain_args));
    } else {
        fb.ins().jump(gate_bb[0], &ba(&chain_args));
    }

    for j in 0..nsinks {
        fb.switch_to_block(gate_bb[j]);
        let gp: Vec<Value> = fb.block_params(gate_bb[j]).to_vec();
        let print = fb
            .ins()
            .icmp(IntCC::SignedGreaterThanOrEqual, gp[ipos], g0[j]);
        let next = if j + 1 < nsinks {
            gate_bb[j + 1]
        } else {
            after_bb
        };
        fb.ins().brif(print, store_bb[j], &ba(&gp), next, &ba(&gp));

        fb.switch_to_block(store_bb[j]);
        let sp: Vec<Value> = fb.block_params(store_bb[j]).to_vec();
        let idx = fb.ins().isub(sp[ipos], g0[j]);
        let soff = fb.ins().imul(idx, eight);
        let saddr = fb.ins().iadd(optr[j], soff);
        let v = sp[plan.sinks[j].val];
        fb.ins().store(MemFlagsData::new(), v, saddr, 0);
        let next = if j + 1 < nsinks {
            gate_bb[j + 1]
        } else {
            after_bb
        };
        fb.ins().jump(next, &ba(&sp));
    }

    fb.switch_to_block(after_bb);
    let ap: Vec<Value> = fb.block_params(after_bb).to_vec();
    let i1 = fb.ins().iadd(ap[ipos], one);
    let mut la = ap[..ipos].to_vec();
    la.push(i1);
    fb.ins().jump(loop_bb, &ba(&la));

    // ---- end: store state once ----
    fb.switch_to_block(end_bb);
    let endp: Vec<Value> = fb.block_params(end_bb).to_vec();
    for j in 0..nops {
        if matches!(plan.ops[j].e, OpE::Ema { .. }) {
            st(&mut fb, state, 8 * plan.ops[j].ema_word as i64, endp[j]);
        }
    }
    for (i, &j) in chained.iter().enumerate() {
        st(
            &mut fb,
            state,
            8 * plan.ops[j].sd_word as i64,
            endp[nops + i],
        );
    }
    st(&mut fb, state, 8 * W_SEED as i64, endp[nops + nsd]);
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
        for split in [90usize, 140, 20, 33, 25] {
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
    /// hand arrays bit-for-bit including lengths (ppo's long slot is the
    /// CLAMPED mirror, ema lines are the hand `Ema` primary shape).
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
        cmp_exact(res.request(1).optional(1), &ppo[2], "ppo_long_clamped");
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
}
