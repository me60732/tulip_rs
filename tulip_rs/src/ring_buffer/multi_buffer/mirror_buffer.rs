use crate::ring_buffer::multi_buffer::multi_buffer::{BufferElement, MultiBuffer};
use crate::ring_buffer::single_buffer::generic_buffer::Buffer as SingleBuffer;

use crate::indicators::simd_indicators::{
    max_simd::{SimdState as MaxState, find_max_scalar}, min_simd::{SimdState as MinState, find_min_scalar}, simd_types::UsizeConstants,
};
use crate::indicators::{
    max::{find_max_simd},
    min::{find_min_simd},
};
use std::simd::{
    cmp::{SimdPartialEq, SimdPartialOrd},
    Select, Simd
};

pub trait MirrorBuffer<const B: usize, T: BufferElement = f64> {
    fn new(capacity: usize) -> Self;
    unsafe fn push_unchecked(&mut self, values: [T; B]);
    fn push(&mut self, values: [T; B]);
    fn push_with_info(&mut self, values: [T; B]) -> Option<[T; B]>;
    unsafe fn push_with_info_unchecked(&mut self, values: [T; B]) -> [T; B];
    fn get_slice(&self, lane: usize, offset: usize) -> &[T];
    fn get_slices(&self, offset: usize) -> [&[T]; B];
    fn window_index_to_bars_ago(&self, idx: usize) -> usize;
    fn window_index_to_bars_ago_simd(&self, window_index: Simd<usize, B>) -> Simd<usize, B>;
    fn from_slice(vals: [&[T]; B], capacity: usize) -> Self;
    fn to_single_buffers(&self) -> [SingleBuffer<T>; B];
}
#[cfg(feature = "portable_simd")]

impl<const B: usize, T: BufferElement> MirrorBuffer<B, T> for MultiBuffer<B, T> {
    fn new(capacity: usize) -> Self {
        Self {
            // Preallocate with zeros.
            vals: core::array::from_fn(|_| vec![T::default(); capacity * 2]),
            index: 0,
            prev_idx: 0,
            capacity,
            count: 0,
        }
    }
    fn from_slice(vals: [&[T]; B], capacity: usize) -> Self {
        let count = vals[0].len().min(capacity);
        let buffer_vals: [Vec<T>; B] = core::array::from_fn(|lane| {
            let mut vec = vals[lane].to_vec();
            if count < capacity {
                vec.resize(capacity, T::default());
            }
            vec.extend_from_within(..);
            vec
        });
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
    fn push(&mut self, values: [T; B]) {
        write_values(self, values);
        self.update_internals();
    }
    #[inline(always)]
    unsafe fn push_unchecked(&mut self, values: [T; B]) {
        write_values(self, values);
        self.update_internals_unchecked();
    }
    #[inline(always)]
    fn push_with_info(&mut self, values: [T; B]) -> Option<[T; B]> {
        if self.count == self.capacity {
            let replaced = write_values_pop(self, values);
            return Some(replaced);
        }
        write_values(self, values);
        self.update_internals();
        None
    }
    #[inline(always)]
    unsafe fn push_with_info_unchecked(&mut self, values: [T; B]) -> [T; B] {
        // Buffer is full, so perform a replacement.
        write_values_pop(self, values)
    }
    #[inline(always)]
    fn get_slice(&self, lane: usize, offset: usize) -> &[T] {
        //let offset = offset.unwrap_or_default();
        debug_assert!(
            lane < B,
            "Lane index {} out of bounds for buffer with {} lanes",
            lane,
            B
        );

        if self.count == 0 {
            return &[];
        } else if self.count == self.capacity {
            // Buffer full - window is all data starting at oldest position, uses mirror for contiguity
            return unsafe { self.vals[lane].get_unchecked(self.index..self.index + self.count - offset) };
        }
        unsafe { self.vals[lane].get_unchecked(0..self.count - offset) }
    }
    #[inline(always)]
    fn get_slices(&self, offset: usize) -> [&[T]; B] {
        if self.count == 0 {
            return core::array::from_fn(|_| [].as_slice());
        } else if self.count == self.capacity {
            return core::array::from_fn(|lane| unsafe {
                self.vals[lane].get_unchecked(self.index..self.index + self.count - offset)
            });
        }
        core::array::from_fn(|lane| unsafe { self.vals[lane].get_unchecked(0..self.count - offset) })
    }

    #[inline(always)]
    fn window_index_to_bars_ago(&self, window_index: usize) -> usize {
        self.count - 1 - window_index
    }
    #[inline(always)]
    fn window_index_to_bars_ago_simd(&self, window_index: Simd<usize, B>) -> Simd<usize, B> {
        Simd::splat(self.count - 1) - window_index
    }
    fn to_single_buffers(&self) -> [SingleBuffer<T>; B] {
        std::array::from_fn(|i| 
            SingleBuffer {
                index: self.index,
                count: self.count,
                prev_idx: self.prev_idx,
                capacity: self.capacity,
                vals: self.vals[i].clone()
            }
        )
    }
}

#[inline(always)]
pub(crate) fn write_values<const B: usize, T: BufferElement>(
    buffer: &mut MultiBuffer<B, T>,
    values: [T; B],
) {
    for (buff, &vals) in buffer.vals.iter_mut().zip(values.iter()) {
        unsafe { *buff.get_unchecked_mut(buffer.index) = vals };
        unsafe { *buff.get_unchecked_mut(buffer.index + buffer.capacity) = vals };
    }
}
#[inline(always)]
pub(crate) fn write_values_pop<const N: usize, T: BufferElement>(
    buffer: &mut MultiBuffer<N, T>,
    values: [T; N],
) -> [T; N] {
    let mut results = [T::default(); N];
    for ((buff, &vals), result) in buffer
        .vals
        .iter_mut()
        .zip(values.iter())
        .zip(results.iter_mut())
    {
        *result = unsafe { *buff.get_unchecked(buffer.index) };
        unsafe { *buff.get_unchecked_mut(buffer.index) = vals };
        unsafe { *buff.get_unchecked_mut(buffer.index + buffer.capacity) = vals };
    }
    results
}

pub trait MinMaxBuffer<const B: usize>: MirrorBuffer<B, f64> {
    fn max<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MaxState<B>,
        bar: Simd<f64, B>,
        look_back: usize,
    ) -> (Simd<f64, B>, Simd<usize, B>);
    fn min<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MinState<B>,
        bar: Simd<f64, B>,
        look_back: usize,
    ) -> (Simd<f64, B>, Simd<usize, B>);
}
impl<const B: usize> MinMaxBuffer<B> for MultiBuffer<B, f64> {
    fn max<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MaxState<B>,
        bar: Simd<f64, B>,
        look_back: usize,
    ) -> (Simd<f64, B>, Simd<usize, B>) {
        let (mut max, mut trail) = (state.max, state.trail);
        trail += UsizeConstants::ONE;
        
        let lookback_simd = Simd::splat(look_back);
        let needs_search = lookback_simd.simd_eq(trail);
        let search_mask = needs_search.to_bitmask();
        //trail = needs_search.select(trail, trail + UsizeConstants::ONE);
        
        let current_is_new_max = bar.simd_ge(max);

        max = current_is_new_max.select(bar, max);
        trail = current_is_new_max.select(UsizeConstants::ZERO, trail);
        
        if search_mask != 0 {
            let max_array = max.as_mut_array();
            let trail_array = trail.as_mut_array();
            //let current = bar.as_array();
            // Const loop - compiler will unroll this automatically
            let mut lane = 0;
            
            while lane < B {
                if search_mask & (1 << lane) != 0 {
                    let (max_val, max_idx) = if CHUNK_SIZE == 1 {
                        find_max_scalar(self.get_slice(lane, 1), bar[lane])
                    } else {
                        find_max_simd::<CHUNK_SIZE>(self.get_slice(lane, 0))
                    };
                    max_array[lane] = max_val;
                    trail_array[lane] = self.window_index_to_bars_ago(max_idx);
                }
                lane += 1;
            }
        }
        (state.max, state.trail) = (max, trail);
        (max, trail)
    }
    fn min<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MinState<B>,
        bar: Simd<f64, B>,
        look_back: usize,
    ) -> (Simd<f64, B>, Simd<usize, B>) {
        let (mut min, mut trail) = (state.min, state.trail);
        trail += UsizeConstants::ONE;
        
        let lookback_simd = Simd::splat(look_back);
        let needs_search = lookback_simd.simd_eq(trail);
        let search_mask = needs_search.to_bitmask();
        //trail = needs_search.select(trail, trail + UsizeConstants::ONE);
        
        let current_is_new_min = bar.simd_le(min);

        min = current_is_new_min.select(bar, min);
        trail = current_is_new_min.select(UsizeConstants::ZERO, trail);

        if search_mask != 0 {
            let min_array = min.as_mut_array();
            let trail_array = trail.as_mut_array();
            //let current = bar.as_array();
            // Const loop - compiler will unroll this automatically
            let mut lane = 0;
            while lane < B {
                if search_mask & (1 << lane) != 0 {
                    let (min_val, min_idx) = if CHUNK_SIZE == 1 {
                        find_min_scalar(self.get_slice(lane, 1), bar[lane])
                    } else {
                        find_min_simd::<CHUNK_SIZE>(self.get_slice(lane, 0))
                    };
                    min_array[lane] = min_val;
                    trail_array[lane] = self.window_index_to_bars_ago(min_idx);
                }
                lane += 1;
            }
        }
        (state.min, state.trail) = (min, trail);
        (min, trail)
    }
    
}
