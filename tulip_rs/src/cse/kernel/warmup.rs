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
    Signature, Values, K_ABS, K_ACCUM, K_ADD, K_CONST, K_DIV, K_EMA, K_FMA, K_HIGHPASS, K_HMA,
    K_HOMODYNEDISCRIMINATOR, K_HTINPHASE, K_HTQUADRATURE, K_IIR, K_IIRCLAMPED, K_IIRCLAMPEDPLAIN,
    K_INPUT, K_KVOFLOW, K_LAG, K_LINREGFLOW, K_LN, K_MAX, K_MEANABSDEV, K_MIN, K_MIRROR, K_MUL,
    K_NVI, K_PREV, K_PSAR, K_PVI, K_ROLLAGE, K_ROLLMAX, K_ROLLMIN, K_ROLLSUM, K_SDFTLEAD,
    K_SDFTROTATE, K_SEEDEDEMA, K_SMA, K_SQRT, K_STDDEV, K_SUB, K_SUPERSMOOTHER, K_SUPERTREND,
    K_TRAIL, K_VARIABLEALPHA, K_WHEREGE, K_WHEREGT, K_WHERELE, K_WHERELT, K_WILDERS, K_WMA, MAX,
    MAX_SINKS,
};
use crate::ring_buffer::buffer::BufferElement;
use crate::ring_buffer::multi_buffer::multi_buffer::MultiBuffer;
use crate::ring_buffer::multi_buffer::multi_mirror_buffer::MultiMirrorBuffer;
use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::ring_buffer::single_buffer::mirror_buffer::MirrorBuffer;
use crate::types::{Cold, Warm};
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
    /// Seam transition for the SCALAR steady loop: every group `exec` filled
    /// becomes a `Warm` ring, so no steady bar drives a `Cold` buffer.
    /// `into_full` debug-asserts `is_full` per group, which turns the ring
    /// fullness at `gate` from an unchecked hope (today's never-fired
    /// `.expect("cap roll steps on an evicting bar")`) into a checked fact.
    pub fn into_warm(self) -> WarmMirrorRingSet {
        WarmMirrorRingSet {
            slots: self
                .slots
                .into_iter()
                .map(|s| {
                    s.map(|s| WarmMirrorSlot {
                        buf: s.buf.into_full(),
                        last_t: s.last_t,
                        evicted: 0.0,
                    })
                })
                .collect(),
        }
    }
}

/// A `Warm` scalar ring slot: full by construction, so the eviction is a bare
/// `f64` — no `Option` tag, no fill-state branch.
#[derive(Clone)]
pub struct WarmMirrorSlot {
    buf: MirrorBuffer<Warm, f64>,
    last_t: usize,
    evicted: f64,
}

/// The SCALAR steady-loop authority. Mirror-based (like `RingSet`) because the
/// scalar path's derived extrema walk `window()`; a pure-sum scalar group pays
/// the mirror's second store here, and dropping it means splitting the scalar
/// authority by group class (queued — the lane path already splits that way).
/// Same group discipline as `RingSet`: `feed` idempotent per bar, first caller
/// pushes and takes the eviction, reads see the ring AFTER this bar's push.
#[derive(Clone)]
pub struct WarmMirrorRingSet {
    slots: Vec<Option<WarmMirrorSlot>>,
}

impl WarmMirrorRingSet {
    /// This bar's value into group `g` (no `cap`: capacity is fixed by the
    /// ring the seam filled). First caller of the bar pushes; later members
    /// are no-ops.
    #[inline(always)]
    pub fn feed(&mut self, g: usize, t: usize, v: f64) {
        if let Some(s) = self.slots[g].as_mut() {
            if s.last_t != t {
                s.last_t = t;
                s.evicted = s.buf.push_with_info(v);
            }
        }
    }
    /// This bar's eviction — the cap window's leaving bar. A hole reads `0.0`,
    /// the same convention as `at` below (every read bar feeds first, so a hole
    /// is a construction bug, not a fill-state question). THIS is the branch the
    /// fill-state law exists to delete: the `Cold` authority handed back an
    /// `Option` and the arm carried a per-bar `.expect`.
    #[inline(always)]
    pub fn evicted(&self, g: usize) -> f64 {
        match &self.slots[g] {
            Some(s) => s.evicted,
            None => 0.0,
        }
    }
    /// `age` bars back, post-push (the hand's `get_by_period`).
    #[inline(always)]
    pub fn at(&self, g: usize, age: usize) -> f64 {
        match &self.slots[g] {
            Some(s) => s.buf.get_by_period(age),
            None => 0.0,
        }
    }
    /// The group ring's newest `len` values as ONE contiguous chronological
    /// slice — the derived-extrema rescan walk, unchanged from `Cold`.
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
    /// Seam transition: every group the replay filled becomes a `Warm` ring, so
    /// the steady loop never drives a `Cold` buffer. `into_full` debug-asserts
    /// `is_full` per group — a short fill trips loudly in debug instead of
    /// reading a zero slot.
    pub fn into_warm(self) -> WarmRingSetL<L> {
        WarmRingSetL {
            slots: self
                .slots
                .into_iter()
                .map(|s| {
                    s.map(|s| WarmRingSlotL {
                        buf: s.buf.into_full(),
                        last_t: s.last_t,
                        evicted: [0.0; L],
                    })
                })
                .collect(),
        }
    }
}

/// A `Warm` lane-major ring slot: full by construction, so the per-lane
/// eviction is a bare `[f64; L]` — no `Option` tag, no fill-state branch.
#[derive(Clone)]
pub struct WarmRingSlotL<const L: usize> {
    buf: MultiMirrorBuffer<L, f64, Warm>,
    last_t: usize,
    evicted: [f64; L],
}

/// The steady-loop authority for rescan-capable (extrema) groups. Same
/// lane-major `MultiMirrorBuffer` layout as `RingSetL` — the mirror and the
/// per-lane contiguous window are REQUIRED here, because a rescan fires on
/// different bars per lane — only the fill state changes: `push_with_info`
/// returns `[f64; L]` directly and advances unchecked, so a steady bar pays
/// neither the `Cold` fill branch nor the eviction `Option`.
/// `window` keeps working because `get_slices_by_period` lives in the shared
/// `impl<B, S, T>` block of `MultiMirrorBuffer`, not in the `Cold` one.
#[derive(Clone)]
pub struct WarmRingSetL<const L: usize> {
    slots: Vec<Option<WarmRingSlotL<L>>>,
}

impl<const L: usize> WarmRingSetL<L> {
    /// This bar's L-lane values into group `g`: the bar's first caller pushes
    /// (taking the per-lane eviction); later members are no-ops. No `cap` —
    /// capacity is fixed by the ring the seam filled.
    #[inline(always)]
    pub fn feed_batch(&mut self, g: usize, t: usize, v: [f64; L]) {
        let s = self.slots[g]
            .as_mut()
            .expect("warm lane ring group missing at seam");
        if s.last_t != t {
            s.last_t = t;
            s.evicted = s.buf.push_with_info(v);
        }
    }
    /// This bar's per-lane eviction — the cap roll's leaving bar for ALL
    /// lanes at once.
    #[inline(always)]
    pub fn evicted_batch(&self, g: usize) -> [f64; L] {
        self.slots[g]
            .as_ref()
            .expect("warm lane ring group missing")
            .evicted
    }
    /// `age` bars back, all L lanes (post-push, the hand's `get_by_period`).
    #[inline(always)]
    pub fn at_batch(&self, g: usize, age: usize) -> [f64; L] {
        match &self.slots[g] {
            Some(s) => s.buf.get_by_period(age),
            None => [0.0; L],
        }
    }
    /// The group's newest `len` values per lane as contiguous chronological
    /// slices — the rescan walk, unchanged from the `Cold` authority.
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

/// The lane kernel's FILL-PHASE ring authority for PURE-SUM groups: the
/// VECTOR-ELEMENT layout — one ring per group whose SLOTS are whole
/// `Simd<f64, L>` bars, i.e. the hand's own by-assets buffer type
/// (`SimdBuffer<N> = Buffer<_, Simd<f64, N>>`, `cvi_simd.rs`'s `assets`
/// module). `Cold` here is the FILL phase ONLY: the seam replay pushes the
/// warmed bars in, then [`SimdRingSet::into_warm`] transitions every group to
/// [`WarmRingSet`] before the steady loop runs. No steady bar ever sees a
/// `Cold` buffer (the fill-state law: `Cold` exists to keep unchecked reads
/// out of the language, so a steady loop driving `Cold` pays an `Option` and a
/// branch the type system says it must not have).
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
    /// Transition every filled group into the `Warm` authority that drives the
    /// steady loop. `Buffer::into_full` debug-asserts `is_full`, so this is the
    /// checked seam: a group the replay left short (gate - lo < cap) trips here
    /// in debug instead of silently reading a zero slot in release.
    pub fn into_warm(self) -> WarmRingSet<Simd<f64, L>> {
        WarmRingSet {
            slots: self
                .slots
                .into_iter()
                .map(|s| {
                    s.map(|s| WarmRingSlot {
                        buf: s.buf.into_full(),
                    })
                })
                .collect(),
        }
    }
}

/// A `Warm` ring slot: full by construction. The steady loop drives it through
/// ONE fused `feed_periods` per group per bar, so it carries no `last_t`
/// idempotency guard and no eviction stash — the fused pre-push read returns
/// every member's leave (and the eviction, for the cap owner) directly.
#[derive(Clone)]
pub struct WarmRingSlot<T: BufferElement> {
    buf: Buffer<Warm, T>,
}

/// FUSED-CONTAINER authority — the B axis. One
/// [`MultiBuffer`] carrying the `B` ring groups the planner put in one
/// [`crate::cse::lowering::Bundle`]: `B` value vectors under ONE shared
/// `index`/`capacity`, so a bar costs B stores and ONE cursor advance for the
/// whole bundle instead of B advances, and every member of every group reads
/// its own leave out of the one returned array (`results[series][rank]`).
///
/// Why the groups in a bundle may share one cursor: they are all `RCLASS_SUM`
/// (lockstep — same bar, same age layout) and mutually value-independent (no
/// group reads another's roll output this bar), which is exactly the two
/// conditions the planner publishes. The container is created ONLY for a
/// bundle; a group that stands alone keeps `SimdRingSet`'s plain vector buffer
/// per group, so a shape with nothing to fuse pays nothing for the machinery.
///
/// Unequal group capacities inside one bundle are not a hazard: the buffer is
/// sized by the max, and a member only ever reads its OWN ages (≤ its own
/// group's cap), so the older slots of a shorter series — filled here with
/// whatever `ringw.at` wraps out of its own window — are never read.
pub struct BundleRings<const L: usize, const B: usize> {
    buf: MultiBuffer<B, Simd<f64, L>, Cold>,
}

impl<const L: usize, const B: usize> BundleRings<L, B> {
    #[inline(always)]
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: MultiBuffer::new(capacity.max(1)),
        }
    }
    /// Seam replay: this bar's `B` series values, one advance for all of them.
    #[inline(always)]
    pub fn fill(&mut self, values: [Simd<f64, L>; B]) {
        self.buf.push(values);
    }
    /// The checked seam: `into_full` debug-asserts every vector `is_full`, so
    /// a bundle the replay left short trips here in a debug build instead of
    /// reading a zero slot in release — the same fill-state law the per-group
    /// authorities enforce.
    pub fn into_warm(self) -> WarmBundleRings<L, B> {
        WarmBundleRings {
            buf: self.buf.into_full(),
        }
    }
}

/// The `Warm` side of [`BundleRings`]. Fullness is a type-level fact, so the
/// fused push returns every age directly — no `Option`, no cap argument, no
/// `last_t` guard.
///
/// Budget per bar for the whole bundle: B stores, ONE advance, B × N pre-push
/// age reads (the ONE shared age list is applied to every series, so a series
/// pays reads for its siblings' ages too — `(B−1) × N` extra loads bought with
/// `B−1` fewer advances, which is why the planner caps a bundle's series
/// count). The read at a member's own period `p` is the post-push
/// `get_by_period(p)`, and at `p == capacity` it lands on the slot about to be
/// overwritten — the eviction — so no cap-owner flag and no `+1` adjustment
/// anywhere.
pub struct WarmBundleRings<const L: usize, const B: usize> {
    buf: MultiBuffer<B, Simd<f64, L>, Warm>,
}

impl<const L: usize, const B: usize> WarmBundleRings<L, B> {
    /// Push this bar's value for every series and read back every member's
    /// leaving bar in ONE pass — the bundle's whole per-bar ring step.
    #[inline(always)]
    pub fn feed<const N: usize>(
        &mut self,
        values: [Simd<f64, L>; B],
        ages: [usize; N],
    ) -> [[Simd<f64, L>; N]; B] {
        self.buf.push_with_info_periods(values, ages)
    }
}

/// The steady-loop ring authority (fill-state law: NO steady bar ever touches
/// a `Cold` buffer). One ring per GROUP, element type `T` — `Simd<f64, L>` for
/// the lane kernel's vector-element sums. Because fullness is a type-level
/// fact here, `push_with_info` returns `T` directly: relative to the `Cold`
/// authority this removes ONE branch (the `count == capacity` fill test), the
/// eviction's `Option` tag and its stash width, and the arm-level `.expect`.
/// The slot lookup keeps an `expect` — a hole means the seam never created
/// that group, which is a construction bug, not a fill-state question, and it
/// must fail loudly rather than compute on a silent no-op.
///
/// The cap-owner/sibling split SURVIVES the transition and is not an artifact
/// of `Cold`: with a full ring, `at(cap)` reads the slot just written (this
/// bar's value), so the leaving bar of the cap window is only available as the
/// eviction. Deleting the split means switching to the fused PRE-push
/// `push_with_info_periods`, whose `periods[i]` convention lands on the
/// eviction at `p == cap` — a separate change (the B/N fold).
#[derive(Clone)]
pub struct WarmRingSet<T: BufferElement> {
    slots: Vec<Option<WarmRingSlot<T>>>,
}

impl<T: BufferElement> WarmRingSet<T> {
    /// Push this bar's value for group `g` and read back every member's
    /// leaving bar in ONE pass — the lane kernel's per-group ring step. Cost
    /// per call: one store, one advance, `N` pre-push age reads; NO `last_t`
    /// idempotency branch, NO eviction stash, NO `Option`, because the caller
    /// is the group's single leader (arena order guarantees it precedes every
    /// sibling, so nobody else pushes this bar).
    ///
    /// Ages need no cap adjustment: `period_to_idx` is `index - period - 1`,
    /// so the pre-push read at a member's own period `p` lands on the same
    /// slot the post-push `get_by_period(p)` used, and at `p == cap` it lands
    /// on the slot about to be overwritten — the eviction. That identity is
    /// what lets the cap-owner/sibling split disappear here.
    #[inline(always)]
    pub fn feed_periods<const N: usize>(&mut self, g: usize, v: T, ages: [usize; N]) -> [T; N] {
        self.slots[g]
            .as_mut()
            .expect("warm ring group missing at seam")
            .buf
            .push_with_info_periods(v, ages)
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
    // K_KVOFLOW's own carry: [prev_hlc, cm, prev_hl]. The trend flip-flop
    // bool rides the node's OWN st[k] slot (unaffected by this array) --
    // these three fields previously (incorrectly) borrowed st[k+1..k+3],
    // which are the NEXT arena node's own carry cells.
    kvacc: &mut [[f64; 3]; MAX],
    hp_acc: &mut [[f64; 2]; MAX],
    ss_acc: &mut [[f64; 3]; MAX],
    // K_HOMODYNEDISCRIMINATOR's bespoke, non-shared 38-slot cell (five
    // private rings + six scalars -- see `homodynediscriminator.rs`'s
    // `hd_ring_push`/`hd_kernel`/`hd_apply_discriminator`/`hd_step` doc).
    hd_acc: &mut [[f64; 38]; MAX],
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
                K_SEEDEDEMA => {
                    let feed = vals.feeds[k] as usize;
                    if t < feed {
                        0.0
                    } else if t == feed {
                        let seedv = v[SIG.a[k] as usize];
                        st[k] = seedv;
                        seedv
                    } else {
                        let s = v[SIG.b[k] as usize];
                        st[k] = st[k].mul_add(vals.invms[k], s * vals.mults[k]);
                        st[k]
                    }
                }
                K_VARIABLEALPHA => {
                    // VariableAlpha: carry' = f(carry, value, live w from SIG.c), two
                    // fold spellings selected by SIG.d (0=Ema, 1=Vidya) -- a structural
                    // fact, never a runtime value (same principle K_IIRCLAMPED's d/e
                    // refs already establish). `K_VARIABLEALPHA_EMA`/`_VIDYA` were
                    // never-declared names here -- an irrefutable catch-all BINDING
                    // bug that silently swallowed every match arm below it (see
                    // ENGINE_NOTES.md, 2026-11-03 codegen-recovery entry).
                    let feed = vals.feeds[k] as usize;
                    if t < feed {
                        0.0
                    } else if t == feed {
                        let seedv = v[SIG.a[k] as usize];
                        st[k] = seedv;
                        seedv
                    } else {
                        let s = v[SIG.b[k] as usize];
                        let w = v[SIG.c[k] as usize]; // live weight from mult node
                        st[k] = if SIG.d[k] == 0 {
                            st[k].mul_add(1.0 - w, s * w)
                        } else if t == feed + 1 {
                            // hand `vidya.rs::init_state`'s FIRST published bar is
                            // spelled with plain multiply/add, NOT `mul_add` --
                            // every later bar (`cycle`'s steady `calc`) uses
                            // `mul_add`. A genuine one-bar spelling asymmetry in
                            // the hand itself, not a recipe choice.
                            (s - st[k]) * w + st[k]
                        } else {
                            (s - st[k]).mul_add(w, st[k])
                        };
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
                K_MIN => v[SIG.a[k] as usize].min(v[SIG.b[k] as usize]),
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
                    //
                    // `vidya` mode (feed or switch set — the only callers
                    // that use either field): `vidya.rs::init_state`'s own
                    // loop, not `StddevState::calc`, computes every bar while
                    // this node is warm, in PLAIN arithmetic (no `mul_add`,
                    // no ε floor) — that applies to the t<p accumulate fold
                    // too, not just the t>=p roll, so both branches below
                    // check the same condition.
                    let p = vals.periods[k] as usize;
                    let m = vals.mults[k];
                    let vidya_mode = vals.lag_n[k] != 0 || vals.feeds[k] != vals.periods[k];
                    let src = streams[SIG.a[SIG.a[k] as usize] as usize];
                    if t < p {
                        let s = src[t];
                        st[k] += s;
                        sumsq[k] = if vidya_mode {
                            sumsq[k] + (s * s)
                        } else {
                            s.mul_add(s, sumsq[k])
                        };
                        if t as u32 == vals.feeds[k] {
                            // One-time naive stat (vidya's long_stddev feed
                            // override): the accumulate step above already
                            // completed this bar's addition, so st[k]/sumsq[k]
                            // here equal the hand's naive sum/sum_sq over
                            // `real[0..=t]`, exactly `vidya.rs`'s `init_state`.
                            // hand init_state spelling exactly: plain
                            // multiply/subtract, NOT mul_add, and no
                            // epsilon floor at this one-time naive bar.
                            let sma = st[k] * m;
                            (sumsq[k] * m - sma * (st[k] * m)).sqrt()
                        } else {
                            0.0
                        }
                    } else {
                        let s = src[t];
                        let prev = src[t - p];
                        st[k] += s - prev;
                        let mean = st[k] * m;
                        if vidya_mode && (t as u32) < vals.lag_n[k] {
                            sumsq[k] += (s * s) - (prev * prev);
                            (sumsq[k] * m - mean * (st[k] * m)).sqrt()
                        } else {
                            sumsq[k] += s.mul_add(s, -(prev * prev));
                            let var = sumsq[k].mul_add(m, -(mean * mean));
                            var.sqrt().max(f64::EPSILON)
                        }
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
                K_KVOFLOW => {
                    // Hand `State::calc_vf`: 4-state machine (trend bool encoded as f64,
                    // cm cumulative measure, prev_hlc, prev_hl). First vf seeds both EMAs.
                    let hlc = v[SIG.a[k] as usize] + v[SIG.b[k] as usize] + v[SIG.c[k] as usize];
                    let dm = v[SIG.a[k] as usize] - v[SIG.b[k] as usize];

                    // Hand `init_state`: BEFORE the recurrence loop starts (at
                    // i=1), it precomputes `initial_trend`/`initial_cm` from
                    // bar 0 vs bar 1 directly off the raw high/low/close
                    // slices -- NOT a zero-initialized carry. `feeds[k]` is
                    // exactly that first loop bar (erase()'s `+1`), so seed
                    // here, once, using the raw streams at `t-1` (high/low/
                    // close are literal Input nodes in every registered kvo()
                    // recipe, so SIG.a[<node id>] IS that node's stream index).
                    if t == vals.feeds[k] as usize {
                        let hs = SIG.a[SIG.a[k] as usize] as usize;
                        let ls = SIG.a[SIG.b[k] as usize] as usize;
                        let cs = SIG.a[SIG.c[k] as usize] as usize;
                        let hlc0 = streams[hs][t - 1] + streams[ls][t - 1] + streams[cs][t - 1];
                        let hl0 = streams[hs][t - 1] - streams[ls][t - 1];
                        st[k] = if hlc > hlc0 { 1.0 } else { 0.0 };
                        let cm0 = if hlc != hlc0 { hl0 } else { 0.0 };
                        kvacc[k] = [hlc0, cm0, hl0];
                    }
                    let [prev_hlc, cm, prev_hl] = kvacc[k];

                    // Trend flip-flop logic:
                    // - If trend=false (st[k] < 0.5) AND hlc > prev_hlc: flip to true, cm=prev_hl
                    // - If trend=true (st[k] >= 0.5) AND hlc < prev_hlc: flip to false, cm=prev_hl
                    // - Otherwise: sign from current trend, no cm update
                    let sign;
                    let mut cm = cm;
                    if st[k] < 0.5 && hlc > prev_hlc {
                        st[k] = 1.0;
                        cm = prev_hl;
                        sign = 1.0;
                    } else if st[k] >= 0.5 && hlc < prev_hlc {
                        st[k] = 0.0;
                        cm = prev_hl;
                        sign = -1.0;
                    } else {
                        sign = if st[k] >= 0.5 { 1.0 } else { -1.0 };
                    }
                    cm += dm.max(f64::EPSILON);

                    // Update state: prev_hlc = hlc, prev_hl = dm
                    kvacc[k] = [hlc, cm, dm];

                    // vf = ((dm / cm) * 2.0 - 1.0).abs() * volume * 100.0 * sign
                    let vf =
                        (dm / cm).mul_add(2.0, -1.0).abs() * v[SIG.d[k] as usize] * 100.0 * sign;
                    vf
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
                K_IIRCLAMPEDPLAIN => {
                    // Plain IIR clamped recurrence: `nv = a * src + b * prev`
                    // (separate mul/add, NO fused mul_add), then clamp with same
                    // thresholds/values as IirClamped. Used by ccfisher val1.
                    let s = v[SIG.a[k] as usize];
                    let feed = vals.feeds[k] as usize;
                    if t < feed {
                        0.0
                    } else {
                        let mut nv = vals.mults[k] * s + vals.invms[k] * st[k];
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
                K_HIGHPASS => {
                    let real = v[SIG.a[k] as usize];
                    let [y1, prev_real] = hp_acc[k];
                    let y = vals.mults[k].mul_add(y1, vals.invms[k] * (real - prev_real));
                    hp_acc[k] = [y, real];
                    y
                }
                K_SUPERSMOOTHER => {
                    let real = v[SIG.a[k] as usize];
                    let [y1, y2, prev_real] = ss_acc[k];
                    let y = vals.st_step[k].mul_add(
                        real + prev_real,
                        vals.mults[k].mul_add(y1, vals.invms[k] * y2),
                    );
                    ss_acc[k] = [y, y1, real];
                    y
                }
                K_HTINPHASE => {
                    let f = vals.feeds[k] as usize;
                    if t < f {
                        0.0
                    } else {
                        let g = vals.ring_of[k] as usize;
                        let cap = vals.ring_cap[g] as usize;
                        let x = v[SIG.a[k] as usize];
                        rings.feed(g, cap, t, x);
                        rings.at(g, 3)
                    }
                }
                K_HTQUADRATURE => {
                    let f = vals.feeds[k] as usize;
                    if t < f {
                        0.0
                    } else {
                        let g = vals.ring_of[k] as usize;
                        let cap = vals.ring_cap[g] as usize;
                        let x = v[SIG.a[k] as usize];
                        rings.feed(g, cap, t, x);
                        let q1 = 0.0962_f64.mul_add(rings.at(g, 0), 0.5769 * rings.at(g, 2));
                        let q2 = (-0.5769_f64).mul_add(rings.at(g, 4), -0.0962 * rings.at(g, 6));
                        q1 + q2
                    }
                }
                K_HOMODYNEDISCRIMINATOR => {
                    use crate::indicators::homodynediscriminator::{
                        hd_apply_discriminator, hd_kernel, hd_ring_at, hd_ring_push,
                    };
                    let x = v[SIG.a[k] as usize];
                    let acc = &mut hd_acc[k];
                    hd_ring_push(acc, 0, 4, x);
                    if t < 3 {
                        0.0
                    } else {
                        let ab =
                            4.0_f64.mul_add(hd_ring_at(acc, 0, 0), 3.0 * hd_ring_at(acc, 0, 1));
                        let cd = 2.0_f64.mul_add(hd_ring_at(acc, 0, 2), hd_ring_at(acc, 0, 3));
                        let smooth = (ab + cd) * 0.1;
                        let gain = 0.075_f64.mul_add(acc[36], 0.54);
                        hd_ring_push(acc, 4, 7, smooth);
                        if t < 9 {
                            0.0
                        } else {
                            let detrender = hd_kernel(acc, 4, gain);
                            hd_ring_push(acc, 11, 7, detrender);
                            if t < 15 {
                                0.0
                            } else {
                                let i1 = hd_ring_at(acc, 11, 3);
                                let q1 = hd_kernel(acc, 11, gain);
                                hd_ring_push(acc, 18, 7, i1);
                                hd_ring_push(acc, 25, 7, q1);
                                if t < 21 {
                                    0.0
                                } else {
                                    let j_i = hd_kernel(acc, 18, gain);
                                    let j_q = hd_kernel(acc, 25, gain);
                                    let smooth_period =
                                        hd_apply_discriminator(acc, i1, q1, j_i, j_q);
                                    let f = vals.feeds[k] as usize;
                                    if t < f {
                                        0.0
                                    } else {
                                        smooth_period
                                    }
                                }
                            }
                        }
                    }
                }
                K_TRAIL => trail[SIG.a[k] as usize] as f64,
                K_ROLLAGE => {
                    let f = vals.feeds[k] as usize;
                    if t < f {
                        0.0
                    } else {
                        let _n = vals.periods[k] as usize;
                        let g = vals.ring_of[k] as usize;
                        let cap = vals.ring_cap[g] as usize;
                        let x = v[SIG.a[k] as usize];
                        rings.feed(g, cap, t, x);
                        rings.evicted(g).unwrap_or(0.0)
                    }
                }
                K_NVI => {
                    let f = vals.feeds[k] as usize;
                    if t < f {
                        0.0
                    } else {
                        if t == f {
                            st[k] = 1000.0;
                        }
                        let volume = v[SIG.c[k] as usize];
                        let prev_volume = v[SIG.d[k] as usize];
                        if volume < prev_volume {
                            let close = v[SIG.a[k] as usize];
                            let prev_close = v[SIG.b[k] as usize];
                            st[k] = close / prev_close * st[k];
                        }
                        st[k]
                    }
                }
                K_PVI => {
                    let f = vals.feeds[k] as usize;
                    if t < f {
                        0.0
                    } else {
                        if t == f {
                            st[k] = 1000.0;
                        }
                        let volume = v[SIG.c[k] as usize];
                        let prev_volume = v[SIG.d[k] as usize];
                        if volume > prev_volume {
                            let close = v[SIG.a[k] as usize];
                            let prev_close = v[SIG.b[k] as usize];
                            st[k] = close / prev_close * st[k];
                        }
                        st[k]
                    }
                }
                K_MEANABSDEV => {
                    let f = vals.feeds[k] as usize;
                    if t < f {
                        0.0
                    } else {
                        let p = vals.periods[k] as usize;
                        let sma = v[SIG.b[k] as usize];
                        let mult = 1.0 / p as f64;
                        if SIG.kinds[SIG.a[k] as usize] == K_INPUT {
                            let stream_idx = SIG.a[SIG.a[k] as usize] as usize;
                            let src = streams[stream_idx];
                            let start = t + 1 - p;
                            crate::indicators::md::calc_md_simd::<4>(&src[start..=t], sma, mult)
                        } else {
                            let g = vals.ring_of[k] as usize;
                            let cap = vals.ring_cap[g] as usize;
                            let x = v[SIG.a[k] as usize];
                            rings.feed(g, cap, t, x);
                            let window = rings.window(g, p);
                            crate::indicators::md::calc_md_simd::<4>(window, sma, mult)
                        }
                    }
                }
                K_LINREGFLOW => {
                    // linreg's moment carry (`Node::LinregFlow` doc). Steady is
                    // the 3-op form, publish BEFORE the evict — the shape no
                    // `RollSum` region spells, measured bit-exact against the
                    // hand on 24 series×period cells.
                    // Below the seam the hand's `init_state` folds with a
                    // BAR-INDEX weight capped at the steady multiplier, and
                    // never evicts. `min(t, mult)` covers both instances with
                    // no branch: `sum_xy` (mult = period) gets weight t, which
                    // IS the value's absolute bar index (`sum_xy += (i+1)·d`
                    // with d = real[1+i]); `sum_y` (mult = 1.0) gets weight 1
                    // from bar 1 on. Bar 0 folds `src · 0.0` — the hand works on
                    // `real[1..]` and never reads it; sound for a FINITE bar 0,
                    // the parity matrices' contract.
                    let f = vals.feeds[k] as usize;
                    let x = v[SIG.a[k] as usize];
                    let m = vals.mults[k];
                    if t < f {
                        st[k] += x * (t as f64).min(m);
                        0.0
                    } else {
                        let out = st[k] + x * m;
                        st[k] = out - v[SIG.b[k] as usize];
                        out
                    }
                }
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
                K_WHEREGT => {
                    if v[SIG.a[k] as usize] > v[SIG.b[k] as usize] {
                        v[SIG.c[k] as usize]
                    } else {
                        0.0
                    }
                }
                K_WHERELE => {
                    // An EARLY-RETURN guard: the hand computes whenever
                    // `a <= b` is FALSE, and a NaN makes it false too — so the
                    // value arm (and its poison) runs. `K_WHEREGT` would pin
                    // `0.0` here and silently un-poison.
                    if v[SIG.a[k] as usize] <= v[SIG.b[k] as usize] {
                        0.0
                    } else {
                        v[SIG.c[k] as usize]
                    }
                }
                K_ACCUM => {
                    // Single-rounding running total: ONE fma and NO weights —
                    // the multiplier is a NODE (volume), which is why `K_IIR`
                    // cannot spell it. Idles below `feeds[k]` with the carry at
                    // its zero init, exactly like `K_IIR`.
                    let feed = vals.feeds[k] as usize;
                    if t < feed {
                        0.0
                    } else {
                        st[k] = v[SIG.a[k] as usize].mul_add(v[SIG.b[k] as usize], st[k]);
                        st[k]
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
                K_PSAR => {
                    // Parabolic SAR (`Node::Psar` doc): bar 1 is the seed
                    // (hand `State::new`/`calc` at i=1, which skips the
                    // lookback-2 clamp `calc_unchecked` applies from bar 2
                    // on); every bar after uses the full clamp formula.
                    let afs = vals.mults[k];
                    let maxaf = vals.invms[k];
                    let hs = SIG.a[SIG.a[k] as usize] as usize;
                    let ls = SIG.a[SIG.b[k] as usize] as usize;
                    if t < 1 {
                        0.0
                    } else if t == 1 {
                        let (uptrend0, extream0, psar0) =
                            if streams[hs][0] + streams[ls][0] <= streams[hs][1] + streams[ls][1] {
                                (true, streams[hs][0], streams[ls][0])
                            } else {
                                (false, streams[ls][0], streams[hs][0])
                            };
                        let mut psar = afs.mul_add(extream0 - psar0, psar0);
                        let mut extream = extream0;
                        let mut accel = afs;
                        let mut uptrend = uptrend0;
                        if uptrend {
                            if psar > streams[ls][0] {
                                psar = streams[ls][0];
                            }
                            if streams[hs][1] > extream {
                                extream = streams[hs][1];
                                accel = (accel + afs).min(maxaf);
                            }
                        } else {
                            if psar < streams[hs][0] {
                                psar = streams[hs][0];
                            }
                            if streams[ls][1] < extream {
                                extream = streams[ls][1];
                                accel = (accel + afs).min(maxaf);
                            }
                        }
                        if (uptrend && streams[ls][1] < psar) || (!uptrend && streams[hs][1] > psar)
                        {
                            uptrend = !uptrend;
                            psar = extream;
                            accel = afs;
                            extream = if uptrend {
                                streams[hs][1]
                            } else {
                                streams[ls][1]
                            };
                        }
                        st[k] = if uptrend { 1.0 } else { 0.0 };
                        kvacc[k] = [psar, extream, accel];
                        psar
                    } else {
                        let [mut psar, mut extream, mut accel] = kvacc[k];
                        let mut uptrend = st[k] != 0.0;
                        let h = streams[hs][t];
                        let ph = streams[hs][t - 1];
                        let oh = streams[hs][t - 2];
                        let l = streams[ls][t];
                        let pl = streams[ls][t - 1];
                        let ol = streams[ls][t - 2];
                        psar = accel.mul_add(extream - psar, psar);
                        if uptrend {
                            if psar > ol {
                                psar = ol;
                            }
                            if psar > pl {
                                psar = pl;
                            }
                            if h > extream {
                                extream = h;
                                accel = (accel + afs).min(maxaf);
                            }
                        } else {
                            if psar < oh {
                                psar = oh;
                            }
                            if psar < ph {
                                psar = ph;
                            }
                            if l < extream {
                                extream = l;
                                accel = (accel + afs).min(maxaf);
                            }
                        }
                        if (uptrend && l < psar) || (!uptrend && h > psar) {
                            uptrend = !uptrend;
                            psar = extream;
                            accel = afs;
                            extream = if uptrend { h } else { l };
                        }
                        st[k] = if uptrend { 1.0 } else { 0.0 };
                        kvacc[k] = [psar, extream, accel];
                        psar
                    }
                }
                K_SDFTROTATE => {
                    // Sliding DFT rotation carry for `msw` (`Node::SdftRotate`
                    // doc). Seed: a full weighted dot-product over the first
                    // `period` samples gives the PRE-fold state (rp0, ip0) --
                    // calling the hand's OWN `calc_rp_ip` verbatim so this
                    // part is bit-identical by construction. The hand's
                    // `cycle_sdft` loop then applies ONE rotation step AT the
                    // first output bar too (`state.calc((real[period],
                    // real[0]))`) -- it never publishes the raw seed phase.
                    // So every bar from `f` on (inclusive) runs the exact
                    // same fold; only the PRIOR (rp, ip) differs: the seed
                    // pair on bar `f`, `kvacc[k]` on every bar after.
                    let f = vals.feeds[k] as usize;
                    if t < f {
                        0.0
                    } else {
                        let s = SIG.a[SIG.a[k] as usize] as usize;
                        let period = vals.periods[k] as usize;
                        let (rp, ip) = if t == f {
                            let window = &streams[s][f - period..f];
                            let mult = 1.0 / period as f64;
                            if period <= 7 {
                                crate::indicators::msw::calc_rp_ip::<4>(window, mult)
                            } else {
                                crate::indicators::msw::calc_rp_ip::<8>(window, mult)
                            }
                        } else {
                            let [rp, ip, _lead] = kvacc[k];
                            (rp, ip)
                        };
                        let wr = vals.mults[k];
                        let wi = vals.invms[k];
                        let new = streams[s][t];
                        let old = streams[s][t - period];
                        let rp2 = wr.mul_add(rp, -(wi * ip)) + (new - old);
                        let ip2 = wr.mul_add(ip, wi * rp);
                        let (sine, lead) = crate::indicators::msw::phase_from_rp_ip(rp2, ip2);
                        kvacc[k] = [rp2, ip2, lead];
                        sine
                    }
                }
                K_SDFTLEAD => {
                    // Thin companion read of its owner's cached lead_sine
                    // (`Node::SdftLead` doc). The owner always runs first in
                    // this same bar's k-loop (a real graph edge), so
                    // kvacc[owner_k] is always fresh by the time this reads it.
                    let f = vals.feeds[k] as usize;
                    if t < f {
                        0.0
                    } else {
                        let owner_k = SIG.a[k] as usize;
                        kvacc[owner_k][2]
                    }
                }
                crate::cse::signature::K_CYBERCYCLE => {
                    // Cybercycle recurrence (`Node::Cybercycle` doc).
                    // Warmup: bars 0–5 follow the hand's init_state path exactly.
                    // First published bar is 6 (min_data=7), so feed = lead = 6.
                    let f = vals.feeds[k] as usize;
                    if t < f {
                        // Warmup bars 0-5: seed the cycles using second-difference formula.
                        // After bar 5, kvacc = [Cycle[5], Cycle[4]].
                        let s = SIG.a[SIG.a[k] as usize] as usize;

                        if t == 0 {
                            // Initialize to zero
                            kvacc[k] = [0.0, 0.0, 0.0];
                        } else if t >= 2 {
                            // Second-difference seeding: (P[t-2] - 2*P[t-1] + P[t]) / 4
                            let p_t = streams[s][t];
                            let p_t_1 = if t >= 1 {
                                streams[s][t - 1]
                            } else {
                                streams[s][0]
                            };
                            let p_t_2 = if t >= 2 {
                                streams[s][t - 2]
                            } else {
                                streams[s][0]
                            };
                            let seed = (p_t_2 - 2.0 * p_t_1 + p_t) / 4.0;

                            // Update carry: [Cycle[t], Cycle[t-1], 0.0]
                            kvacc[k] = [seed, kvacc[k][0], 0.0];
                        }
                        0.0 // No output until bar 6
                    } else {
                        let s = SIG.a[SIG.a[k] as usize] as usize;
                        let coef = vals.mults[k];
                        // Store d1 in invms, compute d2 from it: d1=2b, d2=b², so b=d1/2, d2=(d1/2)²
                        let d1 = vals.invms[k];
                        let d2 = (d1 / 2.0) * (d1 / 2.0);
                        // After warmup at bar 5, kvacc holds [Cycle[5], Cycle[4], 0.0].
                        // Read carry values.
                        let [cycle_prev, cycle_prev2, _smooth_temp] = kvacc[k];

                        // Read current and previous prices for 6-tap weighted smooth.
                        let price = streams[s][t];
                        let p2 = if t >= 2 {
                            streams[s][t - 2]
                        } else {
                            streams[s][0]
                        };
                        let p3 = if t >= 3 {
                            streams[s][t - 3]
                        } else {
                            streams[s][0]
                        };

                        // Smooth(t) = (P[t] + 2*P[t-1] + 2*P[t-2] + P[t-3]) / 6
                        let price_1 = if t >= 1 {
                            streams[s][t - 1]
                        } else {
                            streams[s][0]
                        };
                        let smooth_ab = 2.0_f64.mul_add(price_1, price);
                        let smooth_cd = 2.0_f64.mul_add(p2, p3);
                        let smooth = (smooth_ab + smooth_cd) * (1.0 / 6.0);

                        // Prices needed for smooth(t-1) and smooth(t-2):
                        let p4 = if t >= 4 {
                            streams[s][t - 4]
                        } else {
                            streams[s][0]
                        };
                        let p5 = if t >= 5 {
                            streams[s][t - 5]
                        } else {
                            streams[s][0]
                        };

                        // Smooth(t-1) = (P[t-1] + 2*P[t-2] + 2*P[t-3] + P[t-4]) / 6
                        let smooth_1_ab = 2.0_f64.mul_add(p2, price_1);
                        let smooth_1_cd = 2.0_f64.mul_add(p3, p4);
                        let smooth_1 = (smooth_1_ab + smooth_1_cd) * (1.0 / 6.0);

                        // Smooth(t-2) = (P[t-2] + 2*P[t-3] + 2*P[t-4] + P[t-5]) / 6
                        let smooth_2_ab = 2.0_f64.mul_add(p3, p2);
                        let smooth_2_cd = 2.0_f64.mul_add(p4, p5);
                        let smooth_2 = (smooth_2_ab + smooth_2_cd) * (1.0 / 6.0);

                        let smooth_diff = (-2.0_f64).mul_add(smooth_1, smooth) + smooth_2;
                        let cycle =
                            coef.mul_add(smooth_diff, d1.mul_add(cycle_prev, -d2 * cycle_prev2));

                        // Update carry: [Cycle[t], Cycle[t-1], 0.0]
                        kvacc[k] = [cycle, cycle_prev, 0.0];
                        cycle
                    }
                }
                crate::cse::signature::K_PEAKDECAY => {
                    // Peak decay envelope for trendmode/ccfisher.
                    // `peak = max(prev_peak * decay, src)` with zero init.
                    // No warmup bars; first output at bar 0 (feed = src_lead).
                    let f = vals.feeds[k] as usize;
                    if t < f {
                        0.0
                    } else {
                        let decay = vals.mults[k];
                        let src = v[SIG.a[k] as usize]; // Use computed value (works for both Input and derived)
                        st[k] = (st[k] * decay).max(src);
                        st[k]
                    }
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
