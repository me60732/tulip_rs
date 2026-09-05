#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::stddev::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::stddev::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::simd_indicators::{
    simd_types::F64Constants, sma_simd::SimdState as SmaSimdState,
};
pub use crate::indicators::stddev::{multiplier, State};
use crate::types::Warm;
use serde::{
    de::{self, MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::simd::{num::SimdFloat, Simd, StdFloat};

/// SIMD-parallel state for computing the Standard Deviation indicator across `N` assets simultaneously.
/// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
pub struct SimdState<const N: usize> {
    pub sma_state: SmaSimdState<N>,
    pub sum_sq: Simd<f64, N>,
}
impl<const N: usize> Deref for SimdState<N> {
    type Target = SmaSimdState<N>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.sma_state
    }
}
impl<const N: usize> DerefMut for SimdState<N> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sma_state
    }
}
impl<const N: usize> SimdState<N> {
    pub fn new(sum: Simd<f64, N>, sum_sq: Simd<f64, N>, multiplier: Simd<f64, N>) -> Self {
        Self {
            sma_state: SmaSimdState::new(sum, multiplier),
            sum_sq,
        }
    }
    pub fn init_state<'a>(inputs: &[&'a [f64]; N], period: usize) -> Self {
        let multiplier_val = multiplier(period);
        let mut sums = Simd::splat(0.0);
        let mut sums_sq = Simd::splat(0.0);
        // Optimization: Pre-compute input pointers for the initialization loop
        let input_ptrs: [*const f64; N] = std::array::from_fn(|i| inputs[i].as_ptr());

        for i in 0..period {
            let values =
                Simd::from_array(std::array::from_fn(|j| unsafe { *input_ptrs[j].add(i) }));
            sums += values;
            sums_sq += values * values;
        }
        Self {
            sma_state: SmaSimdState::new(sums, Simd::splat(multiplier_val)),
            sum_sq: sums_sq,
        }
    }
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_impl!(
         sub: [(sma_state: SmaSimdState<N>)],
         scalar: [sum_sq]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>);

    #[inline(always)]
    fn calc<'a>(&mut self, (value, prev_value): Self::Inputs<'a>) -> Self::Outputs {
        let sma = self.sma_state.calc((value, prev_value));

        self.sum_sq += value.mul_add(value, -(prev_value * prev_value));
        //let mut sd = (state.sum_sq * multiplier) - (sma * sma);
        let mut sd = self.sum_sq.mul_add(self.multiplier, -(sma * sma));
        sd = sd.sqrt().simd_max(F64Constants::<N>::EPSILON);

        (sd, sma)
    }
}
// ── Serde ─────────────────────────────────────────────────────────────────────
//
// Hand-rolled because `#[derive(Serialize, Deserialize)]` generates a
// `where Simd<f64, N>: Serialize` bound that cannot be satisfied (orphan rules).
// Instead we round-trip through `[f64; N]`, which serde handles natively.
//
// Wire format is the flat three-field struct { sum, sum_sq, multiplier }, which
// delegates to SmaSimdState (sum + multiplier) and the direct sum_sq field.

impl<const N: usize> Serialize for SimdState<N>
where
    [f64; N]: Serialize,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // self.sum and self.multiplier are accessible via Deref to sma_state
        let mut s = serializer.serialize_struct("SimdState", 3)?;
        s.serialize_field("sum", &self.sum.to_array())?;
        s.serialize_field("sum_sq", &self.sum_sq.to_array())?;
        s.serialize_field("multiplier", &self.multiplier.to_array())?;
        s.end()
    }
}

impl<'de, const N: usize> Deserialize<'de> for SimdState<N>
where
    [f64; N]: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        const FIELDS: &[&str] = &["sum", "sum_sq", "multiplier"];

        enum Field {
            Sum,
            SumSq,
            Multiplier,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str("`sum`, `sum_sq`, or `multiplier`")
                    }

                    fn visit_str<E: de::Error>(self, v: &str) -> Result<Field, E> {
                        match v {
                            "sum" => Ok(Field::Sum),
                            "sum_sq" => Ok(Field::SumSq),
                            "multiplier" => Ok(Field::Multiplier),
                            _ => Err(de::Error::unknown_field(v, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct StddevSimdVisitor<const N: usize>(PhantomData<fn() -> Simd<f64, N>>);

        impl<'de, const N: usize> Visitor<'de> for StddevSimdVisitor<N>
        where
            [f64; N]: Deserialize<'de>,
        {
            type Value = SimdState<N>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct SimdState")
            }

            fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<SimdState<N>, V::Error> {
                let mut sum: Option<[f64; N]> = None;
                let mut sum_sq: Option<[f64; N]> = None;
                let mut multiplier: Option<[f64; N]> = None;

                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::Sum => {
                            if sum.is_some() {
                                return Err(de::Error::duplicate_field("sum"));
                            }
                            sum = Some(map.next_value()?);
                        }
                        Field::SumSq => {
                            if sum_sq.is_some() {
                                return Err(de::Error::duplicate_field("sum_sq"));
                            }
                            sum_sq = Some(map.next_value()?);
                        }
                        Field::Multiplier => {
                            if multiplier.is_some() {
                                return Err(de::Error::duplicate_field("multiplier"));
                            }
                            multiplier = Some(map.next_value()?);
                        }
                    }
                }

                Ok(SimdState::new(
                    Simd::from_array(sum.ok_or_else(|| de::Error::missing_field("sum"))?),
                    Simd::from_array(sum_sq.ok_or_else(|| de::Error::missing_field("sum_sq"))?),
                    Simd::from_array(
                        multiplier.ok_or_else(|| de::Error::missing_field("multiplier"))?,
                    ),
                ))
            }
        }

        deserializer.deserialize_struct("SimdState", FIELDS, StddevSimdVisitor::<N>(PhantomData))
    }
}
