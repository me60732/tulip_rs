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
                // Trail-rescan roll, price-sourced only (same rule as the
                // rolling sums: the rescan window is read straight off the
                // stream; a derived source needs the host anchor).
                if !matches!(arena.nodes[src as usize], Node::Input { .. }) {
                    return None;
                }
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
            Node::Fma { a: x, b: y, c: z } => (K_FMA, x, y, z),
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
            _ => return None, // bespoke/rolling/feedback: later milestones
        };
        sig.kinds[k] = kind;
        sig.a[k] = a;
        sig.b[k] = b;
        sig.c[k] = c;
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
