use serde::{Deserialize, Serialize};
use std::simd::Simd;
// ── SerdeElement ──────────────────────────────────────────────────────────────

/// Serde-compatible round-trip representation for a [`BufferElement`].
///
/// `Simd<f64, N>` cannot implement serde directly (orphan rules), so it maps to
/// `[f64; N]` which does. Scalar types use themselves as `Repr` — zero cost.
///
/// This trait is separate from [`BufferElement`]: not every buffer element needs
/// to be serialisable (e.g. `Buffer<Simd<f64, N>>` used transiently in SIMD
/// indicator hot-paths). Only add this bound when you actually need to
/// serialise/deserialise the buffer.
pub trait SerdeElement: Copy + Default + Send + Sync + 'static {
    /// A `Copy + Serialize + Deserialize` stand-in for `Self`.
    type Repr: Serialize + for<'de> Deserialize<'de> + Copy + Default;
    fn to_repr(self) -> Self::Repr;
    fn from_repr(repr: Self::Repr) -> Self;
}

impl SerdeElement for f64 {
    type Repr = f64;
    #[inline(always)]
    fn to_repr(self) -> f64 {
        self
    }
    #[inline(always)]
    fn from_repr(r: f64) -> f64 {
        r
    }
}
impl SerdeElement for f32 {
    type Repr = f32;
    #[inline(always)]
    fn to_repr(self) -> f32 {
        self
    }
    #[inline(always)]
    fn from_repr(r: f32) -> f32 {
        r
    }
}
impl SerdeElement for usize {
    type Repr = usize;
    #[inline(always)]
    fn to_repr(self) -> usize {
        self
    }
    #[inline(always)]
    fn from_repr(r: usize) -> usize {
        r
    }
}
impl SerdeElement for i64 {
    type Repr = i64;
    #[inline(always)]
    fn to_repr(self) -> i64 {
        self
    }
    #[inline(always)]
    fn from_repr(r: i64) -> i64 {
        r
    }
}
impl SerdeElement for u64 {
    type Repr = u64;
    #[inline(always)]
    fn to_repr(self) -> u64 {
        self
    }
    #[inline(always)]
    fn from_repr(r: u64) -> u64 {
        r
    }
}
impl<const N: usize> SerdeElement for Simd<f64, N>
where
    Simd<f64, N>: Copy + Default + Send + Sync + 'static,
    [f64; N]: Serialize + for<'de> Deserialize<'de> + Copy + Default,
{
    type Repr = [f64; N];
    #[inline(always)]
    fn to_repr(self) -> [f64; N] {
        self.to_array()
    }
    #[inline(always)]
    fn from_repr(r: [f64; N]) -> Self {
        Simd::from_array(r)
    }
}

// ── BufferElement ─────────────────────────────────────────────────────────────

/// Minimal trait for types that can be used as buffer elements.
///
/// Intentionally does **not** require [`SerdeElement`] as a supertrait so that
/// `Buffer<Simd<f64, N>>` used transiently in SIMD indicator hot-paths does not
/// accumulate serde bounds for every generic `N`. Add `+ SerdeElement` at the
/// call-site when you need to serialise/deserialise the buffer.
pub trait BufferElement: Copy + Default + Send + Sync + 'static {}

impl BufferElement for f64 {}
impl BufferElement for f32 {}
impl BufferElement for usize {}
impl BufferElement for i64 {}
impl BufferElement for u64 {}
impl<const N: usize> BufferElement for Simd<f64, N> {}

#[inline(always)]
pub(crate) fn period_to_idx(index: usize, capacity: usize, period: usize) -> usize {
    let mut idx = index as i32 - period as i32 - 1;
    idx = if idx < 0 { idx + capacity as i32 } else { idx };
    idx as usize
}
