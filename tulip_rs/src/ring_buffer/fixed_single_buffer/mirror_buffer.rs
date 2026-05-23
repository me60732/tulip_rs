//! Fixed-size, stack-allocated mirror buffer.
//!
//! Maintains two arrays: a classic ring for O(1) writes/lookback and an
//! always-ordered `view` that makes `get_slice` / `get_slice_mut` a single
//! pointer-and-length load with zero heap allocation or pointer indirection.
//!
//! Because every read **and** every in-place mutation (e.g. lazy-bit updates on
//! `CandleBits`) targets the `view` array, updates are never lost across `push`
//! boundaries and `sync_mirrors()` is a genuine no-op.

use crate::ring_buffer::buffer::{period_to_idx, BufferElement};
use serde::{
    de::{self, MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{fmt, marker::PhantomData};

/// A fixed-capacity, stack-allocated sliding-window buffer with an always-ordered view.
///
/// Generic parameters:
/// * `T` — element type; must implement [`BufferElement`].
/// * `N` — compile-time capacity (number of slots).
///
/// # Layout (mirrors field names used by heap-based `Buffer<T>`)
/// ```text
/// ring:  [T; N]   — classic ring; index advances mod N on each push
/// view:  [T; N]   — always-ordered; view[0]=oldest, view[N-1]=newest
/// index: usize    — next write position in ring  (mirrors Buffer::index)
/// count: usize    — valid elements (0 <= count <= N)
/// ```
#[derive(Clone)]
pub struct FixedMirrorBuffer<T: BufferElement, const N: usize> {
    /// Classic ring buffer — `ring[index]` is the next slot to be written.
    ring: [T; N],
    /// Always-ordered view: `view[0]` = oldest, `view[count-1]` = newest.
    view: [T; N],
    /// Next write position in `ring` (advances mod `N`).  Mirrors `Buffer::index`.
    index: usize,
    /// Number of valid elements currently stored (`0 <= count <= N`).
    count: usize,
}

impl<T: BufferElement, const N: usize> FixedMirrorBuffer<T, N> {
    // ── Construction ──────────────────────────────────────────────────────────

    /// Create a new, empty buffer. All slots are initialised to `T::default()`.
    #[inline]
    pub fn new() -> Self {
        Self {
            ring: [T::default(); N],
            view: [T::default(); N],
            index: 0,
            count: 0,
        }
    }

    // ── Queries ───────────────────────────────────────────────────────────────

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

    // ── Writes ────────────────────────────────────────────────────────────────

    /// Push a new element, evicting the oldest when full.
    ///
    /// # Complexity
    ///
    /// * `ring` write — O(1).
    /// * `view` update while still filling — O(1) append.
    /// * `view` update once full — O(N) `copy_within` (memmove). For N = 5 and
    ///   8-byte elements this is a single 32-byte cache-line operation.
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
            Some(unsafe { self.push_with_info_unchecked(value) })
        } else {
            self.push(value);
            None
        }
    }

    /// Push without the fullness check.
    ///
    /// # Safety
    ///
    /// Caller must ensure `is_full() == true`.
    #[inline(always)]
    pub unsafe fn push_unchecked(&mut self, value: T) {
        *self.ring.get_unchecked_mut(self.index) = value;
        self.index += 1;
        if self.index == N {
            self.index = 0;
        }

        self.view.copy_within(1.., 0);
        *self.view.get_unchecked_mut(N - 1) = value;
    }

    /// Push and return the evicted element, without the fullness check.
    ///
    /// # Safety
    ///
    /// Same precondition as [`push_unchecked`](Self::push_unchecked).
    #[inline(always)]
    pub unsafe fn push_with_info_unchecked(&mut self, value: T) -> T {
        let evicted = *self.view.get_unchecked(0);
        self.push_unchecked(value);
        evicted
    }

    // ── Reads ─────────────────────────────────────────────────────────────────

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

    // ── Sync ──────────────────────────────────────────────────────────────────

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

impl<T: BufferElement, const N: usize> Default for FixedMirrorBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

// ── Serde ─────────────────────────────────────────────────────────────────────
//
// Hand-rolled rather than #[derive] because serde's derive generates
// `where [T; N]: Serialize` bounds the compiler cannot satisfy for generic N.
//
// Serialize  — emit both arrays as &[T] slices (always works when T: Serialize).
// Deserialize — read each array field into Vec<T>, convert via TryFrom (Rust 1.59+).

impl<T: BufferElement + Serialize, const N: usize> Serialize for FixedMirrorBuffer<T, N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("FixedMirrorBuffer", 4)?;
        state.serialize_field("ring", self.ring.as_slice())?;
        state.serialize_field("view", self.view.as_slice())?;
        state.serialize_field("index", &self.index)?;
        state.serialize_field("count", &self.count)?;
        state.end()
    }
}

impl<'de, T: BufferElement + Deserialize<'de>, const N: usize> Deserialize<'de>
    for FixedMirrorBuffer<T, N>
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

        struct FMBVisitor<T, const N: usize>(PhantomData<fn() -> T>);

        impl<'de, T: BufferElement + Deserialize<'de>, const N: usize> Visitor<'de> for FMBVisitor<T, N> {
            type Value = FixedMirrorBuffer<T, N>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct FixedMirrorBuffer")
            }

            fn visit_map<V: MapAccess<'de>>(
                self,
                mut map: V,
            ) -> Result<FixedMirrorBuffer<T, N>, V::Error> {
                let mut ring: Option<Vec<T>> = None;
                let mut view: Option<Vec<T>> = None;
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

                let ring_vec: Vec<T> = ring.ok_or_else(|| de::Error::missing_field("ring"))?;
                let view_vec: Vec<T> = view.ok_or_else(|| de::Error::missing_field("view"))?;
                let index = index.ok_or_else(|| de::Error::missing_field("index"))?;
                let count = count.ok_or_else(|| de::Error::missing_field("count"))?;

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
                })
            }
        }

        deserializer.deserialize_struct(
            "FixedMirrorBuffer",
            FIELDS,
            FMBVisitor::<T, N>(PhantomData),
        )
    }
}
