pub use crate::indicator_types::{TSimdState, TState};
use crate::indicators::instantaneoustrendline::IndicatorState as State;
#[cfg(feature = "simd_assets")]
pub(crate) use crate::indicators::simd_indicators::by_asset::instantaneoustrendline::indicator_by_assets;
use crate::indicators::simd_indicators::homodynediscriminator_simd::SimdState as HdSimdState;
use crate::indicators::simd_indicators::simd_types::F64Constants;
use std::simd::{Simd, StdFloat};

/// SIMD-parallel state for the Ehlers Instantaneous Trendline across `N` assets simultaneously.
///
/// Composes [`HdSimdState`] as the `hd` field — the full four-stage HT cascade and
/// homodyne discriminator — and adds IT-specific SIMD fields on top, exactly mirroring
/// how the scalar [`State`] composes [`homodynediscriminator::State`].
///
/// The gather (`new`) and scatter (`write_states`) methods delegate the HD sub-state
/// to [`HdSimdState::new`] / [`HdSimdState::write_states`] and pack/unpack the
/// IT-specific scalars in a single loop pass.
pub struct SimdState<const N: usize> {
    /// Embedded Homodyne Discriminator SIMD state.
    /// Its `price_buf[0..2]` holds the 3 most-recent raw prices used by the IIR.
    pub hd: HdSimdState<N>,
    /// IT[1] — previous trendline (IIR feedback), one lane per asset.
    pub it_prev: Simd<f64, N>,
    /// IT[2] — two-bar-ago trendline (IIR feedback), one lane per asset.
    pub it_prev2: Simd<f64, N>,
    /// Last computed adaptive α, one lane per asset (for optional output).
    pub alpha: Simd<f64, N>,
}

impl<const N: usize> TSimdState for SimdState<N> {
    type ScalarState = State;
    
    crate::simd_state_impl!(
         sub: [(hd: HdSimdState<N>)],
         scalar: [it_prev, it_prev2, alpha]
    );
}
impl<const N: usize> TState for SimdState<N> {
    type Inputs<'a> = Simd<f64, N>;
    type Outputs = Simd<f64, N>;
    
    #[inline(always)]
    fn calc(&mut self, real: Simd<f64, N>) -> Simd<f64, N> {
        let dc = self.hd.calc(real);

        let alpha = F64Constants::<N>::TWO / (dc + F64Constants::<N>::ONE);
        self.alpha = alpha;
        let a2 = alpha * alpha;
        let beta = F64Constants::<N>::ONE - alpha;

        // 4 FMAs: same chain as the scalar hot path.
        let it = (F64Constants::<N>::TWO * beta).mul_add(
            self.it_prev,
            (-(beta * beta)).mul_add(
                self.it_prev2,
                (alpha - a2 * F64Constants::<N>::QUATER).mul_add(
                    self.hd.price_buf[0],
                    (a2 * F64Constants::<N>::HALF).mul_add(
                        self.hd.price_buf[1],
                        -(alpha - a2 * Simd::splat(0.75_f64)) * self.hd.price_buf[2],
                    ),
                ),
            ),
        );

        self.it_prev2 = self.it_prev;
        self.it_prev = it;
        it
    }
}

