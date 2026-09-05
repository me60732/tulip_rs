#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::sma::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub(crate) use crate::indicators::simd_indicators::by_option::sma::indicator_by_options;
use serde::{
    de::{self, MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use crate::types::Warm;
pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::sma::State;
use std::fmt;
use std::marker::PhantomData;
use std::simd::Simd;

pub struct SimdState<const N: usize> {
    pub sum: Simd<f64, N>,
    pub multiplier: Simd<f64, N>,
}
impl<const N: usize> SimdState<N> {
    pub fn new(sum: Simd<f64, N>, multiplier: Simd<f64, N>) -> Self {
        Self {
            sum,
            multiplier,
        }
    }
}
impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_from_state!(
         sub: [],
         scalar: [sum, multiplier]
    );
    crate::simd_state_write!(
         sub: [],
         scalar: [sum]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = Simd<f64, N>;
    #[inline(always)]
    fn calc<'a>(&mut self, (value, prev_value): Self::Inputs<'a>) -> Simd<f64, N> {
        self.sum += value - prev_value;
        self.sum * self.multiplier
    }
}
    
impl<const N: usize> Serialize for SimdState<N>
where
    [f64; N]: Serialize,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("SimdState", 2)?;
        s.serialize_field("sum", &self.sum.to_array())?;
        s.serialize_field("multiplier", &self.multiplier.to_array())?;
        s.end()
    }
}

impl<'de, const N: usize> Deserialize<'de> for SimdState<N>
where
    [f64; N]: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        const FIELDS: &[&str] = &["sum", "multiplier"];

        enum Field {
            Sum,
            Multiplier,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str("`sum` or `multiplier`")
                    }

                    fn visit_str<E: de::Error>(self, v: &str) -> Result<Field, E> {
                        match v {
                            "sum" => Ok(Field::Sum),
                            "multiplier" => Ok(Field::Multiplier),
                            _ => Err(de::Error::unknown_field(v, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct SmaSimdVisitor<const N: usize>(PhantomData<fn() -> Simd<f64, N>>);

        impl<'de, const N: usize> Visitor<'de> for SmaSimdVisitor<N>
        where
            [f64; N]: Deserialize<'de>,
        {
            type Value = SimdState<N>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct SimdState")
            }

            fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<SimdState<N>, V::Error> {
                let mut sum: Option<[f64; N]> = None;
                let mut multiplier: Option<[f64; N]> = None;

                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::Sum => {
                            if sum.is_some() {
                                return Err(de::Error::duplicate_field("sum"));
                            }
                            sum = Some(map.next_value()?);
                        }
                        Field::Multiplier => {
                            if multiplier.is_some() {
                                return Err(de::Error::duplicate_field("multiplier"));
                            }
                            multiplier = Some(map.next_value()?);
                        }
                    }
                }

                Ok(SimdState {
                    sum: Simd::from_array(sum.ok_or_else(|| de::Error::missing_field("sum"))?),
                    multiplier: Simd::from_array(
                        multiplier.ok_or_else(|| de::Error::missing_field("multiplier"))?,
                    ),
                })
            }
        }

        deserializer.deserialize_struct("SimdState", FIELDS, SmaSimdVisitor::<N>(PhantomData))
    }
}
/// Advances one bar of the Simple Moving Average (SMA) for `N` asset lanes simultaneously.
///
/// The SMA is maintained as a running sum. Each step adds the new value and removes the value
/// that is dropping off the window, then multiplies by `1 / period` to get the average.
///
/// # Arguments
///
/// * `sum`       — Mutable reference to the SIMD vector holding the running window sum for each lane.
/// * `value`     — The incoming bar value for each lane.
/// * `prev_value`— The value leaving the window (i.e. `real[i - period]`) for each lane.
/// * `multiplier`— `1.0 / period` broadcast to all lanes.
///
/// # Returns
///
/// The SMA for the current bar across all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    sum: &mut Simd<f64, N>,
    value: Simd<f64, N>,
    prev_value: Simd<f64, N>,
    multiplier: Simd<f64, N>,
) -> Simd<f64, N> {
    *sum += value - prev_value;
    *sum * multiplier
}
