use std::simd::{Simd, num::SimdFloat, cmp::SimdPartialOrd};

pub const HALLOW: bool = true;
pub const FILL: bool = false;
pub const GREEN: bool = true;
pub const RED: bool = false;
pub const LONG: bool = true;
pub const SHORT: bool = false;
pub const NO_TOP_WICK: i8 = 1;
pub const NO_BOTTOM_WICK: i8 = -1;
pub const NO_WICK: i8 = 0;
pub const BOTH_WICK: i8 = 2;
pub const UP_TREND: bool = true;
pub const DOWN_TREND: bool = false;

pub const NO_GAP: i8 = tulip_rs_shared::NO_GAP;
pub const BODY_GAP_UP: i8 = tulip_rs_shared::BODY_GAP_UP;
pub const BODY_GAP_DOWN: i8 = tulip_rs_shared::BODY_GAP_DOWN;
pub const WICK_GAP_UP: i8 = tulip_rs_shared::WICK_GAP_UP;
pub const WICK_GAP_DOWN: i8 = tulip_rs_shared::WICK_GAP_DOWN;

//min_long_cdl_height,
pub const MIN_LONG_CDL_HEIGHT: f64 = 0.7; //70%
pub const TOLERANCE: f64 = 0.005; //0.5 %
pub const DOJI_MAX_HEIGHT: f64 = 0.03;

#[derive(Default, Debug)]
pub struct CandleShape {
    pub fill: Option<bool>,
    pub wick: Option<i8>,
    pub top_wick_length: Option<bool>,
    pub bottom_wick_length: Option<bool>,
    pub line_height: bool,
    pub body_height: bool,
}
impl CandleShape {
    pub fn new(line_height: bool, body_height: bool) -> Self {
        Self {
            fill: None,
            wick: None,
            top_wick_length: None,
            bottom_wick_length: None,
            line_height,
            body_height,
        }
    }
    #[inline(always)]
    pub fn get_fill(&mut self, open: f64, close: f64) -> bool {
        if let Some(fill) = self.fill {
            return fill;
        }
        let fill = cdl_body_fill(open, close);
        self.fill = Some(fill);
        fill
    }

    #[inline(always)]
    pub fn get_wick(&mut self, open: f64, high: f64, low: f64, close: f64) -> i8 {
        if let Some(wick) = self.wick {
            return wick;
        }
        let wick = cdl_no_wick(open, high, low, close);
        self.wick = Some(wick);
        wick
    }
    #[inline(always)]
    pub fn get_top_wick_length(&mut self, open: f64, high: f64, close: f64) -> bool {
        if let Some(top_wick_length) = self.top_wick_length {
            return top_wick_length;
        }
        let top_wick_length = cdl_wick_length((open, close), high, None);
        self.top_wick_length = Some(top_wick_length);
        top_wick_length
    }
    #[inline(always)]
    pub fn get_bottom_wick_length(&mut self, open: f64, low: f64, close: f64) -> bool {
        if let Some(bottom_wick_length) = self.bottom_wick_length {
            return bottom_wick_length;
        }
        let bottom_wick_length = cdl_wick_length((open, close), low, None);
        self.bottom_wick_length = Some(bottom_wick_length);
        bottom_wick_length
    }
}

#[inline(always)]
pub(crate) fn cdl_total_range(real1: f64, real2: f64) -> f64 {
    (real1 - real2).abs()
}
#[inline(always)]
pub(crate) fn cdl_body_greater(body: (f64, f64), real: f64, multiplier: f64) -> bool {
    let body_range = cdl_total_range(body.0, body.1);
    if body_range > real * multiplier {
        return true;
    }
    false
}
#[inline(always)]
pub(crate) fn cdl_body_greater_body(body1: (f64, f64), body2: (f64, f64), multiplier: f64) -> bool {
    let body_range1 = cdl_total_range(body1.0, body1.1);
    let body_range2 = cdl_total_range(body2.0, body2.1);
    if body_range1 > body_range2 * multiplier {
        return true;
    }
    false
}
#[inline(always)]
pub(crate) fn cdl_body_range(open: f64, close: f64) -> (f64, f64) {
    if cdl_body_fill(open, close) == HALLOW {
        return (open, close);
    }

    (close, open)
}

#[inline(always)]
pub(crate) fn cdl_real_within_body(body: (f64, f64), real: f64) -> bool {
    let (open, close) = body;
    if cdl_body_fill(open, close) == HALLOW {
        real >= open && real <= close
    } else {
        real >= close && real <= open
    }
}
#[inline(always)]
pub(crate) fn cdl_height(body: (f64, f64), avg_range: f64) -> bool {
    let (open, close) = body;
    let body_range = cdl_total_range(open, close);
    let min_range = MIN_LONG_CDL_HEIGHT * avg_range;
    let tol_range = TOLERANCE * avg_range;
    if body_range >= min_range - tol_range {
        LONG
    } else {
        SHORT
    }
}
#[inline(always)]
pub(crate) fn cdl_height_simd(body: (Simd<f64, 2>, Simd<f64, 2>), avg_range: Simd<f64, 2>) -> [bool; 2] {
    let (top, bottom) = body;
    let range = (top - bottom).abs();
    
    let min_range = Simd::splat(MIN_LONG_CDL_HEIGHT) * avg_range;
    let tol_range = Simd::splat(TOLERANCE) * avg_range;
    range.simd_ge(min_range - tol_range).to_array()
    
}
/// Detect the gap type between two candles.
///
/// Checks for a wick gap first (entire current candle outside prev wick range),
/// then falls back to a body-only gap (current body outside prev body range).
///
/// # Arguments
/// * `prev`    - `(open, high, low, close)` of the previous candle
/// * `current` - `(open, high, low, close)` of the current candle
///
/// # Returns
/// * `WICK_GAP_UP`   ( 2) — current candle entirely above prev wick range
/// * `BODY_GAP_UP`   ( 1) — current body above prev body; wicks still overlap
/// * `NO_GAP`        ( 0) — bodies overlap
/// * `BODY_GAP_DOWN` (-1) — current body below prev body; wicks still overlap
/// * `WICK_GAP_DOWN` (-2) — current candle entirely below prev wick range
#[inline(always)]
pub fn cdl_gap(prev: (f64, f64, f64, f64), current: (f64, f64, f64, f64)) -> i8 {
    let (prev_open, prev_high, prev_low, prev_close) = prev;
    let (cur_open, cur_high, cur_low, cur_close) = current;

    // Wick gap: the entire current candle is outside the prev wick range.
    if cur_low > prev_high {
        return WICK_GAP_UP;
    } else if cur_high < prev_low {
        return WICK_GAP_DOWN;
    }

    // Body gap: current body is outside the prev body range (wicks still overlap).
    let prev_body_top = prev_open.max(prev_close);
    let prev_body_bot = prev_open.min(prev_close);
    let cur_body_bot = cur_open.min(cur_close);
    let cur_body_top = cur_open.max(cur_close);

    if cur_body_bot > prev_body_top {
        return BODY_GAP_UP;
    } else if cur_body_top < prev_body_bot {
        return BODY_GAP_DOWN;
    }

    NO_GAP
}
#[inline(always)]
pub fn cdl_similar_height(body1: (f64, f64), body2: (f64, f64), tolerance: Option<f64>) -> bool {
    let tolerance = tolerance.unwrap_or(TOLERANCE);
    let (open1, close1) = body1;
    let (open2, close2) = body2;
    let height1 = (open1 - close1).abs();
    let height2 = (open2 - close2).abs();
    //let tolerance = tolerance / 100.0 * height1;
    let average = (height1 + height2) / 2.0;
    let diff = (height1 - height2).abs();
    diff <= (average * tolerance)
}
#[inline(always)]
pub(crate) fn cdl_wick_length(body: (f64, f64), real: f64, multiplier: Option<f64>) -> bool {
    let multiplier = multiplier.unwrap_or(1.0);
    let (l_body, h_body) = cdl_body_range(body.0, body.1);
    let body_range = cdl_total_range(l_body, h_body);

    if real < l_body {
        let wick_range = cdl_total_range(real, l_body);
        if wick_range >= body_range * multiplier {
            return LONG;
        }
    } else if real > h_body {
        let wick_range = cdl_total_range(real, h_body);
        if wick_range >= body_range * multiplier {
            return LONG;
        }
    }

    SHORT
}
#[inline(always)]
pub(crate) fn cdl_total_wick_length(open: f64, high: f64, low: f64, close: f64) -> f64 {
    let (l_body, h_body) = cdl_body_range(open, close);

    let top_wick = high - h_body;
    let bottom_wick = l_body - low;
    top_wick + bottom_wick
}
#[inline(always)]
pub(crate) fn cdl_body_fill(open: f64, close: f64) -> bool {
    if open < close {
        HALLOW
    } else {
        FILL
    }
}
#[inline(always)]
pub(crate) fn cdl_colour(prev_close: f64, close: f64) -> bool {
    if prev_close > close {
        return RED;
    }
    GREEN
}

//body position within the line
#[inline(always)]
pub fn cdl_body_position(open: f64, high: f64, low: f64, close: f64) -> Option<f64> {
    let range = high - low;
    if range == 0.0 {
        return None;
    }

    // Determine the top and bottom of the candle's body.
    let body_top = open.max(close);
    let body_bottom = open.min(close);

    // If the body has no upper wick, return 100%.
    if body_top == high {
        return Some(100.0);
    }

    // If the body has no lower wick, return 0%.
    if body_bottom == low {
        return Some(0.0);
    }

    // Otherwise, calculate the center of the body using the boundaries.
    let body_center = (body_top + body_bottom) / 2.0;
    Some(((body_center - low) / range) * 100.0)
}
#[inline(always)]
pub fn cdl_bar_engulf_bar(bar1: (f64, f64), bar2: (f64, f64)) -> bool {
    let prev_top = bar1.0.max(bar1.1);
    let prev_bottom = bar1.0.min(bar1.1);
    let top = bar2.0.max(bar2.1);
    let bottom = bar2.0.min(bar2.1);

    if prev_top == top && prev_bottom == bottom {
        return false;
    } else if prev_top >= top && prev_bottom <= bottom {
        return true;
    }
    false
}
#[inline(always)]
pub fn cdl_no_wick(open: f64, high: f64, low: f64, close: f64) -> i8 {
    // Determine the top and bottom of the candle's body.
    let body_top = open.max(close);
    let body_bottom = open.min(close);
    // Calculate the wick lengths.
    let top_wick_len = high - body_top;
    let bottom_wick_len = body_bottom - low;

    match (top_wick_len == 0.0, bottom_wick_len == 0.0) {
        (true, true) => NO_WICK,
        (true, false) => NO_TOP_WICK,
        (false, true) => NO_BOTTOM_WICK,
        (false, false) => BOTH_WICK,
    }
}
/// Returns the position of `val` within (or relative to) the candle's body as a percentage.
/// The candle body is defined by open and close:
///   - 0% means the value equals the lower bound of the body (min(open, close))
///   - 100% means the value equals the upper bound of the body (max(open, close))
/// Values below the body yield a negative percentage and above yield a percentage above 100.
/// If the candle has no visible body (open == close):
///   - if val equals open, returns 50.0,
///   - otherwise computes a scaled percentage difference relative to the open.
#[inline(always)]
pub fn cdl_real_in_body_position(body: (f64, f64), real: f64) -> f64 {
    let (open, close) = body;
    let lower = open.min(close);
    let upper = open.max(close);
    let range = upper - lower;
    if range.abs() < std::f64::EPSILON {
        // Candle has no visible body.
        if (real - open).abs() < std::f64::EPSILON {
            50.0
        } else if real < open {
            // Compute a negative percentage difference relative to the open.
            -(((open - real) / open.abs().max(1e-10)) * 100.0)
        } else {
            // For a value above the point body.
            100.0 + (((real - open) / open.abs().max(1e-10)) * 100.0)
        }
    } else {
        // Compute the percentage position relative to the body range.
        ((real - lower) / range) * 100.0
    }
}

/*#[inline(always)]
pub(crate) fn cdl_down_bar(prev_body: (f64, f64), body: (f64, f64)) -> bool {
    let (open1, close1) = prev_body;
    let (open2, close2) = body;
    if cdl_body_fill(open1, close1) == FILL {
        if cdl_body_fill(open2, close2) == FILL {
            open2 < open1
        } else {
            close2 < open1
        }
    } else if cdl_body_fill(open2, close2) == FILL {
        open2 < close1
    } else {
        close2 < close1
    }
}*/
