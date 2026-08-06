pub use crate::ring_buffer::buffer::BufferElement;
use crate::ring_buffer::buffer::{period_to_idx, SerdeElement};
pub use crate::ring_buffer::multi_buffer::simd_buffer::{SimdBuffer, SimdRingBuffer};
use crate::ring_buffer::single_buffer::generic_buffer::{Buffer as SingleBuffer, Cold, Warm};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct MultiBuffer<const B: usize, T: BufferElement = f64, S = Cold> {
    pub(crate) vals: [Vec<T>; B],
    pub(crate) index: usize,
    pub(crate) capacity: usize,
    pub(crate) count: usize,
    pub(crate) prev_idx: usize,
    pub(crate) state: std::marker::PhantomData<S>,
}

// Helper struct for serialization — uses T::Repr so that non-serde types like
// Simd<f64, N> are represented as their serde-compatible equivalent.
#[derive(Serialize, Deserialize)]
struct MultiBufferSerde<R> {
    vals: Vec<Vec<R>>,
    index: usize,
    capacity: usize,
    count: usize,
    prev_idx: usize,
}

impl<const N: usize, Stat, T: BufferElement + SerdeElement> Serialize for MultiBuffer<N, T, Stat> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let helper = MultiBufferSerde {
            vals: self
                .vals
                .iter()
                .map(|lane| lane.iter().map(|v| T::to_repr(*v)).collect())
                .collect(),
            index: self.index,
            capacity: self.capacity,
            count: self.count,
            prev_idx: self.prev_idx,
        };
        helper.serialize(serializer)
    }
}

impl<'de, const N: usize, Stat, T: BufferElement + SerdeElement> Deserialize<'de>
    for MultiBuffer<N, T, Stat>
where
    T::Repr: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = MultiBufferSerde::<T::Repr>::deserialize(deserializer)?;

        if helper.vals.len() != N {
            return Err(serde::de::Error::custom(format!(
                "Expected {} buffers, got {}",
                N,
                helper.vals.len()
            )));
        }

        let vals_array: [Vec<T>; N] = helper
            .vals
            .into_iter()
            .map(|lane| lane.into_iter().map(T::from_repr).collect())
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| serde::de::Error::custom("Failed to convert to array"))?;

        Ok(MultiBuffer {
            vals: vals_array,
            index: helper.index,
            capacity: helper.capacity,
            count: helper.count,
            prev_idx: helper.prev_idx,
            state: std::marker::PhantomData,
        })
    }
}

// ── Shared methods (valid for any fill state) ──────────────────────────────

impl<const B: usize, S, T: BufferElement> MultiBuffer<B, T, S> {
    #[inline(always)]
    fn get_values(&self, idx: usize) -> [T; B] {
        mb_get_values(&self.vals, idx)
    }

    #[inline(always)]
    pub fn get_by_period(&self, period: usize) -> [T; B] {
        let idx = period_to_idx(self.index, self.capacity, period);
        self.get_values(idx)
    }

    #[inline(always)]
    pub fn get_by_periods<const N: usize>(&self, periods: [usize; N]) -> [[T; N]; B] {
        let idxs: [usize; N] =
            std::array::from_fn(|i| period_to_idx(self.index, self.capacity, periods[i]));
        get_by_periods(self, idxs)
    }

    #[inline(always)]
    pub(crate) fn update_internals(&mut self) {
        mb_advance(
            &mut self.index,
            &mut self.prev_idx,
            &mut self.count,
            self.capacity,
        );
    }

    #[inline(always)]
    pub(crate) fn update_internals_unchecked(&mut self) {
        mb_advance_unchecked(&mut self.index, &mut self.prev_idx, self.capacity);
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

    pub fn raw_slice(&self) -> &[Vec<T>; B] {
        &self.vals
    }

    /// Ordered snapshot from oldest to newest across all lanes.  Allocates.
    pub fn to_ordered_vec(&self) -> [Vec<T>; B] {
        if self.count == 0 {
            return core::array::from_fn(|_| Vec::new());
        }
        core::array::from_fn(|lane| {
            if self.count == self.capacity {
                let mut result = Vec::with_capacity(self.capacity);
                result.extend_from_slice(&self.vals[lane][self.index..]);
                if self.index > 0 {
                    result.extend_from_slice(&self.vals[lane][..self.index]);
                }
                return result;
            }
            self.vals[lane][..self.count].to_vec()
        })
    }

    /// Ordered snapshot of the most recent `period` elements per lane (oldest-first).  Allocates.
    pub fn to_ordered_by_period(&self, period: usize) -> [Vec<T>; B] {
        if self.count == 0 || period == 0 {
            return core::array::from_fn(|_| Vec::new());
        }
        let take = period.min(self.count);
        core::array::from_fn(|lane| {
            (0..take)
                .map(|i| self.get_by_period(take - 1 - i)[lane])
                .collect()
        })
    }

    /// Convert a mirror-window index (0 = oldest) to a bars-ago value (0 = newest).
    #[inline(always)]
    pub fn window_index_to_bars_ago(&self, window_index: usize) -> usize {
        self.count - 1 - window_index
    }

    /// Contiguous mirror-window slices per lane: `vals[lane][index .. index + count - offset]`.
    ///
    /// Valid when `self.vals[lane]` was allocated as a mirror buffer (length `2 * capacity`).
    #[inline(always)]
    pub fn get_slices(&self, offset: usize) -> [&[T]; B] {
        if self.count == 0 {
            return core::array::from_fn(|_| [].as_slice());
        } else if self.count == self.capacity {
            return core::array::from_fn(|lane| unsafe {
                self.vals[lane].get_unchecked(self.index..self.index + self.count - offset)
            });
        }
        core::array::from_fn(|lane| unsafe {
            self.vals[lane].get_unchecked(0..self.count - offset)
        })
    }
}

// ── Cold-specific methods ──────────────────────────────────────────────

impl<const B: usize, T: BufferElement> MultiBuffer<B, T, Cold> {
    /// Create a new ring buffer with `capacity` slots per lane.
    pub fn new(capacity: usize) -> Self {
        Self {
            vals: core::array::from_fn(|_| vec![T::default(); capacity]),
            index: 0,
            prev_idx: 0,
            capacity,
            count: 0,
            state: std::marker::PhantomData,
        }
    }

    /// Build a ring buffer from per-lane slices (non-mirror, `capacity` slots per lane).
    pub fn from_slice(vals: [&[T]; B], capacity: usize) -> Self {
        let count = vals[0].len().min(capacity);
        let buffer_vals: [Vec<T>; B] = core::array::from_fn(|lane| {
            let mut vec = vals[lane].to_vec();
            if count < capacity {
                vec.resize(capacity, T::default());
            }
            vec
        });
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

    /// Push `values` into the ring buffer during the warmup phase.
    #[inline(always)]
    pub fn push(&mut self, values: [T; B]) {
        mb_write_values_ring(&mut self.vals, self.index, values);
        self.update_internals();
    }

    /// Push `values`, evicting and returning the oldest element once full.
    #[inline(always)]
    pub fn push_with_info(&mut self, values: [T; B]) -> Option<[T; B]> {
        if self.count == self.capacity {
            let replaced = mb_write_values_ring_pop(&mut self.vals, self.index, values);
            self.update_internals_unchecked();
            return Some(replaced);
        }
        mb_write_values_ring(&mut self.vals, self.index, values);
        self.update_internals();
        None
    }

    /// Oldest element, or `None` if empty.
    #[inline(always)]
    pub fn front(&self) -> Option<[T; B]> {
        if self.count == 0 {
            None
        } else {
            Some(self.get_values(self.index))
        }
    }

    /// Most recently pushed element, or `None` if empty.
    #[inline(always)]
    pub fn back(&self) -> Option<[T; B]> {
        if self.count == 0 {
            None
        } else {
            Some(self.get_values(self.prev_idx))
        }
    }

    /// Transition into a [`MultiBuffer<B, T, Warm>`].
    ///
    /// # Panics (debug builds)
    /// Panics when `debug_assertions` are enabled and `is_full()` is `false`.
    #[inline(always)]
    pub fn into_full(self) -> MultiBuffer<B, T, Warm> {
        debug_assert!(
            self.is_full(),
            "MultiBuffer::into_full called on a non-full buffer (count={}, capacity={})",
            self.count,
            self.capacity
        );
        MultiBuffer {
            vals: self.vals,
            index: self.index,
            capacity: self.capacity,
            count: self.count,
            prev_idx: self.prev_idx,
            state: std::marker::PhantomData,
        }
    }
}

// ── Warm-specific methods ─────────────────────────────────────────────────

impl<const B: usize, T: BufferElement> MultiBuffer<B, T, Warm> {
    /// Oldest element (always valid; buffer is guaranteed non-empty).
    #[inline(always)]
    pub fn front(&self) -> [T; B] {
        self.get_values(self.index)
    }

    /// Most recently pushed element (always valid; buffer is guaranteed non-empty).
    #[inline(always)]
    pub fn back(&self) -> [T; B] {
        self.get_values(self.prev_idx)
    }

    /// Push `values` and simultaneously read back the values that were at each of
    /// the specified `periods` *before* the push (1-based: `periods[i] = 1` means
    /// the element that was just evicted).
    ///
    /// Safe because the buffer is known to be full at the type level.
    #[inline(always)]
    pub fn push_with_info_periods<const N: usize>(
        &mut self,
        values: [T; B],
        periods: [usize; N],
    ) -> [[T; N]; B] {
        let idxs: [usize; N] =
            std::array::from_fn(|i| period_to_idx(self.index, self.capacity, periods[i] - 1));
        let results = get_by_periods(self, idxs);
        mb_write_values_ring(&mut self.vals, self.index, values);
        self.update_internals_unchecked();
        results
    }

    /// Push `values` — non-mirror, branchless (buffer guaranteed full).
    #[inline(always)]
    pub fn push(&mut self, values: [T; B]) {
        mb_write_values_ring(&mut self.vals, self.index, values);
        self.update_internals_unchecked();
    }

    /// Push `values`, evict and return the oldest element — non-mirror, branchless.
    #[inline(always)]
    pub fn push_with_info(&mut self, values: [T; B]) -> [T; B] {
        let replaced = mb_write_values_ring_pop(&mut self.vals, self.index, values);
        self.update_internals_unchecked();
        replaced
    }

    /// Raw slice for a single lane (ring order, order-independent uses).
    #[inline(always)]
    pub fn get_slice(&self, lane: usize) -> &[T] {
        &self.vals[lane]
    }

    /// Transition back to per-asset `Buffer<Warm>` slices (mirror layout preserved).
    pub fn to_single_buffers(&self) -> [SingleBuffer<Warm, T>; B] {
        std::array::from_fn(|i| SingleBuffer {
            index: self.index,
            count: self.count,
            prev_idx: self.prev_idx,
            capacity: self.capacity,
            vals: self.vals[i].clone(),
            state: std::marker::PhantomData,
        })
    }
}

// ── Index ──────────────────────────────────────────────────────────────────

impl<const B: usize, S, T: BufferElement> std::ops::Index<(usize, usize)> for MultiBuffer<B, T, S> {
    type Output = T;

    /// Index by `(bars_ago, lane)`.
    ///
    /// `buf[(0, lane)]` is the newest element of that lane; `buf[(count-1, lane)]` is the oldest.
    #[inline]
    fn index(&self, (bars_ago, lane): (usize, usize)) -> &T {
        assert!(lane < B, "lane {lane} out of bounds (B={B})");
        assert!(
            bars_ago < self.count,
            "index out of bounds: bars_ago {bars_ago} >= count {}",
            self.count
        );
        let idx = period_to_idx(self.index, self.capacity, bars_ago);
        &self.vals[lane][idx]
    }
}

// ── Iterator ──────────────────────────────────────────────────────────────────

/// Iterator produced by `(&MultiBuffer).into_iter()`.
///
/// Yields `[T; B]` tuples from **newest to oldest** (`buf[(0, _)]` first).
pub struct MultiBufferIter<'a, const B: usize, T: BufferElement, S> {
    buffer: &'a MultiBuffer<B, T, S>,
    /// Current position expressed as bars-ago (0 = newest).
    pos: usize,
}

impl<'a, const B: usize, S, T: BufferElement> Iterator for MultiBufferIter<'a, B, T, S> {
    type Item = [T; B];

    #[inline]
    fn next(&mut self) -> Option<[T; B]> {
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

impl<'a, const B: usize, S, T: BufferElement> ExactSizeIterator for MultiBufferIter<'a, B, T, S> {}

impl<'a, const B: usize, S, T: BufferElement> IntoIterator for &'a MultiBuffer<B, T, S> {
    type Item = [T; B];
    type IntoIter = MultiBufferIter<'a, B, T, S>;

    /// Iterate from newest to oldest (`buf[(0, _)]` first).
    #[inline]
    fn into_iter(self) -> MultiBufferIter<'a, B, T, S> {
        MultiBufferIter {
            buffer: self,
            pos: 0,
        }
    }
}

// ── Module-level free functions ────────────────────────────────────────────

#[inline(always)]
pub fn get_by_periods<const N: usize, const B: usize, S, T: BufferElement>(
    buffer: &MultiBuffer<B, T, S>,
    idxs: [usize; N],
) -> [[T; N]; B] {
    let mut results = [[T::default(); N]; B];

    for (buffer, buffer_results) in buffer.vals.iter().zip(results.iter_mut()) {
        for (&buffer_idx, results_value) in idxs.iter().zip(buffer_results.iter_mut()) {
            *results_value = unsafe { *buffer.get_unchecked(buffer_idx) }
        }
    }

    results
}

// ── pub(super) primitive helpers shared with multi_mirror_buffer ───────────────────────
/// Compute the next ring index without branching.
#[inline(always)]
pub(super) fn mb_next_index(index: usize, capacity: usize) -> usize {
    let next = index + 1;
    if next == capacity {
        0
    } else {
        next
    }
}

/// Advance index/prev_idx/count (used by Cold buffers that check for full).
#[inline(always)]
pub(super) fn mb_advance(
    index: &mut usize,
    prev_idx: &mut usize,
    count: &mut usize,
    capacity: usize,
) {
    *prev_idx = *index;
    *index = mb_next_index(*index, capacity);
    if *count != capacity {
        *count += 1;
    }
}

/// Advance index/prev_idx without touching count (used by Warm buffers, always full).
#[inline(always)]
pub(super) fn mb_advance_unchecked(index: &mut usize, prev_idx: &mut usize, capacity: usize) {
    *prev_idx = *index;
    *index = mb_next_index(*index, capacity);
}

/// Read values from all `B` lanes at `idx`.
#[inline(always)]
pub(super) fn mb_get_values<const B: usize, T: BufferElement>(
    vals: &[Vec<T>; B],
    idx: usize,
) -> [T; B] {
    let mut results = [T::default(); B];
    for (buffer, result) in vals.iter().zip(results.iter_mut()) {
        *result = unsafe { *buffer.get_unchecked(idx) };
    }
    results
}

/// Write `values` to all `B` lanes at `index` (ring, single slot).
#[inline(always)]
pub(super) fn mb_write_values_ring<const B: usize, T: BufferElement>(
    vals: &mut [Vec<T>; B],
    index: usize,
    values: [T; B],
) {
    for (buff, &val) in vals.iter_mut().zip(values.iter()) {
        unsafe {
            *buff.get_unchecked_mut(index) = val;
        }
    }
}

/// Write `values` to all `B` lanes at `index`, returning the evicted values (ring).
#[inline(always)]
pub(super) fn mb_write_values_ring_pop<const B: usize, T: BufferElement>(
    vals: &mut [Vec<T>; B],
    index: usize,
    values: [T; B],
) -> [T; B] {
    let mut results = [T::default(); B];
    for ((buff, &val), result) in vals.iter_mut().zip(values.iter()).zip(results.iter_mut()) {
        *result = unsafe { *buff.get_unchecked(index) };
        unsafe {
            *buff.get_unchecked_mut(index) = val;
        }
    }
    results
}

/// Write `values` to both the primary slot and mirror copy for all `B` lanes.
#[inline(always)]
pub(super) fn mb_write_values_mirror<const B: usize, T: BufferElement>(
    vals: &mut [Vec<T>; B],
    index: usize,
    capacity: usize,
    values: [T; B],
) {
    for (buff, &val) in vals.iter_mut().zip(values.iter()) {
        unsafe {
            *buff.get_unchecked_mut(index) = val;
        }
        unsafe {
            *buff.get_unchecked_mut(index + capacity) = val;
        }
    }
}

/// Write mirror values and return the evicted values for all `B` lanes.
#[inline(always)]
pub(super) fn mb_write_values_mirror_pop<const B: usize, T: BufferElement>(
    vals: &mut [Vec<T>; B],
    index: usize,
    capacity: usize,
    values: [T; B],
) -> [T; B] {
    let mut results = [T::default(); B];
    for ((buff, &val), result) in vals.iter_mut().zip(values.iter()).zip(results.iter_mut()) {
        *result = unsafe { *buff.get_unchecked(index) };
        unsafe {
            *buff.get_unchecked_mut(index) = val;
        }
        unsafe {
            *buff.get_unchecked_mut(index + capacity) = val;
        }
    }
    results
}
