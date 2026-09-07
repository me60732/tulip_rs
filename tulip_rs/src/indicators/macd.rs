use crate::common::validate_inputs;
#[cfg(feature = "simd_options")]
pub use crate::indicator_types::IndicatorByOptions;
#[cfg(any(feature = "simd_assets", feature = "simd_options"))]
pub use crate::indicator_types::SimdIndicatorResult;
pub use crate::indicator_types::{Indicator, IndicatorResult, TIndicatorState, TState};

use crate::indicators::ema::{Ema, State as EmaState};
use crate::types::{Cold, DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm};
use serde::{Deserialize, Serialize};
/// Number of input price series required by this indicator.
pub const INPUTS: usize = 1;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 3;

pub type IndicatorState = State<Warm>;

impl TIndicatorState<1> for IndicatorState {
    #[inline(always)]
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut macd_line, mut signal_line, mut histogram, mut short_ema_line, mut long_ema_line);
        {
            let capacity = inputs[0].len();

            // Pre-allocate the result vectors with the calculated capacities
            macd_line = crate::uninit_vec!(f64, capacity);
            signal_line = crate::uninit_vec!(f64, capacity);
            histogram = crate::uninit_vec!(f64, capacity);

            (short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
            short_ema_line: capacity,
            long_ema_line: capacity
            );
        }
        cycle_macd(
            inputs[0],
            self,
            (&mut macd_line, &mut signal_line, &mut histogram),
            (&mut short_ema_line, &mut long_ema_line),
        );
        Ok(vec![
            macd_line,
            signal_line,
            histogram,
            short_ema_line,
            long_ema_line,
        ])
    }
}
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub short_ema: EmaState<S>,
    pub long_ema: EmaState<S>,
    pub signal_state: EmaState<S>,
}
impl State {
    pub fn new(
        short_ema: f64,
        long_ema: f64,
        signal: f64,
        (short_period, long_period, signal_period): (usize, usize, usize),
    ) -> Self {
        Self {
            short_ema: EmaState::new(short_ema, short_period),
            long_ema: EmaState::new(long_ema, long_period),
            signal_state: EmaState::new(signal, signal_period),
        }
    }

    pub fn init_state(
        real: &[f64],
        (short_period, long_period, signal_period): (usize, usize, usize),
        macd_line: &mut [f64],
        (short_ema_line, long_ema_line): (&mut [f64], &mut [f64]),
    ) -> State<Warm> {
        let mut short_ema = EmaState::init_state(real, short_period);
        let mut long_ema = EmaState::init_state(real, long_period);

        let (has_optional, _, _) = crate::calc_want_flags!(short_ema_line, long_ema_line);

        // Advance short EMA to match long EMA position
        for i in short_period..long_period {
            let ema = short_ema.calc(real[i]);
            crate::init_store_optional_outputs!(i, real.len(),
                short_ema_line => ema
            );
        }

        // First MACD is the signal EMA seed
        let macd = short_ema.ema - long_ema.ema;
        macd_line[0] = macd;
        let mut signal_state = EmaState::new(macd, signal_period).into_warm();
        let mut count = 1; // seed already written at 0

        // signal_period - 2 more bars to complete signal EMA warm-up
        for i in long_period..long_period + signal_period - 2 {
            let s_ema = short_ema.calc(real[i]);
            let l_ema = long_ema.calc(real[i]);
            let macd_val = s_ema - l_ema;
            macd_line[count] = macd_val;
            signal_state.calc(macd_val);
            if has_optional {
                crate::init_store_optional_outputs!(i, real.len(),
                    short_ema_line => s_ema,
                    long_ema_line => l_ema
                );
            }
            count += 1;
        }

        State {
            short_ema,
            long_ema,
            signal_state,
        }
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = f64;
    type Outputs = (f64, f64, f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, real: Self::Inputs<'a>) -> Self::Outputs {
        let short_ema = self.short_ema.calc(real);
        let long_ema = self.long_ema.calc(real);
        let macd_value = short_ema - long_ema;
        let signal = self.signal_state.calc(macd_value);

        (macd_value, signal, macd_value - signal, short_ema, long_ema)
    }
}

pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] || options[2] < 1.0 {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

//#[inline(always)]
fn cycle_macd(
    real: &[f64],
    state: &mut State<Warm>,
    (macd_line, signal_line, histogram_line): (&mut [f64], &mut [f64], &mut [f64]),
    (short_ema_line, long_ema_line): (&mut [f64], &mut [f64]),
) {
    let (has_optional, want_short, want_long) =
        crate::calc_want_flags!(short_ema_line, long_ema_line);

    for i in 0..real.len() {
        let (macd, signal, histogram, short_ema, long_ema) =
            unsafe { state.calc(*real.get_unchecked(i)) };

        unsafe {
            *macd_line.get_unchecked_mut(i) = macd;
            *signal_line.get_unchecked_mut(i) = signal;
            *histogram_line.get_unchecked_mut(i) = histogram;
        }
        if has_optional {
            crate::store_optional_outputs!(i,
                want_short, short_ema_line => short_ema,
                want_long, long_ema_line => long_ema
            );
        }
    }
}

pub struct Macd;

impl Indicator<INPUTS, OPTIONS> for Macd {
    type IndicatorState = IndicatorState;

    const INFO: Info = Info {
        name: "macd",
        full_name: "Moving Average Convergence Divergence",
        indicator_type: IndicatorType::Trend,
        inputs: &["real"],
        options: &["short_period", "long_period", "signal_period"],
        outputs: &["macd_line", "signal_line", "histogram"],
        optional_outputs: &["short_ema", "long_ema"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "macd",
                label: "MACD",
                display_type: DisplayType::Indicator,
                outputs: &["macd_line", "signal_line", "histogram"],
            },
            DisplayGroup {
                offset: None,
                id: "short_ema_long_ema",
                label: "EMAs",
                display_type: DisplayType::Overlay,
                outputs: &["short_ema", "long_ema"],
            },
        ],
    };
    fn slot_lengths(data_len: usize, options: &[f64; OPTIONS]) -> Vec<usize> {
        //let min_data = min_data(&options);
        let long_period = options[1] as usize;
        let signal_period = options[2] as usize;

        let macd_capacity = data_len - long_period + 1;
        let signal_capacity = macd_capacity - signal_period + 1;

        vec![macd_capacity, signal_capacity, signal_capacity]
    }

    fn min_data(options: &[f64; OPTIONS]) -> usize {
        (options[1] + options[2]) as usize - 1
    }

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError> {
        validate_options(options)?;

        validate_inputs(inputs, Self::min_data(options))?;

        let (
            mut macd_line,
            mut signal_line,
            mut histogram,
            mut short_ema_line,
            mut long_ema_line,
            mut state,
            real,
        );
        {
            let short_period = options[0] as usize;
            let long_period = options[1] as usize;
            let signal_period = options[2] as usize;
            // Calculate capacities
            let len = inputs[0].len();
            let caps = Self::slot_lengths(len, options);

            let short_ema_capacity = Ema::output_length(len, &[short_period as f64]);
            let long_ema_capacity = Ema::output_length(len, &[long_period as f64]);
            // Pre-allocate the result vectors with the calculated capacities
            macd_line = crate::uninit_vec!(f64, caps[0]);
            signal_line = crate::uninit_vec!(f64, caps[1]);
            histogram = crate::uninit_vec!(f64, caps[2]);

            (short_ema_line, long_ema_line) = crate::init_optional_outputs!(
                optional_outputs, &[false, false],
                short_ema_line: short_ema_capacity,
                long_ema_line: long_ema_capacity
            );
            state = State::init_state(
                inputs[0],
                (short_period, long_period, signal_period),
                &mut macd_line,
                (&mut short_ema_line, &mut long_ema_line),
            );
            let start = long_period + signal_period - 2;
            real = &inputs[0][start..]
        }
        let (macd_offset, short_offset, long_offset) = crate::slice_outputs_start!(
            signal_line.len(),
            macd_line,
            short_ema_line,
            long_ema_line
        );
        cycle_macd(
            real,
            &mut state,
            (
                &mut macd_line[macd_offset..],
                &mut signal_line,
                &mut histogram,
            ),
            (
                &mut short_ema_line[short_offset..],
                &mut long_ema_line[long_offset..],
            ),
        );

        Ok((
            vec![
                macd_line,
                signal_line,
                histogram,
                short_ema_line,
                long_ema_line,
            ],
            state,
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::macd_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Macd {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::macd_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
