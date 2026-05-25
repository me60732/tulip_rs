use crate::ring_buffer::multi_buffer::multi_buffer::{
    MirrorBuffer, MultiBuffer, RingBuffer,
};
use std::simd::Simd;

impl<const B: usize, const N: usize> MultiBuffer<B, Simd<f64, N>>{
    fn to_simd_buffer(f64_buffers: &[&[&[f64]; B]; N], capacity: usize, mirror: bool) -> Self {
        let actual_capacity = if mirror { capacity * 2 } else { capacity };

        let mut simd_vals: [Vec<Simd<f64, N>>; B] = core::array::from_fn(|_| {
            Vec::with_capacity(actual_capacity)
        });

        for i in 0..actual_capacity {
            // For each buffer lane (B lanes total)
            for b in 0..B {
                // Collect N f64 values from the same position and lane across all N buffers
                let simd_elements: [f64; N] =
                    core::array::from_fn(|n| f64_buffers[n][b].get(i).copied().unwrap_or(0.0));
                // Create SIMD value and push to the corresponding lane
                simd_vals[b].push(Simd::from_array(simd_elements));
            }
        }

        if mirror {
            for vec in simd_vals.iter_mut() {
                vec.extend_from_within(..);
            }
        }
        //let index = count % capacity;
        Self {
            vals: simd_vals,
            index: 0,
            prev_idx: capacity - 1, //index.wrapping_sub(1) % capacity, //capacity - 1,
            capacity,
            count: capacity,
        }
    }

    pub fn to_f64_buffers(&self) -> [MultiBuffer<B, f64>; N] {
        // Auto-detect if this is a mirror buffer
        let actual_length = self.vals[0].len();
        //let is_mirror = actual_length == self.capacity * 2;

        // Create storage for N MultiBuffers, each with B vectors
        let mut storage: [[Vec<f64>; B]; N] =
            core::array::from_fn(|_| core::array::from_fn(|_| Vec::with_capacity(actual_length)));

        // Extract SIMD data back to separate buffers
        for b in 0..B {
            // For each lane/vector in the MultiBuffer
            for simd_val in &self.vals[b] {
                let elements = simd_val.to_array(); // Extract [f64; N] from Simd<f64, N>

                // Distribute each element to its corresponding MultiBuffer
                for n in 0..N {
                    storage[n][b].push(elements[n]);
                }
            }
        }

        // Convert to actual MultiBuffer instances
        core::array::from_fn(|n| MultiBuffer {
            vals: storage[n].clone(),
            index: self.index,
            capacity: self.capacity, // Logical capacity for ring buffer operations
            count: self.count,
            prev_idx: self.prev_idx,
        })
    }
}

pub trait SimdRingBuffer<const B: usize, const N: usize>: RingBuffer<B, Simd<f64, N>>{
    fn from_f64_buffers(multi_buffers: [&MultiBuffer<B, f64>; N]) -> Self;
}
impl<const B: usize, const N: usize> SimdRingBuffer<B, N> for MultiBuffer<B, Simd<f64, N>>{
    fn from_f64_buffers(multi_buffers: [&MultiBuffer<B, f64>; N]) -> Self {
        let capacity = multi_buffers[0].get_capacity();

        // Get ordered vectors from each MultiBuffer
        let ordered_data: [[Vec<f64>; B]; N] =
            core::array::from_fn(|n| multi_buffers[n].to_ordered_vec());

        // Convert to slices for to_simd_buffer
        let slices: [[&[f64]; B]; N] =
            core::array::from_fn(|n| core::array::from_fn(|b| ordered_data[n][b].as_slice()));

        // Create references to the slice arrays
        let slice_refs: [&[&[f64]; B]; N] = core::array::from_fn(|n| &slices[n]);

        Self::to_simd_buffer(&slice_refs, capacity, false)
    }
}

pub trait SimdMirrorBuffer<const B: usize, const N: usize>: MirrorBuffer<B, Simd<f64, N>>{
    fn from_f64_buffers(multi_buffers: [&MultiBuffer<B, f64>; N]) -> Self;
}

impl<const B: usize, const N: usize> SimdMirrorBuffer<B, N> for MultiBuffer<B, Simd<f64, N>> {
    fn from_f64_buffers(multi_buffers: [&MultiBuffer<B, f64>; N]) -> Self {
        let capacity = multi_buffers[0].get_capacity();

        // Get slices from each mirror buffer using get_slices()
        let mirror_slices: [[&[f64]; B]; N] = core::array::from_fn(|n| {
            <MultiBuffer<B, f64> as MirrorBuffer<B, f64>>::get_slices(multi_buffers[n], 0)
        });

        // Create references to the slice arrays for to_simd_buffer
        let slice_refs: [&[&[f64]; B]; N] = core::array::from_fn(|n| &mirror_slices[n]);

        Self::to_simd_buffer(&slice_refs, capacity, true) // true = MirrorBuffer
    }
}
pub type SimdBuffer<const B: usize, const N: usize> = MultiBuffer<B, Simd<f64, N>>;
//pub type SimdBuffer<const N: usize> = Buffer<Simd<f64, N>>;
