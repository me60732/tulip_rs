use crate::ring_buffer::buffer::{period_to_idx, BufferElement};
use crate::ring_buffer::multi_type_buffer::layout::{Layout, SerdeElement};
use serde::{Deserialize, Serialize};

// ── Struct ────────────────────────────────────────────────────────────────────

/// A heterogeneous multi-buffer ring (or mirror) buffer.
///
/// `L` is a tuple type such as `(f64, Simd<f64, 2>)` that implements [`Layout`].
/// Each tuple element corresponds to one internal `Vec` of a distinct type.
///
/// # Ring buffer usage
/// ```ignore
/// let mut buf = MultiTypeBuffer::<(f64, Simd<f64, 2>)>::new(period);
/// if let Some((old_tr, old_vm)) = buf.push_with_info((tr, vm_simd)) { ... }
/// ```
///
/// # Mirror buffer usage
/// ```ignore
/// let mut buf = MultiTypeBuffer::<(f64, f64)>::new_mirror(period);
/// buf.push_mirror((high, low));
/// let (h_slice, l_slice) = buf.get_slices(0);
/// ```
pub struct MultiTypeBuffer<L: Layout> {
    pub(crate) vecs: L::Vecs,
    pub(crate) index: usize,
    pub(crate) capacity: usize,
    pub(crate) count: usize,
    pub(crate) prev_idx: usize,
}

// ── Clone ─────────────────────────────────────────────────────────────────────

impl<L: Layout> Clone for MultiTypeBuffer<L>
where
    L::Vecs: Clone,
{
    fn clone(&self) -> Self {
        Self {
            vecs: self.vecs.clone(),
            index: self.index,
            capacity: self.capacity,
            count: self.count,
            prev_idx: self.prev_idx,
        }
    }
}

// ── Core methods ──────────────────────────────────────────────────────────────

impl<L: Layout> MultiTypeBuffer<L> {
    // ── Construction ──────────────────────────────────────────────────────────

    /// Create a ring buffer with `capacity` slots.
    pub fn new(capacity: usize) -> Self {
        Self {
            vecs: L::make_vecs(capacity),
            index: 0,
            prev_idx: 0,
            capacity,
            count: 0,
        }
    }

    /// Create a mirror buffer (allocates `capacity * 2` storage).
    pub fn new_mirror(capacity: usize) -> Self {
        Self {
            vecs: L::make_mirror_vecs(capacity),
            index: 0,
            prev_idx: 0,
            capacity,
            count: 0,
        }
    }

    // ── Ring buffer push ───────────────────────────────────────────────────────

    #[inline(always)]
    pub fn push(&mut self, values: L::Values) {
        L::write_values(&mut self.vecs, self.index, values);
        self.update_internals();
    }

    #[inline(always)]
    pub unsafe fn push_unchecked(&mut self, values: L::Values) {
        L::write_values(&mut self.vecs, self.index, values);
        self.update_internals_unchecked();
    }

    /// Push a value; if the buffer is full returns the evicted oldest value.
    #[inline(always)]
    pub fn push_with_info(&mut self, values: L::Values) -> Option<L::Values> {
        if self.count == self.capacity {
            let evicted = L::write_values_pop(&mut self.vecs, self.index, values);
            self.update_internals_unchecked();
            return Some(evicted);
        }
        L::write_values(&mut self.vecs, self.index, values);
        self.update_internals();
        None
    }

    /// Push a value assuming the buffer is already full; returns the evicted value.
    ///
    /// # Safety
    /// The buffer must be full (`is_full() == true`).
    #[inline(always)]
    pub unsafe fn push_with_info_unchecked(&mut self, values: L::Values) -> L::Values {
        let evicted = L::write_values_pop(&mut self.vecs, self.index, values);
        self.update_internals_unchecked();
        evicted
    }

    // ── Mirror buffer push ────────────────────────────────────────────────────

    /// Push to a mirror buffer (writes to `idx` and `idx + capacity`).
    #[inline(always)]
    pub fn push_mirror(&mut self, values: L::Values) {
        L::write_mirror(&mut self.vecs, self.index, self.capacity, values);
        self.update_internals();
    }

    #[inline(always)]
    pub unsafe fn push_mirror_unchecked(&mut self, values: L::Values) {
        L::write_mirror(&mut self.vecs, self.index, self.capacity, values);
        self.update_internals_unchecked();
    }

    /// Push to a mirror buffer; returns the evicted value when full.
    /// Note: index does **not** advance when the buffer is full, matching the
    /// behaviour of the existing `MirrorBuffer` impl — use `push_mirror` for
    /// the steady-state streaming loop.
    #[inline(always)]
    pub fn push_mirror_with_info(&mut self, values: L::Values) -> Option<L::Values> {
        if self.count == self.capacity {
            let evicted = L::write_mirror_pop(&mut self.vecs, self.index, self.capacity, values);
            return Some(evicted);
        }
        L::write_mirror(&mut self.vecs, self.index, self.capacity, values);
        self.update_internals();
        None
    }

    // ── Mirror buffer read ────────────────────────────────────────────────────

    /// Return a tuple of contiguous slices — one per internal buffer — trimmed
    /// by `offset` from the newest end.
    ///
    /// Only meaningful on a mirror buffer (`new_mirror`). The caller is
    /// responsible for interpreting each slice according to its element type.
    #[inline(always)]
    pub fn get_slices(&self, offset: usize) -> L::Slices<'_> {
        if self.count == self.capacity {
            L::get_slices(&self.vecs, self.index, self.index + self.count - offset)
        } else {
            L::get_slices(&self.vecs, 0, self.count - offset)
        }
    }

    // ── Reads ─────────────────────────────────────────────────────────────────

    /// The oldest element currently in the buffer (`None` if empty).
    #[inline(always)]
    pub fn front(&self) -> Option<L::Values> {
        if self.count == 0 {
            None
        } else {
            Some(L::read_values(&self.vecs, self.index))
        }
    }

    /// The oldest element, without a bounds check.
    ///
    /// # Safety
    /// Buffer must be non-empty.
    #[inline(always)]
    pub unsafe fn front_unchecked(&self) -> L::Values {
        L::read_values(&self.vecs, self.index)
    }

    /// The most recently pushed element (`None` if empty).
    #[inline(always)]
    pub fn back(&self) -> Option<L::Values> {
        if self.count == 0 {
            None
        } else {
            Some(L::read_values(&self.vecs, self.prev_idx))
        }
    }

    /// The most recently pushed element, without a bounds check.
    ///
    /// # Safety
    /// Buffer must be non-empty.
    #[inline(always)]
    pub unsafe fn back_unchecked(&self) -> L::Values {
        L::read_values(&self.vecs, self.prev_idx)
    }

    /// Fetch the element that was pushed `period` bars ago (0 = most recent).
    #[inline(always)]
    pub fn get_by_period(&self, period: usize) -> L::Values {
        let idx = period_to_idx(self.index, self.capacity, period);
        L::read_values(&self.vecs, idx)
    }

    /// Return all stored elements in chronological order (oldest → newest).
    pub fn to_ordered_vecs(&self) -> L::Vecs {
        L::to_ordered_vecs(&self.vecs, self.index, self.capacity, self.count)
    }

    // ── Inspection ────────────────────────────────────────────────────────────

    pub fn is_full(&self) -> bool {
        self.count == self.capacity
    }
    pub fn get_count(&self) -> usize {
        self.count
    }
    pub fn get_capacity(&self) -> usize {
        self.capacity
    }
    pub fn get_idx(&self) -> usize {
        self.index
    }
    pub fn get_prev_idx(&self) -> usize {
        self.prev_idx
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    #[inline(always)]
    fn update_internals(&mut self) {
        self.prev_idx = self.index;
        self.index = self.next_index();
        if self.count < self.capacity {
            self.count += 1;
        }
    }

    #[inline(always)]
    fn update_internals_unchecked(&mut self) {
        self.prev_idx = self.index;
        self.index = self.next_index();
    }

    #[inline(always)]
    fn next_index(&self) -> usize {
        let next = self.index + 1;
        if next == self.capacity {
            0
        } else {
            next
        }
    }
}

// ── Layout + Serde macro ──────────────────────────────────────────────────────
//
// Generates, for each tuple arity:
//   1. `impl Layout for (T0, T1, …)`
//   2. `impl Serialize for MultiTypeBuffer<(T0, T1, …)>`  (requires SerdeElement)
//   3. `impl Deserialize for MultiTypeBuffer<(T0, T1, …)>` (requires SerdeElement)
//
// Serialised form is a struct with five fields:
//   { index, capacity, count, prev_idx, vecs }
// where `vecs` is a tuple-of-sequences — each inner sequence holds the
// `SerdeElement::Repr` values for that buffer.

macro_rules! impl_layout {
    ($n:expr; $(($T:ident, $i:tt)),+) => {

        // ── Layout ────────────────────────────────────────────────────────────

        impl<$($T: BufferElement),+> Layout for ($($T,)+) {
            type Values = ($($T,)+);
            type Vecs   = ($(Vec<$T>,)+);
            type Slices<'a> = ($(&'a [$T],)+) where Self: 'a;

            #[inline(always)]
            fn make_vecs(capacity: usize) -> Self::Vecs {
                ($(vec![$T::default(); capacity],)+)
            }
            #[inline(always)]
            fn make_mirror_vecs(capacity: usize) -> Self::Vecs {
                ($(vec![$T::default(); capacity * 2],)+)
            }
            #[inline(always)]
            fn read_values(vecs: &Self::Vecs, idx: usize) -> Self::Values {
                ($(unsafe { *vecs.$i.get_unchecked(idx) },)+)
            }
            #[inline(always)]
            fn write_values(vecs: &mut Self::Vecs, idx: usize, values: Self::Values) {
                $(unsafe { *vecs.$i.get_unchecked_mut(idx) = values.$i; })+
            }
            #[inline(always)]
            fn write_values_pop(vecs: &mut Self::Vecs, idx: usize, values: Self::Values) -> Self::Values {
                ($(unsafe {
                    let old = *vecs.$i.get_unchecked(idx);
                    *vecs.$i.get_unchecked_mut(idx) = values.$i;
                    old
                },)+)
            }
            #[inline(always)]
            fn write_mirror(vecs: &mut Self::Vecs, idx: usize, capacity: usize, values: Self::Values) {
                $(unsafe {
                    *vecs.$i.get_unchecked_mut(idx)            = values.$i;
                    *vecs.$i.get_unchecked_mut(idx + capacity) = values.$i;
                })+
            }
            #[inline(always)]
            fn write_mirror_pop(vecs: &mut Self::Vecs, idx: usize, capacity: usize, values: Self::Values) -> Self::Values {
                ($(unsafe {
                    let old = *vecs.$i.get_unchecked(idx);
                    *vecs.$i.get_unchecked_mut(idx)            = values.$i;
                    *vecs.$i.get_unchecked_mut(idx + capacity) = values.$i;
                    old
                },)+)
            }
            #[inline(always)]
            fn get_slices<'a>(vecs: &'a Self::Vecs, start: usize, end: usize) -> Self::Slices<'a> {
                ($(unsafe { vecs.$i.get_unchecked(start..end) },)+)
            }
            fn to_ordered_vecs(vecs: &Self::Vecs, index: usize, capacity: usize, count: usize) -> Self::Vecs {
                if count == 0 {
                    return ($( Vec::<$T>::new(), )+);
                }
                if count == capacity {
                    return ($({
                        let mut v = Vec::with_capacity(capacity);
                        v.extend_from_slice(unsafe { vecs.$i.get_unchecked(index..) });
                        if index > 0 {
                            v.extend_from_slice(unsafe { vecs.$i.get_unchecked(..index) });
                        }
                        v
                    },)+);
                }
                ($( vecs.$i[..count].to_vec(), )+)
            }
            fn num_buffers() -> usize { $n }
        }

        // ── Serialize ─────────────────────────────────────────────────────────

        impl<$($T: SerdeElement),+> Serialize for MultiTypeBuffer<($($T,)+)>
        where $($T::Repr: Serialize),+
        {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                use serde::ser::SerializeStruct;
                let mut s = serializer.serialize_struct("MultiTypeBuffer", 5)?;
                s.serialize_field("index",    &self.index)?;
                s.serialize_field("capacity", &self.capacity)?;
                s.serialize_field("count",    &self.count)?;
                s.serialize_field("prev_idx", &self.prev_idx)?;
                // Convert each Vec<T> → Vec<T::Repr> and serialise as a tuple of sequences.
                let vecs_repr: ($( Vec<$T::Repr>, )+) = (
                    $( self.vecs.$i.iter().map(|v| $T::to_repr(*v)).collect::<Vec<_>>(), )+
                );
                s.serialize_field("vecs", &vecs_repr)?;
                s.end()
            }
        }

        // ── Deserialize ───────────────────────────────────────────────────────

        impl<'de, $($T: SerdeElement),+> Deserialize<'de> for MultiTypeBuffer<($($T,)+)>
        where $($T::Repr: Deserialize<'de>),+
        {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                use serde::de::{MapAccess, Visitor};

                struct MtbVisitor<$($T),+>(std::marker::PhantomData<($($T,)+)>);

                impl<'de, $($T: SerdeElement),+> Visitor<'de> for MtbVisitor<$($T),+>
                where $($T::Repr: Deserialize<'de>),+
                {
                    type Value = MultiTypeBuffer<($($T,)+)>;

                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_str("a MultiTypeBuffer struct")
                    }

                    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                        let mut index:    Option<usize> = None;
                        let mut capacity: Option<usize> = None;
                        let mut count:    Option<usize> = None;
                        let mut prev_idx: Option<usize> = None;
                        let mut vecs: Option<($( Vec<$T::Repr>, )+)> = None;

                        while let Some(key) = map.next_key::<String>()? {
                            match key.as_str() {
                                "index"    => { index    = Some(map.next_value()?); }
                                "capacity" => { capacity = Some(map.next_value()?); }
                                "count"    => { count    = Some(map.next_value()?); }
                                "prev_idx" => { prev_idx = Some(map.next_value()?); }
                                "vecs"     => { vecs     = Some(map.next_value()?); }
                                _          => { let _: serde::de::IgnoredAny = map.next_value()?; }
                            }
                        }

                        let index    = index   .ok_or_else(|| serde::de::Error::missing_field("index"))?;
                        let capacity = capacity.ok_or_else(|| serde::de::Error::missing_field("capacity"))?;
                        let count    = count   .ok_or_else(|| serde::de::Error::missing_field("count"))?;
                        let prev_idx = prev_idx.ok_or_else(|| serde::de::Error::missing_field("prev_idx"))?;
                        let vecs_repr = vecs   .ok_or_else(|| serde::de::Error::missing_field("vecs"))?;

                        let vecs: ($( Vec<$T>, )+) = (
                            $( vecs_repr.$i.into_iter().map($T::from_repr).collect(), )+
                        );

                        Ok(MultiTypeBuffer { vecs, index, capacity, count, prev_idx })
                    }
                }

                const FIELDS: &[&str] = &["index", "capacity", "count", "prev_idx", "vecs"];
                deserializer.deserialize_struct(
                    "MultiTypeBuffer",
                    FIELDS,
                    MtbVisitor::<$($T),+>(std::marker::PhantomData),
                )
            }
        }
    };
}

// Arity 1 – 8
impl_layout!(1; (T0, 0));
impl_layout!(2; (T0, 0), (T1, 1));
impl_layout!(3; (T0, 0), (T1, 1), (T2, 2));
impl_layout!(4; (T0, 0), (T1, 1), (T2, 2), (T3, 3));
impl_layout!(5; (T0, 0), (T1, 1), (T2, 2), (T3, 3), (T4, 4));
impl_layout!(6; (T0, 0), (T1, 1), (T2, 2), (T3, 3), (T4, 4), (T5, 5));
impl_layout!(7; (T0, 0), (T1, 1), (T2, 2), (T3, 3), (T4, 4), (T5, 5), (T6, 6));
impl_layout!(8; (T0, 0), (T1, 1), (T2, 2), (T3, 3), (T4, 4), (T5, 5), (T6, 6), (T7, 7));
