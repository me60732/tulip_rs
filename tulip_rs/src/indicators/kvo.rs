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
pub const INPUTS: usize = 4;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 2;

pub type IndicatorState = State<Warm>;

impl TIndicatorState<4> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut kvo_line, mut short_ema_line, mut long_ema_line);
        {
            let capacity = inputs[0].len();
            (short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                short_ema_line: capacity,
                long_ema_line: capacity
            );

            kvo_line = crate::uninit_vec!(f64, capacity);
        }
        cycle_kvo(
            (inputs[0], inputs[1], inputs[2], inputs[3]),
            &mut kvo_line,
            self,
            (&mut short_ema_line, &mut long_ema_line),
        );

        Ok(vec![kvo_line, short_ema_line, long_ema_line])
    }
}
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub short_ema: EmaState<S>,
    pub long_ema: EmaState<S>,
    pub cm: f64,
    pub prev_hlc: f64,
    pub prev_hl: f64,
    pub trend: bool,
}
impl State<Cold> {
    pub fn new(
        short_ema: EmaState,
        long_ema: EmaState,
        trend: bool,
        cm: f64,
        prev_hlc: f64,
        prev_hl: f64,
    ) -> Self {
        Self {
            short_ema,
            long_ema,
            trend,
            cm,
            prev_hlc,
            prev_hl,
        }
    }
    pub(crate) fn into_warm(self) -> State<Warm> {
        State {
            short_ema: self.short_ema.into_warm(),
            long_ema: self.long_ema.into_warm(),
            trend: self.trend,
            cm: self.cm,
            prev_hlc: self.prev_hlc,
            prev_hl: self.prev_hl,
        }
    }

    pub fn init_state(
        (high, low, close, volume): (&[f64], &[f64], &[f64], &[f64]),
        (short_period, long_period): (usize, usize),
        short_ema_line: &mut [f64],
    ) -> State<Warm> {
        // Pre-compute initial trend from bar 0 vs bar 1 — equivalent to -2.0 sentinel
        let hlc0 = high[0] + low[0] + close[0];
        let hlc1 = high[1] + low[1] + close[1];
        let (initial_trend, initial_cm) = if hlc1 > hlc0 {
            (true, high[0] - low[0])
        } else if hlc1 < hlc0 {
            (false, high[0] - low[0])
        } else {
            (false, 0.0)
        };

        let mut state = Self::new(
            EmaState::new(0.0, short_period),
            EmaState::new(0.0, long_period),
            initial_trend,
            initial_cm,
            hlc0,
            high[0] - low[0],
        )
        .into_warm();

        for i in 1..long_period {
            let inputs = unsafe {
                (
                    *high.get_unchecked(i),
                    *low.get_unchecked(i),
                    *close.get_unchecked(i),
                    *volume.get_unchecked(i),
                )
            };
            if i == 1 {
                let vf = state.calc_vf(inputs);
                state.short_ema.ema = vf;
                state.long_ema.ema = vf;
            } else {
                let (_, short_ema, _) = state.calc(inputs);
                crate::init_store_optional_outputs!(i, high.len(),
                    short_ema_line => short_ema
                );
            }
        }
        state
    }
}
impl State<Warm> {
    #[inline(always)]
    pub(crate) fn calc_vf(&mut self, (high, low, close, volume): (f64, f64, f64, f64)) -> f64 {
        let hlc = high + low + close;
        let dm = high - low;

        let sign;
        // Update trend and cm
        if !self.trend && hlc > self.prev_hlc {
            self.trend = true;
            sign = 1.0;
            self.cm = self.prev_hl;
        } else if self.trend && hlc < self.prev_hlc {
            self.trend = false;
            self.cm = self.prev_hl;
            sign = -1.0;
        } else {
            sign = if self.trend { 1.0 } else { -1.0 }
        }
        self.cm += dm.max(f64::EPSILON);

        self.prev_hlc = hlc;
        self.prev_hl = dm;

        (dm / self.cm).mul_add(2.0, -1.0).abs() * volume * 100.0 * sign
    }
}
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64, f64, f64);
    type Outputs = (f64, f64, f64);
    #[inline(always)]
    fn calc<'a>(&mut self, inputs: Self::Inputs<'a>) -> Self::Outputs {
        // Extract multipliers once (minor optimization)

        let vf = self.calc_vf(inputs);
        let short_ema = self.short_ema.calc(vf);
        let long_ema = self.long_ema.calc(vf);
        (short_ema - long_ema, short_ema, long_ema)
    }
}
pub(crate) fn validate_options(options: &[f64; OPTIONS]) -> Result<(), IndicatorError> {
    if options[0] < 1.0 || options[1] <= options[0] {
        return Err(IndicatorError::InvalidOptions);
    }
    Ok(())
}

/// Performs the main calculation loop for the KVO indicator.
///
/// # Arguments
///
/// * `inputs` - A tuple of four price slices: `(high, low, close, volume)`.
/// * `multipliers` - A tuple of EMA multiplier pairs for the short and long EMAs.
/// * `kvo_line` - A mutable slice for storing the KVO output values.
/// * `state` - A mutable reference to the indicator state.
/// * `out_vecs` - A tuple of mutable optional output slices: `(short_ema_line, long_ema_line)`.
fn cycle_kvo(
    (high, low, close, volume): (&[f64], &[f64], &[f64], &[f64]),
    kvo_line: &mut [f64],
    state: &mut State<Warm>,
    (short_ema_line, long_ema_line): (&mut [f64], &mut [f64]),
) {
    let (has_optional, want_short, want_long) =
        crate::calc_want_flags!(short_ema_line, long_ema_line);

    for i in 0..high.len() {
        let inputs = unsafe {
            (
                *high.get_unchecked(i),
                *low.get_unchecked(i),
                *close.get_unchecked(i),
                *volume.get_unchecked(i),
            )
        };
        let (kvo, short_ema, long_ema) = state.calc(inputs);
        unsafe { *kvo_line.get_unchecked_mut(i) = kvo };

        if has_optional {
            crate::store_optional_outputs!(i,
                want_short, short_ema_line => short_ema,
                want_long, long_ema_line => long_ema
            );
        }
    }
}

pub struct Kvo;
impl Indicator<INPUTS, OPTIONS> for Kvo {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "kvo",
        indicator_type: IndicatorType::Volume,
        full_name: "Klinger Volume Oscillator",
        inputs: &["high", "low", "close", "volume"],
        options: &["short_period", "long_period"],
        outputs: &["kvo"],
        optional_outputs: &["short_ema", "long_ema"],
        display_groups: &[
            DisplayGroup {
                offset: None,
                id: "kvo",
                label: "KVO",
                display_type: DisplayType::Indicator,
                outputs: &["kvo"],
            },
            DisplayGroup {
                offset: None,
                id: "short_ema_long_ema",
                label: "Volume Force EMAs",
                display_type: DisplayType::Indicator,
                outputs: &["short_ema", "long_ema"],
            },
        ],
    };

    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;

        validate_inputs(inputs, Self::min_data(options))?;
        let [high, low, close, volume] = inputs;

        let (mut kvo_line, mut short_ema_line, mut long_ema_line, mut state, inputs);
        {
            let capacity = Self::output_length(high.len(), options);
            let short_capacity = Ema::output_length(high.len(), &[options[0]]);
            kvo_line = crate::uninit_vec!(f64, capacity);

            (short_ema_line, long_ema_line) = crate::init_optional_outputs_eff!(
                optional_outputs, &[false, false],
                short_ema_line: short_capacity,
                long_ema_line: capacity
            );
            let short_period = options[0] as usize;
            let long_period = options[1] as usize;
            // Perform the main KVO calculation
            state = State::init_state(
                (&high, &low, &close, &volume),
                (short_period, long_period),
                &mut short_ema_line,
            );
            let from = high.len() - capacity;
            inputs = (&high[from..], &low[from..], &close[from..], &volume[from..])
        }
        let optional_outputs = {
            let offset = crate::slice_outputs_start!(kvo_line.len(), short_ema_line);
            (&mut short_ema_line[offset..], long_ema_line.as_mut_slice())
        };

        cycle_kvo(inputs, &mut kvo_line, &mut state, optional_outputs);

        Ok((vec![kvo_line, short_ema_line, long_ema_line], state))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N],
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::kvo_simd::indicator_by_assets::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Kvo {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS],
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::kvo_simd::indicator_by_options::<N>(
            inputs,
            options,
            optional_outputs,
        )
    }
}
