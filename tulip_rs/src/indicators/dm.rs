use crate::common::{validate_inputs, validate_options};
pub use crate::indicator_types::{TIndicatorState, Indicator, TState, IndicatorResult, SimdIndicatorResult, IndicatorByOptions};
use crate::types::{
    DisplayGroup, DisplayType, IndicatorError, IndicatorType, Info, Warm, Cold
};
use serde::{Deserialize, Serialize};

/// Number of input price series required by this indicator.
pub const INPUTS: usize = 2;

/// Number of option parameters required by this indicator.
pub const OPTIONS: usize = 1;

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct State<S = Cold> {
    pub dmup: f64,
    pub dmdown: f64,
    pub multiplier: f64,
    pub prev_high: f64,
    pub prev_low: f64,
    pub(crate) state: std::marker::PhantomData<S>,
}

pub type IndicatorState = State<Warm>;
impl TState for State<Warm> {
    type Inputs<'a> = (f64, f64);
    type Outputs = (f64, f64);
    
    #[inline(always)]
    fn calc<'a>(&mut self, (high, low): Self::Inputs<'a>) -> Self::Outputs {
        let (dp, dm) = self.calc_dp_dm(high, low);
        self.calc_dmup_dmdown(dp, dm);
        (self.dmup, self.dmdown)
    }
}
impl State<Cold> {
    pub fn new(dmup: f64, dmdown: f64, prev_high: f64, prev_low: f64, multiplier: f64) -> Self {
        Self {
            dmup,
            dmdown,
            prev_high,
            prev_low,
            multiplier,
            state: std::marker::PhantomData,
        }
    }
    pub(crate) fn into_warm(self) -> State<Warm> {
        State {
            dmup: self.dmup,
            dmdown: self.dmdown,
            prev_high: self.prev_high,
            prev_low: self.prev_low,
            multiplier: self.multiplier,
            state: std::marker::PhantomData,
        }
    }
    pub fn init_state(high: &[f64], low: &[f64], period: usize) -> State<Warm> {
        let multiplier = multiplier(period);
        let mut state = State::new(0.0, 0.0, high[0], low[0], multiplier);
        for (&h, &l) in high.iter().zip(low.iter()).take(period).skip(1) {
            let (dp, dm) = state.calc_dp_dm(h, l);
            state.dmup += dp;
            state.dmdown += dm;
        }
        state.into_warm()
    }
}
impl<S> State<S> {

    /// Applies Wilder's smoothing to update DM+ and DM- in state.
    ///
    /// # Arguments
    ///
    /// * `state` - Mutable reference to the DM state containing `dmup`, `dmdown`, and `multiplier`.
    /// * `dp` - The raw DM+ value for the current bar.
    /// * `dm` - The raw DM- value for the current bar.
    ///
    /// # Returns
    ///
    /// A tuple `(dmup, dmdown)` of the updated smoothed directional movement values.
    #[inline(always)]
    fn calc_dmup_dmdown(&mut self, dp: f64, dm: f64) -> (f64, f64) {
        //state.dmup = state.multiplier * state.dmup + dp;
        self.dmup = self.dmup.mul_add(self.multiplier, dp);
        //state.dmdown = state.multiplier * state.dmdown + dm;
        self.dmdown = self.dmdown.mul_add(self.multiplier, dm);
        (self.dmup, self.dmdown)
    }
    /// Calculates the raw DM+ and DM- values for the current bar.
    ///
    /// Uses `state.prev_high` and `state.prev_low` as the previous bar's values,
    /// then updates them to `high` and `low`.
    ///
    /// # Arguments
    ///
    /// * `state` - Mutable reference to the DM state (reads and updates `prev_high` and `prev_low`).
    /// * `high` - The current high price.
    /// * `low` - The current low price.
    ///
    /// # Returns
    ///
    /// A tuple `(dp, dm)` of the raw directional movement values before smoothing.
    #[inline(always)]
    pub fn calc_dp_dm(&mut self, high: f64, low: f64) -> (f64, f64) {
        let mut dp = high - self.prev_high;
        let mut dm = self.prev_low - low;
        (self.prev_high, self.prev_low) = (high, low);
    
        if dp < 0.0 {
            dp = 0.0;
        } else if dp > dm {
            dm = 0.0;
        }
    
        if dm < 0.0 {
            dm = 0.0;
        } else if dm > dp {
            dp = 0.0;
        }
    
        if dp > dm {
            dm = 0.0;
        } else if dm > dp {
            dp = 0.0;
        }
        (dp, dm)
    }
}
impl TIndicatorState<2> for IndicatorState {
    fn batch_indicator(
        &mut self,
        inputs: &[&[f64]; INPUTS],
        _optional_outputs: Option<&[bool]>,
    ) -> Result<Vec<Vec<f64>>, IndicatorError> {
        validate_inputs(inputs, 1)?;

        let (mut plus_dm_line, mut minus_dm_line) = {
            let capacity = inputs[0].len();
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };
        let [high, low] = inputs;
        cycle_calc(
            high,
            low,
            self,
            &mut plus_dm_line,
            &mut minus_dm_line,
        );

        Ok(vec![plus_dm_line, minus_dm_line])
    }
}


/// Performs the main calculation loop for the DM indicator.
///
/// # Arguments
///
/// * `high` - A slice of high prices.
/// * `low` - A slice of low prices.
/// * `state` - Mutable reference to the DM state.
/// * `plus_dm_line` - Mutable slice to write the DM+ output values into.
/// * `minus_dm_line` - Mutable slice to write the DM- output values into.
fn cycle_calc(
    high: &[f64],
    low: &[f64],
    state: &mut State<Warm>,
    plus_dm_line: &mut [f64],
    minus_dm_line: &mut [f64],
) {
    for i in 0..high.len() {
        unsafe {
            let inputs = (*high.get_unchecked(i), *low.get_unchecked(i));
            let (dmup, dmdown) = state.calc(inputs);
            *plus_dm_line.get_unchecked_mut(i) = dmup;
            *minus_dm_line.get_unchecked_mut(i) = dmdown;
        }
    }
}



#[inline]
pub fn multiplier(period: usize) -> f64 {
    ((period - 1) as f64) / period as f64
}

pub struct Dm;

impl Indicator<INPUTS, OPTIONS> for Dm {
    type IndicatorState = IndicatorState;
    const INFO: Info = Info {
        name: "dm",
        full_name: "Directional Movement",
        indicator_type: IndicatorType::Trend,
        inputs: &["high", "low"],
        options: &["period"],
        outputs: &["plus_dm", "minus_dm"],
        optional_outputs: &[],
        display_groups: &[DisplayGroup {
            offset: None,
            id: "dm",
            label: "DM",
            display_type: DisplayType::Indicator,
            outputs: &["plus_dm", "minus_dm"],
        }],
    };
    fn indicator(
        inputs: &[&[f64]; INPUTS],
        options: &[f64; OPTIONS],
        _optional_outputs: Option<&[bool]>,
    ) -> IndicatorResult<Self::IndicatorState> {
        validate_options(options)?;
        let period = options[0] as usize;
    
        validate_inputs(inputs, Self::min_data(options))?;
        let (mut plus_dm_line, mut minus_dm_line) = {
            let capacity: usize = Self::output_length(inputs[0].len(), options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::uninit_vec!(f64, capacity),
            )
        };
    
        let mut state = State::init_state(inputs[0], inputs[1], period);
        let (high, low) = (&inputs[0][period..], &inputs[1][period..]);
        cycle_calc(
            high,
            low,
            &mut state,
            &mut plus_dm_line,
            &mut minus_dm_line,
        );
    
        Ok((
            vec![plus_dm_line, minus_dm_line],
            state,
        ))
    }

    #[cfg(feature = "simd_assets")]
    fn indicator_by_assets<const N: usize>(
        inputs: &[&[&[f64]; INPUTS]; N], //stock[ fields [ field [f64] ] ]
        options: &[f64; OPTIONS],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::dm_simd::indicator_by_assets::<N>(inputs, options, optional_outputs)
    }
}

#[cfg(feature = "simd_options")]
impl IndicatorByOptions<INPUTS, OPTIONS> for Dm {
    fn indicator_by_options<const N: usize>(
        inputs: &[&[f64]; INPUTS], //stock[ fields [ field [f64] ] ]
        options: &[&[f64; OPTIONS]; N],
        optional_outputs: Option<&[bool]>,
    ) -> SimdIndicatorResult<Vec<Self::IndicatorState>> {
        crate::indicators::simd_indicators::dm_simd::indicator_by_options::<N>(inputs, options, optional_outputs)
    }
}
