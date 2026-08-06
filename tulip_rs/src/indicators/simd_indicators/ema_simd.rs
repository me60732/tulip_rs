#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::ema::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::ema::indicator_by_options;
use serde::{
    de::{self, MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
pub use crate::indicator_types::{TState, TSimdState};
use std::fmt;
use std::marker::PhantomData;
use std::simd::{Simd, StdFloat};
use crate::indicators::ema::State;
use crate::types::Warm;
/// Computes the EMA multiplier pair for `N` lanes with potentially different periods.
///
/// Returns `(per, 1 - per)` where `per = 2.0 / (period + 1.0)` for each lane,
/// suitable for use with [`calc_simd`].
///
/// # Arguments
///
/// * `periods` - Array of per-lane EMA periods.
///
/// # Returns
///
/// A tuple `(multiplier, inv_multiplier)` as SIMD vectors.
#[inline(always)]
pub fn multiplier_simd<const N: usize>(periods: [usize; N]) -> (Simd<f64, N>, Simd<f64, N>) {
    // Convert usize array to f64 array
    let mut f64_periods = [0.0; N];
    for i in 0..N {
        f64_periods[i] = periods[i] as f64;
    }

    // Create SIMD vectors
    let periods_simd = Simd::<f64, N>::from_array(f64_periods);
    let two = Simd::<f64, N>::splat(2.0);
    let one = Simd::<f64, N>::splat(1.0);

    // Calculate: 2.0 / (period + 1.0)
    let per = two / (periods_simd + one);
    (per, one - per)
}

// ── EmaSimd ───────────────────────────────────────────────────────────────────

pub struct SimdState<const N: usize> {
    pub ema: Simd<f64, N>,
    pub inv_multiplier: Simd<f64, N>,
    pub multiplier: Simd<f64, N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_write!(
         sub: [],
         scalar: [ema]
    );
    crate::simd_state_from_state!(
         sub: [],
         scalar: [ema, inv_multiplier, multiplier]
    );
}
impl<const N: usize> SimdState<N> {
    pub fn new(ema: Simd<f64, N>, multipliers: (Simd<f64, N>, Simd<f64, N>)) -> Self {
        Self {
            ema,
            inv_multiplier: multipliers.1,
            multiplier: multipliers.0,
        }
    }
    
    pub fn extract<const S: usize, const L: usize>(&self) -> SimdState<L> {
        let multiplier = self.multiplier.extract::<S, L>();
        let inv_multiplier = self.inv_multiplier.extract::<S, L>();
        let ema = self.ema.extract::<S, L>();

        SimdState::new(ema, (multiplier, inv_multiplier))
    }
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = Simd<f64, N>;
    type Outputs = Simd<f64, N>;

    #[inline(always)]
    fn calc<'a>(&mut self, value: Self::Inputs<'a>) -> Self::Outputs {
        self.ema = calc_simd(value, self.ema, self.multiplier, self.inv_multiplier);
        self.ema
    }
}
// ── Serde ─────────────────────────────────────────────────────────────────────
//
// Hand-rolled because `#[derive(Serialize, Deserialize)]` generates a
// `where Simd<f64, N>: Serialize` bound that cannot be satisfied (orphan rules).
// Instead we round-trip through `[f64; N]`, which serde handles natively.
//
// Serialize  — call `.to_array()` on each Simd field, emit as [f64; N].
// Deserialize — read each field as [f64; N], reconstruct with `Simd::from_array`.

impl<const N: usize> Serialize for SimdState<N>
where
    [f64; N]: Serialize,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("SimdState", 3)?;
        s.serialize_field("ema", &self.ema.to_array())?;
        s.serialize_field("inv_multiplier", &self.inv_multiplier.to_array())?;
        s.serialize_field("multiplier", &self.multiplier.to_array())?;
        s.end()
    }
}

impl<'de, const N: usize> Deserialize<'de> for SimdState<N>
where
    [f64; N]: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        const FIELDS: &[&str] = &["ema", "inv_multiplier", "multiplier"];

        enum Field {
            Ema,
            InvMultiplier,
            Multiplier,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str("`ema`, `inv_multiplier`, or `multiplier`")
                    }

                    fn visit_str<E: de::Error>(self, v: &str) -> Result<Field, E> {
                        match v {
                            "ema" => Ok(Field::Ema),
                            "inv_multiplier" => Ok(Field::InvMultiplier),
                            "multiplier" => Ok(Field::Multiplier),
                            _ => Err(de::Error::unknown_field(v, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct EmaSimdVisitor<const N: usize>(PhantomData<fn() -> Simd<f64, N>>);

        impl<'de, const N: usize> Visitor<'de> for EmaSimdVisitor<N>
        where
            [f64; N]: Deserialize<'de>,
        {
            type Value = SimdState<N>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct SimdState")
            }

            fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<SimdState<N>, V::Error> {
                let mut ema: Option<[f64; N]> = None;
                let mut inv_multiplier: Option<[f64; N]> = None;
                let mut multiplier: Option<[f64; N]> = None;

                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::Ema => {
                            if ema.is_some() {
                                return Err(de::Error::duplicate_field("ema"));
                            }
                            ema = Some(map.next_value()?);
                        }
                        Field::InvMultiplier => {
                            if inv_multiplier.is_some() {
                                return Err(de::Error::duplicate_field("inv_multiplier"));
                            }
                            inv_multiplier = Some(map.next_value()?);
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
                    ema: Simd::from_array(ema.ok_or_else(|| de::Error::missing_field("ema"))?),
                    inv_multiplier: Simd::from_array(
                        inv_multiplier.ok_or_else(|| de::Error::missing_field("inv_multiplier"))?,
                    ),
                    multiplier: Simd::from_array(
                        multiplier.ok_or_else(|| de::Error::missing_field("multiplier"))?,
                    ),
                })
            }
        }

        deserializer.deserialize_struct("SimdState", FIELDS, EmaSimdVisitor::<N>(PhantomData))
    }
}

// ── Public free functions ─────────────────────────────────────────────────────

/// Computes one bar of the Exponential Moving Average (EMA) for `N` assets simultaneously
/// using SIMD parallelism.
///
/// Applies the standard EMA formula: `prev_ema * inv_multiplier + value * multiplier`.
///
/// # Arguments
///
/// * `value` - Current prices for this bar.
/// * `prev_ema` - Previous EMA values for each lane.
/// * `multipliers` - Tuple `(multiplier, inv_multiplier)` from [`multiplier_simd`].
///
/// # Returns
///
/// Updated EMA values for all `N` lanes.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    value: Simd<f64, N>,
    prev_ema: Simd<f64, N>,
    multiplier: Simd<f64, N>, 
    inv_multiplier: Simd<f64, N>,
) -> Simd<f64, N> {
    prev_ema.mul_add(inv_multiplier, value * multiplier)
}
