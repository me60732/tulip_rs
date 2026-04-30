use crate::indicators::fisher::State;
#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::fisher::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::fisher::indicator_by_options;

use crate::indicators::simd_indicators::{
    max_simd::SimdState as MaxSimdState, medprice_simd::calc_simd as calc_medprice_simd,
    min_simd::SimdState as MinSimdState, simd_types::F64Constants,
};

use std::simd::{cmp::SimdPartialOrd, num::SimdFloat, Select, Simd, StdFloat};
//use crate::math_simd::ln;
pub struct FisherConstants<const N: usize>;
impl<const N: usize> FisherConstants<N> {
    pub const PRICE_WEIGHT: Simd<f64, N> = Simd::splat(0.66); // 0.33 * 2.0 - weight for new normalized price
    pub const SMOOTH_WEIGHT: Simd<f64, N> = Simd::splat(0.67); // smoothing factor for exponential average
    pub const MIN_MM: Simd<f64, N> = Simd::splat(0.001);
}
//use crate::ring_buffer::multi_buffer::{mirror_buffer::MirrorBuffer, multi_buffer::MultiBuffer};
pub trait FisherState<const N: usize> {
    fn get_val1(&self) -> Simd<f64, N>;
    fn get_fish(&self) -> Simd<f64, N>;
    fn set_val1(&mut self, value: Simd<f64, N>);
    fn set_fish(&mut self, value: Simd<f64, N>);
}

pub mod assets {
    use super::{
        calc_fisher, calc_medprice_simd, FisherState, MaxSimdState, MinSimdState, Simd, State,
    };
    use crate::ring_buffer::{
        multi_buffer::{
            mirror_buffer::MinMaxBuffer,
            multi_buffer::{MirrorBuffer, MultiBuffer},
        },
        single_buffer::mirror_buffer::MirrorBuffer as SingleMirrorBuffer,
    };
    pub struct SimdState<const N: usize> {
        pub buffer: MultiBuffer<N>,
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
    impl<const N: usize> SimdState<N> {
        pub fn new(states: &mut [&mut State]) -> Self {
            let mut min_refs = Vec::with_capacity(N);
            let mut max_refs = Vec::with_capacity(N);
            let mut buffer_slices = Vec::with_capacity(N);
            let mut val1 = [0.0; N];
            let mut fish = [0.0; N];
            let capacity = states[0].buffer.capacity;
            // Collect references and values
            for (i, state) in states.iter_mut().enumerate() {
                min_refs.push(&mut state.min_state);
                max_refs.push(&mut state.max_state);
                val1[i] = state.val1;
                fish[i] = state.fish;
                buffer_slices.push(state.buffer.get_slice());
            }
            let buffer_refs: [&[f64]; N] =
                buffer_slices.try_into().unwrap_or_else(|v: Vec<&[f64]>| {
                    panic!("Expected {} buffer slices, got {}", N, v.len())
                });

            let buffer = MultiBuffer::from_slice(buffer_refs, capacity);
            let min_state = MinSimdState::new(&mut min_refs);
            let max_state = MaxSimdState::new(&mut max_refs);

            Self {
                buffer,
                min_state,
                max_state,
                val1: Simd::from_array(val1),
                fish: Simd::from_array(fish),
            }
        }
        pub fn write_states(&self, states: &mut [&mut State]) {
            let mut max_refs = Vec::with_capacity(N);
            let mut min_refs = Vec::with_capacity(N);
            let val1 = self.val1.to_array();
            let fish = self.fish.to_array();
            let buffers = self.buffer.to_single_buffers();
            // Collect references and values
            // Use zip to pair states with buffers
            for (i, (state, buffer)) in states.iter_mut().zip(buffers.into_iter()).enumerate() {
                max_refs.push(&mut state.max_state);
                min_refs.push(&mut state.min_state);
                state.val1 = val1[i];
                state.fish = fish[i];
                state.buffer = buffer;
            }

            self.max_state.write_states(&mut max_refs);
            self.min_state.write_states(&mut min_refs);
        }
        #[inline(always)]
        pub fn calc_simd<const CHUNK_SIZE: usize>(
            &mut self,
            high: Simd<f64, N>,
            low: Simd<f64, N>,
            look_back: usize,
        ) -> (Simd<f64, N>, Simd<f64, N>) {
            let medprice = calc_medprice_simd(high, low);

            self.buffer.push(medprice.to_array());

            let (min, _) = self
                .buffer
                .min::<CHUNK_SIZE>(&mut self.min_state, medprice, look_back);
            let (max, _) = self
                .buffer
                .max::<CHUNK_SIZE>(&mut self.max_state, medprice, look_back);
            calc_fisher(self, min, max, medprice)
        }
    }
}

pub mod options {
    use super::{
        calc_fisher, calc_medprice_simd, FisherState, MaxSimdState, MinSimdState, Simd, State,
    };
    use crate::ring_buffer::{
        unsync_multi_buffer::{
            mirror_buffer::MinMaxBuffer,
            multi_buffer::{MirrorBuffer, UnsyncBuffer},
        },
        //single_buffer::mirror_buffer::MirrorBuffer as SingleMirrorBuffer,
    };
    pub struct SimdState<const N: usize> {
        pub buffer: UnsyncBuffer<N, f64>,
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
    impl<const N: usize> SimdState<N> {
        pub fn new(states: &mut [&mut State]) -> Self {
            let mut min_refs = Vec::with_capacity(N);
            let mut max_refs = Vec::with_capacity(N);
            let mut buffer_refs = Vec::with_capacity(N);
            let mut val1 = [0.0; N];
            let mut fish = [0.0; N];

            // Collect references and values
            for (i, state) in states.iter_mut().enumerate() {
                min_refs.push(&mut state.min_state);
                max_refs.push(&mut state.max_state);
                val1[i] = state.val1;
                fish[i] = state.fish;
                buffer_refs.push(&state.buffer);
            }

            let buffer = UnsyncBuffer::from_buffers(buffer_refs);
            let min_state = MinSimdState::new(&mut min_refs);
            let max_state = MaxSimdState::new(&mut max_refs);

            Self {
                buffer,
                min_state,
                max_state,
                val1: Simd::from_array(val1),
                fish: Simd::from_array(fish),
            }
        }
        pub fn write_states(&self, states: &mut [&mut State]) {
            let mut max_refs = Vec::with_capacity(N);
            let mut min_refs = Vec::with_capacity(N);
            let val1 = self.val1.to_array();
            let fish = self.fish.to_array();
            let buffers = self.buffer.to_f64_buffers();
            // Collect references and values
            // Use zip to pair states with buffers
            for (i, (state, buffer)) in states.iter_mut().zip(buffers.into_iter()).enumerate() {
                max_refs.push(&mut state.max_state);
                min_refs.push(&mut state.min_state);
                state.val1 = val1[i];
                state.fish = fish[i];
                state.buffer = buffer;
            }

            self.max_state.write_states(&mut max_refs);
            self.min_state.write_states(&mut min_refs);
        }
        #[inline(always)]
        pub fn calc_simd(
            &mut self,
            high: Simd<f64, N>,
            low: Simd<f64, N>,
            look_back: Simd<usize, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>) {
            let medprice = calc_medprice_simd(high, low);

            self.buffer.push(medprice);

            let (min, _) = self.buffer.min(&mut self.min_state, medprice, look_back);
            let (max, _) = self.buffer.max(&mut self.max_state, medprice, look_back);
            calc_fisher(self, min, max, medprice)
        }
        #[inline(always)]
        pub unsafe fn calc_simd_unchecked(
            &mut self,
            high: Simd<f64, N>,
            low: Simd<f64, N>,
            look_back: Simd<usize, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>) {
            let medprice = calc_medprice_simd(high, low);

            unsafe { self.buffer.push_unchecked(medprice) };

            let (min, _) = self.buffer.min(&mut self.min_state, medprice, look_back);
            let (max, _) = self.buffer.max(&mut self.max_state, medprice, look_back);
            calc_fisher(self, min, max, medprice)
        }
    }
}

use crate::math_simd::ln_unchecked;
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
