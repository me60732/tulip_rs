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

/// `((p−1)/p, 1/p)` — the Wilders step pair (identical to
/// `tulip_rs::indicators::wilders::multiplier`).
#[inline]
pub fn wilders_multiplier(period: usize) -> (f64, f64) {
    let multiplier = ((period - 1) as f64) / period as f64;
    (multiplier, 1.0 - multiplier)
}
