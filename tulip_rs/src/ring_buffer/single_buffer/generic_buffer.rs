use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

use crate::ring_buffer::buffer::{period_to_idx, SerdeElement};
pub use crate::ring_buffer::{
    buffer::BufferElement,
    single_buffer::{
        mirror_buffer::MirrorBuffer,
        simd_buffer::{SimdBuffer, SimdMirrorBuffer, SimdRingBuffer},
    },
};
pub use crate::types::{Cold, Warm};

// ── Shared inner helpers (used by both Buffer and MirrorBuffer) ─────────────

#[inline(always)]
pub(super) fn buf_next_index(index: usize, capacity: usize) -> usize {
    let next = index + 1;
    if next == capacity {
        0
    } else {
        next
    }
}

#[inline(always)]
pub(super) fn buf_advance(
    index: &mut usize,
    prev_idx: &mut usize,
    count: &mut usize,
    capacity: usize,
) {
    *prev_idx = *index;
    *index = buf_next_index(*index, capacity);
    if *count < capacity {
        *count += 1;
    }
}

#[inline(always)]
pub(super) fn buf_advance_unchecked(index: &mut usize, prev_idx: &mut usize, capacity: usize) {
    *prev_idx = *index;
    *index = buf_next_index(*index, capacity);
}

#[inline(always)]
pub(super) fn buf_get_by_period<T: BufferElement>(
    vals: &[T],
    index: usize,
    capacity: usize,
    period: usize,
) -> T {
    use crate::ring_buffer::buffer::period_to_idx;
    let idx = period_to_idx(index, capacity, period);
    unsafe { *vals.get_unchecked(idx) }
}
#[inline(always)]
pub(super) fn buf_to_ordered_vec<T: BufferElement>(
    vals: &[T],
    index: usize,
    capacity: usize,
    count: usize,
) -> Vec<T> {
    if count == 0 {
        return Vec::new();
    }
    if count == capacity {
        let mut result = Vec::with_capacity(capacity);
        result.extend_from_slice(&vals[index..capacity]);
        if index > 0 {
            result.extend_from_slice(&vals[..index]);
        }
        return result;
    }
    vals[..count].to_vec()
}
#[inline(always)]
pub(super) fn buf_to_ordered_by_period<T: BufferElement>(
    vals: &[T],
    index: usize,
    capacity: usize,
    count: usize,
    period: usize,
) -> Vec<T> {
    if count == 0 || period == 0 {
        return Vec::new();
    }
    let take = period.min(count);
    (0..take)
        .map(|i| buf_get_by_period(vals, index, capacity, take - 1 - i))
        .collect()
}

/// A heap-backed ring buffer.
///
/// The `S` parameter encodes fill state at the type level:
/// * [`Cold`] — warmup phase; `front()` returns `Option<T>`, `push_with_info` returns `Option<T>`.
/// * [`Warm`]    — operational phase; `front()` returns `T` (infallible), `push_with_info` returns `T`
///   (always evicts, no branch).
///
/// Transition from `Cold` to `Warm` via [`Buffer::into_full`].
#[derive(Clone)]
pub struct Buffer<S = Cold, T: BufferElement = f64> {
    pub(crate) vals: Vec<T>,
    pub(crate) index: usize,
    pub(crate) capacity: usize,
    pub(crate) count: usize,
    pub(crate) prev_idx: usize,
    pub(crate) state: std::marker::PhantomData<S>,
}

// ── Shared methods (valid for any fill state) ──────────────────────────────

impl<S, T: BufferElement> Buffer<S, T> {
    /// Read element at `period` bars ago (0 = newest, capacity-1 = oldest).
    #[inline(always)]
    pub fn get_by_period(&self, period: usize) -> T {
        buf_get_by_period(&self.vals, self.index, self.capacity, period)
    }

    /// Read multiple elements at the given bars-ago distances.
    #[inline(always)]
    pub fn get_by_periods<const N: usize>(&self, periods: [usize; N]) -> [T; N] {
        let mut results = [T::default(); N];
        let idxs: [usize; N] =
            std::array::from_fn(|i| period_to_idx(self.index, self.capacity, periods[i]));
        for (&buffer_idx, results_value) in idxs.iter().zip(results.iter_mut()) {
            *results_value = unsafe { *self.vals.get_unchecked(buffer_idx) }
        }
        results
    }
    #[inline(always)]
    pub(crate) fn update_internals(&mut self) {
        buf_advance(
            &mut self.index,
            &mut self.prev_idx,
            &mut self.count,
            self.capacity,
        );
    }

    #[inline(always)]
    pub(crate) fn update_internals_unchecked(&mut self) {
        buf_advance_unchecked(&mut self.index, &mut self.prev_idx, self.capacity);
    }
    #[inline(always)]
    pub fn get_count(&self) -> usize {
        self.count
    }
    #[inline(always)]
    pub fn get_idx(&self) -> usize {
        self.index
    }
    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.count == self.capacity
    }
    #[inline(always)]
    pub fn get_prev_idx(&self) -> usize {
        self.prev_idx
    }
    #[inline(always)]
    pub fn get_capacity(&self) -> usize {
        self.capacity
    }
    #[inline(always)]
    pub fn raw_slice(&self) -> &[T] {
        &self.vals
    }
    #[inline(always)]
    pub fn raw_slice_mut(&mut self) -> &mut [T] {
        &mut self.vals
    }

    /// Ordered snapshot from oldest to newest.  Allocates.
    pub fn to_ordered_vec(&self) -> Vec<T> {
        buf_to_ordered_vec(&self.vals, self.index, self.capacity, self.count)
    }

    /// Ordered snapshot of the most recent `period` elements (oldest-first).  Allocates.
    pub fn to_ordered_by_period(&self, period: usize) -> Vec<T> {
        buf_to_ordered_by_period(&self.vals, self.index, self.capacity, self.count, period)
    }
}

// ── Cold-specific methods ────────────────────────────────────────────────

impl<T: BufferElement> Buffer<Cold, T> {
    /// Create a new empty ring buffer (allocates `capacity` slots).
    pub fn new(capacity: usize) -> Self {
        Self {
            vals: vec![T::default(); capacity],
            index: 0,
            prev_idx: 0,
            capacity,
            count: 0,
            state: std::marker::PhantomData,
        }
    }

    /// Push a value into the ring buffer (single-slot write).
    #[inline(always)]
    pub fn push(&mut self, value: T) {
        unsafe {
            *self.vals.get_unchecked_mut(self.index) = value;
        }
        self.update_internals();
    }

    /// Push a value and return the evicted element once full, `None` while filling.
    #[inline(always)]
    pub fn push_with_info(&mut self, value: T) -> Option<T> {
        if self.count == self.capacity {
            let replaced = unsafe { *self.vals.get_unchecked(self.index) };
            unsafe {
                *self.vals.get_unchecked_mut(self.index) = value;
            }
            self.update_internals_unchecked();
            return Some(replaced);
        }
        unsafe {
            *self.vals.get_unchecked_mut(self.index) = value;
        }
        self.update_internals();
        None
    }

    /// Returns all values in the backing store (ring order, includes uninitialized slots).
    /// For a full buffer all slots are valid. Same semantics as `raw_slice()`.
    #[inline(always)]
    pub fn get_slice(&self) -> &[T] {
        &self.vals
    }

    /// Build a `Buffer<Cold, T>` from an existing slice.
    pub fn from_slice(vals: &[T], capacity: usize) -> Self {
        let count = vals.len().min(capacity);
        let mut buffer_vals = vals.to_vec();
        if count < capacity {
            buffer_vals.resize(capacity, T::default());
        }
        let index = count % capacity;
        Self {
            vals: buffer_vals,
            index,
            prev_idx: index.wrapping_sub(1) % capacity,
            capacity,
            count,
            state: std::marker::PhantomData,
        }
    }

    /// Oldest element, or `None` if empty.
    #[inline(always)]
    pub fn front(&self) -> Option<T> {
        if self.count == 0 {
            return None;
        }
        Some(unsafe { *self.vals.get_unchecked(self.index) })
    }

    /// Most recently pushed element, or `None` if empty.
    #[inline(always)]
    pub fn back(&self) -> Option<T> {
        if self.count == 0 {
            return None;
        }
        Some(unsafe { *self.vals.get_unchecked(self.prev_idx) })
    }

    /// Transition into a [`Buffer<Warm, T>`].
    ///
    /// # Panics (debug builds)
    /// Panics when `debug_assertions` are enabled and `is_full()` is `false`.
    #[inline(always)]
    pub fn into_full(self) -> Buffer<Warm, T> {
        debug_assert!(
            self.is_full(),
            "Buffer::into_full called on a non-full buffer (count={}, capacity={})",
            self.count,
            self.capacity
        );
        Buffer {
            vals: self.vals,
            index: self.index,
            capacity: self.capacity,
            count: self.count,
            prev_idx: self.prev_idx,
            state: std::marker::PhantomData,
        }
    }
}

// ── Warm-specific methods ───────────────────────────────────────────────────

impl<T: BufferElement> Buffer<Warm, T> {
    /// Oldest element (always valid; buffer is guaranteed non-empty).
    #[inline(always)]
    pub fn front(&self) -> T {
        // Safety: Warm guarantees count == capacity > 0.
        unsafe { *self.vals.get_unchecked(self.index) }
    }

    /// Most recently pushed element (always valid; buffer is guaranteed non-empty).
    #[inline(always)]
    pub fn back(&self) -> T {
        // Safety: Warm guarantees count == capacity > 0.
        unsafe { *self.vals.get_unchecked(self.prev_idx) }
    }

    /// Returns all values currently in the buffer as a raw slice.
    ///
    /// For a full ring buffer the backing array holds exactly `capacity` live values
    /// (in ring order, not necessarily time order). Use this when order doesn't matter,
    /// e.g. mean-deviation sums. For a time-ordered contiguous view use `get_mirror_slice`.
    #[inline(always)]
    pub fn get_slice(&self) -> &[T] {
        &self.vals
    }

    /// Push `value`, evict the oldest element, and return it.
    #[inline(always)]
    pub fn push_with_info(&mut self, value: T) -> T {
        let replaced = unsafe { *self.vals.get_unchecked(self.index) };
        unsafe { *self.vals.get_unchecked_mut(self.index) = value };
        self.update_internals_unchecked();
        replaced
    }

    /// Push `value` (discarding the evicted element).
    #[inline(always)]
    pub fn push(&mut self, value: T) {
        unsafe { *self.vals.get_unchecked_mut(self.index) = value };
        self.update_internals_unchecked();
    }

    /// Push `value` and simultaneously read back the values that were at each of
    /// the specified `periods` *before* the push (1-based: `periods[i] = 1` means
    /// the element that was just evicted).
    ///
    /// Safe because the buffer is known to be full at the type level.
    #[inline(always)]
    pub fn push_with_info_periods<const N: usize>(
        &mut self,
        value: T,
        periods: [usize; N],
    ) -> [T; N] {
        let idxs: [usize; N] =
            std::array::from_fn(|i| period_to_idx(self.index, self.capacity, periods[i] - 1));
        let mut results = [T::default(); N];
        for (&buffer_idx, result) in idxs.iter().zip(results.iter_mut()) {
            *result = unsafe { *self.vals.get_unchecked(buffer_idx) };
        }
        unsafe { *self.vals.get_unchecked_mut(self.index) = value };
        self.update_internals_unchecked();
        results
    }

    /// Contiguous mirror-buffer window `vals[index .. index + capacity]`.
    ///
    /// Only valid when `self.vals` was allocated as a mirror buffer (length `2 * capacity`).
    /// Returns the always-contiguous oldest-to-newest view used by SIMD mirror-buffer paths.
    #[inline(always)]
    pub fn get_mirror_slice(&self) -> &[T] {
        unsafe {
            self.vals
                .get_unchecked(self.index..self.index + self.capacity)
        }
    }
}

// ── Iterator ──────────────────────────────────────────────────────────────────

pub struct BufferIter<'a, S, T: BufferElement> {
    pub buffer: &'a Buffer<S, T>,
    /// Current position expressed as bars-ago (0 = newest).
    pub pos: usize,
    pub current_idx: usize,
}

impl<'a, S, T: BufferElement> Iterator for BufferIter<'a, S, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        if self.pos >= self.buffer.count {
            return None;
        }
        let val = self.buffer.get_by_period(self.pos);
        self.pos += 1;
        Some(val)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.buffer.count.saturating_sub(self.pos);
        (remaining, Some(remaining))
    }
}

impl<'a, S, T: BufferElement> ExactSizeIterator for BufferIter<'a, S, T> {}

impl<'a, S, T: BufferElement> IntoIterator for &'a Buffer<S, T> {
    type Item = T;
    type IntoIter = BufferIter<'a, S, T>;

    /// Iterate from newest to oldest (`buf[0]` first).
    #[inline]
    fn into_iter(self) -> BufferIter<'a, S, T> {
        BufferIter {
            buffer: self,
            pos: 0,
            current_idx: self.prev_idx,
        }
    }
}

impl<S, T: BufferElement> std::ops::Index<usize> for Buffer<S, T> {
    type Output = T;

    /// Index by bars-ago: `buf[0]` is the newest element, `buf[count-1]` is the oldest.
    #[inline]
    fn index(&self, bars_ago: usize) -> &T {
        assert!(
            bars_ago < self.count,
            "index out of bounds: bars_ago {bars_ago} >= count {}",
            self.count
        );
        let idx = period_to_idx(self.index, self.capacity, bars_ago);
        &self.vals[idx]
    }
}

/// Read multiple elements at the given raw storage indices (not bars-ago) from a buffer.
#[inline(always)]
pub fn get_by_periods<const N: usize, S, T: BufferElement>(
    buffer: &Buffer<S, T>,
    idxs: [usize; N],
) -> [T; N] {
    let mut results = [T::default(); N];
    for (&buffer_idx, results_value) in idxs.iter().zip(results.iter_mut()) {
        *results_value = unsafe { *buffer.vals.get_unchecked(buffer_idx) }
    }
    results
}

// Type aliases for convenience
/// A scalar `f64` buffer in the warmup phase.  For the operational phase use `Buffer<Warm>`.
pub type F64Buffer = Buffer<Cold, f64>;

// ── Serde ─────────────────────────────────────────────────────────────────────
//
// Hand-rolled rather than #[derive] so that Buffer<S, Simd<f64, N>> is serialisable
// via T::Repr even though Simd<f64, N> does not implement serde directly.
//
// The `S` typestate parameter carries no information at runtime (PhantomData) and is
// therefore NOT emitted to the wire.  Deserialization reconstructs the appropriate
// `Buffer<S, T>` purely from type inference — the caller's field type determines `S`.
//
// Field order matches the pre-typestate struct declaration (vals, index, capacity,
// count, prev_idx) so existing JSON snapshots continue to deserialize unchanged.

impl<Stat, T: BufferElement + SerdeElement> Serialize for Buffer<Stat, T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Buffer", 5)?;
        let repr: Vec<T::Repr> = self.vals.iter().map(|v| T::to_repr(*v)).collect();
        s.serialize_field("vals", &repr)?;
        s.serialize_field("index", &self.index)?;
        s.serialize_field("capacity", &self.capacity)?;
        s.serialize_field("count", &self.count)?;
        s.serialize_field("prev_idx", &self.prev_idx)?;
        s.end()
    }
}

impl<'de, Stat, T: BufferElement + SerdeElement> Deserialize<'de> for Buffer<Stat, T>
where
    T::Repr: Deserialize<'de>,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Vis<Stat, T>(std::marker::PhantomData<(Stat, T)>);

        impl<'de, Stat, T: BufferElement + SerdeElement> Visitor<'de> for Vis<Stat, T>
        where
            T::Repr: Deserialize<'de>,
        {
            type Value = Buffer<Stat, T>;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a Buffer struct")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Buffer<Stat, T>, A::Error> {
                let mut vals = None::<Vec<T::Repr>>;
                let mut index = None::<usize>;
                let mut capacity = None::<usize>;
                let mut count = None::<usize>;
                let mut prev_idx = None::<usize>;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "vals" => vals = Some(map.next_value()?),
                        "index" => index = Some(map.next_value()?),
                        "capacity" => capacity = Some(map.next_value()?),
                        "count" => count = Some(map.next_value()?),
                        "prev_idx" => prev_idx = Some(map.next_value()?),
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                Ok(Buffer {
                    vals: vals
                        .ok_or_else(|| serde::de::Error::missing_field("vals"))?
                        .into_iter()
                        .map(T::from_repr)
                        .collect(),
                    index: index.ok_or_else(|| serde::de::Error::missing_field("index"))?,
                    capacity: capacity
                        .ok_or_else(|| serde::de::Error::missing_field("capacity"))?,
                    count: count.ok_or_else(|| serde::de::Error::missing_field("count"))?,
                    prev_idx: prev_idx
                        .ok_or_else(|| serde::de::Error::missing_field("prev_idx"))?,
                    state: std::marker::PhantomData,
                })
            }
        }

        const FIELDS: &[&str] = &["vals", "index", "capacity", "count", "prev_idx"];
        deserializer.deserialize_struct("Buffer", FIELDS, Vis::<Stat, T>(std::marker::PhantomData))
    }
}
