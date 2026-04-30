#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::cci::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::cci::indicator_by_options;

mod imports {
    pub(crate) use crate::indicators::cci::State;
    pub(crate) use crate::indicators::simd_indicators::{
        md_simd::assets::calc_md_simd, simd_types::F64Constants,
        sma_simd::calc_simd as sma_calc_simd, typprice_simd::calc_simd as typprice_calc_simd,
    };
    pub(crate) use std::simd::Simd;
}

pub mod asset {
    use super::imports::*;
    use crate::ring_buffer::single_buffer::generic_buffer::{
        RingBuffer, SimdBuffer, SimdRingBuffer,
    };

    pub struct SimdState<const N: usize> {
        buffer: SimdBuffer<N>,
        sum: Simd<f64, N>,
    }

    impl<const N: usize> SimdState<N> {
        pub fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            let mut buffer_refs = Vec::with_capacity(N);
            let mut sum = [0.0; N];

            for (i, state) in states.iter_mut().enumerate() {
                buffer_refs.push(&state.buffer);
                sum[i] = state.sum;
            }

            let buffer = SimdBuffer::from_f64_buffers(buffer_refs);

            Self {
                buffer,
                sum: Simd::from_array(sum),
            }
        }

        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let buffers = self.buffer.to_f64_buffers();
            for (i, buffer) in buffers.into_iter().enumerate() {
                states[i].buffer = buffer;
            }

            let sum = self.sum.to_array();

            for (i, state) in states.iter_mut().enumerate() {
                state.sum = sum[i];
            }
        }
        /*#[inline(always)]
        pub fn calc_simd(
            &mut self,
            high: Simd<f64, N>,
            low: Simd<f64, N>,
            close: Simd<f64, N>,
            multiplier: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let typprice = typprice_calc_simd(high, low, close);
            //let (mut mean_deviation, mut sma, mut cci) = (0.0, 0.0, 0.0);

            if let Some(old) = self.buffer.push_with_info(typprice) {
                let sma = sma_calc_simd(&mut self.sum, typprice, old, multiplier);
                let md = calc_md_simd(self.buffer.get_slice(), sma, multiplier);

                let cci = (typprice - sma) / (F64Constants::ZERO15 * md);
                return (cci, sma, md, typprice);
            }

            self.sum += typprice;
            (F64Constants::ZERO, F64Constants::ZERO, F64Constants::ZERO, typprice)
        }*/
        #[inline(always)]
        pub unsafe fn calc_unchecked_simd(
            &mut self,
            high: Simd<f64, N>,
            low: Simd<f64, N>,
            close: Simd<f64, N>,
            multiplier: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let typprice = typprice_calc_simd(high, low, close);

            let old = self.buffer.push_with_info_unchecked(typprice);

            let sma = sma_calc_simd(&mut self.sum, typprice, old, multiplier);
            let md = calc_md_simd(self.buffer.get_slice(), sma, multiplier);

            let cci = (typprice - sma) / (F64Constants::ZERO15 * md);
            (cci, sma, md, typprice)
        }
    }
}

pub(crate) mod options {
    use super::imports::*;
    use crate::indicators::{md::calc_md_simd, typprice::calc as typprice_calc};
    use crate::ring_buffer::unsync_multi_buffer::multi_buffer::{RingBuffer, UnsyncBuffer};

    pub struct SimdState<const N: usize> {
        buffer: UnsyncBuffer<N, f64>,
        sum: Simd<f64, N>,
    }

    impl<const N: usize> SimdState<N> {
        pub fn new(states: &mut [&mut State]) -> Self {
            debug_assert_eq!(states.len(), N, "Number of states must match SIMD width");

            let mut buffer_refs = Vec::with_capacity(N);
            let mut sum = [0.0; N];

            for (i, state) in states.iter_mut().enumerate() {
                buffer_refs.push(&state.buffer);
                sum[i] = state.sum;
            }
            let buffer = UnsyncBuffer::from_buffers(buffer_refs);
            Self {
                buffer,
                sum: Simd::from_array(sum),
            }
        }

        pub fn write_states(&self, states: &mut [&mut State]) {
            // First, handle the buffer updates
            let buffers = self.buffer.to_f64_buffers();
            let sum = self.sum.to_array();

            for (i, (buffer, state)) in buffers.into_iter().zip(states.iter_mut()).enumerate() {
                state.buffer = buffer;
                state.sum = sum[i];
            }
        }

        #[inline(always)]
        pub unsafe fn calc_unchecked_simd(
            &mut self,
            high: f64,
            low: f64,
            close: f64,
            multiplier: Simd<f64, N>,
        ) -> (Simd<f64, N>, Simd<f64, N>, Simd<f64, N>, Simd<f64, N>) {
            let typprice = typprice_calc(&high, &low, &close);
            let typprice = Simd::splat(typprice);
            let old = self.buffer.push_with_info_unchecked(typprice);

            let sma = sma_calc_simd(&mut self.sum, typprice, old, multiplier);
            let mut md = Simd::splat(0.0);
            let sma_ref = sma.as_array();
            let md_ref = md.as_mut_array();
            let slices = self.buffer.raw_slice();
            for (i, &multiplier) in multiplier.as_array().iter().enumerate() {
                md_ref[i] = calc_md_simd::<N>(&slices[i], sma_ref[i], multiplier);
            }

            let cci = (typprice - sma) / (F64Constants::ZERO15 * md);
            (cci, sma, md, typprice)
        }
    }
}
