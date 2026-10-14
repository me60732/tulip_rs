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
use crate::cse::recipe::{Id, Node, SinkSrc};

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
    // Ring-group leader table: src slot → first roll slot over it (arena
    // order is deterministic, so the groups are a function of the structure).
    let mut ring_leaders: Vec<(Id, usize)> = Vec::new();
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
            Node::Wilders {
                src,
                period,
                composite,
                feed,
            } => {
                let (m, i) = super::multipliers::wilders_multiplier(period as usize);
                vals.mults[k] = m;
                vals.invms[k] = i;
                vals.periods[k] = period;
                vals.feeds[k] = if feed { leads[src as usize] } else { 0 };
                // mean-vs-composite changes emitted values → structure (E12 rule)
                (K_WILDERS, src, composite as u32, 0)
            }
            Node::Sub { a: x, b: y } => (K_SUB, x, y, 0),
            Node::Add { a: x, b: y } => (K_ADD, x, y, 0),
            Node::Mul { a: x, b: y } => (K_MUL, x, y, 0),
            Node::Div { a: x, b: y } => (K_DIV, x, y, 0),
            Node::Max { a: x, b: y } => (K_MAX, x, y, 0),
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
                    // EXTREMUM RING (the fourth ring shape, fisher 2026-10-13):
                    // the rescan window cannot be a stream slice, so the src
                    // column rides the SAME group table as the ring sums —
                    // one push per group per bar, cap = max period across ALL
                    // rolls over the src (sums and extrema share it; a
                    // fisher+ao co-request feeds medprice ONCE). The rescan
                    // reads ages `0..=look_back` (never past the cap).
                    vals.feeds[k] = leads[src as usize];
                    let leader = match ring_leaders.iter().position(|(s, _)| *s == src) {
                        Some(i) => ring_leaders[i].1,
                        None => {
                            ring_leaders.push((src, k));
                            k
                        }
                    };
                    vals.ring_of[k] = leader as u32;
                    vals.ring_cap[leader] = vals.ring_cap[leader].max(period);
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
                // Its diff ring is a group of one, keyed on the node itself.
                vals.ring_of[k] = k as u32;
                vals.ring_cap[k] = m as u32;
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
                // the group table; leave = eviction at the group cap, age read
                // below it (see `ring_cap` docs). `skip`/`steady_from` are
                // VALUES, not structure: one RollSum kind covers both shapes,
                // the emitter's warm regions (fold/seam/init/steady) read the
                // two numbers per bar. Stoch's slow %K was the unlocking pair.
                vals.periods[k] = period;
                vals.mults[k] = if mean { 1.0 / period as f64 } else { 1.0 };
                vals.skip[k] = skip;
                vals.steady_from[k] = steady_from;
                // The fold starts the bar the source first defines.
                vals.feeds[k] = leads[src as usize];
                let leader = match ring_leaders.iter().position(|(s, _)| *s == src) {
                    Some(i) => ring_leaders[i].1,
                    None => {
                        ring_leaders.push((src, k));
                        k
                    }
                };
                vals.ring_of[k] = leader as u32;
                vals.ring_cap[leader] = vals.ring_cap[leader].max(period);
                (K_ROLLSUM, src, 0, 0)
            }
            _ => return None, // bespoke/rolling/feedback: later milestones
        };
        sig.kinds[k] = kind;
        sig.a[k] = a;
        sig.b[k] = b;
        sig.c[k] = c;
        sig.d[k] = dd;
        sig.e[k] = ee;
    }
    sig.len = arena.nodes.len() as u32;

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
