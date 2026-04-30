use std::simd::Simd;
/// Minimal trait for types that can be used as buffer elements
pub trait BufferElement: Copy + Default + Send + Sync + 'static {}

impl BufferElement for f64 {}
impl<const N: usize> BufferElement for Simd<f64, N> {}
#[inline(always)]
pub(crate) fn period_to_idx(index: usize, capacity: usize, period: usize) -> usize {
    let mut idx = index as i32 - period as i32 - 1;
    idx = if idx < 0 {
        idx + capacity as i32
    } else {
        idx
    };
    idx as usize
}
