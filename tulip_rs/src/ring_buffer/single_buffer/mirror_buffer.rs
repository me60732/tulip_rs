use crate::indicators::{
    max::{find_max_scalar, find_max_simd, State as MaxState},
    min::{find_min_scalar, find_min_simd, State as MinState},
};
use crate::ring_buffer::{
    buffer::period_to_idx,
    single_buffer::generic_buffer::{get_by_periods, Buffer, BufferElement},
};
pub trait MirrorBuffer<T: BufferElement = f64> {
    fn new(capacity: usize) -> Self;
    unsafe fn push_unchecked(&mut self, value: T);
    fn push(&mut self, value: T);
    fn push_with_info(&mut self, value: T) -> Option<T>;
    unsafe fn push_with_info_unchecked(&mut self, value: T) -> T;
    fn get_slice(&self) -> &[T];
    fn get_slice_mut(&mut self) -> &mut [T]; // Add mutable version
    fn window_index_to_bars_ago(&self, idx: usize) -> usize;
    fn get_slice_by_period(&self, period: usize) -> &[T];
    fn from_non_mirror(buffer: &Buffer<T>) -> Self;
    unsafe fn push_with_info_periods_unchecked<const N: usize>(
        &mut self,
        value: T,
        periods: [usize; N],
    ) -> [T; N];
    fn to_non_mirrors_by_periods<const N: usize>(&self, periods: [usize; N]) -> [Buffer<T>; N];
    fn sync_mirrors(&mut self);
}
#[cfg(feature = "portable_simd")]

impl<T: BufferElement> MirrorBuffer<T> for Buffer<T> {
    fn new(capacity: usize) -> Self {
        Self {
            // Preallocate with zeros.
            vals: vec![T::default(); capacity * 2], //crate::uninit_vec!(f64, capacity),
            index: 0,
            prev_idx: 0,
            capacity,
            count: 0,
        }
    }
    fn from_non_mirror(buffer: &Buffer<T>) -> Self {
        let mut vals = buffer.vals.clone();
        vals.extend_from_within(..);
        Self {
            vals,
            index: buffer.index,
            prev_idx: buffer.prev_idx,
            capacity: buffer.capacity,
            count: buffer.count,
        }
    }
    fn to_non_mirrors_by_periods<const N: usize>(&self, periods: [usize; N]) -> [Buffer<T>; N] {
        std::array::from_fn(|i| Buffer {
            vals: if self.capacity == periods[i] {
                self.get_slice().to_vec()
            } else {
                self.get_slice_by_period(periods[i]).to_vec()
            },
            index: 0,
            prev_idx: periods[i] - 1,
            capacity: periods[i],
            count: periods[i],
        })
    }
    #[inline(always)]
    fn push(&mut self, value: T) {
        //print!("mirror push");
        unsafe {
            *self.vals.get_unchecked_mut(self.index) = value;
            *self.vals.get_unchecked_mut(self.index + self.capacity) = value;
        }
        self.update_internals();
    }
    #[inline(always)]
    unsafe fn push_unchecked(&mut self, value: T) {
        //print!("\nmirror push_unchecked: {:?}", self.vals);
        *self.vals.get_unchecked_mut(self.index) = value;
        *self.vals.get_unchecked_mut(self.index + self.capacity) = value;

        self.update_internals_unchecked();
    }
    #[inline(always)]
    fn push_with_info(&mut self, value: T) -> Option<T> {
        if self.count == self.capacity {
            let replaced =
                unsafe { <Self as MirrorBuffer<T>>::push_with_info_unchecked(self, value) };
            return Some(replaced);
        }
        unsafe { *self.vals.get_unchecked_mut(self.index) = value };
        unsafe { *self.vals.get_unchecked_mut(self.index + self.capacity) = value };
        self.update_internals();
        None
    }
    #[inline(always)]
    unsafe fn push_with_info_unchecked(&mut self, value: T) -> T {
        // Buffer is full, so perform a replacement.
        let replaced = *self.vals.get_unchecked(self.index);
        *self.vals.get_unchecked_mut(self.index) = value;
        *self.vals.get_unchecked_mut(self.index + self.capacity) = value;
        self.update_internals_unchecked();
        replaced
    }
    #[inline(always)]
    unsafe fn push_with_info_periods_unchecked<const N: usize>(
        &mut self,
        value: T,
        periods: [usize; N],
    ) -> [T; N] {
        let idxs: [usize; N] =
            std::array::from_fn(|i| period_to_idx(self.index, self.capacity, periods[i] - 1));
        let results = get_by_periods(self, idxs);
        self.push_unchecked(value);
        results
    }
    #[inline(always)]
    fn get_slice(&self) -> &[T] {
        if self.count == 0 {
            return &[];
        }

        if self.count == self.capacity {
            // Buffer full - window is all data starting at oldest position, uses mirror for contiguity
            //&self.vals[self.index..self.index + self.count]
            return unsafe { self.vals.get_unchecked(self.index..self.index + self.count) };
        }
        unsafe { self.vals.get_unchecked(0..self.count) }
    }
    #[inline(always)]
    fn get_slice_mut(&mut self) -> &mut [T] {
        if self.count == 0 {
            return &mut [];
        }

        if self.count == self.capacity {
            // Buffer full - window is all data starting at oldest position, uses mirror for contiguity
            return unsafe {
                self.vals
                    .get_unchecked_mut(self.index..self.index + self.count)
            };
        }
        unsafe { self.vals.get_unchecked_mut(0..self.count) }
    }
    fn sync_mirrors(&mut self) {
        if self.count != self.capacity {
            return;
        }
        for i in 0..self.capacity {
            let canonical = self.index + i;
            let other = if canonical < self.capacity {
                canonical + self.capacity
            } else {
                canonical - self.capacity
            };
            unsafe {
                *self.vals.get_unchecked_mut(other) = *self.vals.get_unchecked(canonical);
            }
        }
    }
    #[inline(always)]
    fn get_slice_by_period(&self, period: usize) -> &[T] {
        if self.count == 0 || period == 0 {
            return &[];
        }
        let take = period.min(self.count);

        // Determine the logical window start (oldest element index in underlying storage)
        let window_start = if self.count == self.capacity {
            self.index
        } else {
            0
        };
        let window_len = self.count;

        // Start of the last `take` elements (oldest -> newest)
        let start = window_start + (window_len - take);

        // For mirror buffer the live window is contiguous when full (index .. index+count),
        // and when not full data is in 0..count, so start..start+take will be contiguous.
        unsafe { self.vals.get_unchecked(start..start + take) }
    }
    #[inline(always)]
    fn window_index_to_bars_ago(&self, window_index: usize) -> usize {
        self.count - 1 - window_index
    }
}

pub trait MinMaxBuffer: MirrorBuffer<f64> {
    fn max<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MaxState,
        bar: f64,
        period: usize,
    ) -> (f64, usize);
    fn min<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MinState,
        bar: f64,
        period: usize,
    ) -> (f64, usize);
}
impl MinMaxBuffer for Buffer<f64> {
    fn max<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MaxState,
        bar: f64,
        period: usize,
    ) -> (f64, usize) {
        let (mut max, mut trail) = (state.max, state.trail);
        trail += 1;
        if period <= trail {
            (max, trail) = if CHUNK_SIZE == 1 {
                find_max_scalar(self.get_slice())
            } else {
                find_max_simd::<CHUNK_SIZE>(self.get_slice())
            };
            trail = self.window_index_to_bars_ago(trail);
        } else if bar >= max {
            max = bar;
            trail = 0;
        }
        (state.max, state.trail) = (max, trail);
        (max, trail)
    }
    fn min<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MinState,
        bar: f64,
        period: usize,
    ) -> (f64, usize) {
        let (mut min, mut trail) = (state.min, state.trail);
        trail += 1;
        if period <= trail {
            (min, trail) = if CHUNK_SIZE == 1 {
                find_min_scalar(self.get_slice())
            } else {
                find_min_simd::<CHUNK_SIZE>(self.get_slice())
            };
            trail = self.window_index_to_bars_ago(trail);
        } else if bar <= min {
            min = bar;
            trail = 0;
        }
        (state.min, state.trail) = (min, trail);
        (min, trail)
    }
}
