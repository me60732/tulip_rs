#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::wilders::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::wilders::indicator_by_options;

use std::simd::{Simd, StdFloat, num::SimdUint};
use serde::{
    de::{self, MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;
use std::marker::PhantomData;
/// Initialises the Wilder's Smoothing SIMD state from raw input slices.
///
/// Computes the simple average of the first `period` values for each lane
/// as the seed for subsequent exponential smoothing.
///
/// # Arguments
///
/// * `inputs` - Per-lane input slices; must each contain at least `period` values.
/// * `period` - Number of bars to average for the initial smoothed value.
///
/// # Returns
///
/// SIMD vector containing the initial Wilder's smoothed value for each lane.
pub fn init_state<'a, const N: usize>(inputs: &[&'a [f64]; N], period: usize) -> Simd<f64, N> {
    let input_ptrs: [*const f64; N] = std::array::from_fn(|i| inputs[i].as_ptr());
    let mut wilders = Simd::splat(0.0);
    for i in 0..period {
        let values = Simd::from_array(std::array::from_fn(|j| unsafe { *input_ptrs[j].add(i) }));
        wilders += values;
    }

    wilders /= Simd::splat(period as f64);

    wilders
}
pub struct SimdState<const N: usize> {
    pub wilders: Simd<f64, N>,
    pub multiplier: Simd<f64, N>,
    pub inv_multiplier: Simd<f64, N>
}
impl<const N: usize> SimdState<N> {
    pub fn new(wilders: Simd<f64, N>, multipliers: (Simd<f64, N>, Simd<f64, N>)) -> Self {
        Self {
            wilders,
            multiplier: multipliers.0,
            inv_multiplier: multipliers.1,
        }
    }
    #[inline(always)]
    pub fn calc_simd(&mut self, real: Simd<f64, N>) -> Simd<f64, N> {
        self.wilders = calc_simd(self.wilders, real, (self.multiplier, self.inv_multiplier));
        self.wilders
    }
    #[inline(always)]
    pub fn partial_calc_simd(
        &mut self,
        real: Simd<f64, N>,
    ) -> Simd<f64, N> {
        self.wilders = partial_calc_simd(self.wilders, real, self.multiplier);
        self.wilders
    }
}
impl<const N: usize> Serialize for SimdState<N>
where
    [f64; N]: Serialize,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("SimdState", 3)?;
        s.serialize_field("ema", &self.wilders.to_array())?;
        s.serialize_field("multiplier", &self.multiplier.to_array())?;
        s.serialize_field("inv_multiplier", &self.inv_multiplier.to_array())?;
        s.end()
    }
}

impl<'de, const N: usize> Deserialize<'de> for SimdState<N>
where
    [f64; N]: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        const FIELDS: &[&str] = &["wilders", "multiplier", "inv_multiplier"];

        enum Field {
            Wilders,
            Multiplier,
            InvMultiplier,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str("`wilders`, `multiplier` or `inv_multiplier`")
                    }

                    fn visit_str<E: de::Error>(self, v: &str) -> Result<Field, E> {
                        match v {
                            "ema" => Ok(Field::Wilders),
                            "multiplier" => Ok(Field::Multiplier),
                            "inv_multiplier" => Ok(Field::InvMultiplier),
                            _ => Err(de::Error::unknown_field(v, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct WildersSimdVisitor<const N: usize>(PhantomData<fn() -> Simd<f64, N>>);

        impl<'de, const N: usize> Visitor<'de> for WildersSimdVisitor<N>
        where
            [f64; N]: Deserialize<'de>,
        {
            type Value = SimdState<N>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct SimdState")
            }

            fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<SimdState<N>, V::Error> {
                let mut wilders: Option<[f64; N]> = None;
                let mut inv_multiplier: Option<[f64; N]> = None;
                let mut multiplier: Option<[f64; N]> = None;

                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::Wilders => {
                            if wilders.is_some() {
                                return Err(de::Error::duplicate_field("ema"));
                            }
                            wilders = Some(map.next_value()?);
                        }
                        Field::Multiplier => {
                            if multiplier.is_some() {
                                return Err(de::Error::duplicate_field("multiplier"));
                            }
                            multiplier = Some(map.next_value()?);
                        }
                        Field::InvMultiplier => {
                            if inv_multiplier.is_some() {
                                return Err(de::Error::duplicate_field("inv_multiplier"));
                            }
                            inv_multiplier = Some(map.next_value()?);
                        }
                    }
                }

                Ok(SimdState {
                    wilders: Simd::from_array(wilders.ok_or_else(|| de::Error::missing_field("wilders"))?),
                    multiplier: Simd::from_array(
                        multiplier.ok_or_else(|| de::Error::missing_field("multiplier"))?,
                    ),
                    inv_multiplier: Simd::from_array(
                        inv_multiplier.ok_or_else(|| de::Error::missing_field("inv_multiplier"))?,
                    ),
                })
            }
        }

        deserializer.deserialize_struct("SimdState", FIELDS, WildersSimdVisitor::<N>(PhantomData))
    }
}
/// Computes one bar of Wilder's Smoothing for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Applies `prev_wilders * multiplier + value * (1 - multiplier)` for each lane.
///
/// # Arguments
///
/// * `prev_wilders` - Previous smoothed values for each lane.
/// * `value` - New input values for this bar.
/// * `multiplier` - Per-lane decay factor `(period - 1) / period`.
///
/// # Returns
///
/// Updated Wilder's smoothed values for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    prev_wilders: Simd<f64, N>,
    value: Simd<f64, N>,
    multipliers: (Simd<f64, N>, Simd<f64, N>),
) -> Simd<f64, N> {
    prev_wilders.mul_add(multipliers.0, value * multipliers.1)
}
/// Computes a partial Wilder's Smoothing step without subtracting the decay residual.
///
/// Applies `prev_wilders * multiplier + value` for each lane, omitting the
/// `(1 - multiplier)` weight on `value`. Used internally for already-scaled inputs.
///
/// # Arguments
///
/// * `prev_wilders` - Previous smoothed values for each lane.
/// * `value` - Pre-scaled new input values for this bar.
/// * `multiplier` - Per-lane decay factor `(period - 1) / period`.
///
/// # Returns
///
/// Partially updated smoothed values for all `N` lanes.
#[inline(always)]
pub fn partial_calc_simd<const N: usize>(
    prev_wilders: Simd<f64, N>,
    value: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> Simd<f64, N> {
    prev_wilders.mul_add(multiplier, value)
}

pub fn multiplier_simd<const N: usize>(periods: [usize; N]) -> (Simd<f64, N>, Simd<f64, N>) {
    let period = Simd::from_array(periods);
    let one = Simd::<f64, N>::splat(1.0);
    let multiplier = (period.cast::<f64>() - one) / period.cast::<f64>();
    (multiplier, one - multiplier)
}