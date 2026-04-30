use crate::ring_buffer::
{
        buffer::period_to_idx,
        multi_buffer::multi_buffer::{
            get_by_periods, write_values, write_values_pop, BufferElement, MultiBuffer,
        },
};

pub trait RingBuffer<const B: usize, T: BufferElement = f64> {
    fn new(capacity: usize) -> Self;
    unsafe fn push_unchecked(&mut self, values: [T; B]);
    fn push(&mut self, values: [T; B]);
    fn push_with_info(&mut self, values: [T; B]) -> Option<[T; B]>;
    unsafe fn push_with_info_unchecked(&mut self, values: [T; B]) -> [T; B];
    fn get_slice(&self, lane: usize) -> &[T];
    fn to_ordered_vec(&self) -> [Vec<T>; B];
    fn from_slice(vals: [&[T]; B], capacity: usize) -> Self;
    unsafe fn push_with_info_periods_unchecked<const N: usize>(
        &mut self,
        values: [T; B],
        periods: [usize; N],
    ) -> [[T; N]; B];
    fn to_ordered_by_period(&self, period: usize) -> [Vec<T>; B];
}

impl<const B: usize, T: BufferElement> RingBuffer<B, T> for MultiBuffer<B, T> {
    fn new(capacity: usize) -> Self {
        Self {
            // Preallocate with default values
            vals: core::array::from_fn(|_| vec![T::default(); capacity]),
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
    fn get_slice(&self, lane: usize) -> &[T] {
        &self.vals[lane]
    }

    #[inline(always)]
    fn push_with_info(&mut self, values: [T; B]) -> Option<[T; B]> {   
        if self.count == self.capacity {
            let replaced = write_values_pop(self, values);
            self.update_internals_unchecked();
            return Some(replaced)
        }
        write_values(self, values);
        self.update_internals();
        None
    }
    #[inline(always)]
    unsafe fn push_with_info_unchecked(&mut self, values: [T; B]) -> [T; B] {
        // Buffer is full, so perform a replacement.
        let replaced = write_values_pop(self, values);
        self.update_internals_unchecked();
        replaced
    }
    #[inline(always)]
    unsafe fn push_with_info_periods_unchecked<const N: usize>(
        &mut self,
        values: [T; B],
        periods: [usize; N],
    ) -> [[T; N]; B] {
        let idxs: [usize; N] =
            std::array::from_fn(|i| period_to_idx(self.index, self.capacity, periods[i] - 1));
        let results = get_by_periods(self, idxs);
        self.push_unchecked(values);
        results
    }

    fn to_ordered_vec(&self) -> [Vec<T>; B] {
        if self.count == 0 {
            return core::array::from_fn(|_| Vec::new());
        }

        core::array::from_fn(|lane| {
            if self.count == self.capacity {
                // Buffer is full
                let mut result = Vec::with_capacity(self.capacity);
                // Add from index (oldest) to end of array
                result.extend_from_slice(&self.vals[lane][self.index..]);
                // Add from start of array to index-1
                if self.index > 0 {
                    result.extend_from_slice(&self.vals[lane][..self.index]);
                }
                return result
            }
             self.vals[lane][..self.count].to_vec()
        })
    }
    fn to_ordered_by_period(&self, period: usize) -> [Vec<T>; B] {
        if self.count == 0 || period == 0 {
            return core::array::from_fn(|_| Vec::new());
        }
        
        let take = period.min(self.count);
        // Use existing get_by_period which maps a bars-ago value into the underlying Vec index.
        // Oldest of the last `take` elements is `bars_ago = take - 1`, newest is `bars_ago = 0`.
        
        core::array::from_fn(|lane| {
            let mut out = Vec::with_capacity(take);
            for i in 0..take {
                // i==0 -> oldest -> bars_ago = take - 1
                let values = self.get_by_period(take - 1 - i);
                out.push(values[lane]);
            }
            out
        })
    }
}
