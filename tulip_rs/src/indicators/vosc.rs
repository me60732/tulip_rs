use crate::common::validate_inputs;
pub use crate::indicator_types::{
    Indicator, IndicatorByOptions, IndicatorResult, SimdIndicatorResult, TIndicatorState, TState,
};
use crate::indicators::sma::{multiplier as sma_multiplier, Sma, State as SmaState};
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;
/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

#[derive(Serialize, Deserialize)]
pub struct IndicatorState {
    volume: Vec<f64>,
    state: State<Warm>,
    periods: (usize, usize),
}
impl IndicatorState {
    pub fn new(volume: &[f64], state: State<Warm>, periods: (usize, usize)) -> Self {
        Self {
            volume: volume[volume.len() - periods.1..].to_vec(),
            state,
            periods,
        }
    }
}

impl TIndicatorState<1> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        self.volume.extend_from_slice(inputs[0]);
        let (mut vosc_line, (mut short_sma_line, mut long_sma_line)) = {
            let len = inputs[0].len();
            (
                crate::uninit_vec!(f64, len),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    short_sma_line: len,
                    long_sma_line: len
                ),
            )
        };

        cycle(
            &self.volume,
            self.periods,
            &mut self.state,
            &mut vosc_line,
            (&mut short_sma_line, &mut long_sma_line),
        );
        self.volume.drain(..self.volume.len() - self.periods.1);
        Ok(vec![vosc_line])
    }
}
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub short_state: SmaState<S>,
    pub long_state: SmaState<S>,
}
impl State<Cold> {
    /// Initializes the VOSC calculation by computing the initial fast and slow sums.
    /// The SMA for each is the sum over period divided by period. We use the last
    /// short_period values from the long window for the fast sum.
    pub fn init_state(
        short_period: usize,
        long_period: usize,
        volume: &[f64],
        short_sma_line: &mut [f64],
    ) -> State<Warm> {
        let mut short_state = SmaState::init_state(volume, short_period);

        for i in short_period..long_period {
            let sma = short_state.calc((volume[i], volume[i - short_period]));
            crate::init_store_optional_outputs!(i, volume.len(),
                short_sma_line => sma
            );
        }
        State {
            short_state,
            long_state: SmaState::init_state(volume, long_period),
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64);
    type Outputs = (f64, f64, f64);

    #[inline(always)]
    fn calc(&mut self, (current, short, long): (f64, f64, f64)) -> (f64, f64, f64) {
        let fast_sma = self.short_state.calc((current, short));
        let slow_sma = self.long_state.calc((current, long));
        if slow_sma == 0.0 {
            return (0.0, fast_sma, slow_sma);
        }
        ((fast_sma - slow_sma) * 100.0 / slow_sma, fast_sma, slow_sma)
    }
}

pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

/// Iterates over the volume data and computes VOSC values for each bar.
///
/// # Arguments
///
/// * `volume` - The full input volume slice.
/// * `periods` - A tuple of `(short_period, long_period)`.
/// * `multipliers` - A tuple of `(short_multiplier, long_multiplier)` from `multiplier()`.
/// * `state` - Mutable reference to the rolling `State` (fast and slow sums).
/// * `vosc_line` - Mutable output slice for VOSC values.
/// * `out_vecs` - Mutable output slices for optional outputs: `(short_sma, long_sma)`.
fn cycle(
    volume: &[f64],
    periods: (usize, usize),
    state: &mut State<Warm>,
    vosc_line: &mut [f64],
    out_vecs: (&mut [f64], &mut [f64]),
) {
    //if long_period <= short_period || volume.len() - long_period != vosc_line.len(){ return }
    let (short_period, long_period) = periods;
    let (short_sma_line, long_sma_line) = out_vecs;
    let (has_optional, want_short_sma, want_long_sma) =
        crate::calc_want_flags!(short_sma_line, long_sma_line);

    for (j, i) in (long_period..volume.len()).enumerate() {
        let (vosc, short_sma, long_sma);
        unsafe {
            (vosc, short_sma, long_sma) = state.calc((
                *volume.get_unchecked(i),
                *volume.get_unchecked(i - short_period),
                *volume.get_unchecked(j),
            ));
            *vosc_line.get_unchecked_mut(j) = vosc;
        }
        if has_optional {
            crate::store_optional_outputs!(j,
                want_short_sma, short_sma_line => short_sma,
                want_long_sma, long_sma_line => long_sma
            );
        }
    }
}

#[inline(always)]
pub fn multiplier(short_period: usize, long_period: usize) -> (f64, f64) {
    (sma_multiplier(short_period), sma_multiplier(long_period))
}

pub struct Vosc;

impl Indicator<INPUTS, OPTIONS> for Vosc {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "vosc",
        full_name: "Volume Oscillator",
        indicator_type: IndicatorType::Volume,
        inputs: &["volume"],
        // Two options: short_period and long_period.
        options: &["short_period", "long_period"],
        outputs: &["vosc"],
        optional_outputs: &["short_sma", "long_sma"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "vosc",
                label: "VOSC",
                display_type: DisplayType::Indicator,
                outputs: &["vosc"],
            },
            DisplayGroup {
                offset: None,
                id: "short_sma_long_sma",
                label: "Volume SMAs",
                display_type: DisplayType::Volume,
                outputs: &["short_sma", "long_sma"],
            },
        ],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let short_period = options[0] as usize;
        let long_period = options[1] as usize;

        validate_inputs(inputs, Self::min_data(options))?;
        let volume = inputs[0];
        let (mut vosc_line, (mut short_sma_line, mut long_sma_line)) = {
            let len = volume.len();
            let capacity = Self::output_length(len, options);
            let short_sma_capacity = Sma::output_length(len, &[short_period as f64]);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false, false],
                    short_sma_line: short_sma_capacity,
                    long_sma_line: capacity
                ),
            )
        };
        let start = crate::slice_outputs_start!(vosc_line.len(), short_sma_line);
        // Initialize state.
        let mut state = State::init_state(short_period, long_period, volume, &mut short_sma_line);

        // The very first value is calculated during initialization.

        // Process from index = long_period (first full window is available).
        cycle(
            volume,
            (short_period, long_period),
            &mut state,
            &mut vosc_line,
            (&mut short_sma_line[start..], &mut long_sma_line),
        );

        Ok((
            vec![vosc_line, short_sma_line, long_sma_line],
            IndicatorState::new(volume, state, (short_period, long_period)),
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::vosc_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Vosc {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::vosc_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
