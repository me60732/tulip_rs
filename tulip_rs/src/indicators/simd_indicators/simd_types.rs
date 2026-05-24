use std::simd::Simd;
/// Pre-splat SIMD constants for `f64` values, avoiding repeated `Simd::splat` calls in hot loops.
pub struct F64Constants<const N: usize>;
impl<const N: usize> F64Constants<N> {
    /// Positive infinity. (Note: intentional historical spelling retained for compatibility.)
    pub const INIFITY: Simd<f64, N> = Simd::from_array([f64::INFINITY; N]);
    /// Maximum finite `f64` value.
    pub const MAX: Simd<f64, N> = Simd::splat(f64::MAX);
    /// Machine epsilon for `f64`.
    pub const EPSILON: Simd<f64, N> = Simd::splat(f64::EPSILON);
    /// 0.25
    pub const QUATER: Simd<f64, N> = Simd::splat(0.25);
    /// 0.5
    pub const HALF: Simd<f64, N> = Simd::splat(0.5);
    /// 1/3
    pub const THIRD: Simd<f64, N> = Simd::splat(1.0 / 3.0);
    /// 100.0
    pub const HUNDRED: Simd<f64, N> = Simd::splat(100.0);
    /// 0.0
    pub const ZERO: Simd<f64, N> = Simd::splat(0.0);
    /// 1.0
    pub const ONE: Simd<f64, N> = Simd::splat(1.0);
    /// -1.0
    pub const NEG_ONE: Simd<f64, N> = Simd::splat(-1.0);
    /// 2.0
    pub const TWO: Simd<f64, N> = Simd::splat(2.0);
    /// 3.0
    pub const THREE: Simd<f64, N> = Simd::splat(3.0);
    /// 4.0
    pub const FOUR: Simd<f64, N> = Simd::splat(4.0);
    /// 10000.0
    pub const TEN_THOUSAND: Simd<f64, N> = Simd::splat(10000.0);
    /// 0.015
    pub const ZERO15: Simd<f64, N> = Simd::splat(0.015);
    /// Square root of 252 (annualisation factor for daily volatility).
    pub const ANNUAL: Simd<f64, N> = Simd::splat(15.874507866387544); //252_f64.sqrt()
}

/// Pre-splat SIMD constants for `usize` values.
pub struct UsizeConstants<const N: usize>;
impl<const N: usize> UsizeConstants<N> {
    /// 0
    pub const ZERO: Simd<usize, N> = Simd::splat(0);
    /// 1
    pub const ONE: Simd<usize, N> = Simd::splat(1);
}
