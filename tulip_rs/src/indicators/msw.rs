use crate::common::{validate_inputs, validate_options};
use crate::math_simd::trig::simd_sin_cos;
pub use crate::indicator_types::TIndicatorState;
use crate::types::{DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::simd::{Simd, num::SimdFloat, StdFloat};
pub const INPUTS_WIDTH: usize = 1;
pub const OPTIONS_WIDTH: usize = 1;

#[cfg(feature = "simd_assets")]
pub use crate::indicators::simd_indicators::msw_simd::indicator_by_assets;

#[cfg(feature = "simd_options")]
pub use crate::indicators::simd_indicators::msw_simd::indicator_by_options;

// Sub-module exports with common naming
#[cfg(feature = "simd_assets")]
pub mod by_assets {
    pub use crate::indicators::simd_indicators::msw_simd::indicator_by_assets as indicator;
}

#[cfg(feature = "simd_options")]
pub mod by_options {
    pub use crate::indicators::simd_indicators::msw_simd::indicator_by_options as indicator;
}

macro_rules! simd_increments {
    ($n:expr) => {{
        const fn gen_increments<const N: usize>() -> [f64; N] {
            let mut arr = [0.0; N];
            let mut i = 0;
            while i < N {
                arr[i] = i as f64;
                i += 1;
            }
            arr
        }
        Simd::from_array(gen_increments::<$n>())
    }};
}

pub struct MSWConstants<const N: usize>;
impl<const N: usize> MSWConstants<N>
{
    pub const INCREMENTS: Simd<f64, N> = simd_increments!(N);
    pub const TPI: Simd<f64, N> = Simd::splat(TPI);
}
//let j_vals = f64x4::from([i as f64, (i + 1) as f64, (i + 2) as f64, (i + 3) as f64]);
/// Returns information about the Mesa Sine Wave (MSW) indicator.
pub fn info() -> Info<'static> {
    Info {
        name: "msw",
        full_name: "Mesa Sine Wave",
        indicator_type: IndicatorType::Cycle,
        display_type: DisplayType::Indicator,
        inputs: &["real"],
        options: &["period"],
        outputs: &["msw_sine", "msw_lead"],
        optional_outputs: &[],
    }
}
#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    real: Vec<f64>,
    period: usize,
    multiplier: f64
}
impl IndicatorState {
    pub fn new(real: &[f64], period: usize, multiplier: f64) -> Self {
        Self {
            real: real[real.len() - period..].to_vec(),
            period,
            multiplier
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS_WIDTH],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.real.extend_from_slice(inputs[0]);
        let (mut sine_line, mut lead_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };

        match self.period {
            0..=7 => {
                cycle_msw::<4>(&self.real, self.period, self.multiplier, &mut sine_line, &mut lead_line);
            }
            _ => {
                cycle_msw::<8>(&self.real, self.period, self.multiplier, &mut sine_line, &mut lead_line);
            }
        }
        

        self.real.drain(..self.real.len() - self.period);

        Ok(vec![sine_line, lead_line])
    }
}
pub fn min_data_accuracy(options: &[f64], _decimals: usize) -> usize {
    min_data(options)
}
/// Returns the minimum amount of data required for the MSW indicator.
pub fn min_data(options: &[f64]) -> usize {
    options[0] as usize + 1
}

/// Returns the output length for the MSW indicator.
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
}

pub fn indicator(
    inputs: &[&[f64]; INPUTS_WIDTH],
    options: &[f64; OPTIONS_WIDTH],
    _optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
    
    validate_options(options)?;
    let (period, multiplier) = {
        let period = options[0] as usize;
        (period, multiplier(period))
    };

    validate_inputs(inputs, min_data(options))?;

    let real = inputs[0];

    let (mut sine_line, mut lead_line) = {
        let capacity = output_length(real.len(), options);
        (
            crate::uninit_vec!(f64, capacity),
            crate::uninit_vec!(f64, capacity),
        )
    };
    match period {
        0..=7 => {
            cycle_msw::<4>(real, period, multiplier, &mut sine_line, &mut lead_line);
        }
        _ => {
            cycle_msw::<8>(real, period, multiplier, &mut sine_line, &mut lead_line);
        }
    }
    

    Ok((
        vec![sine_line, lead_line],
        IndicatorState::new(real, period, multiplier),
    ))
}

/// Iterates over the input data and applies the calc function.
//#[inline(always)]
fn cycle_msw<const N: usize>(real: &[f64], period: usize, multiplier: f64, sine_line: &mut [f64], lead_line: &mut [f64]) {
    for (j, i) in (period..real.len()).enumerate() {
        unsafe {
            (
                *sine_line.get_unchecked_mut(j),
                *lead_line.get_unchecked_mut(j),
                ) = calc::<N>(real.get_unchecked(j+1..=i), multiplier)
        };
    }
}
/// Performs the core calculation for the Mesa Sine Wave (MSW) indicator.
const TPI: f64 = PI * 2.0;
const HPI: f64 = PI * 0.5;
const QPI: f64 = PI * 0.25;
#[inline(always)]
pub fn calc<const N: usize>(prev_slice: &[f64], multiplier: f64) -> (f64, f64)
{
    
    let (rp, ip) = calc_rp_ip::<N>(prev_slice, multiplier);
    // Calculate phase from rp and ip
    let phase = if rp.abs() > 0.001 {
        (ip / rp).atan()
    } else {
        PI * if ip < 0.0 { -1.0 } else { 1.0 }
    };

    let mut phase = if rp < 0.0 { phase + PI } else { phase };
    phase += HPI;
    phase = if phase < 0.0 { phase + TPI } else { phase };
    phase = if phase > TPI { phase - TPI } else { phase };

    (phase.sin(), (phase + QPI).sin())
}
#[inline(always)]
pub fn calc_rp_ip<const N: usize>(slice: &[f64], multiplier: f64) -> (f64, f64) 

{
    calc_rp_ip_internal::<N>(slice, multiplier, 0, slice.len())
}
#[inline(always)]
fn calc_rp_ip_internal<const N: usize>(
    slice: &[f64], 
    multiplier: f64, 
    base_idx: usize,
    total_len: usize,
) -> (f64, f64) 

{
   
    let multiplier_simd = Simd::splat(multiplier);
    let mut rp_simd = Simd::splat(0.0);
    let mut ip_simd = Simd::splat(0.0);
    
    // Process forward in chunks
    let mut chunks = slice.chunks_exact(N);
    let mut current_idx = base_idx;
    let len = total_len - 1;
    
    for chunk in &mut chunks {
        // Load chunk directly
        let weights = Simd::from_slice(chunk);
        
        // Calculate j values (angles) in reverse
        let j_vals = Simd::splat((len - current_idx) as f64) - MSWConstants::<N>::INCREMENTS;
        //let j_vals = MSWConstants::<N>::INCREMENTS.mul_add(-multiplier_simd, multiplier_simd * Simd::splat((len - current_idx) as f64));
        // Calculate angles and trig functions
        let (sin_vals, cos_vals) = simd_sin_cos((MSWConstants::<N>::TPI * j_vals) * multiplier_simd);
        
        // Sum the SIMD results
        /*rp += (cos_vals * weights).reduce_sum();
        ip += (sin_vals * weights).reduce_sum();*/
        rp_simd = cos_vals.mul_add(weights, rp_simd);
        ip_simd = sin_vals.mul_add(weights, ip_simd);
        
        current_idx += N;
    }
    // Reduce at the end, only once
    let mut rp = rp_simd.reduce_sum();
    let mut ip = ip_simd.reduce_sum();
    // Handle remainder recursively with smaller SIMD widths
    let remainder = chunks.remainder();
    if !remainder.is_empty() {
        let (rp_rem, ip_rem) = calc_rp_ip_remainder::<N>(remainder, multiplier, current_idx, total_len);
        rp += rp_rem;
        ip += ip_rem;
    }
    
    (rp, ip)
}

// Macro to generate recursive SIMD calls for progressively smaller lane counts
macro_rules! simd_remainder_dispatch {
    ($slice:expr, $period:expr, $base_idx:expr, $total_len:expr, $current_n:expr, []) => {
        // Base case: process remaining elements with scalar code
        {
            let period_f64 = $period as f64;
            let mut rp = 0.0;
            let mut ip = 0.0;
            let len = $total_len - 1;
            for (idx, &weight) in $slice.iter().enumerate() {
                let j = len - $base_idx - idx;
                let angle = TPI * j as f64 / period_f64;
                let (sin, cos) = angle.sin_cos();
                // Use FMA for scalar accumulation
                rp = cos.mul_add(weight, rp);
                ip = sin.mul_add(weight, ip);
            }
            
            (rp, ip)
        }
    };
    
    ($slice:expr, $period:expr, $base_idx:expr, $total_len:expr, $current_n:expr, [$next_n:expr $(, $rest:expr)*]) => {
        if $current_n > $next_n && $slice.len() >= $next_n {
            calc_rp_ip_internal::<$next_n>($slice, $period, $base_idx, $total_len)
        } else {
            simd_remainder_dispatch!($slice, $period, $base_idx, $total_len, $current_n, [$($rest),*])
        }
    };
}
#[inline(always)]
fn calc_rp_ip_remainder<const N: usize>(
    slice: &[f64], 
    multiplier: f64, 
    base_idx: usize,
    total_len: usize,
) -> (f64, f64) 

{
    // Only check lane counts smaller than N
    simd_remainder_dispatch!(slice, multiplier, base_idx, total_len, N, [64, 32, 16, 8, 4, 2])
}

pub fn multiplier(period: usize) -> f64 {
    1.0 / period as f64
}