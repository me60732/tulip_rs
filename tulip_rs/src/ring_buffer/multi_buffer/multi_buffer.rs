pub use crate::ring_buffer::{buffer::BufferElement, multi_buffer::{ring_buffer::RingBuffer, mirror_buffer::MirrorBuffer, simd_buffer::{SimdRingBuffer, SimdBuffer}}};
use crate::ring_buffer::buffer::period_to_idx;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct MultiBuffer<const B: usize, T: BufferElement = f64> {
    pub(crate) vals: [Vec<T>; B],
    pub(crate) index: usize,
    pub(crate) capacity: usize,
    pub(crate) count: usize,
    pub(crate) prev_idx: usize,
}
// Helper struct for serialization
#[derive(Serialize, Deserialize)]
struct MultiBufferSerde<T> {
    vals: Vec<Vec<T>>,
    index: usize,
    capacity: usize,
    count: usize,
    prev_idx: usize,
}

impl<const N: usize, T> Serialize for MultiBuffer<N, T>
where
    T: BufferElement + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let helper = MultiBufferSerde {
            vals: self.vals.iter().cloned().collect(),
            index: self.index,
            capacity: self.capacity,
            count: self.count,
            prev_idx: self.prev_idx,
        };
        helper.serialize(serializer)
    }
}

impl<'de, const N: usize, T> Deserialize<'de> for MultiBuffer<N, T>
where
    T: BufferElement + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = MultiBufferSerde::<T>::deserialize(deserializer)?;

        if helper.vals.len() != N {
            return Err(serde::de::Error::custom(format!(
                "Expected {} buffers, got {}",
                N,
                helper.vals.len()
            )));
        }

        let vals_array: [Vec<T>; N] = helper
            .vals
            .try_into()
            .map_err(|_| serde::de::Error::custom("Failed to convert to array"))?;

        Ok(MultiBuffer {
            vals: vals_array,
            index: helper.index,
            capacity: helper.capacity,
            count: helper.count,
            prev_idx: helper.prev_idx,
        })
    }
}
impl<const B: usize, T: BufferElement> MultiBuffer<B, T> {
    
    #[inline(always)]
    fn get_values(&self, idx: usize) -> [T; B] {
        let mut results = [T::default(); B];
        for (buffer, result) in self.vals.iter().zip(results.iter_mut()) {
            *result = unsafe { *buffer.get_unchecked(idx) };
        }
        results
    }
    #[inline(always)]
    pub fn front(&self) -> Option<[T; B]> {
        if self.count == 0 {
            None
        } else {
            Some(self.get_values(self.index))
        }
    }

    #[inline(always)]
    pub unsafe fn front_unchecked(&self) -> [T; B] {
        self.get_values(self.index)
    }
    #[inline(always)]
    pub fn back(&self) -> Option<[T; B]> {
        if self.count == 0 {
            None
        } else {
            Some(self.get_values(self.prev_idx))
        }
    }

    #[inline(always)]
    pub unsafe fn back_unchecked(&self) -> [T; B] {
        self.get_values(self.prev_idx)
    }
    #[inline(always)]
    pub fn get_by_period(&self, period: usize) -> [T; B] {
        let idx = period_to_idx(self.index, self.capacity, period);
        self.get_values(idx)
    }

    #[inline(always)]
    pub fn get_by_periods<const N: usize>(&self, periods: [usize; N]) -> [[T; N]; B]{
        let idxs: [usize; N] = std::array::from_fn(|i| period_to_idx(self.index, self.capacity, periods[i]));
        get_by_periods(self, idxs)
    }
    
    #[inline(always)]
    pub(crate) fn update_internals(&mut self) {
        self.prev_idx = self.index;
        self.index = self.calc_index();
        if self.count != self.capacity {
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
    #[inline(always)]
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

    pub fn raw_slice(&self) -> &[Vec<T>; B] {
        &self.vals
    }
}
#[inline(always)]
pub fn get_by_periods<const N: usize, const B: usize, T: BufferElement>(buffer: &MultiBuffer<B, T>, idxs: [usize; N]) -> [[T; N]; B] {
    let mut results = [[T::default(); N]; B];
    
    for (buffer, buffer_results) in buffer.vals.iter().zip(results.iter_mut()) {
        for (&buffer_idx, results_value) in idxs.iter().zip(buffer_results.iter_mut()) {
            *results_value = unsafe { *buffer.get_unchecked(buffer_idx) }
        }
    }
    
    results
}

#[inline(always)]
pub(crate) fn write_values<const B: usize, T: BufferElement>(
    buffer: &mut MultiBuffer<B, T>,
    values: [T; B],
) {
    for (buff, &vals) in buffer.vals.iter_mut().zip(values.iter()) {
        unsafe { *buff.get_unchecked_mut(buffer.index) = vals }
    }
}
#[inline(always)]
pub(crate) fn write_values_pop<const B: usize, T: BufferElement>(
    buffer: &mut MultiBuffer<B, T>,
    values: [T; B],
) -> [T; B] {
    let mut results = [T::default(); B];
    for ((buff, &vals), result) in buffer
        .vals
        .iter_mut()
        .zip(values.iter())
        .zip(results.iter_mut())
    {
        *result = unsafe { *buff.get_unchecked(buffer.index) };
        unsafe { *buff.get_unchecked_mut(buffer.index) = vals }
    }
    results
}