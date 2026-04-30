use crate::cdlcommon::{cdl_colour, cdl_body_fill, cdl_real_within_body, cdl_height, HALLOW, GREEN, RED, SHORT, determine_volatility};
use crate::indicators::rema::{calc as calc_rema, multiplier as rema_multiplier, min_data as rema_min_data};
use crate::types::{IndicatorError, Output, Info, InfoIndicatorState, IndicatorType, DisplayType, IndicatorState};
use crate::cldcommontypes::{CandleInfo, ForcastType, TrendType, CandleStick};
use crate::candle_types::{doji::{CDLDoji, DojiOptions}, CDLBasic, CDLMarubozu};
use crate::common::{validate_options, validate_inputs, validate_indicator_state};

pub fn info() -> CandleInfo {
    CandleInfo {
        parent: Info {
            name: "bearishharami",
            full_name: "Bearish Harami",
            display_type: DisplayType::Indicator,
            indicator_type: IndicatorType::Trend,
            inputs: &["open", "high", "low", "close"],
            options: &["line_period", "body_beriod", "min_long_cdl_height", "min_cdl_hight_tolerance", "doji_max_height"],
            outputs: &["cdl_pattern"],
            indicator_state: InfoIndicatorState { 
                array_values: Some(&["open", "high", "low", "close"]), 
                single_values: Some(&["line_ema", "body_ema"])
            },
            optional_outputs: &[]
        },
        forcast: ForcastType::BearishReversal,
        prior_trend: TrendType::Uptrend,
        bars: 2,
        japanese_name: "Harami",
        crossover_offset: None,
    }
}
pub fn min_data(options: &[f64]) -> usize {
    rema_min_data(options) + 2
}
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 2
}

pub fn indicator(inputs: &[&[f64]; 4], options: &[f64], _optional_outputs: Option<&[bool]>) -> Result<Output, IndicatorError> {
    let info = info();
    let bars = info.bars;
    if !validate_options(options) || options[0] < 1.0 || options[1] < 1.0{
        return Err(IndicatorError::InvalidOptions);
    }
    if !validate_inputs(inputs, min_data(options)) {
        return Err(IndicatorError::InvalidInputs);
    }
    let line_period = options[0] as usize;
    let body_period = options[1] as usize;

    let open  = inputs[0];
    let high = inputs[1];
    let low  = inputs[2];
    let close= inputs[3];

    let (mut avg_line, mut avg_body) = determine_volatility(open, high, low, close, line_period, body_period);
    //TODO do i look for an uptrend???
    let capacity = output_length(close.len(), options);
    let mut output: Vec<f64> = Vec::with_capacity(capacity);
    
    (avg_line, avg_body) = cycle(open, high, low, close, avg_line, avg_body, line_period, options, &mut output);
    Ok(Output {
        indicators: vec![output],
        state: IndicatorState {
            single_values: Some(vec![avg_line, avg_body]),
            array_values: Some(vec![open[open.len()-bars..].to_vec(), high[high.len()-bars..].to_vec(), low[low.len()-bars..].to_vec(), close[close.len()-bars..].to_vec()])
        }
    })
}

pub fn indicator_from_state(inputs: &[&[f64]; 4], options: &[f64], indicator_state: &IndicatorState, _optional_outputs: Option<&[bool]>) -> Result<Output, IndicatorError> {
    let info = info();
    let bars = info.bars;
    if !validate_options(options) || options[0] < 1.0 || options[1] < 1.0{
        return Err(IndicatorError::InvalidOptions);
    }
    if !validate_inputs(inputs, 1) {
        return Err(IndicatorError::InvalidInputs);
    }
    
    if !validate_indicator_state(indicator_state, &info, info.bars) {
        return Err(IndicatorError::InvalidIndicatorState);
    }
    let state_prices = indicator_state.array_values();
    let mut open = state_prices[0].clone();
    let mut high = state_prices[1].clone();
    let mut low = state_prices[2].clone();
    let mut close = state_prices[3].clone();
    let mut avg_line = indicator_state.single_values()[0];
    let mut avg_body = indicator_state.single_values()[1];

    open.extend(inputs[0]);
    high.extend(inputs[1]);
    low.extend(inputs[2]);
    close.extend(inputs[3]);


    let mut output = Vec::with_capacity(open.len());
    (avg_line, avg_body) = cycle(&open, &high, &low, &close, avg_line, avg_body, bars, options, &mut output);

    Ok(Output {
        indicators: vec![output],
        state: IndicatorState {
            single_values: Some(vec![avg_line, avg_body]),
            array_values: Some(vec![open[open.len()-bars..].to_vec(), high[high.len()-bars..].to_vec(), low[low.len()-bars..].to_vec(), close[close.len()-bars..].to_vec()])
        }
    })
}

fn cycle(open: &[f64], high: &[f64], low: &[f64], close: &[f64], avg_line: f64, avg_body: f64, start: usize, options: &[f64], output: &mut Vec<f64>) -> (f64, f64) {
    
    let mut remaining = close.len() - start;
    let capacity = output.capacity();
    let mut pattern;
    let line_multiplier = rema_multiplier(options[0] as usize);
    let body_multiplier = rema_multiplier(options[1] as usize);

    let mut options = DojiOptions::new(avg_line, options[2], options[3], avg_body, options[4]);


    for (i, _) in close.iter().enumerate().take(close.len()).skip(start) {
        pattern = calc(open, high, low, close, i, &options);
        
        if remaining <= capacity {
                output.push(pattern as f64);
        }
        remaining -= 1;        
        options.avg_line = calc_rema(high[i],  low[i], avg_line, line_multiplier);
        options.avg_body = calc_rema(open[i], close[i], avg_body, body_multiplier);
    }
    (avg_line, avg_body)
}
#[inline(always)]
pub fn calc(open: &[f64], high: &[f64], low: &[f64], close: &[f64], i: usize, options: &DojiOptions) -> i8 {
    let (first, second, preceding) = (i-1, i, i-2);

    if cdl_body_fill(open[second], close[second]) == HALLOW { return 0 }
    
    if cdl_colour(close[preceding], close[first]) == RED
    || cdl_colour(close[first], close[second]) == GREEN 
    { return 0 }

    if !cdl_real_within_body((open[first], close[first]), open[second]) 
    && !cdl_real_within_body((open[first], close[first]), close[second]) 
    && !(open[second] == close[first] && close[first] == open[second]) 
    { return 0 }
    
    if CDLDoji::is_candlestick(open[second], high[second], low[second], close[second], options) { return 0 }

    if let Some(basic_result) = CDLBasic::classify(open[first], high[first], low[first], close[first], options) {
        if !matches!(basic_result, CDLBasic::WhiteCandle | CDLBasic::LongWhiteCandle) { return 0 }
    } else if let Some(marubozu_result) = CDLMarubozu::classify(open[first], high[first], low[first], close[first], options) {
        if cdl_height((high[first], low[first]), options.avg_line, options.min_cdl_height, options.min_cdl_height_tolerance) == SHORT
        || !matches!(marubozu_result, CDLMarubozu::WhiteMarubozu | CDLMarubozu::OpeningWhiteMarubozu | CDLMarubozu::ClosingWhiteMarubozu) 
        { return 0 }
    } else {
        return 0
    }
    
    -100
}

