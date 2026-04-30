use crate::ring_buffer::{
    buffer::period_to_idx,
    single_buffer::generic_buffer::{get_by_periods, Buffer, BufferElement},
};

pub trait RingBuffer<T: BufferElement = f64> {
    fn new(capacity: usize) -> Self;
    unsafe fn push_unchecked(&mut self, value: T);
    fn push(&mut self, value: T);
    fn push_with_info(&mut self, value: T) -> Option<T>;
    unsafe fn push_with_info_unchecked(&mut self, value: T) -> T;
    unsafe fn push_with_info_periods_unchecked<const N: usize>(
        &mut self,
        value: T,
        periods: [usize; N],
    ) -> [T; N];
    fn get_slice(&self) -> &[T];
    fn to_ordered_vec(&self) -> Vec<T>;
    fn to_ordered_by_period(&self, period: usize) -> Vec<T>;
}

impl<T: BufferElement> RingBuffer<T> for Buffer<T> {
    fn new(capacity: usize) -> Self {
        Self {
            // Preallocate with default values
            vals: vec![T::default(); capacity],
            index: 0,
            prev_idx: 0,
            capacity,
            count: 0,
        }
    }

    #[inline(always)]
    fn push(&mut self, value: T) {
        unsafe {
            *self.vals.get_unchecked_mut(self.index) = value;
        }
        self.update_internals();
    }

    #[inline(always)]
    unsafe fn push_unchecked(&mut self, value: T) {
        *self.vals.get_unchecked_mut(self.index) = value;
        self.update_internals_unchecked();
    }

    #[inline(always)]
    fn get_slice(&self) -> &[T] {
        &self.vals
    }

    #[inline(always)]
    fn push_with_info(&mut self, value: T) -> Option<T> {
        if self.count == self.capacity {
            let replaced =
                unsafe { <Self as RingBuffer<T>>::push_with_info_unchecked(self, value) };
            return Some(replaced);
        }
        unsafe { *self.vals.get_unchecked_mut(self.index) = value };
        self.update_internals();
        None
    }

    #[inline(always)]
    unsafe fn push_with_info_unchecked(&mut self, value: T) -> T {
        // Buffer is full, so perform a replacement.
        let replaced = *self.vals.get_unchecked(self.index);
        *self.vals.get_unchecked_mut(self.index) = value;
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
    fn to_ordered_vec(&self) -> Vec<T> {
        if self.count == 0 {
            return Vec::new();
        }

        if self.count == self.capacity {
            // Buffer is full
            // oldest value is at index (about to be overwritten)
            let mut result = Vec::with_capacity(self.capacity);

            // Add from index (oldest) to end of array
            result.extend_from_slice(&self.vals[self.index..]);
            // Add from start of array to index-1
            if self.index > 0 {
                result.extend_from_slice(&self.vals[..self.index]);
            }
            return result;
        }
        self.vals[..self.count].to_vec()
    }
    fn to_ordered_by_period(&self, period: usize) -> Vec<T> {
        if self.count == 0 || period == 0 {
            return Vec::new();
        }
        
        let take = period.min(self.count);
        // Use existing get_by_period which maps a bars-ago value into the underlying Vec index.
        // Oldest of the last `take` elements is `bars_ago = take - 1`, newest is `bars_ago = 0`.
        let mut out = Vec::with_capacity(take);
        for i in 0..take {
            // i==0 -> oldest -> bars_ago = take - 1
            out.push(self.get_by_period(take - 1 - i));
        }
        out
    }
}
