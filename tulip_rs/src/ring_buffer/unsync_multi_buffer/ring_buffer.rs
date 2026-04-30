use std::simd::{Simd, Mask, SimdElement};
use crate::ring_buffer::unsync_multi_buffer::multi_buffer::{UnsyncBuffer, write_values, write_values_pop, BufferElement};

pub trait RingBuffer<const B: usize, T: SimdElement + BufferElement = f64>{
    fn new(capacity: [usize; B]) -> Self;
    unsafe fn push_unchecked(&mut self, values: Simd<T, B>);
    unsafe fn push_by_lane_unchecked(&mut self, value: T, lane: usize);
    fn push(&mut self, values: Simd<T, B>);
    fn push_by_lane(&mut self, value: T, lane: usize);
    fn push_with_info(&mut self, values: Simd<T, B>) -> (Simd<T, B>, Mask<i64, B>);
    fn push_with_info_by_lane(&mut self, values: T, lane: usize) -> Option<T>;
    unsafe fn push_with_info_unchecked(&mut self, values: Simd<T, B>) -> Simd<T, B>;
    unsafe fn push_with_info_by_lane_unchecked(&mut self, value: T, lane: usize) -> T;
    fn get_slice(&self, lane: usize) -> &[T];
    fn from_slice(vals: [&[T]; B], capacity: [usize; B]) -> Self;
}

impl<const B: usize, T: BufferElement + SimdElement> RingBuffer<B, T> for UnsyncBuffer<B, T> {
    fn new(capacity: [usize; B]) -> Self {
        let vals = core::array::from_fn(|i| vec![T::default(); capacity[i]]);
        
        Self {
            // Preallocate with default values
            vals: vals,
            index: Simd::splat(0),
            prev_idx: Simd::splat(0),
            capacity: Simd::from_array(capacity),
            count: Simd::splat(0),
        }
    }
    fn from_slice(vals: [&[T]; B], capacity: [usize; B]) -> Self {
        let count = core::array::from_fn(|i| vals[i].len().min(capacity[i]));
        let count_simd = Simd::from_array(count);
        let capacity_simd = Simd::from_array(capacity);
        let buffer_vals: [Vec<T>; B] = core::array::from_fn(|lane| {
            let mut vec = vals[lane].to_vec();
            if count[lane] < capacity[lane] {
                vec.resize(capacity[lane], T::default());
            }
            vec
        });
        let index = count_simd % capacity_simd;
        let prev_idx = (index + capacity_simd - Simd::splat(1)) % capacity_simd;
        Self {
            vals: buffer_vals,
            index: index,
            prev_idx,
            capacity: capacity_simd,
            count: count_simd,
        }
    }
    #[inline(always)]
    fn push(&mut self, values: Simd<T, B>) {
        write_values(self, values);
        self.update_internals();
    }
    #[inline(always)]
    fn push_by_lane(&mut self, value: T, lane: usize) {
        unsafe { *self.vals[lane].get_unchecked_mut(self.index[lane]) = value };
        self.update_internals();
    }
    #[inline(always)]
    unsafe fn push_by_lane_unchecked(&mut self, value: T, lane: usize) {
        unsafe { *self.vals[lane].get_unchecked_mut(self.index[lane]) = value };
        self.update_internals_unchecked();
    }

    #[inline(always)]
    unsafe fn push_unchecked(&mut self, values: Simd<T, B>) {
        write_values(self, values);
        self.update_internals_unchecked();
    }

    #[inline(always)]
    fn get_slice(&self, lane: usize) -> &[T] {
        &self.vals[lane]
    }

    #[inline(always)]
    fn push_with_info(&mut self, values: Simd<T, B>) -> (Simd<T, B>, Mask<i64, B>) {
        let replaced = write_values_pop(self, values);
        let mask = self.is_full();
        self.update_internals();
        (replaced, mask)
    }
    #[inline(always)]
    unsafe fn push_with_info_unchecked(&mut self, values: Simd<T, B>) -> Simd<T, B> {
        // Buffer is full, so perform a replacement.
        let replaced = write_values_pop(self, values);
        self.update_internals_unchecked();
        replaced
    }
    
    #[inline(always)]
    fn push_with_info_by_lane(&mut self, value: T, lane: usize) -> Option<T> {
        if self.count[lane] == self.capacity[lane] {
            let replaced =
                unsafe { <Self as RingBuffer<B, T>>::push_with_info_by_lane_unchecked(self, value, lane) };
            return Some(replaced)
        }
        unsafe { *self.vals[lane].get_unchecked_mut(self.index[lane]) = value };
        self.update_internals();
        None
    }

    #[inline(always)]
    unsafe fn push_with_info_by_lane_unchecked(&mut self, value: T, lane: usize) -> T {
        // Buffer is full, so perform a replacement.
        let replaced = *self.vals[lane].get_unchecked(self.index[lane]);
        *self.vals[lane].get_unchecked_mut(self.index[lane]) = value;
        self.update_internals_unchecked();
        replaced
    }

}