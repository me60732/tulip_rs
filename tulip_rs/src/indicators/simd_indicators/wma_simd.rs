#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::wma::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::wma::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::{simd_indicators::sma_simd::SimdState as SmaSimdState, wma::State};
use serde::{
    de::{self, MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use crate::types::Warm;

use std::fmt;
use std::marker::PhantomData;
use std::simd::Simd;

/// SIMD-parallel state for the Weighted Moving Average (WMA) indicator, holding `N` lanes of per-asset state.
pub struct SimdState<const N: usize> {
    pub(crate) sma_state: SmaSimdState<N>,
    pub(crate) weighted_sum: Simd<f64, N>,
    pub(crate) period: Simd<f64, N>,
    pub(crate) weights: Simd<f64, N>,
}

// ── Serde ─────────────────────────────────────────────────────────────────────
//
// Hand-rolled because `#[derive(Serialize, Deserialize)]` generates a
// `where Simd<f64, N>: Serialize` bound that cannot be satisfied (orphan rules).
// Instead we round-trip through `[f64; N]`, which serde handles natively.

impl<const N: usize> Serialize for SimdState<N>
where
    [f64; N]: Serialize,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("SimdState", 4)?;
        s.serialize_field("sma_state", &self.sma_state)?;
        s.serialize_field("weighted_sum", &self.weighted_sum.to_array())?;
        s.serialize_field("period", &self.period.to_array())?;
        s.serialize_field("weights", &self.weights.to_array())?;
        s.end()
    }
}

impl<'de, const N: usize> Deserialize<'de> for SimdState<N>
where
    [f64; N]: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        const FIELDS: &[&str] = &["sma_state", "weighted_sum", "period", "weights"];

        enum Field {
            SmaState,
            WeightedSum,
            Period,
            Weights,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str("`sma_state`, `weighted_sum`, `period`, or `weights`")
                    }

                    fn visit_str<E: de::Error>(self, v: &str) -> Result<Field, E> {
                        match v {
                            "sma_state" => Ok(Field::SmaState),
                            "weighted_sum" => Ok(Field::WeightedSum),
                            "period" => Ok(Field::Period),
                            "weights" => Ok(Field::Weights),
                            _ => Err(de::Error::unknown_field(v, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct WmaSimdVisitor<const N: usize>(PhantomData<fn() -> Simd<f64, N>>);

        impl<'de, const N: usize> Visitor<'de> for WmaSimdVisitor<N>
        where
            [f64; N]: Deserialize<'de>,
        {
            type Value = SimdState<N>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct SimdState")
            }

            fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<SimdState<N>, V::Error> {
                let mut sma_state: Option<SmaSimdState<N>> = None;
                let mut weighted_sum: Option<[f64; N]> = None;
                let mut period: Option<[f64; N]> = None;
                let mut weights: Option<[f64; N]> = None;

                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::SmaState => {
                            if sma_state.is_some() {
                                return Err(de::Error::duplicate_field("sma_state"));
                            }
                            sma_state = Some(map.next_value()?);
                        }
                        Field::WeightedSum => {
                            if weighted_sum.is_some() {
                                return Err(de::Error::duplicate_field("weighted_sum"));
                            }
                            weighted_sum = Some(map.next_value()?);
                        }
                        Field::Period => {
                            if period.is_some() {
                                return Err(de::Error::duplicate_field("period"));
                            }
                            period = Some(map.next_value()?);
                        }
                        Field::Weights => {
                            if weights.is_some() {
                                return Err(de::Error::duplicate_field("weights"));
                            }
                            weights = Some(map.next_value()?);
                        }
                    }
                }

                Ok(SimdState {
                    sma_state: sma_state.ok_or_else(|| de::Error::missing_field("sma_state"))?,
                    weighted_sum: Simd::from_array(
                        weighted_sum.ok_or_else(|| de::Error::missing_field("weighted_sum"))?,
                    ),
                    period: Simd::from_array(
                        period.ok_or_else(|| de::Error::missing_field("period"))?,
                    ),
                    weights: Simd::from_array(
                        weights.ok_or_else(|| de::Error::missing_field("weights"))?,
                    ),
                })
            }
        }

        deserializer.deserialize_struct("SimdState", FIELDS, WmaSimdVisitor::<N>(PhantomData))
    }
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State<Warm>;
    crate::simd_state_from_state!(
         sub: [(sma_state: SmaSimdState<N>)],
         scalar: [weighted_sum, period, weights]
    );
    crate::simd_state_write!(
         sub: [(sma_state: SmaSimdState<N>)],
         scalar: [weighted_sum]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>);
    type Outputs = (Simd<f64, N>, Simd<f64, N>);

    /// Computes one bar of the Weighted Moving Average (WMA) for `N` assets simultaneously
    /// using SIMD parallelism.
    ///
    /// Slides the rolling weighted sum by one bar and divides by the triangular weight sum.
    ///
    /// # Arguments
    ///
    /// * `prev_value` - Oldest price being dropped from the window.
    /// * `value` - Current prices for this bar.
    /// * `multipliers` - Tuple `(1/period, triangular_weights, period_as_f64)` pre-computed
    ///   constants for SMA and WMA normalisation.
    ///
    /// # Returns
    ///
    /// A tuple `(wma, sma)` for all `N` lanes.
    #[inline(always)]
    fn calc<'a>(&mut self, (value, prev_value): Self::Inputs<'a>) -> (Simd<f64, N>, Simd<f64, N>) {
        self.weighted_sum -= self.sma_state.sum;

        let sma = self.sma_state.calc((value, prev_value));

        self.weighted_sum += value * self.period;

        let wma = self.weighted_sum * self.weights;

        (wma, sma)
    }
}
