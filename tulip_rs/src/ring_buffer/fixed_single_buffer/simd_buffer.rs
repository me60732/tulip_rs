//! Conversion helpers between `LANES` scalar fixed buffers and a single
//! SIMD fixed buffer that packs `LANES` assets per slot.
//!
//! Mirrors [`crate::ring_buffer::single_buffer::simd_buffer`] for the
//! fixed-capacity, stack-allocated buffer family.
//!
//! # Naming conventions
//! * `LANES` — SIMD lane count (number of parallel assets).
//! * `CAP`   — compile-time buffer capacity (number of historical slots).

use crate::ring_buffer::fixed_single_buffer::{FixedMirrorBuffer, FixedRingBuffer};
use std::simd::Simd;

// ── FixedRingBuffer<Simd<f64, LANES>, CAP> ────────────────────────────────────

impl<const LANES: usize, const CAP: usize> FixedRingBuffer<Simd<f64, LANES>, CAP> {
    /// Split this SIMD fixed ring buffer into `LANES` scalar fixed ring buffers.
    ///
    /// Each returned buffer carries the same ring state (`index`, `count`) and
    /// contains the values for one lane (one asset).
    pub fn to_f64_buffers(&self) -> [FixedRingBuffer<f64, CAP>; LANES] {
        std::array::from_fn(|lane| FixedRingBuffer {
            vals: std::array::from_fn(|i| self.vals[i].to_array()[lane]),
            index: self.index,
            count: self.count,
        })
    }
}

/// Conversion from `LANES` scalar [`FixedRingBuffer<f64, CAP>`] instances into a
/// single [`FixedRingBuffer<Simd<f64, LANES>, CAP>`].
pub trait FixedSimdRingBuffer<const LANES: usize, const CAP: usize> {
    /// Merge `LANES` scalar fixed ring buffers (one per asset) into a single
    /// SIMD fixed ring buffer.
    ///
    /// Slice length must equal `LANES`. The buffers may have different `index`
    /// positions (ring heads advance independently per asset); each buffer is
    /// reordered oldest-to-newest before packing, and the resulting SIMD buffer
    /// is normalised to `index = 0`.
    fn from_f64_buffers(buffers: &[&FixedRingBuffer<f64, CAP>]) -> Self;
}

impl<const LANES: usize, const CAP: usize> FixedSimdRingBuffer<LANES, CAP>
    for FixedRingBuffer<Simd<f64, LANES>, CAP>
{
    fn from_f64_buffers(buffers: &[&FixedRingBuffer<f64, CAP>]) -> Self {
        debug_assert_eq!(
            buffers.len(),
            LANES,
            "number of buffers must match SIMD width"
        );
        debug_assert!(
            buffers.windows(2).all(|w| w[0].count == w[1].count),
            "all scalar buffers must have the same count"
        );
        let ordered: [Vec<f64>; LANES] = std::array::from_fn(|j| buffers[j].to_ordered_vec());
        let count = buffers[0].count;
        Self {
            vals: std::array::from_fn(|i| {
                Simd::from_array(std::array::from_fn(|j| {
                    if i < ordered[j].len() {
                        ordered[j][i]
                    } else {
                        0.0
                    }
                }))
            }),
            index: 0,
            count,
        }
    }
}

// ── FixedMirrorBuffer<Simd<f64, LANES>, CAP> ──────────────────────────────────

impl<const LANES: usize, const CAP: usize> FixedMirrorBuffer<Simd<f64, LANES>, CAP> {
    /// Split this SIMD fixed mirror buffer into `LANES` scalar fixed mirror buffers.
    ///
    /// Each returned buffer carries the same state (`index`, `count`) and
    /// contains the values for one lane (one asset).
    pub fn to_f64_buffers(&self) -> [FixedMirrorBuffer<f64, CAP>; LANES] {
        std::array::from_fn(|lane| FixedMirrorBuffer {
            ring: std::array::from_fn(|i| self.ring[i].to_array()[lane]),
            view: std::array::from_fn(|i| self.view[i].to_array()[lane]),
            index: self.index,
            count: self.count,
        })
    }
}

/// Conversion from `LANES` scalar [`FixedMirrorBuffer<f64, CAP>`] instances into a
/// single [`FixedMirrorBuffer<Simd<f64, LANES>, CAP>`].
pub trait FixedSimdMirrorBuffer<const LANES: usize, const CAP: usize> {
    /// Merge `LANES` scalar fixed mirror buffers (one per asset) into a single
    /// SIMD fixed mirror buffer.
    ///
    /// Slice length must equal `LANES`. The buffers may have different `index`
    /// positions; `ring` is reordered oldest-to-newest using each buffer's head
    /// pointer, while `view` (already ordered) is copied directly. The resulting
    /// SIMD buffer is normalised to `index = 0`.
    fn from_f64_buffers(buffers: &[&FixedMirrorBuffer<f64, CAP>]) -> Self;
}

impl<const LANES: usize, const CAP: usize> FixedSimdMirrorBuffer<LANES, CAP>
    for FixedMirrorBuffer<Simd<f64, LANES>, CAP>
{
    fn from_f64_buffers(buffers: &[&FixedMirrorBuffer<f64, CAP>]) -> Self {
        debug_assert_eq!(
            buffers.len(),
            LANES,
            "number of buffers must match SIMD width"
        );
        debug_assert!(
            buffers.windows(2).all(|w| w[0].count == w[1].count),
            "all scalar buffers must have the same count"
        );
        // `view` is always kept oldest-to-newest, so call to_ordered_vec()
        // (which just slices view[..count]) for both ring and view.
        let ordered: [Vec<f64>; LANES] = std::array::from_fn(|j| buffers[j].to_ordered_vec());
        let count = buffers[0].count;
        Self {
            ring: std::array::from_fn(|i| {
                Simd::from_array(std::array::from_fn(|j| {
                    if i < ordered[j].len() {
                        ordered[j][i]
                    } else {
                        0.0
                    }
                }))
            }),
            view: std::array::from_fn(|i| {
                Simd::from_array(std::array::from_fn(|j| {
                    if i < ordered[j].len() {
                        ordered[j][i]
                    } else {
                        0.0
                    }
                }))
            }),
            index: 0,
            count,
        }
    }
}

// ── Type aliases ──────────────────────────────────────────────────────────────

/// A [`FixedRingBuffer`] whose elements are `LANES`-wide SIMD `f64` vectors.
/// Each slot packs one value per asset for `LANES` assets processed simultaneously.
pub type FixedSimdRingBuf<const LANES: usize, const CAP: usize> =
    FixedRingBuffer<Simd<f64, LANES>, CAP>;

/// A [`FixedMirrorBuffer`] whose elements are `LANES`-wide SIMD `f64` vectors.
pub type FixedSimdMirrorBuf<const LANES: usize, const CAP: usize> =
    FixedMirrorBuffer<Simd<f64, LANES>, CAP>;
