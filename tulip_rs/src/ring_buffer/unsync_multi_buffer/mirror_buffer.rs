use crate::indicators::simd_indicators::{
    max_simd::{find_max_scalar, SimdState as MaxState},
    min_simd::{find_min_scalar, SimdState as MinState},
    simd_types::UsizeConstants,
};
use crate::indicators::{max::find_max_simd, min::find_min_simd};
use crate::ring_buffer::unsync_multi_buffer::multi_buffer::{BufferElement, UnsyncBuffer};
//use std::simd::{Mask, Simd, SimdElement};
use std::simd::{
    cmp::{SimdPartialEq, SimdPartialOrd},
    Mask, Select, Simd, SimdElement,
};

pub trait MirrorBuffer<const B: usize, T: SimdElement + BufferElement = f64> {
    fn new(capacity: [usize; B]) -> Self;
    unsafe fn push_unchecked(&mut self, values: Simd<T, B>);
    fn push(&mut self, values: Simd<T, B>);
    fn push_with_info(&mut self, values: Simd<T, B>) -> (Simd<T, B>, Mask<i64, B>);
    unsafe fn push_with_info_unchecked(&mut self, values: Simd<T, B>) -> Simd<T, B>;
    fn get_slice(&self, lane: usize, offset: usize) -> &[T];
    fn get_slices(&self, offset: usize) -> [&[T]; B];
    fn window_index_to_bars_ago(&self, window_index: usize, lane: usize) -> usize;
    fn from_slice(vals: [&[T]; B], capacity: [usize; B]) -> Self;
}
#[cfg(feature = "portable_simd")]

impl<const B: usize, T: BufferElement + SimdElement> MirrorBuffer<B, T> for UnsyncBuffer<B, T> {
    fn new(capacity: [usize; B]) -> Self {
        let vals = core::array::from_fn(|i| vec![T::default(); capacity[i] * 2]);

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
            vec.extend_from_within(..);
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
    unsafe fn push_unchecked(&mut self, values: Simd<T, B>) {
        write_values(self, values);
        self.update_internals_unchecked();
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
    fn get_slice(&self, lane: usize, offset: usize) -> &[T] {
        debug_assert!(
            lane < B,
            "Lane index {} out of bounds for buffer with {} lanes",
            lane,
            B
        );

        if self.count[lane] == 0 {
            return &[];
        } else if self.count[lane] == self.capacity[lane] {
            return unsafe {
                self.vals[lane]
                    .get_unchecked(self.index[lane]..self.index[lane] + self.count[lane] - offset)
            };
        }
        unsafe { self.vals[lane].get_unchecked(0..self.count[lane] - offset) }
    }
    #[inline(always)]
    fn get_slices(&self, offset: usize) -> [&[T]; B] {
        std::array::from_fn(|lane| self.get_slice(lane, offset))
    }

    #[inline(always)]
    fn window_index_to_bars_ago(&self, window_index: usize, lane: usize) -> usize {
        self.count[lane] - 1 - window_index
    }
}

#[inline(always)]
pub(crate) fn write_values<const B: usize, T: BufferElement + SimdElement>(
    buffer: &mut UnsyncBuffer<B, T>,
    values: Simd<T, B>,
) {
    let idx = buffer.index.as_array(); //.to_array();
    let capacity = buffer.capacity.to_array();
    for (((buff, &vals), &idx), &capacity) in buffer
        .vals
        .iter_mut()
        .zip(values.as_array().iter())
        .zip(idx.iter())
        .zip(capacity.iter())
    {
        unsafe { *buff.get_unchecked_mut(idx) = vals }
        unsafe { *buff.get_unchecked_mut(idx + capacity) = vals };
    }
}
#[inline(always)]
pub(crate) fn write_values_pop<const B: usize, T: BufferElement + SimdElement>(
    buffer: &mut UnsyncBuffer<B, T>,
    values: Simd<T, B>,
) -> Simd<T, B> {
    let idx = buffer.index.as_array(); //.to_array();
    let capacity = buffer.capacity.to_array();
    let mut results = Simd::splat(T::default());
    for ((((buff, &vals), result), &idx), &capacity) in buffer
        .vals
        .iter_mut()
        .zip(values.as_array().iter())
        .zip(results.as_mut_array().iter_mut())
        .zip(idx.iter())
        .zip(capacity.iter())
    {
        *result = unsafe { *buff.get_unchecked(idx) };
        unsafe { *buff.get_unchecked_mut(idx) = vals }
        unsafe { *buff.get_unchecked_mut(idx + capacity) = vals };
    }
    results
}

pub trait MinMaxBuffer<const B: usize>: MirrorBuffer<B, f64> {
    fn max(
        &self,
        state: &mut MaxState<B>,
        bar: Simd<f64, B>,
        look_back: Simd<usize, B>,
    ) -> (Simd<f64, B>, Simd<usize, B>);
    fn min(
        &self,
        state: &mut MinState<B>,
        bar: Simd<f64, B>,
        look_back: Simd<usize, B>,
    ) -> (Simd<f64, B>, Simd<usize, B>);
}
impl<const B: usize> MinMaxBuffer<B> for UnsyncBuffer<B, f64> {
    fn max(
        &self,
        state: &mut MaxState<B>,
        bar: Simd<f64, B>,
        look_back: Simd<usize, B>,
    ) -> (Simd<f64, B>, Simd<usize, B>) {
        let (mut max, mut trail) = (state.max, state.trail);
        trail += UsizeConstants::ONE;

        let needs_search = look_back.simd_eq(trail);
        let search_mask = needs_search.to_bitmask();
        //trail = needs_search.select(trail, trail + UsizeConstants::ONE);

        let current_is_new_max = bar.simd_ge(max);

        max = current_is_new_max.select(bar, max);
        trail = current_is_new_max.select(UsizeConstants::ZERO, trail);

        if search_mask != 0 {
            let max_array = max.as_mut_array();
            let trail_array = trail.as_mut_array();
            let look_back_array = look_back.as_array();
            //let current = bar.as_array();
            // Const loop - compiler will unroll this automatically
            let mut lane = 0;

            while lane < B {
                if search_mask & (1 << lane) != 0 {
                    let (max_val, max_idx) = if look_back_array[lane] < 14 {
                        find_max_scalar(self.get_slice(lane, 1), bar[lane])
                    } else {
                        find_max_simd::<8>(self.get_slice(lane, 0))
                    };
                    max_array[lane] = max_val;
                    trail_array[lane] = self.window_index_to_bars_ago(max_idx, lane);
                }
                lane += 1;
            }
        }
        (state.max, state.trail) = (max, trail);
        (max, trail)
    }
    fn min(
        &self,
        state: &mut MinState<B>,
        bar: Simd<f64, B>,
        look_back: Simd<usize, B>,
    ) -> (Simd<f64, B>, Simd<usize, B>) {
        let (mut min, mut trail) = (state.min, state.trail);
        trail += UsizeConstants::ONE;

        let needs_search = look_back.simd_eq(trail);
        let search_mask = needs_search.to_bitmask();
        //trail = needs_search.select(trail, trail + UsizeConstants::ONE);

        let current_is_new_min = bar.simd_le(min);

        min = current_is_new_min.select(bar, min);
        trail = current_is_new_min.select(UsizeConstants::ZERO, trail);

        if search_mask != 0 {
            let min_array = min.as_mut_array();
            let trail_array = trail.as_mut_array();
            let look_back_array = look_back.as_array();
            // Const loop - compiler will unroll this automatically
            let mut lane = 0;
            while lane < B {
                if search_mask & (1 << lane) != 0 {
                    let (min_val, min_idx) = if look_back_array[lane] < 14 {
                        find_min_scalar(self.get_slice(lane, 1), bar[lane])
                    } else {
                        find_min_simd::<8>(self.get_slice(lane, 0))
                    };
                    min_array[lane] = min_val;
                    trail_array[lane] = self.window_index_to_bars_ago(min_idx, lane);
                }
                lane += 1;
            }
        }
        (state.min, state.trail) = (min, trail);
        (min, trail)
    }
}
