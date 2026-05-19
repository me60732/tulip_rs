//! Fixed-size, stack-allocated ring buffer (no mirroring).
//!
//! A single `vals: [T; N]` array advanced by an `index` pointer, matching the
//! field names and semantics of the heap-based `Buffer<T>` / `RingBuffer` pair.
//!
//! Unlike [`FixedMirrorBuffer`](super::FixedMirrorBuffer), there is no always-ordered
//! `view` array.  `get_slice()` returns the raw underlying storage (unordered once
//! the buffer wraps); use `to_ordered_vec()` or `get_by_period()` when order matters.
//!
//! Use this type when you need a fixed-capacity ring with O(1) lookback but do
//! **not** need a contiguous ordered window on the hot path.

use crate::ring_buffer::buffer::{period_to_idx, BufferElement};
use serde::{
    de::{self, MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{fmt, marker::PhantomData};

/// A fixed-capacity, stack-allocated ring buffer without a mirrored view.
///
/// Generic parameters:
/// * `T` — element type; must implement [`BufferElement`].
/// * `N` — compile-time capacity (number of slots).
///
/// # Layout (field names mirror heap-based `Buffer<T>`)
/// ```text
/// vals:  [T; N]   — ring storage; vals[index] is the next slot to write
/// index: usize    — next write position (advances mod N)
/// count: usize    — valid elements (0 <= count <= N)
/// ```
#[derive(Clone)]
pub struct FixedRingBuffer<T: BufferElement, const N: usize> {
    /// Ring storage — `vals[index]` is the next slot to be written.
    vals: [T; N],
    /// Next write position (advances mod `N`).  Mirrors `Buffer::index`.
    index: usize,
    /// Number of valid elements currently stored (`0 <= count <= N`).
    count: usize,
}

impl<T: BufferElement, const N: usize> FixedRingBuffer<T, N> {
    // ── Construction ──────────────────────────────────────────────────────────

    /// Create a new, empty buffer. All slots are initialised to `T::default()`.
    #[inline]
    pub fn new() -> Self {
        Self {
            vals: [T::default(); N],
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
    #[inline(always)]
    pub fn push(&mut self, value: T) {
        self.vals[self.index] = value;
        self.index = (self.index + 1) % N;
        if self.count < N {
            self.count += 1;
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
        *self.vals.get_unchecked_mut(self.index) = value;
        self.index = (self.index + 1) % N;
    }

    /// Push and return the evicted element, without the fullness check.
    ///
    /// # Safety
    ///
    /// Same precondition as [`push_unchecked`](Self::push_unchecked).
    #[inline(always)]
    pub unsafe fn push_with_info_unchecked(&mut self, value: T) -> T {
        let evicted = *self.vals.get_unchecked(self.index);
        self.push_unchecked(value);
        evicted
    }

    // ── Reads ─────────────────────────────────────────────────────────────────

    /// Raw underlying storage slice.
    ///
    /// Elements are in ring order — **not** guaranteed to be oldest-first once the
    /// buffer has wrapped.  For an ordered snapshot use [`to_ordered_vec`](Self::to_ordered_vec).
    /// While still filling (`count < N`) the slice `vals[..count]` is in insertion order.
    #[inline(always)]
    pub fn get_slice(&self) -> &[T] {
        if self.count < N {
            &self.vals[..self.count]
        } else {
            &self.vals
        }
    }

    /// Newest element, or `None` if empty.
    #[inline(always)]
    pub fn back(&self) -> Option<T> {
        if self.count == 0 {
            return None;
        }
        // The slot written most recently is one behind index (mod N).
        let prev = (self.index + N - 1) % N;
        Some(unsafe { *self.vals.get_unchecked(prev) })
    }

    /// Oldest element, or `None` if empty.
    #[inline(always)]
    pub fn front(&self) -> Option<T> {
        if self.count == 0 {
            return None;
        }
        // When full, index points at the oldest slot (about to be overwritten).
        // When still filling, slot 0 is the oldest.
        let oldest = if self.count == N { self.index } else { 0 };
        Some(unsafe { *self.vals.get_unchecked(oldest) })
    }

    /// O(1) lookback.
    ///
    /// `period = 0` → most recently pushed element.
    /// `period = N - 1` → oldest stored element (when full).
    #[inline(always)]
    pub fn get_by_period(&self, period: usize) -> T {
        let idx = period_to_idx(self.index, N, period);
        unsafe { *self.vals.get_unchecked(idx) }
    }

    /// Allocate an ordered `Vec<T>` with elements from oldest to newest.
    pub fn to_ordered_vec(&self) -> Vec<T> {
        if self.count == 0 {
            return Vec::new();
        }
        if self.count < N {
            return self.vals[..self.count].to_vec();
        }
        // Full: oldest is at `index`, wraps around.
        let mut out = Vec::with_capacity(N);
        out.extend_from_slice(&self.vals[self.index..]);
        if self.index > 0 {
            out.extend_from_slice(&self.vals[..self.index]);
        }
        out
    }

    /// Allocate an ordered `Vec<T>` of the newest `period` elements (oldest-first).
    pub fn to_ordered_by_period(&self, period: usize) -> Vec<T> {
        if self.count == 0 || period == 0 {
            return Vec::new();
        }
        let take = period.min(self.count);
        // bars_ago = take-1 is the oldest of the window, bars_ago = 0 is the newest.
        (0..take)
            .map(|i| self.get_by_period(take - 1 - i))
            .collect()
    }

    /// Convert a raw `vals`-slice index (from an ordered snapshot) into a "bars ago" distance.
    ///
    /// `window_index = count - 1` → `0` (newest).
    /// `window_index = 0` → `count - 1` (oldest).
    #[inline(always)]
    pub fn window_index_to_bars_ago(&self, window_index: usize) -> usize {
        self.count - 1 - window_index
    }
}

impl<T: BufferElement, const N: usize> Default for FixedRingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

// ── Serde ─────────────────────────────────────────────────────────────────────
//
// Same strategy as FixedMirrorBuffer: hand-rolled to avoid the
// `where [T; N]: Serialize` bound serde's derive generates for generic N.
//
// Serialize  — emit `vals` as a &[T] slice.
// Deserialize — read into Vec<T>, convert to [T; N] via TryFrom (Rust 1.59+).

impl<T: BufferElement + Serialize, const N: usize> Serialize for FixedRingBuffer<T, N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("FixedRingBuffer", 3)?;
        state.serialize_field("vals", self.vals.as_slice())?;
        state.serialize_field("index", &self.index)?;
        state.serialize_field("count", &self.count)?;
        state.end()
    }
}

impl<'de, T: BufferElement + Deserialize<'de>, const N: usize> Deserialize<'de>
    for FixedRingBuffer<T, N>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        const FIELDS: &[&str] = &["vals", "index", "count"];

        enum Field {
            Vals,
            Index,
            Count,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct FieldVisitor;
                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;
                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str("`vals`, `index`, or `count`")
                    }
                    fn visit_str<E: de::Error>(self, v: &str) -> Result<Field, E> {
                        match v {
                            "vals" => Ok(Field::Vals),
                            "index" => Ok(Field::Index),
                            "count" => Ok(Field::Count),
                            _ => Err(de::Error::unknown_field(v, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct FRBVisitor<T, const N: usize>(PhantomData<fn() -> T>);

        impl<'de, T: BufferElement + Deserialize<'de>, const N: usize> Visitor<'de> for FRBVisitor<T, N> {
            type Value = FixedRingBuffer<T, N>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct FixedRingBuffer")
            }

            fn visit_map<V: MapAccess<'de>>(
                self,
                mut map: V,
            ) -> Result<FixedRingBuffer<T, N>, V::Error> {
                let mut vals: Option<Vec<T>> = None;
                let mut index: Option<usize> = None;
                let mut count: Option<usize> = None;

                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::Vals => {
                            if vals.is_some() {
                                return Err(de::Error::duplicate_field("vals"));
                            }
                            vals = Some(map.next_value()?);
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

                let vals_vec: Vec<T> = vals.ok_or_else(|| de::Error::missing_field("vals"))?;
                let index = index.ok_or_else(|| de::Error::missing_field("index"))?;
                let count = count.ok_or_else(|| de::Error::missing_field("count"))?;

                let vals_arr: [T; N] = vals_vec.try_into().map_err(|v: Vec<T>| {
                    de::Error::invalid_length(v.len(), &"vals array of length N")
                })?;

                Ok(FixedRingBuffer {
                    vals: vals_arr,
                    index,
                    count,
                })
            }
        }

        deserializer.deserialize_struct("FixedRingBuffer", FIELDS, FRBVisitor::<T, N>(PhantomData))
    }
}
