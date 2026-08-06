//! Per-lane mirror buffer (unsynchronized — each lane may have a different capacity).
//!
//! `UnsyncMirrorBuffer<B, T, S>` has identical fields to `UnsyncBuffer<B, T, S>` but
//! each lane's Vec is allocated `2 * capacity[lane]` slots, maintaining the mirror
//! invariant for min/max SIMD scans with per-lane periods.

use crate::indicators::simd_indicators::{
    max_simd::{find_max_scalar, options::SimdState as MaxState, CHUNK_1},
    min_simd::{find_min_scalar, options::SimdState as MinState},
    simd_types::UsizeConstants,
};
use crate::indicators::{
    max::{find_max_scalar as find_max_scalar_1, find_max_simd},
    min::{find_min_scalar as find_min_scalar_1, find_min_simd},
};
use crate::ring_buffer::{
    buffer::SerdeElement,
    single_buffer::{
        generic_buffer::{Cold, Warm},
        mirror_buffer::MirrorBuffer,
    },
    unsync_multi_buffer::multi_buffer::{
        usb_advance, usb_advance_unchecked, usb_get_values, usb_write_mirror, usb_write_mirror_pop,
        BufferElement,
    },
};
use serde::{Deserialize, Serialize};
use std::simd::{
    cmp::{SimdPartialEq, SimdPartialOrd},
    Mask, Select, Simd, SimdElement,
};

// ── Struct ─────────────────────────────────────────────────────────────────

pub struct UnsyncMirrorBuffer<const B: usize, T: BufferElement + SimdElement = f64, S = Cold> {
    /// Per-lane backing: each `vals[i]` has length `2 * capacity[i]`.
    pub(crate) vals: [Vec<T>; B],
    pub(crate) index: Simd<usize, B>,
    pub(crate) capacity: Simd<usize, B>,
    pub(crate) count: Simd<usize, B>,
    pub(crate) prev_idx: Simd<usize, B>,
    pub(crate) state: std::marker::PhantomData<S>,
}

// ── Shared methods ─────────────────────────────────────────────────────────

impl<const B: usize, S, T: BufferElement + SimdElement> UnsyncMirrorBuffer<B, T, S> {
    #[inline(always)]
    pub fn get_count(&self) -> Simd<usize, B> {
        self.count
    }
    #[inline(always)]
    pub fn get_idx(&self) -> Simd<usize, B> {
        self.index
    }
    #[inline(always)]
    pub fn get_capacity(&self) -> Simd<usize, B> {
        self.capacity
    }
    #[inline(always)]
    pub fn get_prev_idx(&self) -> Simd<usize, B> {
        self.prev_idx
    }
    #[inline(always)]
    pub fn is_full(&self) -> Mask<i64, B> {
        self.count.simd_eq(self.capacity).cast::<i64>()
    }
    #[inline(always)]
    pub fn raw_slice(&self) -> &[Vec<T>; B] {
        &self.vals
    }

    /// Contiguous ordered window per lane: `vals[lane][index[lane]..index[lane]+capacity[lane]-offset]`.
    #[inline(always)]
    pub fn get_slices(&self, offset: usize) -> [&[T]; B] {
        std::array::from_fn(|lane| unsafe {
            self.vals[lane]
                .get_unchecked(self.index[lane]..self.index[lane] + self.capacity[lane] - offset)
        })
    }

    #[inline(always)]
    pub fn window_index_to_bars_ago(&self, window_index: usize, lane: usize) -> usize {
        self.capacity[lane] - 1 - window_index
    }

    #[inline(always)]
    pub(crate) fn update_internals(&mut self) {
        usb_advance(
            &mut self.index,
            &mut self.prev_idx,
            &mut self.count,
            self.capacity,
        );
    }

    #[inline(always)]
    pub(crate) fn update_internals_unchecked(&mut self) {
        usb_advance_unchecked(&mut self.index, &mut self.prev_idx, self.capacity);
    }
}

// ── Cold-specific ──────────────────────────────────────────────────────────

impl<const B: usize, T: BufferElement + SimdElement> UnsyncMirrorBuffer<B, T, Cold> {
    pub fn new(capacity: [usize; B]) -> Self {
        let vals = core::array::from_fn(|i| vec![T::default(); capacity[i] * 2]);
        Self {
            vals,
            index: Simd::splat(0),
            prev_idx: Simd::splat(0),
            capacity: Simd::from_array(capacity),
            count: Simd::splat(0),
            state: std::marker::PhantomData,
        }
    }

    pub fn from_slice(vals: [&[T]; B], capacity: [usize; B]) -> Self {
        let count = core::array::from_fn(|i| vals[i].len().min(capacity[i]));
        let count_simd = Simd::from_array(count);
        let capacity_simd = Simd::from_array(capacity);
        let buffer_vals: [Vec<T>; B] = core::array::from_fn(|lane| {
            let mut vec = vals[lane].to_vec();
            if count[lane] < capacity[lane] {
                vec.resize(capacity[lane], T::default());
            }
            vec.extend_from_within(..);
            vec
        });
        let index = count_simd % capacity_simd;
        let prev_idx = (index + capacity_simd - Simd::splat(1)) % capacity_simd;
        Self {
            vals: buffer_vals,
            index,
            prev_idx,
            capacity: capacity_simd,
            count: count_simd,
            state: std::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub fn push(&mut self, values: Simd<T, B>) {
        usb_write_mirror(&mut self.vals, &self.index, &self.capacity, values);
        self.update_internals();
    }

    #[inline(always)]
    pub fn push_with_info(&mut self, values: Simd<T, B>) -> (Simd<T, B>, Mask<i64, B>) {
        let replaced = usb_write_mirror_pop(&mut self.vals, &self.index, &self.capacity, values);
        let mask = self.is_full();
        self.update_internals();
        (replaced, mask)
    }

    pub fn into_full(self) -> UnsyncMirrorBuffer<B, T, Warm> {
        debug_assert!(
            self.count.simd_eq(self.capacity).to_bitmask() == (1u64 << B) - 1,
            "UnsyncMirrorBuffer::into_full: not all lanes are full"
        );
        UnsyncMirrorBuffer {
            vals: self.vals,
            index: self.index,
            capacity: self.capacity,
            count: self.count,
            prev_idx: self.prev_idx,
            state: std::marker::PhantomData,
        }
    }

    pub fn front(&self) -> (Simd<T, B>, Mask<i64, B>) {
        (usb_get_values(&self.vals, self.index), self.is_full())
    }

    pub fn back(&self) -> (Simd<T, B>, Mask<i64, B>) {
        (usb_get_values(&self.vals, self.prev_idx), self.is_full())
    }
}

// ── MinMax (Cold f64) ────────────────────────────────────────────────────────
//
// Cold buffers fill from vals[lane][0..count[lane]] — no wrapping.
// The ordered window for look_back lb is vals[lane][count[lane]-take..count[lane]]
// where take = lb.min(count[lane]).
// trail after rescan = slice.len() - 1 - max_idx.
// look_back is Simd<usize, B> (per-lane, matching the Warm signature).

impl<const B: usize> UnsyncMirrorBuffer<B, f64, Cold> {
    #[inline(always)]
    fn cold_window(&self, lane: usize, look_back: usize) -> &[f64] {
        let count = self.count[lane];
        let take = look_back.min(count);
        &self.vals[lane][count - take..count]
    }

    #[inline(always)]
    pub fn max(
        &self,
        state: &mut MaxState<B>,
        bar: Simd<f64, B>,
        look_back: Simd<usize, B>,
    ) -> (Simd<f64, B>, Simd<usize, B>) {
        let (mut max, mut trail) = (state.max, state.trail);
        trail += UsizeConstants::ONE;

        let needs_search = look_back.simd_le(trail);
        let search_mask = needs_search.to_bitmask();

        if search_mask == 0 {
            let current_is_new_max = bar.simd_ge(max);
            max = current_is_new_max.select(bar, max);
            trail = current_is_new_max.select(UsizeConstants::ZERO, trail);
        } else {
            let max_array = max.as_mut_array();
            let trail_array = trail.as_mut_array();
            let look_back_array = look_back.as_array();
            for lane in 0..B {
                if search_mask & (1 << lane) != 0 {
                    let slice = self.cold_window(lane, look_back_array[lane]);
                    let (max_val, max_idx) = match look_back_array[lane] {
                        1..=14 => find_max_scalar_1(slice),
                        _ => find_max_simd::<4>(slice),
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
    pub fn min(
        &self,
        state: &mut MinState<B>,
        bar: Simd<f64, B>,
        look_back: Simd<usize, B>,
    ) -> (Simd<f64, B>, Simd<usize, B>) {
        let (mut min, mut trail) = (state.min, state.trail);
        trail += UsizeConstants::ONE;

        let needs_search = look_back.simd_le(trail);
        let search_mask = needs_search.to_bitmask();

        if search_mask == 0 {
            let current_is_new_min = bar.simd_le(min);
            min = current_is_new_min.select(bar, min);
            trail = current_is_new_min.select(UsizeConstants::ZERO, trail);
        } else {
            let min_array = min.as_mut_array();
            let trail_array = trail.as_mut_array();
            let look_back_array = look_back.as_array();
            for lane in 0..B {
                if search_mask & (1 << lane) != 0 {
                    let slice = self.cold_window(lane, look_back_array[lane]);
                    let (min_val, min_idx) = match look_back_array[lane] {
                        1..=14 => find_min_scalar_1(slice),
                        _ => find_min_simd::<4>(slice),
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

// ── Warm-specific ──────────────────────────────────────────────────────────

impl<const B: usize, T: BufferElement + SimdElement> UnsyncMirrorBuffer<B, T, Warm> {
    pub fn new(capacity: [usize; B]) -> Self {
        let vals = core::array::from_fn(|i| vec![T::default(); capacity[i] * 2]);
        let capacity_simd = Simd::from_array(capacity);
        Self {
            vals,
            index: Simd::splat(0),
            prev_idx: capacity_simd - Simd::splat(1),
            capacity: capacity_simd,
            count: capacity_simd,
            state: std::marker::PhantomData,
        }
    }

    pub fn from_slice(vals: [&[T]; B], capacity: [usize; B]) -> Self {
        let buffer_vals: [Vec<T>; B] = core::array::from_fn(|lane| {
            let mut vec = vals[lane].to_vec();
            vec.resize(capacity[lane], T::default());
            vec.extend_from_within(..);
            vec
        });
        let capacity_simd = Simd::from_array(capacity);
        Self {
            vals: buffer_vals,
            index: Simd::splat(0),
            prev_idx: capacity_simd - Simd::splat(1),
            capacity: capacity_simd,
            count: capacity_simd,
            state: std::marker::PhantomData,
        }
    }
    #[inline(always)]
    pub fn front(&self) -> Simd<T, B> {
        usb_get_values(&self.vals, self.index)
    }
    #[inline(always)]
    pub fn back(&self) -> Simd<T, B> {
        usb_get_values(&self.vals, self.prev_idx)
    }

    #[inline(always)]
    pub fn push(&mut self, values: Simd<T, B>) {
        usb_write_mirror(&mut self.vals, &self.index, &self.capacity, values);
        self.update_internals_unchecked();
    }

    #[inline(always)]
    pub fn push_with_info(&mut self, values: Simd<T, B>) -> (Simd<T, B>, Mask<i64, B>) {
        let replaced = usb_write_mirror_pop(&mut self.vals, &self.index, &self.capacity, values);
        self.update_internals_unchecked();
        (replaced, Mask::splat(true))
    }

    /// Gather N scalar `MirrorBuffer<Warm, T>` references into an
    /// `UnsyncMirrorBuffer<B, T, Warm>`.
    /// Used by `simd_state_impl!` as the `from_mirror_buffers` constructor.
    pub fn from_mirror_buffers(buffers: Vec<&MirrorBuffer<Warm, T>>) -> Self {
        debug_assert_eq!(buffers.len(), B);
        let mut index = [0usize; B];
        let mut prev_idx = [0usize; B];
        let mut capacity = [0usize; B];
        let mut count = [0usize; B];
        let vals: [Vec<T>; B] = std::array::from_fn(|lane| buffers[lane].vals.clone());
        for (lane, buf) in buffers.iter().enumerate() {
            index[lane] = buf.index;
            prev_idx[lane] = buf.prev_idx;
            capacity[lane] = buf.capacity;
            count[lane] = buf.count;
        }
        UnsyncMirrorBuffer {
            vals,
            index: Simd::from_array(index),
            prev_idx: Simd::from_array(prev_idx),
            capacity: Simd::from_array(capacity),
            count: Simd::from_array(count),
            state: std::marker::PhantomData,
        }
    }

    /// Scatter back into N scalar `MirrorBuffer<Warm, T>` instances.
    pub fn to_f64_buffers(&self) -> Vec<MirrorBuffer<Warm, T>> {
        (0..B)
            .map(|lane| MirrorBuffer {
                vals: self.vals[lane].clone(),
                index: self.index[lane],
                prev_idx: self.prev_idx[lane],
                capacity: self.capacity[lane],
                count: self.count[lane],
                state: std::marker::PhantomData,
            })
            .collect()
    }

    /// Raw backing for a single lane (ring-ordered, with mirror copy).
    #[inline(always)]
    pub fn get_slice(&self, lane: usize) -> &[T] {
        &self.vals[lane]
    }
}

// ── MinMaxBuffer (Warm f64 only) ──────────────────────────────────────────

impl<const B: usize> UnsyncMirrorBuffer<B, f64, Warm> {
    #[inline(always)]
    pub fn max(
        &self,
        state: &mut MaxState<B>,
        bar: Simd<f64, B>,
        look_back: Simd<usize, B>,
    ) -> (Simd<f64, B>, Simd<usize, B>) {
        let (mut max, mut trail) = (state.max, state.trail);
        trail += UsizeConstants::ONE;

        let needs_search = look_back.simd_le(trail);
        let search_mask = needs_search.to_bitmask();

        if search_mask == 0 {
            let current_is_new_max = bar.simd_ge(max);
            max = current_is_new_max.select(bar, max);
            trail = current_is_new_max.select(UsizeConstants::ZERO, trail);
        } else {
            let max_array = max.as_mut_array();
            let trail_array = trail.as_mut_array();
            let look_back_array = look_back.as_array();
            for lane in 0..B {
                if search_mask & (1 << lane) != 0 {
                    let (max_val, max_idx) = match look_back_array[lane] {
                        1..=14 => find_max_scalar(self.get_slices(1)[lane], bar[lane]),
                        _ => find_max_simd::<4>(self.get_slices(0)[lane]),
                    };
                    max_array[lane] = max_val;
                    trail_array[lane] = self.window_index_to_bars_ago(max_idx, lane);
                } else {
                    if bar[lane] >= max_array[lane] {
                        max_array[lane] = bar[lane];
                        trail_array[lane] = 0;
                    }
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
    pub fn min(
        &self,
        state: &mut MinState<B>,
        bar: Simd<f64, B>,
        look_back: Simd<usize, B>,
    ) -> (Simd<f64, B>, Simd<usize, B>) {
        let (mut min, mut trail) = (state.min, state.trail);
        trail += UsizeConstants::ONE;

        let needs_search = look_back.simd_le(trail);
        let search_mask = needs_search.to_bitmask();

        if search_mask == 0 {
            let current_is_new_min = bar.simd_le(min);
            min = current_is_new_min.select(bar, min);
            trail = current_is_new_min.select(UsizeConstants::ZERO, trail);
        } else {
            let min_array = min.as_mut_array();
            let trail_array = trail.as_mut_array();
            let look_back_array = look_back.as_array();
            for lane in 0..B {
                if search_mask & (1 << lane) != 0 {
                    let (min_val, min_idx) = if CHUNK_1.contains(&look_back_array[lane]) {
                        find_min_scalar(self.get_slices(1)[lane], bar[lane])
                    } else {
                        find_min_simd::<4>(self.get_slices(0)[lane])
                    };
                    min_array[lane] = min_val;
                    trail_array[lane] = self.window_index_to_bars_ago(min_idx, lane);
                } else {
                    if bar[lane] <= min_array[lane] {
                        min_array[lane] = bar[lane];
                        trail_array[lane] = 0;
                    }
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

// ── Serde ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct UnsyncMirrorSerde<R> {
    vals: Vec<Vec<R>>,
    index: Vec<usize>,
    capacity: Vec<usize>,
    count: Vec<usize>,
    prev_idx: Vec<usize>,
}

impl<const B: usize, S, T: BufferElement + SerdeElement + SimdElement> Serialize
    for UnsyncMirrorBuffer<B, T, S>
{
    fn serialize<Ser: serde::Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
        let helper = UnsyncMirrorSerde {
            vals: self
                .vals
                .iter()
                .map(|lane| lane.iter().map(|v| T::to_repr(*v)).collect())
                .collect(),
            index: self.index.to_array().into_iter().collect(),
            capacity: self.capacity.to_array().into_iter().collect(),
            count: self.count.to_array().into_iter().collect(),
            prev_idx: self.prev_idx.to_array().into_iter().collect(),
        };
        helper.serialize(serializer)
    }
}

impl<'de, const B: usize, S, T: BufferElement + SerdeElement + SimdElement> Deserialize<'de>
    for UnsyncMirrorBuffer<B, T, S>
where
    T::Repr: Deserialize<'de>,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let helper = UnsyncMirrorSerde::<T::Repr>::deserialize(deserializer)?;
        if helper.vals.len() != B {
            return Err(serde::de::Error::custom(format!(
                "Expected {} buffers, got {}",
                B,
                helper.vals.len()
            )));
        }
        let vals_array: [Vec<T>; B] = helper
            .vals
            .into_iter()
            .map(|lane| lane.into_iter().map(T::from_repr).collect())
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| serde::de::Error::custom("Failed to convert vals to array"))?;
        let index_arr: [usize; B] = helper
            .index
            .try_into()
            .map_err(|_| serde::de::Error::custom("index length mismatch"))?;
        let capacity_arr: [usize; B] = helper
            .capacity
            .try_into()
            .map_err(|_| serde::de::Error::custom("capacity length mismatch"))?;
        let count_arr: [usize; B] = helper
            .count
            .try_into()
            .map_err(|_| serde::de::Error::custom("count length mismatch"))?;
        let prev_arr: [usize; B] = helper
            .prev_idx
            .try_into()
            .map_err(|_| serde::de::Error::custom("prev_idx length mismatch"))?;
        Ok(UnsyncMirrorBuffer {
            vals: vals_array,
            index: Simd::from_array(index_arr),
            capacity: Simd::from_array(capacity_arr),
            count: Simd::from_array(count_arr),
            prev_idx: Simd::from_array(prev_arr),
            state: std::marker::PhantomData,
        })
    }
}
