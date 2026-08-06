//! Multi-lane mirror buffer.
//!
//! `MultiMirrorBuffer<B, T, S>` has identical fields to `MultiBuffer<B, T, S>`
//! but each lane's Vec is allocated with `2 * capacity` slots, maintaining a
//! contiguous ordered window for min/max SIMD scans.

use crate::indicators::simd_indicators::{
    max_simd::{find_max_scalar, assets::SimdState as MaxState},
    min_simd::{find_min_scalar, assets::SimdState as MinState},
    simd_types::UsizeConstants,
};
use crate::indicators::{
    max::{find_max_scalar as find_max_scalar_1, find_max_simd},
    min::{find_min_scalar as find_min_scalar_1, find_min_simd},
};
use crate::ring_buffer::{
    buffer::{period_to_idx, SerdeElement},
    multi_buffer::multi_buffer::{
        mb_advance, mb_advance_unchecked, mb_get_values, mb_write_values_mirror,
        mb_write_values_mirror_pop, BufferElement,
    },
    single_buffer::{
        generic_buffer::{Cold, Warm},
        // MirrorBuffer<S, T> is the concrete single-lane mirror-buffer struct
        // (already refactored from the old MirrorBuffer trait).
        mirror_buffer::MirrorBuffer as SingleMirrorBuffer,
    },
};
use serde::{Deserialize, Serialize};
use std::simd::{cmp::SimdPartialOrd, Select, Simd};

// ── Struct ────────────────────────────────────────────────────────────────────

/// Multi-lane ring buffer with a mirrored backing store.
///
/// Each lane's `Vec<T>` has length `2 * capacity`.  Slot `i` and slot `i + capacity`
/// always hold the same value, so the window `vals[lane][index .. index + capacity]`
/// is always a contiguous, time-ordered slice — enabling SIMD min/max scans without
/// any copy or wrap-around logic.
#[derive(Clone)]
pub struct MultiMirrorBuffer<const B: usize, T: BufferElement = f64, S = Cold> {
    /// Per-lane backing store: each Vec has length `2 * capacity`.
    pub(crate) vals: [Vec<T>; B],
    pub(crate) index: usize,
    pub(crate) capacity: usize,
    pub(crate) count: usize,
    pub(crate) prev_idx: usize,
    pub(crate) state: std::marker::PhantomData<S>,
}

// ── Shared methods (any fill state) ──────────────────────────────────────────

impl<const B: usize, S, T: BufferElement> MultiMirrorBuffer<B, T, S> {
    /// Read all `B` lanes at `period` bars ago (0 = newest).
    #[inline(always)]
    pub fn get_by_period(&self, period: usize) -> [T; B] {
        let idx = period_to_idx(self.index, self.capacity, period);
        mb_get_values(&self.vals, idx)
    }
    #[inline(always)]
    pub fn get_count(&self) -> usize {
        self.count
    }
    #[inline(always)]
    pub fn get_idx(&self) -> usize {
        self.index
    }
    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.count == self.capacity
    }
    #[inline(always)]
    pub fn get_prev_idx(&self) -> usize {
        self.prev_idx
    }
    #[inline(always)]
    pub fn get_capacity(&self) -> usize {
        self.capacity
    }

    /// Convert a window index (0 = oldest slot in the current view) to bars-ago (0 = newest).
    #[inline(always)]
    pub fn window_index_to_bars_ago(&self, window_index: usize) -> usize {
        self.capacity - 1 - window_index
    }

    /// Contiguous ordered window per lane: `vals[lane][index .. index + capacity - offset]`.
    ///
    /// With `offset = 0` returns the full `capacity`-element window (oldest → newest).
    /// With `offset = 1` returns `capacity - 1` elements (excludes the newest slot,
    /// used when scanning for the new max/min without the bar just written).
    #[inline(always)]
    pub fn get_slices(&self, offset: usize) -> [&[T]; B] {
        core::array::from_fn(|lane| unsafe {
            self.vals[lane].get_unchecked(self.index..self.index + self.capacity - offset)
        })
    }

    #[inline(always)]
    pub(crate) fn update_internals(&mut self) {
        mb_advance(
            &mut self.index,
            &mut self.prev_idx,
            &mut self.count,
            self.capacity,
        );
    }

    #[inline(always)]
    pub(crate) fn update_internals_unchecked(&mut self) {
        mb_advance_unchecked(&mut self.index, &mut self.prev_idx, self.capacity);
    }
}

// ── Cold-specific ─────────────────────────────────────────────────────────────

impl<const B: usize, T: BufferElement> MultiMirrorBuffer<B, T, Cold> {
    /// Create a new, empty mirror buffer. Each lane allocates `2 * capacity` slots.
    pub fn new(capacity: usize) -> Self {
        Self {
            vals: core::array::from_fn(|_| vec![T::default(); capacity * 2]),
            index: 0,
            prev_idx: 0,
            capacity,
            count: 0,
            state: std::marker::PhantomData,
        }
    }

    /// Build a mirror buffer from per-lane slices.
    ///
    /// Each lane's backing store is `2 * capacity` slots; the second half is an
    /// exact copy of the first so the mirror invariant holds immediately.
    pub fn from_slice(vals: [&[T]; B], capacity: usize) -> Self {
        let count = vals[0].len().min(capacity);
        let buffer_vals: [Vec<T>; B] = core::array::from_fn(|lane| {
            let mut vec = vals[lane].to_vec();
            if count < capacity {
                vec.resize(capacity, T::default());
            }
            vec.extend_from_within(..);
            vec
        });
        let index = count % capacity;
        Self {
            vals: buffer_vals,
            index,
            prev_idx: index.wrapping_sub(1) % capacity,
            capacity,
            count,
            state: std::marker::PhantomData,
        }
    }

    /// Push `values` into the mirror buffer during the warmup phase.
    #[inline(always)]
    pub fn push(&mut self, values: [T; B]) {
        mb_write_values_mirror(&mut self.vals, self.index, self.capacity, values);
        self.update_internals();
    }

    /// Push `values`, evicting and returning the oldest element once full.
    #[inline(always)]
    pub fn push_with_info(&mut self, values: [T; B]) -> Option<[T; B]> {
        if self.count == self.capacity {
            let replaced =
                mb_write_values_mirror_pop(&mut self.vals, self.index, self.capacity, values);
            self.update_internals_unchecked();
            return Some(replaced);
        }
        mb_write_values_mirror(&mut self.vals, self.index, self.capacity, values);
        self.update_internals();
        None
    }

    /// Transition to [`MultiMirrorBuffer<B, T, Warm>`].
    ///
    /// # Panics (debug builds)
    /// Panics when `debug_assertions` are enabled and `is_full()` is `false`.
    pub fn into_full(self) -> MultiMirrorBuffer<B, T, Warm> {
        debug_assert!(
            self.is_full(),
            "MultiMirrorBuffer::into_full called on non-full buffer"
        );
        MultiMirrorBuffer {
            vals: self.vals,
            index: self.index,
            capacity: self.capacity,
            count: self.count,
            prev_idx: self.prev_idx,
            state: std::marker::PhantomData,
        }
    }

    /// Oldest set of values, or `None` if empty.
    #[inline(always)]
    pub fn front(&self) -> Option<[T; B]> {
        if self.count == 0 {
            None
        } else {
            Some(mb_get_values(&self.vals, self.index))
        }
    }

    /// Most recently pushed values, or `None` if empty.
    #[inline(always)]
    pub fn back(&self) -> Option<[T; B]> {
        if self.count == 0 {
            None
        } else {
            Some(mb_get_values(&self.vals, self.prev_idx))
        }
    }
}

// ── MinMax (Cold f64) ─────────────────────────────────────────────────────────
//
// Cold buffers fill from vals[lane][0..count] — no wrapping, so the ordered
// window for look_back is vals[lane][count-take..count] where take = look_back.min(count).
// trail after rescan = slice.len() - 1 - max_idx  (window may be smaller than capacity).

impl<const B: usize> MultiMirrorBuffer<B, f64, Cold> {
    #[inline(always)]
    fn cold_window(&self, lane: usize, look_back: usize) -> &[f64] {
        let take = look_back.min(self.count);
        &self.vals[lane][self.count - take..self.count]
    }

    #[inline(always)]
    pub fn max<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MaxState<B>,
        bar: Simd<f64, B>,
        look_back: usize,
    ) -> (Simd<f64, B>, Simd<usize, B>) {
        let (mut max, mut trail) = (state.max, state.trail);
        trail += UsizeConstants::ONE;

        let lookback_simd = Simd::splat(look_back);
        let needs_search = lookback_simd.simd_le(trail);
        let search_mask = needs_search.to_bitmask();

        if search_mask == 0 {
            let current_is_new_max = bar.simd_ge(max);
            max = current_is_new_max.select(bar, max);
            trail = current_is_new_max.select(UsizeConstants::ZERO, trail);
        } else {
            let max_array = max.as_mut_array();
            let trail_array = trail.as_mut_array();
            for lane in 0..B {
                if search_mask & (1 << lane) != 0 {
                    let slice = self.cold_window(lane, look_back);
                    let (max_val, max_idx) = if CHUNK_SIZE == 1 {
                        find_max_scalar_1(slice)
                    } else {
                        find_max_simd::<CHUNK_SIZE>(slice)
                    };
                    max_array[lane] = max_val;
                    trail_array[lane] = slice.len().saturating_sub(1 + max_idx);
                } else if bar[lane] >= max_array[lane] {
                    max_array[lane] = bar[lane];
                    trail_array[lane] = 0;
                }
            }
            state.max = max;
            state.trail = trail;
            return (max, trail);
        }

        (state.max, state.trail) = (max, trail);
        (max, trail)
    }

    #[inline(always)]
    pub fn min<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MinState<B>,
        bar: Simd<f64, B>,
        look_back: usize,
    ) -> (Simd<f64, B>, Simd<usize, B>) {
        let (mut min, mut trail) = (state.min, state.trail);
        trail += UsizeConstants::ONE;

        let lookback_simd = Simd::splat(look_back);
        let needs_search = lookback_simd.simd_le(trail);
        let search_mask = needs_search.to_bitmask();

        if search_mask == 0 {
            let current_is_new_min = bar.simd_le(min);
            min = current_is_new_min.select(bar, min);
            trail = current_is_new_min.select(UsizeConstants::ZERO, trail);
        } else {
            let min_array = min.as_mut_array();
            let trail_array = trail.as_mut_array();
            for lane in 0..B {
                if search_mask & (1 << lane) != 0 {
                    let slice = self.cold_window(lane, look_back);
                    let (min_val, min_idx) = if CHUNK_SIZE == 1 {
                        find_min_scalar_1(slice)
                    } else {
                        find_min_simd::<CHUNK_SIZE>(slice)
                    };
                    min_array[lane] = min_val;
                    trail_array[lane] = slice.len().saturating_sub(1 + min_idx);
                } else if bar[lane] <= min_array[lane] {
                    min_array[lane] = bar[lane];
                    trail_array[lane] = 0;
                }
            }
            state.min = min;
            state.trail = trail;
            return (min, trail);
        }

        (state.min, state.trail) = (min, trail);
        (min, trail)
    }
}

// ── Warm-specific ─────────────────────────────────────────────────────────────

impl<const B: usize, T: BufferElement> MultiMirrorBuffer<B, T, Warm> {
    /// Create a Warm mirror buffer pre-filled with `T::default()`.
    pub fn new(capacity: usize) -> Self {
        Self {
            vals: core::array::from_fn(|_| vec![T::default(); capacity * 2]),
            index: 0,
            prev_idx: capacity.saturating_sub(1),
            capacity,
            count: capacity,
            state: std::marker::PhantomData,
        }
    }

    /// Build a Warm mirror buffer from per-lane slices (always full).
    pub fn from_slice(vals: [&[T]; B], capacity: usize) -> Self {
        let buffer_vals: [Vec<T>; B] = core::array::from_fn(|lane| {
            let mut vec = vals[lane].to_vec();
            vec.resize(capacity, T::default());
            vec.extend_from_within(..);
            vec
        });
        Self {
            vals: buffer_vals,
            index: 0,
            prev_idx: capacity.saturating_sub(1),
            capacity,
            count: capacity,
            state: std::marker::PhantomData,
        }
    }

    /// Oldest values (always valid; buffer is guaranteed full).
    #[inline(always)]
    pub fn front(&self) -> [T; B] {
        mb_get_values(&self.vals, self.index)
    }

    /// Most recently pushed values (always valid; buffer is guaranteed full).
    #[inline(always)]
    pub fn back(&self) -> [T; B] {
        mb_get_values(&self.vals, self.prev_idx)
    }

    /// Push `values` — mirror, branchless (buffer guaranteed full).
    #[inline(always)]
    pub fn push(&mut self, values: [T; B]) {
        mb_write_values_mirror(&mut self.vals, self.index, self.capacity, values);
        self.update_internals_unchecked();
    }

    /// Push `values`, evict and return the oldest element — mirror, branchless.
    #[inline(always)]
    pub fn push_with_info(&mut self, values: [T; B]) -> [T; B] {
        let replaced =
            mb_write_values_mirror_pop(&mut self.vals, self.index, self.capacity, values);
        self.update_internals_unchecked();
        replaced
    }

    /// Gather `B` scalar `MirrorBuffer<Warm, T>` references into a `MultiMirrorBuffer<B, T, Warm>`.
    ///
    /// Used by `simd_state_impl!` as `from_mirror_buffers`. Each input buffer must have
    /// been allocated with mirror layout (`vals.len() == 2 * capacity`).
    pub fn from_mirror_buffers(buffers: Vec<&SingleMirrorBuffer<Warm, T>>) -> Self {
        debug_assert_eq!(buffers.len(), B);
        let capacity = buffers[0].capacity;
        let buffer_vals: [Vec<T>; B] = core::array::from_fn(|i| buffers[i].vals.clone());
        Self {
            vals: buffer_vals,
            index: buffers[0].index,
            prev_idx: buffers[0].prev_idx,
            capacity,
            count: capacity,
            state: std::marker::PhantomData,
        }
    }

    /// Scatter back into `B` scalar `MirrorBuffer<Warm, T>` values (mirror layout preserved).
    pub fn to_f64_buffers(&self) -> Vec<SingleMirrorBuffer<Warm, T>> {
        (0..B)
            .map(|i| SingleMirrorBuffer {
                vals: self.vals[i].clone(),
                index: self.index,
                prev_idx: self.prev_idx,
                capacity: self.capacity,
                count: self.count,
                state: std::marker::PhantomData,
            })
            .collect()
    }

    /// Raw slice for a single lane (mirror order; length = `2 * capacity`).
    #[inline(always)]
    pub fn get_slice(&self, lane: usize) -> &[T] {
        unsafe { self.vals[lane].get_unchecked(self.index..self.index + self.capacity) }
    }
}

// ── MinMaxBuffer (Warm f64 only) ──────────────────────────────────────────────
//
// Inherent methods replacing the old MinMaxBuffer trait on MultiBuffer<B, f64, Warm>.

impl<const B: usize> MultiMirrorBuffer<B, f64, Warm> {
    #[inline(always)]
    pub fn max(
        &self,
        state: &mut MaxState<B>,
        bar: Simd<f64, B>,
        look_back: usize,
    ) -> (Simd<f64, B>, Simd<usize, B>) {
        self.max_chuncked::<4>(state, bar, look_back)
    }
    /// Rolling maximum over the current mirror window across all `B` lanes.
    ///
    /// `CHUNK_SIZE` controls the SIMD width used during full rescans (1 = scalar).
    #[inline(always)]
    pub fn max_chuncked<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MaxState<B>,
        bar: Simd<f64, B>,
        look_back: usize,
    ) -> (Simd<f64, B>, Simd<usize, B>) {
        let (mut max, mut trail) = (state.max, state.trail);
        trail += UsizeConstants::ONE;

        let lookback_simd = Simd::splat(look_back);
        // Trigger a rescan whenever trail >= look_back (not just ==), because trail
        // can be initialised to `period` and then incremented past it before the
        // first warm bar.
        let needs_search = lookback_simd.simd_le(trail);
        let search_mask = needs_search.to_bitmask();

        if search_mask == 0 {
            // No lane needs a rescan — incremental update only.
            let current_is_new_max = bar.simd_ge(max);
            max = current_is_new_max.select(bar, max);
            trail = current_is_new_max.select(UsizeConstants::ZERO, trail);
        } else {
            let max_array = max.as_mut_array();
            let trail_array = trail.as_mut_array();
            for lane in 0..B {
                if search_mask & (1 << lane) != 0 {
                    let (max_val, max_idx) = if CHUNK_SIZE == 1 {
                        find_max_scalar(self.get_slices(1)[lane], bar[lane])
                    } else {
                        find_max_simd::<CHUNK_SIZE>(self.get_slices(0)[lane])
                    };
                    max_array[lane] = max_val;
                    trail_array[lane] = self.window_index_to_bars_ago(max_idx);
                } else {
                    // Incremental update for lanes that don't need a rescan.
                    if bar[lane] >= max_array[lane] {
                        max_array[lane] = bar[lane];
                        trail_array[lane] = 0;
                    }
                }
            }
            // Write back
            state.max = max;
            state.trail = trail;
            return (max, trail);
        }

        (state.max, state.trail) = (max, trail);
        (max, trail)
    }

    #[inline(always)]
    pub fn min(
        &self,
        state: &mut MinState<B>,
        bar: Simd<f64, B>,
        look_back: usize,
    ) -> (Simd<f64, B>, Simd<usize, B>) {
        self.min_chuncked::<4>(state, bar, look_back)
    }
    /// Rolling minimum over the current mirror window across all `B` lanes.
    ///
    /// `CHUNK_SIZE` controls the SIMD width used during full rescans (1 = scalar).
    #[inline(always)]
    pub fn min_chuncked<const CHUNK_SIZE: usize>(
        &self,
        state: &mut MinState<B>,
        bar: Simd<f64, B>,
        look_back: usize,
    ) -> (Simd<f64, B>, Simd<usize, B>) {
        let (mut min, mut trail) = (state.min, state.trail);
        trail += UsizeConstants::ONE;

        let lookback_simd = Simd::splat(look_back);
        // Trigger a rescan whenever trail >= look_back (not just ==).
        let needs_search = lookback_simd.simd_le(trail);
        let search_mask = needs_search.to_bitmask();

        if search_mask == 0 {
            // No lane needs a rescan — incremental update only.
            let current_is_new_min = bar.simd_le(min);
            min = current_is_new_min.select(bar, min);
            trail = current_is_new_min.select(UsizeConstants::ZERO, trail);
        } else {
            let min_array = min.as_mut_array();
            let trail_array = trail.as_mut_array();
            for lane in 0..B {
                if search_mask & (1 << lane) != 0 {
                    let (min_val, min_idx) = if CHUNK_SIZE == 1 {
                        find_min_scalar(self.get_slices(1)[lane], bar[lane])
                    } else {
                        find_min_simd::<CHUNK_SIZE>(self.get_slices(0)[lane])
                    };
                    min_array[lane] = min_val;
                    trail_array[lane] = self.window_index_to_bars_ago(min_idx);
                } else {
                    // Incremental update for lanes that don't need a rescan.
                    if bar[lane] <= min_array[lane] {
                        min_array[lane] = bar[lane];
                        trail_array[lane] = 0;
                    }
                }
            }
            // Write back
            state.min = min;
            state.trail = trail;
            return (min, trail);
        }

        (state.min, state.trail) = (min, trail);
        (min, trail)
    }
}

// ── Serde ─────────────────────────────────────────────────────────────────────
//
// Hand-rolled (identical in structure to the MultiBuffer serde impl) because the
// `S` typestate carries no runtime data and must be excluded from the wire format.

#[derive(Serialize, Deserialize)]
struct MultiMirrorBufferSerde<R> {
    vals: Vec<Vec<R>>,
    index: usize,
    capacity: usize,
    count: usize,
    prev_idx: usize,
}

impl<const N: usize, Stat, T: BufferElement + SerdeElement> Serialize
    for MultiMirrorBuffer<N, T, Stat>
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let helper = MultiMirrorBufferSerde {
            vals: self
                .vals
                .iter()
                .map(|lane| lane.iter().map(|v| T::to_repr(*v)).collect())
                .collect(),
            index: self.index,
            capacity: self.capacity,
            count: self.count,
            prev_idx: self.prev_idx,
        };
        helper.serialize(serializer)
    }
}

impl<'de, const N: usize, Stat, T: BufferElement + SerdeElement> Deserialize<'de>
    for MultiMirrorBuffer<N, T, Stat>
where
    T::Repr: Deserialize<'de>,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let helper = MultiMirrorBufferSerde::<T::Repr>::deserialize(deserializer)?;
        if helper.vals.len() != N {
            return Err(serde::de::Error::custom(format!(
                "Expected {} buffers, got {}",
                N,
                helper.vals.len()
            )));
        }
        let vals_array: [Vec<T>; N] = helper
            .vals
            .into_iter()
            .map(|lane| lane.into_iter().map(T::from_repr).collect())
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| serde::de::Error::custom("Failed to convert to array"))?;
        Ok(MultiMirrorBuffer {
            vals: vals_array,
            index: helper.index,
            capacity: helper.capacity,
            count: helper.count,
            prev_idx: helper.prev_idx,
            state: std::marker::PhantomData,
        })
    }
}
