use crate::cdlcommon::{cdl_body_position, cdl_height, cdl_no_wick, cdl_total_range,LONG, SHORT, NO_WICK};
use crate::cldcommontypes::{CandleStick, BodyOptions};
use std::ops::{Deref, DerefMut};

pub struct DojiOptions {
    pub doji_max_height: f64,
    pub body_options: BodyOptions,
}
impl DojiOptions {
    pub fn new(avg_line: f64, min_line_height: f64, min_line_height_tolerance: f64, avg_body: f64, doji_max_height: f64) -> Self {
        DojiOptions {
            doji_max_height,
            body_options: BodyOptions::new(avg_line, min_line_height, min_line_height_tolerance, avg_body),
        }
    }
}
impl Deref for DojiOptions {
    type Target = BodyOptions;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.body_options
    }
}
impl DerefMut for DojiOptions {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.body_options
    }
}
pub enum CDLDoji {
    Doji,
    LongLeggedDoji,
    DragonflyDoji,
    GravestoneDoji,
    FourPriceDoji,
}

impl CandleStick for CDLDoji {
    type Classified = CDLDoji;
    type Options = DojiOptions;
    fn to_string(&self) -> String {
        match self {
            CDLDoji::Doji => "Doji".to_string(),
            CDLDoji::LongLeggedDoji => "Long Legged Doji".to_string(),
            CDLDoji::DragonflyDoji => "Dragonfly Doji".to_string(),
            CDLDoji::GravestoneDoji => "Gravestone Doji".to_string(),
            CDLDoji::FourPriceDoji => "Four Price Doji".to_string(),
        }
    }
    #[inline(always)]
    fn classify(open: f64, high: f64, low: f64, close: f64, options: &Self::Options) -> Option<Self::Classified> {
        // Check for Four Price Doji first—as it’s a very specific pattern.
        if CDLDoji::is_four_price_doji(open, high, low, close, options) {
            // Additional condition example: sometimes you want to check that the body is near zero.
            //if (open - close).abs() < 0.05 { //////// Dont know why this is here
            return Some(CDLDoji::FourPriceDoji);
            //}
        } else if CDLDoji::is_doji(open, close, options) {
            // Next, check for Long Legged Doji.
            if CDLDoji::is_long_legged_doji(open, high, low, close, options) {
                return Some(CDLDoji::LongLeggedDoji);
            }
            // Then, check for Dragonfly Doji.
            if CDLDoji::is_dragonfly_doji(open, high, low, close, options) {
                return Some(CDLDoji::DragonflyDoji);
            }
            // Then, check for Gravestone Doji.
            if CDLDoji::is_gravestone_doji(open, high, low, close, options) {
                return Some(CDLDoji::GravestoneDoji);
            }
            // Finally, if it's a doji (based on body size relative to total range), choose the basic Doji.
            return Some(CDLDoji::Doji);
        }
        // If none of the tests match, return None.
        None
    }
}

impl CDLDoji {
    #[inline(always)]
    fn is_doji(open: f64, close: f64, options: &DojiOptions) -> bool {
        
        let body_range = cdl_total_range(open, close);
        
        if  body_range <= options.avg_body * (options.doji_max_height / 100.0) {
            return true;
        }
        false
    }
    #[inline(always)]
    fn is_long_legged_doji(open: f64, high: f64, low: f64, close: f64, options: &DojiOptions) -> bool {
        if cdl_height((high, low), options.avg_line, options.min_cdl_height, options.min_cdl_height_tolerance) == LONG {
            if let Some(body_position) = cdl_body_position(open, high, low, close) {
                if (45.0..=55.0).contains(&body_position) {
                    return true;
                }
            }
        }

        false
    }
    #[inline(always)]
    fn is_dragonfly_doji(open: f64, high: f64, low: f64, close: f64, options: &DojiOptions) -> bool {
        if cdl_height((high, low), options.avg_line, options.min_cdl_height, options.min_cdl_height_tolerance) == LONG {
            if let Some(body_position) = cdl_body_position(open, high, low, close) {
                if body_position == 100.0 {
                    return true;
                }
            }
        }

        false
    }
    #[inline(always)]
    fn is_gravestone_doji(open: f64, high: f64, low: f64, close: f64, options: &DojiOptions) -> bool {
        if cdl_height((high, low), options.avg_line, options.min_cdl_height, options.min_cdl_height_tolerance) == LONG {
            if let Some(body_position) = cdl_body_position(open, high, low, close) {
                if body_position == 0.0 {
                    return true;
                }
            }
        }

        false
    }
    #[inline(always)]
    fn is_four_price_doji(open: f64, high: f64, low: f64, close: f64, options: &DojiOptions) -> bool {
        if cdl_no_wick(open, high, low, close) == NO_WICK && cdl_height((open, close), options.avg_body, options.doji_max_height, 0.1) == SHORT {
            return true;
        }

        false
    }
}