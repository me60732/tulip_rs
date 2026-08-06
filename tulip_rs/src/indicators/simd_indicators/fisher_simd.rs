use crate::indicators::fisher::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::fisher::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::fisher::indicator_by_options;

pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::simd_indicators::{
    medprice_simd::calc_simd as calc_medprice_simd, simd_types::F64Constants,
};
use crate::types::Warm;
use std::simd::{cmp::SimdPartialOrd, num::SimdFloat, Select, Simd, StdFloat};
//use crate::math_simd::ln;
/// Compile-time constants for the Fisher Transform computation.
pub struct FisherConstants<const N: usize>;
impl<const N: usize> FisherConstants<N> {
    /// Weight applied to the newly normalised price (0.33 × 2).
    pub const PRICE_WEIGHT: Simd<f64, N> = Simd::splat(0.66); // 0.33 * 2.0 - weight for new normalized price
    /// Smoothing factor applied to the running `val1` exponential average.
    pub const SMOOTH_WEIGHT: Simd<f64, N> = Simd::splat(0.67); // smoothing factor for exponential average
    /// Minimum allowed max-minus-min range to prevent division by zero.
    pub const MIN_MM: Simd<f64, N> = Simd::splat(0.001);
}
//use crate::ring_buffer::multi_buffer::{mirror_buffer::MirrorBuffer, multi_buffer::MultiBuffer};
/// Trait abstracting over the two `SimdState` variants (`assets` and `options`) so that
/// the core Fisher Transform formula in [`calc_fisher`] can operate on either.
pub trait FisherState<const N: usize> {
    fn get_val1(&self) -> Simd<f64, N>;
    fn get_fish(&self) -> Simd<f64, N>;
    fn set_val1(&mut self, value: Simd<f64, N>);
    fn set_fish(&mut self, value: Simd<f64, N>);
}

/// SIMD state variants for the by-asset and by-option execution paths.
pub mod assets {
    use super::{
        calc_fisher, calc_medprice_simd, FisherState, Simd, State, TSimdState, TState, Warm,
    };
    use crate::indicators::simd_indicators::{
        max_simd::assets::SimdState as MaxSimdState,
        min_simd::assets::SimdState as MinSimdState,
    };
    use crate::ring_buffer::multi_buffer::multi_mirror_buffer::MultiMirrorBuffer;
    /// SIMD-parallel state for computing the Fisher Transform across `N` assets simultaneously.
    /// Each field is a SIMD vector where lane `i` corresponds to asset `i`.
    pub struct SimdState<const N: usize> {
        pub buffer: MultiMirrorBuffer<N, f64, Warm>,
        pub min_state: MinSimdState<N>,
        pub max_state: MaxSimdState<N>,
        pub val1: Simd<f64, N>,
        pub fish: Simd<f64, N>,
    }
    impl<const N: usize> FisherState<N> for SimdState<N> {
        fn get_val1(&self) -> Simd<f64, N> {
            self.val1
        }
        fn get_fish(&self) -> Simd<f64, N> {
            self.fish
        }
        fn set_val1(&mut self, value: Simd<f64, N>) {
            self.val1 = value;
        }
        fn set_fish(&mut self, value: Simd<f64, N>) {
            self.fish = value;
        }
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;

        crate::simd_state_impl!(
             sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
             scalar: [val1, fish],
             buf: [(buffer: MultiMirrorBuffer<N, f64, Warm>, from_mirror_buffers)]
        );
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, usize);
        type Outputs = (Simd<f64, N>, Simd<f64, N>);

        #[inline(always)]
        fn calc(
            &mut self,
            (high, low, look_back): Self::Inputs<'_>,
        ) -> (Simd<f64, N>, Simd<f64, N>) {
            let medprice = calc_medprice_simd(high, low);

            self.buffer.push(medprice.to_array());

            let (min, _) = self.buffer.min(&mut self.min_state, medprice, look_back);
            let (max, _) = self.buffer.max(&mut self.max_state, medprice, look_back);
            calc_fisher(self, min, max, medprice)
        }
    }
}

pub mod options {
    use super::{
        calc_fisher, calc_medprice_simd, FisherState, Simd, State, TSimdState, TState, Warm,
    };
    use crate::indicators::simd_indicators::{
        max_simd::options::SimdState as MaxSimdState,
        min_simd::options::SimdState as MinSimdState,
    };
    use crate::ring_buffer::unsync_multi_buffer::unsync_mirror_buffer::UnsyncMirrorBuffer;
    /// SIMD-parallel state for computing the Fisher Transform across `N` option lanes simultaneously.
    /// Each field is a SIMD vector where lane `i` corresponds to option set `i`.
    pub struct SimdState<const N: usize> {
        pub buffer: UnsyncMirrorBuffer<N, f64, Warm>,
        pub min_state: MinSimdState<N>,
        pub max_state: MaxSimdState<N>,
        pub val1: Simd<f64, N>,
        pub fish: Simd<f64, N>,
    }
    impl<const N: usize> FisherState<N> for SimdState<N> {
        fn get_val1(&self) -> Simd<f64, N> {
            self.val1
        }
        fn get_fish(&self) -> Simd<f64, N> {
            self.fish
        }
        fn set_val1(&mut self, value: Simd<f64, N>) {
            self.val1 = value;
        }
        fn set_fish(&mut self, value: Simd<f64, N>) {
            self.fish = value;
        }
    }
    impl<const N: usize> TSimdState for SimdState<N> {
        type ScalarState = State<Warm>;

        crate::simd_state_impl!(
            sub: [(min_state: MinSimdState<N>), (max_state: MaxSimdState<N>)],
             scalar: [val1, fish],
             buf: [(buffer: UnsyncMirrorBuffer<N, f64, Warm>, from_mirror_buffers)]
        );
    }
    impl<const N: usize> TState for SimdState<N> {
        type Inputs<'a> = (Simd<f64, N>, Simd<f64, N>, Simd<usize, N>);
        type Outputs = (Simd<f64, N>, Simd<f64, N>);

        #[inline(always)]
        fn calc<'a>(&mut self, (high, low, look_back): Self::Inputs<'a>) -> Self::Outputs {
            let medprice = calc_medprice_simd(high, low);

            self.buffer.push(medprice);

            let (min, _) = self.buffer.min(&mut self.min_state, medprice, look_back);
            let (max, _) = self.buffer.max(&mut self.max_state, medprice, look_back);
            calc_fisher(self, min, max, medprice)
        }
    }
}

use crate::math_simd::ln_unchecked;
/// Core Fisher Transform computation shared by both the `assets` and `options` SIMD states.
///
/// Given the current rolling `min` and `max` over the lookback window and the current
/// `medprice`, updates `val1` — a smoothed, clamped normalisation of the price within
/// the min/max range — then applies the Fisher formula:
/// `fish = 0.5 * (ln((1 + val1)/(1 - val1)) + prev_fish)`.
///
/// Returns `(fish, signal)` where `signal` is the previous bar's `fish` value.
#[inline(always)]
fn calc_fisher<const N: usize, T: FisherState<N>>(
    state: &mut T,
    min: Simd<f64, N>,
    max: Simd<f64, N>,
    medprice: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>) {
    let mut val1 = state.get_val1();
    let mm = (max - min).simd_max(FisherConstants::<N>::MIN_MM);

    val1 = FisherConstants::<N>::PRICE_WEIGHT.mul_add(
        (medprice - min) / mm - F64Constants::HALF,
        FisherConstants::<N>::SMOOTH_WEIGHT * val1,
    );

    val1 = val1.simd_gt(Simd::splat(0.99)).select(
        Simd::splat(0.999),
        val1.simd_lt(Simd::splat(-0.99))
            .select(Simd::splat(-0.999), val1),
    );
    state.set_val1(val1);

    let signal = state.get_fish();

    let ln_arg = (F64Constants::ONE + val1) / (F64Constants::ONE - val1);
    let fish = F64Constants::HALF * (unsafe { ln_unchecked(ln_arg) } + signal);
    state.set_fish(fish);
    (fish, signal)
}
