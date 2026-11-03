//! The fused-kernel backend's compile-time unit: the [`Signature`] — the
//! merged arena with **every parameter value erased** — and [`Values`], the
//! numbers the signature can't carry (multipliers, feed bars, sink leads).
//!
//! Same planner as the JIT (`recipe.rs` → `merge.rs` → this module instead of
//! `jit.rs`): the arena's SHAPE (node kinds, slot edges, sink count/source
//! wiring, dedup partition) selects a monomorphised [`crate::cse::kernel`]
//! instantiation; its VALUES arrive per request as plain data bound at kernel
//! init — loop-invariant, so LLVM hoists them out of the specialized bar loop
//! (the M0 IR canary asserts exactly this hoist). Two periods different →
//! same signature, different `Values`; chains dedup vs don't → different
//! signature.
//!
//! M0 vocabulary = the fold shapes: `Input, Const, Ema, Wilders, Sub, Add,
//! Mul, Div, Max, Abs, Lag, Mirror, Sqrt, Fma`, plus `SuperTrend` (the
//! feedback node) and the price-sourced rolling sums `Sma`/`Stddev` (the
//! window is read directly from the stream at `t − period`, so whole-series
//! runs need no anchor; derived-source rolls would — `erase` returns `None`
//! for those). Any other node kind makes [`erase`] return `None` (the
//! request simply doesn't hit a fused template — tier-2/3 territory, by
//! design).

use crate::cse::merge::{Merged, Resolved};
use crate::cse::recipe::{AlphaShape, Id, Node, SinkSrc};

pub const MAX: usize = 64;
pub const MAX_SINKS: usize = 32;

// Node-kind tags — the signature's alphabet. Fixed forever once patterns ship.
pub const K_UNUSED: u8 = 0;
pub const K_INPUT: u8 = 1;
pub const K_CONST: u8 = 2;
pub const K_EMA: u8 = 3;
pub const K_WILDERS: u8 = 4;
pub const K_SUB: u8 = 5;
pub const K_ADD: u8 = 6;
pub const K_MUL: u8 = 7;
pub const K_DIV: u8 = 8;
pub const K_MAX: u8 = 9;
pub const K_ABS: u8 = 10;
pub const K_LAG: u8 = 11;
pub const K_MIRROR: u8 = 12;
pub const K_SQRT: u8 = 13;
pub const K_FMA: u8 = 14;
pub const K_SUPERTREND: u8 = 15;
pub const K_SMA: u8 = 16;
pub const K_STDDEV: u8 = 17;
pub const K_WMA: u8 = 18;
pub const K_ROLLMAX: u8 = 19;
pub const K_ROLLMIN: u8 = 20;
pub const K_TRAIL: u8 = 21;
pub const K_WHEREGE: u8 = 22;
pub const K_WHERELT: u8 = 23;
/// Rolling sum over a DERIVED source (ao / stoch smoothers / hma stages):
/// the leaving value comes from a ring, not the stream. Rings are SHARED
/// per source — interning already made the source structural; per roll node
/// `Values::ring_of[k]` names its group's leader slot and
/// `Values::ring_cap[leader]` is the group's max period (capacity is a
/// VALUE, never a pattern — rev-2 law).
pub const K_ROLLSUM: u8 = 24;
/// The bespoke Hull MA machine (E3 stage 3): two inner WMA engines (p and
/// p/2 with the hand's OFFSET fold window), the diff series and its √p ring
/// ALL inside one node — composition is not bit-exact (the state2 fold
/// weights restart; the tail divides before its subtractions). Values ride
/// per kind: `periods`=p, `roll_lb`=p/2, `lag_n`=trunc √p, `mults`/`invms`=
/// the two weight reciprocals, `st_step`=√p as f64 (the tail's enter
/// weight), `consts`=the triangular weight sum; the diff ring is its own
/// group (`ring_of[k] = k`, `ring_cap[k] = √p`).
pub const K_HMA: u8 = 25;
/// `ln(src)` — a pure host call per bar (`f64::ln`, the SAME libm the hand's
/// `calc_fisher` uses; the JIT-era §6.41 finding: tuned libm beats the
/// inline polynomial for a scalar per-bar recurrence). Lead = lead(src).
pub const K_LN: u8 = 26;
/// The generic 2-op IIR line: `new = prev·b + src·a` (`mul_add(prev, invm,
/// src·mult)` — the Ema/mean-Wilders op shape with EXPLICIT weights; the
/// weights are values (`mults`=a, `invms`=b), so any (a,b) hits one kernel).
/// Carries ride `st[k]`; below `feeds[k]` the node idles 0 and the carry
/// stays at its init (the hand's zero-initialized state) — no seed bar.
pub const K_IIR: u8 = 27;
/// `Iir` plus the hand's if/elif clamp folded INTO the carried value (what
/// feeds the next bar is the clamped one): `gt(hi_t)→hi_v elif lt(lo_t)→lo_v`
/// — selects, bit-identical to the hand's branches for all floats incl.
/// NaN. Operands: `a`=src, `b`=hi_t, `c`=hi_v, `d`=lo_t, `e`=lo_v (all Const
/// slots; the four refs are STRUCTURE, their numbers ride `consts` — rev-2).
pub const K_IIRCLAMPED: u8 = 28;
/// The chain's carried-IN value — `signal = prev fish`: emits the src
/// chain's PREVIOUS bar output. Owns a delayed slot in `st[k]`: emit
/// `st[k]`, then `st[k] := src's value this bar` (arena order guarantees the
/// src node ran first; below `feeds[k]` both sides are the zero init — the
/// hand's first-bar signal is its zero-initialized carry).
pub const K_PREV: u8 = 29;
/// IEEE minNum — the exact mirror of `K_MAX`, spelled `f64::min` on the scalar
/// axis and `simd_min` on the lane axis (the hand's own split: `ultosc.rs`
/// `low.min(prev_close)` vs `ultosc_simd.rs` `low.simd_min(prev_close)`). NOT
/// a `WhereLt` select — that pins `0.0` on one arm, so it disagrees on NaN and
/// on the `-0.0`/`+0.0` tie. Its own `lane_formula` id, because `minpd` and
/// `maxpd` are different instructions and one id means one instruction
/// sequence.
pub const K_MIN: u8 = 30;
/// Strict select keeping the value on `>`: `if a > b { t } else { 0.0 }` — the
/// `K_WHEREGE` sibling. On NaN the compare is false, so this takes the PINNED
/// zero; that is the opposite of an early-return guard, which computes. See
/// [`K_WHERELE`] for the polarity a `range <= eps` guard needs.
pub const K_WHEREGT: u8 = 31;
/// The `K_WHERELT` sibling that keeps the value on `a > b` *by negating `<=`*:
/// `if a <= b { 0.0 } else { t }`. A NaN compare makes `<=` false, so the value
/// arm runs and poisons — which is exactly the hand's early-return guard
/// (`ad.rs`: `if range <= eps { return self.ad }`).
pub const K_WHERELE: u8 = 32;
/// Single-rounding running total `st[k] = a.mul_add(b, st[k])` — zero init,
/// gated by `feeds[k]` like the other recurrences, no weights. Distinct from
/// `K_IIR` (which double-rounds the product) and from `K_FMA` (whose third
/// operand is a node, not this node's own carry).
pub const K_ACCUM: u8 = 33;
/// Negative Volume Index recurrence (`Node::NviRecur` doc): a gated
/// multiplicative prefix product, scalar-only (unpackable).
pub const K_NVI: u8 = 34;
/// Positive Volume Index recurrence (`Node::PviRecur` doc) — the mirror gate.
pub const K_PVI: u8 = 35;
/// Ring-backed age roll (K_ROLLAGE): publish src delayed by n bars, backed
/// by a ring of capacity n. The published value is the raw evicted/age-read
/// from the ring (no running sum accumulation). Same group protocol as RollSum.
pub const K_ROLLAGE: u8 = 36;
/// Mean Absolute Deviation rescan: `sum(|x - sma|) / period` over the window
/// ending at this bar (`md`/`cci`'s shared source). `b` names the explicit
/// shared `sma` operand (a `RollSum{mean:true}` node) so the mean-of-window
/// math is read off ONE carried sum, not an internally recomputed copy.
pub const K_MEANABSDEV: u8 = 37;
/// KvoFlow uses tag 38 (must match `Kind::tag()` in planner.rs).
pub const K_KVOFLOW: u8 = 38;
/// Custom-seeded EMA (`Node::SeededEma` doc): `a`=seed node, `b`=fold
/// (src) node. `feeds[k]` = the SEED's own lead or explicit feed override;
/// `t == feed` seeds `st[k]` from the SEED's value, every later bar folds
/// the FOLD node's value with the ordinary `K_EMA` step
/// (`st.mul_add(invms, s*mults)`). `trvi`'s `ema`: seed=`high-low` (bar 0),
/// fold=`tr` (bar 1 on). `zlema` seeds from `real[lag-1]` via an explicit
/// feed override where the raw input's own lead is 0.
pub const K_SEEDEDEMA: u8 = 39;
/// One-pole HighPass recurrence (`Node::HighPass`): `a`=input, own 2-field
/// carry (`y1`, `prev_real`) lives in the kernel's dedicated `hp_acc` array
/// (NOT `st`/`consts` - a fresh parallel array, same shape as `kvacc`).
/// Coefficients `(a1, a2)` ride `mults`/`invms` (constant per `period`, the
/// same storage convention `K_IIR`'s explicit weights use). No feed gate:
/// the hand recurrence runs unconditionally from bar 0 with a zero carry.
pub const K_HIGHPASS: u8 = 40;
/// Two-pole SuperSmoother recurrence (`Node::SuperSmoother`): `a`=src, own
/// 3-field carry (`y1`, `y2`, `prev_real`) lives in the kernel's dedicated
/// `ss_acc` array. Coefficients `(a1, a2, b0)` ride `mults`/`invms`/`st_step`
/// (reusing SuperTrend's step slot - shape match, not identity). No feed
/// gate, same unconditional-from-bar-0 rule as `K_HIGHPASS`.
pub const K_SUPERSMOOTHER: u8 = 41;
/// Hilbert in-phase tap (`Node::HtInPhase`): `a`=src (the roofing/SuperSmoother
/// node). Shares a physical 7-tap ring with any `K_HTQUADRATURE` over the SAME
/// `src` (lowering's `ring_src`/`member_class` group them, `RCLASS_BESPOKE`).
/// `feeds[k]` = `leads[src]` - the ring is fed only from the bar the roofing
/// chain itself first defines (hand `init_state` never pushes into the ring
/// before that). Published value = ring age 3 (the centre tap).
pub const K_HTINPHASE: u8 = 42;
/// Hilbert quadrature FIR sum (`Node::HtQuadrature`) over the SAME shared ring
/// `K_HTINPHASE` feeds. `C0..C3` are kind-intrinsic literals (ht_kernel), not
/// `Values` data - both kinds are structurally identical graph leaves, only
/// the published read differs.
pub const K_HTQUADRATURE: u8 = 43;
/// `linreg`'s sliding-moment carry (`Node::LinregFlow` doc). Steady is three
/// ops with NO branch — `pub = st + src·mults[k]`, publish `pub`,
/// `st = pub − v[sub]` — with the publish BEFORE the evict, which no `RollSum`
/// region spells. `mults[k]` is the fold weight: `period` on the weighted line,
/// `1.0` on the plain sum (`x * 1.0` is bit-identical to `x`, so ONE emitted
/// shape serves both — the same trick `K_WILDERS`' composite uses). That makes
/// the weighted/plain choice a VALUES fact like any other period, NOT
/// structure: the two instances already differ in `b`, and `intern` keys on the
/// whole `Node`. `feeds[k]` is the first published bar AND the seam — below it
/// the fold runs with a bar-index weight and no eviction; because the sink lead
/// equals the seam and the steady loop starts at `max_gate + 1`, every fold bar
/// is a warm bar, so the JIT body never tests it. Scalar-only: one instance's
/// `sub` is a per-lane `Lag` load.
pub const K_LINREGFLOW: u8 = 44;
/// Variable-alpha blend recurrence (`Node::VariableAlpha` doc): `a`=seed,
/// `b`=src (value), `c`=mult (per-bar live blend-weight read). `feeds[k]` =
/// the SEED's own lead or explicit feed override, same convention as
/// `K_SEEDEDEMA`. `shape` (Ema vs Vidya fold spelling) rides structure, not
/// VALUES, so the two spellings never collide in one signature bucket.
pub const K_VARIABLEALPHA: u8 = 45;

/// The erased structure. Integers only — a `const` generic parameter must
/// compare structurally, and every f64/period/lead lives in [`Values`].
#[derive(Clone, Copy, PartialEq, Eq, std::marker::ConstParamTy)]
pub struct Signature {
    pub len: u32,
    pub kinds: [u8; MAX],
    /// child slots (a = src/first operand, b/c = rest; period/n/composite
    /// parameters are VALUES, indexed by slot)
    pub a: [u32; MAX],
    pub b: [u32; MAX],
    pub c: [u32; MAX],
    /// extra child refs for 5-operand kinds (K_IIRCLAMPED's clamp Consts;
    /// every other kind keeps 0 — the arrays join the canonical hash).
    pub d: [u32; MAX],
    pub e: [u32; MAX],
    pub n_streams: u32,
    pub n_sinks: u32,
    pub sink_src: [u32; MAX_SINKS],
    /// The published ring-group class at each ring-backed slot
    /// (`RCLASS_NONE` elsewhere) — the layout fact the lowering planner
    /// derives from `kinds`/`a` in `lowering::ring_facts`. Part of the key,
    /// so two arenas that erase equal MUST lower identically: the emitters
    /// read this instead of scanning node kinds for a layout decision.
    pub ring_class: [u8; MAX],
    /// Each ring-backed slot's group LEADER slot — the slot whose
    /// `Values::ring_cap` sizes the shared ring. The same one derivation;
    /// `erase` binds `Values::ring_of` from it, so the key and the runtime
    /// binding are one fact seen twice, never two derivations.
    pub ring_leader: [u32; MAX],
}

impl core::fmt::Debug for Signature {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Signature{{len:{}, sinks:{}}}", self.len, self.n_sinks)
    }
}
impl core::hash::Hash for Signature {
    fn hash<H: core::hash::Hasher>(&self, st: &mut H) {
        self.bytes().for_each(|b| st.write_u8(b));
    }
}

impl Signature {
    pub const fn zeroed() -> Self {
        Signature {
            len: 0,
            kinds: [K_UNUSED; MAX],
            a: [0; MAX],
            b: [0; MAX],
            c: [0; MAX],
            d: [0; MAX],
            e: [0; MAX],
            n_streams: 0,
            n_sinks: 0,
            sink_src: [0; MAX_SINKS],
            ring_class: [crate::cse::lowering::RCLASS_NONE; MAX],
            ring_leader: [0; MAX],
        }
    }
    /// Const constructor for registry literals: copy the live heads of the
    /// pattern arrays into the fixed-size slots (const-fn loops only, so the
    /// literals cannot silently differ from what `erase` derives — the
    /// registry consistency test asserts byte equality).
    pub const fn build<const L: usize, const S: usize>(
        kinds: &[u8; L],
        a: &[u32; L],
        b: &[u32; L],
        c: &[u32; L],
        d: &[u32; L],
        e: &[u32; L],
        sink_src: &[u32; S],
        n_streams: u32,
    ) -> Signature {
        let mut s = Self::zeroed();
        let mut i = 0;
        while i < L {
            s.kinds[i] = kinds[i];
            s.a[i] = a[i];
            s.b[i] = b[i];
            s.c[i] = c[i];
            s.d[i] = d[i];
            s.e[i] = e[i];
            i += 1;
        }
        s.len = L as u32;
        let mut j = 0;
        while j < S {
            s.sink_src[j] = sink_src[j];
            j += 1;
        }
        s.n_sinks = S as u32;
        s.n_streams = n_streams;
        // The layout plan is DERIVED here, not passed in: a registry literal
        // cannot be built whose published plan disagrees with its own arena
        // edges, because there is no second implementation to disagree with.
        // `erase` fills the same fields through the same function, so the
        // sig-consistency tests are an equality check on one derivation.
        let (class, leader) = crate::cse::lowering::ring_facts(&s.kinds, &s.a, L);
        s.ring_class = class;
        s.ring_leader = leader;
        s
    }
    /// Canonical byte view — the dispatch hash's input.
    pub fn bytes(&self) -> impl Iterator<Item = u8> + '_ {
        // only used for hashing; iterate the live head of each array
        let n = self.len as usize;
        let ns = self.n_sinks as usize;
        self.kinds[..n]
            .iter()
            .copied()
            .chain(self.a[..n].iter().flat_map(|v| v.to_le_bytes()))
            .chain(self.b[..n].iter().flat_map(|v| v.to_le_bytes()))
            .chain(self.c[..n].iter().flat_map(|v| v.to_le_bytes()))
            .chain(self.d[..n].iter().flat_map(|v| v.to_le_bytes()))
            .chain(self.e[..n].iter().flat_map(|v| v.to_le_bytes()))
            .chain(std::array::from_fn::<u8, 8, _>(|i| {
                (self.n_streams as u64).to_le_bytes()[i]
            }))
            .chain(std::array::from_fn::<u8, 8, _>(|i| {
                (self.n_sinks as u64).to_le_bytes()[i]
            }))
            .chain(self.sink_src[..ns].iter().flat_map(|v| v.to_le_bytes()))
    }
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in self.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
        h
    }
}

/// The runtime binding: everything the signature erases, per arena slot
/// (chain slots carry mults/feeds; Const slots carry values; sink slots carry
/// the EFFECTIVE publish lead).
#[derive(Clone, Debug)]
pub struct Values {
    pub consts: [f64; MAX],
    pub mults: [f64; MAX],
    pub invms: [f64; MAX],
    pub feeds: [u32; MAX],
    pub lag_n: [u32; MAX],
    pub periods: [u32; MAX],
    /// Trail-rescan rolls: the WINDOW span is `roll_lb + 1` bars (`look_back`
    /// of the node); the rescan TRIGGER is `periods[k]`. Both are VALUES —
    /// the aroon window rule rides them, the arena keys stay structural.
    pub roll_lb: [u32; MAX],
    /// per-sink: composite (raw-sum) Wilders seed — a VALUES fact, the
    /// mean/composite choice itself is folded into kind via `c` (2 slots:
    /// it changes emitted numbers, so it is structure per the E12 rule).
    pub sink_lead: [u32; MAX_SINKS],
    pub max_gate: u32,
    /// SuperTrend step multiplier (band half-width = step × atr), one f64
    /// per node.
    pub st_step: [f64; MAX],
    /// Runtime optional-output enable flags (parallel to sink order). A sink
    /// with want=false gets a zero-length RowRef window (is_empty() == true).
    /// This is the NEW runtime model — optional sinks are no longer filtered at
    /// Signature construction; they're always present in the arena, but their
    /// stores are guarded and their rows are empty.
    pub sink_want: [bool; MAX_SINKS],
    /// K_ROLLSUM only: the roll node's ring GROUP = the leader slot of the
    /// first ring roll over the same src (0 where the node is not a roll).
    pub ring_of: [u32; MAX],
    /// Valid at a ring group's LEADER slot: the shared ring's capacity = max
    /// period across the group. A roll whose period equals the cap leaves by
    /// eviction (`push_with_info`); a shorter one by age read at its period.
    pub ring_cap: [u32; MAX],
    /// K_ROLLSUM only: leading bars whose source value the hand discards —
    /// the fold starts at `feeds[k] + skip` (the ring still feeds from
    /// `feeds[k]` so absolute-age reads line up). Stoch's slow %K skips 2.
    pub skip: [u32; MAX],
    /// K_ROLLSUM only: first bar (absolute) where the hand's steady form
    /// (`sum += v - old`) takes over. Before it, past the seam, the hand's
    /// init loop subtracts the leave as its OWN statement (`sum += v;
    /// sum -= old`) — float-visible, so the emitter must switch forms at
    /// exactly this bar. 0 = steady from the seam (all pre-stoch shapes).
    pub steady_from: [u32; MAX],
}

impl Values {
    pub fn zeroed() -> Self {
        Values {
            consts: [0.0; MAX],
            mults: [0.0; MAX],
            invms: [0.0; MAX],
            feeds: [0; MAX],
            lag_n: [0; MAX],
            periods: [0; MAX],
            roll_lb: [0; MAX],
            sink_lead: [0; MAX_SINKS],
            max_gate: 0,
            st_step: [0.0; MAX],
            sink_want: [false; MAX_SINKS],
            ring_of: [0; MAX],
            ring_cap: [0; MAX],
            skip: [0; MAX],
            steady_from: [0; MAX],
        }
    }
}

/// Which request+slot a sink row belongs to (result regrouping; derived from
/// the same resolved data as the signature, but a lookup table, not shape).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SinkOwner {
    pub req: usize,
    pub primary: bool,
    pub idx: usize,
}

/// Erase a merged arena into (signature, values). `None` if any node kind is
/// outside the M0 fold vocabulary, or arena/sink sizes exceed the caps.
pub fn erase(
    arena: &Merged,
    resolved: &[Resolved],
    leads: &[Id],
) -> Option<(Signature, Values, Vec<SinkOwner>)> {
    let mut sig = Signature::zeroed();
    let mut vals = Values::zeroed();
    if arena.nodes.len() > MAX || resolved.is_empty() {
        return None;
    }
    for (k, node) in arena.nodes.iter().enumerate() {
        // K_IIRCLAMPED's two extra Const refs (lo_t/lo_v) travel through
        // these locals: the refs ARE structure — two arenas wired
        // differently must not share a signature.
        let mut dd = 0u32;
        let mut ee = 0u32;
        let (kind, a, b, c) = match *node {
            Node::Input { stream } => {
                sig.n_streams = sig.n_streams.max(stream + 1);
                (K_INPUT, stream, 0, 0)
            }
            Node::Const { bits } => {
                vals.consts[k] = f64::from_bits(bits);
                (K_CONST, 0, 0, 0)
            }
            Node::Ema { src, period } => {
                let (m, i) = super::multipliers::ema_multiplier(period as usize);
                vals.mults[k] = m;
                vals.invms[k] = i;
                vals.periods[k] = period;
                vals.feeds[k] = if matches!(arena.nodes[src as usize], Node::Input { .. }) {
                    0
                } else {
                    leads[src as usize]
                };
                (K_EMA, src, 0, 0)
            }
            Node::SeededEma {
                seed,
                src,
                period,
                feed,
            } => {
                let (m, i) = super::multipliers::ema_multiplier(period as usize);
                vals.mults[k] = m;
                vals.invms[k] = i;
                vals.periods[k] = period;
                vals.feeds[k] = feed.unwrap_or_else(|| {
                    if matches!(arena.nodes[seed as usize], Node::Input { .. }) {
                        0
                    } else {
                        leads[seed as usize]
                    }
                });
                (K_SEEDEDEMA, seed, src, 0)
            }
            Node::Wilders {
                src,
                period,
                composite,
                feed,
            } => {
                let (m, i) = super::multipliers::wilders_multiplier(period as usize);
                vals.mults[k] = m;
                // COMPOSITE (partial_calc) is the MEAN step with the source
                // scale forced to unity: hand `wilders::partial_calc` is
                // `st.mul_add(multiplier, value)` against mean's
                // `st.mul_add(multiplier, value * inv_multiplier)`, so ONE
                // magnitude carries the whole difference and every reader of
                // `invms` — the embedded warm steady branch, the scalar steady
                // arm, a packed Wilders group — lands on the hand's VALUES
                // without a per-path branch (`x * 1.0` is bit-identical to `x`
                // for every f64: NaN payload, signed zero, infinities).
                // COST, stated not assumed: the emitted step stays `fmul`+`fma`
                // where the hand pays one `fma` — `invms[k]` is a runtime load,
                // so LLVM will not fold the multiply (no fast-math). The hub
                // arena carries THREE composite slots (`+DM`, `−DM`, DI's
                // ATR-sum), so the delta is 3 `fmul`/bar; the wall-clock cost is
                // UNMEASURED. Removing it needs a second `chain_step` spelling in
                // BOTH renderers, which is exactly the drift the shared `shape`
                // helper exists to prevent — so it stays off the table until the
                // hub's runonly row measures the difference.
                vals.invms[k] = if composite { 1.0 } else { i };
                vals.periods[k] = period;
                vals.feeds[k] = if feed { leads[src as usize] } else { 0 };
                // mean-vs-composite changes emitted values → structure (E12 rule)
                (K_WILDERS, src, composite as u32, 0)
            }
            Node::HighPass { input, period } => {
                let (a1, a2) = super::multipliers::highpass_multiplier(period as usize);
                vals.mults[k] = a1;
                vals.invms[k] = a2;
                vals.periods[k] = period;
                (K_HIGHPASS, input, 0, 0)
            }
            Node::SuperSmoother { src, period } => {
                let (a1, a2, b0) = super::multipliers::supersmoother_multiplier(period as usize);
                vals.mults[k] = a1;
                vals.invms[k] = a2;
                vals.st_step[k] = b0;
                vals.periods[k] = period;
                (K_SUPERSMOOTHER, src, 0, 0)
            }
            Node::HtInPhase { src } => {
                vals.feeds[k] = leads[src as usize];
                vals.periods[k] = 7;
                (K_HTINPHASE, src, 0, 0)
            }
            Node::HtQuadrature { src } => {
                vals.feeds[k] = leads[src as usize];
                vals.periods[k] = 7;
                (K_HTQUADRATURE, src, 0, 0)
            }
            Node::Sub { a: x, b: y } => (K_SUB, x, y, 0),
            Node::Add { a: x, b: y } => (K_ADD, x, y, 0),
            Node::Mul { a: x, b: y } => (K_MUL, x, y, 0),
            Node::Div { a: x, b: y } => (K_DIV, x, y, 0),
            Node::Max { a: x, b: y } => (K_MAX, x, y, 0),
            Node::Min { a: x, b: y } => (K_MIN, x, y, 0),
            Node::Abs { src } => (K_ABS, src, 0, 0),
            Node::Lag { src, n } => {
                // price-stream reads only (recipe.rs asserts); clamp rule
                // is the hand's: t < n reads sample 0.
                vals.lag_n[k] = n;
                (K_LAG, src, 0, 0)
            }
            Node::Mirror { src } => (K_MIRROR, src, 0, 0),
            Node::Sqrt { src } => (K_SQRT, src, 0, 0),
            Node::RollMax {
                src,
                period,
                look_back,
            }
            | Node::RollMin {
                src,
                period,
                look_back,
            } => {
                let derived = !matches!(arena.nodes[src as usize], Node::Input { .. });
                if derived {
                    // EXTREMUM RING (the fourth ring shape): the rescan window
                    // cannot be a stream slice, so the src column rides the
                    // SAME group the ring sums do — one push per group per bar,
                    // cap = max period across ALL rolls over the src (sums and
                    // extrema share it; a fisher+ao co-request feeds medprice
                    // ONCE). The group's leader and class are the published
                    // plan's fact (bound after this loop); the rescan reads
                    // ages `0..=look_back` (never past the cap).
                    vals.feeds[k] = leads[src as usize];
                }
                // Pointer form (price-sourced) keeps its trail-rescan straight
                // off the stream (the hand's `max.rs` window shape).
                vals.periods[k] = period;
                vals.roll_lb[k] = look_back;
                (
                    if matches!(*node, Node::RollMax { .. }) {
                        K_ROLLMAX
                    } else {
                        K_ROLLMIN
                    },
                    src,
                    0,
                    0,
                )
            }
            Node::Trail { src } => (K_TRAIL, src, 0, 0),
            Node::WhereGe { a, b, t } => (K_WHEREGE, a, b, t),
            Node::WhereLt { a, b, t } => (K_WHERELT, a, b, t),
            Node::WhereGt { a, b, t } => (K_WHEREGT, a, b, t),
            Node::WhereLe { a, b, t } => (K_WHERELE, a, b, t),
            Node::Accum { a, b } => {
                // A recurrence with two node operands and no weights: it starts
                // accumulating once BOTH are defined, exactly like the product it
                // fuses (`Mul`'s lead rule), and idles at its zero init below.
                vals.feeds[k] = leads[a as usize].max(leads[b as usize]);
                (K_ACCUM, a, b, 0)
            }
            Node::Sma { src, period } => {
                // Price-sourced rolling sum only: the steady step reads the
                // leaving bar `src[t − p]` straight off the stream, which is
                // only sound when the source IS a stream. A derived-source
                // roll needs the host anchor window — not emittable yet.
                if !matches!(arena.nodes[src as usize], Node::Input { .. }) {
                    return None;
                }
                vals.mults[k] = 1.0 / period as f64;
                vals.periods[k] = period;
                (K_SMA, src, 0, 0)
            }
            Node::Stddev { src, period } => {
                // Self-contained two-moment node (sum + sum-of-squares),
                // same price-source rule as `Sma`.
                if !matches!(arena.nodes[src as usize], Node::Input { .. }) {
                    return None;
                }
                vals.mults[k] = 1.0 / period as f64;
                vals.periods[k] = period;
                (K_STDDEV, src, 0, 0)
            }
            Node::Wma { src, period } => {
                // The weighted trick node: carries its own window `sum`
                // (read BEFORE its update per the hand `calc` order) plus
                // the weighted-sum line. `mults` = the weight reciprocal
                // exactly as the hand's `multiplier()` computes it.
                if !matches!(arena.nodes[src as usize], Node::Input { .. }) {
                    return None;
                }
                let n = period as f64;
                vals.mults[k] = 2.0 / (n * (n + 1.0));
                vals.periods[k] = period;
                (K_WMA, src, 0, 0)
            }
            Node::Hma { src, period } => {
                // Derivations mirror `hma.rs` exactly (integer truncations
                // first, THEN f64 — a float sqrt fed forward would diverge).
                let p2 = (period / 2) as usize;
                let m = ((period as f64).sqrt() as usize) as f64;
                let mf = period as f64;
                let n2f = p2 as f64;
                vals.periods[k] = period;
                vals.roll_lb[k] = p2 as u32;
                vals.lag_n[k] = m as u32;
                vals.mults[k] = 2.0 / (mf * (mf + 1.0));
                vals.invms[k] = if p2 > 0 {
                    2.0 / (n2f * (n2f + 1.0))
                } else {
                    0.0
                };
                vals.st_step[k] = m;
                vals.consts[k] = m * (m + 1.0) / 2.0;
                vals.feeds[k] = leads[src as usize];
                // Its diff ring is a group of one keyed on the node itself —
                // the published plan says so (class `BESPOKE`, leader = slot);
                // `lag_n` (= trunc √p) is what sizes it, bound after this loop.
                (K_HMA, src, 0, 0)
            }
            Node::Fma { a: x, b: y, c: z } => (K_FMA, x, y, z),
            Node::Ln { src } => (K_LN, src, 0, 0),
            Node::Iir {
                src,
                a_bits,
                b_bits,
            } => {
                // Explicit-weight IIR: the weights are VALUES (any a/b hits
                // one kernel); `feeds` = the bar the SRC first defines —
                // below it the node idles 0 and the carry keeps the hand's
                // zero init (fisher's state.fish/val1 start at 0.0, there is
                // NO seed bar — unlike Ema's `t == feed` seed).
                vals.mults[k] = f64::from_bits(a_bits);
                vals.invms[k] = f64::from_bits(b_bits);
                vals.feeds[k] = leads[src as usize];
                (K_IIR, src, 0, 0)
            }
            Node::IirClamped {
                src,
                a_bits,
                b_bits,
                hi_t,
                hi_v,
                lo_t,
                lo_v,
            } => {
                vals.mults[k] = f64::from_bits(a_bits);
                vals.invms[k] = f64::from_bits(b_bits);
                vals.feeds[k] = leads[src as usize];
                dd = lo_t;
                ee = lo_v;
                (K_IIRCLAMPED, src, hi_t, hi_v)
            }
            Node::Prev { src } => {
                // Delayed-copy slot (see K_PREV doc): `feeds` gates the copy
                // exactly like the chain it shadows.
                vals.feeds[k] = leads[src as usize];
                (K_PREV, src, 0, 0)
            }
            Node::RollAge { src, n } => {
                // Ring-backed age roll: no carry needed (pure age-read), just
                // the group coords for feed/evict. The ring is shared per src
                // via the published plan; cap = period (ring capacity). No mult.
                vals.periods[k] = n as u32;
                vals.feeds[k] = leads[src as usize];
                (K_ROLLAGE, src, 0, 0)
            }
            Node::SuperTrend {
                atr,
                med,
                close,
                period,
                step_bits,
            } => {
                vals.periods[k] = period;
                vals.st_step[k] = f64::from_bits(step_bits);
                // Execution seam = the bar the ATR chain first defines (the
                // mean seed lands there and the hand runs its ONE seed
                // `calc_st` on the same bar with zero carry). Before it the
                // kernel arm idles — running earlier bars would poison the
                // band/trend carry (hand: init_state seeds st at period−1).
                vals.feeds[k] = leads[atr as usize];
                // `atr`/`med`/`close` are arena SLOT ids (they become a/b/c —
                // the edges), NOT stream indices; n_streams stays whatever the
                // K_INPUT arms established (max stream index + 1).
                (K_SUPERTREND, atr, med, close)
            }
            Node::RollSum {
                src,
                period,
                mean,
                skip,
                steady_from,
            } => {
                // Ring-backed rolling sum — derived sources included (that is
                // its whole point vs `Sma`). The ring is shared per src via
                // the published plan (bound after this loop); leave =
                // eviction at the group cap, age read below it (see `ring_cap`
                // docs). `skip`/`steady_from` are VALUES, not structure: one
                // RollSum kind covers both shapes, the emitter's warm regions
                // (fold/seam/init/steady) read the two numbers per bar.
                // Stoch's slow %K was the unlocking pair.
                vals.periods[k] = period;
                vals.mults[k] = if mean { 1.0 / period as f64 } else { 1.0 };
                vals.skip[k] = skip;
                vals.steady_from[k] = steady_from;
                // The fold starts the bar the source first defines.
                vals.feeds[k] = leads[src as usize];
                (K_ROLLSUM, src, 0, 0)
            }
            Node::MeanAbsDev { src, period, sma } => {
                vals.periods[k] = period;
                vals.feeds[k] = leads[sma as usize] + 1;
                (K_MEANABSDEV, src, sma, 0)
            }
            Node::LinregFlow {
                src,
                sub,
                period,
                weighted,
            } => {
                // ONE emitted shape serves both of the hand's sums: the plain
                // `sum_y` folds with weight 1.0, the weighted `sum_xy` with
                // weight `period`, and `x * 1.0` is bit-identical to `x` for
                // every f64 — so the difference rides `mults` and never splits
                // the kind (same argument `K_WILDERS`' composite makes).
                vals.periods[k] = period;
                vals.mults[k] = if weighted { period as f64 } else { 1.0 };
                // Seam == lead. The hand slices `real[1..]` and its first
                // published bar is `period`, so every earlier bar only feeds
                // the seed fold — and because the steady loop starts at
                // `max_gate + 1`, above this seam, the emitted arm needs no
                // warm branch at all.
                vals.feeds[k] = leads[src as usize] + period;
                (K_LINREGFLOW, src, sub, 0)
            }
            Node::NviRecur {
                src,
                prev_close,
                volume,
                prev_volume,
            } => {
                vals.feeds[k] = leads[src as usize]
                    .max(leads[prev_close as usize])
                    .max(leads[volume as usize])
                    .max(leads[prev_volume as usize]);
                dd = prev_volume;
                (K_NVI, src, prev_close, volume)
            }
            Node::PviRecur {
                src,
                prev_close,
                volume,
                prev_volume,
            } => {
                vals.feeds[k] = leads[src as usize]
                    .max(leads[prev_close as usize])
                    .max(leads[volume as usize])
                    .max(leads[prev_volume as usize]);
                dd = prev_volume;
                (K_PVI, src, prev_close, volume)
            }
            Node::KvoFlow {
                high,
                low,
                close,
                volume,
            } => {
                // Hand `init_state` loop starts at i=1, never calling
                // `calc_vf` at i=0 (needs a "previous" hlc/hl to compare
                // against, even though the arithmetic itself would run at
                // bar 0) -- first valid vf bar is one past the inputs' lead.
                vals.feeds[k] = leads[high as usize]
                    .max(leads[low as usize])
                    .max(leads[close as usize])
                    .max(leads[volume as usize])
                    + 1;
                dd = volume;
                (K_KVOFLOW, high, low, close)
            }
            Node::VariableAlpha {
                seed,
                src,
                mult,
                feed,
                shape,
            } => {
                vals.feeds[k] = feed.unwrap_or_else(|| {
                    if matches!(arena.nodes[seed as usize], Node::Input { .. }) {
                        0
                    } else {
                        leads[seed as usize]
                    }
                });
                dd = match shape {
                    AlphaShape::Ema => 0,
                    AlphaShape::Vidya => 1,
                };
                (K_VARIABLEALPHA, seed, src, mult)
            }
        };
        sig.kinds[k] = kind;
        sig.a[k] = a;
        sig.b[k] = b;
        sig.c[k] = c;
        sig.d[k] = dd;
        sig.e[k] = ee;
    }
    sig.len = arena.nodes.len() as u32;

    // The published lowering plan: the ring partition, each member's leader
    // and each group's class, derived from the key's own arrays by the ONE
    // function `Signature::build` runs for the generated const. `Values`
    // binds from the same table, so the dispatch key and the runtime binding
    // are one fact seen twice — never a renderer-side re-scan. Capacity
    // (= max member period; the HMA machine's own √p window) stays a VALUE.
    let (class, leader) = crate::cse::lowering::ring_facts(&sig.kinds, &sig.a, sig.len as usize);
    sig.ring_class = class;
    sig.ring_leader = leader;
    for k in 0..sig.len as usize {
        if class[k] == crate::cse::lowering::RCLASS_NONE {
            continue;
        }
        vals.ring_of[k] = leader[k];
        let cap = if sig.kinds[k] == K_HMA {
            vals.lag_n[k]
        } else {
            vals.periods[k]
        };
        vals.ring_cap[leader[k] as usize] = vals.ring_cap[leader[k] as usize].max(cap);
    }

    // Sinks: request order, primaries then enabled optionals (the Results
    // order the JIT uses); effective lead = explicit-or-node-defined-from,
    // clamped up (recipe.rs contract).
    let mut owners = Vec::new();
    let mut s = 0usize;
    for (ri, r) in resolved.iter().enumerate() {
        // Primaries: always enabled
        for (i, sink) in r.outputs.iter().enumerate() {
            put_sink(arena, leads, sink, &mut sig, &mut vals, s);
            vals.sink_want[s] = true;
            owners.push(SinkOwner {
                req: ri,
                primary: true,
                idx: i,
            });
            s += 1;
        }
        // Optionals: enabled only if r.optional_enabled[i]
        for (i, sink) in r.optional_outputs.iter().enumerate() {
            let enabled = r.optional_enabled.get(i).copied().unwrap_or(false);
            vals.sink_want[s] = enabled;
            put_sink(arena, leads, sink, &mut sig, &mut vals, s);
            owners.push(SinkOwner {
                req: ri,
                primary: false,
                idx: i,
            });
            s += 1;
        }
    }
    if s > MAX_SINKS {
        return None;
    }
    sig.n_sinks = s as u32;
    vals.max_gate = vals.sink_lead[..s].iter().copied().max().unwrap_or(0);
    // Lag legality, decided HERE and not in the emitter: the steady loop reads
    // `bar − lag_n[k]` off a pointer already pre-sliced at `gate = max_gate + 1`,
    // so a depth past `max_gate + 1` loads BEFORE bar 0 — that is UB, not a wrong
    // value, and no parity test could see it coming. A shape whose published rows
    // start shallower than one of its own lag reads is therefore rejected here and
    // falls to the next tier, exactly as an over-wide arena does.
    for k in 0..sig.len as usize {
        if sig.kinds[k] == K_LAG && vals.lag_n[k] > vals.max_gate + 1 {
            return None;
        }
    }
    Some((sig, vals, owners))
}

fn put_sink(
    arena: &Merged,
    leads: &[Id],
    sink: &crate::cse::recipe::Sink,
    sig: &mut Signature,
    vals: &mut Values,
    s: usize,
) {
    let SinkSrc::Interface(id) = sink.src;
    let node_lead = leads[id as usize];
    let eff = if sink.lead == 0 {
        node_lead
    } else {
        sink.lead.max(node_lead)
    };
    sig.sink_src[s] = id;
    vals.sink_lead[s] = eff;
    let _ = arena;
}
