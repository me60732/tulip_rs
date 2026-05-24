#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::by_asset::roc::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::by_option::roc::indicator_by_options;

use crate::indicators::simd_indicators::{
    mom_simd::calc_simd as calc_mom_simd, rocr_simd::calc_simd as calc_rocr_simd,
};
use std::simd::Simd;

/// Computes the Rate of Change (ROC) and Momentum (MOM) for `N` asset lanes simultaneously.
///
/// ROC is defined as `(real - prev_real) / prev_real`, i.e. the Momentum divided by the
/// previous value. Both the ROC ratio and the raw Momentum value are returned so that
/// callers can avoid recomputing Momentum when they need it as an optional output.
///
/// # Returns
///
/// `(roc, mom)` where each is a SIMD vector of `N` lanes:
/// * `roc` — rate-of-change ratio `(real − prev_real) / prev_real` for each lane.
/// * `mom` — raw momentum `real − prev_real` for each lane.
#[inline(always)]
pub fn calc_simd<const N: usize>(
    real: Simd<f64, N>,
    prev_real: Simd<f64, N>,
) -> (Simd<f64, N>, Simd<f64, N>) {
    let mom = calc_mom_simd(real, prev_real);
    (calc_rocr_simd(mom, prev_real), mom)
}
