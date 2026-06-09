pub use crate::ring_buffer::buffer::{BufferElement, SerdeElement};

// ── Layout trait ─────────────────────────────────────────────────────────────

/// Defines the per-buffer type structure of a [`MultiTypeBuffer`].
///
/// Implemented for tuples `(T0,)` through `(T0, …, T7)` by the `impl_layout!`
/// macro. Each tuple element maps to one internal ring-buffer `Vec`.
///
/// [`MultiTypeBuffer`]: super::multi_type_buffer::MultiTypeBuffer
pub trait Layout: Sized + 'static {
    /// Tuple of values pushed/popped per operation, e.g. `(f64, Simd<f64, 2>)`.
    type Values: Copy;
    /// Tuple of `Vec`s that store the ring data, e.g. `(Vec<f64>, Vec<Simd<f64, 2>>)`.
    type Vecs: Clone;
    /// Tuple of contiguous slice references for mirror-buffer reads,
    /// e.g. `(&[f64], &[Simd<f64, 2>])`. The caller decides what to do with
    /// each slice; no SIMD interpretation is imposed here.
    type Slices<'a>
    where
        Self: 'a;

    fn make_vecs(capacity: usize) -> Self::Vecs;
    /// Allocates `capacity * 2` storage for mirror-buffer use.
    fn make_mirror_vecs(capacity: usize) -> Self::Vecs;

    /// Read all buffer values at raw index `idx`.
    fn read_values(vecs: &Self::Vecs, idx: usize) -> Self::Values;
    /// Write all buffer values at raw index `idx` (ring buffer).
    fn write_values(vecs: &mut Self::Vecs, idx: usize, values: Self::Values);
    /// Write to `idx`, returning the evicted value (ring buffer pop).
    fn write_values_pop(vecs: &mut Self::Vecs, idx: usize, values: Self::Values) -> Self::Values;
    /// Write to `idx` **and** `idx + capacity` (mirror buffer write).
    fn write_mirror(vecs: &mut Self::Vecs, idx: usize, capacity: usize, values: Self::Values);
    /// Write to `idx` and `idx + capacity`, returning the evicted value at `idx`.
    fn write_mirror_pop(
        vecs: &mut Self::Vecs,
        idx: usize,
        capacity: usize,
        values: Self::Values,
    ) -> Self::Values;
    /// Return `vecs[i][start..end]` for every internal vec `i`.
    fn get_slices(vecs: &Self::Vecs, start: usize, end: usize) -> Self::Slices<'_>;
    /// Return all elements in chronological order (oldest → newest).
    fn to_ordered_vecs(
        vecs: &Self::Vecs,
        index: usize,
        capacity: usize,
        count: usize,
    ) -> Self::Vecs;

    fn num_buffers() -> usize;
}
