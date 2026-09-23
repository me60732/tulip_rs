//use crate::common::{Inputs, Options, Outputs};
/*pub fn abs(data: &[f64], start_index: usize) -> Vec<f64> {
    data[start_index..].iter().map(|&x| x.abs()).collect()
}

pub fn acos(data: &[f64], start_index: usize) -> Vec<f64> {
    data[start_index..].iter().map(|&x| x.acos()).collect()
}*/

pub fn abs(value: f64) -> f64 {
    value.abs()
}

pub fn acos(value: f64) -> f64 {
    value.acos()
}

pub fn crossover(inputs: &[&[f64]; 2]) -> Vec<f64> {
    let mut result = Vec::new();
    if inputs[0].len() < 2 {
        return result;
    }

    let in1 = &inputs[0];
    let in2 = &inputs[1];

    for i in 1..in1.len() {
        if in1[i - 1] < in2[i - 1] && in1[i] > in2[i] {
            result.push(1.0);
        } else {
            result.push(0.0);
        }
    }

    result
}

pub fn crossany(inputs: &[&[f64]; 2]) -> Vec<f64> {
    let mut result = Vec::new();
    if inputs.len() < 2 || inputs[0].len() < 2 {
        return result;
    }

    let in1 = &inputs[0];
    let in2 = &inputs[1];

    for i in 1..in1.len() {
        if (in1[i - 1] < in2[i - 1] && in1[i] > in2[i])
            || (in1[i - 1] > in2[i - 1] && in1[i] < in2[i])
        {
            result.push(1.0);
        } else {
            result.push(0.0);
        }
    }

    result
}
pub fn sum(real: &[f64], period: usize) -> f64 {
    real[0..period].iter().sum::<f64>()
}

/// Fast, accuracy-verified scalar atan for **all** argument magnitudes.
///
/// Faithful port of openlibm / FreeBSD `s_atan.c` (fdlibm-derived, Sun BSD
/// license — same algorithm glibc descends from; `e_atan2.c` itself reduces
/// to a call of this `atan` after quadrant handling). Four-region argument
/// reduction maps every finite `|x|` onto `|z| ≤ 0.4375`, where one 11-term
/// series evaluation applies; `atanhi/atanlo` double-word constants give ~1-ulp
/// accuracy:
///
/// * `[0, 7/16)`      direct series
/// * `[7/16, 11/16)`  atan(0.5) + atan((2x−1)/(2+x))
/// * `[11/16, 19/16)` atan(1)   + atan((x−1)/(x+1))
/// * `[19/16, 39/16)` atan(1.5) + atan((x−1.5)/(1+1.5x))
/// * `[39/16, ∞)`     atan(∞)   + atan(−1/x)
///
/// Unlike the previous small-domain-only attempt (dead code for MSW's
/// `atan(ip/rp)`), this accelerates the whole range — HD's small `im/re`,
/// MAMA's `q1/i1`, and MSW's unbounded phase ratio alike. Sweep-validated
/// against libm below.
#[inline(always)]
pub fn atan_fast(x: f64) -> f64 {
    // aT[] — Taylor coefficients of atan(z)/z in w = z², terms 1/3 … 1/23.
    const A0: f64 = 3.33333333333329318027e-01;
    const A1: f64 = -1.99999999998764832476e-01;
    const A2: f64 = 1.42857142725034663711e-01;
    const A3: f64 = -1.11111104054623557880e-01;
    const A4: f64 = 9.09088713343650656196e-02;
    const A5: f64 = -7.69187620504482999495e-02;
    const A6: f64 = 6.66107313738753120669e-02;
    const A7: f64 = -5.83357013379057348645e-02;
    const A8: f64 = 4.97687799461593236017e-02;
    const A9: f64 = -3.65315727442169155270e-02;
    const A10: f64 = 1.62858201153657823623e-02;

    // atanhi/atanlo — hi part + double-word correction of each anchor angle.
    const ATANHI: [f64; 4] = [
        4.63647609000806093515e-01, // atan(0.5)
        7.85398163397448278999e-01, // atan(1.0)
        9.82793723247329054082e-01, // atan(1.5)
        1.57079632679489655800e+00, // atan(inf)
    ];
    const ATANLO: [f64; 4] = [
        2.26987774529616870924e-17,
        3.06161699786838301793e-17,
        1.39033110312309984516e-17,
        6.12323399573676603587e-17,
    ];

    let t = x.abs();

    // NaN — note inf must NOT be caught here; it saturates in the >=2^66 branch below.
    if t.is_nan() {
        return x + x;
    }
    // |x| >= 2^66 (incl. ±inf): atan saturates at ±π/2 (1/x below half-ulp)
    if t >= 7.378697629483821e19 {
        return (ATANHI[3] + ATANLO[3]).copysign(x);
    }

    // Series core — shared by all paths. mul_add (explicit FMA) for the Horner
    // steps; algebraic_* for the remaining plain ops, letting LLVM reassociate
    // and contract the whole evaluation (results stay ~1 ulp, well inside the
    // 1e-14 libm sweeps below).
    #[inline(always)]
    fn series(z: f64) -> f64 {
        let w = z.algebraic_mul(z);
        let s1 = {
            let t = A10.mul_add(w, A8);
            let t = t.mul_add(w, A6);
            let t = t.mul_add(w, A4);
            let t = t.mul_add(w, A2);
            let t = t.mul_add(w, A0);
            z.algebraic_mul(t)
        };
        let s2 = {
            let u = A9.mul_add(w, A7);
            let u = u.mul_add(w, A5);
            let u = u.mul_add(w, A3);
            let u = u.mul_add(w, A1);
            w.algebraic_mul(u)
        };
        s1.algebraic_add(s2)
    }

    if t < 0.4375 {
        if t < 7.450580596923828125e-9 {
            return x; // |x| < 2^-27: atan(x) == x at this precision
        }
        // id = -1: direct, signed x — fma: x - x*sv
        let sv = series(x.algebraic_mul(x));
        return (-sv).mul_add(x, x);
    }

    let ax = t;
    let (id, u): (usize, f64) = if ax < 0.6875 {
        (
            0,
            (2.0f64)
                .mul_add(ax, -1.0)
                .algebraic_div(2.0f64.algebraic_add(ax)),
        )
    } else if ax < 1.1875 {
        (
            1,
            ax.algebraic_sub(1.0).algebraic_div(ax.algebraic_add(1.0)),
        )
    } else if ax < 2.4375 {
        (
            2,
            ax.algebraic_sub(1.5)
                .algebraic_div((1.5f64).mul_add(ax, 1.0)),
        )
    } else {
        (3, (-1.0f64).algebraic_div(ax))
    };
    let sv = series(u.algebraic_mul(u));
    // ATANHI[id] - ((u*(s1+s2) - ATANLO[id]) - u), first step fused
    let b = sv.mul_add(u, -ATANLO[id]);
    let z = ATANHI[id].algebraic_sub(b.algebraic_sub(u));
    z.copysign(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dense sweep in every reduction region: must stay ~1 ulp of libm.
    #[test]
    fn atan_fast_matches_libm_all_regions() {
        const REGIONS: [(f64, f64); 5] = [
            (0.0, 0.4375),
            (0.4375, 0.6875),
            (0.6875, 1.1875),
            (1.1875, 2.4375),
            (2.4375, 64.0),
        ];
        for (lo, hi) in REGIONS {
            let n = 100_000u64;
            let mut worst = 0.0f64;
            let mut worst_x = 0.0f64;
            for i in 0..=n {
                let x = lo + (hi - lo) * (i as f64) / (n as f64);
                let diff = (atan_fast(x) - x.atan()).abs();
                if diff > worst {
                    worst = diff;
                    worst_x = x;
                }
            }
            assert!(
                worst < 1e-14,
                "region [{lo}, {hi}]: max abs err {worst:.3e} at x = {worst_x}"
            );
        }
    }

    /// Log-uniform sweep: tiny → 2^60 magnitudes, relative error.
    #[test]
    fn atan_fast_log_sweep() {
        for e in -33..=59 {
            let base = 2f64.powi(e);
            for k in 0..8 {
                let x = base * (1.0 + k as f64 * 0.125);
                let (ours, libm) = (atan_fast(x), x.atan());
                let rel = (ours - libm).abs() / libm.max(f64::EPSILON);
                assert!(rel < 1e-14, "x = {x}: rel err {rel:.3e} ({ours} vs {libm})");
            }
        }
    }

    #[test]
    fn atan_fast_is_strictly_odd() {
        let n = 100_000u64;
        for i in 1..=n {
            let x = 3.0 * (i as f64) / (n as f64); // spans every region
            assert_eq!(
                atan_fast(-x).to_bits(),
                (-atan_fast(x)).to_bits(),
                "odd symmetry broken at x = {x}"
            );
        }
    }

    #[test]
    fn atan_fast_edges() {
        assert_eq!(atan_fast(0.0), 0.0);
        // atan(−0) = −0 per IEEE/glibc — our tiny-value early return preserves the sign.
        assert_eq!(atan_fast(-0.0).to_bits(), (-0.0f64).atan().to_bits());
        assert!(atan_fast(f64::NAN).is_nan());
        assert_eq!(atan_fast(f64::INFINITY), (f64::INFINITY).atan());
        assert_eq!(atan_fast(f64::NEG_INFINITY), (f64::NEG_INFINITY).atan());
        assert_eq!(atan_fast(f64::MAX), (f64::MAX).atan());
        // Exact region boundaries + the 2^66 saturation point.
        for &b in &[0.4375f64, 0.6875, 1.1875, 2.4375, 7.378697629483821e19] {
            let d = (atan_fast(b) - b.atan()).abs();
            assert!(d < 1e-14, "boundary +{b}: err {d:.3e}");
            let d = (atan_fast(-b) + b.atan()).abs();
            assert!(d < 1e-14, "boundary −{b}: err {d:.3e}");
        }
        // Tiny arguments: below 2^-27 the series shortcut is exact vs libm.
        for &t in &[1e-12f64, 2f64.powi(-27), 3e-8, 0.4] {
            assert_eq!(atan_fast(t).to_bits(), t.atan().to_bits(), "at {t}");
        }
    }

    /// Fixed-seed LCG across the whole positive axis — no region may fallback
    /// to libm (there is no fallback anymore; every path must be ≤1e-14).
    #[test]
    fn atan_fast_random_against_libm() {
        let mut s = 0x2545F4914F6CDD1Du64;
        let mut next = move || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        let scale = (1u128 << 64) as f64; // 2^64
        for _ in 0..200_000 {
            let r = (next() as u128) as f64 / scale; // [0,1)
            let x = r * 10.0;
            let diff = (atan_fast(x) - x.atan()).abs();
            assert!(diff < 1e-14, "abs err {diff:.3e} at x = {x}");
        }
    }
}
