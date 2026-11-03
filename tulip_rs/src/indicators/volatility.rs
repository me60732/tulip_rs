use crate::common::{validate_inputs, validate_options};
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};
pub use crate::indicators::stddev::multiplier;
use crate::indicators::stddev::State as StddevState;
use crate::ring_buffer::single_buffer::generic_buffer::Buffer;
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;
/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

const ANNUAL: f64 = 15.874507866387544; // 252_f64.sqrt()

pub type IndicatorState = State<Warm>;
impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let mut volatility_line = crate::uninit_vec!(f64, inputs[0].len());

        cycle(inputs[0], self, &mut volatility_line);

        Ok(vec![volatility_line])
    }
}
#[derive(Serialize, Deserialize, Clone)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub buffer: Buffer<S>,
    pub stddev_state: StddevState<S>,
    pub prev_real: f64,
}
impl State<Cold> {
    pub fn new(prev_real: f64, period: usize) -> Self {
        let stddev_state = StddevState::new(0.0, 0.0, period);
        let buffer = Buffer::new(period);
        State {
            prev_real,
            stddev_state,
            buffer,
        }
    }
    pub fn init_state(real: &[f64], period: usize) -> State<Warm> {
        let (mut sum, mut sum_sq) = (0.0, 0.0);
        let mut buffer = Buffer::new(period);
        for i in 1..=period {
            let v = real[i] / real[i - 1] - 1.0;
            buffer.push(v);
            sum += v;
            sum_sq += v * v;
        }

        State {
            stddev_state: StddevState::new(sum, sum_sq, period).into_warm(),
            buffer: buffer.into_full(),
            prev_real: real[period],
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = f64;
    #[inline(always)]
    fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
        // Rearranged for better numerical stability when prices are large and close
        let value = (real - self.prev_real) / self.prev_real;
        self.prev_real = real;
        let prev_value = self.buffer.push_with_info(value);
        let (sd, _) = self.stddev_state.calc((value, prev_value));
        sd * ANNUAL
    }
}

/// Iterates over the real data slice and computes a Volatility value for each bar.
///
/// # Arguments
///
/// * `real` - Input data slice starting after the initialization window.
/// * `multiplier` - The stddev multiplier computed from the period.
/// * `state` - Mutable reference to the rolling calculation state.
/// * `vol_line` - Mutable output slice for volatility values.
fn cycle(real: &[f64], state: &mut State<Warm>, vol_line: &mut [f64]) {
    for i in 0..real.len() {
        unsafe {
            *vol_line.get_unchecked_mut(i) = state.calc(*real.get_unchecked(i));
        }
    }
}

pub struct Volatility;

impl Indicator<INPUTS, OPTIONS> for Volatility {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "volatility",
        full_name: "Volatility Indicator",
        indicator_type: IndicatorType::Volatility,
        inputs: &["real"],
        options: &["period"],
        outputs: &["volatility"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "volatility",
            label: "VOLATILITY",
            display_type: DisplayType::Indicator,
            outputs: &["volatility"],
        }],
    };

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        options[0] as usize + 2
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
        validate_options(options)?;
        let period = options[0] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let mut vol_line = {
            let capacity = Self::output_length(inputs[0].len(), options);
            crate::uninit_vec!(f64, capacity)
        };
        let mut state = State::init_state(inputs[0], period);

        cycle(&inputs[0][period + 1..], &mut state, &mut vol_line);

        Ok((vec![vol_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::volatility_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Volatility {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::volatility_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

// ── planner recipe (cse) ─────────────────────────────────────────────

impl crate::cse::Planable for Volatility {
    /// The graph: close → lag(1) → sub(diff) → div(roc), then two RollSum
    /// moments over roc/roc² to form variance, then sqrt + annual scale,
    /// with a close Mirror to capture `prev_real` for restoration.
    fn recipe(options: &[f64]) -> crate::cse::Recipe {
        crate::cse::recipes::volatility(options)
    }
}

/// Rebuild the hand `Volatility::State<Warm>` from the two RollSum chains
/// (sum/sum_sq via roc_sum and roc_sq_sum) plus close_mirror (prev_real).
/// The hand's init_state computes sum/sum_sq over ROC[1..=period]; the CSE
/// captures these via two separate RollSum nodes which each export a ring.
impl crate::cse::Restorable<INPUTS, OPTIONS> for Volatility {
    const KIND: &'static str = "volatility";

    fn restore_state(
        v: Vec<crate::cse::ChainState>,
        windows: &[&[f64]],
    ) -> Option<Self::IndicatorState> {
        let [crate::cse::ChainState::RollSumRing {
            sum: roc_sum,
            ring: roc_ring,
            cursor: roc_cursor,
            period: p1,
        }, crate::cse::ChainState::RollSumRing {
            sum: roc_sq_sum,
            ring: _roc_sq_ring,
            cursor: _roc_sq_cursor,
            period: p2,
        }, crate::cse::ChainState::Mirror { value: prev_real }] = v.as_slice()
        else {
            return None;
        };
        // Both RollSums should use the same period
        if p1 != p2 {
            return None;
        }
        let period = *p1 as usize;
        if windows[0].len() < period + 1 {
            // Volatility needs period+1 bars to compute ROC values for init_state
            return None;
        }
        // Rebuild the buffer from roc_ring (same cursor convention as Hma/Adxr:
        // index = cursor / 8, count = capacity). The buffer holds past ROC values
        // for stddev's prev_value handling.
        let capacity = period;
        if roc_ring.len() != capacity || capacity == 0 {
            return None;
        }
        let index = (roc_cursor / 8) as usize;
        let prev_idx = (index + capacity - 1) % capacity;
        let buffer = crate::ring_buffer::single_buffer::generic_buffer::Buffer::<Warm> {
            vals: roc_ring.clone(),
            index,
            capacity,
            count: capacity,
            prev_idx,
            state: std::marker::PhantomData,
        };
        // The hand's stddev uses sum/sum_sq directly; the CSE's RollSum nodes
        // capture these sums over ROC values. Rebuild State with same period and
        // seed from mirror-captured prev_real.
        Some(State::<Warm> {
            buffer,
            stddev_state: crate::indicators::stddev::State::new(*roc_sum, *roc_sq_sum, period)
                .into_warm(),
            prev_real: *prev_real,
        })
    }
}
