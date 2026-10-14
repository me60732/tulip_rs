//! The warm-phase machine: `exec::<SIG, WARM>` — the const-generic per-bar
//! interpreter over a [`Signature`]'s SSA arena, embedded by the emitter as
//! EVERY generated kernel's `[0, gate)` prologue (chain seeds, Wilders mean
//! seams, SuperTrend feed idling, roll trails, lead-gated head stores).
//! The steady bars are always the generated straight-line loop; nothing in
//! this crate runs the full series through `exec` any more (the retired
//! `run` wrapper and its bench rows were removed 2026-10-12 — the parity
//! gate is generated-kernel-vs-hand, the hand being the spec).
//!
//! Bit-exact forms (mirroring the hand modules, the spec per doctrine):
//! * `Ema`: `prev.mul_add(inv, src * mult)` — `ema::calc` exactly; seed =
//!   source value at its feed bar (`0` for price-sourced: bar 0, mirroring
//!   `init_state`'s recursion from `real[0]`).
//! * `Wilders` mean-mode: seed = `sum(first p)/p` accumulated in bar order,
//!   then `prev.mul_add(mult, src * inv)`; composite keeps the raw sum.
//! * `Max` = IEEE maxNum via `f64::max` (the hand's own NaN rule).

use crate::cse::signature::{
    Signature, Values, K_ABS, K_ADD, K_CONST, K_DIV, K_EMA, K_FMA, K_HMA, K_IIR, K_IIRCLAMPED,
    K_INPUT, K_LAG, K_LN, K_MAX, K_MIRROR, K_MUL, K_PREV, K_ROLLMAX, K_ROLLMIN, K_ROLLSUM, K_SMA,
    K_SQRT, K_STDDEV, K_SUB, K_SUPERTREND, K_TRAIL, K_WHEREGE, K_WHERELT, K_WILDERS, K_WMA, MAX,
    MAX_SINKS,
};
use crate::ring_buffer::multi_buffer::multi_mirror_buffer::MultiMirrorBuffer;
use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::ring_buffer::single_buffer::mirror_buffer::MirrorBuffer;
use crate::types::Cold;
use std::simd::Simd;

/// Rings for DERIVED-source rolls (`K_ROLLSUM`): one ring per GROUP —
/// interning already collapsed the shared source structurally, the leader
/// slot is `Values::ring_of[k]` and `Values::ring_cap[leader]` the group's
/// max period. The rings are the HAND's OWN `MirrorBuffer<Cold>` (the
/// type the fisher extrema use: every push writes `vals[i]` AND
/// `vals[i+capacity]`, so the window is ALWAYS a contiguous chronological
/// slice), whose `push_with_info` returns the evicted bar from the fill-cap
/// push onward exactly like the plain `Buffer<Cold>` the AO sums use.
/// Group discipline: EXACTLY ONE RING PUSH PER BAR —
/// `feed(g, t, x)` is idempotent within a bar (every roll of the group
/// calls it with the same bar value; the first performs the push and stores
/// the eviction), so reads see the ring AFTER this bar's push on the hand's
/// side. The hand's `calc` — buffer-first. This makes the age read
/// (`at(p)` = `med[t−p]`, the hand's post-push `get_by_period`) and the
/// eviction correct for every arena order of the group's members. M1's
/// resume inherits the hand's serde from the buffer type.
#[derive(Clone)]
pub struct RingSlot {
    buf: MirrorBuffer<Cold>,
    last_t: usize,
    evicted: Option<f64>,
}

#[derive(Clone)]
pub struct RingSet {
    slots: Vec<Option<RingSlot>>,
}

impl Default for RingSet {
    fn default() -> Self {
        Self { slots: Vec::new() }
    }
}

impl RingSet {
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }
    #[inline(always)]
    fn entry(&mut self, g: usize, cap: usize) -> &mut RingSlot {
        while self.slots.len() <= g {
            self.slots.push(None);
        }
        self.slots[g].get_or_insert_with(|| RingSlot {
            buf: MirrorBuffer::new(cap.max(1)),
            last_t: usize::MAX,
            evicted: None,
        })
    }
    /// Feed this bar's value into group `g`'s ring: the bar's first caller
    /// pushes (recording the eviction); later group members are no-ops.
    #[inline(always)]
    pub fn feed(&mut self, g: usize, cap: usize, t: usize, v: f64) {
        let s = self.entry(g, cap);
        if s.last_t != t {
            s.last_t = t;
            s.evicted = s.buf.push_with_info(v);
        }
    }
    /// This bar's eviction for the group, once the fill-cap passed
    /// (`Some`) — the cap-owner roll's leaving bar.
    #[inline(always)]
    pub fn evicted(&self, g: usize) -> Option<f64> {
        match &self.slots[g] {
            Some(s) => s.evicted,
            None => None,
        }
    }
    /// Read `age` bars back from group `g`'s ring AFTER this bar's feed
    /// (`at(g, p)` is the bar leaving a `p`-window — the hand's
    /// post-push `get_by_period(SHORT_PERIOD)`).
    #[inline(always)]
    pub fn at(&self, g: usize, age: usize) -> f64 {
        match &self.slots[g] {
            Some(s) => s.buf.get_by_period(age),
            // Only reachable before any feed; every read bar feeds first.
            None => 0.0,
        }
    }
    /// The group ring's newest `len` values as ONE contiguous,
    /// chronologically-ordered slice (oldest first) — the mirror invariant
    /// exists for this: an extremum rescan walks it with the SAME
    /// branchless `find_*_simd` the pointer rolls run on the raw stream
    /// slice. Callers guarantee the window is complete (`len <= count`) —
    /// the fold region never reads it.
    #[inline(always)]
    pub fn window(&self, g: usize, len: usize) -> &[f64] {
        match &self.slots[g] {
            Some(s) => s.buf.get_slice_by_period(len),
            None => &[],
        }
    }
}

/// The lane kernel's BATCHED ring authority (2026-10-13): the hand's own
/// `MultiMirrorBuffer<L, f64, Cold>` — lane-major, LANE-MAJOR by the user's
/// ruling (`fisher_simd.rs` is the existence proof): one array push per
/// GROUP per bar replaces the L scalar `RingSet::feed` calls (the measured
/// +4–5 % second-store tax and the L× protocol calls were the point), age
/// reads come back as one `[f64; L]`, and the shared window-minus-newest-
/// `offset` accessor feeds per-lane `find_*_simd` rescans. Trail carries
/// live OUTSIDE the buffer (the kernels' `st`/`trail` arrays) — exactly the
/// hand's split, and required: fold-region lanes must not push the buffer.
/// Group discipline is the scalar `RingSet`'s, 1:1: `feed` idempotent per
/// bar, first caller pushes and stashes the eviction, cap owners leave by
/// `evicted_batch()`, siblings by `at_batch()` — reads see the ring AFTER
/// this bar's push (buffer-first, the hand's order).
#[derive(Clone)]
pub struct RingSlotL<const L: usize> {
    buf: MultiMirrorBuffer<L, f64, Cold>,
    last_t: usize,
    evicted: Option<[f64; L]>,
}

#[derive(Clone)]
pub struct RingSetL<const L: usize> {
    slots: Vec<Option<RingSlotL<L>>>,
}

impl<const L: usize> Default for RingSetL<L> {
    fn default() -> Self {
        Self { slots: Vec::new() }
    }
}

impl<const L: usize> RingSetL<L> {
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }
    #[inline(always)]
    fn entry(&mut self, g: usize, cap: usize) -> &mut RingSlotL<L> {
        while self.slots.len() <= g {
            self.slots.push(None);
        }
        self.slots[g].get_or_insert_with(|| RingSlotL {
            buf: MultiMirrorBuffer::<L, f64, Cold>::new(cap.max(1)),
            last_t: usize::MAX,
            evicted: None,
        })
    }
    /// This bar's L-lane values into group `g`: the bar's first caller
    /// pushes (recording the per-lane eviction); later members are no-ops.
    #[inline(always)]
    pub fn feed_batch(&mut self, g: usize, cap: usize, t: usize, v: [f64; L]) {
        let s = self.entry(g, cap);
        if s.last_t != t {
            s.last_t = t;
            s.evicted = s.buf.push_with_info(v);
        }
    }
    /// This bar's per-lane eviction, `Some` once the fill-cap passed — the
    /// cap-owner roll's leaving bar for ALL lanes at once.
    #[inline(always)]
    pub fn evicted_batch(&self, g: usize) -> Option<[f64; L]> {
        match &self.slots[g] {
            Some(s) => s.evicted,
            None => None,
        }
    }
    /// `age` bars back, all L lanes (post-push, the hand's `get_by_period`).
    #[inline(always)]
    pub fn at_batch(&self, g: usize, age: usize) -> [f64; L] {
        match &self.slots[g] {
            Some(s) => s.buf.get_by_period(age),
            None => [0.0; L],
        }
    }
    /// The group's newest `len` values per lane as contiguous,
    /// chronologically-ordered slices (oldest first) — the mirror invariant
    /// makes each lane's TAIL contiguous, so an extremum rescan walks
    /// `wins[lane]` with the SAME branchless `find_*_simd` the pointer rolls
    /// run on the raw stream. Callers guarantee the window is complete
    /// (`len <= count`) — the fold region never reads it.
    #[inline(always)]
    pub fn window(&self, g: usize, len: usize) -> [&[f64]; L] {
        match &self.slots[g] {
            Some(s) => s.buf.get_slices_by_period(len),
            None => std::array::from_fn(|_| {
                let empty: &[f64] = &[];
                empty
            }),
        }
    }
}

/// The lane kernel's ring authority for PURE-SUM groups (2026-10-14): the
/// VECTOR-ELEMENT layout — one ring per group whose SLOTS are whole
/// `Simd<f64, L>` bars, i.e. the hand's own by-assets buffer type
/// (`SimdBuffer<N> = Buffer<_, Simd<f64, N>>`, `cvi_simd.rs`'s `assets`
/// module) held in `Cold` so the eviction is still an `Option` at the seam.
/// Two differences from `RingSetL`, both deliberate:
/// * ONE STORE PER BAR, not two: sums never rescan, so they do not need the
///   mirror's contiguous per-lane tail — the mirror's second write is pure
///   tax (the user's rule: only min/max buffers are mirror buffers; `cci.rs`
///   keeps its `MirrorBuffer` import commented out for exactly this reason).
/// * ONE vector push + ONE vector age read replaces `L` scattered scalar
///   slots and the `[f64; L]` gather/scatter on both sides.
/// LEGALITY: valid only where every lane advances in lockstep — same `t`,
/// same age, same cap — which is what makes the leaving bar a single vector
/// read. A group containing `RollMax`/`RollMin` cannot use this: its rescan
/// trigger is per-lane and fires on different bars (the rescans are disjoint
/// in time), so those groups keep the lane-major `RingSetL` above. Group
/// discipline is unchanged: `feed` is idempotent per bar, the first caller
/// pushes and stashes the eviction, cap owners leave by `evicted`, siblings
/// by `at` — reads see the ring AFTER this bar's push (buffer-first).
#[derive(Clone)]
pub struct SimdRingSlot<const L: usize> {
    buf: Buffer<Cold, Simd<f64, L>>,
    last_t: usize,
    evicted: Option<Simd<f64, L>>,
}

#[derive(Clone)]
pub struct SimdRingSet<const L: usize> {
    slots: Vec<Option<SimdRingSlot<L>>>,
}

impl<const L: usize> Default for SimdRingSet<L> {
    fn default() -> Self {
        Self { slots: Vec::new() }
    }
}

impl<const L: usize> SimdRingSet<L> {
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }
    #[inline(always)]
    fn entry(&mut self, g: usize, cap: usize) -> &mut SimdRingSlot<L> {
        while self.slots.len() <= g {
            self.slots.push(None);
        }
        self.slots[g].get_or_insert_with(|| SimdRingSlot {
            buf: Buffer::<Cold, Simd<f64, L>>::new(cap.max(1)),
            last_t: usize::MAX,
            evicted: None,
        })
    }
    /// This bar's whole-vector value into group `g`: the bar's first caller
    /// pushes (recording the vector eviction); later members are no-ops.
    #[inline(always)]
    pub fn feed(&mut self, g: usize, cap: usize, t: usize, v: Simd<f64, L>) {
        let s = self.entry(g, cap);
        if s.last_t != t {
            s.last_t = t;
            s.evicted = s.buf.push_with_info(v);
        }
    }
    /// This bar's per-group eviction, `Some` once the fill-cap passed.
    #[inline(always)]
    pub fn evicted(&self, g: usize) -> Option<Simd<f64, L>> {
        match &self.slots[g] {
            Some(s) => s.evicted,
            None => None,
        }
    }
    /// `age` bars back, ALL lanes in one vector (post-push, the hand's
    /// `get_by_period`) — legal only under the lockstep invariant above.
    #[inline(always)]
    pub fn at(&self, g: usize, age: usize) -> Simd<f64, L> {
        match &self.slots[g] {
            Some(s) => s.buf.get_by_period(age),
            None => Simd::splat(0.0),
        }
    }
}

/// One trail-rescan roll step — the hand's `max`/`min` `State` logic
/// transcribed literally, `find_max_simd::<4>`/`find_min_simd::<4>` called
/// for the rescan so the tie/plateau rules cannot drift by construction:
/// bars `0..lb` fold (`init_state`: seed at 0, then `>=`/`<=` replaces the
/// extremum and zeroes the trail, else trail += 1); from bar `lb` the calc
/// rule (`trail += 1`; `p <= trail` RESCANs the `lb + 1`-bar window; else
/// the entering bar takes the throne on `>=`/`<=`). `MAXD` selects the
/// direction; p = trigger period, lb = look_back (window = lb + 1 bars).
#[inline(always)]
fn roll_step<const MAXD: bool>(
    src: &[f64],
    t: usize,
    p: usize,
    lb: usize,
    val: &mut f64,
    trail: &mut usize,
) -> f64 {
    if t < lb {
        if t == 0 {
            *val = src[0];
            *trail = 0;
        } else {
            let bar = src[t];
            let hit = if MAXD { bar >= *val } else { bar <= *val };
            if hit {
                *val = bar;
                *trail = 0;
            } else {
                *trail += 1;
            }
        }
        0.0
    } else {
        *trail += 1;
        if p <= *trail {
            let window = &src[t - lb..=t];
            let (v, idx) = if MAXD {
                crate::indicators::max::find_max_simd::<4>(window)
            } else {
                crate::indicators::min::find_min_simd::<4>(window)
            };
            *val = v;
            *trail = lb - idx;
        } else {
            let current = src[t];
            let hit = if MAXD {
                current >= *val
            } else {
                current <= *val
            };
            if hit {
                *val = current;
                *trail = 0;
            }
        }
        *val
    }
}

/// Extremum roll over a DERIVED source (the fourth ring shape, fisher
/// 2026-10-13): same trail semantics as [`roll_step`] — fold compares only
/// the current value, the step rescan triggers at `p <= trail` — and the
/// window reads go through the group ring's MIRROR slice
/// (`window(g, lb+1)` — contiguous, oldest→newest), so the rescan calls
/// the SAME `find_max_simd::<4>`/`find_min_simd::<4>` the pointer roll
/// calls on the stream slice: last-position tie, `trail = lb − idx`,
/// cannot drift by construction. The CALLER has already fed the group this
/// bar (idempotent — sums and extrema over one src share a single push).
fn roll_step_ring<const MAXD: bool>(
    rings: &RingSet,
    g: usize,
    x: f64,
    tr: usize,
    p: usize,
    lb: usize,
    val: &mut f64,
    trail: &mut usize,
) -> f64 {
    if tr < lb {
        if tr == 0 {
            *val = x;
            *trail = 0;
        } else {
            let hit = if MAXD { x >= *val } else { x <= *val };
            if hit {
                *val = x;
                *trail = 0;
            } else {
                *trail += 1;
            }
        }
        0.0
    } else {
        *trail += 1;
        if p <= *trail {
            let window = rings.window(g, lb + 1);
            let (v, idx) = if MAXD {
                crate::indicators::max::find_max_simd::<4>(window)
            } else {
                crate::indicators::min::find_min_simd::<4>(window)
            };
            *val = v;
            *trail = lb - idx;
        } else {
            let hit = if MAXD { x >= *val } else { x <= *val };
            if hit {
                *val = x;
                *trail = 0;
            }
        }
        *val
    }
}

/// The warm-phase machine. `exec::<SIG, WARM>` steps bars `[t0, t1)` of
/// pattern `SIG` against `Values`, writing carries (`st/stb/stc_ub/stlb/
/// sumsq/wsum/trail`), per-bar node values (`v`), and lead-gated stores
/// (`out` when `WARM`). It is NOT a runtime steady path: every generated
/// kernel embeds exactly one `exec::<SIG, true>` call for bars `[0, gate)`
/// — cold-seed semantics (chain seeds, seams like the Wilders mean landing
/// at `feed + p − 1`, SuperTrend idling before its feed, roll trails,
/// per-sink publish gating) live HERE, once, so no warm/steady drift can
/// exist between engines and the correctness bar is the simple one the
/// parity gates use: **generated kernel == hand `::indicator()`** (the hand
/// is the spec; the former full-series `run` oracle was retired with the
/// interpreter-as-referee idea, 2026-10-12). The per-kind arms below also
/// remain the vocabulary reference for what the emitter can lower.
#[inline(always)]
pub fn exec<const SIG: Signature, const WARM: bool>(
    vals: &Values,
    streams: &[&[f64]],
    t0: usize,
    t1: usize,
    st: &mut [f64; MAX],
    stb: &mut [f64; MAX],     // SuperTrend prev_st
    stc_ub: &mut [f64; MAX],  // SuperTrend prev_ub
    stlb: &mut [f64; MAX],    // SuperTrend prev_lb
    sumsq: &mut [f64; MAX],   // Stddev sum-of-squares carry
    wsum: &mut [f64; MAX],    // Wma weighted-sum carry
    trail: &mut [usize; MAX], // RollMax/RollMin trail carry (Trail reads it)
    rings: &mut RingSet,      // K_ROLLSUM group rings (leader-slot keyed)
    // K_HMA's seven accumulators, mirroring `ChainState::Hma` order:
    // [sum1, wsum1, sum2, wsum2, wss, ss, first_diff].
    hacc: &mut [[f64; 7]; MAX],
    v: &mut [f64; MAX],
    out: &mut [*mut f64; MAX_SINKS],
    leads: &[u32; MAX_SINKS],
) {
    // Iteration over the CONST capacity with a const `break` per slot: every
    // iteration's `k` is a literal for the optimizer, so `SIG.kinds[k]` folds
    // to a constant and the `match` collapses to its single arm WITHOUT
    // relying on unroll heuristics (the M0 lesson: LLVM's heuristics refused
    // to unroll this body over the runtime `SIG.len`, leaving the dispatch
    // loop intact — 1.16x SLOWER than hand). `break` bounds it to the real
    // arena; the tail is dead code.
    for t in t0..t1 {
        for k in 0..MAX {
            if k == SIG.len as usize {
                break;
            }
            v[k] = match SIG.kinds[k] {
                K_INPUT => streams[SIG.a[k] as usize][t],
                K_CONST => vals.consts[k],
                K_EMA => {
                    let s = v[SIG.a[k] as usize];
                    let feed = vals.feeds[k] as usize;
                    if t < feed {
                        0.0
                    } else if t == feed {
                        st[k] = s;
                        s
                    } else {
                        st[k] = st[k].mul_add(vals.invms[k], s * vals.mults[k]);
                        st[k]
                    }
                }
                K_WILDERS => {
                    let s = v[SIG.a[k] as usize];
                    let feed = vals.feeds[k] as usize;
                    let p = vals.periods[k] as usize;
                    if t < feed {
                        0.0
                    } else if t - feed < p - 1 {
                        st[k] += s;
                        0.0
                    } else if t - feed == p - 1 && SIG.b[k] == 0 {
                        st[k] = (st[k] + s) / p as f64;
                        st[k]
                    } else if t - feed == p - 1 {
                        st[k] += s;
                        st[k]
                    } else {
                        st[k] = st[k].mul_add(vals.mults[k], s * vals.invms[k]);
                        st[k]
                    }
                }
                K_SUB => v[SIG.a[k] as usize] - v[SIG.b[k] as usize],
                K_ADD => v[SIG.a[k] as usize] + v[SIG.b[k] as usize],
                K_MUL => v[SIG.a[k] as usize] * v[SIG.b[k] as usize],
                K_DIV => v[SIG.a[k] as usize] / v[SIG.b[k] as usize],
                K_MAX => v[SIG.a[k] as usize].max(v[SIG.b[k] as usize]),
                K_ABS => v[SIG.a[k] as usize].abs(),
                K_MIRROR => v[SIG.a[k] as usize],
                K_SQRT => v[SIG.a[k] as usize].sqrt(),
                K_SMA => {
                    // hand `cycle_sma`: `init_state` folds bars `0..p`
                    // (`sum += real[i]` in order), then from bar `p` every
                    // step is `sum += real[i] − real[i−p]` and the value is
                    // `sum · (1/p)`. The window's leaving bar reads the
                    // stream directly (price source only — erase guarantees
                    // it), so no anchor is needed for a whole-series run.
                    let p = vals.periods[k] as usize;
                    let src = streams[SIG.a[SIG.a[k] as usize] as usize];
                    if t < p {
                        st[k] += src[t];
                        0.0
                    } else {
                        st[k] += src[t] - src[t - p];
                        st[k] * vals.mults[k]
                    }
                }
                K_STDDEV => {
                    // hand `stddev::State::calc` op for op: the sma step
                    // first (its own sum carry), then the sum-of-squares
                    // step, then the two-moment variance in the hand's
                    // association (`mul_add` for both differences), and the
                    // flat-bar floor `sqrt(var).max(ε)` — a negative var
                    // from rounding or a flat window is legal data, so the
                    // ε stays (per the `safe_inputs` retirement note).
                    let p = vals.periods[k] as usize;
                    let m = vals.mults[k];
                    let src = streams[SIG.a[SIG.a[k] as usize] as usize];
                    if t < p {
                        let s = src[t];
                        st[k] += s;
                        sumsq[k] = s.mul_add(s, sumsq[k]);
                        0.0
                    } else {
                        let s = src[t];
                        let prev = src[t - p];
                        st[k] += s - prev;
                        let mean = st[k] * m;
                        sumsq[k] += s.mul_add(s, -(prev * prev));
                        let var = sumsq[k].mul_add(m, -(mean * mean));
                        var.sqrt().max(f64::EPSILON)
                    }
                }
                K_WMA => {
                    // hand `wma::State::calc` in its EXACT order: the
                    // weighted line first drops the PRE-update window sum,
                    // then the sum steps, then the entering bar lands at
                    // weight `n`, then the one multiply by the weight
                    // reciprocal `2/(n(n+1))`. `init_state` folds bars
                    // `0..p` with weights `1..=p` (entering index + 1).
                    let p = vals.periods[k] as usize;
                    let n = vals.periods[k] as f64;
                    let src = streams[SIG.a[SIG.a[k] as usize] as usize];
                    if t < p {
                        let value = src[t];
                        st[k] += value;
                        wsum[k] += value * (t + 1) as f64;
                        0.0
                    } else {
                        let value = src[t];
                        let prev = src[t - p];
                        wsum[k] -= st[k];
                        st[k] += value - prev;
                        wsum[k] += value * n;
                        wsum[k] * vals.mults[k]
                    }
                }
                K_ROLLSUM => {
                    // Ring-backed rolling sum over any source (derived is
                    // its point vs `Sma`). EXACTLY ONE RING PUSH PER GROUP
                    // PER BAR: the cap-owning roll (p == cap, one exists by
                    // construction since cap = group max) feeds the ring and
                    // leaves by EVICTION; shorter siblings only age-read
                    // `at(p)` — two pushes a bar would walk the shared ring
                    // at double speed (first bug this arm shipped with).
                    // Four regions, WYSIWYG vs the hand (stoch's slow %K is
                    // the shape that unlocked skip/steady_from):
                    //  fold   t < f+skip            `st += x` — skipped bars
                    //         (feed only)            FEED the ring but never
                    //                               sum; absolute age/evict
                    //                               reads at the first leave
                    //                               bar land on the same
                    //                               window the hand's ring
                    //                               holds (its own ring only
                    //                               starts at f+skip, and the
                    //                               first read is bar f+skip+p)
                    //  seam   t == f+skip+p−1       `st += x; st·mults`
                    //  init   seam < t < steady     `st += x; st −= leave;
                    //                               st·mults` — TWO statements,
                    //                               the hand's init loop
                    //                               verbatim; folding `x −
                    //                               leave` early flips ~60% of
                    //                               the sums' last bits
                    //  steady t >= steady_from      `st += x − leave; st·mults`
                    //                               (hand `sma::calc` order)
                    // steady_from = 0 collapses init into steady — every
                    // pre-stoch shape keeps its exact old behavior.
                    let f = vals.feeds[k] as usize;
                    if t < f {
                        0.0
                    } else {
                        let p = vals.periods[k] as usize;
                        let g = vals.ring_of[k] as usize;
                        let cap = vals.ring_cap[g] as usize;
                        let x = v[SIG.a[k] as usize];
                        rings.feed(g, cap, t, x);
                        let fold = f + vals.skip[k] as usize;
                        if t < fold {
                            0.0
                        } else if t < fold + p - 1 {
                            st[k] += x;
                            0.0
                        } else if t == fold + p - 1 {
                            st[k] += x;
                            st[k] * vals.mults[k]
                        } else {
                            let leave = if p == cap {
                                rings.evicted(g).expect("cap roll steps on an evicting bar")
                            } else {
                                rings.at(g, p)
                            };
                            if (t as u32) < vals.steady_from[k] {
                                st[k] += x;
                                st[k] -= leave;
                                st[k] * vals.mults[k]
                            } else {
                                st[k] += x - leave;
                                st[k] * vals.mults[k]
                            }
                        }
                    }
                }
                K_FMA => v[SIG.a[k] as usize].mul_add(v[SIG.b[k] as usize], v[SIG.c[k] as usize]),
                K_HMA => {
                    // The hand `hma.rs` machine transcribed statement for
                    // statement: stage1 = WMA(n) folding bars `0..n` with
                    // weights `1..=n`; stage2 = WMA(n/2) whose fold is the
                    // hand's OFFSET window `[n−n2, n)` with LOCAL weights
                    // restarting at 1 (this is why decomposition into generic
                    // Wma nodes is not bit-exact); the diff `2·o2 − o1`
                    // (fmul THEN fsub, hand order) feeds the tail: fold bars
                    // `[n, n+m)` run `wss += d·m; ss += d; push; wss −= ss`
                    // with the post-loop `ss −= first_diff` correction on
                    // the last fold bar; warm bars run
                    // `wss += d·m; ss += d; push; out = wss/ww; wss −= ss;
                    // ss −= front()`, `front()` read post-push as
                    // `at(m−1)` (same slot on a full ring). `m = trunc √p` —
                    // integer truncation, like the hand.
                    let n = vals.periods[k] as usize;
                    let n2 = vals.roll_lb[k] as usize;
                    let m = vals.lag_n[k] as usize;
                    let g = vals.ring_of[k] as usize;
                    let x = v[SIG.a[k] as usize];
                    let [mut s1, mut w1, mut s2, mut w2, mut wss, mut ss, mut fd] = hacc[k];
                    let o1 = if t < n {
                        s1 += x;
                        w1 += x * (t + 1) as f64;
                        0.0
                    } else {
                        let old = streams[SIG.a[SIG.a[k] as usize] as usize][t - n];
                        w1 -= s1;
                        s1 += x - old;
                        w1 += x * n as f64;
                        w1 * vals.mults[k]
                    };
                    let o2 = if t < n {
                        if t >= n - n2 {
                            s2 += x;
                            w2 += x * (t - (n - n2) + 1) as f64;
                        }
                        0.0
                    } else {
                        let old = streams[SIG.a[SIG.a[k] as usize] as usize][t - n2];
                        w2 -= s2;
                        s2 += x - old;
                        w2 += x * n2 as f64;
                        w2 * vals.invms[k]
                    };
                    let out = if t < n {
                        0.0
                    } else {
                        let diff = 2.0 * o2 - o1;
                        if t < n + m {
                            wss += diff * vals.st_step[k];
                            ss += diff;
                            rings.feed(g, m, t, diff);
                            wss -= ss;
                            if t == n {
                                fd = diff;
                            }
                            if t == n + m - 1 {
                                ss -= fd;
                            }
                            0.0
                        } else {
                            wss += diff * vals.st_step[k];
                            ss += diff;
                            rings.feed(g, m, t, diff);
                            let out = wss / vals.consts[k];
                            wss -= ss;
                            ss -= rings.at(g, m - 1);
                            out
                        }
                    };
                    hacc[k] = [s1, w1, s2, w2, wss, ss, fd];
                    out
                }
                K_ROLLMAX => {
                    let p = vals.periods[k] as usize;
                    let lb = vals.roll_lb[k] as usize;
                    if SIG.kinds[SIG.a[k] as usize] == K_INPUT {
                        let src = streams[SIG.a[SIG.a[k] as usize] as usize];
                        roll_step::<true>(src, t, p, lb, &mut st[k], &mut trail[k])
                    } else {
                        // derived source: the src column rides the group ring
                        // (feeds gate at `feeds[k]`, the bar the src defines).
                        let f = vals.feeds[k] as usize;
                        if t < f {
                            0.0
                        } else {
                            let g = vals.ring_of[k] as usize;
                            let cap = vals.ring_cap[g] as usize;
                            let x = v[SIG.a[k] as usize];
                            rings.feed(g, cap, t, x);
                            roll_step_ring::<true>(
                                rings,
                                g,
                                x,
                                t - f,
                                p,
                                lb,
                                &mut st[k],
                                &mut trail[k],
                            )
                        }
                    }
                }
                K_ROLLMIN => {
                    let p = vals.periods[k] as usize;
                    let lb = vals.roll_lb[k] as usize;
                    if SIG.kinds[SIG.a[k] as usize] == K_INPUT {
                        let src = streams[SIG.a[SIG.a[k] as usize] as usize];
                        roll_step::<false>(src, t, p, lb, &mut st[k], &mut trail[k])
                    } else {
                        let f = vals.feeds[k] as usize;
                        if t < f {
                            0.0
                        } else {
                            let g = vals.ring_of[k] as usize;
                            let cap = vals.ring_cap[g] as usize;
                            let x = v[SIG.a[k] as usize];
                            rings.feed(g, cap, t, x);
                            roll_step_ring::<false>(
                                rings,
                                g,
                                x,
                                t - f,
                                p,
                                lb,
                                &mut st[k],
                                &mut trail[k],
                            )
                        }
                    }
                }
                K_LN => v[SIG.a[k] as usize].ln(),
                K_IIR => {
                    // `prev·b + src·a` — the Ema op shape with explicit
                    // weights (the fold `mul_add(prev, invm, src·mult)`);
                    // idles below `feeds[k]` with the carry at its zero
                    // init (the hand's state carries; NO seed bar).
                    let s = v[SIG.a[k] as usize];
                    let feed = vals.feeds[k] as usize;
                    if t < feed {
                        0.0
                    } else {
                        st[k] = st[k].mul_add(vals.invms[k], s * vals.mults[k]);
                        st[k]
                    }
                }
                K_IIRCLAMPED => {
                    // hand `calc_fisher` val1 op for op: the ONE mul_add in
                    // the hand's operand order (`mult.mul_add(inner, invm·prev)`
                    // — fma(a, x, round(b·prev)), NOT the Ema-side swap: a/b
                    // here are not exact halves, so association is visible),
                    // then the if/elif clamp folded into the CARRIED value
                    // (the clamped number is what feeds next bar and the ln).
                    let s = v[SIG.a[k] as usize];
                    let feed = vals.feeds[k] as usize;
                    if t < feed {
                        0.0
                    } else {
                        let mut nv = vals.mults[k].mul_add(s, st[k] * vals.invms[k]);
                        if nv > vals.consts[SIG.b[k] as usize] {
                            nv = vals.consts[SIG.c[k] as usize];
                        } else if nv < vals.consts[SIG.d[k] as usize] {
                            nv = vals.consts[SIG.e[k] as usize];
                        }
                        st[k] = nv;
                        nv
                    }
                }
                K_PREV => {
                    // delayed copy (K_PREV doc): emit the slot, then shadow
                    // the src chain's THIS-bar value into it. Arena order
                    // guarantees the src computed before this arm runs.
                    let feed = vals.feeds[k] as usize;
                    if t < feed {
                        0.0
                    } else {
                        let x = st[k];
                        st[k] = v[SIG.a[k] as usize];
                        x
                    }
                }
                K_TRAIL => trail[SIG.a[k] as usize] as f64,
                K_WHEREGE => {
                    if v[SIG.a[k] as usize] >= v[SIG.b[k] as usize] {
                        v[SIG.c[k] as usize]
                    } else {
                        0.0
                    }
                }
                K_WHERELT => {
                    // the hand's denominator-guard form: `<` takes the PINNED
                    // zero, `>=` (and NaN — false both ways) takes the value.
                    if v[SIG.a[k] as usize] < v[SIG.b[k] as usize] {
                        0.0
                    } else {
                        v[SIG.c[k] as usize]
                    }
                }
                K_SUPERTREND => {
                    // SuperTrend per bar — ONE uniform formula transcribing the hand
                    // `calc_st` (indicators/supertrend.rs): prev_st=stb[k],
                    // prev_ub=stc_ub[k], prev_lb=stlb[k], and the carried trend BOOL
                    // in the node's own st[k] slot as 1.0/0.0 (an st node carries no
                    // chain value, so that slot is otherwise unused). The arrays are
                    // zero-initialized, which IS the hand's init_state
                    // (prev_st/prev_ub/prev_lb = 0.0, trend = false) — the first bar
                    // past the seam therefore needs no special seed: the clamp with
                    // the zero bands falls out of the same formula.
                    let lead = vals.feeds[k] as usize;
                    if t < lead {
                        0.0 // idle before the atr seam; carries untouched
                    } else {
                        let atr_v = v[SIG.a[k] as usize];
                        let med_v = v[SIG.b[k] as usize];
                        let close_v = v[SIG.c[k] as usize];
                        let w = vals.st_step[k] * atr_v;
                        let mut ub = med_v + w;
                        let mut lb = med_v - w;
                        let crosses_up = close_v > stb[k];
                        let crosses_down = close_v < stb[k];
                        let trend = crosses_up || (st[k] != 0.0 && !crosses_down);
                        let st_val = if trend {
                            lb = stlb[k].max(lb);
                            lb
                        } else {
                            ub = stc_ub[k].min(ub);
                            ub
                        };
                        st[k] = if trend { 1.0 } else { 0.0 };
                        stb[k] = st_val;
                        stlb[k] = lb;
                        stc_ub[k] = ub;
                        st_val
                    }
                }
                K_LAG => {
                    // price stream (erase guarantees); hand clamp: bars < n
                    // read sample 0 (the E6 bar-0 tr trick depends on it).
                    let nn = vals.lag_n[k] as usize;
                    let src = SIG.a[SIG.a[k] as usize] as usize;
                    streams[src][if t >= nn { t - nn } else { 0 }]
                }
                _ => {
                    debug_assert!(false, "kernel: kind {} outside M0 vocabulary", SIG.kinds[k]);
                    0.0
                }
            };
        }
        for s in 0..MAX_SINKS {
            if s == SIG.n_sinks as usize {
                break;
            }
            let lead = leads[s] as usize;
            if WARM && t < lead {
                continue;
            }
            // Guarded store: only write if sink is enabled (runtime want flag)
            unsafe {
                if vals.sink_want[s] {
                    *out[s].add(t - lead) = v[SIG.sink_src[s] as usize];
                }
            }
        }
    }
}
