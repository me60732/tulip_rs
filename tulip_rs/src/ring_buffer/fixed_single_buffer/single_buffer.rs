//! Fixed-size, stack-allocated ring buffer (no mirroring).

use crate::ring_buffer::buffer::{period_to_idx, BufferElement, SerdeElement};
use crate::ring_buffer::single_buffer::generic_buffer::{Warm, Cold};
use serde::{
    de::{self, MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{fmt, marker::PhantomData};

/// A fixed-capacity, stack-allocated ring buffer without a mirrored view.
///
/// `S` encodes fill state at the type level:
/// * [`Cold`] — warmup; `front`/`back` return `Option<T>`, `push_with_info` returns `Option<T>`.
/// * [`Warm`]    — operational; `front`/`back` return `T`, `push_with_info` returns `T` (no branch).
#[derive(Clone)]
pub struct FixedRingBuffer<T: BufferElement, const N: usize, S = Cold> {
    pub(crate) vals: [T; N],
    pub(crate) index: usize,
    pub(crate) count: usize,
    pub(crate) state: PhantomData<S>,
}

// ── Shared methods (any fill state) ──────────────────────────────────────────

impl<T: BufferElement, const N: usize, S> FixedRingBuffer<T, N, S> {
    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.count == N
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.count
    }
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Raw underlying storage (unordered once the buffer has wrapped).
    #[inline(always)]
    pub fn get_slice(&self) -> &[T] {
        if self.count < N {
            &self.vals[..self.count]
        } else {
            &self.vals
        }
    }

    /// O(1) lookback.  `period = 0` → newest, `period = N-1` → oldest.
    #[inline(always)]
    pub fn get_by_period(&self, period: usize) -> T {
        let idx = period_to_idx(self.index, N, period);
        unsafe { *self.vals.get_unchecked(idx) }
    }

    /// Convert a raw-slice window index to bars-ago.
    #[inline(always)]
    pub fn window_index_to_bars_ago(&self, window_index: usize) -> usize {
        self.count - 1 - window_index
    }

    /// Ordered snapshot (oldest → newest). Allocates.
    pub fn to_ordered_vec(&self) -> Vec<T> {
        if self.count == 0 {
            return Vec::new();
        }
        if self.count < N {
            return self.vals[..self.count].to_vec();
        }
        let mut out = Vec::with_capacity(N);
        out.extend_from_slice(&self.vals[self.index..]);
        if self.index > 0 {
            out.extend_from_slice(&self.vals[..self.index]);
        }
        out
    }

    /// Ordered snapshot of the newest `period` elements. Allocates.
    pub fn to_ordered_by_period(&self, period: usize) -> Vec<T> {
        if self.count == 0 || period == 0 {
            return Vec::new();
        }
        let take = period.min(self.count);
        (0..take)
            .map(|i| self.get_by_period(take - 1 - i))
            .collect()
    }
}

// ── Cold methods ───────────────────────────────────────────────────────────

impl<T: BufferElement, const N: usize> FixedRingBuffer<T, N, Cold> {
    /// Create a new, empty buffer.
    #[inline]
    pub fn new() -> Self {
        Self {
            vals: [T::default(); N],
            index: 0,
            count: 0,
            state: PhantomData,
        }
    }

    /// Oldest element, or `None` if empty.
    #[inline(always)]
    pub fn front(&self) -> Option<T> {
        if self.count == 0 {
            return None;
        }
        let oldest = if self.count == N { self.index } else { 0 };
        Some(unsafe { *self.vals.get_unchecked(oldest) })
    }

    /// Newest element, or `None` if empty.
    #[inline(always)]
    pub fn back(&self) -> Option<T> {
        if self.count == 0 {
            return None;
        }
        let prev = (self.index + N - 1) % N;
        Some(unsafe { *self.vals.get_unchecked(prev) })
    }

    /// Push a new element, evicting the oldest when full.
    #[inline(always)]
    pub fn push(&mut self, value: T) {
        self.vals[self.index] = value;
        self.index += 1;
        if self.index == N {
            self.index = 0;
        }
        if self.count < N {
            self.count += 1;
        }
    }

    /// Push and return the evicted element once full, `None` while filling.
    #[inline(always)]
    pub fn push_with_info(&mut self, value: T) -> Option<T> {
        if self.count == N {
            let evicted = unsafe { *self.vals.get_unchecked(self.index) };
            unsafe { *self.vals.get_unchecked_mut(self.index) = value };
            self.index += 1;
            if self.index == N {
                self.index = 0;
            }
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
    pub fn into_full(self) -> FixedRingBuffer<T, N, Warm> {
        debug_assert!(
            self.count == N,
            "FixedRingBuffer::into_full called on non-full buffer (count={}, N={N})",
            self.count
        );
        FixedRingBuffer {
            vals: self.vals,
            index: self.index,
            count: self.count,
            state: PhantomData,
        }
    }
}

impl<T: BufferElement, const N: usize> Default for FixedRingBuffer<T, N, Cold> {
    fn default() -> Self {
        Self::new()
    }
}

// ── Warm methods ──────────────────────────────────────────────────────────────

impl<T: BufferElement, const N: usize> FixedRingBuffer<T, N, Warm> {
    /// Oldest element (always valid — buffer is full).
    #[inline(always)]
    pub fn front(&self) -> T {
        unsafe { *self.vals.get_unchecked(self.index) }
    }

    /// Newest element (always valid — buffer is full).
    #[inline(always)]
    pub fn back(&self) -> T {
        let prev = (self.index + N - 1) % N;
        unsafe { *self.vals.get_unchecked(prev) }
    }

    /// Push (branchless — buffer is guaranteed full, no count update needed).
    #[inline(always)]
    pub fn push(&mut self, value: T) {
        unsafe { *self.vals.get_unchecked_mut(self.index) = value };
        self.index += 1;
        if self.index == N {
            self.index = 0;
        }
    }

    /// Push and return the evicted element (branchless, always evicts).
    #[inline(always)]
    pub fn push_with_info(&mut self, value: T) -> T {
        let evicted = unsafe { *self.vals.get_unchecked(self.index) };
        unsafe { *self.vals.get_unchecked_mut(self.index) = value };
        self.index += 1;
        if self.index == N {
            self.index = 0;
        }
        evicted
    }
}

// ── Iterator ──────────────────────────────────────────────────────────────────

pub struct FixedRingIter<'a, T: BufferElement, const N: usize, S> {
    buffer: &'a FixedRingBuffer<T, N, S>,
    pos: usize,
}

impl<'a, T: BufferElement, const N: usize, S> Iterator for FixedRingIter<'a, T, N, S> {
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
        let r = self.buffer.count.saturating_sub(self.pos);
        (r, Some(r))
    }
}
impl<'a, T: BufferElement, const N: usize, S> ExactSizeIterator for FixedRingIter<'a, T, N, S> {}
impl<'a, T: BufferElement, const N: usize, S> IntoIterator for &'a FixedRingBuffer<T, N, S> {
    type Item = T;
    type IntoIter = FixedRingIter<'a, T, N, S>;
    #[inline]
    fn into_iter(self) -> FixedRingIter<'a, T, N, S> {
        FixedRingIter {
            buffer: self,
            pos: 0,
        }
    }
}

impl<T: BufferElement, const N: usize, S> std::ops::Index<usize> for FixedRingBuffer<T, N, S> {
    type Output = T;
    #[inline]
    fn index(&self, bars_ago: usize) -> &T {
        debug_assert!(
            bars_ago < self.count,
            "index out of bounds: bars_ago {bars_ago} >= count {}",
            self.count
        );
        let idx = period_to_idx(self.index, N, bars_ago);
        &self.vals[idx]
    }
}

// ── Serde ─────────────────────────────────────────────────────────────────────
//
// `S` carries no runtime data (PhantomData), so the wire format is unchanged.
// Deserialization reconstructs `FixedRingBuffer<T, N, S>` from type inference.

impl<T: BufferElement + SerdeElement, const N: usize, S> Serialize for FixedRingBuffer<T, N, S> {
    fn serialize<Ser: Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
        let mut state = serializer.serialize_struct("FixedRingBuffer", 3)?;
        let repr: Vec<T::Repr> = self.vals.iter().map(|v| T::to_repr(*v)).collect();
        state.serialize_field("vals", &repr)?;
        state.serialize_field("index", &self.index)?;
        state.serialize_field("count", &self.count)?;
        state.end()
    }
}

impl<'de, T: BufferElement + SerdeElement, const N: usize, S> Deserialize<'de>
    for FixedRingBuffer<T, N, S>
where
    T::Repr: Deserialize<'de>,
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
        struct FRBVisitor<T, S, const N: usize>(PhantomData<fn() -> (T, S)>);
        impl<'de, T: BufferElement + SerdeElement, const N: usize, S> Visitor<'de> for FRBVisitor<T, S, N>
        where
            T::Repr: Deserialize<'de>,
        {
            type Value = FixedRingBuffer<T, N, S>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct FixedRingBuffer")
            }
            fn visit_map<V: MapAccess<'de>>(
                self,
                mut map: V,
            ) -> Result<FixedRingBuffer<T, N, S>, V::Error> {
                let mut vals: Option<Vec<T::Repr>> = None;
                let mut index: Option<usize> = None;
                let mut count: Option<usize> = None;
                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::Vals => {
                            vals = Some(map.next_value()?);
                        }
                        Field::Index => {
                            index = Some(map.next_value()?);
                        }
                        Field::Count => {
                            count = Some(map.next_value()?);
                        }
                    }
                }
                let vals_repr = vals.ok_or_else(|| de::Error::missing_field("vals"))?;
                let index = index.ok_or_else(|| de::Error::missing_field("index"))?;
                let count = count.ok_or_else(|| de::Error::missing_field("count"))?;
                let vals_vec: Vec<T> = vals_repr.into_iter().map(T::from_repr).collect();
                let vals_arr: [T; N] = vals_vec.try_into().map_err(|v: Vec<T>| {
                    de::Error::invalid_length(v.len(), &"vals array of length N")
                })?;
                Ok(FixedRingBuffer {
                    vals: vals_arr,
                    index,
                    count,
                    state: PhantomData,
                })
            }
        }
        deserializer.deserialize_struct(
            "FixedRingBuffer",
            FIELDS,
            FRBVisitor::<T, S, N>(PhantomData),
        )
    }
}
