use crate::ring_buffer::buffer::SerdeElement;
use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
pub use crate::ring_buffer::{
    buffer::BufferElement,
    unsync_multi_buffer::{mirror_buffer::MirrorBuffer, ring_buffer::RingBuffer},
};
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
/// We implement custom Serialize/Deserialize because `Simd<usize, B>` does not
/// implement Serde traits; we convert the simd lanes to plain Vec<usize> for
/// (de)serialization. Val lanes go through T::Repr for the same reason.
pub struct UnsyncBuffer<const B: usize, T: BufferElement + SimdElement> {
    pub(crate) vals: [Vec<T>; B],
    pub(crate) index: Simd<usize, B>,
    pub(crate) capacity: Simd<usize, B>,
    pub(crate) count: Simd<usize, B>,
    pub(crate) prev_idx: Simd<usize, B>,
}

impl<const B: usize, T: BufferElement + SimdElement> UnsyncBuffer<B, T> {
    pub(crate) fn to_f64_buffers(&self) -> Vec<Buffer<T>> {
        let mut buffers = Vec::with_capacity(B);
        for (lane, vals) in self.vals.iter().enumerate() {
            buffers.push(Buffer::<T> {
                vals: vals.to_vec(),
                index: self.index[lane],
                prev_idx: self.prev_idx[lane],
                capacity: self.capacity[lane],
                count: self.count[lane],
            });
        }
        buffers
    }
    pub(crate) fn from_buffers(buffers: Vec<&Buffer<T>>) -> Self {
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
        Self {
            vals,
            index: Simd::from_array(index),
            prev_idx: Simd::from_array(prev_idx),
            count: Simd::from_array(count),
            capacity: Simd::from_array(capacity),
        }
    }
    #[inline(always)]
    pub(crate) fn update_internals(&mut self) {
        self.prev_idx = self.index;
        self.index = self.calc_index();
        self.count = self
            .count
            .simd_eq(self.capacity)
            .select(self.count, self.count + UsizeConstants::ONE);
    }

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
    fn get_values(&self, idx: Simd<usize, B>) -> Simd<T, B> {
        let idx = idx.as_array(); //idx.to_array();
        let mut results = Simd::splat(T::default());
        // zip buffers and results; iteration stops at the shorter of the two
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

impl<const B: usize, T: BufferElement + SerdeElement + SimdElement> Serialize
    for UnsyncBuffer<B, T>
{
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
            index: self.index.to_array().into_iter().collect(),
            capacity: self.capacity.to_array().into_iter().collect(),
            count: self.count.to_array().into_iter().collect(),
            prev_idx: self.prev_idx.to_array().into_iter().collect(),
        };
        helper.serialize(serializer)
    }
}

impl<'de, const B: usize, T: BufferElement + SerdeElement + SimdElement> Deserialize<'de>
    for UnsyncBuffer<B, T>
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
        })
    }
}

#[inline(always)]
pub(crate) fn write_values<const B: usize, T: BufferElement + SimdElement>(
    buffer: &mut UnsyncBuffer<B, T>,
    values: Simd<T, B>,
) {
    let idx = buffer.index.as_array(); //.to_array();
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
pub(crate) fn write_values_pop<const B: usize, T: BufferElement + SimdElement>(
    buffer: &mut UnsyncBuffer<B, T>,
    values: Simd<T, B>,
) -> Simd<T, B> {
    let idx = buffer.index.as_array(); //.to_array();
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

/*#[inline(always)]
pub(crate) fn period_to_idx<const N: usize>(
    index: Simd<usize, N>,
    capacity: Simd<usize, N>,
    periods: Simd<usize, N>,
) -> Simd<usize, N>
where
    LaneCount<N>: SupportedLaneCount,
{
    // index - periods - 1  (with wrap-around handled by adding capacity when negative)
    let mut idx = index.cast::<i32>() - periods.cast::<i32>() - UsizeConstants::ONE.cast::<i32>();
    idx = idx
        .simd_le(UsizeConstants::ZERO.cast::<i32>())
        .select(idx + capacity.cast::<i32>(), idx);

    idx.cast::<usize>()
}*/
