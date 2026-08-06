//! Heap-allocated mirror buffer.
//!
//! `MirrorBuffer<S, T>` has identical fields to `Buffer<S, T>` but `vals` is
//! allocated with `2 * capacity` slots. Every push writes to both `vals[index]`
//! AND `vals[index + capacity]`, maintaining the invariant that
//! `vals[index..index+capacity]` is always a contiguous, chronologically-ordered
//! window — enabling branchless SIMD min/max scans.

use crate::indicators::{
    max::{find_max_scalar, find_max_simd, State as MaxState},
    min::{find_min_scalar, find_min_simd, State as MinState},
};

use crate::ring_buffer::{
    buffer::{period_to_idx, SerdeElement},
    single_buffer::generic_buffer::{
        buf_advance, buf_advance_unchecked, buf_get_by_period,
        buf_to_ordered_by_period, buf_to_ordered_vec, BufferElement, Cold, Warm,
    },
};
//use serde::{Deserialize, Serialize};

// ── Struct ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MirrorBuffer<S = Cold, T: BufferElement = f64> {
    /// Backing store: length is always `2 * capacity`.
    /// `vals[0..capacity]` is the primary ring; `vals[capacity..2*capacity]` is the mirror.
    pub(crate) vals: Vec<T>,
    pub(crate) index: usize,
    pub(crate) capacity: usize,
    pub(crate) count: usize,
    pub(crate) prev_idx: usize,
    pub(crate) state: std::marker::PhantomData<S>,
}

// ── Shared methods (valid for any fill state) ─────────────────────────────

impl<S, T: BufferElement> MirrorBuffer<S, T> {
    #[inline(always)]
    pub fn get_by_period(&self, period: usize) -> T {
        buf_get_by_period(&self.vals, self.index, self.capacity, period)
    }

    #[inline(always)]
    pub fn get_by_periods<const N: usize>(&self, periods: [usize; N]) -> [T; N] {
        std::array::from_fn(|i| self.get_by_period(periods[i]))
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

    /// Raw backing storage (length = `2 * capacity`).
    #[inline(always)]
    pub fn raw_slice(&self) -> &[T] {
        &self.vals
    }
    #[inline(always)]
    pub fn raw_slice_mut(&mut self) -> &mut [T] {
        &mut self.vals
    }

    /// Ordered snapshot from oldest to newest, using only the primary ring.
    pub fn to_ordered_vec(&self) -> Vec<T> {
        // Pass only the primary ring (first capacity slots) to avoid mirror data.
        buf_to_ordered_vec(
            &self.vals[..self.capacity],
            self.index,
            self.capacity,
            self.count,
        )
    }

    pub fn to_ordered_by_period(&self, period: usize) -> Vec<T> {
        buf_to_ordered_by_period(&self.vals, self.index, self.capacity, self.count, period)
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
}

// ── Cold-specific methods ─────────────────────────────────────────────────

impl<T: BufferElement> MirrorBuffer<Cold, T> {
    /// Creates a new empty mirror buffer. Allocates `2 * capacity` slots.
    pub fn new(capacity: usize) -> Self {
        Self {
            vals: vec![T::default(); capacity * 2],
            index: 0,
            prev_idx: 0,
            capacity,
            count: 0,
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

    /// Push a value, writing to both the primary ring slot and the mirror copy.
    #[inline(always)]
    pub fn push(&mut self, value: T) {
        unsafe {
            *self.vals.get_unchecked_mut(self.index) = value;
            *self.vals.get_unchecked_mut(self.index + self.capacity) = value;
        }
        self.update_internals();
    }

    /// Push and return the evicted value once full, `None` while filling.
    #[inline(always)]
    pub fn push_with_info(&mut self, value: T) -> Option<T> {
        if self.count == self.capacity {
            let replaced = unsafe { *self.vals.get_unchecked(self.index) };
            unsafe {
                *self.vals.get_unchecked_mut(self.index) = value;
                *self.vals.get_unchecked_mut(self.index + self.capacity) = value;
            }
            self.update_internals_unchecked();
            return Some(replaced);
        }
        unsafe {
            *self.vals.get_unchecked_mut(self.index) = value;
            *self.vals.get_unchecked_mut(self.index + self.capacity) = value;
        }
        self.update_internals();
        None
    }

    /// Build a `MirrorBuffer<Cold, T>` from an existing slice (no mirror).
    pub fn from_slice(vals: &[T], capacity: usize) -> Self {
        let count = vals.len().min(capacity);
        let mut buf_vals = vals[..count].to_vec();
        buf_vals.resize(capacity, T::default());
        buf_vals.extend_from_within(..); // duplicate to make mirror
        let index = count % capacity;
        Self {
            vals: buf_vals,
            index,
            prev_idx: index.wrapping_sub(1) % capacity,
            capacity,
            count,
            state: std::marker::PhantomData,
        }
    }
    /// Contiguous ordered window for the current fill state.
    /// When full uses the mirror region; when partial data is at `vals[0..count]`.
    #[inline(always)]
    pub fn get_slice(&self) -> &[T] {
        if self.count == 0 {
            return &[];
        }
        if self.count == self.capacity {
            return unsafe { self.vals.get_unchecked(self.index..self.index + self.count) };
        }
        unsafe { self.vals.get_unchecked(0..self.count) }
    }

    /// Transition to `MirrorBuffer<Warm, T>`.
    #[inline(always)]
    pub fn into_full(self) -> MirrorBuffer<Warm, T> {
        debug_assert!(
            self.is_full(),
            "MirrorBuffer::into_full called on a non-full buffer (count={}, capacity={})",
            self.count,
            self.capacity
        );
        MirrorBuffer {
            vals: self.vals,
            index: self.index,
            capacity: self.capacity,
            count: self.count,
            prev_idx: self.prev_idx,
            state: std::marker::PhantomData,
        }
    }
}

// ── MinMax — works for both Cold and Warm ─────────────────────────────────────
//
// A single generic impl replaces the old Cold-only and Warm-only blocks.
// `current_window()` returns the right slice for whichever fill state S is.

impl MirrorBuffer<Warm, f64> {
    #[inline(always)]
    pub fn max(
        &self,
        state: &mut MaxState<Warm>,
        bar: f64,
        period: usize,
    ) -> (f64, usize) {
        self.max_chuncked::<4>(state, bar, period)
    }
    #[inline(always)]
    pub fn max_chuncked<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MaxState<Warm>,
        bar: f64,
        period: usize,
    ) -> (f64, usize) {
        let (mut max, mut trail) = (state.max, state.trail);
        trail += 1;
        if period <= trail {
            let slice = self.get_slice();
            let (max_val, max_idx) = if CHUNK_SIZE == 1 {
                find_max_scalar(slice)
            } else {
                find_max_simd::<CHUNK_SIZE>(slice)
            };
            max = max_val;
            trail = slice.len() - 1 - max_idx;
        } else if bar >= max {
            max = bar;
            trail = 0;
        }
        (state.max, state.trail) = (max, trail);
        (max, trail)
    }

    #[inline(always)]
    pub fn min(
        &self,
        state: &mut MinState<Warm>,
        bar: f64,
        period: usize,
    ) -> (f64, usize) {
        self.min_chuncked::<4>(state, bar, period)
    }
    #[inline(always)]
    pub fn min_chuncked<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MinState<Warm>,
        bar: f64,
        period: usize,
    ) -> (f64, usize) {
        let (mut min, mut trail) = (state.min, state.trail);
        trail += 1;
        if period <= trail {
            let slice = self.get_slice();
            let (min_val, min_idx) = if CHUNK_SIZE == 1 {
                find_min_scalar(slice)
            } else {
                find_min_simd::<CHUNK_SIZE>(slice)
            };
            min = min_val;
            trail = slice.len() - 1 - min_idx;
        } else if bar <= min {
            min = bar;
            trail = 0;
        }
        (state.min, state.trail) = (min, trail);
        (min, trail)
    }
}
impl MirrorBuffer<Cold, f64> {
    #[inline(always)]
    pub(crate) fn max(
        &self,
        state: &mut MaxState<Cold>,
        bar: f64,
    ) -> (f64, usize) {
        let (mut max, mut trail) = (state.max, state.trail);
        if bar >= max {
            max = bar;
            trail = 0;
        } else {
            trail += 1;
        }
        (state.max, state.trail) = (max, trail);
        (max, trail)
    }

    #[inline(always)]
    pub fn min(
        &self,
        state: &mut MinState<Cold>,
        bar: f64,
    ) -> (f64, usize) {
        let (mut min, mut trail) = (state.min, state.trail);

        if bar <= min {
            min = bar;
            trail = 0;
        } else {
            trail += 1;
        }
        (state.min, state.trail) = (min, trail);
        (min, trail)
    }
}
impl<T: BufferElement> MirrorBuffer<Warm, T> {
    /// Oldest element (always valid — buffer is full).
    #[inline(always)]
    pub fn front(&self) -> T {
        unsafe { *self.vals.get_unchecked(self.index) }
    }

    /// Most recently pushed element (always valid — buffer is full).
    #[inline(always)]
    pub fn back(&self) -> T {
        unsafe { *self.vals.get_unchecked(self.prev_idx) }
    }

    /// Push a value (branchless), writing to both ring and mirror slots.
    #[inline(always)]
    pub fn push(&mut self, value: T) {
        unsafe {
            *self.vals.get_unchecked_mut(self.index) = value;
            *self.vals.get_unchecked_mut(self.index + self.capacity) = value;
        }
        self.update_internals_unchecked();
    }

    /// Push and return the evicted value (branchless, always evicts).
    #[inline(always)]
    pub fn push_with_info(&mut self, value: T) -> T {
        let replaced = unsafe { *self.vals.get_unchecked(self.index) };
        unsafe {
            *self.vals.get_unchecked_mut(self.index) = value;
            *self.vals.get_unchecked_mut(self.index + self.capacity) = value;
        }
        self.update_internals_unchecked();
        replaced
    }

    /// Push and simultaneously read back values at N periods-ago distances.
    #[inline(always)]
    pub fn push_with_info_periods<const N: usize>(
        &mut self,
        value: T,
        periods: [usize; N],
    ) -> [T; N] {
        let idxs: [usize; N] =
            std::array::from_fn(|i| period_to_idx(self.index, self.capacity, periods[i] - 1));
        let results: [T; N] = std::array::from_fn(|i| unsafe { *self.vals.get_unchecked(idxs[i]) });
        unsafe {
            *self.vals.get_unchecked_mut(self.index) = value;
            *self.vals.get_unchecked_mut(self.index + self.capacity) = value;
        }
        self.update_internals_unchecked();
        results
    }

    /// Returns the contiguous, chronologically-ordered window `vals[index..index+capacity]`.
    ///
    /// Always valid because the mirror invariant guarantees contiguity.
    /// `result[0]` = oldest, `result[capacity-1]` = newest.
    #[inline(always)]
    pub fn get_slice(&self) -> &[T] {
        unsafe {
            self.vals
                .get_unchecked(self.index..self.index + self.capacity)
        }
    }

    /// Returns the newest `period` elements as a contiguous ordered slice.
    #[inline(always)]
    pub fn get_slice_by_period(&self, period: usize) -> &[T] {
        let take = period.min(self.capacity);
        let start = self.index + (self.capacity - take);
        unsafe { self.vals.get_unchecked(start..start + take) }
    }

    /// Convert mirror-window index (0 = oldest) to bars-ago (0 = newest).
    #[inline(always)]
    pub fn window_index_to_bars_ago(&self, window_index: usize) -> usize {
        self.capacity - 1 - window_index
    }
    
}

// ── Iterator ──────────────────────────────────────────────────────────────

pub struct MirrorBufferIter<'a, S, T: BufferElement> {
    buffer: &'a MirrorBuffer<S, T>,
    pos: usize,
}

impl<'a, S, T: BufferElement> Iterator for MirrorBufferIter<'a, S, T> {
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
        let rem = self.buffer.count.saturating_sub(self.pos);
        (rem, Some(rem))
    }
}

impl<'a, S, T: BufferElement> ExactSizeIterator for MirrorBufferIter<'a, S, T> {}

impl<'a, S, T: BufferElement> IntoIterator for &'a MirrorBuffer<S, T> {
    type Item = T;
    type IntoIter = MirrorBufferIter<'a, S, T>;
    #[inline]
    fn into_iter(self) -> MirrorBufferIter<'a, S, T> {
        MirrorBufferIter {
            buffer: self,
            pos: 0,
        }
    }
}

impl<S, T: BufferElement> std::ops::Index<usize> for MirrorBuffer<S, T> {
    type Output = T;
    #[inline]
    fn index(&self, bars_ago: usize) -> &T {
        assert!(bars_ago < self.count);
        let idx = period_to_idx(self.index, self.capacity, bars_ago);
        &self.vals[idx]
    }
}

// ── Serde (hand-rolled, same wire format as Buffer) ──────────────────────

impl<Stat, T: BufferElement + SerdeElement> serde::Serialize for MirrorBuffer<Stat, T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("MirrorBuffer", 5)?;
        let repr: Vec<T::Repr> = self.vals.iter().map(|v| T::to_repr(*v)).collect();
        s.serialize_field("vals", &repr)?;
        s.serialize_field("index", &self.index)?;
        s.serialize_field("capacity", &self.capacity)?;
        s.serialize_field("count", &self.count)?;
        s.serialize_field("prev_idx", &self.prev_idx)?;
        s.end()
    }
}

impl<'de, Stat, T: BufferElement + SerdeElement> serde::Deserialize<'de> for MirrorBuffer<Stat, T>
where
    T::Repr: serde::Deserialize<'de>,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{MapAccess, Visitor};
        struct Vis<Stat, T>(std::marker::PhantomData<(Stat, T)>);
        impl<'de, Stat, T: BufferElement + SerdeElement> Visitor<'de> for Vis<Stat, T>
        where
            T::Repr: serde::Deserialize<'de>,
        {
            type Value = MirrorBuffer<Stat, T>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a MirrorBuffer struct")
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<MirrorBuffer<Stat, T>, A::Error> {
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
                Ok(MirrorBuffer {
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
        deserializer.deserialize_struct(
            "MirrorBuffer",
            FIELDS,
            Vis::<Stat, T>(std::marker::PhantomData),
        )
    }
}
