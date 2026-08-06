//! Fixed-size, stack-allocated mirror buffer.
//!
//! Maintains two arrays: a classic ring for O(1) writes/lookback and an
//! always-ordered `view` that makes `get_slice` / `get_slice_mut` a single
//! pointer-and-length load with zero heap allocation or pointer indirection.
//!
//! Because every read **and** every in-place mutation (e.g. lazy-bit updates on
//! `CandleBits`) targets the `view` array, updates are never lost across `push`
//! boundaries and `sync_mirrors()` is a genuine no-op.

//use crate::indicators::{max::State as MaxState, min::State as MinState};
use crate::ring_buffer::buffer::{period_to_idx, BufferElement, SerdeElement};
use crate::ring_buffer::single_buffer::generic_buffer::{Cold, Warm};
//use crate::ring_buffer::single_buffer::mirror_buffer::{mirror_max, mirror_min};
use serde::{
    de::{self, MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{fmt, marker::PhantomData};

/// A fixed-capacity, stack-allocated sliding-window buffer with an always-ordered view.
///
/// `S` encodes fill state at the type level:
/// * [`Cold`] — warmup; `push_with_info` returns `Option<T>`.
/// * [`Warm`]    — operational; `push_with_info` returns `T` (branchless), `max`/`min` available.
///
/// # Layout (mirrors field names used by heap-based `Buffer<T>`)
/// ```text
/// ring:  [T; N]   — classic ring; index advances mod N on each push
/// view:  [T; N]   — always-ordered; view[0]=oldest, view[N-1]=newest
/// index: usize    — next write position in ring  (mirrors Buffer::index)
/// count: usize    — valid elements (0 <= count <= N)
/// ```
#[derive(Clone)]
pub struct FixedMirrorBuffer<T: BufferElement, const N: usize, S = Cold> {
    /// Classic ring buffer — `ring[index]` is the next slot to be written.
    pub(crate) ring: [T; N],
    /// Always-ordered view: `view[0]` = oldest, `view[count-1]` = newest.
    pub(crate) view: [T; N],
    /// Next write position in `ring` (advances mod `N`).  Mirrors `Buffer::index`.
    pub(crate) index: usize,
    /// Number of valid elements currently stored (`0 <= count <= N`).
    pub(crate) count: usize,
    pub(crate) state: PhantomData<S>,
}

// ── Shared methods (any fill state) ──────────────────────────────────────────

impl<T: BufferElement, const N: usize, S> FixedMirrorBuffer<T, N, S> {
    /// `true` when the buffer holds exactly `N` elements.
    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.count == N
    }

    /// `true` when the buffer holds no elements.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Number of valid elements currently stored (`0 <= len <= N`).
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.count
    }

    /// The compile-time maximum capacity of this buffer (always `N`).
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Ordered slice of all valid elements: `[oldest .. newest]`.
    #[inline(always)]
    pub fn get_slice(&self) -> &[T] {
        &self.view[..self.count]
    }

    /// Mutable ordered slice of all valid elements.
    ///
    /// Mutations hit `view`, the single authoritative copy, so lazy-bit updates
    /// survive the next `push` without any reconciliation step.
    #[inline(always)]
    pub fn get_slice_mut(&mut self) -> &mut [T] {
        &mut self.view[..self.count]
    }

    /// Ordered slice of the newest `period` elements.
    ///
    /// Returns fewer elements if fewer are stored.
    #[inline(always)]
    pub fn get_slice_by_period(&self, period: usize) -> &[T] {
        if self.count == 0 || period == 0 {
            return &[];
        }
        let take = period.min(self.count);
        &self.view[self.count - take..self.count]
    }

    /// O(1) lookback via the ring.
    ///
    /// `period = 0` → most recently pushed element.
    /// `period = N - 1` → oldest stored element (when full).
    #[inline(always)]
    pub fn get_by_period(&self, period: usize) -> T {
        let idx = period_to_idx(self.index, N, period);
        unsafe { *self.ring.get_unchecked(idx) }
    }

    /// Convert a `view`-slice index into a "bars ago" distance.
    ///
    /// `window_index = count - 1` → `0` (newest).
    /// `window_index = 0` → `count - 1` (oldest).
    #[inline(always)]
    pub fn window_index_to_bars_ago(&self, window_index: usize) -> usize {
        self.count - 1 - window_index
    }

    /// Allocate an ordered `Vec<T>` with elements from oldest to newest.
    ///
    /// Because `view` is always kept in order, this is a simple slice copy.
    pub fn to_ordered_vec(&self) -> Vec<T> {
        self.view[..self.count].to_vec()
    }

    /// Propagate any in-place mutations made via [`get_slice_mut`](Self::get_slice_mut)
    /// back into the `ring` array so that [`get_by_period`](Self::get_by_period)
    /// lookbacks also see the updated values.
    ///
    /// Under normal operation this is not needed: `push` keeps both arrays in sync
    /// and the hot-path only reads through `view`.  Call this if you have mutated
    /// elements via `get_slice_mut` (e.g. written lazy bits) **and** subsequently
    /// need accurate results from `get_by_period`.
    ///
    /// # Complexity  O(N) — one copy per slot.
    pub fn sync_mirrors(&mut self) {
        if self.count == 0 {
            return;
        }
        if self.count < N {
            for i in 0..self.count {
                self.ring[i] = self.view[i];
            }
        } else {
            for i in 0..N {
                self.ring[(self.index + i) % N] = self.view[i];
            }
        }
    }
}

// ── Cold methods ───────────────────────────────────────────────────────────

impl<T: BufferElement, const N: usize> FixedMirrorBuffer<T, N, Cold> {
    /// Create a new, empty buffer. All slots are initialised to `T::default()`.
    #[inline]
    pub fn new() -> Self {
        Self {
            ring: [T::default(); N],
            view: [T::default(); N],
            index: 0,
            count: 0,
            state: PhantomData,
        }
    }

    /// Push a new element, evicting the oldest when full.
    ///
    /// # Complexity
    ///
    /// * `ring` write — O(1).
    /// * `view` update while still filling — O(1) append.
    /// * `view` update once full — O(N) `copy_within` (memmove).
    #[inline(always)]
    pub fn push(&mut self, value: T) {
        self.ring[self.index] = value;
        self.index += 1;
        if self.index == N {
            self.index = 0;
        }

        if self.count < N {
            self.view[self.count] = value;
            self.count += 1;
        } else {
            self.view.copy_within(1.., 0);
            self.view[N - 1] = value;
        }
    }

    /// Push and return the evicted element, if any.
    ///
    /// Returns `Some(evicted)` once the buffer is full, `None` while filling.
    #[inline(always)]
    pub fn push_with_info(&mut self, value: T) -> Option<T> {
        if self.count == N {
            let evicted = unsafe { *self.view.get_unchecked(0) };
            self.push(value);
            Some(evicted)
        } else {
            self.push(value);
            None
        }
    }

    /// Transition to [`Warm`] once `count == N`.
    ///
    /// # Panics (debug builds)
    /// Panics if the buffer is not yet full.
    #[inline(always)]
    pub fn into_full(self) -> FixedMirrorBuffer<T, N, Warm> {
        debug_assert!(
            self.count == N,
            "FixedMirrorBuffer::into_full called on non-full buffer (count={}, N={N})",
            self.count
        );
        FixedMirrorBuffer {
            ring: self.ring,
            view: self.view,
            index: self.index,
            count: self.count,
            state: PhantomData,
        }
    }
}

impl<T: BufferElement, const N: usize> Default for FixedMirrorBuffer<T, N, Cold> {
    fn default() -> Self {
        Self::new()
    }
}

// ── Warm methods ──────────────────────────────────────────────────────────────

impl<T: BufferElement, const N: usize> FixedMirrorBuffer<T, N, Warm> {
    /// Push (branchless — buffer is guaranteed full, no count update needed).
    ///
    /// Writes to the ring, advances the ring head, and shifts `view` left by one
    /// slot so the newest element lands at `view[N-1]`.
    #[inline(always)]
    pub fn push(&mut self, value: T) {
        unsafe { *self.ring.get_unchecked_mut(self.index) = value };
        self.index += 1;
        if self.index == N {
            self.index = 0;
        }
        self.view.copy_within(1.., 0);
        unsafe { *self.view.get_unchecked_mut(N - 1) = value };
    }

    /// Push and return the evicted element (branchless, always evicts).
    ///
    /// The evicted value is `view[0]` before the shift.
    #[inline(always)]
    pub fn push_with_info(&mut self, value: T) -> T {
        let evicted = unsafe { *self.view.get_unchecked(0) };
        self.push(value);
        evicted
    }
}

// ── MinMax on FixedMirrorBuffer<f64, N, Cold> ─────────────────────────────────
//
// Available on `Cold` buffers too — `get_slice_by_period` already handles partial
// fills, clamping `take` to `count`, so the implementation is identical to Warm.

/*impl<const N: usize> FixedMirrorBuffer<f64, N, Cold> {
    /// Rolling maximum over the current (possibly partial) window.
    ///
    /// Delegates to `mirror_max` via `get_slice_by_period`, which clamps the
    /// slice length to however many elements have been pushed so far.
    ///
    /// `CHUNK_SIZE` controls the SIMD width used during rescans (1 = scalar).
    #[inline(always)]
    pub fn max<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MaxState,
        bar: f64,
        period: usize,
    ) -> (f64, usize) {
        mirror_max::<CHUNK_SIZE>(self.get_slice_by_period(period), state, bar, period)
    }

    /// Rolling minimum over the current (possibly partial) window.
    ///
    /// Mirror of [`Self::max`] for minimum tracking.
    #[inline(always)]
    pub fn min<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MinState,
        bar: f64,
        period: usize,
    ) -> (f64, usize) {
        mirror_min::<CHUNK_SIZE>(self.get_slice_by_period(period), state, bar, period)
    }
}*/

// ── MinMax on FixedMirrorBuffer<f64, N, Warm> ─────────────────────────────────
//
// Only available on `Warm` buffers — guarantees `view[..N]` is fully populated
// and `get_slice_by_period(period)` always returns exactly `period` elements.

/*impl<const N: usize> FixedMirrorBuffer<f64, N, Warm> {
    /// Rolling maximum over the current window.
    ///
    /// Identical semantics to [`MinMaxBuffer::max`] on the heap-based `Buffer`:
    /// - Increments `state.trail` each call.
    /// - When `trail >= period`, rescans the full window slice.
    /// - Otherwise, latches `bar` if it is a new maximum.
    ///
    /// `CHUNK_SIZE` controls the SIMD width used during rescans (1 = scalar).
    #[inline(always)]
    pub fn max<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MaxState,
        bar: f64,
        period: usize,
    ) -> (f64, usize) {
        mirror_max::<CHUNK_SIZE>(self.get_slice_by_period(period), state, bar, period)
    }

    /// Rolling minimum over the current window.
    ///
    /// Mirror of [`Self::max`] for minimum tracking.
    #[inline(always)]
    pub fn min<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MinState,
        bar: f64,
        period: usize,
    ) -> (f64, usize) {
        mirror_min::<CHUNK_SIZE>(self.get_slice_by_period(period), state, bar, period)
    }
}*/

// ── Iterator ──────────────────────────────────────────────────────────────────

/// Iterator produced by `(&FixedMirrorBuffer).into_iter()`.
///
/// Yields elements from **newest to oldest** (`buf[0]` first).
/// Reads from the always-ordered `view` array, so in-place mutations via
/// `get_slice_mut` are immediately visible without calling `sync_mirrors`.
pub struct FixedMirrorIter<'a, T: BufferElement, const N: usize, S> {
    buffer: &'a FixedMirrorBuffer<T, N, S>,
    /// Current position expressed as bars-ago (0 = newest).
    pos: usize,
}

impl<'a, T: BufferElement, const N: usize, S> Iterator for FixedMirrorIter<'a, T, N, S> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        if self.pos >= self.buffer.count {
            return None;
        }
        // view[0]=oldest, view[count-1]=newest → bars_ago 0 maps to view[count-1]
        let val = self.buffer.view[self.buffer.count - 1 - self.pos];
        self.pos += 1;
        Some(val)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.buffer.count.saturating_sub(self.pos);
        (remaining, Some(remaining))
    }
}

impl<'a, T: BufferElement, const N: usize, S> ExactSizeIterator for FixedMirrorIter<'a, T, N, S> {}

impl<'a, T: BufferElement, const N: usize, S> IntoIterator for &'a FixedMirrorBuffer<T, N, S> {
    type Item = T;
    type IntoIter = FixedMirrorIter<'a, T, N, S>;

    /// Iterate from newest to oldest (`buf[0]` first).
    #[inline]
    fn into_iter(self) -> FixedMirrorIter<'a, T, N, S> {
        FixedMirrorIter {
            buffer: self,
            pos: 0,
        }
    }
}

impl<T: BufferElement, const N: usize, S> std::ops::Index<usize> for FixedMirrorBuffer<T, N, S> {
    type Output = T;

    /// Index by bars-ago: `buf[0]` is the newest element, `buf[count-1]` is the oldest.
    ///
    /// Reads from the always-ordered `view` array, so mutations via `get_slice_mut`
    /// are visible without calling `sync_mirrors`.
    #[inline]
    fn index(&self, bars_ago: usize) -> &T {
        assert!(
            bars_ago < self.count,
            "index out of bounds: bars_ago {bars_ago} >= count {}",
            self.count
        );
        // view[0]=oldest, view[count-1]=newest
        &self.view[self.count - 1 - bars_ago]
    }
}

// ── Serde ─────────────────────────────────────────────────────────────────────
//
// Hand-rolled rather than #[derive] because serde's derive generates
// `where [T; N]: Serialize` bounds the compiler cannot satisfy for generic N,
// and to go through T::Repr so that non-serde types like Simd<f64, N> work.
//
// `S` carries no runtime data (PhantomData), so the wire format is unchanged.
//
// Serialize  — map each element through T::to_repr, emit as Vec<T::Repr>.
// Deserialize — read Vec<T::Repr>, map through T::from_repr, convert via TryFrom.

impl<T: BufferElement + SerdeElement, const N: usize, S> Serialize for FixedMirrorBuffer<T, N, S> {
    fn serialize<Ser: Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
        let mut state = serializer.serialize_struct("FixedMirrorBuffer", 4)?;
        let ring_repr: Vec<T::Repr> = self.ring.iter().map(|v| T::to_repr(*v)).collect();
        let view_repr: Vec<T::Repr> = self.view.iter().map(|v| T::to_repr(*v)).collect();
        state.serialize_field("ring", &ring_repr)?;
        state.serialize_field("view", &view_repr)?;
        state.serialize_field("index", &self.index)?;
        state.serialize_field("count", &self.count)?;
        state.end()
    }
}

impl<'de, T: BufferElement + SerdeElement, const N: usize, S> Deserialize<'de>
    for FixedMirrorBuffer<T, N, S>
where
    T::Repr: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        const FIELDS: &[&str] = &["ring", "view", "index", "count"];

        enum Field {
            Ring,
            View,
            Index,
            Count,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct FieldVisitor;
                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;
                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str("`ring`, `view`, `index`, or `count`")
                    }
                    fn visit_str<E: de::Error>(self, v: &str) -> Result<Field, E> {
                        match v {
                            "ring" => Ok(Field::Ring),
                            "view" => Ok(Field::View),
                            "index" => Ok(Field::Index),
                            "count" => Ok(Field::Count),
                            _ => Err(de::Error::unknown_field(v, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct FMBVisitor<T, S, const N: usize>(PhantomData<fn() -> (T, S)>);

        impl<'de, T: BufferElement + SerdeElement, const N: usize, S> Visitor<'de> for FMBVisitor<T, S, N>
        where
            T::Repr: Deserialize<'de>,
        {
            type Value = FixedMirrorBuffer<T, N, S>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct FixedMirrorBuffer")
            }

            fn visit_map<V: MapAccess<'de>>(
                self,
                mut map: V,
            ) -> Result<FixedMirrorBuffer<T, N, S>, V::Error> {
                let mut ring: Option<Vec<T::Repr>> = None;
                let mut view: Option<Vec<T::Repr>> = None;
                let mut index: Option<usize> = None;
                let mut count: Option<usize> = None;

                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::Ring => {
                            if ring.is_some() {
                                return Err(de::Error::duplicate_field("ring"));
                            }
                            ring = Some(map.next_value()?);
                        }
                        Field::View => {
                            if view.is_some() {
                                return Err(de::Error::duplicate_field("view"));
                            }
                            view = Some(map.next_value()?);
                        }
                        Field::Index => {
                            if index.is_some() {
                                return Err(de::Error::duplicate_field("index"));
                            }
                            index = Some(map.next_value()?);
                        }
                        Field::Count => {
                            if count.is_some() {
                                return Err(de::Error::duplicate_field("count"));
                            }
                            count = Some(map.next_value()?);
                        }
                    }
                }

                let ring_repr: Vec<T::Repr> =
                    ring.ok_or_else(|| de::Error::missing_field("ring"))?;
                let view_repr: Vec<T::Repr> =
                    view.ok_or_else(|| de::Error::missing_field("view"))?;
                let index = index.ok_or_else(|| de::Error::missing_field("index"))?;
                let count = count.ok_or_else(|| de::Error::missing_field("count"))?;

                let ring_vec: Vec<T> = ring_repr.into_iter().map(T::from_repr).collect();
                let view_vec: Vec<T> = view_repr.into_iter().map(T::from_repr).collect();

                let ring_arr: [T; N] = ring_vec.try_into().map_err(|v: Vec<T>| {
                    de::Error::invalid_length(v.len(), &"ring array of length N")
                })?;
                let view_arr: [T; N] = view_vec.try_into().map_err(|v: Vec<T>| {
                    de::Error::invalid_length(v.len(), &"view array of length N")
                })?;

                Ok(FixedMirrorBuffer {
                    ring: ring_arr,
                    view: view_arr,
                    index,
                    count,
                    state: PhantomData,
                })
            }
        }

        deserializer.deserialize_struct(
            "FixedMirrorBuffer",
            FIELDS,
            FMBVisitor::<T, S, N>(PhantomData),
        )
    }
}
