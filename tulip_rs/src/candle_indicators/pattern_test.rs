use crate::candle_indicators::candle_patterns::{CandlePattern, PATTERN_REGISTRY};
use crate::candle_indicators::common::{
    cdl_colour, cdl_height_simd, CandleShape, DOWN_TREND, UP_TREND,
};
use crate::candle_indicators::registry::CandleBits;
use crate::candle_indicators::types::CandleTypes;
use crate::candle_indicators::types::ForecastType;
use crate::indicators::simd_indicators::ema_simd::{calc_simd as ema_calc, multiplier_simd};
use crate::ring_buffer::buffer::BufferElement;
use crate::ring_buffer::fixed_single_buffer::FixedMirrorBuffer;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::simd::Simd;

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
        state: &mut EmaState,
    ) {
        let (o, h, l, c) = (open[current], high[current], low[current], close[current]);

        state.calc_ema(o, h, l, c);

        let [line_height, body_height] = cdl_height_simd(
            (Simd::from_array([h, c]), Simd::from_array([l, o])),
            Simd::from_array([state.ema_line, state.ema_body]),
        );

        // Calculate current trend from EMA state
        let trend = {
            let [ema, signal] = state.get_trend_emas();
            if ema > signal {
                UP_TREND
            } else if ema < signal {
                DOWN_TREND
            } else {
                self.last_trend // When equal, use previous trend
            }
        };

        // Cache for next time
        self.last_trend = trend;

        let mut candle_shape = CandleShape::new(line_height, body_height);
        let colour = cdl_colour(close[current - 1], c);
        let candle_type = CandleTypes::get_type_fast(o, h, l, c, &mut candle_shape, state);

        //DO NOT REMOVE USED FOR TESTING!!!!!
        /*if current >= 9 {
            println!("\nBar {}: trend: {:?}, colour: {:?}, candle_shape: {:?},\nEmaState: {:?}\ncandle_type: {:?}",
                current, trend, colour, candle_shape, state, candle_type
            );
        }*/

        // Compute mandatory wick-vs-body bits using CandleShape (cached from classification)
        // get_bottom/top_wick_length returns LONG (true) when wick >= body height
        // so lower/upper_wick_lt_body is the negation
        let lower_wick_lt_body = !candle_shape.get_bottom_wick_length(o, l, c);
        let upper_wick_lt_body = !candle_shape.get_top_wick_length(o, h, c);

        // Create CandleBits with all compulsory attributes
        // Position/engulf/wick-2x attributes are computed on-demand
        // when patterns actually need them
        self.bars.push(CandleBits::new(
            &candle_type,
            colour,
            candle_shape.get_fill(o, c),
            trend,
            line_height,
            lower_wick_lt_body,
            upper_wick_lt_body,
            body_height,
        ));
    }
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub ema_state: EmaState,
    pub pattern_test: PatternTest,
}
#[derive(Debug)]
pub struct EmaState {
    multipliers: (Simd<f64, 4>, Simd<f64, 4>),
    ema: Simd<f64, 4>,
    pub ema_line: f64,
    pub ema_body: f64,
}

impl Serialize for EmaState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("EmaState", 4)?;
        s.serialize_field("ema", &self.ema.to_array())?;
        s.serialize_field(
            "multipliers",
            &[self.multipliers.0.to_array(), self.multipliers.1.to_array()],
        )?;
        s.serialize_field("ema_line", &self.ema_line)?;
        s.serialize_field("ema_body", &self.ema_body)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for EmaState {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Ema,
            Multipliers,
            #[serde(rename = "ema_line")]
            EmaLine,
            #[serde(rename = "ema_body")]
            EmaBody,
        }

        struct EmaStateVisitor;
        impl<'de> Visitor<'de> for EmaStateVisitor {
            type Value = EmaState;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct EmaState")
            }
            fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<EmaState, V::Error> {
                let mut ema = None;
                let mut multipliers = None;
                let mut ema_line = None;
                let mut ema_body = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Ema => {
                            let arr: [f64; 4] = map.next_value()?;
                            ema = Some(Simd::from_array(arr));
                        }
                        Field::Multipliers => {
                            let [a, b]: [[f64; 4]; 2] = map.next_value()?;
                            multipliers = Some((Simd::from_array(a), Simd::from_array(b)));
                        }
                        Field::EmaLine => ema_line = Some(map.next_value()?),
                        Field::EmaBody => ema_body = Some(map.next_value()?),
                    }
                }
                Ok(EmaState {
                    ema: ema.ok_or_else(|| de::Error::missing_field("ema"))?,
                    multipliers: multipliers
                        .ok_or_else(|| de::Error::missing_field("multipliers"))?,
                    ema_line: ema_line.ok_or_else(|| de::Error::missing_field("ema_line"))?,
                    ema_body: ema_body.ok_or_else(|| de::Error::missing_field("ema_body"))?,
                })
            }
        }
        const FIELDS: &[&str] = &["ema", "multipliers", "ema_line", "ema_body"];
        deserializer.deserialize_struct("EmaState", FIELDS, EmaStateVisitor)
    }
}

impl EmaState {
    pub fn new(
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        candle_period: usize,
        trend_period: usize,
        signal_period: usize,
    ) -> Self {
        let multipliers = multiplier_simd([
            candle_period,
            candle_period,
            trend_period,
            trend_period + signal_period,
        ]);
        let ema_line = high[0] - low[0];
        let ema_body = (open[0] - close[0]).abs();
        Self {
            ema: Simd::from_array([ema_line, ema_body, close[0], close[0]]),
            ema_line,
            ema_body,
            multipliers,
        }
    }
    #[inline(always)]
    pub fn get_candle_emas(&self) -> [f64; 2] {
        self.ema.extract::<0, 2>().to_array()
    }
    #[inline(always)]
    pub fn get_candle_emas_simd(&self) -> Simd<f64, 2> {
        self.ema.extract::<0, 2>()
    }
    #[inline(always)]
    pub fn get_trend_emas(&self) -> [f64; 2] {
        self.ema.extract::<2, 2>().to_array()
    }
    #[inline(always)]
    pub fn get_ema_line(&self) -> f64 {
        self.ema[0]
    }
    #[inline(always)]
    pub fn get_ema_body(&self) -> f64 {
        self.ema[1]
    }
    #[inline(always)]
    pub fn get_ema(&self) -> f64 {
        self.ema[2]
    }
    #[inline(always)]
    pub fn get_ema_signal(&self) -> f64 {
        self.ema[3]
    }
    #[inline(always)]
    pub fn calc_ema(&mut self, open: f64, high: f64, low: f64, close: f64) {
        [self.ema_line, self.ema_body] = self.get_candle_emas();
        self.ema = ema_calc(
            Simd::from_array([high - low, (open - close).abs(), close, close]),
            self.ema,
            self.multipliers,
        );
    }
}
impl State {
    pub fn init(
        inputs: &[&[f64]; 4],
        candle_period: usize,
        trend_period: usize,
        signal_period: usize,
    ) -> Self {
        let [open, high, low, close] = inputs;
        let mut ema_state = EmaState::new(
            open,
            high,
            low,
            close,
            candle_period,
            trend_period,
            signal_period,
        );
        let mut pattern_test = PatternTest::new();

        let greater_period = if trend_period + signal_period > candle_period {
            trend_period + signal_period
        } else {
            candle_period
        };

        let mut i = 1;
        while !pattern_test.bars.is_full() {
            let (open_val, high_val, low_val, close_val) = (open[i], high[i], low[i], close[i]);
            if i < greater_period {
                ema_state.calc_ema(open_val, high_val, low_val, close_val);
            } else {
                pattern_test.test_bars(open, high, low, close, i, &mut ema_state);
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
        forecast_type: Option<ForecastType>,
    ) -> Option<Vec<CandlePattern>> {
        // Update test pattern with current bar and previous ema state
        self.pattern_test
            .test_bars(open, high, low, close, i, &mut self.ema_state);

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

        ret
    }
}
