use crate::ring_buffer::single_buffer::generic_buffer::{
    Buffer, F64Buffer, MirrorBuffer, RingBuffer,
};
use std::simd::Simd;

impl<const N: usize> Buffer<Simd<f64, N>> {
    fn to_simd_buffer(f64_buffers: &[&[f64]], capacity: usize, mirror: bool) -> Self {
        let count = f64_buffers[0].len().min(capacity);
        //let capacity = f64_buffers[0].len();

        let mut simd_vals =
            Vec::<Simd<f64, N>>::with_capacity(if mirror { capacity * 2 } else { capacity });

        for i in 0..capacity {
            let mut vals = [0.0; N];
            for j in 0..N {
                vals[j] = unsafe { *f64_buffers.get_unchecked(j).get_unchecked(i) };
            }
            simd_vals.push(Simd::from_array(vals));
        }
        if mirror {
            simd_vals.extend_from_within(..);
        }
        //let index = count % capacity;
        Self {
            vals: simd_vals,
            index: 0,
            prev_idx: capacity - 1, //index.wrapping_sub(1) % capacity, //capacity - 1,
            capacity,
            count: count,
        }
    }
    pub fn to_f64_buffers(&self) -> Vec<F64Buffer> {
        let mut vals = Vec::with_capacity(N);

        for _ in 0..N {
            vals.push(Vec::with_capacity(self.capacity));
        }

        for simd_val in self.vals.iter() {
            let val = simd_val.to_array();
            for j in 0..N {
                vals[j].push(val[j]);
            }
        }
        let mut buffers = Vec::with_capacity(N);
        for val in vals.into_iter() {
            buffers.push(F64Buffer {
                vals: val,
                prev_idx: self.prev_idx,
                capacity: self.capacity,
                count: self.count,
                index: self.index,
            });
        }
        buffers
    }
}

pub trait SimdRingBuffer<const N: usize>: RingBuffer<Simd<f64, N>> {
    fn from_f64_buffers(f64_buffers: Vec<&Buffer<f64>>) -> Self;
}
#[cfg(feature = "portable_simd")]
impl<const N: usize> SimdRingBuffer<N> for Buffer<Simd<f64, N>> {
    fn from_f64_buffers(buffers: Vec<&Buffer<f64>>) -> Self {
        debug_assert_eq!(buffers.len(), N, "Number of buffers must match SIMD width");

        let capacity = buffers[0].get_capacity();

        // Get ordered vectors from each buffer (owned data)
        let ordered_vecs: Vec<Vec<f64>> = buffers.iter().map(|buf| buf.to_ordered_vec()).collect();
        /*for buffer in buffers {
            println!("\nCount: {:?}, Capacity: {:?}", buffer.count, buffer.capacity);
        }*/
        // Now we can safely create slices from the owned vectors
        let slices: Vec<&[f64]> = ordered_vecs.iter().map(|vec| vec.as_slice()).collect();

        Self::to_simd_buffer(&slices, capacity, false)
    }
}

pub trait SimdMirrorBuffer<const N: usize>: MirrorBuffer<Simd<f64, N>> {
    fn from_f64_buffers(f64_slices: Vec<&Buffer<f64>>) -> Self;
}
#[cfg(feature = "portable_simd")]
impl<const N: usize> SimdMirrorBuffer<N> for Buffer<Simd<f64, N>> {
    fn from_f64_buffers(buffers: Vec<&Buffer<f64>>) -> Self {
        debug_assert_eq!(buffers.len(), N, "Number of buffers must match SIMD width");

        let capacity = buffers[0].get_capacity();

        // Get slices from each mirror buffer
        let slices: Vec<&[f64]> = buffers
            .iter()
            .map(|buf| <Buffer<f64> as MirrorBuffer<f64>>::get_slice(buf))
            .collect();

        Self::to_simd_buffer(&slices, capacity, true) // true = MirrorBuffer
    }
}
pub trait FlatSimdBuffer<const N: usize> {
    fn from_f64_buffers(buffers: Vec<&Buffer<f64>>) -> &Self;
    fn to_f64_buffers(&self, periods: [usize; N]) -> Vec<F64Buffer>;
}
impl<const N: usize> FlatSimdBuffer<N> for Buffer {
    fn from_f64_buffers(buffers: Vec<&Buffer<f64>>) -> &Self {
        let mut i = 0;
        for (j, buffer) in buffers.iter().skip(1).enumerate() {
            if buffer.capacity > buffers[i].capacity {
                i = j;
            }
        }

        buffers[i]
    }
    fn to_f64_buffers(&self, periods: [usize; N]) -> Vec<F64Buffer> {
        let mut buffers = Vec::with_capacity(N);
        for period in periods {
            if period != self.capacity {
                let vals = self.to_ordered_by_period(period);
                buffers.push(Buffer {
                    index: 0,
                    prev_idx: vals.len() - 1,
                    count: vals.len() - 1,
                    capacity: vals.len(),
                    vals,
                });
            }
        }
        buffers
    }
}
pub type SimdBuffer<const N: usize> = Buffer<Simd<f64, N>>;
