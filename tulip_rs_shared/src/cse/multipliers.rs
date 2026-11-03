//! Reusable multipliers for the recurrence shapes, lifted from the hand
//! indicator modules (2026-10, recipes-as-data R1) so the leaf's `erase`
//! can bind `Values.mults/invms` without naming indicator modules. Bodies
//! copied VERBATIM — bit-exactness is the contract; the hand modules
//! re-export these as their own `multiplier()` so there is one formula.

/// `(2/(p+1), 1 − 2/(p+1))` — the EMA step pair (identical to
/// `tulip_rs::indicators::ema::multiplier`).
#[inline]
pub fn ema_multiplier(period: usize) -> (f64, f64) {
    let per = 2.0 / (period as f64 + 1.0);
    (per, 1.0 - per)
}

/// kama's multiplier returns (fast_ema, slow_ema) smoothing constants.
pub fn kama_multiplier() -> (f64, f64) {
    let fast = ema_multiplier(2).0;
    let slow = ema_multiplier(30).0;
    (fast, slow)
}

/// `((p−1)/p, 1/p)` — the Wilders step pair (identical to
/// `tulip_rs::indicators::wilders::multiplier`).
#[inline]
pub fn wilders_multiplier(period: usize) -> (f64, f64) {
    let multiplier = ((period - 1) as f64) / period as f64;
    (multiplier, 1.0 - multiplier)
}

/// `(a1, a2)` for the one-pole HighPass recurrence, bit-identical to
/// `tulip_rs::indicators::highpass`'s own derivation:
/// `omega = TAU/period; a1 = (1 - sin(omega)) / cos(omega); a2 = 0.5*(1 + a1)`.
#[inline]
pub fn highpass_multiplier(period: usize) -> (f64, f64) {
    let omega = core::f64::consts::TAU / period as f64;
    let a1 = (1.0 - omega.sin()) / omega.cos();
    let a2 = 0.5 * (1.0 + a1);
    (a1, a2)
}

/// `(a1, a2, b0)` for the two-pole SuperSmoother recurrence, bit-identical to
/// `tulip_rs::indicators::supersmoother`'s own derivation:
/// `omega = PI/period; a1 = 2*exp(-1.414*omega)*cos(1.414*omega);
/// a2 = -exp(-2.828*omega); b0 = (1-a1-a2)*0.5`.
#[inline]
pub fn supersmoother_multiplier(period: usize) -> (f64, f64, f64) {
    let omega = core::f64::consts::PI / period as f64;
    let a1 = 2.0 * (-1.414 * omega).exp() * (1.414 * omega).cos();
    let a2 = -(-2.828 * omega).exp();
    let b0 = (1.0 - a1 - a2) * 0.5;
    (a1, a2, b0)
}
