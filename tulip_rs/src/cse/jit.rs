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
//!       arith: fsub / fmul / fdiv / maxNum over node operands (Max lowers to
//!       `max_num`, not `fmax` — see §2.1) — a `Sub`
//!              declared identically by APO, MACD and PPO is ONE node
//!              (merge interns it: one fsub, up to 3 consumers)
//!       Const  : materialized f64 (max's `0.0`/`EPSILON` guard, the ×100)
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
//! `Max` + the rolling pair `Sma`/`Stddev` (price-sourced rings in the
//! state image, warm fold → steady trick at `lead + 1`) with `Sqrt`/`Fma`
//! graph glue, over any merged recipe graph — the engine carries NO indicator
//! knowledge (not even a `match` on indicator): each recipe's `restorer`
//! fn pointer (defined in the indicator module) rebuilds its hand state.
//! Filter/HT node kinds panic at build — emitting them (and WMA/ring taps)
//! is the coverage roadmap (Handover §6 Step E). Sources are
//! assumed complete (producer-NaN gating would be one more select per node);
//! E1 lane packing is in: price-sourced EMA chains group in `f64x2` pairs
//! stepped by one vector `fma` (cranelift x64's 128-bit ceiling — see
//! `MAX_LANES`), chained recurrences stay scalar. E2-lite pairs `short −
//! long` subs into one vector `fsub` (role-aligned lane groups, absorbed
//! subs alias lane 1). E2 asset lanes (`build_assets`/`run_assets`)
//! replicate the arena over N price streams so cross-asset subs group
//! automatically — but at the f64x2 ceiling every symbol still owns its
//! output buffers (stores do NOT amortize) and A signals + A/2·chains
//! group vectors overflow the 16-xmm file, so the all-assets-one-loop
//! shape spills per bar beyond a handful of symbols; `run`-per-asset
//! remains the shipped assets path and the lane plan is kept measured.

use cranelift::codegen::ir::{BlockArg, Function, MemFlagsData, UserFuncName};
use cranelift::codegen::isa::TargetFrontendConfig;
use cranelift::codegen::Context;
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};

use crate::cse::query::{IndicatorStateBox, Request, Results, RowRef};
use crate::cse::{merge, ChainState, Merged, Node, Sink, SinkSrc, StateRestorer};
use crate::indicators::ema::multiplier;

/// CLIF emitter (`emit` + its private block helpers) — see `jit/codegen.rs`.
mod codegen;

/// Compiled entry point: advance `len` bars of every stream in `prices`
/// (a table of `nassets` f64 slice pointers — one entry for the classic
/// plan), storing to `outs` (one `*mut f64` per sink slot, plan order),
/// carrying `state` (raw u64 words, layout owned by the plan) across
/// chunk calls.
pub type JitFn = unsafe extern "C" fn(
    state: *mut u64,
    prices: *const *const f64,
    // E3 per-stream price ANCHORS: a virtual array covering bars
    // `next_bar - pmax ..` — the caller's slice shifted back on cold
    // chunks, a host-stitched [window | chunk] buffer on resumed ones.
    // Rolling prev reads become one `iadd` + `load`, no per-bar select.
    anchors: *const *const f64,
    len: usize,
    outs: *mut *mut f64,
);

/// Source reference: the price stream.
const PRICE: usize = usize::MAX;
/// Sink lead meaning "never print" (disabled optional).
const LEAD_NEVER: u64 = u64::MAX;

/// Per-advance bookkeeping tables live on the STACK for sane plans (dense
/// all-on ≈ 22, 8 assets × 6 = 48 ≤ 64); beyond that a heap fallback. Bounds
/// `tick`'s scratch too. The only per-advance heap is the arena (`tick` skips
/// it).
const STACK_STORES: usize = 64;

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
    /// State-word indices (EMA ops only; rolling ops: the `sum`).
    ema_word: usize,
    sd_word: usize,
    /// Stddev/Wma ops: the `sum_sq` / `weighted_sum` word (E3).
    aux_word: usize,
    /// E3: first op of this op's `(src, period)` rolling group (itself for
    /// the leader — one sum per group; the other kind becomes a follower
    /// that reads the leader's lines).
    roll_leader: usize,
    /// E9: for a `Roll` op, the narrower live roll on the SAME stream and
    /// direction whose per-bar value seeds a gap-only rescan walk (`usize::MAX`
    /// = walk the whole window, as before). See
    /// `gap_seeded_rescan_matches_full_walk` for the proof this is exact.
    seed: usize,
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
    /// `|a|` — CLIF `fabs` over a scalarized operand.
    Abs { src: usize },
    /// `if a >= b { t } else { 0.0 }` — CLIF `fcmp.ge` feeding a `select`
    /// (the DM+ / DM− mutual exclusion; the `0.0` else arm is a literal, so
    /// no `-0.0`/NaN is ever produced). Operands scalarized like `Arith`.
    WhereGe { a: usize, b: usize, t: usize },
    /// A raw price stream delayed `n` bars (E6). No accumulator — it loads
    /// through the anchor like the rolling prev-reads; the guard `ivar < n`
    /// (only reachable at global bars `< n`, never published) yields `0.0`.
    Lag {
        /// physical stream slot (`PRICE - …`), asserted price-sourced.
        src: usize,
        n: u32,
    },
    /// `wma(period)` over a PRICE `src` (E3 stage 2): hand trick on the
    /// GROUP's raw sum + its own weighted-sum line (`aux`); warm bars fold
    /// `ws += src·(i+1)` like the hand's init. `w = P as f64`,
    /// `wr = 2/(P(P+1))` (reciprocal multiply, hand-faithful).
    Wma {
        src: usize,
        period: u32,
        w: f64,
        wr: f64,
    },
    /// `sma(period)` over a PRICE `src` (asserted at build): ring of src
    /// values in state, rolling `sum`, result `sum * mult` (E3).
    Sma { src: usize, period: u32, mult: f64 },
    /// `stddev(period)` over a PRICE `src` — hand `Stddev::calc` op-for-op
    /// (sum roll + `sum_sq`, warm segments fold `fma(v,v,sq)` exactly like
    /// the hand's init, steady segments the double-rounded update).
    Stddev { src: usize, period: u32, mult: f64 },
    /// Rolling max/min over a PRICE `src` (E9) — the trail-rescan recurrence
    /// (`Node::RollMax`/`RollMin`). TWO carried lines: the extremum rides the
    /// op's f64 value slot like any chain; the `trail` (bars since the
    /// NEWEST occurrence) is an i64 in a dedicated loop-param run
    /// (`ROLL_TRAIL_*`). Per bar: `trail += 1`; if `period <= trail` the
    /// anchor window `[i − look_back ..= i]` is rescanned (forward with
    /// `>=`/`<=` so ties yield the newest index) and `trail = look_back − idx`;
    /// otherwise the carried value survives, replaced by this bar's `src` with
    /// `trail = 0` when the compare hits. The rescan goes out on a `brif`
    /// side-exit so an ordinary bar costs one compare + two selects, and in
    /// segments starting below bar `period` no rescan is reachable at all, so
    /// the guard is not emitted there. `want_max` picks `>=` vs `<=` — the
    /// only difference between the two shapes.
    ///
    /// Do NOT merge a max/min pair over ONE series into a shared window walk:
    /// their rescans can never coincide. Both firing at bar `i` means the carried
    /// max AND min are both exactly `period` bars old, i.e. bar `i − period` is
    /// both extremum of the previous window ⇒ that window is constant ⇒ every bar
    /// in it satisfied `x >= max` AND `x <= min`, resetting both trails to 0 ⇒
    /// neither ever reaches `period`. Measured 0 overlaps in 6692 bars at p=14 (0
    /// at p=20 and p=52 too — `tulip_test/examples/minmax_probe.rs`), so a
    /// union-fired walk performs exactly the same NUMBER of walks with the same
    /// load traffic, but computes both extremums on every one of them: a reduction
    /// nobody asked for on each fired bar. More ALU, zero saving.
    Roll {
        src: usize,
        period: u32,
        look_back: u32,
        want_max: bool,
    },
    /// E9: a roll's carried `trail` (bars since the newest occurrence) read as
    /// a value — `fcvt_from_uint`, the hand's `usize as f64`. Stateless, so it
    /// owns no slot/word: it is ONE extra instruction at the consumer, and
    /// because it is an ordinary arena node, `aroon` and `aroonosc` share the
    /// roll, the projection AND the scaled lines automatically.
    Trail {
        /// the referenced roll's op id (SSA order guarantees it is computed
        /// first in the same block chain).
        roll: usize,
    },
    /// `hma(period)` over a PRICE `src` — E3 stage 3, hand `hma.rs` as ONE
    /// bespoke rolling op. Private state (NOT the `(src,period)` group):
    /// two WMA engines (`sum1/ws1` at P — warm ≡ the hand's
    /// `init_state(real, P)`; `sum2/ws2` at P/2 with the hand's OFFSET
    /// init window `[P−P/2 .. P−1]`, hence the shifted fold weight
    /// `i − (P − P/2) + 1`), `wss`/`ss` accumulators over the diff series
    /// `2·fast − slow`, and a `psqrt = trunc(√P)` ring of DIFF values in
    /// the state image (the legitimate stored-calculation buffer).
    /// Segments: fold1 `[0, P−P/2)`, fold1+2 `[P−P/2, P)`, FILL `[P,
    /// P+psqrt)` (trick + accumulate + push, no output), STEADY from
    /// `P+psqrt` (fill's one-time `ss −= ring[0]` correction rides a
    /// `bar == seam` select in the entry segment). Output = TRUE
    /// DIVISION `wss / wssq`, hand order.
    Hma {
        src: usize,
        period: u32,
        p2: u32,
        psqrt: u32,
        wc1: f64,
        wr1: f64,
        wc2: f64,
        wr2: f64,
        psqrf: f64,
        wssq: f64,
    },
    /// `wilders(period)` — an `Ema`-style single chain: fold `sum += v` over
    /// the first `period` source bars, `÷period` once at the seam (the mean
    /// seed, matching the hand's `init_state` `sum/period`), then the steady
    /// IIR `w = w·mult + v·inv` with `mult = (p−1)/p`, `inv = 1−mult` (hand
    /// `multiplier`). No history/anchor read (only the current bar's value).
    /// `chained = false` (price src, E6 leaf): the chunk's bar 0 is the
    /// setup seed (`p0`) and the loop folds from bar 1. `chained = true` (a
    /// derived src — `atr`'s `Wilders` over the `tr` node): there is no
    /// per-stream `p0` to seed from, so the cold chain value is `0.0` and the
    /// loop folds the source NODE-VALUE from bar 0 (the `Lag` clamp makes
    /// `tr[0] = high[0] − low[0]`, the hand's bar-0 term) — bit-identical sum.
    Wilders {
        src: usize,
        mult: f64,
        inv: f64,
        period: u32,
        chained: bool,
        composite: bool,
        feed: u64,
    },
    /// `a*b+c` in one rounding (`fma.f64` — the hand's `mul_add`).
    Fma { a: usize, b: usize, c: usize },
    /// `sqrt(a)` — IEEE exact root.
    Sqrt { src: usize },
    /// A `Node::Const` leaf — materialized, rematerialized by RA at use.
    Const(f64),
    /// `supertrend(period, step)` — the hand's multi-value feedback band
    /// (E7). `atr`/`med` are resolved operand op-ids (both may be derived
    /// nodes — the shared mean-`Wilders(tr)` and `0.5·(high+low)` medprice);
    /// `close` is a price-stream slot (`PRICE - …`). Carries FOUR loop params
    /// in a dedicated `spos` block mirroring hand `State`: three f64
    /// (`prev_st`/`prev_ub`/`prev_lb`) + one i64-backed `bool` (`trend`, same
    /// 0/1 convention as `seeded`). The per-bar value `st` is `cur[j]` (a sink
    /// source), NOT a loop param — like HMA, whose value is likewise fresh
    /// each bar. `feed` = its source lead (the atr seam): it seeds one
    /// `calc_st` there then steady-decays; `seam = op.lead` (derived from
    /// `period` by the merge pass, so `period` itself is not re-carried).
    SuperTrend {
        atr: usize,
        med: usize,
        close: usize,
        step: f64,
    },
}

// ── §2.3 state-shape descriptors: the ONE source of truth for what each
// stateful op kind carries across bars ────────────────────────────────────
// `codegen::emit` (the loop-carried param runs `hpos/cpos/spos/tpos` + the
// `sq` aux run) and `compile_request` (the aux-word reservation → `nwords`)
// both index these runs, so the per-kind sizes live HERE, read by both — a
// new stateful kind declares its shape once instead of repeating magic
// numbers in two files. Distinct concerns, deliberately NOT collapsed:
//   · `*_f64_params / *_i64_params` = loop-carried PARAM slots (xmm/int file
//     budget) — `emit` lays them out in this fixed order:
//     [ sq aux · hma f64 · hma i64 · sup f64 · sup i64 ]
//   · `*_aux_words` = STATE-IMAGE words reserved after the per-op value run
//     (`compile_request` walks ops in order; HMA's ring size is `psqrt`, so
//     its word total is not a constant — see `HMA_AUX_BASE`).
// The `schedule()`/`Phase` warm-up generalization is intentionally NOT here
// (deferred to the next new stateful primitive — see review_feedback §2.3):
// `Hma`/`SuperTrend` are permanent bespoke ops whose seams don't fit one DSL.

/// `Stddev` / `Wma`: one `sum_sq` / `weighted_sum` aux loop-param + state word
/// each (their rolling `sum` is the group leader's, not per-op).
const ROLLING_AUX_PARAMS: usize = 1;
const ROLLING_AUX_WORDS: usize = 1;
/// E9 `RollMax`/`RollMin`: one i64 `trail` loop param + state word each. The
/// carried extremum needs NOTHING here — it is the op's ordinary f64 value
/// slot, exactly like an `Ema`'s or `Wilders`' chain value.
const ROLL_TRAIL_PARAMS: usize = 1;
const ROLL_TRAIL_WORDS: usize = 1;
/// `Hma`: 6 carried f64 accumulators `[sum1, ws1, sum2, ws2, wss, ss]` + 1
/// i64 ring cursor as loop params; state words = `[6 acc + 1 cursor]` plus the
/// `psqrt` diff-ring words (total `7 + psqrt`).
const HMA_F64_PARAMS: usize = 6;
const HMA_I64_PARAMS: usize = 1;
const HMA_AUX_BASE: usize = 7; // + per-op `psqrt` ring words
/// `SuperTrend`: 3 carried f64 band states (`prev_st/prev_ub/prev_lb`) + 1
/// i64-backed bool (`trend`); 4 state words (the bool is one of the 4).
const SUP_F64_PARAMS: usize = 3;
const SUP_I64_PARAMS: usize = 1;
const SUP_AUX_WORDS: usize = 4;

/// `alias`/lane sentinel: "the op's own scalar value".
const LANE_SELF: u8 = 0xFE;

#[derive(Clone, Copy)]
enum ArithKind {
    Sub,
    Add,
    Mul,
    Div,
    /// Rust `f64::max` (IEEE maxNum), lowered by `codegen::max_num` — NOT CLIF
    /// `fmax`, whose WebAssembly NaN rules are the opposite of Rust (see §2.1).
    Max,
}

/// One compiled sink: value op + publish lead (clamped `max(declared,
/// value lead)`; `LEAD_NEVER` when disabled).
struct SinkSpec {
    val: usize,
    lead: u64,
}

/// E2c: one request/output/asset slot. `store` indexes the unique
/// write-stream for its node (`usize::MAX` ⇒ disabled optional: prints
/// EMPTY); `lead` is the slot's publish lead. Hand parity makes leads
/// differ per request for the same node (apo long @25 vs ppo long @26),
/// so the ROW is stored at the node's MIN lead and each slot reads a
/// window — `offset = max(start, slot.lead) - max(start, store.lead)`,
/// resolved per chunk in `advance_assets`. Nothing is ever copied.
#[derive(Clone, Copy)]
struct View {
    store: usize,
    lead: u64,
}

/// Per-request metadata for `get_states` reconstitution — the request's
/// state chains (op + period, `TState` field order) and the INDICATOR's own
/// restorer fn. The engine holds no indicator identity at all: `restorer`
/// is an opaque pointer and the sink layout lives in `counts`.
struct ReqMeta {
    /// Per asset: state chains (op, period) in `TState` field order.
    state: Vec<Vec<(usize, u32)>>,
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
    /// E2c: unique write-streams — one compiled store per distinct
    /// output node, at the node's minimum publish lead.
    stores: Vec<SinkSpec>,
    /// One view per request slot (order: request → output → asset),
    /// aliasing `stores`. Slot count == the pre-E2c `sinks` count.
    views: Vec<View>,
    counts: Vec<(usize, usize)>,
    reqs: Vec<ReqMeta>,
    nwords: usize,
    /// E2: number of asset streams the arena was replicated over.
    nassets: usize,
    /// E6: number of price FIELDS each asset exposes (1 = close-only; 2 =
    /// high/low; 3 = high/low/close; …). The `(field, asset)` pair is the
    /// physical stream slot `asset · n_fields + field`, so `n_fields == 1`
    /// reduces to the pre-E6 `PRICE - asset` encoding EXACTLY.
    n_fields: usize,
    /// E3: max rolling period (0 = none) — how far back `anchors` reach.
    pmax: usize,
    /// E4: `hash64` of the canonical request bytes this plan was compiled
    /// from; stamped onto every [`JitState`] and asserted on `advance` —
    /// the DB-reload safety net (state words are only meaningful under the
    /// arena layout that created them).
    fingerprint: u64,
}

/// `JitPlan` is shareable (`Arc<JitPlan>` across threads — the E4 cache).
/// Soundness: the `JITModule` field is touched ONLY during
/// `compile_request`/`emit`, before any plan is ever handed out; the run
/// path reads immutable plan data and calls the finalized, reentrant
/// function with caller-owned [`JitState`] (all machine-code state lives
/// in that image — nothing global inside the code itself).
unsafe impl Sync for JitPlan {}

const _: fn() = || {
    fn assert<T: Send + Sync>() {}
    assert::<JitPlan>();
};

/// Per-asset resumable state: the plan's raw state words (header
/// `next_bar`/`seeded`, chained seed flags, chain values). Layout owned by
/// the emitter; opaque otherwise.
/// `Serialize`/`Deserialize` so a symbol's resumable state persists across a
/// process/DB/FFI boundary exactly like the hand `IndicatorState`: save it,
/// reload it, `resume`/`tick` with only the next bar — the caller never
/// assembles the price tail (that lives in `window`). `plan_key` is serialized
/// so the foreign-state guard still fires after a reload; `scratch` is a
/// transient stitch buffer rebuilt on demand, so it is skipped.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct JitState {
    pub words: Vec<u64>,
    /// E3: per-stream tail of real prices (the `pmax` bars before
    /// `next_bar`) that anchors stitch in front of resumed chunks.
    /// Persist alongside `words`; empty vectors for non-rolling plans.
    pub window: Vec<Vec<f64>>,
    /// E4: the fingerprint of the plan that created this image;
    /// `advance` refuses to run under a different one.
    plan_key: u64,
    /// Stitch buffers (transient — rebuilt by `stitch`, never serialized).
    #[serde(skip, default)]
    scratch: Vec<Vec<f64>>,
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
fn compile_request(req: &Request, n_assets: usize) -> JitPlan {
    let fingerprint = crate::cse::cache::fingerprint_of(req, n_assets);
    let (arena, mut resolved) = merge(&req.recipes);
    // same switch `Request::build` applies — disabled optionals compile to
    // never-print sinks (EMPTY in position, values unchanged elsewhere).
    for (r, flags) in resolved.iter_mut().zip(&req.opt_flags) {
        r.optional_enabled = flags.clone();
    }
    let leads = arena.leads();

    // ---- dead-node elimination (review §2.2 / D3) ----
    // Roots = every ENABLED sink's src node ∪ every `tail_inputs` state chain.
    // The latter is load-bearing: `get_states` may read a chain no sink wants
    // *now* (and an indicator whose `get_states` is merely deferred still names
    // its chains here), so pruning a "dead-looking" recurrence would silently
    // break resume-after-`get_states`. A node is live iff reachable from a root
    // through child edges; SSA order ⇒ children < parent ⇒ ONE descending scan
    // propagates liveness. Dead nodes get no `Op`: no state word, no loop-carried
    // param, no per-bar math — the register-pressure relief DCE exists to give.
    // The arena itself and the stream encoding are NOT touched (a dead `Input`
    // slot is simply never read; `n_fields` stays the caller-visible count so
    // `series.len() == n_fields · n_assets` holds for an unchanged `Request`).
    let mut live = vec![false; arena.nodes.len()];
    for res in &resolved {
        for s in &res.outputs {
            let SinkSrc::Interface(id) = s.src;
            live[id as usize] = true;
        }
        for (oi, s) in res.optional_outputs.iter().enumerate() {
            if res.optional_enabled.get(oi).copied().unwrap_or(true) {
                let SinkSrc::Interface(id) = s.src;
                live[id as usize] = true;
            }
        }
        for &i in &res.inputs {
            live[i as usize] = true;
        }
    }
    for id in (0..arena.nodes.len()).rev() {
        if live[id] {
            merge::node_children(&arena.nodes[id], &mut |c| live[c as usize] = true);
        }
    }

    // ---- pass 1: arena nodes → ops, SSA order (producers are lower ids) ----
    let mut ops: Vec<Op> = Vec::new();
    // E6: how many price FIELDS the recipes reference (max `Input{stream}`+1).
    // 1 = close-only (classic/ema/rolling/HMA). The physical stream slot for
    // (field f, asset s) is `s · n_fields + f`, encoded as `PRICE - slot`.
    let n_fields = arena
        .nodes
        .iter()
        .map(|n| match *n {
            Node::Input { stream } => stream as usize + 1,
            _ => 0,
        })
        .max()
        .unwrap_or(1)
        .max(1);
    let n_streams = n_fields * n_assets;
    // op ids per asset; an asset's `Input{f}` maps to its `(f, asset)` price
    // SLOT (`PRICE - (asset·n_fields + f)`), so `src` doubles as the stream
    // selector and `n_fields == 1` keeps the classic `src == PRICE` identity.
    let mut node_op: Vec<Vec<usize>> = vec![vec![PRICE; arena.nodes.len()]; n_assets];
    let is_price = |id: usize| id > PRICE - n_streams;
    macro_rules! push {
        ($s:expr, $id:expr, $e:expr) => {{
            let id = $id as usize;
            ops.push(Op {
                e: $e,
                lead: leads[id] as u64,
                ema_word: 0,
                sd_word: 0,
                aux_word: 0,
                roll_leader: usize::MAX,
                seed: usize::MAX,
            });
            node_op[$s][id] = ops.len() - 1;
        }};
    }
    for s in 0..n_assets {
        for (id, node) in arena.nodes.iter().enumerate() {
            // DCE: skip a dead node's op, but keep walking `Input` (it only
            // sets `node_op`, never pushes) so live consumers still resolve
            // their price-slot operands.
            if !live[id] && !matches!(node, Node::Input { .. }) {
                continue;
            }
            match *node {
                Node::Input { stream } => {
                    // (field, asset) → physical stream slot, field-fastest
                    // so asset-major `series` slicing stays contiguous.
                    node_op[s][id] = PRICE - (s * n_fields + stream as usize);
                }
                Node::Const { bits } => push!(s, id, OpE::Const(f64::from_bits(bits))),
                Node::Ema { src, period } => {
                    let (mult, inv) = multiplier(period as usize);
                    let src_op = node_op[s][src as usize];
                    push!(
                        s,
                        id,
                        OpE::Ema {
                            src: src_op,
                            mult,
                            inv,
                            feed: leads[src as usize] as u64,
                            chained: !is_price(src_op),
                        }
                    );
                }
                Node::Sub { a, b } => push!(
                    s,
                    id,
                    OpE::Arith {
                        op: ArithKind::Sub,
                        a: node_op[s][a as usize],
                        b: node_op[s][b as usize],
                        vec: false,
                    }
                ),
                Node::Add { a, b } => push!(
                    s,
                    id,
                    OpE::Arith {
                        op: ArithKind::Add,
                        a: node_op[s][a as usize],
                        b: node_op[s][b as usize],
                        vec: false,
                    }
                ),
                Node::Mul { a, b } => push!(
                    s,
                    id,
                    OpE::Arith {
                        op: ArithKind::Mul,
                        a: node_op[s][a as usize],
                        b: node_op[s][b as usize],
                        vec: false,
                    }
                ),
                Node::Div { a, b } => push!(
                    s,
                    id,
                    OpE::Arith {
                        op: ArithKind::Div,
                        a: node_op[s][a as usize],
                        b: node_op[s][b as usize],
                        vec: false,
                    }
                ),
                Node::Max { a, b } => push!(
                    s,
                    id,
                    OpE::Arith {
                        op: ArithKind::Max,
                        a: node_op[s][a as usize],
                        b: node_op[s][b as usize],
                        vec: false,
                    }
                ),
                Node::WhereGe { a, b, t } => push!(
                    s,
                    id,
                    OpE::WhereGe {
                        a: node_op[s][a as usize],
                        b: node_op[s][b as usize],
                        t: node_op[s][t as usize],
                    }
                ),
                Node::Abs { src } => push!(
                    s,
                    id,
                    OpE::Abs {
                        src: node_op[s][src as usize]
                    }
                ),
                Node::Lag { src, n } => {
                    let src_op = node_op[s][src as usize];
                    assert!(
                        is_price(src_op),
                        "jit: Lag must read a raw price stream (E6)"
                    );
                    assert!(n >= 1, "jit: Lag n must be >= 1");
                    push!(s, id, OpE::Lag { src: src_op, n });
                }
                Node::Hma { src, period } => {
                    let src_op = node_op[s][src as usize];
                    assert!(
                        is_price(src_op),
                        "jit: HMA over a derived src is not emittable"
                    );
                    let p = period as usize;
                    assert!(p >= 2, "jit: HMA period must be >= 2 (period2 = P/2)");
                    let p2 = (p / 2) as u32;
                    // hand: ((period as f64).sqrt() as usize) as f64 —
                    // TRUNCATED integer sqrt root, then exact integer f64s
                    let psqrt = (p as f64).sqrt() as u32;
                    let nn1 = p as f64;
                    let nn2 = p2 as f64;
                    let psqrf = psqrt as f64;
                    push!(
                        s,
                        id,
                        OpE::Hma {
                            src: src_op,
                            period,
                            p2,
                            psqrt,
                            wc1: nn1,
                            wr1: 2.0 / (nn1 * (nn1 + 1.0)),
                            wc2: nn2,
                            wr2: 2.0 / (nn2 * (nn2 + 1.0)),
                            psqrf,
                            wssq: psqrf * (psqrf + 1.0) / 2.0,
                        }
                    );
                }
                Node::Wilders {
                    src,
                    period,
                    composite,
                    feed,
                } => {
                    let src_op = node_op[s][src as usize];
                    // hand wilders::multiplier(period) = ((p−1)/p, 1−that)
                    let (mult, inv) = crate::indicators::wilders::multiplier(period as usize);
                    // A `feed` wilders (adx over dx) starts folding only at its
                    // source's publish lead; otherwise it folds from bar 0.
                    let feed_bar = if feed { leads[src as usize] as u64 } else { 0 };
                    push!(
                        s,
                        id,
                        OpE::Wilders {
                            src: src_op,
                            mult,
                            inv,
                            period,
                            chained: !is_price(src_op),
                            composite,
                            feed: feed_bar,
                        }
                    );
                }
                Node::SuperTrend {
                    atr,
                    med,
                    close,
                    step_bits,
                    ..
                } => push!(
                    s,
                    id,
                    OpE::SuperTrend {
                        atr: node_op[s][atr as usize],
                        med: node_op[s][med as usize],
                        close: node_op[s][close as usize],
                        step: f64::from_bits(step_bits),
                    }
                ),
                Node::Wma { src, period } => {
                    let src_op = node_op[s][src as usize];
                    assert!(
                        is_price(src_op),
                        "jit: rolling over a derived src (chained Wma) is E3 stage 3"
                    );
                    let nn = period as f64;
                    push!(
                        s,
                        id,
                        OpE::Wma {
                            src: src_op,
                            period,
                            w: nn,
                            wr: 2.0 / (nn * (nn + 1.0)),
                        }
                    );
                }
                Node::Sma { src, period } => {
                    let src_op = node_op[s][src as usize];
                    assert!(
                        is_price(src_op),
                        "jit: rolling over a derived src (chained Sma/Wma) is E3 stage 2"
                    );
                    push!(
                        s,
                        id,
                        OpE::Sma {
                            src: src_op,
                            period,
                            mult: 1.0 / period as f64,
                        }
                    );
                }
                Node::Stddev { src, period } => {
                    let src_op = node_op[s][src as usize];
                    assert!(
                        is_price(src_op),
                        "jit: rolling over a derived src is E3 stage 2"
                    );
                    push!(
                        s,
                        id,
                        OpE::Stddev {
                            src: src_op,
                            period,
                            mult: 1.0 / period as f64,
                        }
                    );
                }
                // E9: the trail-rescan roll. Price-sourced only (the rescan
                // walks the ANCHOR); a derived src is E8's shift-eval.
                // `look_back` stays an independent parameter — `(P, P−1)` and
                // `(P, P)` are different windows and must not intern together.
                n @ (Node::RollMax { .. } | Node::RollMin { .. }) => {
                    let (src, period, look_back, want_max) = match n {
                        Node::RollMax {
                            src,
                            period,
                            look_back,
                        } => (src, period, look_back, true),
                        Node::RollMin {
                            src,
                            period,
                            look_back,
                        } => (src, period, look_back, false),
                        _ => unreachable!(),
                    };
                    let src_op = node_op[s][src as usize];
                    assert!(
                        is_price(src_op),
                        "jit: rolling min/max over a derived src needs E8 shift-eval (Handover §6.5)"
                    );
                    assert!(period >= 1, "jit: RollMin/RollMax period must be >= 1");
                    assert!(
                        look_back <= period,
                        "jit: roll look_back {look_back} above period {period} would rescan a \
                         window wider than the trail guard can ever expire"
                    );
                    push!(
                        s,
                        id,
                        OpE::Roll {
                            src: src_op,
                            period,
                            look_back,
                            want_max,
                        }
                    );
                }
                // E9: the roll's trail as a graph value. Referenced op must be
                // a roll — the emitter reads that roll's per-bar trail register,
                // so an arbitrary `src` would have nothing to project.
                Node::Trail { src } => {
                    let roll = node_op[s][src as usize];
                    assert!(
                        matches!(ops[roll].e, OpE::Roll { .. }),
                        "jit: Trail must reference a RollMax/RollMin node"
                    );
                    push!(s, id, OpE::Trail { roll });
                }
                Node::Fma { a, b, c } => push!(
                    s,
                    id,
                    OpE::Fma {
                        a: node_op[s][a as usize],
                        b: node_op[s][b as usize],
                        c: node_op[s][c as usize],
                    }
                ),
                Node::Sqrt { src } => push!(
                    s,
                    id,
                    OpE::Sqrt {
                        src: node_op[s][src as usize]
                    }
                ),
                other => panic!("jit: node {other:?} is not emittable yet"),
            }
        }
    }
    debug_assert_eq!(
        ops.len(),
        arena
            .nodes
            .iter()
            .enumerate()
            .filter(|(id, n)| live[*id] && !matches!(n, Node::Input { .. }))
            .count()
            * n_assets,
        "ops.len() must equal the LIVE non-input node count × n_assets (DCE)"
    );

    // ---- E9: seed links (nested windows share one walk) ----
    // A roll whose window strictly contains another live roll's window on the
    // SAME stream and direction needs only the GAP beyond the narrower one:
    // the narrower roll's value at this bar already covers `[i − lb_n ..= i]`,
    // so the wider rescan walks `[i − lb_w .. i − lb_n − 1]` and seeds its
    // running extremum with that value. Exact including `trail`, because when
    // the gap fails to STRICTLY beat the seed, the newest occurrence of the
    // value is the seed's own bar ⇒ the wider roll inherits `trail_n` — proved
    // by `gap_seeded_rescan_matches_full_walk`, which also validates the model
    // against the hand oracle.
    //
    // WHY THE SEED IS ALWAYS SAFE TO READ (checked here once, so nobody has to
    // re-derive it per plan). Two things a link needs: the seed's bars must be
    // INSIDE the wide window — `lb_n < lb_w` gives that and the gap — and the
    // seed's value must be a FULL-window maximum, not a warm-up partial. The
    // second needs no extra condition: the wide roll's rescan branch is only
    // emitted for bars `b >= period_w = lb_w + 1` (below that the carried trail
    // cannot reach `period_w`, so the segment has no guard at all), and its
    // guard fires at `trail >= period_w`; the trail is at most `b`, so the
    // EARLIEST bar a wide rescan can happen is `lb_w + 1 > lb_n` — by which bar
    // the narrow roll's own window `[b − lb_n ..= b]` has long been complete.
    // The same inequality is what keeps the walk's reads inside the chunk:
    // `b − lb_w >= 1` at the first eligible bar, and a resumed chunk's stitched
    // window supplies the rest (`pmax >= lb_w`). An earlier draft of this
    // comment claimed a link could read an "empty region" unless
    // `lb_w >= 2·lb_n + 1`; that was a bad derivation — the counter-example it
    // imagined is a bar where the wide roll cannot rescan at all — and the
    // constraint cost the `aroon(14) ← willr(14)` link, worth 30% on that row.
    //
    // The addresses coincide because BOTH rolls read the same anchor for the
    // same bar (`this_bar = abases[src] + 8·pmax + poff`), not because a caller
    // happened to line them up — unlike the by-option hand, whose road scheduler
    // reslices each period's input independently so its lanes' pointers differ
    // and the nesting must be skipped at runtime ("never assert it", see
    // `indicators::simd_indicators::max_simd::…has_shorter_neighbor`).
    // Restricted to a LOWER op id in the past; no longer, because the emitter
    // now runs a two-phase order (see `codegen::emit`): ALL rolls first, sorted
    // by (stream, direction, ascending `look_back`), so a seed is always emitted
    // before whatever links to it regardless of how the caller ordered the
    // requests. Linking cannot cycle — each link strictly decreases `look_back`.
    for j in 0..ops.len() {
        let (src, lb, want_max) = match ops[j].e {
            OpE::Roll {
                src,
                look_back,
                want_max,
                ..
            } => (src, look_back, want_max),
            _ => continue,
        };
        let mut best: Option<(usize, u32)> = None;
        for k in 0..ops.len() {
            if k == j {
                continue;
            }
            if let OpE::Roll {
                src: s2,
                look_back: lb2,
                want_max: w2,
                ..
            } = ops[k].e
            {
                if s2 == src && w2 == want_max && lb2 < lb {
                    if best.is_none_or(|(_, bl)| lb2 > bl) {
                        best = Some((k, lb2));
                    }
                }
            }
        }
        if let Some((k, _)) = best {
            ops[j].seed = k;
        }
    }

    // ---- pass 2: stores/views + state wiring, all by arena id ----
    let mut stores: Vec<SinkSpec> = Vec::new();
    let mut views: Vec<View> = Vec::new();
    let mut counts = Vec::new();
    let mut reqs: Vec<ReqMeta> = Vec::new();

    for res in &resolved {
        // sink slot order: request → output (declared) → asset — `advance`
        // regroups per asset with that stride.
        let mut add = |list: &[Sink], enabled: Option<&[bool]>| {
            for (oi, s) in list.iter().enumerate() {
                for st_s in 0..n_assets {
                    if enabled.is_some_and(|f| !f.get(oi).copied().unwrap_or(true)) {
                        views.push(View {
                            store: usize::MAX,
                            lead: LEAD_NEVER,
                        });
                        continue;
                    }
                    let SinkSrc::Interface(id) = s.src;
                    let o = node_op[st_s][id as usize];
                    assert!(
                        !is_price(o),
                        "jit: mirroring a bare Input needs sink support for price streams"
                    );
                    // hand invariant (slot_lengths/output_length): publish
                    // lead = max(declared, value lead) — asset-independent
                    let lead = (s.lead as u64).max(ops[o].lead);
                    // E2c: intern the write-stream by node. A later slot
                    // with a SMALLER lead lowers the store's lead (the row
                    // grows at its head) and every view keeps its own lead,
                    // so windows re-offset correctly at advance time.
                    let k = match stores.iter().position(|x| x.val == o) {
                        Some(k) => {
                            if lead < stores[k].lead {
                                stores[k].lead = lead;
                            }
                            k
                        }
                        None => {
                            stores.push(SinkSpec { val: o, lead });
                            stores.len() - 1
                        }
                    };
                    views.push(View { store: k, lead });
                }
            }
        };
        add(&res.outputs, None);
        add(&res.optional_outputs, Some(&res.optional_enabled));

        counts.push((res.outputs.len(), res.optional_outputs.len()));
        let states: Vec<Vec<(usize, u32)>> = (0..n_assets)
            .map(|st_s| {
                res.inputs
                    .iter()
                    .map(|&i| match arena.nodes[i as usize] {
                        Node::Ema { period, .. }
                        | Node::Sma { period, .. }
                        | Node::Stddev { period, .. }
                        | Node::Wma { period, .. }
                        | Node::Hma { period, .. }
                        | Node::RollMax { period, .. }
                        | Node::RollMin { period, .. }
                        | Node::Wilders { period, .. } => (node_op[st_s][i as usize], period),
                        _ => panic!("jit: `tail_inputs` id {i} must name a state chain"),
                    })
                    .collect()
            })
            .collect();
        reqs.push(ReqMeta {
            state: states,
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
    let is_chain = |o: &Op| matches!(o.e, OpE::Ema { src, chained: false, .. } if is_price(src));
    let sub_operands: Vec<Option<(usize, usize)>> = (0..base_ops)
        .map(|j| match ops[j].e {
            OpE::Arith {
                op: ArithKind::Sub,
                a,
                b,
                vec: false,
            } if a < base_ops
                && b < base_ops
                && is_chain(&ops[a])
                && is_chain(&ops[b])
                && a != b =>
            {
                Some((a, b))
            }
            _ => None,
        })
        .collect();

    // Pair candidates: a disjoint partner (zero clones) when one exists,
    // otherwise any partner (reuse absorbed by cloning).
    let mut cands: Vec<usize> = (0..base_ops)
        .filter(|&j| sub_operands[j].is_some())
        .collect();
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
        for (chain, g, lane) in [(a1, gs, 0usize), (a2, gs, 1), (b1, gl, 0), (b2, gl, 1)] {
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
    // E3 rolling layout: aux words only (stddev's sum_sq, wma's
    // weighted_sum). Price history lives HOST-side in the anchor stitch,
    // so no ring words occupy the state image.
    let mut word_next = ema_base + ops.len();
    // (src, period) groups: FIRST op owns the sum roll; a sibling of the
    // other kind (Sma+Stddev over one window — bbands and standalone sma
    // in one query makes this the norm) carries nothing and reads the
    // leader's lines.
    let mut group_leader: Vec<(usize, u32, usize)> = Vec::new();
    let mut pmax = 0usize;
    for (j, op) in ops.iter_mut().enumerate() {
        match op.e {
            OpE::Sma { src, period, .. }
            | OpE::Stddev { src, period, .. }
            | OpE::Wma { src, period, .. } => {
                let key = (src, period);
                let leader = match group_leader.iter().find(|(s, p, _)| (*s, *p) == key) {
                    Some(&(_, _, l)) => l,
                    None => {
                        group_leader.push((src, period, j));
                        pmax = pmax.max(period as usize);
                        j
                    }
                };
                op.roll_leader = leader;
                if matches!(op.e, OpE::Stddev { .. } | OpE::Wma { .. }) {
                    op.aux_word = word_next;
                    word_next += ROLLING_AUX_WORDS;
                } else {
                    op.aux_word = 0;
                }
            }
            // E3 stage 3: HMA is self-contained — its state run is
            // [sum1, ws1, sum2, ws2, wss, ss] (f64), cursor (i64), then
            // `psqrt` DIFF ring words (f64). It joins NO group (the
            // period-P engine duplicates at most 4 flops/bar vs a
            // standalone Wma(P) — a deliberate non-share, see Handover).
            OpE::Hma { period, psqrt, .. } => {
                op.roll_leader = usize::MAX;
                op.aux_word = word_next;
                word_next += HMA_AUX_BASE + psqrt as usize;
                pmax = pmax.max(period as usize);
            }
            // E7: supertrend carries FOUR feedback words — `prev_st`,
            // `prev_ub`, `prev_lb` (f64 bits) + `trend` (i64 0/1), mirroring
            // hand `State` — in its own `aux_word` run (threaded as the
            // `spos` loop-param block). No anchor history (it reads the atr
            // NODE value, not a price tail).
            OpE::SuperTrend { .. } => {
                op.aux_word = word_next;
                word_next += SUP_AUX_WORDS;
                op.roll_leader = usize::MAX;
            }
            // E9: a trail-rescan roll joins NO group (its "window" is read
            // from the anchor on the rare rescan bar, not rolled every bar).
            // Its one aux word is the `trail`; the carried extremum is the
            // op's ordinary f64 value slot. `look_back` widens the anchor so
            // the rescan can always reach bar `i − look_back`.
            OpE::Roll { look_back, .. } => {
                op.aux_word = word_next;
                word_next += ROLL_TRAIL_WORDS;
                op.roll_leader = usize::MAX;
                pmax = pmax.max(look_back as usize);
            }
            // E6: a delayed read widens the anchor window to reach `n` bars
            // back; it has no accumulator of its own.
            OpE::Lag { n, .. } => {
                op.aux_word = 0;
                op.roll_leader = usize::MAX;
                pmax = pmax.max(n as usize);
            }
            _ => {
                op.aux_word = 0;
                op.roll_leader = usize::MAX;
            }
        }
    }
    let nwords = word_next;

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
        stores,
        views,
        counts,
        reqs,
        nwords,
        nassets: n_assets,
        n_fields,
        pmax,
        fingerprint,
    };
    codegen::emit(&mut plan, fc);
    plan
}

impl JitPlan {
    /// Compile entry used by `Request::build`.
    pub fn compile(req: &Request) -> Self {
        compile_request(req, 1)
    }

    /// E2: compile one plan whose lanes are ASSETS — the arena replicates
    /// over `n_assets` price streams (same leads/segments for all), and the
    /// E1/E2-lite groupers pair cross-asset chains and subs automatically.
    /// `run_assets`/`advance_assets` feed the streams; results are per-asset.
    pub fn compile_assets(req: &Request, n_assets: usize) -> Self {
        assert!(n_assets >= 1, "jit: an asset-lane plan needs ≥ 1 stream");
        compile_request(req, n_assets)
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
        (self.ops.len(), self.stores.len())
    }

    /// E1 lane-group widths, e.g. `[2]` (classic) or `[2, 2, 2, 2, 2, 2]`
    /// (dense, role-aligned pairs).
    pub fn lanes(&self) -> Vec<usize> {
        self.groups.iter().map(|g| g.ops.len()).collect()
    }

    /// E6: physical price-stream count = `n_fields · n_assets`. The ABI
    /// `prices`/`anchors` tables and the host `window`/`stitch` have this
    /// many entries; output grouping stays per `nassets`. `== nassets`
    /// when every recipe is single-field (classic/ema/rolling/HMA).
    pub fn n_streams(&self) -> usize {
        self.n_fields * self.nassets
    }

    /// E9: the seed links this plan actually uses — `(wide roll op, narrow
    /// seed op)` for every roll whose window strictly contains a narrower live
    /// roll on the SAME stream and direction (so its rescan walks only the gap).
    /// Empty when nothing nests: every roll then walks its whole window, as
    /// before. Introspection, like [`Self::lanes`] / [`Self::vec_subs`] — the
    /// count, not the values, is what proves a query deduped/shared work.
    pub fn roll_seeds(&self) -> Vec<(usize, usize)> {
        self.ops
            .iter()
            .enumerate()
            .filter_map(|(j, o)| (o.seed != usize::MAX).then_some((j, o.seed)))
            .collect()
    }

    /// E2-lite: number of `short − long` subtractions computed as ONE
    /// vector `fsub` over lane-aligned groups (dense dashboards pair their
    /// lines; classic's lone sub stays scalar).
    pub fn vec_subs(&self) -> usize {
        self.ops
            .iter()
            .filter(|op| {
                matches!(
                    op.e,
                    OpE::Arith {
                        op: ArithKind::Sub,
                        vec: true,
                        ..
                    }
                )
            })
            .count()
    }

    pub fn new_state(&self) -> JitState {
        JitState {
            words: vec![0u64; self.nwords],
            // Empty outer for classic plans: `resume` clones the state and
            // an outer `Vec<Vec<f64>>` allocation would be a tick-time
            // malloc (+1 alloc, measured). `stitch` sizes it on demand.
            // E6: one window per PHYSICAL stream (field · asset).
            window: if self.pmax == 0 {
                Vec::new()
            } else {
                vec![Vec::new(); self.n_streams()]
            },
            plan_key: self.fingerprint,
            scratch: Vec::new(),
        }
    }

    /// E4: canonical fingerprint of the request shape this plan implements
    /// (stable across processes for the same engine + request).
    pub fn fingerprint(&self) -> u64 {
        self.fingerprint
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

    /// E2: one cold call over all asset streams (see `advance_assets`).
    pub fn run_assets(&self, series: &[&[f64]]) -> (Vec<Results>, JitState) {
        let mut st = self.new_state();
        let res = self.advance_assets(&mut st, series);
        (res, st)
    }

    /// E2: streaming twin of `run_assets`.
    pub fn resume_assets(&self, mut st: JitState, series: &[&[f64]]) -> (Vec<Results>, JitState) {
        let res = self.advance_assets(&mut st, series);
        (res, st)
    }

    /// Advance one chunk on a single-asset plan (see [`Self::advance_assets`]).
    pub fn advance(&self, st: &mut JitState, series: &[&[f64]]) -> Results {
        assert_eq!(
            self.nassets, 1,
            "advance: plan has {} lanes; use advance_assets",
            self.nassets
        );
        let series: &[&[f64]] = if series.is_empty() { &[&[]] } else { series };
        self.advance_assets(st, series).remove(0)
    }

    /// E2: advance one asset-lane chunk — ALL `nassets` streams are the
    /// same bar count (the hand `by_assets` contract). Per-sink rows
    /// `len − max(lead, start)` as usual; the compiled loop writes every
    /// (output, asset) sink slot; `Results` are regrouped PER ASSET,
    /// request-major (disabled optionals keep their EMPTY slot).
    pub fn advance_assets(&self, st: &mut JitState, series: &[&[f64]]) -> Vec<Results> {
        assert_eq!(
            st.plan_key, self.fingerprint,
            "jit: JitState was created by a different plan ({:x} vs {:x}) — its state \
             words would be reinterpreted against foreign arena ids",
            st.plan_key, self.fingerprint
        );
        let a = self.nassets;
        // E6: `series` is field-major per asset (`n_fields · n_assets`
        // streams); output grouping is still per asset (`a`).
        let ns = self.n_streams();
        assert_eq!(series.len(), ns, "jit: plan takes {ns} streams");
        let len = series[0].len();
        assert!(
            series.iter().all(|s| s.len() == len),
            "jit: all streams must have equal bar counts"
        );
        let start = st.words[W_NEXT] as usize;

        // ONE chunk arena for every unique row (E2c): sizes are exact, the
        // compiled loop writes each store's whole slice, so `assume_init`
        // at the end leaves NO uninitialized reads. Per-store bookkeeping
        // tables live on the STACK (`STACK_STORES`) for sane plans; beyond
        // that a heap fallback. The only per-advance heap is the arena
        // itself — tick-size advances must not malloc scratch (the original
        // Vec trio measured +40ns/chunk).
        let nst = self.stores.len();
        let mut rows_buf = [0usize; STACK_STORES];
        let mut offs_buf = [0usize; STACK_STORES];
        let mut ptrs_buf = [std::ptr::null_mut::<f64>(); STACK_STORES];
        let mut rows_heap: Vec<usize> = Vec::new();
        let mut offs_heap: Vec<usize> = Vec::new();
        let mut ptrs_heap: Vec<*mut f64> = Vec::new();
        let (rows, offs, ptrs) = if nst <= STACK_STORES {
            (
                rows_buf.as_mut_slice(),
                offs_buf.as_mut_slice(),
                ptrs_buf.as_mut_slice(),
            )
        } else {
            rows_heap.resize(nst, 0);
            offs_heap.resize(nst, 0);
            ptrs_heap.resize(nst, std::ptr::null_mut());
            (
                rows_heap.as_mut_slice(),
                offs_heap.as_mut_slice(),
                ptrs_heap.as_mut_slice(),
            )
        };
        let mut total = 0usize;
        for k in 0..nst {
            let r = len.saturating_sub(self.stores[k].lead.saturating_sub(start as u64) as usize);
            rows[k] = r;
            offs[k] = total;
            total += r;
        }
        let mut arc: std::sync::Arc<[std::mem::MaybeUninit<f64>]> =
            std::sync::Arc::new_uninit_slice(total);
        let base: *mut f64 = if total == 0 {
            std::ptr::null_mut()
        } else {
            // uniquely owned (fresh, refcount 1) ⇒ `get_mut` yields the slice;
            // the compiled loop writes through `base` while it is the only
            // holder, exactly as the former `Rc` did. `Arc` only so the
            // resulting `Results` is `Send` (threading across symbols, §3.3).
            std::sync::Arc::get_mut(&mut arc)
                .expect("fresh arena is uniquely owned")
                .as_mut_ptr()
                .cast::<f64>()
        };
        for k in 0..nst {
            ptrs[k] = unsafe { base.add(offs[k]) };
        }
        let ptr_table: *mut *mut f64 = ptrs.as_mut_ptr();
        self.run_chunk(st, series, start, len, ptr_table);
        // SAFETY: every element was written by the compiled loop (row
        // lengths equal per-store write counts exactly).
        let arena: std::sync::Arc<[f64]> = unsafe { arc.assume_init() };

        // E2c: wrap each UNIQUE row once, then hand every request slot a
        // window into it — mirror slots alias (no copy, refcount share),
        // a later publish lead simply skips leading elements. Chunk bases
        // (`max(start, lead)`) keep the resume slicing identical to the
        // old one-row-per-slot layout.
        let mk = |slot: usize| -> RowRef {
            let v = self.views[slot];
            if v.store == usize::MAX {
                return RowRef::window(&arena, 0, 0);
            }
            let rowbase = start.max(self.stores[v.store].lead as usize);
            let lead_off = start.max(v.lead as usize) - rowbase;
            let len_row = rows[v.store];
            RowRef::window(
                &arena,
                offs[v.store] + lead_off,
                len_row.saturating_sub(lead_off),
            )
        };
        // `with_capacity(0)` STILL mallocs (minimum bucket) — use the
        // allocation-free empty vec there; primary rows are always ≥ 1.
        let mut results: Vec<Results> = (0..a)
            .map(|_| Results {
                outputs: self
                    .counts
                    .iter()
                    .map(|&(np, _)| Vec::with_capacity(np))
                    .collect(),
                optional: self
                    .counts
                    .iter()
                    .map(|&(_, no)| {
                        if no == 0 {
                            Vec::new()
                        } else {
                            Vec::with_capacity(no)
                        }
                    })
                    .collect(),
            })
            .collect();
        let mut slot = 0usize;
        for (req, &(np, no)) in self.counts.iter().enumerate() {
            for _o in 0..np {
                for asset in 0..a {
                    results[asset].outputs[req].push(mk(slot));
                    slot += 1;
                }
            }
            for _o in 0..no {
                for asset in 0..a {
                    results[asset].optional[req].push(mk(slot));
                    slot += 1;
                }
            }
        }
        debug_assert_eq!(slot, self.views.len());
        results
    }

    /// Number of report slots in a `tick`/`Results` output for a
    /// single-asset plan: one per (request, output) and (request, optional),
    /// in `advance`'s slot order (`request → output → asset`, asset dropped
    /// when `nassets == 1`). A streaming consumer sizes its `tick` `out`
    /// buffer with this and learns the request/output layout once from
    /// `Request`.
    pub fn n_slots(&self) -> usize {
        self.views.len()
    }

    /// Advance ONE bar for one symbol, writing each report slot's current
    /// value into `out[slot]` (slot order = [`Self::n_slots`]). Zero heap
    /// allocation: the stores write into a stack scratch, not a fresh arena,
    /// and no `Results`/`RowRef`/`Vec` tree is built. The recurrence state
    /// still advances every bar in `st.words`, so this is bit-identical to the
    /// corresponding element of a cold `run` — `tick` *is* a resume (§1.4).
    ///
    /// `bar[k]` is the single new value of physical stream `k`
    /// (`k = asset · n_fields + field`; for a single-asset plan just the
    /// field order — high/low/close). Each slice must have length 1.
    ///
    /// A slot whose publish lead exceeds this bar (still warming up) — and a
    /// disabled optional (`store == MAX`) — is written `f64::NAN`, never `0.0`
    /// (§2.1 lesson: a real reading can be `0.0`; a "not yet published" is not
    /// a reading at all). Returns the values for the first `out.len()` slots.
    pub fn tick(&self, st: &mut JitState, bar: &[&[f64]], out: &mut [f64]) {
        assert_eq!(
            self.nassets, 1,
            "tick: plan has {} asset lanes; tick is per-symbol (thread the assets, §3.3)",
            self.nassets
        );
        assert_eq!(
            st.plan_key, self.fingerprint,
            "jit: JitState created by a different plan"
        );
        let ns = self.n_streams();
        assert_eq!(bar.len(), ns, "tick: plan takes {ns} streams");
        assert!(
            bar.iter().all(|s| s.len() == 1),
            "tick: every stream must carry exactly one bar"
        );
        let nst = self.stores.len();
        assert!(
            nst <= STACK_STORES,
            "tick: {nst} stores exceeds the stack scratch ({STACK_STORES}); use advance"
        );
        let start = st.words[W_NEXT] as usize;

        let mut scratch = [0.0f64; STACK_STORES];
        let mut pbuf = [std::ptr::null_mut::<f64>(); STACK_STORES];
        for k in 0..nst {
            pbuf[k] = std::ptr::addr_of_mut!(scratch[k]);
        }
        self.run_chunk(st, bar, start, 1, pbuf.as_mut_ptr());

        for (slot, o) in out.iter_mut().enumerate() {
            let v = self.views[slot];
            *o = if v.store != usize::MAX && start >= v.lead as usize {
                scratch[v.store]
            } else {
                f64::NAN
            };
        }
    }

    /// Invoke the compiled loop for `series[..len]` starting at absolute bar
    /// `start`, storing each sink through `ptr_table[k]` (the caller owns the
    /// destinations — an arena row for `advance_assets`, a 1-slot scratch for
    /// `tick`), and advance `W_NEXT`. The store addressing is a cursor that
    /// walks `+8` per *published* bar, so the caller's buffer must be sized for
    /// the bars this chunk publishes (`advance_assets` sizes it from
    /// `rows[k]`; `tick` passes `len == 1` so each store writes ≤ 1 `f64`).
    fn run_chunk(
        &self,
        st: &mut JitState,
        series: &[&[f64]],
        start: usize,
        len: usize,
        ptr_table: *mut *mut f64,
    ) {
        let ns = self.n_streams();
        if len > 0 {
            // pointer + anchor tables on the stack for sane stream counts
            // (no per-chunk heap alloc — matters at tick sizes), heap
            // fallback beyond 32. Anchors: cold chunks alias the caller's
            // slice shifted back pmax slots; resumed chunks read the
            // host-stitched [window | chunk] buffer (E3).
            // canonical index_of(bar b) = b - start + pmax; a SHORT early
            // window sits at the tail of its allocation, so the anchor
            // shifts back by the missing prefix (address math only — bars
            // below 0 are never dereferenced, warm folds use a literal 0).
            let shifted: Vec<*const f64> = if self.pmax == 0 {
                Vec::new()
            } else if start == 0 {
                // cold chunk: no stitch needed (the slice itself is the
                // virtual array), but the window must learn this chunk's
                // tail — the FIRST resumed chunk stitches from it.
                let n = self.pmax.min(len);
                for (win, ch) in st.window.iter_mut().zip(series) {
                    win.clear();
                    win.extend_from_slice(&ch[ch.len() - n..]);
                }
                Vec::new()
            } else {
                self.stitch(st, series)
            };
            let anchor = |k: usize| -> *const f64 {
                if self.pmax == 0 {
                    series[k].as_ptr()
                } else if start == 0 {
                    (series[k].as_ptr() as usize - 8 * self.pmax) as *const f64
                } else {
                    shifted[k]
                }
            };
            let f = self.func.expect("compiled plan has a function");
            if ns <= 32 {
                let mut tab: [*const f64; 32] = [std::ptr::null(); 32];
                let mut atab: [*const f64; 32] = [std::ptr::null(); 32];
                for k in 0..ns {
                    tab[k] = series[k].as_ptr();
                    atab[k] = anchor(k);
                }
                unsafe {
                    (f)(
                        st.words.as_mut_ptr(),
                        tab.as_ptr(),
                        atab.as_ptr(),
                        len,
                        ptr_table,
                    )
                };
            } else {
                let tab: Vec<*const f64> = series.iter().map(|s| s.as_ptr()).collect();
                let atab: Vec<*const f64> = (0..ns).map(anchor).collect();
                unsafe {
                    (f)(
                        st.words.as_mut_ptr(),
                        tab.as_ptr(),
                        atab.as_ptr(),
                        len,
                        ptr_table,
                    )
                };
            }
        }
        st.words[W_NEXT] = (start + len) as u64;
    }

    /// E3 host stitch: scratch[s] = window ++ chunk; returns the shifted
    /// anchor per stream so `anchor[index_of(b)]` is exact even while the
    /// window is still shorter than `pmax`. Also refreshes the window to
    /// the new trailing `pmax` prices. O(pmax + len) memcpy per RESUMED
    /// chunk; cold chunks never stitch.
    fn stitch(&self, st: &mut JitState, series: &[&[f64]]) -> Vec<*const f64> {
        let w = self.pmax;
        if st.window.len() != series.len() {
            st.window = vec![Vec::new(); series.len()];
        }
        if st.scratch.len() != series.len() {
            st.scratch = vec![Vec::new(); series.len()];
        }
        let mut out = Vec::with_capacity(series.len());
        for ((sc, win), chunk) in st.scratch.iter_mut().zip(st.window.iter_mut()).zip(series) {
            let w_old = win.len();
            sc.clear();
            sc.reserve_exact(w + chunk.len());
            sc.extend_from_slice(win);
            sc.extend_from_slice(chunk);
            out.push((sc.as_ptr() as usize - 8 * (w - w_old)) as *const f64);
            let n = w.min(sc.len());
            let tail_start = sc.len() - n;
            win.clear();
            win.extend_from_slice(&sc[tail_start..]);
        }
        out
    }

    fn ema_of(&self, st: &JitState, op: usize) -> f64 {
        f64::from_bits(st.words[self.ops[op].ema_word])
    }

    /// Resolve one `tail_inputs` chain (op + its declared period) into the
    /// engine-vocabulary [`ChainState`] the indicator's restorer consumes.
    /// Rolling sums live in the GROUP LEADER's slot (`roll_leader`), so a
    /// follower `Wma` reads the same sum its leader rolled; aux lines and
    /// the HMA accumulator/ring/cursor run live in the op's own `aux_word`.
    fn chain_of(&self, st: &JitState, op: usize, period: u32) -> ChainState {
        let o = &self.ops[op];
        // the group's rolled raw sum lives in the leader's chain slot
        let sum_slot = if o.roll_leader == usize::MAX {
            op
        } else {
            o.roll_leader
        };
        let sum = f64::from_bits(st.words[self.ops[sum_slot].ema_word]);
        match o.e {
            OpE::Ema { .. } => ChainState::Ema {
                value: self.ema_of(st, op),
                period,
            },
            OpE::Sma { .. } => ChainState::Sma { sum, period },
            OpE::Wilders { .. } => ChainState::Wilders {
                value: self.ema_of(st, op),
                period,
            },
            OpE::Stddev { .. } => ChainState::Stddev {
                sum,
                sum_sq: f64::from_bits(st.words[o.aux_word]),
                period,
            },
            OpE::Wma { .. } => ChainState::Wma {
                sum,
                weighted: f64::from_bits(st.words[o.aux_word]),
                period,
            },
            // E9: the roll's chain slot holds the extremum, its one aux word
            // the trail (an integer count, never a bit-pattern).
            OpE::Roll {
                want_max,
                look_back,
                ..
            } => {
                let trail = st.words[o.aux_word];
                if want_max {
                    ChainState::RollMax {
                        value: sum,
                        trail,
                        period,
                        look_back,
                    }
                } else {
                    ChainState::RollMin {
                        value: sum,
                        trail,
                        period,
                        look_back,
                    }
                }
            }
            OpE::Hma { p2, psqrt, .. } => {
                let base = o.aux_word;
                let accs = core::array::from_fn(|m| f64::from_bits(st.words[base + m]));
                let cursor = st.words[base + 6];
                let ring = (0..psqrt as usize)
                    .map(|t| f64::from_bits(st.words[base + 7 + t]))
                    .collect();
                ChainState::Hma {
                    accs,
                    cursor,
                    ring,
                    period,
                    p2,
                    psqrt,
                }
            }
            _ => panic!("jit: `tail_inputs` op {op} is not a state chain"),
        }
    }

    /// Reconstitute hand states by feeding each request's chain slots to
    /// the INDICATOR's `restorer` (carried through `Recipe::restorer`) —
    /// the engine never names an indicator to rebuild its state.
    pub fn get_states(&self, st: &JitState) -> Vec<IndicatorStateBox> {
        self.get_states_assets(st).remove(0)
    }

    /// E2: per-asset hand states (asset-major; an A = 1 plan has one row).
    pub fn get_states_assets(&self, st: &JitState) -> Vec<Vec<IndicatorStateBox>> {
        (0..self.nassets)
            .map(|a| {
                self.reqs
                    .iter()
                    .map(|r| match r.restorer {
                        Some(rest) => {
                            let states = r.state[a]
                                .iter()
                                .map(|&(op, period)| self.chain_of(st, op, period))
                                .collect();
                            rest(states)
                        }
                        None => IndicatorStateBox::Other,
                    })
                    .collect()
            })
            .collect()
    }
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

    /// Compare two rows to a relative tolerance. Used for the adx/dx `atr`
    /// MIRROR: the engine emits the shared MEAN-ATR chain (deduped with natr),
    /// whereas the hand's adx mirror is the `sum·inv` approximation — the two
    /// agree to ~1e-16, and the project's own oracle (`adx_test`/`natr_test`)
    /// validates that ATR against C-tulip `ti_atr` at 1e-12, NOT bit-exact.
    fn cmp_within(a: &[f64], b: &[f64], eps: f64, what: &str) {
        assert_eq!(a.len(), b.len(), "{what}: len {} vs {}", a.len(), b.len());
        for i in 0..a.len() {
            let scale = a[i].abs().max(1.0);
            assert!(
                (a[i] - b[i]).abs() <= eps * scale,
                "{what}: mismatch at {i}: {:?} vs {:?}",
                a[i],
                b[i]
            );
        }
    }

    fn classic() -> Request {
        Request::new()
            .add(Apo::recipe(&APO), None)
            .add(Ppo::recipe(&PPO), None)
            .add(Macd::recipe(&MACD), None)
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
                    let mut c = h1.outputs[req][out].to_vec();
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
            .add_all(Apo::recipe(&APO))
            .add_all(Ppo::recipe(&PPO))
            .add_all(Macd::recipe(&MACD))
            .add(Ema::recipe(&[12.0]), None)
            .add(Ema::recipe(&[26.0]), None)
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
            .add(Apo::recipe(&[5.0, 8.0]), None)
            .add(Apo::recipe(&[6.0, 9.0]), None)
            .add(Ppo::recipe(&[9.0, 25.0]), None)
            .add(Ppo::recipe(&[10.0, 26.0]), None)
            .add(Macd::recipe(&[11.0, 23.0, 17.0]), None)
            .add(Macd::recipe(&[12.0, 24.0, 9.0]), None)
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
            .add(Ppo::recipe(&PPO), Some(&[true, true]))
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
            .add_all(Apo::recipe(&APO))
            .add_all(Ppo::recipe(&PPO))
            .add_all(Macd::recipe(&MACD))
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
        // E2c: 11 request SLOTS collapse to 6 unique write-streams —
        // ema12 (×3 optionals), ema26 (×3, min lead 25 + windows), line
        // (apo ≡ macd primaries), ratio, signal, hist.
        assert_eq!(nsinks, 6, "one store per distinct output node");
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
            .add(Apo::recipe(&[5.0, 8.0]), None)
            .add(Apo::recipe(&[6.0, 9.0]), None)
            .add(Ppo::recipe(&[9.0, 25.0]), None)
            .add(Ppo::recipe(&[10.0, 26.0]), None)
            .add(Macd::recipe(&[11.0, 23.0, 17.0]), None)
            .add(Macd::recipe(&[12.0, 24.0, 9.0]), None)
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

    /// E2 asset lanes: the replicated arena PAIRS ASSETS by construction
    /// (line subs of two assets share no chains ⇒ disjoint ⇒ lane-aligned
    /// S/L groups and one vector `fsub` per asset pair), and every asset's
    /// rows are bit-exact against the single-asset plan — values, lengths,
    /// the optional switch, and `get_states` per symbol. Streaming at a
    /// phase boundary rides the same shared `next_bar`/`seeded` words.
    #[test]
    fn jit_asset_lanes_match_single_asset_runs() {
        let c0 = series(500);
        let c1: Vec<f64> = c0.iter().map(|v| v * 0.5 + 3.0).collect();
        let c2: Vec<f64> = c0.iter().map(|v| v * 2.0 - 7.0).collect();
        let streams = [&c0, &c1, &c2];

        let one = classic3_all_on().build();
        let expect: Vec<Results> = streams.iter().map(|c| one.run(&[c.as_slice()]).0).collect();

        // A = 2: the two line subs pair cross-asset — one vector fsub,
        // two f64x2 groups; A = 3: third sub is LONE (odd remainder), its
        // chains fall back to their own bucket pair.
        let p2 = classic3_all_on().build_assets(2);
        assert_eq!(p2.lanes(), vec![2, 2], "S pair group + L pair group");
        assert_eq!(p2.vec_subs(), 1, "one vector fsub serves both assets");
        let (r2, _qs) = p2.run_assets(&[streams[0].as_slice(), streams[1].as_slice()]);
        for asset in 0..2 {
            cmp_results(&r2[asset], &expect[asset], &format!("A2 asset{asset}"));
        }

        let p3 = classic3_all_on().build_assets(3);
        assert_eq!(p3.lanes(), vec![2, 2, 2]);
        assert_eq!(p3.vec_subs(), 1);
        let all: Vec<&[f64]> = streams.iter().map(|c| c.as_slice()).collect();
        let (r3, qs3) = p3.run_assets(&all);
        for asset in 0..3 {
            cmp_results(&r3[asset], &expect[asset], &format!("A3 asset{asset}"));
        }

        // streaming: resume a split at a phase boundary (24/25/26 leads)
        let s1: Vec<&[f64]> = streams.iter().map(|c| &c[..24]).collect();
        let s2: Vec<&[f64]> = streams.iter().map(|c| &c[24..]).collect();
        let (h1, qs) = p3.run_assets(&s1);
        let (h2, _) = p3.resume_assets(qs, &s2);
        for asset in 0..3 {
            for req in 0..expect[asset].outputs.len() {
                for out in 0..expect[asset].outputs[req].len() {
                    let mut c = h1[asset].outputs[req][out].to_vec();
                    c.extend_from_slice(&h2[asset].outputs[req][out]);
                    cmp_exact(
                        &c,
                        &expect[asset].outputs[req][out],
                        &format!("A3 stream r{req}o{out}"),
                    );
                }
            }
        }

        // per-asset hand states export identically to the single plan
        let st1 = one.run(&[c0.as_slice()]).1;
        let gs_a = p3.get_states_assets(&qs3);
        let gs_1 = one.get_states(&st1);
        let IndicatorStateBox::Apo(ap_a) = &gs_a[0][0] else {
            panic!("asset0 apo state");
        };
        let IndicatorStateBox::Apo(ap_1) = &gs_1[0] else {
            panic!("single apo state");
        };
        assert_eq!(ap_a.short_ema.ema.to_bits(), ap_1.short_ema.ema.to_bits());

        // dense across 2 assets, all optionals on: mirrors + clone path
        let d1 = dense_all_on().build();
        let e0 = d1.run(&[c0.as_slice()]).0;
        let e1 = d1.run(&[c1.as_slice()]).0;
        let dd = dense_all_on().build_assets(2);
        let (dr, _) = dd.run_assets(&[c0.as_slice(), c1.as_slice()]);
        cmp_results(&dr[0], &e0, "dense A2 asset0");
        cmp_results(&dr[1], &e1, "dense A2 asset1");
    }

    /// E3 stage 1: SMA / STDDEV / BBANDS fused through the arena — rings
    /// in the state image, warm fold until the hand's first trick bar,
    /// single-rounded `fma` bands. Bit-exact vs hand per line (values AND
    /// lengths), streaming split at every rolling boundary (lead, the
    /// fold→trick switch at lead+1, and mid-steady), and the CSE shares
    /// the rolling recurrences across requests (stddev's optional sma
    /// line, bbands' middle — one sum, one ring, many mirrors).
    #[test]
    fn jit_rolling_indicators_match_hand() {
        use crate::indicators::bbands::BBands;
        use crate::indicators::sma::Sma;
        use crate::indicators::stddev::StdDev;
        let close = series(500);
        let s: [&[f64]; 1] = [close.as_slice()];
        let req = Request::new()
            .add(Sma::recipe(&[20.0]), None)
            .add(StdDev::recipe(&[20.0]), Some(&[true]))
            .add(BBands::recipe(&[20.0, 2.0]), None);
        let plan = req.build();

        let (res, _) = plan.run(&s);
        let sma = Sma::indicator(&s, &[20.0], None).unwrap().0;
        cmp_exact(res.request(0).output(0), &sma[0], "sma");
        let sd = StdDev::indicator(&s, &[20.0], Some(&[true])).unwrap().0;
        cmp_exact(res.request(1).output(0), &sd[0], "stddev");
        cmp_exact(res.request(1).optional(0), &sd[1], "stddev_sma_opt");
        let bb = BBands::indicator(&s, &[20.0, 2.0], None).unwrap().0;
        for (i, nm) in [(0, "lo"), (1, "mid"), (2, "up")] {
            cmp_exact(res.request(2).output(i), &bb[i], nm);
        }
        // the middle line ≡ the standalone sma ≡ stddev's optional — views
        // over ONE row each, and the sums/rings are shared recurrences:
        assert!(res.outputs[0][0].shares_data_with(&res.optional[1][0]));
        assert!(res.outputs[0][0].shares_data_with(&res.outputs[2][1]));

        // streaming: splits at 19 (lead-1), 20 (lead), 21 (first trick
        // bar), 90 mid-steady — every run ++ resume == single run
        for split in [19usize, 20, 21, 22, 90] {
            let s1: [&[f64]; 1] = [&close[..split]];
            let s2: [&[f64]; 1] = [&close[split..]];
            let (h1, qs) = plan.run(&s1);
            let (h2, _) = plan.resume(qs, &s2);
            for req in 0..3 {
                for out in 0..h1.outputs[req].len() {
                    let mut c = h1.outputs[req][out].to_vec();
                    c.extend_from_slice(&h2.outputs[req][out]);
                    cmp_exact(
                        &c,
                        &res.request(req).output(out),
                        &format!("roll split{split} r{req}o{out}"),
                    );
                }
            }
        }
    }

    /// E3 get_states (A-structured) end-to-end on the simplest rolling
    /// kind: run the first chunk, `get_states` rebuilds the hand
    /// `sma::State<Warm>` from the group sum, the caller re-pairs it with
    /// the host `window` (the price tail the hand `IndicatorState` also
    /// holds), and `batch_indicator` over the rest must be BIT-EXACT to the
    /// hand's own cold full run — proving the restored state is genuinely
    /// resumable, not just a matching snapshot.
    #[test]
    fn jit_get_states_sma_continues_bit_exact() {
        use crate::indicator_types::TIndicatorState;
        use crate::indicators::sma::{IndicatorState as SmaIS, Sma};
        let close = series(500);
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = Request::new().add(Sma::recipe(&[20.0]), None).build();
        // cold full run == the hand reference for the whole series
        let hand = Sma::indicator(&s, &[20.0], None).unwrap().0;

        // advance 200 bars, then export state + host window
        let split = 200usize;
        let s1: [&[f64]; 1] = [&close[..split]];
        let (_r1, qs) = plan.run(&s1);
        let states = plan.get_states(&qs);
        let IndicatorStateBox::Sma(restored) = &states[0] else {
            panic!("sma get_states did not rebuild the hand state");
        };

        // rebuild the hand IndicatorState from (restored recurrence, host
        // price tail) — the window the engine keeps outside the words
        let tail = &qs.window[0];
        let mut is = SmaIS::new(tail, restored.clone(), 20);
        let s2: [&[f64]; 1] = [&close[split..]];
        let cont = is.batch_indicator(&s2, None).unwrap();

        // continuation values == the hand cold-run tail from bar 200 on
        cmp_exact(
            &cont[0],
            &hand[0][split - 20..],
            "sma get_states continuation",
        );
    }

    /// E3 stage 2: WMA trick over the shared rolling group — a
    /// wma+sma+stddev+bbands query on ONE window advances one ring, one
    /// cursor, one raw sum (the `×1/P` multiply appears only in the lines
    /// that need it), and stores dedup to five rows. Bit-exact vs hand,
    /// streaming splits at the fold→trick seams.
    #[test]
    fn jit_wma_matches_hand() {
        use crate::indicators::bbands::BBands;
        use crate::indicators::sma::Sma;
        use crate::indicators::stddev::StdDev;
        use crate::indicators::wma::Wma;
        let close = series(500);
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = Request::new()
            .add(Wma::recipe(&[20.0]), Some(&[true]))
            .add(Sma::recipe(&[20.0]), None)
            .add(StdDev::recipe(&[20.0]), Some(&[true]))
            .add(BBands::recipe(&[20.0, 2.0]), None)
            .build();
        // five unique write streams for SIX slots (wma, sma×3 shared, sd,
        // lo, up): the three sma consumers + bbands' middle collapse.
        assert_eq!(plan.info().1, 5);
        let (res, _) = plan.run(&s);

        let w = Wma::indicator(&s, &[20.0], Some(&[true])).unwrap().0;
        cmp_exact(res.request(0).output(0), &w[0], "wma");
        cmp_exact(res.request(0).optional(0), &w[1], "wma_sma_opt");
        let sm = Sma::indicator(&s, &[20.0], None).unwrap().0;
        cmp_exact(res.request(1).output(0), &sm[0], "sma_again");
        let sd = StdDev::indicator(&s, &[20.0], Some(&[true])).unwrap().0;
        cmp_exact(res.request(2).output(0), &sd[0], "stddev_w");
        cmp_exact(res.request(2).optional(0), &sd[1], "stddev_sma_w");
        let bb = BBands::indicator(&s, &[20.0, 2.0], None).unwrap().0;
        for (i, nm) in [(0, "lo"), (1, "mid"), (2, "up")] {
            cmp_exact(res.request(3).output(i), &bb[i], nm);
        }
        assert!(res.optional[0][0].shares_data_with(&res.outputs[1][0]));

        for split in [19usize, 20, 21, 22, 90] {
            let s1: [&[f64]; 1] = [&close[..split]];
            let s2: [&[f64]; 1] = [&close[split..]];
            let (h1, qs) = plan.run(&s1);
            let (h2, _) = plan.resume(qs, &s2);
            for req in 0..4 {
                for out in 0..h1.outputs[req].len() {
                    let mut c = h1.outputs[req][out].to_vec();
                    c.extend_from_slice(&h2.outputs[req][out]);
                    cmp_exact(
                        &c,
                        &res.request(req).output(out),
                        &format!("wma split{split} r{req}o{out}"),
                    );
                }
            }
        }
    }

    /// E3 coverage the single-window tests miss: a DASHBOARD of different
    /// option values — five distinct windows (5, 8, 14, 20, 26) across
    /// four kinds, with deliberate sharing (three consumers of window 20
    /// ride ONE group: one sum, one stitched window, one anchor) and fully
    /// independent groups elsewhere. Different periods also means the
    /// phase-split now carries six leads + six fold→trick seams; streaming
    /// splits straddle several of them at once.
    #[test]
    fn jit_rolling_dashboard_mixed_windows() {
        use crate::indicators::bbands::BBands;
        use crate::indicators::sma::Sma;
        use crate::indicators::stddev::StdDev;
        use crate::indicators::wma::Wma;
        let close = series(600);
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = Request::new()
            .add(Sma::recipe(&[5.0]), None)
            .add(Wma::recipe(&[8.0]), Some(&[true])) // wma + its sma(8) opt
            .add(StdDev::recipe(&[14.0]), Some(&[true]))
            .add(Sma::recipe(&[20.0]), None) // window-20 group …
            .add(StdDev::recipe(&[20.0]), Some(&[true])) // … same group
            .add(BBands::recipe(&[20.0, 2.5]), None) // … same group
            .add(Wma::recipe(&[26.0]), None)
            .build();
        // slots: 1+2+1+2+2+3+1 = 12; window-20's sma line is consumed 3×
        // (standalone, stddev-opt, bbands-mid) → ONE store ⇒ 10 rows.
        let (ops, stores) = plan.info();
        assert_eq!(stores, 10, "window-20 sma line dedups across requests");
        assert!(ops >= 13, "5 group sums + arith");

        let (res, _) = plan.run(&s);
        // (request, output slot) walked explicitly against hand
        let h0 = Sma::indicator(&s, &[5.0], None).unwrap().0;
        cmp_exact(res.request(0).output(0), &h0[0], "sma5");
        let h1 = Wma::indicator(&s, &[8.0], Some(&[true])).unwrap().0;
        cmp_exact(res.request(1).output(0), &h1[0], "wma8");
        cmp_exact(res.request(1).optional(0), &h1[1], "sma8_opt");
        let h2 = StdDev::indicator(&s, &[14.0], Some(&[true])).unwrap().0;
        cmp_exact(res.request(2).output(0), &h2[0], "stddev14");
        cmp_exact(res.request(2).optional(0), &h2[1], "sma14_opt");
        let h3 = Sma::indicator(&s, &[20.0], None).unwrap().0;
        cmp_exact(res.request(3).output(0), &h3[0], "sma20");
        let h4 = StdDev::indicator(&s, &[20.0], Some(&[true])).unwrap().0;
        cmp_exact(res.request(4).output(0), &h4[0], "stddev20");
        cmp_exact(res.request(4).optional(0), &h3[0], "stddev20_sma_eq");
        let h5 = BBands::indicator(&s, &[20.0, 2.5], None).unwrap().0;
        for i in 0..3 {
            cmp_exact(res.request(5).output(i), &h5[i], "bbands20_25");
        }
        // the window-20 sma line is ONE row behind all four consumers
        assert!(res.outputs[3][0].shares_data_with(&res.optional[4][0]));
        assert!(res.outputs[3][0].shares_data_with(&res.outputs[5][1]));
        let h6 = Wma::indicator(&s, &[26.0], None).unwrap().0;
        cmp_exact(res.request(6).output(0), &h6[0], "wma26");

        // streaming across several groups' seams simultaneously: 13/14/15
        // (stddev14), 19/20/21 (window-20 trio), 25/26/27 (wma26)
        for split in [4usize, 5, 6, 14, 15, 20, 21, 26, 27, 120] {
            let s1: [&[f64]; 1] = [&close[..split]];
            let s2: [&[f64]; 1] = [&close[split..]];
            let (h1r, qs) = plan.run(&s1);
            let (h2r, _) = plan.resume(qs, &s2);
            for req in 0..7 {
                for out in 0..h1r.outputs[req].len() {
                    let mut c = h1r.outputs[req][out].to_vec();
                    c.extend_from_slice(&h2r.outputs[req][out]);
                    cmp_exact(
                        &c,
                        &res.request(req).output(out),
                        &format!("mix split{split} r{req}o{out}"),
                    );
                }
                for out in 0..h1r.optional[req].len() {
                    let mut c = h1r.optional[req][out].to_vec();
                    c.extend_from_slice(&h2r.optional[req][out]);
                    cmp_exact(
                        &c,
                        &res.request(req).optional(out),
                        &format!("mix split{split} r{req}x{out}"),
                    );
                }
            }
        }
    }

    /// E3 anchor stitch at the EXTREME chunk shape: one bar per resume.
    /// Every tick re-stitches `window ++ chunk`, and for the first `pmax`
    /// ticks the window is still SHORT — the shifted-anchor path that no
    /// two-way split exercises. Stitching may change ADDRESSES only; the
    /// 1-bar-at-a-time stream must be bit-identical to one full run.
    #[test]
    fn jit_rolling_tick_torture_stitch_every_bar() {
        use crate::indicators::bbands::BBands;
        use crate::indicators::sma::Sma;
        use crate::indicators::stddev::StdDev;
        use crate::indicators::wma::Wma;
        let close = series(300);
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = Request::new()
            .add(Sma::recipe(&[5.0]), None)
            .add(Wma::recipe(&[8.0]), None)
            .add(StdDev::recipe(&[14.0]), Some(&[true]))
            .add(BBands::recipe(&[20.0, 2.5]), None)
            .add(Wma::recipe(&[26.0]), None)
            .build();
        let (full, _) = plan.run(&s);

        let (r0, mut qs) = plan.run(&[&close[..1]]);
        let mut acc_p: Vec<Vec<Vec<f64>>> = Vec::new();
        let mut acc_x: Vec<Vec<Vec<f64>>> = Vec::new();
        for req in 0..full.outputs.len() {
            acc_p.push(
                (0..r0.outputs[req].len())
                    .map(|o| r0.outputs[req][o].to_vec())
                    .collect(),
            );
            acc_x.push(
                (0..r0.optional[req].len())
                    .map(|o| r0.optional[req][o].to_vec())
                    .collect(),
            );
        }
        drop(r0);
        for b in 1..close.len() {
            let (r, q2) = plan.resume(qs, &[&close[b..b + 1]]);
            for req in 0..full.outputs.len() {
                for o in 0..r.outputs[req].len() {
                    acc_p[req][o].extend_from_slice(&r.outputs[req][o]);
                }
                for o in 0..r.optional[req].len() {
                    acc_x[req][o].extend_from_slice(&r.optional[req][o]);
                }
            }
            drop(r);
            qs = q2;
        }
        for req in 0..full.outputs.len() {
            for o in 0..full.outputs[req].len() {
                cmp_exact(
                    &acc_p[req][o],
                    &full.request(req).output(o),
                    &format!("tick r{req}o{o}"),
                );
            }
            for o in 0..full.optional[req].len() {
                cmp_exact(
                    &acc_x[req][o],
                    &full.request(req).optional(o),
                    &format!("tick r{req}x{o}"),
                );
            }
        }
    }

    /// E3 stage 3: HMA as ONE bespoke op — bit-exact vs hand
    /// `Hma::indicator` incl. the degenerate `psqrt == 1` periods (2, 3 —
    /// the ring-pop alias case) and streaming splits at ALL three seams
    /// (engine-2 fold start, fill start, output start) plus mid-steady.
    #[test]
    fn jit_hma_matches_hand() {
        use crate::indicators::hma::Hma;
        let close = series(600);
        let s: [&[f64]; 1] = [close.as_slice()];
        for period in [2u32, 3, 4, 9, 20, 50] {
            let plan = Request::new()
                .add(Hma::recipe(&[period as f64]), None)
                .build();
            let (res, _) = plan.run(&s);
            let h = Hma::indicator(&s, &[period as f64], None).unwrap().0;
            cmp_exact(res.request(0).output(0), &h[0], &format!("hma{period}"));
            let p = period as usize;
            let psqrt = (p as f64).sqrt() as usize;
            let fold2 = p - p / 2;
            // every seam ± 1 and a couple of steady interior cuts
            let mut splits = vec![
                fold2 - 1,
                fold2,
                fold2 + 1,
                p - 1,
                p,
                p + 1,
                p + psqrt - 1,
                p + psqrt,
                p + psqrt + 1,
                3 * p,
                3 * p + 1,
            ];
            splits.retain(|&x| x > 0 && x < close.len() as usize);
            for split in splits {
                let s1: [&[f64]; 1] = [&close[..split]];
                let s2: [&[f64]; 1] = [&close[split..]];
                let (h1, qs) = plan.run(&s1);
                let (h2, _) = plan.resume(qs, &s2);
                let mut c = h1.outputs[0][0].to_vec();
                c.extend_from_slice(&h2.outputs[0][0]);
                cmp_exact(&c, &h[0], &format!("hma{period} split{split}"));
            }
        }
    }

    /// E3 stage 3 composition: HMA alongside the shared window-20 group.
    /// The HMA is SELF-CONTAINED (its two private engines + diff ring do
    /// NOT join the `(src, period)` group), so an `Hma(20) + Sma(20) +
    /// BBands(20)` query keeps the sma/stddev/bbands sharing intact AND
    /// the HMA independent — all four bit-exact, streaming across the
    /// HMA fill→steady seam (bar 24) proves the cursor + ring state image
    /// round-trips with the anchor window.
    #[test]
    fn jit_hma_composes_with_shared_group() {
        use crate::indicators::bbands::BBands;
        use crate::indicators::hma::Hma;
        use crate::indicators::sma::Sma;
        let close = series(400);
        let s: [&[f64]; 1] = [close.as_slice()];
        let plan = Request::new()
            .add(Hma::recipe(&[20.0]), None)
            .add(Sma::recipe(&[20.0]), None)
            .add(BBands::recipe(&[20.0, 2.0]), None)
            .build();
        let (res, _) = plan.run(&s);
        let hh = Hma::indicator(&s, &[20.0], None).unwrap().0;
        cmp_exact(res.request(0).output(0), &hh[0], "hma20");
        let sma = Sma::indicator(&s, &[20.0], None).unwrap().0;
        cmp_exact(res.request(1).output(0), &sma[0], "sma20");
        let bb = BBands::indicator(&s, &[20.0, 2.0], None).unwrap().0;
        for (i, nm) in [(0, "lo"), (1, "mid"), (2, "up")] {
            cmp_exact(res.request(2).output(i), &bb[i], nm);
        }
        // Row dedup (the E2c single arena makes `shares_data_with` prove
        // only arena identity): the WRITE-STREAM count shows the sharing.
        // hma = 1 row; the window-20 sma line is ONE row for BOTH the
        // standalone sma AND the bbands middle; bbands lo/up = 2 more ⇒ 4.
        let (_ops, stores) = plan.info();
        assert_eq!(stores, 4, "hma row + shared sma/mid row + bbands lo/up");

        // streaming across the HMA fill→steady seam (20 + psqrt(20)=4 → 24)
        for split in [23usize, 24, 25, 120] {
            let s1: [&[f64]; 1] = [&close[..split]];
            let s2: [&[f64]; 1] = [&close[split..]];
            let (h1, qs) = plan.run(&s1);
            let (h2, _) = plan.resume(qs, &s2);
            for req in 0..3 {
                for out in 0..h1.outputs[req].len() {
                    let mut c = h1.outputs[req][out].to_vec();
                    c.extend_from_slice(&h2.outputs[req][out]);
                    cmp_exact(
                        &c,
                        &res.request(req).output(out),
                        &format!("hma-comp split{split} r{req}o{out}"),
                    );
                }
            }
        }
    }

    /// E3 get_states on the HARDEST rolling kind: HMA. `get_states` must
    /// reconstitute the two inner WMA engines, the `wss`/`ss` accumulators,
    /// AND the `prev_diff: Buffer<Warm>` (the engine's only stored-CALCULATION
    /// ring) from the op's word run. Re-paired with the host `window` (the
    /// trailing `period` prices the hand `IndicatorState` also holds), the
    /// hand `batch_indicator` over the rest must be BIT-EXACT to the hand's
    /// own cold run — the same bar-for-bar proof as SMA, on the case where a
    /// wrong cursor/ring/order would silently corrupt the windowed sum.
    #[test]
    fn jit_get_states_hma_continues_bit_exact() {
        use crate::indicator_types::TIndicatorState;
        use crate::indicators::hma::{Hma, IndicatorState as HmaIS};
        let close = series(600);
        let s: [&[f64]; 1] = [close.as_slice()];
        for period in [4u32, 9, 20] {
            let p = period as usize;
            let psqrt = (p as f64).sqrt() as usize; // hand HMA lead = p + psqrt
            let plan = Request::new()
                .add(Hma::recipe(&[period as f64]), None)
                .build();
            let hand = Hma::indicator(&s, &[period as f64], None).unwrap().0;
            let split = 200usize;
            let s1: [&[f64]; 1] = [&close[..split]];
            let (_r1, qs) = plan.run(&s1);
            let states = plan.get_states(&qs);
            let IndicatorStateBox::Hma(restored) = &states[0] else {
                panic!("hma get_states did not rebuild the hand state");
            };
            // the window the hand re-indexes against == the host tail
            let mut is = HmaIS::new(&qs.window[0], restored.clone(), p, p / 2);
            let s2: [&[f64]; 1] = [&close[split..]];
            let cont = is.batch_indicator(&s2, None).unwrap();
            cmp_exact(
                &cont[0],
                &hand[0][split - (p + psqrt)..],
                &format!("hma{period} get_states continuation"),
            );
        }
    }

    /// E3 rolling kinds (group + HMA) inside an E2 ASSET-LANE plan: the
    /// replicated arena must give every symbol its OWN sum / aux / ring /
    /// cursor words (the group key is `(src-per-asset, period)`, and HMA's
    /// word run is per-op) while the host anchor window is already
    /// per-stream. Every lane bit-exact vs its single-asset run, vs the
    /// hand, and streaming from a mid-series split.
    #[test]
    fn jit_rolling_asset_lanes_partition_state() {
        use crate::indicators::bbands::BBands;
        use crate::indicators::hma::Hma;
        use crate::indicators::sma::Sma;
        use crate::indicators::stddev::StdDev;
        use crate::indicators::wma::Wma;
        let c0 = series(500);
        let c1: Vec<f64> = c0.iter().map(|v| v * 0.5 + 3.0).collect();
        let c2: Vec<f64> = c0.iter().map(|v| v * 2.0 - 7.0).collect();
        let streams = [&c0, &c1, &c2];
        let req = || {
            Request::new()
                .add(Sma::recipe(&[20.0]), None)
                .add(StdDev::recipe(&[14.0]), Some(&[true]))
                .add(Wma::recipe(&[26.0]), None)
                .add(BBands::recipe(&[20.0, 2.5]), None)
                .add(Hma::recipe(&[9.0]), None)
        };
        let one = req().build();
        let expect: Vec<Results> = streams.iter().map(|c| one.run(&[c.as_slice()]).0).collect();

        // hand check on lane 0 for every kind (requests 0..4)
        let h0: [&[f64]; 1] = [c0.as_slice()];
        cmp_exact(
            expect[0].request(0).output(0),
            &Sma::indicator(&h0, &[20.0], None).unwrap().0[0],
            "A3 lane0 sma20",
        );
        cmp_exact(
            expect[0].request(1).output(0),
            &StdDev::indicator(&h0, &[14.0], Some(&[true])).unwrap().0[0],
            "A3 lane0 stddev14",
        );
        cmp_exact(
            expect[0].request(2).output(0),
            &Wma::indicator(&h0, &[26.0], None).unwrap().0[0],
            "A3 lane0 wma26",
        );
        cmp_exact(
            expect[0].request(4).output(0),
            &Hma::indicator(&h0, &[9.0], None).unwrap().0[0],
            "A3 lane0 hma9",
        );

        let lanes = req().build_assets(3);
        let ss: Vec<&[f64]> = streams.iter().map(|c| c.as_slice()).collect();
        let (r3, _qs) = lanes.run_assets(&ss);
        for asset in 0..3 {
            cmp_results(&r3[asset], &expect[asset], &format!("roll-A3 asset{asset}"));
        }

        // streaming resume, all three lanes from a mid-series split
        let p: Vec<&[f64]> = streams.iter().map(|c| &c[..100]).collect();
        let (r1, qs) = lanes.run_assets(&p);
        let q: Vec<&[f64]> = streams.iter().map(|c| &c[100..]).collect();
        let (r2, _) = lanes.resume_assets(qs, &q);
        for asset in 0..3 {
            for rq in 0..expect[asset].outputs.len() {
                for out in 0..expect[asset].outputs[rq].len() {
                    let mut cc = r1[asset].outputs[rq][out].to_vec();
                    cc.extend_from_slice(&r2[asset].outputs[rq][out]);
                    cmp_exact(
                        &cc,
                        &expect[asset].request(rq).output(out),
                        &format!("roll-A3 stream a{asset} r{rq}o{out}"),
                    );
                }
            }
        }
    }

    /// E6 leaf: Wilders smoothing, single price source. Bit-exact vs hand
    /// `Wilders::indicator` across periods whose warmup straddles the
    /// fold→steady seam (the `÷period` mean at bar `period-1`, then the
    /// decayed IIR), plus streaming every split around that seam, plus a
    /// `get_states` hand-continuation proof (restore → hand `batch_indicator`
    /// over the rest == hand cold run). The engine's first ADX-family node.
    #[test]
    fn jit_wilders_matches_hand() {
        use crate::indicator_types::TIndicatorState;
        use crate::indicators::wilders::Wilders;
        let close = series(500);
        let s: [&[f64]; 1] = [close.as_slice()];
        for period in [1u32, 2, 3, 9, 14] {
            let p = period as usize;
            let plan = Request::new()
                .add(Wilders::recipe(&[period as f64]), None)
                .build();
            let (res, _) = plan.run(&s);
            let h = Wilders::indicator(&s, &[period as f64], None).unwrap();
            cmp_exact(
                res.request(0).output(0),
                &h.0[0],
                &format!("wilders{period}"),
            );

            // streaming: splits at the seam (period-1, period) and interior
            for split in [p.saturating_sub(1), p, p + 1, p + 2, 2 * p, 250] {
                if split == 0 || split >= close.len() {
                    continue;
                }
                let s1: [&[f64]; 1] = [&close[..split]];
                let s2: [&[f64]; 1] = [&close[split..]];
                let (h1, qs) = plan.run(&s1);
                let (h2, _) = plan.resume(qs, &s2);
                let mut c = h1.outputs[0][0].to_vec();
                c.extend_from_slice(&h2.outputs[0][0]);
                cmp_exact(&c, &h.0[0], &format!("wilders{period} split{split}"));
            }

            // get_states continuation: wilders is a single chain (no price
            // window) — restore it and run the HAND's batch_indicator over
            // the rest == the hand cold run's tail from bar `split`.
            let split = 300usize;
            let s1: [&[f64]; 1] = [&close[..split]];
            let (_r1, qs) = plan.run(&s1);
            let states = plan.get_states(&qs);
            let IndicatorStateBox::Wilders(restored) = &states[0] else {
                panic!("wilders get_states did not rebuild the hand state");
            };
            let mut is = restored.clone();
            let s2: [&[f64]; 1] = [&close[split..]];
            let cont = is.batch_indicator(&s2, None).unwrap();
            // hand wilders publishes from bar `period`; align the tail.
            cmp_exact(
                &cont[0],
                &h.0[0][split - p..],
                &format!("wilders{period} get_states continuation"),
            );
        }
    }

    /// PRE-FLIGHT proof for the min/max family (E9 M0): the hand rescans its
    /// window with `find_max_simd::<4>` / `find_min_simd::<4>`, and the engine
    /// intends to emit a SCALAR `>=`-forward scan. Both must return the
    /// *newest* occurrence of the extremum, or `aroon` (which consumes the
    /// trail, not the value) would print a different line. Swept over window
    /// lengths that cross both chunk boundaries (N=2 and N=4) and over the
    /// tie shapes that can break an index: plateau at the extremum, extremum
    /// only at index 0 (the lane the SIMD form skips in `search_window`),
    /// extremum repeated in the remainder, all-equal, and monotone.
    #[test]
    fn rescan_scalar_and_simd_forms_agree_on_extremum_and_index() {
        use crate::indicators::{max, min};
        // Deterministic shape sweep — no RNG, every case is hand-checkable.
        let mut windows: Vec<Vec<f64>> = Vec::new();
        for len in 1..=40usize {
            // plateau at the max (newest occurrence is the LAST lane)
            let mut w = vec![1.0; len];
            for (i, x) in w.iter_mut().enumerate() {
                if i >= len / 2 {
                    *x = 5.0;
                }
            }
            windows.push(w);
            // extremum ONLY at index 0 — the `search_window = &window[1..]` skip
            let mut w = vec![0.0; len];
            w[0] = 9.0;
            windows.push(w.clone());
            // extremum repeated across a chunk boundary and in the remainder
            for off in [len - 1, len / 2, len.saturating_sub(3)] {
                let mut w = vec![-3.0; len];
                w[1.min(len - 1)] = 7.0;
                w[off] = 7.0;
                windows.push(w);
            }
            // strictly monotone up and down
            windows.push((0..len).map(|i| i as f64).collect());
            windows.push((0..len).map(|i| (len - i) as f64).collect());
            // every lane equal
            windows.push(vec![2.5; len]);
            // equal values interleaved with a distinct max in the remainder
            let mut w: Vec<f64> = (0..len).map(|i| (i % 3) as f64).collect();
            if len > 5 {
                w[len - 2] = 4.0;
                w[2] = 4.0;
            }
            windows.push(w);
        }
        for w in &windows {
            let scalar = max::find_max_scalar(w);
            for n in [2usize, 4] {
                let simd = match n {
                    2 => max::find_max_simd::<2>(w),
                    _ => max::find_max_simd::<4>(w),
                };
                assert_eq!(
                    simd, scalar,
                    "max: N={n} disagrees on window {w:?} (scalar {scalar:?})"
                );
            }
            let scalar = min::find_min_scalar(w);
            for n in [2usize, 4] {
                let simd = match n {
                    2 => min::find_min_simd::<2>(w),
                    _ => min::find_min_simd::<4>(w),
                };
                assert_eq!(
                    simd, scalar,
                    "min: N={n} disagrees on window {w:?} (scalar {scalar:?})"
                );
            }
        }
    }

    /// E9 M0 data shapes for the roll tests. Two of them force the rescan on
    /// almost EVERY bar (`falling` for max, `rising` for min: the carried
    /// extremum is always the oldest bar in the window, so the trail hits
    /// `period` constantly) and one is all ties (`ties`, where the
    /// newest-occurrence rule is the only thing deciding `trail` — the shapes
    /// a value-only test would never catch).
    fn roll_datasets() -> Vec<(&'static str, Vec<f64>)> {
        vec![
            ("wiggle", series(500)),
            ("falling", (0..300).map(|i| -(i as f64)).collect()),
            ("rising", (0..300).map(|i| i as f64).collect()),
            ("ties", (0..300).map(|i| ((i * 7) % 11) as f64).collect()),
            ("flat", vec![3.25; 300]),
        ]
    }

    /// E9 M0: the trail-rescan roll standalone, bit-exact vs the hand over
    /// every data shape × period, then streaming (splits at the first printed
    /// bar `period−1`, the guard seam `period`, one bar past it, and
    /// mid-steady) — `run ++ resume == hand` on the same row.
    ///
    /// Period ≥ 2 by construction: a period-1 roll publishes from bar 0 while
    /// its cold seed CONSUMES bar 0, and no seeded op in the engine prints a
    /// bar it has already folded (a general boundary fact of the phase-split
    /// emitter, not a roll-specific limit — `seeds_bar0`).
    #[test]
    fn jit_max_matches_hand() {
        roll_parity(true)
    }

    #[test]
    fn jit_min_matches_hand() {
        roll_parity(false)
    }

    fn roll_parity(want_max: bool) {
        use crate::indicators::{max::Max, min::Min};
        let which = if want_max { "max" } else { "min" };
        // Plan PER PERIOD, not per dataset×period: the compiled loop is
        // data-length-independent, so rebuilding it per series was pure churn
        // (each `build` mints a fresh cranelift function in the shared
        // executable arena — the one thing the suite must not multiply).
        for period in [2usize, 5, 14, 50] {
            let opt = [period as f64];
            let recipe = if want_max {
                Max::recipe(&opt)
            } else {
                Min::recipe(&opt)
            };
            let plan = Request::new().add(recipe, None).build();
            for (shape, data) in roll_datasets() {
                let n = data.len();
                let s: [&[f64]; 1] = [data.as_slice()];
                let hand = if want_max {
                    Max::indicator(&s, &opt, None).unwrap().0
                } else {
                    Min::indicator(&s, &opt, None).unwrap().0
                };
                let (res, _) = plan.run(&s);
                cmp_exact(
                    res.request(0).output(0),
                    &hand[0],
                    &format!("{which} {shape} p{period}"),
                );

                for split in [period - 1, period, period + 1, n / 3] {
                    let s1: [&[f64]; 1] = [&data[..split]];
                    let s2: [&[f64]; 1] = [&data[split..]];
                    let (h1, qs) = plan.run(&s1);
                    let (h2, _) = plan.resume(qs, &s2);
                    let mut c = h1.outputs[0][0].to_vec();
                    c.extend_from_slice(&h2.outputs[0][0]);
                    cmp_exact(
                        &c,
                        &hand[0],
                        &format!("{which} {shape} p{period} split{split}"),
                    );
                }
            }
        }
    }

    /// E9: `get_states` on the rolls rebuilds the hand `{max, trail}` /
    /// `{min, trail}`; re-paired with the host anchor window (the rescan window
    /// the hand `IndicatorState` also holds outside the words), the HAND's
    /// `batch_indicator` over the rest is bit-exact to its cold full run.
    /// Max and min ride ONE plan here, so this also proves two trail-carrying
    /// ops coexist (two i64 params, one shared anchor) without interference.
    #[test]
    fn jit_get_states_roll_continues_bit_exact() {
        use crate::indicator_types::TIndicatorState;
        use crate::indicators::{
            max::{IndicatorState as MaxIS, Max},
            min::{IndicatorState as MinIS, Min},
        };
        let data = roll_datasets()[0].1.clone();
        let p = 14usize;
        let opt = [p as f64];
        let s: [&[f64]; 1] = [data.as_slice()];
        let hand_max = Max::indicator(&s, &opt, None).unwrap().0;
        let hand_min = Min::indicator(&s, &opt, None).unwrap().0;

        let plan = Request::new()
            .add(Max::recipe(&opt), None)
            .add(Min::recipe(&opt), None)
            .build();
        assert_eq!(plan.info().0, 2, "one roll op per request");

        let split = 200usize;
        let s1: [&[f64]; 1] = [&data[..split]];
        let (_r1, qs) = plan.run(&s1);
        let states = plan.get_states(&qs);
        let (IndicatorStateBox::Max(mx), IndicatorStateBox::Min(mn)) = (&states[0], &states[1])
        else {
            panic!("roll get_states did not rebuild the hand states");
        };
        assert_eq!(
            qs.window[0].len(),
            p - 1,
            "the anchor window is the roll's rescan tail"
        );

        let tail = &qs.window[0];
        let s2: [&[f64]; 1] = [&data[split..]];
        let mut is = MaxIS::new(tail, mx.clone(), (p, p - 1));
        let cont = is.batch_indicator(&s2, None).unwrap();
        cmp_exact(
            &cont[0],
            &hand_max[0][split - (p - 1)..],
            "max get_states continuation",
        );

        let mut is = MinIS::new(tail, mn.clone(), (p, p - 1));
        let cont = is.batch_indicator(&s2, None).unwrap();
        cmp_exact(
            &cont[0],
            &hand_min[0][split - (p - 1)..],
            "min get_states continuation",
        );
    }

    /// E9: the field axis with TWO rolls bound to DIFFERENT columns — the
    /// shape `aroon`/`willr`/`donchian` require (max over high, min over low):
    /// one request, two sinks, `n_fields = 2`, bit-exact against each hand
    /// indicator on its own column, cold and streamed.
    ///
    /// This has to be bound INSIDE the recipe, and that is the point of the
    /// test: `Max::recipe` and `Min::recipe` each declare `Input{stream: 0}`
    /// and the merged arena gives every request the same physical column per
    /// index (a two-request `add(Max).add(Min)` query has `n_streams() == 1`
    /// and computes BOTH over one series). Field binding is recipe-internal
    /// today — the gap E8's named columns/`Calc` axis would close.
    #[test]
    fn jit_roll_pair_binds_two_fields() {
        use crate::cse::Recipe;
        use crate::indicators::{max::Max, min::Min};
        let period = 14u32;
        let mut r = Recipe::new();
        let h = r.add(Node::Input { stream: 0 });
        let l = r.add(Node::Input { stream: 1 });
        let mx = r.add(Node::RollMax {
            src: h,
            period,
            look_back: period - 1,
        });
        let mn = r.add(Node::RollMin {
            src: l,
            period,
            look_back: period - 1,
        });
        r.kind = "roll_pair";
        r.options = vec![period as f64];
        r.outputs = vec![Sink::iface(mx), Sink::iface(mn)];
        let plan = Request::new().add(r, None).build();
        assert_eq!(plan.n_streams(), 2, "both fields are live");
        assert_eq!(plan.info().0, 2, "two roll ops, no duplication");

        let hi = series(500);
        let lo: Vec<f64> = hi
            .iter()
            .enumerate()
            .map(|(i, v)| *v - 1.0 - (i % 4) as f64)
            .collect();
        let s: [&[f64]; 2] = [hi.as_slice(), lo.as_slice()];
        let want_max = Max::indicator(&[hi.as_slice()], &[period as f64], None)
            .unwrap()
            .0;
        let want_min = Min::indicator(&[lo.as_slice()], &[period as f64], None)
            .unwrap()
            .0;

        let (res, _) = plan.run(&s);
        cmp_exact(res.request(0).output(0), &want_max[0], "pair max");
        cmp_exact(res.request(0).output(1), &want_min[0], "pair min");

        for split in [13usize, 14, 15, 250] {
            let cut: Vec<&[f64]> = s.iter().map(|x| &x[..split]).collect();
            let tail: Vec<&[f64]> = s.iter().map(|x| &x[split..]).collect();
            let (r1, qs) = plan.run(&cut);
            let r2 = plan.resume(qs, &tail).0;
            for (row, want) in [(0usize, &want_max[0]), (1, &want_min[0])] {
                let mut cat = r1.request(0).output(row).to_vec();
                cat.extend_from_slice(r2.request(0).output(row));
                cmp_exact(
                    &cat,
                    want,
                    &format!("pair split{split} row{row} (each column on its own roll)"),
                );
            }
        }
    }

    /// The two scan ORDERS the engine could emit for a rescan produce the same
    /// `(value, trail)`. FORWARD (what the emitter does): oldest → newest,
    /// NON-STRICT, carry the window slot, `trail = look_back − slot`. REVERSE (the
    /// form the by-option hand's `find_max_scalar(window, current)` uses, and the
    /// shape a chunked scan needs): newest → oldest, STRICT, seeded with a running
    /// `(value, trail)` pair so the ORDER does the tie-breaking and a win simply
    /// takes the winner's age — no window coordinates at all, which is why it is
    /// the natural form for the seed-inheriting gap walk. Kept as a pre-flight for
    /// that direction of work: the reverse form is bit-equal here yet measured ~1
    /// cycle/step slower through cranelift (codegen's `walk!` comment has the
    /// numbers), so equivalence is established independently of the emitter.
    #[test]
    fn reverse_walk_matches_forward_walk_including_trail() {
        #[derive(Clone, Copy, Debug)]
        enum Dir {
            Max,
            Min,
        }
        use Dir::*;
        let better = |d: Dir, a: f64, b: f64| match d {
            Max => a > b,
            Min => a < b,
        };

        // forward: oldest → newest, NON-STRICT, carry the slot, trail = lb − slot.
        let forward = |w: &[f64], d: Dir| -> (f64, usize) {
            let lb = w.len() - 1;
            let mut v = w[0];
            let mut sl = 0usize;
            for k in 1..w.len() {
                let beat = match d {
                    Max => w[k] >= v,
                    Min => w[k] <= v,
                };
                if beat {
                    v = w[k];
                    sl = k;
                }
            }
            (v, lb - sl)
        };

        // reverse: ages newest → oldest, STRICT, seeded with (value, trail); a win
        // takes the winner's AGE, so no window coordinates exist to convert.
        let reverse = |bars: &[f64],
                       i: usize,
                       lb: usize,
                       from_age: usize,
                       d: Dir,
                       seed: (f64, usize)|
         -> (f64, usize) {
            let mut v = seed.0;
            let mut t = seed.1;
            for age in from_age..=lb {
                let x = bars[i - age];
                if better(d, x, v) {
                    v = x;
                    t = age;
                }
            }
            (v, t)
        };

        let datasets = vec![
            (
                "ties",
                (0..120usize).map(|k| (k % 5) as f64).collect::<Vec<f64>>(),
            ),
            (
                "plateau",
                (0..120usize)
                    .map(|k| if k % 17 < 8 { 3.0 } else { -2.0 })
                    .collect::<Vec<f64>>(),
            ),
            (
                "rising",
                (0..120usize).map(|k| k as f64).collect::<Vec<f64>>(),
            ),
            (
                "falling",
                (0..120usize)
                    .map(|k| 120.0 - k as f64)
                    .collect::<Vec<f64>>(),
            ),
            ("wiggle", series(120)),
        ];
        for (shape, bars) in datasets {
            for d in [Max, Min] {
                for lb in [1usize, 2, 3, 7, 13] {
                    // UNSEEDED: seed = this bar's own price at trail 0, walk ages 1..=lb.
                    for i in lb..bars.len() {
                        let full: Vec<f64> = bars[i - lb..=i].to_vec();
                        let want = forward(&full, d);
                        let got = reverse(&bars, i, lb, 1, d, (bars[i], 0));
                        assert_eq!(
                            (want.0.to_bits(), want.1),
                            (got.0.to_bits(), got.1),
                            "{shape} {d:?} lb={lb} bar={i}: unseeded reverse != forward"
                        );
                        // SEEDED: the narrow roll's (value, trail) comes from ITS OWN
                        // forward walk over `[i − lb_n ..= i]`, as the engine carries it.
                        for lb_n in 0..lb {
                            let narrow = forward(&bars[i - lb_n..=i], d);
                            let got = reverse(&bars, i, lb, lb_n + 1, d, narrow);
                            assert_eq!(
                                (want.0.to_bits(), want.1),
                                (got.0.to_bits(), got.1),
                                "{shape} {d:?} lb={lb} lb_n={lb_n} bar={i}: seeded reverse != full walk"
                            );
                        }
                    }
                }
            }
        }
    }

    /// E9 seed-link PRE-FLIGHT (before any codegen depends on it): a wide roll
    /// may rescan by walking ONLY the gap beyond a narrower live roll of the
    /// same direction on the same stream, seeded with that roll's current
    /// `(value, trail)`. Claim being proved: this yields the SAME `(value,
    /// trail)` on every bar as walking the wide window in full.
    ///
    /// The tie rule is the whole risk: `trail` counts bars since the NEWEST
    /// occurrence, so an equal value in the gap must NOT steal the win (the
    /// newest occurrence is inside the narrow window) — hence strict `>`
    /// against the seed, and the seed's `trail` carried through unchanged when
    /// the gap fails to strictly beat it. Swept over every narrow/wide pair
    /// with `lb_n < lb_w`, both guard conventions (`period = lb+1` canonical and
    /// `period = lb` like aroon), and shapes built to put equal values in the
    /// gap, the extremum solely in the gap, and plateaus spanning both.
    #[test]
    fn gap_seeded_rescan_matches_full_walk() {
        // One bar of the recurrence: `Some((value, trail))` from a walk, else
        // the compare-and-reset branch. `full` walks [i-lb ..= i]; `seeded`
        // walks only the gap [i-lb_w .. i-lb_n-1] against the seed.
        fn step(
            src: &[f64],
            i: usize,
            period: usize,
            lb: usize,
            want_max: bool,
            prev: (f64, usize),
            seed: Option<((f64, usize), usize)>,
        ) -> (f64, usize) {
            let (v, mut t) = prev;
            t += 1;
            if period > t {
                // no rescan due: the else-branch only
                let x = src[i];
                let hit = if want_max { x >= v } else { x <= v };
                if hit {
                    (x, 0)
                } else {
                    (v, t)
                }
            } else {
                let (wv, wi) = match seed {
                    None => {
                        // FULL walk of the wide window
                        let mut bv = src[i - lb];
                        let mut bi = 0usize;
                        for k in 1..=lb {
                            let x = src[i - lb + k];
                            let take = if want_max { x >= bv } else { x <= bv };
                            if take {
                                bv = x;
                                bi = k;
                            }
                        }
                        (bv, bi)
                    }
                    Some(((sv, st), lb_n)) => {
                        // GAP-only walk, seeded by the narrow roll. Indexes are
                        // positions within the WIDE window, so the seed's own
                        // index is `lb − its trail`.
                        let gap = lb - lb_n;
                        let mut bv = sv;
                        let mut best_j = None;
                        for j in 0..gap {
                            let x = src[i - lb + j];
                            let take = if want_max { x > bv } else { x < bv };
                            if take {
                                bv = x;
                                best_j = Some(j);
                            }
                        }
                        match best_j {
                            Some(j) => (bv, j),
                            // no strict winner ⇒ the newest occurrence is the
                            // seed's own bar ⇒ same value AND same trail
                            None => (sv, lb - st),
                        }
                    }
                };
                (wv, lb - wi)
            }
        }

        fn run(
            src: &[f64],
            period: usize,
            lb: usize,
            want_max: bool,
            narrow: Option<(&[(f64, usize)], usize)>,
        ) -> Vec<(f64, usize)> {
            // bar 0 seeds exactly as the hand's `init_state` does
            let mut out = Vec::with_capacity(src.len());
            let mut state = (src[0], 0usize);
            out.push(state);
            for i in 1..src.len() {
                let seed = narrow.map(|(states, lb_n)| (states[i], lb_n));
                state = step(src, i, period, lb, want_max, state, seed);
                out.push(state);
            }
            out
        }

        let mut shapes: Vec<Vec<f64>> = Vec::new();
        let mut s = Vec::with_capacity(400);
        let mut rng = 0x2545_F491_4F6C_DD1Du64;
        for _ in 0..400 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            s.push(((rng % 1000) as f64 - 500.0) / 7.0);
        }
        shapes.push(s);
        shapes.push((0..400).map(|i| i as f64).collect()); // strictly rising
        shapes.push((0..400).map(|i| -(i as f64)).collect()); // falling
        shapes.push(vec![3.5; 400]); // all equal: every compare is a tie
        let mut plateau = vec![1.0; 400];
        for i in (0..400).step_by(37) {
            plateau[i.min(399)] = 9.0; // isolated peaks, ties nowhere
        }
        shapes.push(plateau.clone());
        let mut both = plateau;
        for i in 12..400 {
            if both[i - 12] == 9.0 {
                both[i] = 9.0; // a peak replicated into the gap at distance 12
            }
        }
        shapes.push(both);
        shapes.push((0..400).map(|i| (i % 5) as f64).collect()); // dense ties

        // The model must agree with the real oracle first, or the equivalence
        // below would only prove two simulations share the same mistake.
        use crate::indicator_types::Indicator;
        use crate::indicators::{max::Max, min::Min};
        for src in &shapes[..3] {
            let p = 14usize;
            let n = src.len();
            let hand = Max::indicator(&[src.as_slice()], &[p as f64], None)
                .unwrap()
                .0;
            let model = run(src, p, p - 1, true, None);
            for j in 0..hand[0].len() {
                assert_eq!(
                    model[p - 1 + j].0.to_bits(),
                    hand[0][j].to_bits(),
                    "model max differs from hand at output {j}"
                );
            }
            let hand_min = Min::indicator(&[src.as_slice()], &[p as f64], None)
                .unwrap()
                .0;
            let model_min = run(src, p, p - 1, false, None);
            for j in 0..hand_min[0].len() {
                assert_eq!(
                    model_min[p - 1 + j].0.to_bits(),
                    hand_min[0][j].to_bits(),
                    "model min differs from hand at output {j}"
                );
            }
            assert_eq!(hand[0].len(), n - p + 1);
        }

        for src in &shapes {
            for lb_n in [1usize, 3, 6, 11, 19] {
                for lb_w in [lb_n + 1, lb_n + 5, lb_n + 17] {
                    for want_max in [true, false] {
                        // canonical guard (period = lb+1) and aroon's early one
                        for per_delta in [1usize, 0] {
                            let full_n = run(src, lb_n + per_delta, lb_n, want_max, None);
                            let full_w = run(src, lb_w + per_delta, lb_w, want_max, None);
                            let seed_w =
                                run(src, lb_w + per_delta, lb_w, want_max, Some((&full_n, lb_n)));
                            for i in 0..src.len() {
                                assert_eq!(
                                    (full_w[i].0.to_bits(), full_w[i].1),
                                    (seed_w[i].0.to_bits(), seed_w[i].1),
                                    "seeded wide walk differs at bar {i} (lb_n={lb_n} lb_w={lb_w} \
                                     max={want_max} period_delta={per_delta})"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// E9 M2: the cross-INDICATOR window sharing the family exists for.
    /// `willr` and `donchian` both build `RollMax{high, p, p−1}` +
    /// `RollMin{low, p, p−1}` — IDENTICAL node triples — so one query advances
    /// ONE rolling max and ONE rolling min for both: 10 + 5 ops become 13, not
    /// 15, and 3 + 3 stores become 4 (donchian's `lower` IS willr's `min`
    /// mirror, its `upper` IS willr's `max` mirror). Counted, not value-checked:
    /// a duplicated roll is invisible to value parity.
    #[test]
    fn jit_willr_and_donchian_share_both_rolls() {
        use crate::indicators::donchianchannel::DonchianChannel;
        use crate::indicators::willr::Willr;
        let period = 14u32;
        let opt = [period as f64];
        let w = Request::new().add_all(Willr::recipe(&opt)).build();
        let d = Request::new()
            .add(DonchianChannel::recipe(&opt), None)
            .build();
        assert_eq!(w.info(), (10, 3), "willr alone: 2 rolls + 7 math/const ops");
        assert_eq!(
            d.info(),
            (5, 3),
            "donchian alone: 2 rolls + add + mul + const"
        );

        let both = Request::new()
            .add_all(Willr::recipe(&opt))
            .add(DonchianChannel::recipe(&opt), None)
            .build();
        // 2 shared rolls instead of 4 → 13 ops; 4 unique value nodes → 4 stores.
        assert_eq!(both.info(), (13, 4), "willr + donchian dedup both rolls");

        let hi = series(500);
        let lo: Vec<f64> = hi.iter().map(|v| v * 0.97 - 2.0).collect();
        let close: Vec<f64> = (0..hi.len())
            .map(|i| lo[i] + (hi[i] - lo[i]) * 0.5)
            .collect();
        let (high, low) = (hi, lo);
        let s: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];
        let (res, st) = both.run(&s);
        let wh = Willr::indicator(&s, &opt, Some(&[true, true])).unwrap().0;
        let dh = DonchianChannel::indicator(&[high.as_slice(), low.as_slice()], &opt, None)
            .unwrap()
            .0;
        cmp_exact(res.request(0).output(0), &wh[0], "willr (shared rolls)");
        cmp_exact(
            res.request(1).output(0),
            &dh[0],
            "donchian lower (== willr min)",
        );
        cmp_exact(
            res.request(1).output(2),
            &dh[2],
            "donchian upper (== willr max)",
        );
        // one arena, six slots: the shared rows literally alias.
        assert!(res.outputs[1][0].shares_data_with(&res.optional[0][0]));
        assert!(res.outputs[1][2].shares_data_with(&res.optional[0][1]));
        // and both restorers really fire (not the `Other` fallback).
        let states = both.get_states(&st);
        let (IndicatorStateBox::Willr(_), IndicatorStateBox::DonchianChannel(_)) =
            (&states[0], &states[1])
        else {
            panic!("M2: willr/donchian restorers must rebuild the hand states, not fall to Other");
        };
    }

    /// E9 M2: the NEGATIVE half of the same rule, which the user's window note
    /// predicts. `aroon` uses `(period, period)` — a P+1-bar window — so at the
    /// SAME period it must NOT intern with willr/donchian's `(period, period−1)`
    /// rolls. If `look_back` were ever derived from `period`, this would silently
    /// collapse four different windows into two and print wrong aroon lines.
    #[test]
    fn jit_aroon_windows_do_not_merge_with_the_p_minus_1_family() {
        use crate::indicators::aroon::Aroon;
        use crate::indicators::willr::Willr;
        let period = 14u32;
        let opt = [period as f64];
        let a = Request::new().add(Aroon::recipe(&opt), None).build();
        let w = Request::new().add_all(Willr::recipe(&opt)).build();
        let both = Request::new()
            .add(Aroon::recipe(&opt), None)
            .add_all(Willr::recipe(&opt))
            .build();
        // aroon 10 + willr 10 = 20: NO roll is shared across the two widths.
        assert_eq!(a.info(), (10, 2));
        assert_eq!(w.info(), (10, 3));
        assert_eq!(
            both.info(),
            (20, 5),
            "aroon(p,p) and willr(p,p−1) must keep four separate rolls"
        );
    }

    /// E6 field axis: MEDPRICE = 0.5·(high+low), the engine's first
    /// MULTI-INPUT indicator (`Input{stream:1}` = low, non-zero stream the
    /// classic path asserted away). Stateless, publishes every bar (lead 0)
    /// — so it also proves a chain-free plan's bar 0 is emitted, not
    /// swallowed by the Ema cold-seed skip. Bit-exact vs hand + streaming.
    #[test]
    fn jit_medprice_multi_input_matches_hand() {
        use crate::indicators::medprice::Medprice;
        let high = series(300);
        let low: Vec<f64> = high.iter().map(|v| v * 0.98 - 1.0).collect();
        let s: [&[f64]; 2] = [high.as_slice(), low.as_slice()];
        let plan = Request::new().add_all(Medprice::recipe(&[])).build();
        assert_eq!(plan.n_fields, 2, "medprice recipe must declare 2 fields");
        let (res, _) = plan.run(&s);
        let h = Medprice::indicator(&s, &[], None).unwrap().0;
        cmp_exact(res.request(0).output(0), &h[0], "medprice");
        assert_eq!(
            res.request(0).output(0).len(),
            high.len(),
            "every bar publishes"
        );

        // streaming: the field-major [high|low] windows stitch per-stream
        for split in [1usize, 2, 150, 299] {
            let s1: [&[f64]; 2] = [&high[..split], &low[..split]];
            let s2: [&[f64]; 2] = [&high[split..], &low[split..]];
            let (h1, qs) = plan.run(&s1);
            let (h2, _) = plan.resume(qs, &s2);
            let mut c = h1.outputs[0][0].to_vec();
            c.extend_from_slice(&h2.outputs[0][0]);
            cmp_exact(&c, &h[0], &format!("medprice split{split}"));
        }
    }

    /// E6 multi-input + Abs + Lag: TRUE RANGE over (high, low, close) =
    /// `max(h−l, |h−prevClose|, |l−prevClose|)`, the second multi-input
    /// indicator, proving `Node::Abs`/`Node::Lag` and the anchor prev-close
    /// read. Bit-exact vs hand + streaming across the lead-1 boundary. TR
    /// is the leaf `Atr`/`Natr`/`Adx` all reuse, so this is the family's
    /// foundation.
    #[test]
    fn jit_tr_multi_input_matches_hand() {
        use crate::indicators::tr::Tr;
        let high = series(400);
        let low: Vec<f64> = high.iter().map(|v| v * 0.97 - 2.0).collect();
        let close: Vec<f64> = (0..400)
            .map(|i| low[i] + (high[i] - low[i]) * 0.5)
            .collect();
        let s: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];
        let plan = Request::new().add_all(Tr::recipe(&[])).build();
        assert_eq!(plan.n_fields, 3, "tr declares high/low/close");
        let (res, _) = plan.run(&s);
        let h = Tr::indicator(&s, &[], None).unwrap().0;
        cmp_exact(res.request(0).output(0), &h[0], "tr");
        assert_eq!(
            res.request(0).output(0).len(),
            high.len() - 1,
            "tr publishes from bar 1"
        );

        // streaming: prevClose is the one cross-chunk value, read off the
        // per-stream stitched window — every split must stay bit-exact.
        for split in [2usize, 3, 4, 200, 399] {
            let s1: [&[f64]; 3] = [&high[..split], &low[..split], &close[..split]];
            let s2: [&[f64]; 3] = [&high[split..], &low[split..], &close[split..]];
            let (h1, qs) = plan.run(&s1);
            let (h2, _) = plan.resume(qs, &s2);
            let mut c = h1.outputs[0][0].to_vec();
            c.extend_from_slice(&h2.outputs[0][0]);
            cmp_exact(&c, &h[0], &format!("tr split{split}"));
        }
    }

    /// E6 chained-Wilders: `ATR = Wilders(tr, p)` over the SAME `tr`
    /// subgraph. Bit-exact vs hand + streaming across the publish-lead
    /// boundary. Proves (a) the `Lag` clamp yields `tr[0] = high[0] − low[0]`
    /// (hand `init_state`'s bar-0 seed term), (b) a chained `Wilders` folds
    /// the tr node-value from bar 0 with cold chain value `0.0`, and (c) under
    /// CSE a `tr`+`atr` plan rolls true range once yet stays bit-exact.
    #[test]
    fn jit_atr_matches_hand() {
        use crate::indicators::atr::Atr;
        use crate::indicators::tr::Tr;
        let high = series(400);
        let low: Vec<f64> = high.iter().map(|v| v * 0.97 - 2.0).collect();
        let close: Vec<f64> = (0..400)
            .map(|i| low[i] + (high[i] - low[i]) * 0.5)
            .collect();
        let s: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];
        for period in [7usize, 14, 20] {
            let plan = Request::new()
                .add(Atr::recipe(&[period as f64]), None)
                .build();
            let (res, _) = plan.run(&s);
            let h = Atr::indicator(&s, &[period as f64], None).unwrap().0;
            cmp_exact(res.request(0).output(0), &h[0], &format!("atr{period}"));
            assert_eq!(
                res.request(0).output(0).len(),
                high.len() - period,
                "atr publishes from bar {period}"
            );

            // streaming: tr's prevClose rides the stitched window, atr's
            // decayed avg the chain slot — every split stays bit-exact.
            for split in [period, period + 1, period + 2, 200, 399] {
                let s1: [&[f64]; 3] = [&high[..split], &low[..split], &close[..split]];
                let s2: [&[f64]; 3] = [&high[split..], &low[split..], &close[split..]];
                let (h1, qs) = plan.run(&s1);
                let (h2, _) = plan.resume(qs, &s2);
                let mut c = h1.outputs[0][0].to_vec();
                c.extend_from_slice(&h2.outputs[0][0]);
                cmp_exact(&c, &h[0], &format!("atr{period} split{split}"));
            }
        }

        // CSE sharing: `tr` + `atr(14)` in one plan roll true range once —
        // both outputs stay bit-exact (the shared tr nodes compute in SSA
        // order before atr's chained Wilders scalarizes them).
        let combo = Request::new()
            .add_all(Tr::recipe(&[]))
            .add_all(Atr::recipe(&[14.0]))
            .build();
        let (cres, _) = combo.run(&s);
        let th = Tr::indicator(&s, &[], None).unwrap().0;
        let ah = Atr::indicator(&s, &[14.0], None).unwrap().0;
        cmp_exact(cres.request(0).output(0), &th[0], "tr (shared)");
        cmp_exact(cres.request(1).output(0), &ah[0], "atr (shared)");
    }

    /// E6: NATR = `atr/close · 100` over the SAME mean-chained `Wilders(tr)`
    /// node — bit-exact + streaming, and a `atr`+`natr` combo proves CSE
    /// collapses them to ONE tr subgraph + ONE atr chain (both outputs exact).
    #[test]
    fn jit_natr_matches_hand() {
        use crate::indicators::atr::Atr;
        use crate::indicators::natr::Natr;
        let high = series(400);
        let low: Vec<f64> = high.iter().map(|v| v * 0.97 - 2.0).collect();
        let close: Vec<f64> = (0..400)
            .map(|i| low[i] + (high[i] - low[i]) * 0.5)
            .collect();
        let s: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];
        for period in [7usize, 14, 20] {
            let plan = Request::new()
                .add(Natr::recipe(&[period as f64]), None)
                .build();
            let (res, _) = plan.run(&s);
            let h = Natr::indicator(&s, &[period as f64], None).unwrap().0;
            cmp_exact(res.request(0).output(0), &h[0], &format!("natr{period}"));
            assert_eq!(
                res.request(0).output(0).len(),
                high.len() - period,
                "natr publishes from bar {period}"
            );
            for split in [period, period + 1, period + 2, 200, 399] {
                let s1: [&[f64]; 3] = [&high[..split], &low[..split], &close[..split]];
                let s2: [&[f64]; 3] = [&high[split..], &low[split..], &close[split..]];
                let (h1, qs) = plan.run(&s1);
                let (h2, _) = plan.resume(qs, &s2);
                let mut c = h1.outputs[0][0].to_vec();
                c.extend_from_slice(&h2.outputs[0][0]);
                cmp_exact(&c, &h[0], &format!("natr{period} split{split}"));
            }
        }

        // natr + atr(14) share the tr subgraph AND the atr Wilders chain
        // (same `(Wilders(tr), 14)` node) — one fused loop, both exact.
        let combo = Request::new()
            .add_all(Atr::recipe(&[14.0]))
            .add_all(Natr::recipe(&[14.0]))
            .build();
        let (cres, _) = combo.run(&s);
        let ah = Atr::indicator(&s, &[14.0], None).unwrap().0;
        let nh = Natr::indicator(&s, &[14.0], None).unwrap().0;
        cmp_exact(cres.request(0).output(0), &ah[0], "atr (shared w/ natr)");
        cmp_exact(cres.request(1).output(0), &nh[0], "natr (shared w/ atr)");
    }

    /// E6: DM = composite (sum) `Wilders` over the `+DM`/`−DM` mutual
    /// exclusion (`WhereGe` + `Max`), `INPUTS=2`. Bit-exact on BOTH outputs +
    /// streaming across the lead boundary. Proves the `WhereGe` select never
    /// emits `-0.0`/NaN (the hand's literal-`0.0` arms) and the composite
    /// fold-from-bar-0 (`up[0]=down[0]=0`) matches the hand's `skip(1)` warmup.
    #[test]
    fn jit_dm_matches_hand() {
        use crate::indicators::dm::Dm;
        let high = series(400);
        let low: Vec<f64> = high.iter().map(|v| v * 0.98 - 1.5).collect();
        let s: [&[f64]; 2] = [high.as_slice(), low.as_slice()];
        for period in [7usize, 14, 20] {
            let plan = Request::new().add_all(Dm::recipe(&[period as f64])).build();
            let (res, _) = plan.run(&s);
            let h = Dm::indicator(&s, &[period as f64], None).unwrap().0;
            cmp_exact(res.request(0).output(0), &h[0], &format!("dm+{period}"));
            cmp_exact(res.request(0).output(1), &h[1], &format!("dm−{period}"));
            assert_eq!(
                res.request(0).output(0).len(),
                high.len() - period,
                "dm publishes from bar {period}"
            );
            for split in [period, period + 1, period + 2, 200, 399] {
                let s1: [&[f64]; 2] = [&high[..split], &low[..split]];
                let s2: [&[f64]; 2] = [&high[split..], &low[split..]];
                let (h1, qs) = plan.run(&s1);
                let (h2, _) = plan.resume(qs, &s2);
                for k in 0..2 {
                    let mut c = h1.outputs[0][k].to_vec();
                    c.extend_from_slice(&h2.outputs[0][k]);
                    cmp_exact(
                        &c,
                        &h[k],
                        &format!("dm{}{period} split{split}", if k == 0 { "+" } else { "−" }),
                    );
                }
            }
        }
    }

    /// E6: DI = `max(±DM · (100/ATR_sum), 0)` over the shared DM chain + a
    /// composite (sum) ATR. Bit-exact on both outputs + streaming, and a
    /// `dm`+`di` combo proves they share the whole `+DM`/`−DM` accumulator.
    #[test]
    fn jit_di_matches_hand() {
        use crate::indicators::di::Di;
        use crate::indicators::dm::Dm;
        let high = series(400);
        let low: Vec<f64> = high.iter().map(|v| v * 0.97 - 2.0).collect();
        let close: Vec<f64> = (0..400)
            .map(|i| low[i] + (high[i] - low[i]) * 0.5)
            .collect();
        let s: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];
        for period in [7usize, 14, 20] {
            let plan = Request::new().add_all(Di::recipe(&[period as f64])).build();
            let (res, _) = plan.run(&s);
            let h = Di::indicator(&s, &[period as f64], None).unwrap().0;
            cmp_exact(res.request(0).output(0), &h[0], &format!("di+{period}"));
            cmp_exact(res.request(0).output(1), &h[1], &format!("di−{period}"));
            assert_eq!(
                res.request(0).output(0).len(),
                high.len() - period,
                "di publishes from bar {period}"
            );
            for split in [period, period + 1, period + 2, 200, 399] {
                let s1: [&[f64]; 3] = [&high[..split], &low[..split], &close[..split]];
                let s2: [&[f64]; 3] = [&high[split..], &low[split..], &close[split..]];
                let (h1, qs) = plan.run(&s1);
                let (h2, _) = plan.resume(qs, &s2);
                for k in 0..2 {
                    let mut c = h1.outputs[0][k].to_vec();
                    c.extend_from_slice(&h2.outputs[0][k]);
                    cmp_exact(&c, &h[k], &format!("di{k}{period} split{split}"));
                }
            }
        }

        // dm + di(14) share the whole +DM/−DM accumulator subgraph (both use
        // the composite `Wilders(dp/dm)` nodes) — one fused loop, all exact.
        let combo = Request::new()
            .add_all(Dm::recipe(&[14.0]))
            .add_all(Di::recipe(&[14.0]))
            .build();
        let (cres, _) = combo.run(&s);
        let dh = Dm::indicator(&[high.as_slice(), low.as_slice()], &[14.0], None)
            .unwrap()
            .0;
        let ih = Di::indicator(&s, &[14.0], None).unwrap().0;
        cmp_exact(cres.request(0).output(0), &dh[0], "dm+ (shared w/ di)");
        cmp_exact(cres.request(0).output(1), &dh[1], "dm− (shared w/ di)");
        cmp_exact(cres.request(1).output(0), &ih[0], "di+ (shared w/ dm)");
        cmp_exact(cres.request(1).output(1), &ih[1], "di− (shared w/ dm)");
    }

    /// E6: DX = `(|DM+−DM−|·100/(DM++DM−)).max(0)` — a pure function of the
    /// shared composite accumulators (`n_fields=2`). Bit-exact + streaming,
    /// and a `dx`+`di` combo (3 fields) shows them roll directional movement
    /// once (identical `dmup`/`dmdown` nodes).
    #[test]
    fn jit_dx_matches_hand() {
        use crate::indicators::di::Di;
        use crate::indicators::dx::Dx;
        let high = series(400);
        let low: Vec<f64> = high.iter().map(|v| v * 0.97 - 2.0).collect();
        let close: Vec<f64> = (0..400)
            .map(|i| low[i] + (high[i] - low[i]) * 0.5)
            .collect();
        let two: [&[f64]; 2] = [high.as_slice(), low.as_slice()];
        let three: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];
        for period in [7usize, 14, 20] {
            let plan = Request::new().add_all(Dx::recipe(&[period as f64])).build();
            assert_eq!(plan.n_fields, 2, "dx value needs only high/low");
            let (res, _) = plan.run(&two);
            let h = Dx::indicator(&three, &[period as f64], None).unwrap().0;
            cmp_exact(res.request(0).output(0), &h[0], &format!("dx{period}"));
            assert_eq!(
                res.request(0).output(0).len(),
                high.len() - period,
                "dx publishes from bar {period}"
            );
            for split in [period, period + 1, period + 2, 200, 399] {
                let s1: [&[f64]; 2] = [&high[..split], &low[..split]];
                let s2: [&[f64]; 2] = [&high[split..], &low[split..]];
                let (h1, qs) = plan.run(&s1);
                let (h2, _) = plan.resume(qs, &s2);
                let mut c = h1.outputs[0][0].to_vec();
                c.extend_from_slice(&h2.outputs[0][0]);
                cmp_exact(&c, &h[0], &format!("dx{period} split{split}"));
            }
        }

        // dx + di(14): both consume the SAME dmup/dmdown nodes (merged), so
        // directional movement is rolled once across the two requests.
        let combo = Request::new()
            .add_all(Dx::recipe(&[14.0]))
            .add_all(Di::recipe(&[14.0]))
            .build();
        let (cres, _) = combo.run(&three);
        let xh = Dx::indicator(&three, &[14.0], None).unwrap().0;
        let ih = Di::indicator(&three, &[14.0], None).unwrap().0;
        cmp_exact(cres.request(0).output(0), &xh[0], "dx (shared w/ di)");
        cmp_exact(cres.request(1).output(0), &ih[0], "di+ (shared w/ dx)");
        cmp_exact(cres.request(1).output(1), &ih[1], "di− (shared w/ dx)");
    }

    /// §2.1 / D6 — degenerate (flat-bar) data. When `h==l==c` the composite
    /// ATR **sum** is `0`, so `di` evaluates `0 · (100/0) = 0·inf = NaN` and
    /// `dx` evaluates `0/0 = NaN`; both then hit `.max(0.0)`. Rust `f64::max`
    /// returns the *other* operand (`0.0`), while CLIF `fmax` propagates NaN —
    /// so the JIT emitted `NaN` where the hand emits `0.0`. The smooth
    /// `series()` suite is structurally blind to this (it never goes flat).
    #[test]
    fn jit_flat_bar_degenerate_matches_hand() {
        use crate::indicators::di::Di;
        use crate::indicators::dx::Dx;
        let n = 80;
        let flat: Vec<f64> = (0..n).map(|_| 100.0).collect();
        let three: [&[f64]; 3] = [flat.as_slice(), flat.as_slice(), flat.as_slice()];
        let two: [&[f64]; 2] = [flat.as_slice(), flat.as_slice()];
        for period in [7usize, 14] {
            // di = max(±DM · (100/ATR_sum), 0): every bar `0·inf → NaN → 0`.
            let plan = Request::new().add_all(Di::recipe(&[period as f64])).build();
            let (res, _) = plan.run(&three);
            let h = Di::indicator(&three, &[period as f64], None).unwrap().0;
            assert!(
                h[0].iter().all(|v| *v == 0.0),
                "hand di+ on flat data must be all 0.0 (Rust NaN→other)"
            );
            cmp_exact(
                res.request(0).output(0),
                &h[0],
                &format!("di+ flat {period}"),
            );
            cmp_exact(
                res.request(0).output(1),
                &h[1],
                &format!("di− flat {period}"),
            );

            // dx = (|ΔDM|·100 / ΣDM).max(0): `0/0 → NaN → 0`.
            let plan = Request::new().add_all(Dx::recipe(&[period as f64])).build();
            let (res, _) = plan.run(&two);
            let h = Dx::indicator(&three, &[period as f64], None).unwrap().0;
            assert!(
                h[0].iter().all(|v| *v == 0.0),
                "hand dx on flat data must be all 0.0 (Rust NaN→other)"
            );
            cmp_exact(
                res.request(0).output(0),
                &h[0],
                &format!("dx flat {period}"),
            );
        }
    }

    /// §2.1 / D6 — `stddev`'s `.max(EPSILON)` clamp under **catastrophic
    /// cancellation**, the third `fmax`→`max_num` site (the sqrt clamp is a raw
    /// `fb.ins().fmax`, not `ArithKind::Max`). The hand computes variance as
    /// `sum_sq·(1/P) − sma²`; for large-magnitude, near-constant POSITIVE prices
    /// the two rounded terms can leave the result slightly NEGATIVE (not a
    /// negative price — a pure float artifact of the naive formula), so
    /// `sqrt(<0) = NaN`. Rust `NaN.max(EPSILON)` yields `EPSILON`; CLIF `fmax`
    /// yields `NaN`. Verified: this construction makes 1320/1980 rows clamp to
    /// EPSILON in the hand, and the old `fmax` emitted `NaN` there.
    #[test]
    fn jit_stddev_negative_variance_clamps_like_hand() {
        use crate::indicators::stddev::StdDev;
        // 1e8 + {0,1,2} jitter: positive prices, true variance ≈ 0.67 vs a
        // ~1e16 mean² — the rolling subtraction cancels to sign-flipping noise.
        let n = 2000;
        let real: Vec<f64> = (0..n).map(|k| 1e8 + (k % 3) as f64).collect();
        let s: [&[f64]; 1] = [real.as_slice()];
        let period = 20.0;
        let plan = Request::new().add_all(StdDev::recipe(&[period])).build();
        let (res, _) = plan.run(&s);
        let h = StdDev::indicator(&s, &[period], None).unwrap().0;
        let jit = res.request(0).output(0);
        cmp_exact(jit, &h[0], "stddev negative-variance clamp");
        // The clamp must ACTUALLY fire — otherwise the test proves nothing.
        let clamped = h[0].iter().filter(|v| **v == f64::EPSILON).count();
        assert!(
            clamped > 0,
            "expected ≥1 EPSILON-clamped (negative-variance) row; the data no longer trips cancellation"
        );
        assert!(
            jit.iter().all(|v| !v.is_nan()),
            "JIT must never emit NaN here (old fmax did)"
        );
    }

    /// DCE (review §2.2 / D3): a recipe whose disabled optional mirrors pulled
    /// in an entire sub-chain must stop paying for it. `Adx`'s `[dx, atr, tr]`
    /// mirrors graft the **mean** `Wilders(tr)` ATR node; under `add(_, None)`
    /// that mean chain is dead (adx's primary folds the *composite* sum-ATR and
    /// only `dx`/`tr` stay reachable), so it is pruned — strictly fewer ops, and
    /// the primary output is unchanged (bit-identical to hand). This is the
    /// register-pressure relief DCE exists to give, asserted structurally.
    #[test]
    fn jit_dce_prunes_disabled_optional_chain() {
        use crate::indicators::adx::Adx;
        let high = series(300);
        let low: Vec<f64> = high.iter().map(|v| v * 0.97 - 2.0).collect();
        let close: Vec<f64> = (0..300)
            .map(|i| low[i] + (high[i] - low[i]) * 0.5)
            .collect();
        let s: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];
        let primary = Request::new().add(Adx::recipe(&[14.0]), None).build();
        let all = Request::new().add_all(Adx::recipe(&[14.0])).build();
        let (p_ops, p_stores) = primary.info();
        let (a_ops, a_stores) = all.info();
        assert!(
            p_ops < a_ops,
            "primary-only must prune the dead mean-ATR chain: {p_ops} ops !< {a_ops}"
        );
        assert!(
            p_stores < a_stores,
            "primary-only writes fewer rows: {p_stores} !< {a_stores}"
        );
        // Pruning is output-preserving: the surviving primary matches hand.
        let (res, _) = primary.run(&s);
        let h = Adx::indicator(&s, &[14.0], None).unwrap().0;
        cmp_exact(res.request(0).output(0), &h[0], "adx primary (DCE)");
    }

    /// Streaming surface (§3.2 / §1.4): tick bar-by-bar must be bit-identical
    /// to a cold `run` — both the OUTPUT VALUES (cold == resumed == tick) and
    /// the carried STATE. Covers a `pmax == 0` plan (classic: no stitch) and a
    /// rolling `Sma` (pmax > 0: the `tick` path drives the host anchor/stitch).
    /// Disabled optionals report `NaN` every bar; warmup bars report `NaN`
    /// before their lead — stripping leading `NaN`s aligns each slot with its
    /// cold row (which begins AT its lead).
    #[test]
    fn jit_tick_matches_cold_run() {
        use crate::indicators::sma::Sma;

        // Flatten one asset's Results into slot order: per request, all
        // primaries then all optionals (the `advance` slot order, asset==1).
        fn slot_rows(res: &Results) -> Vec<Vec<f64>> {
            let mut v = Vec::new();
            for r in 0..res.outputs.len() {
                for row in res.outputs[r].iter() {
                    v.push(row.to_vec());
                }
                for row in res.optional[r].iter() {
                    v.push(row.to_vec());
                }
            }
            v
        }
        fn strip_leading_nan(g: &[f64]) -> &[f64] {
            let s = g.iter().position(|x| !x.is_nan()).unwrap_or(g.len());
            &g[s..]
        }

        let close = series(400);
        for (label, plan) in [
            ("classic", classic().build()),
            (
                "sma14",
                Request::new().add_all(Sma::recipe(&[14.0])).build(),
            ),
        ] {
            let n = plan.n_slots();
            assert!(n <= 32, "{label}: widen the test out buffer");
            let (res, st_full) = plan.run(&[close.as_slice()]);
            let expected = slot_rows(&res);
            assert_eq!(expected.len(), n, "{label}: slot count");

            let mut st = plan.new_state();
            let mut got: Vec<Vec<f64>> = vec![Vec::new(); n];
            let mut out = [0.0f64; 32];
            for i in 0..close.len() {
                let v = close[i];
                let bar = [std::slice::from_ref(&v)];
                plan.tick(&mut st, &bar, &mut out[..n]);
                for s in 0..n {
                    got[s].push(out[s]);
                }
            }
            // values: each slot's published stream == its cold row.
            for s in 0..n {
                assert_eq!(
                    strip_leading_nan(&got[s]),
                    expected[s].as_slice(),
                    "{label}: slot {s} tick vs cold"
                );
            }
            // state: ticking every bar lands on the same carried words as a
            // cold whole-chunk run (this is the cold==resumed invariant).
            assert_eq!(st.words, st_full.words, "{label}: tick state != run");
        }
    }

    /// E9: `tick` on the trail-rescan roll — the 1-bar streaming path is where
    /// the anchor contract bites HARDEST for this kind, because a rescan reads
    /// up to `look_back` bars back while every tick re-stitches
    /// `window ++ chunk`. Ticks whole series bar-by-bar and asserts each
    /// published slot equals its cold row (leading `NaN`s stripped) AND that the
    /// carried words — the extremum AND the `trail` — land exactly on the cold
    /// run's. Run on a tie-heavy series too: ties are what move `trail`, and
    /// `trail` is what decides whether a tick rescans.
    #[test]
    fn jit_roll_tick_matches_cold_run() {
        use crate::indicators::{max::Max, min::Min};

        fn slot_rows(res: &Results) -> Vec<Vec<f64>> {
            let mut v = Vec::new();
            for r in 0..res.outputs.len() {
                for row in res.outputs[r].iter() {
                    v.push(row.to_vec());
                }
                for row in res.optional[r].iter() {
                    v.push(row.to_vec());
                }
            }
            v
        }
        fn strip_leading_nan(g: &[f64]) -> &[f64] {
            let s = g.iter().position(|x| !x.is_nan()).unwrap_or(g.len());
            &g[s..]
        }

        let period = 14u32;
        let datasets = vec![
            ("wiggle", series(400)),
            (
                "ties",
                (0..400)
                    .map(|i| ((i * 7) % 11) as f64)
                    .collect::<Vec<f64>>(),
            ),
            ("rising", (0..400).map(|i| i as f64).collect::<Vec<f64>>()),
        ];
        for want_max in [true, false] {
            let which = if want_max { "max" } else { "min" };
            for (shape, data) in &datasets {
                let opt = [period as f64];
                let recipe = if want_max {
                    Max::recipe(&opt)
                } else {
                    Min::recipe(&opt)
                };
                let plan = Request::new().add(recipe, None).build();
                let n = plan.n_slots();
                assert!(n <= 32, "{which} {shape}: widen the test out buffer");
                let (res, st_full) = plan.run(&[data.as_slice()]);
                let expected = slot_rows(&res);

                let mut st = plan.new_state();
                let mut got: Vec<Vec<f64>> = vec![Vec::new(); n];
                let mut out = [0.0f64; 32];
                for v in data {
                    let bar = [std::slice::from_ref(v)];
                    plan.tick(&mut st, &bar, &mut out[..n]);
                    for s in 0..n {
                        got[s].push(out[s]);
                    }
                }
                for s in 0..n {
                    assert_eq!(
                        strip_leading_nan(&got[s]),
                        expected[s].as_slice(),
                        "{which} {shape}: slot {s} tick vs cold"
                    );
                }
                assert_eq!(
                    st.words, st_full.words,
                    "{which} {shape}: ticked state (value + trail) != cold run"
                );
                assert_eq!(
                    st.window, st_full.window,
                    "{which} {shape}: ticked anchor window != cold run"
                );
            }
        }
    }

    /// E9 M1 data pairs `(high, low)`. `rising` makes the rolling MIN always
    /// stale (rescan nearly every bar) while the MAX is always fresh; `ties`
    /// puts the newest-occurrence rule under pressure; `flat` pins both trails
    /// at their maximum. These are the shapes that decide `trail` — which is
    /// exactly what aroon PRINTS, so a wrong tie index is a wrong row.
    fn aroon_datasets() -> Vec<(&'static str, Vec<f64>, Vec<f64>)> {
        let wiggle = series(400);
        let wlow: Vec<f64> = wiggle.iter().map(|v| v * 0.97 - 2.0).collect();
        vec![
            ("wiggle", wiggle.clone(), wlow),
            (
                "rising",
                (0..300).map(|i| i as f64).collect(),
                (0..300).map(|i| i as f64 - 1.0).collect(),
            ),
            (
                "ties",
                (0..300).map(|i| 10.0 + ((i * 7) % 11) as f64).collect(),
                (0..300).map(|i| ((i * 5) % 13) as f64).collect(),
            ),
            ("flat", vec![10.0; 300], vec![4.0; 300]),
        ]
    }

    /// E9 M1: AROON — the trail-scaled lines over a `RollMax` of high and a
    /// `RollMin` of low, with the hand's WIDER window (`look_back = period`, so
    /// P+1 bars) and publish lead `period`. Bit-exact on BOTH rows, every shape
    /// × period, cold and streamed (splits around the lead and the guard seam).
    #[test]
    fn jit_aroon_matches_hand() {
        use crate::indicators::aroon::Aroon;
        for period in [7usize, 14, 25] {
            let opt = [period as f64];
            let plan = Request::new().add(Aroon::recipe(&opt), None).build();
            for (shape, high, low) in aroon_datasets() {
                let n = high.len();
                let s: [&[f64]; 2] = [high.as_slice(), low.as_slice()];
                let hand = Aroon::indicator(&s, &opt, None).unwrap().0;
                assert_eq!(
                    hand[0].len(),
                    n - period,
                    "aroon publishes from bar {period}"
                );
                let (res, _) = plan.run(&s);
                cmp_exact(
                    res.request(0).output(0),
                    &hand[0],
                    &format!("aroon down {shape} p{period}"),
                );
                cmp_exact(
                    res.request(0).output(1),
                    &hand[1],
                    &format!("aroon up {shape} p{period}"),
                );

                for split in [period - 1, period, period + 1, n / 3] {
                    let c: [&[f64]; 2] = [&high[..split], &low[..split]];
                    let t: [&[f64]; 2] = [&high[split..], &low[split..]];
                    let (r1, qs) = plan.run(&c);
                    let (r2, _) = plan.resume(qs, &t);
                    for (row, want) in [(0usize, &hand[0]), (1, &hand[1])] {
                        let mut cat = r1.request(0).output(row).to_vec();
                        cat.extend_from_slice(r2.request(0).output(row));
                        cmp_exact(
                            &cat,
                            want,
                            &format!("aroon {shape} p{period} split{split} row{row}"),
                        );
                    }
                }
            }
        }
    }

    /// E9 M1: the fusion showcase. `aroonosc` = `up − down` over THE SAME graph,
    /// so co-requesting them must intern everything: the two rolls, both trail
    /// projections, both constants, both scaled lines — leaving exactly ONE new
    /// op (the `fsub`) and ONE new write stream. Value parity CANNOT catch a
    /// regressed duplicate, so this counts ops.
    #[test]
    fn jit_aroonosc_interns_the_whole_aroon_graph() {
        use crate::indicators::aroon::Aroon;
        use crate::indicators::aroonosc::AroonOsc;
        let (_, high, low) = aroon_datasets()[0].clone();
        let s: [&[f64]; 2] = [high.as_slice(), low.as_slice()];
        let period = 14usize;
        let opt = [period as f64];

        let alone = Request::new().add(Aroon::recipe(&opt), None).build();
        // rolls 2 + trails 2 + consts 2 + subs 2 + muls 2 = 10, two stores.
        assert_eq!(alone.info(), (10, 2), "aroon alone");

        let both = Request::new()
            .add(Aroon::recipe(&opt), None)
            .add_all(AroonOsc::recipe(&opt))
            .build();
        // ONE extra op: the oscillator's `fsub`. Its `aroon_down`/`aroon_up`
        // mirrors are the SAME nodes, so three write streams serve four slots.
        assert_eq!(both.info(), (11, 3), "aroon + aroonosc share the graph");

        let (res, _) = both.run(&s);
        let ah = Aroon::indicator(&s, &opt, None).unwrap().0;
        let oh = AroonOsc::indicator(&s, &opt, Some(&[true, true]))
            .unwrap()
            .0;
        cmp_exact(res.request(0).output(0), &ah[0], "aroon down (w/ osc)");
        cmp_exact(res.request(0).output(1), &ah[1], "aroon up (w/ osc)");
        cmp_exact(res.request(1).output(0), &oh[0], "aroonosc");
        // The osc's two mirrors are DECLARED OPTIONALS, so they live in
        // `Results::optional` (hand order: down, up) — and because they are the
        // SAME arena nodes as aroon's own rows, they alias one write stream.
        cmp_exact(
            res.optional[1][0].as_slice(),
            &oh[1],
            "osc's aroon_down mirror",
        );
        cmp_exact(
            res.optional[1][1].as_slice(),
            &oh[2],
            "osc's aroon_up mirror",
        );
        assert!(res.outputs[0][0].shares_data_with(&res.optional[1][0]));
        assert!(res.outputs[0][1].shares_data_with(&res.optional[1][1]));

        // primary-only (optionals off) still matches the hand's `None` call.
        let plain = Request::new()
            .add(Aroon::recipe(&opt), None)
            .add(AroonOsc::recipe(&opt), None)
            .build();
        let (pres, _) = plain.run(&s);
        let ph = AroonOsc::indicator(&s, &opt, None).unwrap().0;
        cmp_exact(pres.request(1).output(0), &ph[0], "aroonosc primary-only");
        assert_eq!(
            pres.optional[1][0].len(),
            0,
            "disabled optionals print empty rows"
        );

        // streamed across the lead + guard seam
        for split in [period, period + 1, 200] {
            let c: [&[f64]; 2] = [&high[..split], &low[..split]];
            let t: [&[f64]; 2] = [&high[split..], &low[split..]];
            let (r1, qs) = both.run(&c);
            let (r2, _) = both.resume(qs, &t);
            let mut dn = r1.request(0).output(0).to_vec();
            dn.extend_from_slice(r2.request(0).output(0));
            cmp_exact(&dn, &ah[0], &format!("aroon+osc split{split} down"));
            let mut up = r1.request(0).output(1).to_vec();
            up.extend_from_slice(r2.request(0).output(1));
            cmp_exact(&up, &ah[1], &format!("aroon+osc split{split} up"));
            let mut oc = r1.request(1).output(0).to_vec();
            oc.extend_from_slice(r2.request(1).output(0));
            cmp_exact(&oc, &oh[0], &format!("aroon+osc split{split} osc"));
            let mut m0 = r1.optional[1][0].to_vec();
            m0.extend_from_slice(r2.optional[1][0].as_slice());
            cmp_exact(&m0, &oh[1], &format!("aroon+osc split{split} mirror down"));
            let mut m1 = r1.optional[1][1].to_vec();
            m1.extend_from_slice(r2.optional[1][1].as_slice());
            cmp_exact(&m1, &oh[2], &format!("aroon+osc split{split} mirror up"));
        }
    }

    /// E9 M1: `get_states` on aroon rebuilds the hand `{min_state, max_state,
    /// multiplier}` (the multiplier recomputed from the period, never stored);
    /// re-paired with the host anchor window — which for aroon is EXACTLY the
    /// `period` bars of high and low the hand `IndicatorState` keeps — the
    /// hand's `batch_indicator` continues bit-exactly.
    #[test]
    fn jit_get_states_aroon_continues_bit_exact() {
        use crate::indicator_types::TIndicatorState;
        use crate::indicators::aroon::{Aroon, IndicatorState as AroonIS};
        let (_, high, low) = aroon_datasets()[2].clone(); // ties: trails decide
        let period = 14usize;
        let opt = [period as f64];
        let s: [&[f64]; 2] = [high.as_slice(), low.as_slice()];
        let hand = Aroon::indicator(&s, &opt, None).unwrap().0;

        let plan = Request::new().add(Aroon::recipe(&opt), None).build();
        let split = 200usize;
        let c: [&[f64]; 2] = [&high[..split], &low[..split]];
        let (_r1, qs) = plan.run(&c);
        let states = plan.get_states(&qs);
        let IndicatorStateBox::Aroon(restored) = &states[0] else {
            panic!("aroon get_states did not rebuild the hand state");
        };
        assert_eq!(
            qs.window[0].len(),
            period,
            "aroon's anchor window = look_back = period bars per field"
        );

        let mut is = AroonIS::new(&qs.window[0], &qs.window[1], restored.clone(), period);
        let t: [&[f64]; 2] = [&high[split..], &low[split..]];
        let cont = is.batch_indicator(&t, None).unwrap();
        cmp_exact(
            &cont[0],
            &hand[0][split - period..],
            "aroon get_states continuation (down)",
        );
        cmp_exact(
            &cont[1],
            &hand[1][split - period..],
            "aroon get_states continuation (up)",
        );
    }

    /// §1.4 kept contract, persisted: a `JitState` survives a serialize →
    /// reload round-trip and resumes BIT-EXACTLY, with the caller supplying
    /// only new bars (never the price tail). Covers a rolling plan (window +
    /// transient `scratch` must round-trip / rebuild) and asserts the
    /// foreign-state guard still fires on a reloaded state.
    #[test]
    fn jit_state_persist_reload_resumes_bit_exact() {
        use crate::indicators::sma::Sma;
        let close = series(400);
        let period = 20usize;
        let plan = Request::new()
            .add_all(Sma::recipe(&[period as f64]))
            .build();
        let cold = {
            let s: [&[f64]; 1] = [close.as_slice()];
            let (r, _) = plan.run(&s);
            r.request(0).output(0).to_vec()
        };

        // run first 150 bars, persist, reload, resume the rest.
        let split = 150;
        let s1: [&[f64]; 1] = [&close[..split]];
        let (_r1, st) = plan.run(&s1);
        let bytes = serde_json::to_vec(&st).expect("JitState serializes");
        let st_re: JitState = serde_json::from_slice(&bytes).expect("JitState deserializes");
        // transient stitch buffer is not persisted, and must be empty on load.
        assert!(st_re.scratch.is_empty(), "scratch must not serialize");

        let s2: [&[f64]; 1] = [&close[split..]];
        let (r2, _) = plan.resume(st_re.clone(), &s2);

        // bit-exact continuity: (first-chunk rows) ++ (resumed rows) == cold.
        let s1full: [&[f64]; 1] = [close.as_slice()];
        let (r1full, _) = plan.run(&s1full);
        let head = r1full.request(0).output(0);
        let head_part = &head[..head.len() - r2.request(0).output(0).len()];
        let mut joined = head_part.to_vec();
        joined.extend_from_slice(r2.request(0).output(0));
        cmp_exact(&joined, &cold, "sma persist→reload→resume");

        // foreign-state guard survives the round-trip: the reloaded state's
        // plan_key is intact, so it still refuses a DIFFERENT plan.
        let other = Request::new()
            .add_all(Sma::recipe(&[(period + 1) as f64]))
            .build();
        let s_one: [&[f64]; 1] = [[1.0f64].as_slice()];
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = other.resume(st_re.clone(), &s_one);
        }));
        assert!(
            result.is_err(),
            "reloaded JitState must still reject a foreign plan"
        );
        let _ = st_re.next_bar();
    }

    /// E6 feed-gated mean `Wilders`: ADX = `Wilders(dx, feed)` — the fold
    /// window slides to start at `dx`'s publish lead (doubled warmup `2·period−1`).
    /// Emits its real `optional_outputs=[dx, atr, tr]` (so `n_fields=3`, the `tr`
    /// chain needs close), which lets a `natr`+`adx` panel SHARE the `tr`
    /// subgraph + the mean-`ATR` node (both fold the same `tr` chain — deduped
    /// by CSE). Bit-exact primary + all optional mirrors, streaming across the
    /// `2·period−1` seam AND the shared `tr` row (lead 1).
    #[test]
    fn jit_adx_matches_hand() {
        use crate::indicators::adx::Adx;
        use crate::indicators::atr::Atr;
        use crate::indicators::natr::Natr;
        let high = series(500);
        let low: Vec<f64> = high.iter().map(|v| v * 0.97 - 2.0).collect();
        let close: Vec<f64> = (0..500)
            .map(|i| low[i] + (high[i] - low[i]) * 0.5)
            .collect();
        let three: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];
        for period in [7usize, 14] {
            // `.add` enables ALL declared optionals ([dx, atr, tr]) → faithful panel.
            let plan = Request::new()
                .add_all(Adx::recipe(&[period as f64]))
                .build();
            assert_eq!(
                plan.n_fields, 3,
                "adx folds tr (close) for its atr/tr mirrors"
            );
            let (res, _) = plan.run(&three);
            let h = Adx::indicator(&three, &[period as f64], Some(&[true, true, true]))
                .unwrap()
                .0;
            // adx's ATR MIRROR is the shared MEAN chain (= the standalone Atr
            // indicator, bit-exact), NOT hand-adx's `sum·inv` approximation;
            // the two agree to ~1e-16, inside the project's 1e-12 ti_atr oracle.
            let mean_atr = Atr::indicator(&three, &[period as f64], None).unwrap().0;
            cmp_exact(res.request(0).output(0), &h[0], &format!("adx{period}"));
            cmp_exact(
                res.request(0).optional(0),
                &h[1],
                &format!("adx{period} opt-dx"),
            );
            cmp_exact(
                res.request(0).optional(1),
                &mean_atr[0],
                &format!("adx{period} opt-atr=mean"),
            );
            cmp_within(
                res.request(0).optional(1),
                &h[2],
                1e-12,
                &format!("adx{period} opt-atr vs hand sum*inv"),
            );
            cmp_exact(
                res.request(0).optional(2),
                &h[3],
                &format!("adx{period} opt-tr"),
            );
            assert_eq!(
                res.request(0).output(0).len(),
                high.len() - (2 * period - 1),
                "adx publishes from bar 2p-1"
            );
            for split in [2 * period - 1, 2 * period, 200, 499] {
                let s1: [&[f64]; 3] = [&high[..split], &low[..split], &close[..split]];
                let s2: [&[f64]; 3] = [&high[split..], &low[split..], &close[split..]];
                let (h1, qs) = plan.run(&s1);
                let (h2, _) = plan.resume(qs, &s2);
                let mut c0 = h1.request(0).output(0).to_vec();
                c0.extend_from_slice(h2.request(0).output(0));
                cmp_exact(&c0, &h[0], &format!("adx{period} split{split}"));
                // the shared tr row (lead 1) must stitch across the seam too.
                let mut c3 = h1.request(0).optional(2).to_vec();
                c3.extend_from_slice(h2.request(0).optional(2));
                cmp_exact(&c3, &h[3], &format!("adx{period} tr split{split}"));
            }
        }

        // natr + adx panel: both reference the IDENTICAL mean-`Wilders(tr)` node
        // (natr's ATR = adx's ATR mirror) → CSE dedups the whole ATR chain, one
        // level up from `tr`. Every row exact; the two atr rows are bit-identical
        // (same fused node), proving the share.
        let panel = Request::new()
            .add_all(Natr::recipe(&[14.0]))
            .add_all(Adx::recipe(&[14.0]))
            .build();
        let (pres, _) = panel.run(&three);
        let ah = Adx::indicator(&three, &[14.0], Some(&[true, true, true]))
            .unwrap()
            .0;
        // natr with its [atr, tr] optionals on (`.add` enables them).
        let nh = Natr::indicator(&three, &[14.0], Some(&[true, true]))
            .unwrap()
            .0;
        cmp_exact(pres.request(0).output(0), &nh[0], "natr (panel)");
        cmp_exact(pres.request(0).optional(0), &nh[1], "natr atr (panel)");
        cmp_exact(pres.request(0).optional(1), &nh[2], "natr tr (panel)");
        cmp_exact(pres.request(1).output(0), &ah[0], "adx (panel)");
        cmp_exact(pres.request(1).optional(0), &ah[1], "adx dx (panel)");
        cmp_exact(
            pres.request(1).optional(2),
            &ah[3],
            "adx tr (panel, shared with natr)",
        );
        // THE PROOF: natr's ATR and adx's ATR are the SAME fused node →
        // bit-identical rows (one shared mean-`Wilders(tr)`, not two chains).
        cmp_exact(
            pres.request(0).optional(0),
            pres.request(1).optional(1),
            "natr ATR == adx ATR (deduped to one node)",
        );
    }

    /// E7 multi-value FEEDBACK op: SUPERTREND — the first chain that carries
    /// several states (prev_st/prev_ub/prev_lb f64 + a `trend` boolean), fed by
    /// the SHARED mean-`atr`/`medprice`/`tr` nodes. Bit-exact primary + all
    /// optional mirrors (the atr mirror == the standalone Atr, proving the
    /// share), streaming across the `period−1` seed seam, and a `natr`+
    /// `supertrend` combo where the two ATR rows are bit-identical (one fused
    /// mean-`Wilders(tr)` node).
    #[test]
    fn jit_supertrend_matches_hand() {
        use crate::indicators::natr::Natr;
        use crate::indicators::supertrend::SuperTrend;
        let high = series(500);
        let low: Vec<f64> = high.iter().map(|v| v * 0.97 - 2.0).collect();
        let close: Vec<f64> = (0..500)
            .map(|i| low[i] + (high[i] - low[i]) * 0.5)
            .collect();
        let three: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];
        for period in [7usize, 14] {
            let opts = [period as f64, 3.0];
            let plan = Request::new()
                .add(SuperTrend::recipe(&opts), Some(&[true, true, true]))
                .build();
            let (res, _) = plan.run(&three);
            let h = SuperTrend::indicator(&three, &opts, Some(&[true, true, true]))
                .unwrap()
                .0;
            cmp_exact(res.request(0).output(0), &h[0], &format!("st p{period}"));
            cmp_exact(
                res.request(0).optional(0),
                &h[1],
                &format!("st p{period} atr"),
            );
            cmp_exact(
                res.request(0).optional(1),
                &h[2],
                &format!("st p{period} tr"),
            );
            assert_eq!(
                res.request(0).output(0).len(),
                high.len() - period,
                "supertrend publishes from bar {period}"
            );
            for split in [period - 1, period, period + 1, 200, 499] {
                let s1: [&[f64]; 3] = [&high[..split], &low[..split], &close[..split]];
                let s2: [&[f64]; 3] = [&high[split..], &low[split..], &close[split..]];
                let (h1, qs) = plan.run(&s1);
                let (h2, _) = plan.resume(qs, &s2);
                let mut c = h1.request(0).output(0).to_vec();
                c.extend_from_slice(h2.request(0).output(0));
                cmp_exact(&c, &h[0], &format!("st p{period} split{split}"));
            }
        }

        // natr + supertrend share the WHOLE mean-ATR chain (identical node),
        // not just tr: the two ATR rows must be bit-identical (dedup proof).
        let combo = Request::new()
            .add_all(Natr::recipe(&[14.0]))
            .add(SuperTrend::recipe(&[14.0, 3.0]), Some(&[true, true, true]))
            .build();
        let (cres, _) = combo.run(&three);
        let nh = Natr::indicator(&three, &[14.0], Some(&[true, true]))
            .unwrap()
            .0;
        let sh = SuperTrend::indicator(&three, &[14.0, 3.0], Some(&[true, true, true]))
            .unwrap()
            .0;
        cmp_exact(cres.request(0).output(0), &nh[0], "natr (st combo)");
        cmp_exact(cres.request(1).output(0), &sh[0], "supertrend (st combo)");
        cmp_exact(
            cres.request(0).optional(0),
            cres.request(1).optional(0),
            "natr ATR == supertrend ATR (deduped mean chain)",
        );
    }

    /// The planner/optimizer must dedup shared math SYMMETRICALLY — request
    /// order cannot change the merged shape or the values. `(adx,natr)` and
    /// `(natr,adx)` yield identical op/store counts and bit-identical outputs,
    /// and the fused op count is STRICTLY below the sum of the two standalone
    /// plans (the shared `tr` subgraph + the three price reads collapse to one).
    #[test]
    fn jit_adx_natr_cse_is_order_independent() {
        use crate::indicators::adx::Adx;
        use crate::indicators::natr::Natr;
        let high = series(300);
        let low: Vec<f64> = high.iter().map(|v| v * 0.97 - 2.0).collect();
        let close: Vec<f64> = (0..300)
            .map(|i| low[i] + (high[i] - low[i]) * 0.5)
            .collect();
        let three: [&[f64]; 3] = [high.as_slice(), low.as_slice(), close.as_slice()];

        let plan_ab = Request::new()
            .add_all(Adx::recipe(&[14.0]))
            .add_all(Natr::recipe(&[14.0]))
            .build();
        let plan_ba = Request::new()
            .add_all(Natr::recipe(&[14.0]))
            .add_all(Adx::recipe(&[14.0]))
            .build();

        // (1) symmetric dedup: order cannot change the compiled shape.
        assert_eq!(
            plan_ab.info(),
            plan_ba.info(),
            "CSE must dedup identically for (adx,natr) and (natr,adx)"
        );

        // (2) sharing really happens: fused < sum-of-standalone op counts.
        let n_natr = Request::new()
            .add_all(Natr::recipe(&[14.0]))
            .build()
            .info()
            .0;
        let n_adx = Request::new()
            .add_all(Adx::recipe(&[14.0]))
            .build()
            .info()
            .0;
        let n_fused = plan_ab.info().0;
        assert!(
            n_fused < n_natr + n_adx,
            "shared tr + the ONE mean-ATR chain must collapse: fused {n_fused} < {n_natr}+{n_adx}"
        );

        // (3) bit-identical values regardless of order (adx is req0 in ab /
        // req1 in ba; natr is req1 in ab / req0 in ba).
        let (ra, _) = plan_ab.run(&three);
        let (rb, _) = plan_ba.run(&three);
        cmp_exact(
            rb.request(1).output(0),
            ra.request(0).output(0),
            "adx primary order-indep",
        );
        cmp_exact(
            ra.request(1).output(0),
            rb.request(0).output(0),
            "natr primary order-indep",
        );
        for k in 0..3 {
            cmp_exact(
                rb.request(1).optional(k),
                ra.request(0).optional(k),
                &format!("adx opt{k} order-indep"),
            );
        }
    }

    fn classic3_all_on() -> Request {
        Request::new()
            .add_all(Apo::recipe(&APO))
            .add_all(Ppo::recipe(&PPO))
            .add_all(Macd::recipe(&MACD))
    }

    fn dense_all_on() -> Request {
        Request::new()
            .add_all(Apo::recipe(&[5.0, 8.0]))
            .add_all(Apo::recipe(&[6.0, 9.0]))
            .add_all(Ppo::recipe(&[9.0, 25.0]))
            .add_all(Ppo::recipe(&[10.0, 26.0]))
            .add_all(Macd::recipe(&[11.0, 23.0, 17.0]))
            .add_all(Macd::recipe(&[12.0, 24.0, 9.0]))
    }

    // NOTE: the allocation-count guard (`jit_resume_allocations`) lives in
    // `tests/jit_alloc_count.rs` — its OWN integration-test binary. A
    // `#[global_allocator]` is process-wide, so keeping it here routed EVERY
    // cse test's allocations through the counting TLS and SIGSEGV'd the `--lib`
    // suite nondeterministically under many threads (reproduced on clean HEAD).

    /// E2c: mirror slots share ONE engine-written buffer per node, with
    /// hand-exact lengths preserved as windows. The all-on classic query
    /// stores six rows; eleven request slots read them (three ema12
    /// mirrors alias whole, the ±1 ema26 case is apo@25 full row and
    /// ppo/macd@26 windows). Values/lengths vs hand are the existing
    /// parity suite; this pins the STRUCTURE + aliasing.
    #[test]
    fn jit_output_mirrors_are_written_once() {
        let close = series(500);
        let s: [&[f64]; 1] = [close.as_slice()];
        let req = Request::new()
            .add_all(Apo::recipe(&APO))
            .add_all(Ppo::recipe(&PPO))
            .add_all(Macd::recipe(&MACD));
        let plan = req.build();
        let (_, stores) = plan.info();
        assert_eq!(stores, 6, "unique write-streams");

        let (res, _) = plan.run(&s);
        // same-node mirrors → same buffer
        assert!(res.optional[0][0].shares_data_with(&res.optional[1][0]));
        assert!(res.optional[1][0].shares_data_with(&res.optional[2][0]));
        // the ±1 lead case: ONE row, windowed reads, hand-exact lengths
        assert!(res.optional[0][1].shares_data_with(&res.optional[1][1]));
        assert_eq!(res.optional[0][1].len(), res.optional[1][1].len() + 1);
        cmp_exact(
            &res.optional[0][1][1..],
            &res.optional[1][1],
            "ppo long window",
        );
        // apo line ≡ macd line primaries alias too
        assert!(res.outputs[0][0].shares_data_with(&res.outputs[2][0]));
        // hand parity of ALIASED rows (values AND per-slot lengths)
        let apo = Apo::indicator(&s, &APO, Some(&[true, true])).unwrap().0;
        let ppo = Ppo::indicator(&s, &PPO, Some(&[true, true])).unwrap().0;
        cmp_exact(res.request(0).optional(1), &apo[2], "apo long full row");
        cmp_exact(
            res.request(1).optional(1),
            &ppo[2],
            "ppo long window vs hand",
        );
        // disabled optionals still print EMPTY through the view layer
        let (rn, _) = Request::new().add(Apo::recipe(&APO), None).build().run(&s);
        assert!(rn.optional[0][0].is_empty() && rn.optional[0][1].is_empty());
    }

    fn cmp_results(a: &Results, b: &Results, what: &str) {
        assert_eq!(a.outputs.len(), b.outputs.len(), "{what}: requests");
        for req in 0..a.outputs.len() {
            for out in 0..a.outputs[req].len() {
                cmp_exact(
                    &a.outputs[req][out],
                    &b.outputs[req][out],
                    &format!("{what} p{req}o{out}"),
                );
            }
            for out in 0..a.optional[req].len() {
                cmp_exact(
                    &a.optional[req][out],
                    &b.optional[req][out],
                    &format!("{what} x{req}o{out}"),
                );
            }
        }
    }
}
