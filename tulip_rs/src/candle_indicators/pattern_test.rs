use crate::candle_indicators::candle_patterns::{CandlePattern, PATTERN_REGISTRY};
use crate::candle_indicators::common::{
    cdl_colour, CandleShape, DOWN_TREND, UP_TREND,
};
use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::types::CandleTypes;
use crate::candle_indicators::types::ForcastType;
use crate::indicators::ema::calc as ema_calc;
use crate::ring_buffer::buffer::BufferElement;
use crate::ring_buffer::fixed_single_buffer::FixedMirrorBuffer;
use serde::{Deserialize, Serialize};

pub(crate) const MAX_PATTERN_LENGTH: usize = 4;
pub(crate) const PATTERN_BAR_WINDOW: usize = MAX_PATTERN_LENGTH + 1;

impl BufferElement for CandleBits {}
/// Pattern test - the complete pattern of bars
///
/// Note: Trend is now stored in each CandleBits (bit 31), not as a separate field
#[derive(Clone, Serialize, Deserialize)]
pub struct PatternTest {
    pub bars: FixedMirrorBuffer<CandleBits, PATTERN_BAR_WINDOW>,
    last_trend: bool, // Cache of last calculated trend for when ema == ema_signal
}

impl PatternTest {
    pub fn new() -> Self {
        Self {
            bars: FixedMirrorBuffer::new(),
            last_trend: false,
        }
    }
    #[inline(always)]
    pub fn test_bars(
        &mut self,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        current: usize,
        state: &EmaState,
    ) {
        let (o, h, l, c) = (open[current], high[current], low[current], close[current]);
        // Calculate current trend from EMA state
        let trend = if state.ema > state.ema_signal {
            UP_TREND
        } else if state.ema < state.ema_signal {
            DOWN_TREND
        } else {
            self.last_trend // When equal, use previous trend
        };

        // Cache for next time
        self.last_trend = trend;

        let mut candle_shape = CandleShape::default();
        let colour = cdl_colour(close[current - 1], c);
        let candle_type = CandleTypes::get_type_fast(o, h, l, c, &mut candle_shape, state);


        //DO NOT REMOVE USED FOR TESTING!!!!!
        /*if current >= 9 {
            println!("\nBar {}: trend: {:?}, colour: {:?}, candle_shape: {:?},\ncandle_type: {:?}", current, trend, colour, candle_shape, candle_type);
        }*/

        // Compute mandatory wick-vs-body bits using CandleShape (cached from classification)
        // get_bottom/top_wick_length returns LONG (true) when wick >= body height
        // so lower/upper_wick_lt_body is the negation
        let lower_wick_lt_body = !candle_shape.get_bottom_wick_length(o, l, c);
        let upper_wick_lt_body = !candle_shape.get_top_wick_length(o, h, c);

        // Create CandleBits with all compulsory attributes
        // Lazy position/engulf/wick-2x attributes are computed on-demand
        // when patterns actually need them
        self.bars.push(CandleBits::new(
            &candle_type,
            colour,
            candle_shape.get_fill(o, c),
            trend,
            candle_shape.get_line_height(h, l, state),
            lower_wick_lt_body,
            upper_wick_lt_body,
        ));
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub ema_state: EmaState,
    pub pattern_test: PatternTest,
}
#[derive(Serialize, Deserialize)]
pub struct EmaState {
    pub ema_line: f64,
    pub ema_body: f64,
    pub ema: f64,
    pub ema_signal: f64,
}
impl EmaState {
    pub fn new(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Self {
        Self {
            ema_line: high[0] - low[0],
            ema_body: (open[0] - close[0]).abs(),
            ema: close[0],
            ema_signal: close[0],
        }
    }
    #[inline(always)]
    pub fn calc_candle_ema(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        candle_multipliers: (f64, f64),
    ) {
        self.ema_line = ema_calc(&(high - low), self.ema_line, candle_multipliers);
        self.ema_body = ema_calc(&((open - close).abs()), self.ema_body, candle_multipliers);
    }
    #[inline(always)]
    pub fn calc_trend_ema(&mut self, close: f64, multipliers: ((f64, f64), (f64, f64))) {
        let (trend_multipliers, signal_multipliers) = multipliers;
        self.ema = ema_calc(&close, self.ema, trend_multipliers);
        self.ema_signal = ema_calc(&self.ema, self.ema_signal, signal_multipliers);
    }
}
impl State {
    pub fn init(
        inputs: &[&[f64]; 4],
        candle_period: usize,
        trend_period: usize,
        signal_period: usize,
        multipliers: ((f64, f64), (f64, f64), (f64, f64)),
    ) -> Self {
        let [open, high, low, close] = inputs;
        let mut ema_state = EmaState::new(open, high, low, close);
        let mut pattern_test = PatternTest::new();

        let greater_period = if trend_period + signal_period > candle_period {
            trend_period + signal_period
        } else {
            candle_period
        };
        let (candle_multipliers, trend_multipliers, signal_multipliers) = multipliers;

        let mut i = 1;
        while !pattern_test.bars.is_full() {
            let (open_val, high_val, low_val, close_val) = (open[i], high[i], low[i], close[i]);
            if i < greater_period {
                ema_state.calc_candle_ema(
                    open_val,
                    high_val,
                    low_val,
                    close_val,
                    candle_multipliers,
                );
                if i <= trend_period {
                    ema_state.ema = ema_calc(&close_val, ema_state.ema, trend_multipliers);
                    if i == trend_period {
                        ema_state.ema_signal = ema_state.ema;
                    }
                } else {
                    ema_state.ema = ema_calc(&close_val, ema_state.ema, trend_multipliers);
                    ema_state.ema_signal =
                        ema_calc(&ema_state.ema, ema_state.ema_signal, signal_multipliers);
                }
            } else {
                ema_state.calc_trend_ema(close_val, (trend_multipliers, signal_multipliers));
                pattern_test.test_bars(open, high, low, close, i, &ema_state);
                ema_state.calc_candle_ema(
                    open_val,
                    high_val,
                    low_val,
                    close_val,
                    candle_multipliers,
                );
            }
            i += 1;
        }

        Self {
            ema_state,
            pattern_test,
        }
    }

    #[inline(always)]
    pub fn calc(
        &mut self,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        i: usize,
        multipliers: ((f64, f64), (f64, f64), (f64, f64)),
        forecast_type: Option<ForcastType>,
    ) -> Option<Vec<CandlePattern>> {
        let (candle_multipliers, trend_multipliers) =
            (multipliers.0, (multipliers.1, multipliers.2));
        self.ema_state.calc_trend_ema(close[i], trend_multipliers);
        // Update test pattern with current bar and previous ema state
        self.pattern_test
            .test_bars(open, high, low, close, i, &self.ema_state);

        // Get mutable bars slice for lazy evaluation
        // Patterns will compute body_height, body_gap, wick_gap on-demand
        let bars: &mut [CandleBits] = self.pattern_test.bars.get_slice_mut();

        // Get validated patterns (registry filters + calc() validation with early break)
        let ret = PATTERN_REGISTRY.get_validated_patterns(
            bars,
            (open, high, low, close),
            i,
            &self.ema_state,
            forecast_type,
        );

        //update the ema state with the current bar
        self.ema_state
            .calc_candle_ema(open[i], high[i], low[i], close[i], candle_multipliers);
        ret
    }
}
