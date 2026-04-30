use crate::cdlcommon::{cdl_body_fill, determine_volatility, FILL};
use crate::indicators::rema::{calc as calc_rema, multiplier as rema_multiplier, min_data as rema_min_data};
use crate::types::{IndicatorError, Output, Info, InfoIndicatorState, IndicatorType, DisplayType, IndicatorState};
use crate::cldcommontypes::{CandleInfo, ForcastType, TrendType, CandleStick};
use crate::candle_types::{CDLDoji, DojiOptions};
use crate::common::{validate_options, validate_inputs, validate_indicator_state};

pub fn info() -> CandleInfo {
    CandleInfo {
        parent: Info {
            name: "northerndoji",
            full_name: "Northern Doji",
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
        bars: 1,
        japanese_name: "Kita no Doji",
        crossover_offset: None,
    }
}
pub fn min_data(options: &[f64]) -> usize {
    let info = info();
    rema_min_data(options) + info.bars
}
pub fn output_length(data_len: usize, options: &[f64]) -> usize {
    data_len - min_data(options) + 1
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
    let mut output: Vec<f64> = Vec::with_capacity(output_length(close.len(), options));
    
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

fn cycle(open: &[f64], high: &[f64], low: &[f64], close: &[f64], avg_line: f64, avg_body: f64, start: usize, options: &[f64], output: &mut [f64]) -> (f64, f64) {
    let len = close.len();
    let mut remaining = close.len() - start;
    let line_multiplier = rema_multiplier(options[0] as usize);
    let body_multiplier = rema_multiplier(options[1] as usize);
    let capacity = output.capacity();
    let mut pattern;
    let mut options = DojiOptions::new(avg_line, options[2], options[3], avg_body, options[4]);
    for (i, _) in close.iter().enumerate().take(len).skip(start) {
    

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

    if !CDLDoji::is_candlestick(open[i], high[i], low[i], close[i], options) { return 0 }
    if let Some(CDLDoji::FourPriceDoji) = CDLDoji::classify(open[i], high[i], low[i], close[i], options) { return 0 }

    if CDLDoji::is_candlestick(open[i-1], high[i-1], low[i-1], close[i-1], options) {
        if open[i] < open[i-1] || close[i] < close[i-1] { return 0 }
    } else if cdl_body_fill(open[i-1], close[i-1]) == FILL {
        if open[i] < open[i-1] { return 0 }
    } else if cdl_body_fill(open[i-1], close[i-1]) != FILL { 
        if open[i] < close[i-1] { return 0 }
    }

    if low[i] >= high[i-1] { return 0 }

    -100
}

