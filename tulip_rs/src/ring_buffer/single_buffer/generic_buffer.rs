//use crate::indicators::max::{find_max_scalar, find_max_simd, State as MaxState};
//use crate::indicators::min::{find_min_scalar, find_min_simd, State as MinState};
use crate::ring_buffer::buffer::period_to_idx;
#[cfg(feature = "portable_simd")]
pub use crate::ring_buffer::{
    buffer::BufferElement,
    single_buffer::{
        mirror_buffer::MirrorBuffer,
        ring_buffer::RingBuffer,
        simd_buffer::{SimdBuffer, SimdMirrorBuffer, SimdRingBuffer},
    },
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
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
            return None
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
            return None
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
    pub pos: usize,
    pub current_idx: usize, // Pre-computed starting index
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
