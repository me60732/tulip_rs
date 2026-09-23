use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
use crate::ring_buffer::single_buffer::generic_buffer::{Buffer, Cold, Warm};
use crate::types::{DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize, Clone)]
pub struct IndicatorState {
    /// Retains only the last `m1` bars — everything older is already folded into
    /// `state.sum1` and the `sma1_ring`. With exactly `m1` history bars, the batch
    /// cycle needs no slicing at all (first output bar sits at index `m1`).
    state: State<Warm>,
    real: Vec<f64>,
}
impl IndicatorState {
    pub fn new(real: &[f64], state: State<Warm>) -> Self {
        let m1 = state.m1;
        Self {
            real: real[real.len() - m1..].to_vec(),
            state,
        }
    }
}
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;
        self.real.extend_from_slice(inputs[0]);

        let mut trima_line = crate::uninit_vec!(f64, inputs[0].len());

        // `self.real` holds exactly the last m1 bars, so the first new bar already
        // sits at index m1 — the pre-sliced convention `cycle_trima` expects.
        cycle_trima(&self.real, &mut trima_line, &mut self.state);

        self.real.drain(..self.real.len() - self.state.m1);

        Ok(vec![trima_line])
    }
}

/// State for the SMA-of-SMA formulation of TRIMA.
///
/// `m1 = (p+1)/2` and `m2 = p-m1+1` are derived from period `p`.
/// TRIMA = SMA_m2(SMA_m1(x)).
#[derive(Serialize, Deserialize, Clone)]
pub struct State<S = Cold> {
    /// Ring buffer holding the most recent m1 SMA values (capacity = m2)
    pub sma1_ring: Buffer<Warm>,
    /// Running sum for first SMA (m1-period window)
    pub sum1: f64,
    /// Running sum for second SMA (m2-period window of first SMA outputs)
    pub sum2: f64,
    /// First SMA window length, `m1 = (period + 1) / 2` — cached so cycles never
    /// re-derive it from the period.
    pub m1: usize,
    /// Inverse of m1 (1/m1)
    pub inv_m1: f64,
    /// Inverse of m2 (1/m2)
    pub inv_m2: f64,
    pub(crate) state: std::marker::PhantomData<S>,
}

impl State {
    /// Creates a new cold state. For warm initialization, use `init_state`.
    pub fn new(
        sma1_ring: Buffer<Warm>,
        sum1: f64,
        sum2: f64,
        m1: usize,
        inv_m1: f64,
        inv_m2: f64,
    ) -> Self {
        Self {
            sma1_ring,
            sum1,
            sum2,
            m1,
            inv_m1,
            inv_m2,
            state: std::marker::PhantomData,
        }
    }

    /// Initializes a warm state from input data.
    ///
    /// Warms up by computing s1 values for bars i = m1-1 .. period-2 plus a zero
    /// sentinel slot, so the ring is exactly full when the state is returned.
    /// The first *valid* TRIMA output is at bar `period-1` and is produced by the
    /// first `calc` call (its push evicts the sentinel, contributing 0).
    pub fn init_state(real: &[f64], period: usize) -> State<Warm> {
        let m1 = (period + 1) / 2;
        let m2 = period - m1 + 1;
        let inv_m1 = 1.0 / m1 as f64;
        let inv_m2 = 1.0 / m2 as f64;

        // Warm the first SMA: sum of first m1 values
        let mut sum1: f64 = real[..m1].iter().sum();

        // Build ring buffer for SMA2 with capacity m2 (start as Cold, then warm)
        let mut sma1_ring: Buffer<Cold> = Buffer::new(m2);

        // Push s1 values for bars i = m1-1 .. period-2 inclusive (that's m2-1 real pushes).
        // The m2-th slot is NOT pushed: it holds the 0.0 default from `Buffer::new`, which
        // acts as the phantom eviction consumed by the first `calc` (bar period-1) — the
        // s1 for bar period-1 is produced by `calc` itself, keeping output alignment
        // identical to the historical formulation (first output at bar period-1).
        let mut s1_values: Vec<f64> = Vec::with_capacity(m2);
        for i in (m1 - 1)..(period - 1) {
            if i >= m1 {
                sum1 += real[i] - real[i - m1];
            }
            let s1 = sum1 * inv_m1;
            s1_values.push(s1);
        }

        // Fill the ring buffer with the s1 values
        for &s1 in &s1_values {
            sma1_ring.push(s1);
        }
        // Count the pre-zeroed sentinel slot as filled so the ring is genuinely full:
        // vals[m2-1] is 0.0 from the allocation and is the value evicted by the first
        // calc's push (index now sits at m2-1), contributing 0 to sum2 — correct, since
        // the s1 it "represents" (bar period-1-m2) predates the series.
        sma1_ring.count = m2;

        let sum2: f64 = s1_values.iter().sum();

        State {
            sma1_ring: sma1_ring.into_full(),
            sum1,
            sum2,
            m1,
            inv_m1,
            inv_m2,
            state: std::marker::PhantomData,
        }
    }
}

impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = f64;

    /// Calculates one bar of TRIMA using the SMA-of-SMA formulation.
    ///
    /// Inputs: `(x[i], x[i - m1])` where `m1 = (p+1)/2`.
    #[inline(always)]
    fn calc<'a>(&mut self, (x_i, x_i_minus_m1): Self::Inputs<'a>) -> Self::Outputs {
        // First SMA: sliding window sum
        self.sum1 += x_i - x_i_minus_m1;
        let s1 = self.sum1 * self.inv_m1;

        self.sum2 += s1 - self.sma1_ring.push_with_info(s1);
        self.sum2 * self.inv_m2
    }
}

/// Performs the main calculation loop for the TRIMA indicator.
///
/// `real` must be pre-sliced so that the first output bar sits at index `state.m1`
/// (i.e. pass `&real[period-1-m1..]`, or `self.real` unchanged when it holds exactly
/// `m1` history bars). Output `j` then reads the pair `(real[j + m1], real[j])`, so
/// the loop needs no per-bar subtraction and no period arithmetic at all.
pub fn cycle_trima(real: &[f64], trima_line: &mut [f64], state: &mut State<Warm>) {
    let m1 = state.m1;

    for (j, i) in (m1..real.len()).enumerate() {
        unsafe {
            *trima_line.get_unchecked_mut(j) =
                state.calc((*real.get_unchecked(i), *real.get_unchecked(j)));
        }
    }
}

/// Computes a multiplier for normalizing the weighted sums in the TRIMA calculation.
///
/// If the period is odd:
///   `multiplier = 1.0 / ((period / 2 + 1) * (period / 2 + 1))`
/// If the period is even:
///   `multiplier = 1.0 / ((period / 2 + 1) * (period / 2))`
///
/// # Arguments
///
/// * `period` - The TRIMA period.
///
/// # Returns
///
/// A `f64` scaling factor applied to produce the final TRIMA value.
pub fn multiplier(period: usize) -> f64 {
    if period % 2 == 1 {
        1.0 / ((period / 2 + 1) * (period / 2 + 1)) as f64
    } else {
        1.0 / ((period / 2 + 1) * (period / 2)) as f64
    }
}

pub struct Trima;

impl Indicator<INPUTS, OPTIONS> for Trima {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "trima",
        full_name: "Triangular Moving Average",
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["period"],
        outputs: &["trima"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "trima",
            label: "TRIMA",
            display_type: DisplayType::Overlay,
            outputs: &["trima"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
        validate_options(options)?;
        validate_inputs(inputs, Self::min_data(options))?;
        let period = options[0] as usize;
        let real = inputs[0];

        let mut trima_line = {
            let capacity = Self::output_length(real.len(), options);
            crate::uninit_vec!(f64, capacity)
        };

        // Initialize rolling sums for the 2 SMA passes in TRIMA.
        // The original TRIMA logic can be performed with a single pass using these sums.
        let mut state = State::init_state(real, period);
        let m1 = state.m1;
        let real = &real[period - 1 - m1..];
        cycle_trima(real, &mut trima_line, &mut state);

        Ok((vec![trima_line], IndicatorState::new(inputs[0], state)))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::trima_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Trima {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::trima_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(test)]
mod sma_of_sma_tests {
    use super::*;

    /// Reference implementation: brute-force O(p²) TRIMA computation
    fn trima_reference_brute_force(real: &[f64], period: usize) -> Vec<f64> {
        let m1 = (period + 1) / 2;
        let m2 = period - m1 + 1;

        // First compute SMA_m1 for all bars
        let mut sma_m1: Vec<f64> = vec![0.0; real.len()];
        let mut sum1: f64 = real[..m1].iter().sum();
        sma_m1[m1 - 1] = sum1 / m1 as f64;

        for i in m1..real.len() {
            sum1 += real[i] - real[i - m1];
            sma_m1[i] = sum1 / m1 as f64;
        }

        // Then compute SMA_m2 of the first SMA
        let mut trima: Vec<f64> = vec![0.0; real.len()];
        let mut sum2: f64 = sma_m1[..m2].iter().sum();
        trima[m1 + m2 - 2] = sum2 / m2 as f64;

        for i in m1 + m2 - 2..real.len() {
            if i >= m1 + m2 - 1 {
                sum2 += sma_m1[i] - sma_m1[i - m2];
            }
            trima[i] = sum2 / m2 as f64;
        }

        // First valid output is at index period-1
        trima.truncate(real.len());
        trima
    }

    #[test]
    fn test_constant_input_returns_constant() {
        for &period in &[4, 5, 10] {
            let constant: f64 = 42.0;
            let real: Vec<f64> = vec![constant; 30];

            let (outputs, _) =
                Trima::indicator(&[real.as_slice()], &[period as f64], None).unwrap();
            let result = &outputs[0];

            for (i, &val) in result.iter().enumerate() {
                assert!(
                    (val - constant).abs() < 1e-9,
                    "p={} bar {}: expected {}, got {}",
                    period,
                    i + period - 1,
                    constant,
                    val
                );
            }
        }
    }

    #[test]
    fn test_equivalence_brute_force() {
        let pseudo_random: Vec<f64> = (0..300)
            .map(|i| ((i * 17 + 3) as f64 / 29.0).fract())
            .collect();

        for &period in &[4, 5, 9, 30] {
            let (outputs, _) =
                Trima::indicator(&[pseudo_random.as_slice()], &[period as f64], None).unwrap();
            let result = &outputs[0];

            let reference = trima_reference_brute_force(&pseudo_random, period);

            // Compare from index period-1 onwards
            for i in (period - 1)..result.len() {
                assert!(
                    (result[i] - reference[i]).abs() < 1e-9,
                    "p={} bar {}: new={}, ref={}",
                    period,
                    i,
                    result[i],
                    reference[i]
                );
            }
        }
    }

    #[test]
    fn test_continuation() {
        let real_a: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
        let real_b: Vec<f64> = (50..100).map(|i| i as f64 * 0.1).collect();

        for &period in &[5, 30] {
            // Full indicator on A++B
            let full_real: Vec<f64> = [real_a.as_slice(), real_b.as_slice()].concat();
            let (full_outputs, _) =
                Trima::indicator(&[full_real.as_slice()], &[period as f64], None).unwrap();

            // Indicator on A, then batch on B
            let (a_outputs, mut state) =
                Trima::indicator(&[real_a.as_slice()], &[period as f64], None).unwrap();
            let b_outputs = state.batch_indicator(&[real_b.as_slice()], None).unwrap();

            // Compare outputs from bar period-1 onwards
            for i in (period - 1)..full_outputs[0].len() {
                assert!(
                    (full_outputs[0][i] - a_outputs[0][i]).abs() < 1e-9,
                    "p={} continuation A mismatch at bar {}: full={}, a={}",
                    period,
                    i,
                    full_outputs[0][i],
                    a_outputs[0][i]
                );
                assert!(
                    (full_outputs[0][i] - b_outputs[0][i - (period - 1)]).abs() < 1e-9,
                    "p={} continuation B mismatch at bar {}: full={}, b={}",
                    period,
                    i,
                    full_outputs[0][i],
                    b_outputs[0][i - (period - 1)]
                );
            }
        }
    }

    #[test]
    fn test_serde_roundtrip() {
        // Test serde using serde_json instead of bincode

        let real: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
        let period: usize = 5;

        // First indicator call
        let (_outputs, mut original_state) =
            Trima::indicator(&[real.as_slice()], &[period as f64], None).unwrap();

        // Serialize and deserialize using serde_json
        let json = serde_json::to_string(&original_state).expect("serialize");
        let mut restored_state: IndicatorState = serde_json::from_str(&json).expect("deserialize");

        // Continue with fresh state vs restored state
        let continuation_real: Vec<f64> = (50..70).map(|i| i as f64 * 0.1).collect();

        let a = original_state
            .batch_indicator(&[continuation_real.as_slice()], None)
            .unwrap();
        let b = restored_state
            .batch_indicator(&[continuation_real.as_slice()], None)
            .unwrap();

        assert_eq!(a.len(), b.len());
        for (ra, rb) in a.iter().zip(b.iter()) {
            assert_eq!(ra.len(), rb.len());
            for (x, y) in ra.iter().zip(rb.iter()) {
                assert_eq!(x.to_bits(), y.to_bits());
            }
        }
    }
}
