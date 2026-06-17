use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

use crate::ring_buffer::buffer::{period_to_idx, SerdeElement};
pub use crate::ring_buffer::{
    buffer::BufferElement,
    single_buffer::{
        mirror_buffer::MirrorBuffer,
        ring_buffer::RingBuffer,
        simd_buffer::{SimdBuffer, SimdMirrorBuffer, SimdRingBuffer},
    },
};

#[derive(Clone)]
pub struct Buffer<T: BufferElement = f64> {
    pub(crate) vals: Vec<T>,
    pub(crate) index: usize,
    pub(crate) capacity: usize,
    pub(crate) count: usize,
    pub(crate) prev_idx: usize,
}
impl<T: BufferElement> Buffer<T> {
    pub fn from_slice(vals: &[T], capacity: usize) -> Self {
        let count = vals.len().min(capacity);
        let mut buffer_vals = vals.to_vec();
        if count < capacity {
            buffer_vals.resize(capacity, T::default());
        }
        let index = count % capacity;
        Self {
            vals: buffer_vals,
            index: index,
            prev_idx: index.wrapping_sub(1) % capacity,
            capacity,
            count,
        }
    }
    #[inline(always)]
    pub fn front(&self) -> Option<T> {
        if self.count == 0 {
            return None;
        }
        Some(unsafe { self.front_unchecked() })
    }

    #[inline(always)]
    pub unsafe fn front_unchecked(&self) -> T {
        *self.vals.get_unchecked(self.index)
    }
    #[inline(always)]
    pub fn back(&self) -> Option<T> {
        if self.count == 0 {
            return None;
        }
        Some(unsafe { self.back_unchecked() })
    }

    #[inline(always)]
    pub unsafe fn back_unchecked(&self) -> T {
        *self.vals.get_unchecked(self.prev_idx)
    }

    #[inline(always)]
    pub fn get_by_period(&self, period: usize) -> T {
        let idx = period_to_idx(self.index, self.capacity, period);
        unsafe { *self.vals.get_unchecked(idx) }
    }
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
    pub(crate) fn update_internals(&mut self) {
        self.prev_idx = self.index;
        self.index = self.calc_index();
        if self.count < self.capacity {
            self.count += 1;
        }
    }
    #[inline(always)]
    pub(crate) fn calc_index(&self) -> usize {
        let mut new_idx = self.index + 1;
        if new_idx == self.capacity {
            new_idx = 0;
        }
        new_idx
    }
    pub(crate) fn update_internals_unchecked(&mut self) {
        self.prev_idx = self.index;
        self.index = self.calc_index();
    }

    pub fn get_count(&self) -> usize {
        self.count
    }

    pub fn get_idx(&self) -> usize {
        self.index
    }

    pub fn is_full(&self) -> bool {
        self.count == self.capacity
    }

    pub fn get_prev_idx(&self) -> usize {
        self.prev_idx
    }

    pub fn get_capacity(&self) -> usize {
        self.capacity
    }

    pub fn raw_slice(&self) -> &[T] {
        &self.vals
    }
    pub fn raw_slice_mut(&mut self) -> &mut [T] {
        &mut self.vals
    }
}

pub struct BufferIter<'a, T: BufferElement> {
    pub buffer: &'a Buffer<T>,
    /// Current position expressed as bars-ago (0 = newest).
    pub pos: usize,
    pub current_idx: usize, // Pre-computed starting index
}

impl<'a, T: BufferElement> Iterator for BufferIter<'a, T> {
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

impl<'a, T: BufferElement> ExactSizeIterator for BufferIter<'a, T> {}

impl<'a, T: BufferElement> IntoIterator for &'a Buffer<T> {
    type Item = T;
    type IntoIter = BufferIter<'a, T>;

    /// Iterate from newest to oldest (`buf[0]` first).
    #[inline]
    fn into_iter(self) -> BufferIter<'a, T> {
        BufferIter {
            buffer: self,
            pos: 0,
            current_idx: self.prev_idx,
        }
    }
}

impl<T: BufferElement> std::ops::Index<usize> for Buffer<T> {
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

#[inline(always)]
pub fn get_by_periods<const N: usize, T: BufferElement>(
    buffer: &Buffer<T>,
    idxs: [usize; N],
) -> [T; N] {
    let mut results = [T::default(); N];

    for (&buffer_idx, results_value) in idxs.iter().zip(results.iter_mut()) {
        *results_value = unsafe { *buffer.vals.get_unchecked(buffer_idx) }
    }
    results
}
// Type aliases for convenience
pub type F64Buffer = Buffer<f64>;

// ── Serde ─────────────────────────────────────────────────────────────────────
//
// Hand-rolled rather than #[derive] so that Buffer<Simd<f64, N>> is serialisable
// via T::Repr even though Simd<f64, N> does not implement serde directly.
//
// Requires T: BufferElement + SerdeElement (not just BufferElement) so that
// Buffer<Simd<f64, N>> used transiently in SIMD hot-paths does not accumulate
// serde bounds for every generic N.
//
// Field order matches the struct declaration (vals, index, capacity, count,
// prev_idx) so the wire format is identical to what #[derive] produced for
// scalar element types — existing JSON round-trips continue to work unchanged.

impl<T: BufferElement + SerdeElement> Serialize for Buffer<T> {
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

impl<'de, T: BufferElement + SerdeElement> Deserialize<'de> for Buffer<T>
where
    T::Repr: Deserialize<'de>,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Vis<T>(std::marker::PhantomData<T>);

        impl<'de, T: BufferElement + SerdeElement> Visitor<'de> for Vis<T>
        where
            T::Repr: Deserialize<'de>,
        {
            type Value = Buffer<T>;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a Buffer struct")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Buffer<T>, A::Error> {
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
                })
            }
        }

        const FIELDS: &[&str] = &["vals", "index", "capacity", "count", "prev_idx"];
        deserializer.deserialize_struct("Buffer", FIELDS, Vis::<T>(std::marker::PhantomData))
    }
}
