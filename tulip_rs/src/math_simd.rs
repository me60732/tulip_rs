//use crate::indicators::nightly::simd_types::F64Constants;
use std::simd::{
    cmp::{SimdPartialEq, SimdPartialOrd},
    num::{SimdFloat, SimdInt, SimdUint},
    Select, Simd, StdFloat,
};
/// High-performance SIMD natural logarithm
/// Based on Agner Fog's implementation
/// High-performance SIMD natural logarithm
/// Based on the wide crate implementation
/// High-performance SIMD natural logarithm
/// Based on fdlibm implementation
#[inline(always)]
pub fn ln<const N: usize>(x: Simd<f64, N>) -> Simd<f64, N> {
    // Pre-computed constants to reduce Simd::splat() calls
    const L2U: f64 = 0.6931471805599452862;
    const L2L: f64 = 2.3190468138462996e-17;
    const SQRT2: f64 = 1.4142135623730950488;
    const TWO54: f64 = 18014398509481984.0;
    const ZERO: f64 = 0.0;
    const ONE: f64 = 1.0;
    const HALF: f64 = 0.5;
    const TWO: f64 = 2.0;
    const NEG_INF: f64 = f64::NEG_INFINITY;
    const POS_INF: f64 = f64::INFINITY;
    const NAN: f64 = f64::NAN;
    
    // Full 8-term polynomial coefficients for accuracy
    const C1: f64 = 1.0;
    const C3: f64 = 0.3333333333333333333333333333333;
    const C5: f64 = 0.2;
    const C7: f64 = 0.14285714285714285714285714285714;
    const C9: f64 = 0.11111111111111111111111111111111;
    const C11: f64 = 0.09090909090909090909090909090909;
    const C13: f64 = 0.07692307692307692307692307692308;
    const C15: f64 = 0.06666666666666666666666666666667;
    
    // Pre-splat constants
    let zero_splat = Simd::splat(ZERO);
    let one_splat = Simd::splat(ONE);
    let half_splat = Simd::splat(HALF);
    let two_splat = Simd::splat(TWO);
    let sqrt2_splat = Simd::splat(SQRT2);
    let two54_splat = Simd::splat(TWO54);
    let pos_inf_splat = Simd::<f64, N>::splat(POS_INF);
    let neg_inf_splat = Simd::<f64, N>::splat(NEG_INF);
    
    // OPTIMIZATION 1: Reduced Special Case Checks (kept from previous)
    let is_zero = x.simd_eq(zero_splat);
    let is_neg = x.simd_lt(zero_splat);
    let is_inf = x.simd_eq(pos_inf_splat);
    let is_nan = x.is_nan();
    
    let special_mask = is_zero | is_neg | is_inf | is_nan;
    
    if special_mask.all() {
        return is_zero.select(
            neg_inf_splat,
            is_neg.select(
                Simd::splat(NAN),
                is_inf.select(
                    pos_inf_splat,
                    Simd::splat(NAN)
                )
            )
        );
    }
    
    // OPTIMIZATION 2: Combined Bit Operations
    // Extract bits once and do all bit operations together
    let ix = x.to_bits().cast::<i64>();
    let is_subnormal = ix.simd_lt(Simd::splat(0x0010000000000000_i64));
    
    // Combined subnormal handling and bit extraction
    let (x_normalized, exponent_adjustment) = if is_subnormal.any() {
        let x_scaled = x * two54_splat;
        let x_work = is_subnormal.select(x_scaled, x);
        let adj = is_subnormal.select(Simd::splat(-54_i32), Simd::splat(0_i32));
        (x_work, adj)
    } else {
        // Fast path when no subnormals
        (x, Simd::splat(0_i32))
    };
    
    // Single bit extraction for both exponent and mantissa
    let ix_work = x_normalized.to_bits().cast::<i64>();
    
    // Extract exponent and create mantissa in one step
    let raw_exponent = (ix_work >> 52).cast::<i32>();
    let e = exponent_adjustment + raw_exponent - Simd::splat(0x3ff_i32);
    
    // Create mantissa directly from the same bits
    let mantissa_bits = (ix_work & Simd::splat(0x000fffffffffffff_i64)) | Simd::splat(0x3ff0000000000000_i64);
    let mantissa = Simd::<f64, N>::from_bits(mantissa_bits.cast::<u64>());
    
    // Range reduction (kept the same)
    let adjust = mantissa.simd_ge(sqrt2_splat);
    let mantissa = adjust.select(mantissa * half_splat, mantissa);
    let e = adjust.select(e + Simd::splat(1_i32), e);
    
    // Optimized transformation s = (m-1)/(m+1)
    let numerator = mantissa - one_splat;
    let denominator = mantissa + one_splat;
    let s = numerator / denominator;
    let s2 = s * s;
    
    // Full 8-term polynomial evaluation for accuracy
    let mut poly = Simd::splat(C15);
    poly = s2.mul_add(poly, Simd::splat(C13));
    poly = s2.mul_add(poly, Simd::splat(C11));
    poly = s2.mul_add(poly, Simd::splat(C9));
    poly = s2.mul_add(poly, Simd::splat(C7));
    poly = s2.mul_add(poly, Simd::splat(C5));
    poly = s2.mul_add(poly, Simd::splat(C3));
    poly = s2.mul_add(poly, Simd::splat(C1));
    
    // Final result computation with optimized mul_add chain
    let log_mantissa = s * poly * two_splat;
    let e_f64 = e.cast::<f64>();
    
    // Single mul_add chain for final result
    let result = e_f64.mul_add(
        Simd::splat(L2U), 
        e_f64.mul_add(Simd::splat(L2L), log_mantissa)
    );
    
    // Apply special case mask only if there are any special cases
    if special_mask.any() {
        special_mask.select(
            is_zero.select(
                neg_inf_splat,
                is_neg.select(
                    Simd::splat(NAN),
                    is_inf.select(
                        pos_inf_splat,
                        Simd::splat(NAN)
                    )
                )
            ),
            result
        )
    } else {
        result
    }
}

/// Unsafe natural logarithm that assumes inputs are positive, finite, and non-NaN.
/// 
/// # Safety
/// 
/// The caller must guarantee that all input values satisfy:
/// - x > 0.0 (no negative values or zero)
/// - x is finite (no positive infinity)
/// - x is not NaN
/// 
/// Violating these preconditions may result in incorrect results or undefined behavior.
/// 
/// # Performance
/// 
/// This function skips all special case checks and branching, providing maximum
/// performance for hot paths where input constraints are guaranteed by the caller.
#[inline(always)]
pub unsafe fn ln_unchecked<const N: usize>(x: Simd<f64, N>) -> Simd<f64, N> {
    // Pre-computed constants
    const L2U: f64 = 0.6931471805599452862;
    const L2L: f64 = 2.3190468138462996e-17;
    const SQRT2: f64 = 1.4142135623730950488;
    const TWO54: f64 = 18014398509481984.0;
    const ONE: f64 = 1.0;
    const HALF: f64 = 0.5;
    const TWO: f64 = 2.0;
    
    // Full 8-term polynomial coefficients for accuracy
    const C1: f64 = 1.0;
    const C3: f64 = 0.3333333333333333333333333333333;
    const C5: f64 = 0.2;
    const C7: f64 = 0.14285714285714285714285714285714;
    const C9: f64 = 0.11111111111111111111111111111111;
    const C11: f64 = 0.09090909090909090909090909090909;
    const C13: f64 = 0.07692307692307692307692307692308;
    const C15: f64 = 0.06666666666666666666666666666667;
    
    // Pre-splat constants
    let one_splat = Simd::splat(ONE);
    let half_splat = Simd::splat(HALF);
    let two_splat = Simd::splat(TWO);
    let sqrt2_splat = Simd::splat(SQRT2);
    let two54_splat = Simd::splat(TWO54);
    
    // Debug assertion in debug builds to catch violations
    #[cfg(debug_assertions)]
    {
        debug_assert!(
            !x.simd_le(Simd::splat(0.0)).any(),
            "ln_unchecked called with non-positive value"
        );
        debug_assert!(
            !x.is_nan().any(),
            "ln_unchecked called with NaN"
        );
        debug_assert!(
            x.is_finite().all(),
            "ln_unchecked called with infinite value"
        );
    }
    
    // Combined bit operations for subnormal handling
    let ix = x.to_bits().cast::<i64>();
    let is_subnormal = ix.simd_lt(Simd::splat(0x0010000000000000_i64));
    
    // Handle subnormals by scaling
    let (x_normalized, exponent_adjustment) = if is_subnormal.any() {
        let x_scaled = x * two54_splat;
        let x_work = is_subnormal.select(x_scaled, x);
        let adj = is_subnormal.select(Simd::splat(-54_i32), Simd::splat(0_i32));
        (x_work, adj)
    } else {
        // Fast path when no subnormals
        (x, Simd::splat(0_i32))
    };
    
    // Single bit extraction for both exponent and mantissa
    let ix_work = x_normalized.to_bits().cast::<i64>();
    
    // Extract exponent and create mantissa in one step
    let raw_exponent = (ix_work >> 52).cast::<i32>();
    let e = exponent_adjustment + raw_exponent - Simd::splat(0x3ff_i32);
    
    // Create mantissa directly from the same bits
    let mantissa_bits = (ix_work & Simd::splat(0x000fffffffffffff_i64)) | Simd::splat(0x3ff0000000000000_i64);
    let mantissa = Simd::<f64, N>::from_bits(mantissa_bits.cast::<u64>());
    
    // Range reduction
    let adjust = mantissa.simd_ge(sqrt2_splat);
    let mantissa = adjust.select(mantissa * half_splat, mantissa);
    let e = adjust.select(e + Simd::splat(1_i32), e);
    
    // Optimized transformation s = (m-1)/(m+1)
    let numerator = mantissa - one_splat;
    let denominator = mantissa + one_splat;
    let s = numerator / denominator;
    let s2 = s * s;
    
    // Full 8-term polynomial evaluation for accuracy using Horner's method with FMA
    let poly = s2
        .mul_add(s2
            .mul_add(s2
                .mul_add(s2
                    .mul_add(s2
                        .mul_add(s2
                            .mul_add(s2
                                .mul_add(Simd::splat(C15), Simd::splat(C13)), 
                                Simd::splat(C11)), 
                            Simd::splat(C9)), 
                        Simd::splat(C7)), 
                    Simd::splat(C5)), 
                Simd::splat(C3)), 
            Simd::splat(C1));
    // Final result computation with optimized mul_add chain
    let log_mantissa = s * poly * two_splat;
    let e_f64 = e.cast::<f64>();
    
    // Single mul_add chain for final result
    e_f64.mul_add(
        Simd::splat(L2U), 
        e_f64.mul_add(Simd::splat(L2L), log_mantissa)
    )
}



/// High-performance SIMD exponential (e^x)
#[inline(always)]
pub fn exp<const N: usize>(x: Simd<f64, N>) -> Simd<f64, N> {
    // Coefficients for exp polynomial
    let log2e = Simd::splat(1.44269504088896340736);
    let ln2 = Simd::splat(0.693147180559945309417);

    let c1 = Simd::splat(1.0);
    let c2 = Simd::splat(0.5);
    let c3 = Simd::splat(1.66666666666666019037e-1);
    let c4 = Simd::splat(4.16666666666666019037e-2);
    let c5 = Simd::splat(8.33333333333329318027e-3);
    let c6 = Simd::splat(1.38888888889814059901e-3);
    let c7 = Simd::splat(1.98412698413242405037e-4);

    // Range reduction
    let k = (x * log2e).round();
    let r = x - k * ln2;

    // Polynomial approximation
    let r2 = r * r;
    let poly = c1 + r * (c1 + r2 * (c2 + r * (c3 + r * (c4 + r * (c5 + r * (c6 + r * c7))))));

    // Reconstruct with 2^k
    let k_int = unsafe { k.to_int_unchecked::<i64>() };
    let scale = Simd::<f64, N>::from_bits(((k_int + Simd::splat(1023)) << 52).cast::<u64>());

    poly * scale
}

/// SIMD power function (x^y)
#[inline(always)]
pub fn pow<const N: usize>(x: Simd<f64, N>, y: Simd<f64, N>) -> Simd<f64, N> {
    // x^y = exp(y * ln(x))
    exp(y * ln(x))
}

/// SIMD square root (already in std::simd but provided for completeness)
#[inline(always)]
pub fn sqrt<const N: usize>(x: Simd<f64, N>) -> Simd<f64, N> {
    x.sqrt() // std::simd has this optimized
}

pub mod trig {
    use std::simd::{
        cmp::{SimdPartialEq, SimdPartialOrd},
        num::{SimdFloat, SimdInt},
        Simd, StdFloat, Select
    };
    #[inline(always)]
    pub fn simd_atan<const N: usize>(x: Simd<f64, N>) -> Simd<f64, N> {
        // Polynomial coefficients for atan approximation
        const P4: f64 = -8.750608600031904122785E-1;
        const P3: f64 = -1.615753718733365076637E1;
        const P2: f64 = -7.500855792314704667340E1;
        const P1: f64 = -1.228866684490136173410E2;
        const P0: f64 = -6.485021904942025371773E1;

        const Q4: f64 = 2.485846490142306297962E1;
        const Q3: f64 = 1.650270098316988542046E2;
        const Q2: f64 = 4.328810604912902668951E2;
        const Q1: f64 = 4.853903996359136964868E2;
        const Q0: f64 = 1.945506571482613964425E2;

        const MORE_BITS: f64 = 6.123233995736765886130E-17;
        const MORE_BITS_O2: f64 = 6.123233995736765886130E-17 * 0.5;
        const T3PO8: f64 = std::f64::consts::SQRT_2 + 1.0;
        let zero_splat = Simd::<f64, N>::splat(0.0);
        let one_splat = Simd::<f64, N>::splat(1.0);
        let t = x.abs();

        // Range classification
        let notbig = t.simd_le(Simd::splat(T3PO8));
        let notsmal = t.simd_ge(Simd::splat(0.66));

        // Select offset angle
        let mut s = notbig.select(
            Simd::splat(std::f64::consts::FRAC_PI_4),
            Simd::splat(std::f64::consts::FRAC_PI_2),
        );
        s = notsmal.select(s, zero_splat);

        let mut fac = notbig.select(Simd::splat(MORE_BITS_O2), Simd::splat(MORE_BITS));
        fac = notsmal.select(fac, zero_splat);

        // Compute reduced argument z
        let mut a = notbig.select(t, zero_splat);
        a = notsmal.select(a - one_splat, a);

        let mut b = notbig.select(one_splat, zero_splat);
        b = notsmal.select(b + t, b);

        let z = a / b;
        let zz = z * z;

        // Numerator polynomial with mul_add (Horner's method)
        let px = zz.mul_add(
            zz.mul_add(
                zz.mul_add(
                    zz.mul_add(Simd::splat(P4), Simd::splat(P3)),
                    Simd::splat(P2)
                ),
                Simd::splat(P1)
            ),
            Simd::splat(P0)
        );

        // Denominator polynomial with mul_add (Horner's method)
        let qx = zz.mul_add(
            zz.mul_add(
                zz.mul_add(
                    zz.mul_add(Simd::splat(Q4), Simd::splat(Q3)),
                    Simd::splat(Q2)
                ),
                Simd::splat(Q1)
            ),
            Simd::splat(Q0)
        );
        
        // Compute final result: z * ((px/qx) * zz + 1)
        let ratio = px / qx;
        let temp = ratio.mul_add(zz, one_splat);
        let mut result = z * temp;

        // Add offset and correction
        result = result + s + fac;

        let sign_mask = x.to_bits() & Simd::splat(0x8000000000000000u64);
        let result_bits = result.to_bits() ^ sign_mask;
        Simd::from_bits(result_bits)

    }

    #[inline(always)]
    pub fn simd_sin_cos<const N: usize>(x: Simd<f64, N>) -> (Simd<f64, N>, Simd<f64, N>) {
        // Polynomial coefficients
        let p0sin = Simd::splat(-1.66666666666666307295E-1);
        let p1sin = Simd::splat(8.33333333332211858878E-3);
        let p2sin = Simd::splat(-1.98412698295895385996E-4);
        let p3sin = Simd::splat(2.75573136213857245213E-6);
        let p4sin = Simd::splat(-2.50507477628578072866E-8);
        let p5sin = Simd::splat(1.58962301576546568060E-10);

        let p0cos = Simd::splat(4.16666666666665929218E-2);
        let p1cos = Simd::splat(-1.38888888888730564116E-3);
        let p2cos = Simd::splat(2.48015872888517045348E-5);
        let p3cos = Simd::splat(-2.75573141792967388112E-7);
        let p4cos = Simd::splat(2.08757008419747316778E-9);
        let p5cos = Simd::splat(-1.13585365213876817300E-11);

        let dp1 = Simd::splat(7.853981554508209228515625E-1 * 2.0);
        let dp2 = Simd::splat(7.94662735614792836714E-9 * 2.0);
        let dp3 = Simd::splat(3.06161699786838294307E-17 * 2.0);
        let two_over_pi = Simd::splat(2.0 / std::f64::consts::PI);

        let i_one_splat = Simd::<i64, N>::splat(1);
        let i_zero_splat = Simd::<i64, N>::splat(0);
        let f64_zero_splat = Simd::<f64, N>::splat(0.0);
        let f64_one_splat = Simd::<f64, N>::splat(1.0);
        
        let xa = x.abs();
        let y = (xa * two_over_pi).round();
        let q: Simd<i64, N> = unsafe { y.to_int_unchecked() };

        // Cody-Waite range reduction
        //let x_reduced = xa - y * dp1 - y * dp2 - y * dp3;
        let x_reduced = y.mul_add(-dp3, y.mul_add(-dp2, y.mul_add(-dp1, xa)));

        let x2 = x_reduced * x_reduced;

        // Polynomial evaluation using Horner's method
        // Single expression for sine polynomial
        let s = x_reduced * (x2.mul_add(
            x2.mul_add(
                x2.mul_add(
                    x2.mul_add(
                        x2.mul_add(p5sin, p4sin),
                        p3sin
                    ),
                    p2sin
                ),
                p1sin
            ) * x2 + p0sin,
            f64_one_splat
        ));
        
        let c = (x2 * x2).mul_add(
            x2.mul_add(
                x2.mul_add(
                    x2.mul_add(
                        x2.mul_add(
                            x2.mul_add(p5cos, p4cos),
                            p3cos
                        ),
                        p2cos
                    ),
                    p1cos
                ),
                p0cos
            ),
            (-x2).mul_add(Simd::splat(0.5), f64_one_splat)
        );

        // Swap sin/cos for odd quadrants
        let swap = (q & i_one_splat).simd_ne(i_zero_splat);

        // Handle overflow (check for finite input)
        let overflow = q.simd_gt(Simd::splat(0x80000000000000i64)) & xa.is_finite();
        let s = overflow.select(f64_zero_splat, s);
        let c = overflow.select(f64_one_splat, c);

        // Apply swap using select
        let sin1 = swap.select(c, s);
        let cos1 = swap.select(s, c);

        // Apply signs using SIMD bit manipulation
        // Convert to bits for manipulation
        let sin1_bits: Simd<u64, N> = sin1.to_bits();
        let cos1_bits: Simd<u64, N> = cos1.to_bits();
        let x_bits: Simd<u64, N> = x.to_bits();

        // Sign for sin: XOR bit 63 (sign bit) based on quadrant and original sign
        let sign_sin_bits = (q << 62).cast::<u64>() ^ x_bits;
        let sign_sin_mask = sign_sin_bits & Simd::splat(0x8000000000000000u64);
        let sin_result_bits = sin1_bits ^ sign_sin_mask;

        // Sign for cos: bit 1 of (q+1) determines sign
        let sign_cos_bits = ((q + i_one_splat) & Simd::splat(2)) << 62;
        let sign_cos_mask = sign_cos_bits.cast::<u64>();
        let cos_result_bits = cos1_bits ^ sign_cos_mask;

        // Convert back to f64
        let sin_result = Simd::<f64, N>::from_bits(sin_result_bits);
        let cos_result = Simd::<f64, N>::from_bits(cos_result_bits);

        (sin_result, cos_result)
    }

    #[inline(always)]
    pub fn simd_sin<const N: usize>(x: Simd<f64, N>) -> Simd<f64, N> {
        let (sin, _) = simd_sin_cos(x);
        sin
    }

    /// High-performance SIMD cosine
    /// Wrapper around simd_sin_cos when only cosine is needed
    #[inline(always)]
    pub fn simd_cos<const N: usize>(x: Simd<f64, N>) -> Simd<f64, N> {
        let (_, cos) = simd_sin_cos(x);
        cos
    }
}
