pub use crate::ring_buffer::buffer::BufferElement;
use crate::ring_buffer::buffer::{period_to_idx, SerdeElement};
use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, Cold, Warm};
use serde::{Deserialize, Serialize};
use std::simd::{cmp::SimdPartialEq, Mask, Select, Simd, SimdElement};

pub struct F64Constants<const N: usize>;
impl<const N: usize> F64Constants<N> {
    pub const ZERO: Simd<f64, N> = Simd::splat(0.0);
    pub const ONE: Simd<f64, N> = Simd::splat(1.0);
}

pub struct UsizeConstants<const N: usize>;
impl<const N: usize> UsizeConstants<N> {
    pub const ZERO: Simd<usize, N> = Simd::splat(0);
    pub const ONE: Simd<usize, N> = Simd::splat(1);
}

/// Unsynchronized multi-lane buffer backed by per-lane Vec<T>.
///
/// The `S` parameter encodes fill state at the type level:
/// * [`Cold`] — warmup phase; `front()` / `back()` return `(Simd<T, B>, Mask<i64, B>)`.
/// * [`Warm`]    — operational phase; `front()` / `back()` return `Simd<T, B>` (infallible).
///
/// Transition from `Cold` to `Warm` via [`UnsyncBuffer::into_full`].
///
/// We implement custom Serialize/Deserialize because `Simd<usize, B>` does not
/// implement Serde traits; we convert the simd lanes to plain Vec<usize> for
/// (de)serialization. Val lanes go through T::Repr for the same reason.
pub struct UnsyncBuffer<const B: usize, T: BufferElement + SimdElement, S = Cold> {
    pub(crate) vals: [Vec<T>; B],
    pub(crate) index: Simd<usize, B>,
    pub(crate) capacity: Simd<usize, B>,
    pub(crate) count: Simd<usize, B>,
    pub(crate) prev_idx: Simd<usize, B>,
    pub(crate) state: std::marker::PhantomData<S>,
}

// ── Shared methods (valid for any fill state) ──────────────────────────────

impl<const B: usize, S, T: BufferElement + SimdElement> UnsyncBuffer<B, T, S> {
    #[inline(always)]
    pub(crate) fn calc_index(&self) -> Simd<usize, B> {
        let new_idx = self.index + UsizeConstants::ONE;
        new_idx
            .simd_eq(self.capacity)
            .select(UsizeConstants::ZERO, new_idx)
    }

    #[inline(always)]
    pub(crate) fn update_internals_unchecked(&mut self) {
        self.prev_idx = self.index;
        self.index = self.calc_index();
        // intentionally do not modify count here
    }

    #[inline(always)]
    pub(crate) fn get_values(&self, idx: Simd<usize, B>) -> Simd<T, B> {
        let idx = idx.as_array();
        let mut results = Simd::splat(T::default());
        for ((buffer, result), &idx) in self
            .vals
            .iter()
            .zip(results.as_mut_array().iter_mut())
            .zip(idx.iter())
        {
            *result = unsafe { *buffer.get_unchecked(idx) };
        }
        results
    }

    pub fn raw_slice(&self) -> &[Vec<T>; B] {
        &self.vals
    }

    #[inline(always)]
    pub fn get_count(&self) -> Simd<usize, B> {
        self.count
    }

    pub fn get_idx(&self) -> Simd<usize, B> {
        self.index
    }

    #[inline(always)]
    pub fn is_full(&self) -> Mask<i64, B> {
        self.count.simd_eq(self.capacity).cast::<i64>()
    }

    pub fn get_prev_idx(&self) -> Simd<usize, B> {
        self.prev_idx
    }

    pub fn get_capacity(&self) -> Simd<usize, B> {
        self.capacity
    }

    /// Return an iterator over a single `lane`, from newest to oldest.
    ///
    /// # Panics
    /// Panics if `lane >= B`.
    #[inline]
    pub fn lane_iter(&self, lane: usize) -> UnsyncLaneIter<'_, B, T, S> {
        assert!(lane < B, "lane {lane} out of bounds (B={B})");
        UnsyncLaneIter {
            buffer: self,
            lane,
            pos: 0,
            count: self.count[lane],
        }
    }

    /// Contiguous mirror-window slices per lane: `vals[lane][index[lane] .. index[lane] + count[lane] - offset]`.
    /// Valid when each `vals[lane]` was allocated as a mirror buffer (length `2 * capacity[lane]`).
    #[inline(always)]
    pub fn get_slices(&self, offset: usize) -> [&[T]; B] {
        std::array::from_fn(|lane| {
            if self.count[lane] == 0 {
                return &[] as &[T];
            } else if self.count[lane] == self.capacity[lane] {
                return unsafe {
                    self.vals[lane].get_unchecked(
                        self.index[lane]..self.index[lane] + self.count[lane] - offset,
                    )
                };
            }
            unsafe { self.vals[lane].get_unchecked(0..self.count[lane] - offset) }
        })
    }

    /// Convert a mirror-window index (0 = oldest) for a given lane to bars-ago (0 = newest).
    #[inline(always)]
    pub fn window_index_to_bars_ago(&self, window_index: usize, lane: usize) -> usize {
        self.count[lane] - 1 - window_index
    }
}

// ── Cold-specific methods ────────────────────────────────────────────────

impl<const B: usize, T: BufferElement + SimdElement> UnsyncBuffer<B, T, Cold> {
    pub fn new(capacity: [usize; B]) -> Self {
        let vals = core::array::from_fn(|i| vec![T::default(); capacity[i]]);
        Self {
            vals,
            index: Simd::splat(0),
            prev_idx: Simd::splat(0),
            capacity: Simd::from_array(capacity),
            count: Simd::splat(0),
            state: std::marker::PhantomData,
        }
    }

    pub(crate) fn from_f64_buffers(buffers: Vec<&Buffer<Warm, T>>) -> UnsyncBuffer<B, T, Warm> {
        let mut index = [0usize; B];
        let mut prev_idx = [0usize; B];
        let mut capacity = [0usize; B];
        let mut count = [0usize; B];
        let vals: [Vec<T>; B] = std::array::from_fn(|lane| buffers[lane].vals.clone());
        for (lane, buffer) in buffers.iter().enumerate() {
            index[lane] = buffer.index;
            prev_idx[lane] = buffer.prev_idx;
            count[lane] = buffer.count;
            capacity[lane] = buffer.capacity;
        }
        UnsyncBuffer {
            vals,
            index: Simd::from_array(index),
            prev_idx: Simd::from_array(prev_idx),
            count: Simd::from_array(count),
            capacity: Simd::from_array(capacity),
            state: std::marker::PhantomData::<Warm>,
        }
    }

    #[inline(always)]
    pub fn front(&self) -> (Simd<T, B>, Mask<i64, B>) {
        (self.get_values(self.index), self.is_full())
    }

    #[inline(always)]
    pub fn front_unchecked(&self) -> Simd<T, B> {
        self.get_values(self.index)
    }

    #[inline(always)]
    pub fn back(&self) -> (Simd<T, B>, Mask<i64, B>) {
        (self.get_values(self.prev_idx), self.is_full())
    }

    #[inline(always)]
    pub fn back_unchecked(&self) -> Simd<T, B> {
        self.get_values(self.prev_idx)
    }

    /// Transition into an [`UnsyncBuffer<B, T, Warm>`].
    ///
    /// # Panics (debug builds)
    /// Panics when `debug_assertions` are enabled and not all lanes are full.
    #[inline(always)]
    pub fn into_full(self) -> UnsyncBuffer<B, T, Warm> {
        debug_assert!(
            self.count.simd_eq(self.capacity).to_bitmask() == (1u64 << B) - 1,
            "UnsyncBuffer::into_full: not all lanes are full"
        );
        UnsyncBuffer {
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

impl<const B: usize, T: BufferElement + SimdElement> UnsyncBuffer<B, T, Warm> {
    pub fn new(capacity: [usize; B]) -> Self {
        let vals = core::array::from_fn(|i| vec![T::default(); capacity[i]]);
        let capacity_simd = Simd::from_array(capacity);
        Self {
            vals,
            index: Simd::splat(0),
            prev_idx: capacity_simd - Simd::splat(1),
            capacity: capacity_simd,
            count: capacity_simd,
            state: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub fn push(&mut self, values: Simd<T, B>) {
        write_values(self, values);
        self.update_internals_unchecked();
    }

    #[inline(always)]
    pub fn push_with_info(&mut self, values: Simd<T, B>) -> (Simd<T, B>, Mask<i64, B>) {
        let replaced = write_values_pop(self, values);
        self.update_internals_unchecked();
        (replaced, Mask::splat(true))
    }

    #[inline(always)]
    pub fn get_slice(&self, lane: usize) -> &[T] {
        &self.vals[lane]
    }

    /// Oldest element (all lanes guaranteed non-empty — buffer is full).
    #[inline(always)]
    pub fn front(&self) -> Simd<T, B> {
        self.get_values(self.index)
    }

    /// Most recently pushed element (all lanes guaranteed non-empty — buffer is full).
    #[inline(always)]
    pub fn back(&self) -> Simd<T, B> {
        self.get_values(self.prev_idx)
    }

    /// Split this SIMD unsync buffer into `B` scalar `Buffer<Warm, T>` instances (one per lane).
    pub(crate) fn to_f64_buffers(&self) -> Vec<Buffer<Warm, T>> {
        let mut buffers = Vec::with_capacity(B);
        for (lane, vals) in self.vals.iter().enumerate() {
            buffers.push(Buffer::<Warm, T> {
                vals: vals.to_vec(),
                index: self.index[lane],
                prev_idx: self.prev_idx[lane],
                capacity: self.capacity[lane],
                count: self.count[lane],
                state: std::marker::PhantomData::<Warm>,
            });
        }
        buffers
    }

    /// Merge `B` scalar `Buffer<Warm, T>` instances back into an `UnsyncBuffer<B, T, Warm>`.
    pub(crate) fn from_f64_buffers(buffers: Vec<&Buffer<Warm, T>>) -> UnsyncBuffer<B, T, Warm> {
        let mut index = [0usize; B];
        let mut prev_idx = [0usize; B];
        let mut capacity = [0usize; B];
        let mut count = [0usize; B];
        let vals: [Vec<T>; B] = std::array::from_fn(|lane| buffers[lane].vals.clone());
        for (lane, buffer) in buffers.iter().enumerate() {
            index[lane] = buffer.index;
            prev_idx[lane] = buffer.prev_idx;
            count[lane] = buffer.count;
            capacity[lane] = buffer.capacity;
        }
        UnsyncBuffer {
            vals,
            index: Simd::from_array(index),
            prev_idx: Simd::from_array(prev_idx),
            count: Simd::from_array(count),
            capacity: Simd::from_array(capacity),
            state: std::marker::PhantomData::<Warm>,
        }
    }
}

// ── Index ──────────────────────────────────────────────────────────────────

impl<const B: usize, S, T: BufferElement + SimdElement> std::ops::Index<(usize, usize)>
    for UnsyncBuffer<B, T, S>
{
    type Output = T;

    /// Index by `(bars_ago, lane)`.
    ///
    /// `buf[(0, lane)]` is the newest element of that lane; `buf[(count[lane]-1, lane)]`
    /// is the oldest. Each lane's valid range is `0..count[lane]`.
    #[inline]
    fn index(&self, (bars_ago, lane): (usize, usize)) -> &T {
        assert!(lane < B, "lane {lane} out of bounds (B={B})");
        let count = self.count[lane];
        assert!(
            bars_ago < count,
            "index out of bounds: bars_ago {bars_ago} >= count {count} for lane {lane}"
        );
        let idx = period_to_idx(self.index[lane], self.capacity[lane], bars_ago);
        &self.vals[lane][idx]
    }
}

// ── Per-lane iterator ──────────────────────────────────────────────────────────

/// Iterator over a single lane of an [`UnsyncBuffer`].
///
/// Yields elements from **newest to oldest** (bars-ago order).
/// Obtain via [`UnsyncBuffer::lane_iter`].
pub struct UnsyncLaneIter<'a, const B: usize, T: BufferElement + SimdElement, S = Cold> {
    buffer: &'a UnsyncBuffer<B, T, S>,
    lane: usize,
    /// Current position expressed as bars-ago (0 = newest).
    pos: usize,
    count: usize,
}

impl<'a, const B: usize, S, T: BufferElement + SimdElement> Iterator
    for UnsyncLaneIter<'a, B, T, S>
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        if self.pos >= self.count {
            return None;
        }
        let idx = period_to_idx(
            self.buffer.index[self.lane],
            self.buffer.capacity[self.lane],
            self.pos,
        );
        let val = unsafe { *self.buffer.vals[self.lane].get_unchecked(idx) };
        self.pos += 1;
        Some(val)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count.saturating_sub(self.pos);
        (remaining, Some(remaining))
    }
}

impl<'a, const B: usize, S, T: BufferElement + SimdElement> ExactSizeIterator
    for UnsyncLaneIter<'a, B, T, S>
{
}

// Helper struct for serialization: converts SIMD index fields to Vec<usize>
// and val lanes through T::Repr for Serde compatibility.
#[derive(Serialize, Deserialize)]
struct MultiBufferSerde<R> {
    vals: Vec<Vec<R>>,
    index: Vec<usize>,
    capacity: Vec<usize>,
    count: Vec<usize>,
    prev_idx: Vec<usize>,
}

impl<const B: usize, S, T: BufferElement + SerdeElement + SimdElement> Serialize
    for UnsyncBuffer<B, T, S>
{
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: serde::Serializer,
    {
        let helper = MultiBufferSerde {
            vals: self
                .vals
                .iter()
                .map(|lane| lane.iter().map(|v| T::to_repr(*v)).collect())
                .collect(),
            index: self.index.to_array().into_iter().collect(),
            capacity: self.capacity.to_array().into_iter().collect(),
            count: self.count.to_array().into_iter().collect(),
            prev_idx: self.prev_idx.to_array().into_iter().collect(),
        };
        helper.serialize(serializer)
    }
}

impl<'de, const B: usize, S, T: BufferElement + SerdeElement + SimdElement> Deserialize<'de>
    for UnsyncBuffer<B, T, S>
where
    T::Repr: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = MultiBufferSerde::<T::Repr>::deserialize(deserializer)
            .map_err(|e| serde::de::Error::custom(format!("helper deserialize failed: {}", e)))?;

        if helper.vals.len() != B {
            return Err(serde::de::Error::custom(format!(
                "Expected {} buffers, got {}",
                B,
                helper.vals.len()
            )));
        }

        let index_arr: [usize; B] = helper
            .index
            .try_into()
            .map_err(|_| serde::de::Error::custom("index length mismatch"))?;
        let capacity_arr: [usize; B] = helper
            .capacity
            .try_into()
            .map_err(|_| serde::de::Error::custom("capacity length mismatch"))?;
        let count_arr: [usize; B] = helper
            .count
            .try_into()
            .map_err(|_| serde::de::Error::custom("count length mismatch"))?;
        let prev_arr: [usize; B] = helper
            .prev_idx
            .try_into()
            .map_err(|_| serde::de::Error::custom("prev_idx length mismatch"))?;

        let vals_array: [Vec<T>; B] = helper
            .vals
            .into_iter()
            .map(|lane| lane.into_iter().map(T::from_repr).collect())
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| serde::de::Error::custom("Failed to convert vals to array"))?;

        Ok(UnsyncBuffer {
            vals: vals_array,
            index: Simd::from_array(index_arr),
            capacity: Simd::from_array(capacity_arr),
            count: Simd::from_array(count_arr),
            prev_idx: Simd::from_array(prev_arr),
            state: std::marker::PhantomData,
        })
    }
}

// ── Shared inner helpers for UnsyncMirrorBuffer ─────────────────────────────

#[inline(always)]
pub(super) fn usb_next_index<const B: usize>(
    index: Simd<usize, B>,
    capacity: Simd<usize, B>,
) -> Simd<usize, B> {
    let new_idx = index + Simd::splat(1);
    new_idx.simd_eq(capacity).select(Simd::splat(0), new_idx)
}

#[inline(always)]
pub(super) fn usb_advance<const B: usize>(
    buf_index: &mut Simd<usize, B>,
    prev_idx: &mut Simd<usize, B>,
    count: &mut Simd<usize, B>,
    capacity: Simd<usize, B>,
) {
    *prev_idx = *buf_index;
    *buf_index = usb_next_index(*buf_index, capacity);
    let at_cap = count.simd_eq(capacity);
    *count = at_cap.select(*count, *count + Simd::splat(1));
}

#[inline(always)]
pub(super) fn usb_advance_unchecked<const B: usize>(
    buf_index: &mut Simd<usize, B>,
    prev_idx: &mut Simd<usize, B>,
    capacity: Simd<usize, B>,
) {
    *prev_idx = *buf_index;
    *buf_index = usb_next_index(*buf_index, capacity);
}

#[inline(always)]
pub(super) fn usb_get_values<const B: usize, T: BufferElement + SimdElement>(
    vals: &[Vec<T>; B],
    idx: Simd<usize, B>,
) -> Simd<T, B> {
    let idx = idx.as_array();
    let mut results = Simd::splat(T::default());
    for ((buffer, result), &idx) in vals
        .iter()
        .zip(results.as_mut_array().iter_mut())
        .zip(idx.iter())
    {
        *result = unsafe { *buffer.get_unchecked(idx) };
    }
    results
}

#[inline(always)]
pub(super) fn usb_write_mirror<const B: usize, T: BufferElement + SimdElement>(
    vals: &mut [Vec<T>; B],
    index: &Simd<usize, B>,
    capacity: &Simd<usize, B>,
    values: Simd<T, B>,
) {
    let idx = index.as_array();
    let cap = capacity.to_array();
    for (((buff, &val), &idx), &cap) in vals
        .iter_mut()
        .zip(values.as_array().iter())
        .zip(idx.iter())
        .zip(cap.iter())
    {
        unsafe {
            *buff.get_unchecked_mut(idx) = val;
        }
        unsafe {
            *buff.get_unchecked_mut(idx + cap) = val;
        }
    }
}

#[inline(always)]
pub(super) fn usb_write_mirror_pop<const B: usize, T: BufferElement + SimdElement>(
    vals: &mut [Vec<T>; B],
    index: &Simd<usize, B>,
    capacity: &Simd<usize, B>,
    values: Simd<T, B>,
) -> Simd<T, B> {
    let idx = index.as_array();
    let cap = capacity.to_array();
    let mut results = Simd::splat(T::default());
    for ((((buff, &val), result), &idx), &cap) in vals
        .iter_mut()
        .zip(values.as_array().iter())
        .zip(results.as_mut_array().iter_mut())
        .zip(idx.iter())
        .zip(cap.iter())
    {
        *result = unsafe { *buff.get_unchecked(idx) };
        unsafe {
            *buff.get_unchecked_mut(idx) = val;
        }
        unsafe {
            *buff.get_unchecked_mut(idx + cap) = val;
        }
    }
    results
}

#[inline(always)]
pub(crate) fn write_values<const B: usize, S, T: BufferElement + SimdElement>(
    buffer: &mut UnsyncBuffer<B, T, S>,
    values: Simd<T, B>,
) {
    let idx = buffer.index.as_array();
    for ((buff, &vals), &idx) in buffer
        .vals
        .iter_mut()
        .zip(values.as_array().iter())
        .zip(idx.iter())
    {
        unsafe { *buff.get_unchecked_mut(idx) = vals }
    }
}

#[inline(always)]
pub(crate) fn write_values_pop<const B: usize, S, T: BufferElement + SimdElement>(
    buffer: &mut UnsyncBuffer<B, T, S>,
    values: Simd<T, B>,
) -> Simd<T, B> {
    let idx = buffer.index.as_array();
    let mut results = Simd::splat(T::default());
    for (((buff, &vals), result), &idx) in buffer
        .vals
        .iter_mut()
        .zip(values.as_array().iter())
        .zip(results.as_mut_array().iter_mut())
        .zip(idx.iter())
    {
        *result = unsafe { *buff.get_unchecked(idx) };
        unsafe { *buff.get_unchecked_mut(idx) = vals }
    }
    results
}
